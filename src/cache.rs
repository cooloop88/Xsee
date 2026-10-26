use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;

use crate::Rgba;
use crate::scan;

/// 自己的缩略图缓存：定长索引 + 只追加的数据段。
/// 索引每条 80 字节，数据段存 JPEG，读命中只要解一张几十 KB 的小 JPEG。
/// 缩略图和相似指纹写进同一条记录：每张图每个档只有一份账，指纹跟着它自己
/// 那一档的缩略图生成、失效、搬走，不再另开一个库。
/// 文件名带 4：记录从 32 字节长到 80 字节（多了 40 字节指纹），旧索引在启动时
/// 原地迁过来，数据段不动，缩略图一张都不用重做。
const REC: usize = 80;
/// 上一版的索引：32 字节一条、没有指纹。只在迁移时读一次，迁完删掉。
const OLD_REC: usize = 32;
/// 失败名单每条 20 字节：路径哈希 + mtime + 长度。键只到文件本身，
/// 不含盒尺寸——解不开跟要多大缩略图没关系。
const BADREC: usize = 20;
/// 单张缩略图 JPEG 的上限。坏索引里的长度字段不能直接拿去申请缓冲区。
const MAX_REC_LEN: u32 = 16 * 1024 * 1024;
/// 索引/失败名单的最大读取体积；超过就按坏文件处理，别让坏数据拖住启动。
const MAX_LIST_BYTES: u64 = 64 * 1024 * 1024;

const IDX_FILE: &str = "cache4.idx";
const OLD_IDX_FILE: &str = "cache2.idx";
const DAT_FILE: &str = "cache2.dat";
/// 看过的图的名字：一行一个完整路径，只追加。索引里只存路径哈希，反推不出是谁，
/// 有了这份名单才谈得上"把整座缩略图重做一遍"——全部重建的名单从这儿来。
const NAME_FILE: &str = "cache2.name";
/// 失败名单按我们自己的解码能力记账。加了新格式或新解码路径要把这个文件名换掉，
/// 老名单是上一代能力下的结论（PSD 以前必然记在案，接上解码器后就再也试不到了）。
/// 这次从 cache3.bad 换到 cache4.bad：PSD 改吃档内预览、出图线改成一次开档，
/// 都是新的解码能力——旧名单里有 158 条是上一代试不出来记的，它们解不出路径
/// （失败过的文件从没进过名字名单），留着就等于把这些图永久挡在缩略图库外。
const BAD_FILE: &str = "cache4.bad";

static DISABLED: AtomicBool = AtomicBool::new(false);
static HITS: AtomicU64 = AtomicU64::new(0);

pub fn set_enabled(on: bool) {
    DISABLED.store(!on, Ordering::Relaxed);
}

/// 缓存目录只放 C 盘的用户缓存里：%LOCALAPPDATA%\xsee\thumbs。
/// 分发包是"一个 exe + 一个 skins"，旁边不该凭空长出一个几百 MB 的 thumbs；
/// 所以这里压根不留 exe 旁这条路，AppData 写不进就换 LocalLow / 公用目录。
/// 环境变量 XSEE_CACHE_DIR 直接指定（调试用）。
fn cache_dir() -> &'static Path {
    static CHOSEN: OnceLock<PathBuf> = OnceLock::new();
    CHOSEN.get_or_init(|| {
        let mut cand: Vec<PathBuf> = Vec::new();
        if let Some(p) = std::env::var_os("XSEE_CACHE_DIR") {
            cand.push(PathBuf::from(p));
        } else {
            if let Some(l) = std::env::var_os("LOCALAPPDATA") {
                cand.push(PathBuf::from(&l).join("xsee").join("thumbs"));
                cand.push(PathBuf::from(l).join("Temp").join("xsee-thumbs"));
            }
            if let Some(u) = std::env::var_os("USERPROFILE") {
                cand.push(
                    PathBuf::from(u)
                        .join("AppData")
                        .join("LocalLow")
                        .join("xsee")
                        .join("thumbs"),
                );
            }
            if let Some(p) = std::env::var_os("PUBLIC") {
                cand.push(PathBuf::from(p).join("xsee").join("thumbs"));
            }
        }
        for c in &cand {
            // 只试 mkdir 不够：这台机器上有些目录能建但里面建不出文件，所以要真建一个。
            let probe = c.join(".probe");
            if std::fs::create_dir_all(c).is_ok() && std::fs::File::create(&probe).is_ok() {
                let _ = std::fs::remove_file(&probe);
                return c.clone();
            }
        }
        // 全都不让写就退系统临时目录，宁可缓存失效，也不在分发包旁边留下东西。
        std::env::temp_dir().join("xsee-thumbs")
    })
}

pub fn dir() -> PathBuf {
    cache_dir().to_path_buf()
}

/// 相似指纹：dHash 256 位 + DCT 63 位，和缩略图记录存在一起。
/// 位布局是存储格式的一部分，sim 只负责怎么算、怎么比。
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Fp {
    pub d: [u64; 4],
    pub c: u64,
}

impl Fp {
    fn to_bytes(self) -> [u8; 40] {
        let mut b = [0u8; 40];
        for (i, w) in self.d.iter().enumerate() {
            b[i * 8..i * 8 + 8].copy_from_slice(&w.to_le_bytes());
        }
        b[32..40].copy_from_slice(&self.c.to_le_bytes());
        b
    }

    fn from_bytes(b: &[u8]) -> Fp {
        let mut d = [0u64; 4];
        for (i, w) in d.iter_mut().enumerate() {
            *w = u64::from_le_bytes(b[i * 8..i * 8 + 8].try_into().unwrap());
        }
        Fp {
            d,
            c: u64::from_le_bytes(b[32..40].try_into().unwrap()),
        }
    }
}

#[derive(Clone, Copy)]
struct Slot {
    mtime: u32,
    size: u64,
    off: u32,
    len: u32,
    /// None = 这条只有缩略图，指纹还没补/这一档太小不记指纹。
    fp: Option<Fp>,
}

pub type Key = (u64, u16, u16);

struct Idx {
    map: HashMap<Key, Slot>,
    file: Option<File>,
}

struct Dat {
    file: Option<File>,
    pos: u64,
}

fn enabled() -> bool {
    !DISABLED.load(Ordering::Relaxed)
}

fn idx() -> &'static Mutex<Idx> {
    static I: OnceLock<Mutex<Idx>> = OnceLock::new();
    I.get_or_init(|| Mutex::new(Idx::open()))
}

fn dat() -> &'static Mutex<Dat> {
    static D: OnceLock<Mutex<Dat>> = OnceLock::new();
    D.get_or_init(|| Mutex::new(Dat::open()))
}

fn fnv(path: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.as_bytes() {
        h ^= b.to_ascii_lowercase() as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// 指纹表按哈希和索引对齐：同一张图大小写不同的路径必须落到同一条。
pub fn hash_of(path: &str) -> u64 {
    fnv(path)
}

#[derive(Clone, Copy, PartialEq)]
struct Stamp {
    mtime: u32,
    size: u64,
}

/// 原图动了就作废，所以命中必须验这两项。
fn stamp(path: &str) -> Option<Stamp> {
    if let Some(st) = KNOWN.with(|k| {
        k.borrow()
            .as_ref()
            .filter(|(p, _)| p == path)
            .map(|(_, st)| *st)
    }) {
        return Some(st);
    }
    let m = std::fs::metadata(path).ok()?;
    let secs = m
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(Stamp {
        mtime: secs.min(u32::MAX as u64) as u32,
        size: m.len(),
    })
}

thread_local! {
    /// 扫描线从目录枚举里顺手带回当前这一张的大小和修改时间。同一个文件在这一趟里
    /// 要问七八次（查缓存、判大小、入库、挂指纹、算指纹），慢盘上这些都是白跑的
    /// 盘访问。只有路径正是当前这一张才认这份现成的，对不上照常去问盘。
    static KNOWN: RefCell<Option<(String, Stamp)>> = RefCell::new(None);
}

/// 开始处理这一张：把枚举时拿到的大小和修改时间（秒）记在当前线程上。
pub fn note_size(path: &str, size: u64, mtime: u32) {
    KNOWN.with(|k| {
        *k.borrow_mut() = Some((
            path.to_string(),
            Stamp {
                mtime: mtime.min(u32::MAX),
                size,
            },
        ))
    });
}

/// 这一张完了，别把这份数留给下一张。
pub fn forget_size() {
    KNOWN.with(|k| *k.borrow_mut() = None);
}

/// 文件的大小和修改时间：先认扫描线带回来的那份，问不到才去 stat。
/// 出图那条线判大小、备缓冲区都靠它。
pub fn file_stamp(path: &str) -> Option<(u64, u32)> {
    let st = stamp(path)?;
    Some((st.size, st.mtime))
}

fn rec_bytes(k: &Key, s: &Slot) -> [u8; REC] {
    let mut b = [0u8; REC];
    b[0..8].copy_from_slice(&k.0.to_le_bytes());
    b[8..10].copy_from_slice(&k.1.to_le_bytes());
    b[10..12].copy_from_slice(&k.2.to_le_bytes());
    b[12..16].copy_from_slice(&s.mtime.to_le_bytes());
    b[16..24].copy_from_slice(&s.size.to_le_bytes());
    b[24..28].copy_from_slice(&s.off.to_le_bytes());
    b[28..32].copy_from_slice(&s.len.to_le_bytes());
    b[32..72].copy_from_slice(&s.fp.map(Fp::to_bytes).unwrap_or([0u8; 40]));
    b[72] = s.fp.is_some() as u8;
    b
}

fn rec_parse(chunk: &[u8]) -> (Key, Slot) {
    let k = (
        u64::from_le_bytes(chunk[0..8].try_into().unwrap()),
        u16::from_le_bytes(chunk[8..10].try_into().unwrap()),
        u16::from_le_bytes(chunk[10..12].try_into().unwrap()),
    );
    let s = Slot {
        mtime: u32::from_le_bytes(chunk[12..16].try_into().unwrap()),
        size: u64::from_le_bytes(chunk[16..24].try_into().unwrap()),
        off: u32::from_le_bytes(chunk[24..28].try_into().unwrap()),
        len: u32::from_le_bytes(chunk[28..32].try_into().unwrap()),
        fp: (chunk[72] & 1 != 0).then(|| Fp::from_bytes(&chunk[32..72])),
    };
    (k, s)
}

/// 旧索引（32 字节一条）转成新索引（80 字节一条），指纹位先空着。
/// 数据段格式没变，一条 JPEG 都不用重写；迁完把旧索引删掉，下次启动不会再走这里。
/// 返回 false 表示旧索引还在、这次没迁成——调用方别落一个空的新索引把重试堵死。
fn migrate_index() -> bool {
    let old = cache_dir().join(OLD_IDX_FILE);
    let dst = cache_dir().join(IDX_FILE);
    if dst.exists() {
        return true;
    }
    let Ok(all) = std::fs::read(&old) else {
        return true;
    };
    let dat_len = cache_dir()
        .join(DAT_FILE)
        .metadata()
        .map(|m| m.len())
        .unwrap_or(0);
    let full = all.len() / OLD_REC * OLD_REC;
    let mut out = Vec::with_capacity(full / OLD_REC * REC);
    for chunk in all[..full].chunks_exact(OLD_REC) {
        let k = (
            u64::from_le_bytes(chunk[0..8].try_into().unwrap()),
            u16::from_le_bytes(chunk[8..10].try_into().unwrap()),
            u16::from_le_bytes(chunk[10..12].try_into().unwrap()),
        );
        let s = Slot {
            mtime: u32::from_le_bytes(chunk[12..16].try_into().unwrap()),
            size: u64::from_le_bytes(chunk[16..24].try_into().unwrap()),
            off: u32::from_le_bytes(chunk[24..28].try_into().unwrap()),
            len: u32::from_le_bytes(chunk[28..32].try_into().unwrap()),
            fp: None,
        };
        if s.len == 0
            || s.len > MAX_REC_LEN
            || s.off as u64 + s.len as u64 > dat_len
        {
            continue;
        }
        out.extend_from_slice(&rec_bytes(&k, &s));
    }
    let tmp = dst.with_extension("migrate");
    if std::fs::write(&tmp, &out).is_err() {
        return false;
    }
    if let Ok(f) = std::fs::File::open(&tmp) {
        let _ = f.sync_all();
    }
    if std::fs::rename(&tmp, &dst).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    let _ = std::fs::remove_file(&old);
    true
}

impl Idx {
    fn open() -> Idx {
        if !enabled() {
            return Idx {
                map: HashMap::new(),
                file: None,
            };
        }
        let p = cache_dir().join(IDX_FILE);
        // 上一次版本留下的 32 字节索引先原地迁过来；数据段不动。
        if !p.exists() && cache_dir().join(OLD_IDX_FILE).exists() && !migrate_index() {
            // 旧索引还在但这次没迁成：本次不建新索引，别把下次启动的重试也堵死。
            return Idx {
                map: HashMap::new(),
                file: None,
            };
        }
        let dat_len = cache_dir()
            .join(DAT_FILE)
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);
        let mut map = HashMap::new();
        let mut keep = 0usize;
        let mut truncate = false;
        if let Ok(mut f) = File::open(&p) {
            let sz = f.metadata().map(|m| m.len()).unwrap_or(0);
            if sz > MAX_LIST_BYTES {
                // 这么大的索引只可能是坏文件；留档后重建，别在启动路上读它。
                let old = cache_dir().join("cache4.idx.old");
                if std::fs::rename(&p, &old).is_err() {
                    if let Ok(f2) = OpenOptions::new().write(true).open(&p) {
                        let _ = f2.set_len(0);
                    }
                }
            } else {
                let mut all = Vec::new();
                let _ = f.read_to_end(&mut all);
                let full = all.len() / REC * REC;
                if all.len() != full {
                    // 崩在追加中途会留半条。不截掉的话，下次 append 会从错位处接着写，
                    // 之后所有索引项都对不上号，缩略图长期失效。
                    truncate = true;
                }
                for chunk in all[..full].chunks_exact(REC) {
                    let (k, s) = rec_parse(chunk);
                    // 长度越界或指向数据段外的记录不能信：坏值会被当成缓冲区大小。
                    // len 为 0 不是坏条：指纹迁进来时原图那档可能没有缩略图。
                    if s.len > MAX_REC_LEN || s.off as u64 + s.len as u64 > dat_len {
                        continue;
                    }
                    if s.len == 0 && s.fp.is_none() {
                        continue;
                    }
                    map.insert(k, s);
                }
                keep = full;
            }
        }
        if truncate {
            if let Ok(f2) = OpenOptions::new().write(true).open(&p) {
                let _ = f2.set_len(keep as u64);
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(&p).ok();
        Idx { map, file }
    }
}

impl Dat {
    fn open() -> Dat {
        if !enabled() {
            return Dat {
                file: None,
                pos: 0,
            };
        }
        let p = cache_dir().join(DAT_FILE);
        // append 不带 read 时这个句柄是只写的，命中读盘会直接失败，所以两个都要。
        let file = OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(&p)
            .ok();
        let pos = file
            .as_ref()
            .and_then(|f| f.metadata().ok())
            .map(|m| m.len())
            .unwrap_or(0);
        Dat { file, pos }
    }
}

/// 删掉一条打不开的缩略图。索引只追加，必须整份重写，坏记录才不会下次启动又回来。
/// 这条要是还带着指纹，就把缩略图指针清掉、指纹留着——坏的是那段 JPEG，
/// 相似库里这张图不该跟着消失。
fn forget(key: Key) {
    let Ok(mut i) = idx().lock() else { return };
    let Some(s) = i.map.get(&key).copied() else { return };
    if s.len == 0 && s.fp.is_none() {
        return;
    }
    if s.fp.is_none() {
        i.map.remove(&key);
    } else {
        i.map.insert(
            key,
            Slot {
                mtime: s.mtime,
                size: s.size,
                off: 0,
                len: 0,
                fp: s.fp,
            },
        );
    }
    rewrite(&mut i);
}

/// 把内存里这份表整份落到盘上：先写临时文件、同步、原子替换，再把追加句柄接回来。
/// 追加句柄必须先放下，不然同名替换顶不掉一个正被人攥着的文件。
fn rewrite(i: &mut Idx) {
    let p = cache_dir().join(IDX_FILE);
    let mut out = Vec::with_capacity(i.map.len() * REC);
    for (k, s) in &i.map {
        out.extend_from_slice(&rec_bytes(k, s));
    }
    let tmp = p.with_extension("rewrite");
    if std::fs::write(&tmp, &out).is_err() {
        return;
    }
    if let Ok(f) = std::fs::File::open(&tmp) {
        let _ = f.sync_all();
    }
    // 先关掉追加句柄再替换，换完立刻重开；全程持着 idx 锁，别的线程看不到中间态。
    i.file = None;
    if std::fs::rename(&tmp, &p).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    i.file = OpenOptions::new().create(true).append(true).open(&p).ok();
}

/// 按 (路径, 盒尺寸) 取缓存里那份 JPEG。
/// None = 压根没这条（或这条只剩指纹、没有缩略图）；Some((key, None)) = 有条目但
/// 数据段读不出来（坏条）；Some((key, Some(jpg))) = 拿到了。要不要把坏条删掉归调用方决定。
fn cached(path: &str, bw: u32, bh: u32) -> Option<(Key, Option<Vec<u8>>)> {
    let st = stamp(path)?;
    let key = (fnv(path), bw as u16, bh as u16);
    let slot = {
        let i = idx().lock().ok()?;
        i.map.get(&key).filter(|s| s.mtime == st.mtime && s.size == st.size)?
            .clone()
    };
    if slot.len == 0 {
        // 指纹记录：图在这一档没缓存（或者那段 JPEG 坏过被清掉了），照没命中走。
        return None;
    }
    if slot.len > MAX_REC_LEN {
        return Some((key, None));
    }
    let mut jpg = vec![0u8; slot.len as usize];
    let read_ok = {
        let mut d = dat().lock().ok()?;
        let Some(f) = d.file.as_mut() else { return None };
        f.seek(SeekFrom::Start(slot.off as u64)).is_ok() && f.read_exact(&mut jpg).is_ok()
    };
    Some((key, read_ok.then_some(jpg)))
}

fn to_rgba(jpg: &[u8]) -> Option<Rgba> {
    let img = image::load_from_memory(jpg).ok()?.into_rgba8();
    Some(Rgba {
        w: img.width(),
        h: img.height(),
        px: img.into_raw(),
    })
}

/// 命中就返回紧凑 RGBA；原图改过、数据段读飞了、JPEG 解不开都算没命中，
/// 后两种顺手把这条缩略图清掉，免得每次滚回来都撞同一堵墙。另外告诉调用方
/// 这条记录里有没有现成指纹——没有就顺手补一个，不用等"补齐指纹"那一趟。
pub fn lookup_full(path: &str, bw: u32, bh: u32) -> Option<(Rgba, bool)> {
    if !enabled() {
        return None;
    }
    let (key, jpg) = cached(path, bw, bh)?;
    let img = match jpg.as_deref().and_then(to_rgba) {
        Some(i) => i,
        None => {
            forget(key);
            return None;
        }
    };
    let has_fp = idx()
        .lock()
        .ok()
        .and_then(|i| i.map.get(&key).map(|s| s.fp.is_some()))
        .unwrap_or(false);
    HITS.fetch_add(1, Ordering::Relaxed);
    Some((img, has_fp))
}

/// 只拿图，不记命中也不删坏条。重建前拿它当"缓存里原来那张"的对照。
pub fn peek(path: &str, bw: u32, bh: u32) -> Option<Rgba> {
    if !enabled() {
        return None;
    }
    cached(path, bw, bh).and_then(|(_, jpg)| jpg.as_deref().and_then(to_rgba))
}

/// 这条路径在这一档有没有"现在还作数"的缩略图：原图时间戳、大小都得对得上，
/// 长度还得真有数据（只剩指纹的记录不算）。只查索引不解 JPEG，也不碰数据段——
/// 扫描文件夹判断"这张扫过没有"靠它；扫描线把枚举拿到的大小和时间递下来时，
/// 连问盘那一步都省了，几万张就只是几万次数值比对。
pub fn has_thumb(path: &str, bw: u32, bh: u32) -> bool {
    if !enabled() || bw > u16::MAX as u32 || bh > u16::MAX as u32 {
        return false;
    }
    let Some(st) = stamp(path) else { return false };
    let key = (fnv(path), bw as u16, bh as u16);
    idx().lock().map_or(false, |i| {
        i.map
            .get(&key)
            .is_some_and(|s| s.len > 0 && s.mtime == st.mtime && s.size == st.size)
    })
}

/// 这一轮里有多少张是缓存直接给的（一张原图都没解）。
pub fn hits() -> u64 {
    HITS.load(Ordering::Relaxed)
}

/// 文件被搬走：把旧路径的缩略图数据原样挂一份到新路径上。
/// 只是省解码，失败也不影响正确性——新路径查不到照样重新生成；
/// 缓存键里带完整路径，旧图绝不会被拿去配新文件。
pub fn move_path(old: &str, new: &str) {
    if !enabled() || old == new {
        return;
    }
    let Some(st) = stamp(new) else { return };
    let oh = fnv(old);
    let nh = fnv(new);
    let olds: Vec<(u16, u16, Slot)> = match idx().lock() {
        Ok(i) => i
            .map
            .iter()
            .filter(|(k, _)| k.0 == oh)
            .map(|(k, s)| (k.1, k.2, *s))
            .collect(),
        Err(_) => return,
    };
    if olds.is_empty() {
        return;
    }
    let mut copied: Vec<(Key, Slot)> = Vec::new();
    {
        let mut d = match dat().lock() {
            Ok(d) => d,
            Err(_) => return,
        };
        let mut pos = d.pos;
        if let Some(f) = d.file.as_mut() {
            for (bw, bh, slot) in olds {
                if slot.len == 0 {
                    // 只有指纹、没有缩略图的那条也要跟着改名，不然新路径搜不到。
                    copied.push((
                        (nh, bw, bh),
                        Slot {
                            mtime: st.mtime,
                            size: st.size,
                            off: 0,
                            len: 0,
                            fp: slot.fp,
                        },
                    ));
                    continue;
                }
                let mut jpg = vec![0u8; slot.len as usize];
                if f.seek(SeekFrom::Start(slot.off as u64)).is_err() {
                    continue;
                }
                if f.read_exact(&mut jpg).is_err() {
                    continue;
                }
                let off = pos;
                if off.checked_add(jpg.len() as u64).unwrap_or(u64::MAX) > u32::MAX as u64 {
                    break;
                }
                if f.write_all(&jpg).is_err() {
                    // 半途失败时真实 EOF 已越过 pos（append 的写永远落在文件尾），
                    // 先对齐再退出，别让后面每条记录都指向残骸。
                    if let Ok(m) = f.metadata() {
                        pos = m.len();
                    }
                    break;
                }
                copied.push((
                    (nh, bw, bh),
                    Slot {
                        mtime: st.mtime,
                        size: st.size,
                        off: off as u32,
                        len: jpg.len() as u32,
                        fp: slot.fp,
                    },
                ));
                pos = off + jpg.len() as u64;
            }
        }
        d.pos = pos;
    }
    if copied.is_empty() {
        return;
    }
    // 改了名的这张要能在"全部重建"的名单里找得到。
    note_path(new);
    if let Ok(mut i) = idx().lock() {
        for (k, slot) in copied {
            i.map.insert(k, slot.clone());
            if let Some(f) = i.file.as_mut() {
                let _ = f.write_all(&rec_bytes(&k, &slot));
            }
        }
    }
}

pub fn store(path: &str, bw: u32, bh: u32, img: &Rgba) {
    store_fp(path, bw, bh, img, None);
}

/// 出图时顺手把指纹一起写进这条记录。指纹是这一档缩略图算出来的，
/// 所以它就该和这张缩略图同生共死，不再另写一份表。
pub fn store_fp(path: &str, bw: u32, bh: u32, img: &Rgba, fp: Option<Fp>) {
    let Some(jpg) = encode(img) else { return };
    put_at(path, bw, bh, img, jpg, fp, false);
}

/// 重建那条线要用：同一个键不能"已经存过就跳过"，必须拿新解出来的那张盖上去。
/// 索引只追加，同键留两条时后写的胜出，下次启动读到的也是新的这条。
pub fn replace(path: &str, bw: u32, bh: u32, img: &Rgba) {
    replace_fp(path, bw, bh, img, None);
}

pub fn replace_fp(path: &str, bw: u32, bh: u32, img: &Rgba, fp: Option<Fp>) {
    let Some(jpg) = encode(img) else { return };
    put_at(path, bw, bh, img, jpg, fp, true);
}

/// 出图线专用：JPEG 字节已经在手（刚编出来准备算指纹的那串），直接用，不再编第二遍。
/// 缩略图和它的指纹进同一条索引记录，一次落盘。
pub fn store_jpg(path: &str, bw: u32, bh: u32, img: &Rgba, jpg: Vec<u8>, fp: Option<Fp>) {
    put_at(path, bw, bh, img, jpg, fp, false);
}

pub fn replace_jpg(path: &str, bw: u32, bh: u32, img: &Rgba, jpg: Vec<u8>, fp: Option<Fp>) {
    put_at(path, bw, bh, img, jpg, fp, true);
}

fn put_at(path: &str, bw: u32, bh: u32, img: &Rgba, jpg: Vec<u8>, fp: Option<Fp>, force: bool) {
    let Some((st, key)) = stamp_key(path, bw, bh, img) else {
        return;
    };
    if !force && fp.is_none() {
        if let Ok(i) = idx().lock() {
            if i.map
                .get(&key)
                .is_some_and(|s| s.mtime == st.mtime && s.size == st.size && s.len > 0)
            {
                return;
            }
        }
    }
    put(path, st, key, jpg, fp)
}

fn stamp_key(path: &str, bw: u32, bh: u32, img: &Rgba) -> Option<(Stamp, Key)> {
    if !enabled() || img.w > u16::MAX as u32 || img.h > u16::MAX as u32 {
        return None;
    }
    let st = stamp(path)?;
    Some((st, (fnv(path), bw as u16, bh as u16)))
}

/// 把一张位图编成缓存里那种 JPEG 字节。出图线先编一次拿到手，指纹从这串字节算、
/// 落盘也落这串字节——同一份像素，不用写完再从盘上读回来解一遍。
pub fn encode(img: &Rgba) -> Option<Vec<u8>> {
    // image 的 JPEG 编码器只收 L8/Rgb8（给 Rgba8 直接回 Unsupported），
    // 缩略图本来全不透明，所以扔掉 alpha 再编。
    let mut rgb = Vec::with_capacity((img.w as usize) * (img.h as usize) * 3);
    for p in img.px.chunks_exact(4) {
        rgb.extend_from_slice(&p[..3]);
    }
    let mut jpg = Vec::new();
    {
        let mut enc = JpegEncoder::new_with_quality(&mut jpg, 75);
        if enc.encode(&rgb, img.w, img.h, ExtendedColorType::Rgb8).is_err() {
            return None;
        }
    }
    if jpg.is_empty() || jpg.len() as u32 > MAX_REC_LEN {
        return None;
    }
    Some(jpg)
}

/// 缓存里那种 JPEG 字节解回紧凑 RGBA。和从盘上读回来那条用的是同一个解码器。
pub fn decode(jpg: &[u8]) -> Option<Rgba> {
    to_rgba(jpg)
}

fn put(path: &str, st: Stamp, key: Key, jpg: Vec<u8>, fp: Option<Fp>) {
    note_path(path);
    let slot = {
        let mut d = match dat().lock() {
            Ok(d) => d,
            Err(_) => return,
        };
        let off = d.pos;
        if off.checked_add(jpg.len() as u64).unwrap_or(u64::MAX) > u32::MAX as u64 {
            return;
        }
        let mut wrote = false;
        let mut sync_pos: Option<u64> = None;
        if let Some(f) = d.file.as_mut() {
            wrote = f.write_all(&jpg).is_ok();
            if !wrote {
                // 半途失败（典型是盘满）时真实 EOF 已经越过 pos；append 句柄的写
                // 永远落在真实文件尾，不同步回去，后面每条记录的 off 都指向残骸。
                if let Ok(m) = f.metadata() {
                    sync_pos = Some(m.len());
                }
            }
        }
        if let Some(p) = sync_pos {
            d.pos = p;
        }
        if !wrote {
            return;
        }
        d.pos = off + jpg.len() as u64;
        Slot {
            mtime: st.mtime,
            size: st.size,
            off: off as u32,
            len: jpg.len() as u32,
            fp,
        }
    };
    if let Ok(mut i) = idx().lock() {
        i.map.insert(key, slot.clone());
        if let Some(f) = i.file.as_mut() {
            let _ = f.write_all(&rec_bytes(&key, &slot));
        }
    }
}

/// 给测量台报账用：有缩略图的条数 + 数据段字节（只剩指纹的记录不算缩略图）。
pub fn stats() -> (usize, u64) {
    let n = idx()
        .lock()
        .map(|i| i.map.values().filter(|s| s.len > 0).count())
        .unwrap_or(0);
    let p = cache_dir().join(DAT_FILE);
    let bytes = p.metadata().map(|m| m.len()).unwrap_or(0);
    (n, bytes)
}

/// 把一枚指纹挂到已有的索引键上。缩略图指针原样留着——指纹和缩略图是一条记录，
/// 这一步只补那 40 个字节。原图时间戳对不上就不硬挂：旧指纹配新图会认错。
/// 排除规则命中的路一律不挂：贴图的缩略图照常缓存，但不进搜图库。
pub fn attach_fp(path: &str, bw: u32, bh: u32, fp: Fp) {
    if !enabled() || scan::excluded(path) || bw > u16::MAX as u32 || bh > u16::MAX as u32 {
        return;
    }
    let Some(st) = stamp(path) else { return };
    let key = (fnv(path), bw as u16, bh as u16);
    let Ok(mut i) = idx().lock() else { return };
    let old = i.map.get(&key).copied();
    let (off, len, old_fp) = match old {
        Some(s) if s.mtime == st.mtime && s.size == st.size => (s.off, s.len, s.fp),
        _ => (0, 0, None),
    };
    if old_fp == Some(fp) {
        return;
    }
    let slot = Slot {
        mtime: st.mtime,
        size: st.size,
        off,
        len,
        fp: Some(fp),
    };
    i.map.insert(key, slot);
    if let Some(f) = i.file.as_mut() {
        let _ = f.write_all(&rec_bytes(&key, &slot));
    }
}

/// 按索引键写指纹。补库那条线用：候选已经按原图时间戳筛过，这里不再碰文件。
/// 返回 true 表示表里原来没这枚指纹、这次真写进去了。
pub fn put_fp(key: Key, fp: Fp) -> bool {
    if !enabled() {
        return false;
    }
    let Ok(mut i) = idx().lock() else { return false };
    let Some(s) = i.map.get(&key).copied() else {
        return false;
    };
    if s.fp == Some(fp) {
        return false;
    }
    let slot = Slot { fp: Some(fp), ..s };
    i.map.insert(key, slot);
    if let Some(f) = i.file.as_mut() {
        if f.write_all(&rec_bytes(&key, &slot)).is_err() {
            return false;
        }
    }
    true
}

/// 全库指纹快照给相似库建内存表：一条记录一枚指纹，带档位和原图时间戳。
pub fn fps() -> Vec<(u64, u16, u16, u32, u64, Fp)> {
    let Ok(i) = idx().lock() else {
        return Vec::new();
    };
    i.map
        .iter()
        .filter_map(|(k, s)| s.fp.map(|fp| (k.0, k.1, k.2, s.mtime, s.size, fp)))
        .collect()
}

/// 这一档里"缩略图在、指纹还缺"的那些，补库用。返回 (路径, 键, 原图时间戳, 长度)。
/// 原图已经被改过或删掉的跳过：补出来的指纹得对得上现在这张图才有意义。
/// `prefix` 有值就只要这个文件夹底下的。
pub fn missing_fps(bw: u32, bh: u32, prefix: Option<&str>) -> Vec<(String, Key, u32, u64)> {
    if !enabled() || bw > u16::MAX as u32 || bh > u16::MAX as u32 {
        return Vec::new();
    }
    let cand: Vec<(Key, u32, u64)> = match idx().lock() {
        Ok(i) => i
            .map
            .iter()
            .filter(|(k, s)| {
                k.1 == bw as u16 && k.2 == bh as u16 && s.len > 0 && s.fp.is_none()
            })
            .map(|(k, s)| (*k, s.mtime, s.size))
            .collect(),
        Err(_) => return Vec::new(),
    };
    let paths = path_map();
    let mut out = Vec::with_capacity(cand.len());
    for (key, mtime, size) in cand {
        let Some(p) = paths.get(&key.0) else { continue };
        if let Some(pre) = prefix {
            if !p.to_ascii_lowercase().starts_with(pre) {
                continue;
            }
        }
        let Some(st) = stamp(p) else { continue };
        if st.mtime != mtime || st.size != size {
            continue;
        }
        out.push((p.clone(), key, mtime, size));
    }
    out
}

/// 补库时按索引键直接读那张缩略图，不用路径、不用再 stat 原图。
pub fn thumb_at(key: Key) -> Option<Rgba> {
    if !enabled() {
        return None;
    }
    let slot = {
        let i = idx().lock().ok()?;
        *i.map.get(&key)?
    };
    if slot.len == 0 || slot.len > MAX_REC_LEN {
        return None;
    }
    let mut jpg = vec![0u8; slot.len as usize];
    {
        let mut d = dat().lock().ok()?;
        let f = d.file.as_mut()?;
        if f.seek(SeekFrom::Start(slot.off as u64)).is_err() || f.read_exact(&mut jpg).is_err() {
            return None;
        }
    }
    to_rgba(&jpg)
}

/// 把一批指纹并进索引，旧的 sim1.rec 迁移用。有缩略图且时间戳对得上的，
/// 缩略图指针照抄；只有指纹、缩略图那一档没缓存的，记一条长度 0 的记录。
/// 原图已经改过/不在盘上的行直接丢掉——旧指纹配新图比没有更坏。
/// 返回 true 表示整批已经落盘，可以删旧文件了。
pub fn import_fps(rows: &[(String, u32, u32, u32, u64, Fp)]) -> bool {
    if !enabled() || rows.is_empty() {
        return true;
    }
    let mut imported: Vec<String> = Vec::new();
    let ok = {
        let Ok(mut i) = idx().lock() else { return false };
        let mut buf = Vec::new();
        for (path, bw, bh, mtime, size, fp) in rows {
            if *bw > u16::MAX as u32 || *bh > u16::MAX as u32 || scan::excluded(path) {
                continue;
            }
            let key = (fnv(path), *bw as u16, *bh as u16);
            let (off, len) = match i.map.get(&key).copied() {
                Some(s) if s.mtime == *mtime && s.size == *size => (s.off, s.len),
                _ => {
                    let Some(st) = stamp(path) else { continue };
                    if st.mtime != *mtime || st.size != *size {
                        continue;
                    }
                    (0, 0)
                }
            };
            let slot = Slot {
                mtime: *mtime,
                size: *size,
                off,
                len,
                fp: Some(*fp),
            };
            i.map.insert(key, slot);
            buf.extend_from_slice(&rec_bytes(&key, &slot));
            imported.push(path.clone());
        }
        if buf.is_empty() {
            true
        } else {
            let Some(f) = i.file.as_mut() else {
                return false;
            };
            if f.write_all(&buf).is_err() {
                return false;
            }
            // 迁移要删源文件，先确认这批量真落盘了。
            f.sync_all().is_ok()
        }
    };
    // 索引只存哈希，迁移进来的路径得在名字名单里有一笔，相似结果才显示得出来。
    for p in &imported {
        note_path(p);
    }
    ok
}

/// 把这些键的指纹摘掉，缩略图本身留着。清理死路径时用，重写一次索引顺便压实。
pub fn clear_fps(keys: &[Key]) -> usize {
    if !enabled() || keys.is_empty() {
        return 0;
    }
    let Ok(mut i) = idx().lock() else { return 0 };
    let mut n = 0usize;
    for key in keys {
        let Some(s) = i.map.get(key).copied() else {
            continue;
        };
        if s.fp.is_none() {
            continue;
        }
        if s.len == 0 {
            i.map.remove(key);
        } else {
            i.map.insert(*key, Slot { fp: None, ..s });
        }
        n += 1;
    }
    if n > 0 {
        rewrite(&mut i);
    }
    n
}

/// 缓存这条线现在通不通。关着的时候相似库也不该写，免得测量台的历史污染真库。
pub fn on() -> bool {
    enabled()
}

/// 看过的图的名字，一行一个，只追加不删。存一条缩略图就顺手记一笔，
/// 同一张图被不同盒尺寸存过会留好几行，取的时候去重。
fn names() -> &'static Mutex<Option<File>> {
    static N: OnceLock<Mutex<Option<File>>> = OnceLock::new();
    N.get_or_init(|| {
        Mutex::new(
            enabled()
                .then(|| {
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(cache_dir().join(NAME_FILE))
                })
                .and_then(|r| r.ok()),
        )
    })
}

fn note_path(path: &str) {
    if !enabled() || path.is_empty() {
        return;
    }
    if let Ok(mut f) = names().lock() {
        if let Some(f) = f.as_mut() {
            let _ = writeln!(f, "{path}");
        }
    }
    // 哈希对照表只在相似库问过之后才在内存里；没初始化就别为写一行名字把几 MB 读起来。
    if let Some(m) = PATHS.get() {
        if let Ok(mut m) = m.lock() {
            m.insert(fnv(path), path.to_string());
        }
    }
}

static PATHS: OnceLock<Mutex<HashMap<u64, String>>> = OnceLock::new();

/// 索引只存路径哈希，指纹结果要显示路径，所以从名字名单建一份对照。
/// 名单里同一张图可能有很多行（每换一个档就记一笔），后读到的覆盖先读到的，无所谓。
fn paths() -> &'static Mutex<HashMap<u64, String>> {
    PATHS.get_or_init(|| {
        let mut m = HashMap::new();
        if enabled() {
            if let Ok(all) = std::fs::read(cache_dir().join(NAME_FILE)) {
                for line in all.split(|b| *b == b'\n') {
                    if line.is_empty() {
                        continue;
                    }
                    let p = String::from_utf8_lossy(line).into_owned();
                    m.insert(fnv(&p), p);
                }
            }
        }
        Mutex::new(m)
    })
}

/// 哈希 → 路径的一份快照。相似库启动时照着它把索引里的指纹配回路径。
pub fn path_map() -> HashMap<u64, String> {
    paths().lock().map(|m| m.clone()).unwrap_or_default()
}

/// 名单上还没作废的那些名字。整座缩略图重做靠的就是这份名单——索引本身给不出路径。
/// 一条路径占几十字节，四万张也就几 MB，只在点"全部重建"时才读一遍。
pub fn known_paths() -> Vec<String> {
    if !enabled() {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    let Some(all) = std::fs::read(cache_dir().join(NAME_FILE)).ok() else {
        return out;
    };
    let mut seen = std::collections::HashSet::new();
    for line in all.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let p = String::from_utf8_lossy(line).into_owned();
        if seen.insert(p.to_ascii_lowercase()) {
            out.push(p);
        }
    }
    out.sort_by_key(|p| p.to_ascii_lowercase());
    out
}

/// 解不开名单：下次滚回这一屏，别再花两百毫秒重演一遍同样的失败。
/// 键只到文件本身，原图 mtime+长度一变就自动作废；名单只在缓存开着时生效，
/// 测量台关掉缓存就不会被历史结果污染。
fn bad() -> &'static Mutex<HashMap<u64, Stamp>> {
    static B: OnceLock<Mutex<HashMap<u64, Stamp>>> = OnceLock::new();
    B.get_or_init(|| {
        let mut m = HashMap::new();
        if enabled() {
            let p = cache_dir().join(BAD_FILE);
            if let Ok(mut f) = File::open(&p) {
                let sz = f.metadata().map(|x| x.len()).unwrap_or(0);
                if sz > MAX_LIST_BYTES {
                    if let Ok(f2) = OpenOptions::new().write(true).open(&p) {
                        let _ = f2.set_len(0);
                    }
                } else {
                    let mut all = Vec::new();
                    let _ = f.read_to_end(&mut all);
                    let full = all.len() / BADREC * BADREC;
                    if all.len() != full {
                        // 同样截掉崩溃留下的半条，否则后面的记录全部错位。
                        if let Ok(f2) = OpenOptions::new().write(true).open(&p) {
                            let _ = f2.set_len(full as u64);
                        }
                    }
                    for r in all[..full].chunks_exact(BADREC) {
                        m.insert(
                            u64::from_le_bytes(r[0..8].try_into().unwrap()),
                            Stamp {
                                mtime: u32::from_le_bytes(r[8..12].try_into().unwrap()),
                                size: u64::from_le_bytes(r[12..20].try_into().unwrap()),
                            },
                        );
                    }
                }
            }
        }
        Mutex::new(m)
    })
}

pub fn failed(path: &str) -> bool {
    if !enabled() {
        return false;
    }
    let Some(st) = stamp(path) else {
        return false;
    };
    bad()
        .lock()
        .map(|m| m.get(&fnv(path)) == Some(&st))
        .unwrap_or(false)
}

pub fn note_fail(path: &str) {
    if !enabled() {
        return;
    }
    let Some(st) = stamp(path) else {
        return;
    };
    let h = fnv(path);
    {
        let mut m = match bad().lock() {
            Ok(m) => m,
            Err(_) => return,
        };
        if m.get(&h) == Some(&st) {
            return;
        }
        m.insert(h, st);
    }
    let mut b = [0u8; BADREC];
    b[0..8].copy_from_slice(&h.to_le_bytes());
    b[8..12].copy_from_slice(&st.mtime.to_le_bytes());
    b[12..20].copy_from_slice(&st.size.to_le_bytes());
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(cache_dir().join(BAD_FILE))
    {
        let _ = f.write_all(&b);
    }
}

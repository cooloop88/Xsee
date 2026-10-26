use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use crate::Rgba;
use crate::cache;
use crate::scan;

/// 相似图片的指纹库。指纹不再单独记账：缩略图索引每条记录后面就跟一枚指纹，
/// 同一档缩略图生成/命中时一起写，搬文件时跟着搬。
/// 这个模块只保留怎么算、怎么比，以及一份内存表让查找不必每次扫索引。
/// 这个文件名是上一版的指纹表，只在升级后第一次打开相似库时导进来，导完删掉。
const REC_FILE: &str = "sim1.rec";
/// 指纹只从够大的缩略图上取。列表档那几十像素拉成 32×32 已经糊成一团，
/// 找相似要用它就得先把视图切大。界面挡一下，别让人白点一次。
pub const MIN_SIDE: u32 = 96;
/// 允许的最大不同位数（总共 319 位）。实测同一档内两张不同的图最小也差到 57 位，
/// 所以 40 这一档只会漏不会认错。
pub const MAX_DIST: u32 = 40;
/// 一次最多回报这么多张，够翻两页。
pub const MAX_HITS: usize = 240;

/// dHash 那张小图的形状：17 列比出 16 个位，16 行一共 256 位。
const DH_W: usize = 17;
const DH_H: usize = 16;
/// DCT 那张小图的边长，取左上 8×8 系数。
const DN: usize = 32;

/// 指纹的位布局归缩略图库管——它和缩略图一条记录，存储层得知道这 40 个字节怎么摆。
pub use crate::cache::Fp;

#[derive(Clone)]
struct Rec {
    /// 盘上那套大小写原样留着：结果视图和分类库都要拿它当路径用，
    ///  map 的键只是查得快的小写。
    path: String,
    mtime: u32,
    size: u64,
    fp: Fp,
}

/// 一档一张内存表。指纹只有同档才可比：同一张图从 128 档和 512 档各取一次指纹，
/// 实测能差到一百多位（比两张不同的图还远），混在一块儿比就是乱认。
/// 所以窗口改了缩略图高度（那一对 −/+ 或 Ctrl+滚轮），那一档的相似库等于重新开始——
/// 和缩略图缓存按盒尺寸分键是同一件事。
struct Lib {
    maps: HashMap<(u16, u16), HashMap<String, Rec>>,
}

static LIB: OnceLock<Mutex<Lib>> = OnceLock::new();

fn stamp(path: &str) -> Option<(u32, u64)> {
    let m = std::fs::metadata(path).ok()?;
    let secs = m
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((secs.min(u32::MAX as u64) as u32, m.len()))
}

/// 上一版的指纹表按行记着路径和指纹；一次性并进缩略图索引后就删掉。
/// 原图已经改过或不在盘上的行不导——旧指纹配新图比没有更坏，这个检查在
/// `cache::import_fps` 里统一做。落盘失败就留着文件，下次再试。
fn import_old() {
    if !cache::on() {
        return;
    }
    let d = cache::dir();
    if d.as_os_str().is_empty() {
        return;
    }
    let p = d.join(REC_FILE);
    let Ok(all) = std::fs::read(&p) else {
        return;
    };
    let mut rows = Vec::new();
    for line in all.split(|b| *b == b'\n') {
        if let Some((bx, _, r)) = parse(line) {
            rows.push((r.path, bx.0 as u32, bx.1 as u32, r.mtime, r.size, r.fp));
        }
    }
    if cache::import_fps(&rows) {
        let _ = std::fs::remove_file(&p);
    }
}

fn lib() -> &'static Mutex<Lib> {
    LIB.get_or_init(|| {
        import_old();
        let mut maps: HashMap<(u16, u16), HashMap<String, Rec>> = HashMap::new();
        let paths = cache::path_map();
        for (h, bw, bh, mtime, size, fp) in cache::fps() {
            let Some(p) = paths.get(&h) else { continue };
            maps.entry((bw, bh)).or_default().insert(
                p.to_ascii_lowercase(),
                Rec {
                    path: p.clone(),
                    mtime,
                    size,
                    fp,
                },
            );
        }
        Mutex::new(Lib { maps })
    })
}

/// 一行拆成 (盒尺寸, 小写路径, 记录)；旧指纹表的行格式，只在迁移时用。
/// 字段不齐或数字解不开就丢这一行，别牵连其余。
fn parse(line: &[u8]) -> Option<((u16, u16), String, Rec)> {
    let s = std::str::from_utf8(line).ok()?;
    let mut f = s.split('\t');
    let (bw, bh, mtime, size, d, c, path) = (
        f.next()?,
        f.next()?,
        f.next()?,
        f.next()?,
        f.next()?,
        f.next()?,
        f.next()?,
    );
    if d.len() != 64 || c.len() != 16 {
        return None;
    }
    let mut bits = [0u64; 4];
    for (i, w) in bits.iter_mut().enumerate() {
        *w = u64::from_str_radix(&d[i * 16..i * 16 + 16], 16).ok()?;
    }
    Some((
        (bw.parse().ok()?, bh.parse().ok()?),
        path.to_ascii_lowercase(),
        Rec {
            path: path.to_string(),
            mtime: mtime.parse().ok()?,
            size: size.parse().ok()?,
            fp: Fp {
                d: bits,
                c: u64::from_str_radix(c, 16).ok()?,
            },
        },
    ))
}

/// 灰度 + 盒式平均重采样到指定尺寸。面积平均比双线性抗噪，
/// 指纹要的是整体明暗结构，不是细节。
fn gray_stretch(img: &Rgba, tw: usize, th: usize) -> Vec<u8> {
    let (sw, sh) = (img.w as usize, img.h as usize);
    let mut out = vec![0u8; tw * th];
    if sw == 0 || sh == 0 {
        return out;
    }
    for y in 0..th {
        let y0 = y * sh / th;
        let y1 = ((y + 1) * sh + th - 1).div_ceil(th).clamp(y0 + 1, sh);
        for x in 0..tw {
            let x0 = x * sw / tw;
            let x1 = ((x + 1) * sw + tw - 1).div_ceil(tw).clamp(x0 + 1, sw);
            let (mut sum, mut n) = (0u32, 0u32);
            for py in y0..y1 {
                let row = py * sw;
                for px in x0..x1 {
                    let o = (row + px) * 4;
                    if o + 2 >= img.px.len() {
                        continue;
                    }
                    sum += (img.px[o] as u32 * 77
                        + img.px[o + 1] as u32 * 150
                        + img.px[o + 2] as u32 * 28)
                        >> 8;
                    n += 1;
                }
            }
            out[y * tw + x] = (sum / n.max(1)) as u8;
        }
    }
    out
}

/// 每行相邻两列比大小：16 行 × 16 位 = 256 位。管的是画面的骨架。
fn dhash_of(g: &[u8]) -> [u64; 4] {
    let mut bits = [0u64; 4];
    for y in 0..DH_H {
        for x in 0..DH_W - 1 {
            if g[y * DH_W + x] > g[y * DH_W + x + 1] {
                let i = y * (DH_W - 1) + x;
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }
    }
    bits
}

fn cos_table() -> &'static [[f32; DN]; 8] {
    static T: OnceLock<[[f32; DN]; 8]> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = [[0f32; DN]; 8];
        for u in 0..8 {
            for x in 0..DN {
                t[u][x] = (((2 * x + 1) as f32) * u as f32 * std::f32::consts::PI / 64.0).cos();
            }
        }
        t
    })
}

/// 32×32 灰度做 DCT，只算左上 8×8 那些系数，逐位和"交流分量的平均"比大小。
/// 直流那一位不参与——它只反映整张图的明暗，剪影换了也可能一样亮。
/// 这一半管的是内容和色调，比 dHash 抗轻微形变。
fn dct_of(img: &Rgba) -> u64 {
    let g = gray_stretch(img, DN, DN);
    let t = cos_table();
    let mut tmp = [[0f32; 8]; DN];
    for y in 0..DN {
        let row = &g[y * DN..y * DN + DN];
        for v in 0..8 {
            let mut s = 0f32;
            for x in 0..DN {
                s += row[x] as f32 * t[v][x];
            }
            tmp[y][v] = s;
        }
    }
    let mut f = [[0f32; 8]; 8];
    for u in 0..8 {
        for v in 0..8 {
            let mut s = 0f32;
            for y in 0..DN {
                s += tmp[y][v] * t[u][y];
            }
            f[u][v] = s;
        }
    }
    let mut sum = 0f64;
    for u in 0..8 {
        for v in 0..8 {
            if u + v > 0 {
                sum += f[u][v] as f64;
            }
        }
    }
    let mean = (sum / 63.0) as f32;
    let mut bits = 0u64;
    for u in 0..8 {
        for v in 0..8 {
            if u + v == 0 {
                continue;
            }
            if f[u][v] > mean {
                bits |= 1u64 << (u * 8 + v - 1);
            }
        }
    }
    bits
}

/// 一张现成的 RGBA 位图算指纹。
pub fn fingerprint(img: &Rgba) -> Fp {
    Fp {
        d: dhash_of(&gray_stretch(img, DH_W, DH_H)),
        c: dct_of(img),
    }
}

/// 两张图差多少位（一共 319 位）。对拍台也用它。
pub fn distance(a: &Fp, b: &Fp) -> u32 {
    let mut n = (a.c ^ b.c).count_ones();
    for i in 0..4 {
        n += (a.d[i] ^ b.d[i]).count_ones();
    }
    n
}

/// 这一档的指纹够不够格入库。盒太小解不出可靠的位，也塞不进 u16 的键。
pub fn usable(bw: u32, bh: u32) -> bool {
    bw.min(bh) >= MIN_SIDE && bw <= u16::MAX as u32 && bh <= u16::MAX as u32
}

/// 这一条是不是已经原样记过了。记过就别再往文件上堆一行。
fn have(k: &str, bx: (u16, u16), st: (u32, u64), fp: Fp) -> bool {
    let Ok(l) = lib().lock() else { return false };
    l.maps
        .get(&bx)
        .and_then(|m| m.get(k))
        .is_some_and(|r| r.fp == fp && r.mtime == st.0 && r.size == st.1)
}

/// 只补内存表。盘上那条记录由缩略图库负责写。
fn remember(path: &str, bx: (u16, u16), st: (u32, u64), fp: Fp) {
    let k = path.to_ascii_lowercase();
    if let Ok(mut l) = lib().lock() {
        l.maps.entry(bx).or_default().insert(
            k,
            Rec {
                path: path.to_string(),
                mtime: st.0,
                size: st.1,
                fp,
            },
        );
    }
}

/// 出完一张缩略图顺手记一笔，键上这一档的盒尺寸。刚 store 的走 `adopt`，
/// 这条是给"缓存命中了、那条记录里还没有指纹"补的。
/// 相似库没打开过就只写盘、不建内存表：浏览缩略图不该为此等一次全库加载。
pub fn note(path: &str, bw: u32, bh: u32, img: &Rgba) {
    if !cache::on() || path.is_empty() || !usable(bw, bh) || scan::excluded(path) {
        return;
    }
    let Some(st) = stamp(path) else { return };
    let bx = (bw as u16, bh as u16);
    let fp = fingerprint(img);
    let k = path.to_ascii_lowercase();
    let open = LIB.get().is_some();
    if open && have(&k, bx, st, fp) {
        return;
    }
    cache::attach_fp(path, bw, bh, fp);
    if open {
        remember(path, bx, st, fp);
    }
}

/// 刚随缩略图写进索引的那枚指纹：盘上已经有了，只把内存表补上。
/// 相似库还没加载过就先不动——加载时从索引里一次读全。
pub fn adopt(path: &str, bw: u32, bh: u32, fp: Option<Fp>) {
    let Some(fp) = fp else { return };
    if !cache::on() || path.is_empty() || !usable(bw, bh) || LIB.get().is_none() || scan::excluded(path)
    {
        return;
    }
    let Some(st) = stamp(path) else { return };
    remember(path, (bw as u16, bh as u16), st, fp);
}

/// 指纹一律从缓存里那张 JPEG 解回来算。入库和查询必须吃同一份像素，
/// 不然纯色/平坦图会被编码噪声推成两张"不同"的图，自己搜不到自己。
pub fn fp_from_cache(path: &str, bw: u32, bh: u32) -> Option<Fp> {
    if !cache::on() || !usable(bw, bh) {
        return None;
    }
    Some(fingerprint(&cache::peek(path, bw, bh)?))
}

/// 把相似库读进内存，第一次顺带把旧的指纹表并进索引。补库要赶在挑
/// "缺指纹"之前调：不然刚算出来的那批会被随后导入的旧记录顶掉。
pub fn ensure_loaded() {
    let _ = lib();
}

/// 这一档里缩略图已在、指纹还缺的那些，从缩略图缓存里现成那张取图补上。
/// 全程不开原图。返回补上的条数；`prog` 每处理一张候选加一。
/// 候选由缩略图库直接扫索引给出，不再逐行读路径名单、逐张 stat 原图两遍。
pub fn catch_up(
    prefix: Option<&str>,
    bw: u32,
    bh: u32,
    prog: &AtomicUsize,
    stop: &AtomicBool,
) -> usize {
    if !cache::on() || !usable(bw, bh) {
        return 0;
    }
    ensure_loaded();
    let todo = cache::missing_fps(bw, bh, prefix);
    fill(todo, bw, bh, prog, stop)
}

/// 补库的执行段。界面要一个总数做进度条，可以先拿 `cache::missing_fps`
/// 算好再交回来，免得扫两遍。
pub fn fill(
    todo: Vec<(String, cache::Key, u32, u64)>,
    bw: u32,
    bh: u32,
    prog: &AtomicUsize,
    stop: &AtomicBool,
) -> usize {
    if !cache::on() || !usable(bw, bh) {
        return 0;
    }
    let bx = (bw as u16, bh as u16);
    let mut made = 0usize;
    for (p, key, mtime, size) in todo {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        // 排除规则命中的不补：贴图不入账，就算缩略图还在也不进搜图库。
        if scan::excluded(&p) {
            continue;
        }
        prog.fetch_add(1, Ordering::Relaxed);
        let Some(img) = cache::thumb_at(key) else { continue };
        let fp = fingerprint(&img);
        if !cache::put_fp(key, fp) {
            continue;
        }
        remember(&p, bx, (mtime, size), fp);
        made += 1;
    }
    made
}

/// 对拍台用：这一档库里现有的 (路径, 指纹)。界面不走这条。
pub fn entries(bw: u32, bh: u32) -> Vec<(String, Fp)> {
    if !usable(bw, bh) {
        return Vec::new();
    }
    let Ok(l) = lib().lock() else { return Vec::new() };
    match l.maps.get(&(bw as u16, bh as u16)) {
        Some(m) => m.values().map(|r| (r.path.clone(), r.fp)).collect(),
        None => Vec::new(),
    }
}

/// 和这张指纹对得上的那些，只在这一档里比。按差异从小到大，够不上阈值的不进来。
pub fn similar(bw: u32, bh: u32, q: &Fp, limit: usize) -> Vec<String> {
    if !usable(bw, bh) {
        return Vec::new();
    }
    let Ok(l) = lib().lock() else { return Vec::new() };
    let Some(m) = l.maps.get(&(bw as u16, bh as u16)) else {
        return Vec::new();
    };
    let mut hits: Vec<(u32, String)> = m
        .iter()
        .filter_map(|(_, r)| {
            let d = distance(q, &r.fp);
            (d <= MAX_DIST).then(|| (d, r.path.clone()))
        })
        .collect();
    hits.sort_by_key(|(d, p)| (*d, p.to_ascii_lowercase()));
    hits.truncate(limit);
    hits.into_iter().map(|(_, k)| k).collect()
}

/// 指纹两半各自的位数。只比其中一半时，相似度得按那一半的总位数换算。
pub const D_BITS: u32 = 256;
pub const C_BITS: u32 = 63;
pub const ALL_BITS: u32 = D_BITS + C_BITS;
/// 匹配算法开关：只比 dHash、只比 DCT、或两个都比。
pub const ALGO_D: u8 = 1;
pub const ALGO_C: u8 = 2;
pub const ALGO_ALL: u8 = ALGO_D | ALGO_C;

/// 这一档算法一共比多少位——界面上"相似度 87%" 换成"允许差 40 位"用的分母。
pub fn bits_of(algo: u8) -> u32 {
    match algo & ALGO_ALL {
        ALGO_D => D_BITS,
        ALGO_C => C_BITS,
        _ => ALL_BITS,
    }
}

/// 只比选中那几段。两个都选时和 `distance` 完全等价。
pub fn distance_algo(a: &Fp, b: &Fp, algo: u8) -> u32 {
    let m = algo & ALGO_ALL;
    let mut n = 0u32;
    if m & ALGO_D != 0 {
        for i in 0..4 {
            n += (a.d[i] ^ b.d[i]).count_ones();
        }
    }
    if m & ALGO_C != 0 {
        n += (a.c ^ b.c).count_ones();
    }
    n
}

/// 相似度百分比换成"允许差几位"。100 就是要求一位不差。
pub fn dist_from_pct(pct: u32, algo: u8) -> u32 {
    let p = pct.clamp(0, 100);
    (((bits_of(algo) * (100 - p)) as f64) / 100.0).ceil() as u32
}

/// 百分比反过来：差 `d` 位算多少相似。
pub fn pct_from_dist(d: u32, algo: u8) -> u32 {
    let bits = bits_of(algo).max(1);
    if d >= bits {
        0
    } else {
        (100.0 - (d as f64 * 100.0 / bits as f64).ceil()) as u32
    }
}

/// 按方向搬一张图：1 左右镜像、2 上下颠倒、3 顺时针 90°、4 转 180°、5 逆时针 90°。
/// 搬完照旧走原来的重采样，所以转了 90° 之后 dHash 仍然是 17×16，位数不变、不用另算一套。
fn oriented(img: &Rgba, m: u8) -> Rgba {
    let (w, h) = (img.w as usize, img.h as usize);
    let swap = m == 3 || m == 5;
    let (ow, oh) = if swap { (h, w) } else { (w, h) };
    let mut px = vec![0u8; ow * oh * 4];
    for y in 0..h {
        for x in 0..w {
            let s = (y * w + x) * 4;
            // 目标坐标：镜像只翻一个轴，转置把 x/y 对调，180 两个轴一起翻。
            let (dx, dy) = match m {
                1 => (w - 1 - x, y),
                2 => (x, h - 1 - y),
                3 => (h - 1 - y, x),
                4 => (w - 1 - x, h - 1 - y),
                5 => (y, w - 1 - x),
                _ => (x, y),
            };
            let d = (dy * ow + dx) * 4;
            px[d..d + 4].copy_from_slice(&img.px[s..s + 4]);
        }
    }
    Rgba { w: ow as u32, h: oh as u32, px }
}

/// 查询图要算的几份指纹。方向和参考软件那一套对齐：正的一律算，
/// 勾选了才多算旋转的三份、翻转的两份。库里不多存任何东西——
/// 同图镜像实测差 100 位，把阈值放到那么宽会把不相干的整片捞进来，
/// 所以取"各方向里最像的那一个"，而不是放宽尺子。
pub fn variants(img: &Rgba, rot: bool, flip: bool) -> Vec<Fp> {
    let mut out = vec![fingerprint(img)];
    if rot {
        for m in [3u8, 4, 5] {
            out.push(fingerprint(&oriented(img, m)));
        }
    }
    if flip {
        for m in [1u8, 2] {
            out.push(fingerprint(&oriented(img, m)));
        }
    }
    out
}

/// 一次查找的完整口径：多个方向各算一份取最像的那个，按选中的算法段比，
/// 阈值由界面上的相似度换算来。返回 (差几位, 路径)，差异小的在前。
pub fn search(
    bw: u32,
    bh: u32,
    qs: &[Fp],
    algo: u8,
    max_dist: u32,
    limit: usize,
) -> Vec<(u32, String)> {
    if !usable(bw, bh) || qs.is_empty() {
        return Vec::new();
    }
    let Ok(l) = lib().lock() else { return Vec::new() };
    let Some(m) = l.maps.get(&(bw as u16, bh as u16)) else {
        return Vec::new();
    };
    let mut hits: Vec<(u32, String)> = m
        .iter()
        .filter_map(|(_, r)| {
            let d = qs.iter().map(|q| distance_algo(q, &r.fp, algo)).min().unwrap_or(u32::MAX);
            (d <= max_dist).then(|| (d, r.path.clone()))
        })
        .collect();
    hits.sort_by_key(|(d, p)| (*d, p.to_ascii_lowercase()));
    hits.truncate(limit);
    hits
}

/// 所有档位的合计：(档数, 总条数)。界面上"全部档位"那一行用它。
pub fn stats_all() -> (usize, usize) {
    let Ok(l) = lib().lock() else { return (0, 0) };
    (l.maps.len(), l.maps.values().map(|m| m.len()).sum())
}

/// 库里账最多的一档。相似库还没读进内存就是 None——界面线程用这个版本，
/// 别为了挑个档把整库读进来；后台先 ensure_loaded 再问就有数了。
pub fn biggest_tier_loaded() -> Option<(u16, u16)> {
    LIB.get()?
        .lock()
        .ok()?
        .maps
        .iter()
        .max_by_key(|(_, m)| m.len())
        .map(|(k, _)| *k)
}

/// 搜图用哪个档：界面线程能当场定的（缩略图视图的当前盒尺寸）就直接用；
/// 定不了的是小图标/列表/详细资料这些自己不建缩略图的档，得借库里账最多的一档，
/// 而账最多是哪一档要把库读进内存才知道。所以这条只许在后台线程调——
/// 界面线程问到空就退回去了，没人会再去读库，小图标档那一页会永远说库是空的。
pub fn resolve_tier(want: Option<(u32, u32)>) -> Option<(u32, u32)> {
    if let Some(t) = want {
        return Some(t);
    }
    ensure_loaded();
    biggest_tier_loaded().map(|(bw, bh)| (bw as u32, bh as u32))
}

/// 跨档搜索：查询图在哪几档有过缩略图，就各算一份指纹、各搜各的账——
/// 指纹只有同档才可比，所以查询图的指纹也得按那一档的缩略图算。
/// 结果按路径去重（同一张图在好几档都有账，取差异最小的那个），差异升序。
/// 一档现成缩略图都没有就返回空，由调用方兜底解码入档。纯内存比对加
/// 二十来次小 JPEG 解码，几十毫秒级。
pub fn search_all_tiers(
    q: &str,
    rot: bool,
    flip: bool,
    algo: u8,
    max_dist: u32,
    limit: usize,
) -> Vec<(u32, String)> {
    if !cache::on() {
        return Vec::new();
    }
    let tiers: Vec<(u16, u16)> = {
        let Ok(l) = lib().lock() else { return Vec::new() };
        l.maps.keys().copied().collect()
    };
    let mut merged: Vec<(u32, String)> = Vec::new();
    for (bw, bh) in tiers {
        let Some(img) = cache::peek(q, bw as u32, bh as u32) else {
            continue;
        };
        let qs = variants(&img, rot, flip);
        merged.extend(search(bw as u32, bh as u32, &qs, algo, max_dist, limit));
    }
    merged.sort_by(|a, b| {
        (a.0, a.1.to_ascii_lowercase()).cmp(&(b.0, b.1.to_ascii_lowercase()))
    });
    // 排序后相邻去重，留下的是差异最小的那一笔。
    merged.dedup_by(|a, b| a.1.eq_ignore_ascii_case(&b.1));
    merged
}

/// 四路并行问盘，返回没了的那些下标（升序）。exists 的耗时几乎全在慢盘上
/// （网络盘、虚拟盘一次几毫秒，五万次就是几十秒级），并行等的就是这段；
/// 本地盘暖机后一次几十微秒，四路照样有得赚。
fn dead_of(paths: &[String]) -> Vec<usize> {
    const W: usize = 4;
    let mut dead: Vec<usize> = std::thread::scope(|s| {
        let parts: Vec<_> = (0..W)
            .map(|w| {
                s.spawn(move || {
                    paths
                        .iter()
                        .skip(w)
                        .step_by(W)
                        .enumerate()
                        .filter(|(_, p)| !Path::new(p).exists())
                        .map(|(i, _)| i * W + w)
                        .collect::<Vec<usize>>()
                })
            })
            .collect();
        parts
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    dead.sort_unstable();
    dead
}

/// 把全部档位里盘上已经没有的那些一次剔掉。指纹现在长在缩略图索引上，所以只摘
/// 指纹、缩略图记录留着（文件回来还能命中，不用重解）；索引由缩略图库重写一次压实。
/// 没有档位口径的地方（数据库维护页）用这条；返回 (摘掉的条数, 清完还剩的条数)。
/// 问盘同样放锁外（dead_of 四路并行），一张原图只问一次；最后短锁摘账。
pub fn prune_all() -> (usize, usize) {
    // 同一张图可能在好几档都有一笔：路径先去重，问盘只问一次。
    let mut uniq: Vec<String> = {
        let Ok(l) = lib().lock() else { return (0, 0) };
        l.maps
            .values()
            .flat_map(|m| m.values().map(|r| r.path.clone()))
            .collect()
    };
    uniq.sort_unstable_by_key(|p| p.to_ascii_lowercase());
    uniq.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    let dead: std::collections::HashSet<String> =
        dead_of(&uniq).into_iter().map(|i| uniq[i].to_ascii_lowercase()).collect();
    let mut gone = Vec::new();
    let mut left = 0usize;
    {
        let Ok(mut l) = lib().lock() else { return (0, 0) };
        for (bx, m) in l.maps.iter_mut() {
            let doomed: Vec<String> = m
                .iter()
                .filter(|(_, r)| dead.contains(&r.path.to_ascii_lowercase()))
                .map(|(k, _)| k.clone())
                .collect();
            for k in &doomed {
                if let Some(r) = m.remove(k) {
                    gone.push((r.path, *bx));
                }
            }
            left += m.len();
        }
    }
    let keys: Vec<cache::Key> = gone
        .iter()
        .map(|(p, b)| (cache::hash_of(p), b.0, b.1))
        .collect();
    cache::clear_fps(&keys);
    (gone.len(), left)
}

/// 按排除规则把已入账的指纹摘掉（内存表和缩略图索引都清）。返回摘掉的条数。
/// 规则只拦得住以后入库的，之前扫过/看过的存量要靠这一下清——规则对话框
/// 保存后调它。缩略图本身保留，浏览不受影响。
pub fn purge_excluded() -> usize {
    if !cache::on() {
        return 0;
    }
    let mut keys: Vec<cache::Key> = Vec::new();
    {
        let Ok(mut l) = lib().lock() else { return 0 };
        for (bx, m) in l.maps.iter_mut() {
            let doomed: Vec<String> = m
                .iter()
                .filter(|(_, r)| scan::excluded(&r.path))
                .map(|(k, _)| k.clone())
                .collect();
            for k in &doomed {
                if let Some(r) = m.remove(k) {
                    keys.push((cache::hash_of(&r.path), bx.0, bx.1));
                }
            }
        }
    }
    let n = keys.len();
    if n > 0 {
        cache::clear_fps(&keys);
    }
    n
}

/// 文件改了名：内存表跟着换键，不然这次运行里新路径搜不到、旧路径指向空。
/// 相似库还没加载过就什么都不用做——下次加载直接读索引里已经搬好的记录。
pub fn moved(old: &str, new: &str) {
    let Some(l) = LIB.get() else { return };
    let (ok, nk) = (old.to_ascii_lowercase(), new.to_ascii_lowercase());
    if ok == nk {
        return;
    }
    if let Ok(mut l) = l.lock() {
        for m in l.maps.values_mut() {
            if let Some(mut r) = m.remove(&ok) {
                r.path = new.to_string();
                m.insert(nk.clone(), r);
            }
        }
    }
}

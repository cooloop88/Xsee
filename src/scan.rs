use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::FILETIME;
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FIND_FIRST_EX_LARGE_FETCH,
    FindClose, FindExInfoStandard, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
    WIN32_FIND_DATAW,
};

#[derive(Clone)]
pub struct Entry {
    pub path: String,
    pub is_dir: bool,
    pub is_image: bool,
}

pub fn is_image(path: &str) -> bool {
    // RAW 单独问 preview：哪些后缀算 RAW 只有一处口径，出图那条线也用它。
    matches!(
        Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("jpg") | Some("jpeg") | Some("jpe") | Some("png") | Some("bmp") | Some("gif")
            | Some("tif") | Some("tiff") | Some("ico") | Some("jfif") | Some("webp")
            | Some("heic") | Some("psd") | Some("psb")
            // image 库默认解码器段的格式：exe 里带着解码器就得认，否则就是"有解码器没生效"。
            | Some("avif") | Some("tga") | Some("dds") | Some("qoi") | Some("exr") | Some("hdr")
            | Some("ff") | Some("pbm") | Some("pgm") | Some("ppm") | Some("pnm") | Some("pam")
            // JPEG XR：WIC 内置解码（微软自家格式），只差入口认后缀。
            | Some("wdp") | Some("hdp") | Some("jxr")
    ) || crate::preview::is_raw(path)
}

/// 只列名，不碰内容。文件夹在前，同类按名字排。
/// all=false 只收图片（缩略图视图）；all=true 连非图片文件一起列（列表/详细资料）。
pub fn list(dir: &str, all: bool) -> Vec<Entry> {
    let mut out: Vec<(String, bool, bool)> = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let path = e.path();
            // 目录枚举本来就带回了属性，普通文件和目录不用再逐项 stat；
            // 只有链接（桌面上的 junction）要问一次它到底指向哪。
            let is_dir = match e.file_type() {
                Ok(t) if t.is_dir() => true,
                Ok(t) if t.is_symlink() => path.is_dir(),
                Ok(_) => false,
                Err(_) => path.is_dir(),
            };
            let s = match path.to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            let img = !is_dir && is_image(&s);
            if is_dir || all || img {
                out.push((s, is_dir, img));
            }
        }
    }
    out.sort_by(|a, b| match (a.1, b.1) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()),
    });
    out.into_iter()
        .map(|(path, is_dir, is_image)| Entry { path, is_dir, is_image })
        .collect()
}

/// 目录枚举一趟带回的一条：完整路径、是不是文件夹，加上枚举顺带给的大小和修改时间。
/// 后面判断"这张的缩略图还作不作数"要一遍遍用这两个数，等用到时再回头问一次盘，
/// 慢盘上就是每张白跑好几趟。
pub struct Item {
    pub path: String,
    pub is_dir: bool,
    /// junction、符号链接那一类：不能跟进。
    pub reparse: bool,
    pub size: u64,
    pub mtime: u32,
}

/// 深层目录很容易顶破 260 字的旧限制，问盘时先套上 `\\?\` 这一层。
/// 正斜杠也在这里抹平：`\\?\` 这一层只认反斜杠，地址栏手敲的那种写法带着正斜杠
/// 就一个目录都问不出来。
fn long_path(dir: &str) -> String {
    let d = dir.replace('/', "\\");
    let d = d.trim_end_matches('\\');
    if d.starts_with(r"\\?\") {
        return d.to_string();
    }
    match d.strip_prefix(r"\\") {
        Some(u) => format!(r"\\?\UNC\{u}"),
        None => format!(r"\\?\{d}"),
    }
}

/// 拿回来的路径摘掉前缀：缓存键、界面显示、解码用的都还是原来那种写法。
fn short_path(p: &str) -> String {
    if let Some(u) = p.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{u}")
    } else if let Some(r) = p.strip_prefix(r"\\?\") {
        r.to_string()
    } else {
        p.to_string()
    }
}

fn filetime_secs(t: &FILETIME) -> u32 {
    // FILETIME 从 1601 年数起、100 纳秒一格，换算成和 std 的修改时间同一个口径。
    let raw = ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64;
    raw.saturating_sub(116_444_736_000_000_000)
        .div_euclid(10_000_000)
        .min(u32::MAX as u64) as u32
}

fn find_name(fd: &WIN32_FIND_DATAW) -> String {
    let end = fd
        .cFileName
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(fd.cFileName.len());
    String::from_utf16_lossy(&fd.cFileName[..end])
}

/// 枚举一个文件夹的一层。走系统的大批量枚举：一次多要几十条，几十万小文件的
/// 目录上比一条条讨要快得多，而且名字、属性、大小、修改时间一趟全带回。
/// 读不到（没权限、路径没了）给空表，调用方当这一层没有东西。
pub fn read_level(dir: &str) -> Vec<Item> {
    let base = long_path(dir);
    let pat = format!(r"{base}\*");
    let mut out = Vec::new();
    unsafe {
        let mut fd = WIN32_FIND_DATAW::default();
        let w = HSTRING::from(&pat);
        let Ok(h) = FindFirstFileExW(
            PCWSTR(w.as_ptr()),
            FindExInfoStandard,
            &mut fd as *mut WIN32_FIND_DATAW as *mut core::ffi::c_void,
            FindExSearchNameMatch,
            None,
            FIND_FIRST_EX_LARGE_FETCH,
        ) else {
            return out;
        };
        loop {
            let name = find_name(&fd);
            // 这两条是枚举自己带的"这一层"和"上一层"，属性上都是文件夹。
            if name != "." && name != ".." {
                out.push(Item {
                    path: short_path(&format!(r"{base}\{name}")),
                    is_dir: fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0,
                    reparse: fd.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0,
                    size: ((fd.nFileSizeHigh as u64) << 32) | fd.nFileSizeLow as u64,
                    mtime: filetime_secs(&fd.ftLastWriteTime),
                });
            }
            if FindNextFileW(h, &mut fd).is_err() {
                break;
            }
        }
        let _ = FindClose(h);
    }
    out
}

/// 文件夹名模糊命中关键字就整棵剪掉（里面的几十万张根本不枚举）。
/// 文件后缀命中排除格式则不收、计入跳过数。
fn ruled_out_dir(name: &str, dirs_kw: &[String]) -> bool {
    let low = name.to_ascii_lowercase();
    !dirs_kw.is_empty() && dirs_kw.iter().any(|k| !k.is_empty() && low.contains(k))
}

fn ruled_out_ext(path: &str, exts: &[String]) -> bool {
    if exts.is_empty() {
        return false;
    }
    let low = path.to_ascii_lowercase();
    exts.iter().any(|k| {
        !k.is_empty()
            && Path::new(&low)
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x == k)
    })
}

/// 这一层要不要收文件、子文件夹照走，判断都归调用方（那边才有队列和停止开关）。
/// 这两个函数只是把原来一趟走查里的规则口径留在 scan 里，别处也能对上。
pub fn skip_dir(it: &Item, dirs_kw: &[String]) -> bool {
    it.reparse
        || ruled_out_dir(
            it.path
                .rsplit_once(['\\', '/'])
                .map(|(_, n)| n)
                .unwrap_or(&it.path),
            dirs_kw,
        )
}

pub fn skip_file(path: &str, exts: &[String]) -> bool {
    ruled_out_ext(path, exts)
}

/// 文件夹扫描状态：一条 16 字节 —— 路径哈希 8 + 盒尺寸 2+2 + 文件夹修改时间 4。
/// 记的是"这一档、这个文件夹，在它这个修改时间上已经整层扫完了"。
/// 文件夹的直接子项增删才会改它的修改时间，所以时间没变就说明这一层的文件
/// 一个没多一个没少，不用再一张张去问有没有缩略图。子文件夹里变了不影响
/// 这一层的判断，所以只跳过这一层的文件，子文件夹照走。
const DIR_REC: usize = 16;
const DIR_FILE: &str = "scan2.dir";
/// 表只追加，反复扫同一批目录会越写越肥；过这个头就把内存里那份整表重写。
const DIR_COMPACT: u64 = 4 * 1024 * 1024;
/// 读表时的体积上限，坏文件别拖住启动。
const DIR_MAX_BYTES: u64 = 64 * 1024 * 1024;

type DirKey = (u64, u16, u16);

static SCANNED: OnceLock<Mutex<HashMap<DirKey, u32>>> = OnceLock::new();

fn dir_file() -> PathBuf {
    crate::cache::dir().join(DIR_FILE)
}

fn load_dirs() -> HashMap<DirKey, u32> {
    let mut m = HashMap::new();
    let p = dir_file();
    if !crate::cache::on() || std::fs::metadata(&p).map(|x| x.len()).unwrap_or(0) > DIR_MAX_BYTES {
        return m;
    }
    let Ok(bytes) = std::fs::read(&p) else { return m };
    for c in bytes.chunks_exact(DIR_REC) {
        let k = (
            u64::from_le_bytes(c[0..8].try_into().unwrap()),
            u16::from_le_bytes(c[8..10].try_into().unwrap()),
            u16::from_le_bytes(c[10..12].try_into().unwrap()),
        );
        m.insert(k, u32::from_le_bytes(c[12..16].try_into().unwrap()));
    }
    m
}

fn scanned() -> &'static Mutex<HashMap<DirKey, u32>> {
    SCANNED.get_or_init(|| Mutex::new(load_dirs()))
}

fn box_ok(bw: u32, bh: u32) -> bool {
    crate::cache::on() && bw <= u16::MAX as u32 && bh <= u16::MAX as u32
}

/// 这一档、这个修改时间，表里已经记着整层扫完过 → 这一层的文件不用再收。
pub fn dir_done(dir: &str, bw: u32, bh: u32, mtime: u32) -> bool {
    if !box_ok(bw, bh) {
        return false;
    }
    scanned().lock().map_or(false, |m| {
        m.get(&(crate::cache::hash_of(dir), bw as u16, bh as u16))
            .is_some_and(|t| *t == mtime)
    })
}

/// 一轮扫完的文件夹整批落盘：一条条写会把盘跑成碎步，收尾一次写完。
/// 内存表同时更新，同一趟里后面再碰到这个文件夹也不用回盘查。
pub fn note_dirs(list: &[(String, u32)], bw: u32, bh: u32) {
    if list.is_empty() || !box_ok(bw, bh) {
        return;
    }
    let (w, h) = (bw as u16, bh as u16);
    let mut buf: Vec<u8> = Vec::with_capacity(list.len() * DIR_REC);
    let mut keys: Vec<(DirKey, u32)> = Vec::with_capacity(list.len());
    for (dir, mtime) in list {
        let k = (crate::cache::hash_of(dir), w, h);
        keys.push((k, *mtime));
        buf.extend_from_slice(&k.0.to_le_bytes());
        buf.extend_from_slice(&w.to_le_bytes());
        buf.extend_from_slice(&h.to_le_bytes());
        buf.extend_from_slice(&mtime.to_le_bytes());
    }
    let p = dir_file();
    let grew = {
        let Ok(mut m) = scanned().lock() else { return };
        for (k, t) in keys {
            m.insert(k, t);
        }
        if std::fs::metadata(&p).map(|x| x.len()).unwrap_or(0) > DIR_COMPACT {
            let mut out = Vec::with_capacity(m.len() * DIR_REC);
            for (k, t) in m.iter() {
                out.extend_from_slice(&k.0.to_le_bytes());
                out.extend_from_slice(&k.1.to_le_bytes());
                out.extend_from_slice(&k.2.to_le_bytes());
                out.extend_from_slice(&t.to_le_bytes());
            }
            let tmp = p.with_extension("rewrite");
            if std::fs::write(&tmp, &out).is_ok() {
                let _ = std::fs::rename(&tmp, &p);
            }
            false
        } else {
            true
        }
    };
    if grew {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
            let _ = f.write_all(&buf);
        }
    }
}

/// 扫描名单的排除规则，每个根一份。所有写指纹的口子（浏览顺手记、出图补、
/// 补齐、重建、旧库迁移）都先问 excluded——命中的路一律不记账，缩略图照常
/// 生成、浏览不受影响，搜图库里就是没有它们。
#[derive(Clone, Default)]
pub struct ExclRule {
    /// 根路径小写、以 \ 结尾：规则只对根底下的路径生效。
    root: String,
    /// 排除文件夹关键字：路径里任何一级文件夹名包含它就命中（模糊匹配）。
    dirs: Vec<String>,
    /// 排除文件格式：后缀精确命中（不带点、小写）。
    exts: Vec<String>,
}

impl ExclRule {
    pub fn new(root: &str, dirs: &[String], exts: &[String]) -> ExclRule {
        let mut r = root.trim_end_matches(['\\', '/']).to_ascii_lowercase();
        r.push('\\');
        ExclRule {
            root: r,
            dirs: dirs.iter().map(|s| s.trim().to_ascii_lowercase()).collect(),
            exts: exts.iter().map(|s| s.trim().trim_start_matches('.').to_ascii_lowercase()).collect(),
        }
    }
}

static EXCLUDE: Mutex<Vec<ExclRule>> = Mutex::new(Vec::new());

/// 规则整份替换。名单、规则编辑保存之后都要调一次。
pub fn set_rules(rules: Vec<ExclRule>) {
    if let Ok(mut r) = EXCLUDE.lock() {
        *r = rules;
    }
}

/// 这条路径按现行规则收不收。路径不落在任何扫描根底下就不受规则限制
/// （手动浏览等于手动认可）。命中返回 true = 不记指纹、扫描不收。
pub fn excluded(path: &str) -> bool {
    if EXCLUDE.lock().map(|r| r.is_empty()).unwrap_or(true) {
        return false;
    }
    let low = path.to_ascii_lowercase();
    let Ok(rules) = EXCLUDE.lock() else { return false };
    for r in rules.iter() {
        if !low.starts_with(&r.root) {
            continue;
        }
        if r.exts.iter().any(|e| {
            Path::new(&low)
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x == e)
        }) {
            return true;
        }
        // 只看文件夹那几段，不算文件名——关键字 "map" 不该拦住 mymap.jpg。
        if let Some(parent) = Path::new(&low).parent() {
            if parent
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .any(|seg| r.dirs.iter().any(|k| !k.is_empty() && seg.contains(k)))
            {
                return true;
            }
        }
    }
    false
}

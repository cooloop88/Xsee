use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

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

/// 递归列出 root 底下（含全部子文件夹）的图片文件，完整路径。扫描文件夹建库用。
/// reparse 点（junction、符号链接）不跟进——环形链接会把整趟扫描挂死，
/// 桌面上挂着指向自身的 junction 并不稀奇。stop 一置就尽快收手，返回已找到的那批。
/// 排除规则：文件夹名包含 dirs_kw 里任何一个关键字就整棵剪掉（不进去也不计数），
/// 文件后缀命中 exts 的文件不收、计入跳过数。返回 (图片文件, 按规则跳过的文件数)。
pub fn walk_images(
    root: &str,
    stop: &AtomicBool,
    dirs_kw: &[String],
    exts: &[String],
) -> (Vec<String>, usize) {
    let (mut out, mut skipped) = (Vec::new(), 0usize);
    let mut stack: Vec<PathBuf> = vec![PathBuf::from(root)];
    while let Some(dir) = stack.pop() {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_dir() {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    // 0x400 = FILE_ATTRIBUTE_REPARSE_POINT。DirEntry 的 file_type
                    // 在老系统上认不全 junction，直接问属性最稳。
                    if e.metadata()
                        .map(|m| m.file_attributes() & 0x400 != 0)
                        .unwrap_or(true)
                    {
                        continue;
                    }
                }
                #[cfg(not(windows))]
                if ft.is_symlink() {
                    continue;
                }
                // 文件夹名模糊命中关键字就整棵剪枝：里面的几十万张根本不枚举。
                if !dirs_kw.is_empty() {
                    let name = p
                        .file_name()
                        .map(|s| s.to_ascii_lowercase().to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if dirs_kw.iter().any(|k| !k.is_empty() && name.contains(k)) {
                        continue;
                    }
                }
                stack.push(p);
            } else if ft.is_file() {
                let Some(s) = p.to_str() else { continue };
                if !exts.is_empty() {
                    let ext = p
                        .extension()
                        .map(|x| x.to_ascii_lowercase().to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if exts.iter().any(|k| !k.is_empty() && ext == *k) {
                        skipped += 1;
                        continue;
                    }
                }
                if is_image(s) {
                    out.push(s.to_string());
                }
            }
        }
    }
    (out, skipped)
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

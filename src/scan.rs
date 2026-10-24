use std::fs;
use std::path::Path;

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

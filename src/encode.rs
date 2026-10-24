//! 格式转换：WIC/image 解全尺寸，image/jpeg-encoder/tiff 编码。
//! 先写同目录的 .part 再改名，中途失败不会留下半截目标文件。

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use image::{DynamicImage, ExtendedColorType, ImageFormat, RgbaImage};

pub const FORMATS: &[(&str, &str, &str)] = &[
    ("JPEG", "jpg", "JFIF 联合图像专家组"),
    ("PNG", "png", "可移植网络图形"),
    ("BMP", "bmp", "Windows 位图"),
    ("TIFF", "tif", "标签图像文件"),
    ("GIF", "gif", "图形交换格式"),
];

/// 同名文件怎么办。转换对话框里的四档落到这里：询问在批量队列里等同于跳过，
/// 不逐张弹框把整条队列卡死（界面会提示）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Collision {
    Overwrite,
    Skip,
    Rename,
}

/// 一个文件转换完了：写成文件了，或者撞名跳过了。
pub enum Outcome {
    Done(String, u64),
    Skipped,
}

/// 各格式的设置。字段都放在一处，皮肤/对话框只读写这一个结构。
#[derive(Clone)]
pub struct Opt {
    /// JPEG 质量 1–100
    pub quality: u8,
    pub progressive: bool,
    pub optimize_huffman: bool,
    /// 2:1 水平 / 垂直色度取样
    pub sample_h: bool,
    pub sample_v: bool,
    /// 0 从来不更新、1 仅更新已存在的、2 总是增加更新
    pub thumb: u8,
    pub dcf: bool,
    /// PNG 压缩级别 0–9（0 存储，9 最好）
    pub png_level: u8,
    /// TIFF 压缩：0 不压缩、1 LZW、2 ZIP、3 PackBits
    pub tiff_comp: u8,
}

impl Default for Opt {
    fn default() -> Self {
        Opt {
            quality: 90,
            progressive: false,
            optimize_huffman: true,
            sample_h: true,
            sample_v: true,
            thumb: 0,
            dcf: false,
            png_level: 9,
            tiff_comp: 1,
        }
    }
}

impl Opt {
    pub fn for_fmt(fmt: usize) -> Opt {
        let mut o = Opt::default();
        if fmt == 0 {
            o.sample_h = true;
            o.sample_v = true;
        }
        o
    }
}

/// 缩放方式。和批量转换面板上的五档一一对应。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    /// 不动尺寸，只做旋转/翻转/锐化
    None,
    /// 等比装进 W×H（可放大，结果尺寸可能小于框）
    Contain,
    /// 等比铺满 W×H 后居中裁切
    Cover,
    /// 直接拉到 W×H，不管比例
    Stretch,
    /// 等比缩到不超过 W×H，只缩不放
    Fit,
}

/// 一批转换共用的几何/画质处理。默认全关，等于原来的纯格式转换。
#[derive(Clone, Copy)]
pub struct Xform {
    pub fit: FitMode,
    /// 目标宽高，fit 为 None 时不看
    pub w: u32,
    pub h: u32,
    pub allow_upscale: bool,
    /// 顺时针 0/90/180/270
    pub rotate: u16,
    pub flip_h: bool,
    pub flip_v: bool,
    /// 0 关；1–100 是 unsharp 强度
    pub sharpen: u8,
}

impl Default for Xform {
    fn default() -> Self {
        Xform {
            fit: FitMode::None,
            w: 0,
            h: 0,
            allow_upscale: false,
            rotate: 0,
            flip_h: false,
            flip_v: false,
            sharpen: 0,
        }
    }
}

impl Xform {
    /// 全是默认值时像素不会变，EXIF 可以照抄。
    pub fn is_noop(&self) -> bool {
        self.fit == FitMode::None
            && self.rotate % 360 == 0
            && !self.flip_h
            && !self.flip_v
            && self.sharpen == 0
    }
}

/// 文字水印。空文字等于不加。颜色 0xRRGGBB，位置 0 左上 1 右上 2 左下 3 右下 4 居中。
#[derive(Clone)]
pub struct Watermark {
    pub text: String,
    pub pos: u8,
    /// 字号占画面长边的百分比
    pub size_pct: u8,
    /// 0–100
    pub opacity: u8,
    pub color: u32,
}

impl Default for Watermark {
    fn default() -> Self {
        Watermark {
            text: String::new(),
            pos: 3,
            size_pct: 4,
            opacity: 55,
            color: 0xFFFFFF,
        }
    }
}

/// JPEG 用 jpeg-decoder 解：image 的 JPEG 后端会把自家写出的 4:2:0 文件
/// 解成绿色横条且不报错。CMYK 交回 image 处理，它认 Adobe 反相那套。
fn decode_jpeg(path: &str) -> Option<RgbaImage> {
    use jpeg_decoder::PixelFormat;
    let f = File::open(path).ok()?;
    let mut d = jpeg_decoder::Decoder::new(std::io::BufReader::new(f));
    d.read_info().ok()?;
    let px = d.decode().ok()?;
    let info = d.info()?;
    let (w, h) = (info.width as u32, info.height as u32);
    let n = (w as usize).checked_mul(h as usize)?;
    let mut out = Vec::with_capacity(n * 4);
    match info.pixel_format {
        PixelFormat::RGB24 => {
            if px.len() != n * 3 {
                return None;
            }
            for p in px.chunks_exact(3) {
                out.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        PixelFormat::L8 => {
            if px.len() != n {
                return None;
            }
            for &v in &px {
                out.extend_from_slice(&[v, v, v, 255]);
            }
        }
        PixelFormat::L16 => {
            if px.len() != n * 2 {
                return None;
            }
            for p in px.chunks_exact(2) {
                out.extend_from_slice(&[p[0], p[0], p[0], 255]);
            }
        }
        _ => return None,
    }
    RgbaImage::from_raw(w, h, out)
}

/// 解码器只给像素，不替人摆正；这里补上 EXIF 方向，不然手机竖拍的照片转出来还是横的。
fn orient_rgba(path: &str, rgba: RgbaImage) -> Result<RgbaImage, String> {
    let o = crate::preview::orientation(path);
    if o == 1 {
        return Ok(rgba);
    }
    let (mut w, mut h) = (rgba.width(), rgba.height());
    let mut px = rgba.into_raw();
    crate::dec::orient(&mut px, &mut w, &mut h, o);
    RgbaImage::from_raw(w, h, px).ok_or_else(|| format!("摆正后尺寸不对：{path}"))
}

/// 解成 RGBA8。WIC 认的格式（含 HEIC/WebP 的系统解码器）交给 WIC，
/// image 能吃的直接吃；PSD 没有全尺寸解码器时退回档内 JPEG 预览。
pub fn decode(path: &str, for_preview: bool) -> Result<RgbaImage, String> {
    let lower = path.to_ascii_lowercase();
    let is_jpg = [".jpg", ".jpeg", ".jpe", ".jfif"]
        .iter()
        .any(|e| lower.ends_with(e));
    let wic_first = lower.ends_with(".psd")
        || lower.ends_with(".psb")
        || lower.ends_with(".heic")
        || lower.ends_with(".webp");
    if is_jpg {
        if let Some(rgba) = decode_jpeg(path) {
            return orient_rgba(path, rgba);
        }
    }
    if !wic_first {
        if let Ok(img) = image::open(path) {
            return orient_rgba(path, img.to_rgba8());
        }
    }
    crate::job::init_com();
    if let Ok(v) = crate::job::decode_view(path, 0, 0) {
        return Ok(bgra_to_rgba(v.w, v.h, v.px));
    }
    if for_preview {
        if let Some(v) = crate::job::embedded_view(path, u32::MAX, u32::MAX) {
            return Ok(bgra_to_rgba(v.w, v.h, v.px));
        }
    }
    Err(format!("解不开 {}", path))
}

fn bgra_to_rgba(w: u32, h: u32, px: Vec<u8>) -> RgbaImage {
    let mut out = px;
    for p in out.chunks_exact_mut(4) {
        p.swap(0, 2);
        p[3] = 0xFF;
    }
    RgbaImage::from_raw(w, h, out).unwrap_or_else(|| RgbaImage::new(w.max(1), h.max(1)))
}

/// 把一个文件转成目标格式。返回输出路径和字节数；撞名跳过时返回 Skipped。
pub fn convert_file(
    src: &str,
    out_dir: &str,
    fmt: usize,
    opt: &Opt,
    policy: Collision,
) -> Result<Outcome, String> {
    convert_file_x(src, out_dir, None, fmt, opt, policy, None, None)
}

/// 带缩放/旋转/翻转/锐化/水印和改名的一版。`rename` 是已经展开好的目标主名，
/// 空或 None 就沿用源文件名。
pub fn convert_file_x(
    src: &str,
    out_dir: &str,
    rename: Option<&str>,
    fmt: usize,
    opt: &Opt,
    policy: Collision,
    xf: Option<&Xform>,
    wm: Option<&Watermark>,
) -> Result<Outcome, String> {
    let (_, ext, _) = *FORMATS.get(fmt).ok_or("未知格式")?;
    let stem = rename
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| Path::new(src).file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
        .ok_or("文件名不合法")?;
    let stem = stem.as_str();
    let out = target_path(src, out_dir, stem, ext, policy)?;
    if out.is_none() {
        return Ok(Outcome::Skipped);
    }
    let out = out.unwrap();
    let mut img = decode(src, true)?;
    let part = Path::new(out_dir).join(format!(".{}.{}.part", stem, std::process::id()));
    // 并行转换时同一目录可能有两张同名不同扩展名的源文件，part 名再带一次随机后缀
    let part = if part.exists() {
        Path::new(out_dir).join(format!(
            ".{}.{}.{}.part",
            stem,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ))
    } else {
        part
    };
    // 动过像素就别再抄源文件的 EXIF：方向已经归到像素里了，缩放过尺寸也对不上。
    let exif_src = match xf {
        Some(x) if !x.is_noop() => None,
        _ => Some(src),
    };
    if let Some(x) = xf {
        img = apply_xform(img, x);
    }
    if let Some(w) = wm {
        draw_watermark(&mut img, w);
    }
    let r = encode_to(&img, &part, fmt, opt, exif_src);
    if let Err(e) = r {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    if out.exists() {
        let _ = std::fs::remove_file(&out);
    }
    if let Err(e) = std::fs::rename(&part, &out) {
        let _ = std::fs::remove_file(&part);
        return Err(format!("改名失败：{e}"));
    }
    let n = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    Ok(Outcome::Done(out.display().to_string(), n))
}

// ---- 图像变换：先转正（旋转/翻转），再按目标框缩放，最后锐化 ----

fn apply_xform(img: RgbaImage, xf: &Xform) -> RgbaImage {
    let mut im = img;
    if xf.flip_h {
        im = image::imageops::flip_horizontal(&im);
    }
    if xf.flip_v {
        im = image::imageops::flip_vertical(&im);
    }
    im = match xf.rotate % 360 {
        90 => image::imageops::rotate90(&im),
        180 => image::imageops::rotate180(&im),
        270 => image::imageops::rotate270(&im),
        _ => im,
    };
    let (w0, h0) = (im.width().max(1), im.height().max(1));
    // 目标框只填一边时，另一边按这张图自己的比例补出来。每张图比例不同，
    // 所以只能在拿到像素之后算，面板那边不用逼着人把两个格子都填满。
    // 两边都空就是 0×0，下面按"不缩放"处理，锐化照做。
    let (tw, th) = if xf.w == 0 && xf.h == 0 {
        (0, 0)
    } else {
        let mut tw = xf.w;
        let mut th = xf.h;
        if tw == 0 {
            tw = ((th as f64 * w0 as f64 / h0 as f64).round() as u32).max(1);
        }
        if th == 0 {
            th = ((tw as f64 * h0 as f64 / w0 as f64).round() as u32).max(1);
        }
        (tw.max(1), th.max(1))
    };
    let up = xf.allow_upscale;
    im = match xf.fit {
        FitMode::None => im,
        _ if tw == 0 || th == 0 => im,
        FitMode::Stretch => resize_to(&im, tw, th),
        FitMode::Contain | FitMode::Fit => {
            let mut s = (tw as f64 / w0 as f64).min(th as f64 / h0 as f64);
            if !up || xf.fit == FitMode::Fit {
                s = s.min(1.0);
            }
            if s <= 0.0 {
                im
            } else {
                let nw = (w0 as f64 * s).round().max(1.0) as u32;
                let nh = (h0 as f64 * s).round().max(1.0) as u32;
                resize_to(&im, nw, nh)
            }
        }
        FitMode::Cover => {
            let mut s = (tw as f64 / w0 as f64).max(th as f64 / h0 as f64);
            if !up {
                s = s.min(1.0);
            }
            if s <= 0.0 {
                im
            } else {
                let nw = (w0 as f64 * s).round().max(1.0) as u32;
                let nh = (h0 as f64 * s).round().max(1.0) as u32;
                let scaled = resize_to(&im, nw, nh);
                // 居中裁到目标框；缩得太小到不了目标框时按现有尺寸裁，不补边
                let cw = tw.min(nw);
                let ch = th.min(nh);
                let x = (nw - cw) / 2;
                let y = (nh - ch) / 2;
                image::imageops::crop_imm(&scaled, x, y, cw, ch).to_image()
            }
        }
    };
    if xf.sharpen > 0 {
        im = sharpen(&im, xf.sharpen);
    }
    im
}

/// 缩小走 Lanczos，放大走 Catmull-Rom（双三次）。和常见批量缩放的取法一致。
fn resize_to(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    if w == img.width() && h == img.height() {
        return img.clone();
    }
    let f = if w < img.width() || h < img.height() {
        image::imageops::FilterType::Lanczos3
    } else {
        image::imageops::FilterType::CatmullRom
    };
    image::imageops::resize(img, w, h, f)
}

/// 3×3 高斯模糊做反锐化掩模。只动 RGB，alpha 原样留着。
fn sharpen(img: &RgbaImage, amount: u8) -> RgbaImage {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let src = img.as_raw();
    let mut out = src.clone();
    if w < 3 || h < 3 {
        return img.clone();
    }
    let k = amount.min(100) as f32 / 100.0 * 1.2;
    let at = |x: usize, y: usize, c: usize| src[(y * w + x) * 4 + c] as f32;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y * w + x) * 4;
            for c in 0..3 {
                let c0 = at(x, y, c);
                let blur = (4.0 * c0
                    + 2.0 * (at(x, y - 1, c) + at(x, y + 1, c) + at(x - 1, y, c) + at(x + 1, y, c))
                    + at(x - 1, y - 1, c)
                    + at(x + 1, y - 1, c)
                    + at(x - 1, y + 1, c)
                    + at(x + 1, y + 1, c))
                    / 16.0;
                out[i + c] = (c0 + (c0 - blur) * k).clamp(0.0, 255.0) as u8;
            }
        }
    }
    RgbaImage::from_raw(img.width(), img.height(), out).unwrap_or_else(|| img.clone())
}

/// 用 GDI 把文字画进一张 32 位 DIB，再按黑色底的亮度当覆盖率叠回 RGBA。
/// 这样中英文、系统任意字体都能用，不需要自带字模。
fn draw_watermark(img: &mut RgbaImage, wm: &Watermark) {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::{COLORREF, RECT};
    use windows::Win32::Graphics::Gdi::*;

    let text = wm.text.trim();
    if text.is_empty() {
        return;
    }
    let (w, h) = (img.width() as i32, img.height() as i32);
    if w < 4 || h < 4 {
        return;
    }
    let long = w.max(h) as f32;
    let px = ((long * wm.size_pct.clamp(1, 100) as f32 / 100.0).round() as i32).clamp(8, h.max(8));
    unsafe {
        let mem = CreateCompatibleDC(None);
        if mem.is_invalid() {
            return;
        }
        let bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let Ok(bmp) = CreateDIBSection(Some(mem), &bi, DIB_RGB_COLORS, &mut bits, None, 0) else {
            let _ = DeleteDC(mem);
            return;
        };
        if bits.is_null() {
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            let _ = DeleteDC(mem);
            return;
        }
        let old_bmp = SelectObject(mem, HGDIOBJ(bmp.0));
        // GDI 不写 alpha，内存又没清零，先把底刷黑：文字以外的地方覆盖率自然为 0
        let black = CreateSolidBrush(COLORREF(0));
        let _ = FillRect(mem, &RECT { left: 0, top: 0, right: w, bottom: h }, black);
        let _ = DeleteObject(HGDIOBJ(black.0));
        let _ = SetBkMode(mem, TRANSPARENT);
        // 画白色文字，读回来时取 RGB 最大值当覆盖率，边缘抗锯齿就是软过渡
        let _ = SetTextColor(mem, COLORREF(0x00FFFFFF));
        let face = HSTRING::from("Microsoft YaHei");
        let font = CreateFontW(
            -px,
            0,
            0,
            0,
            FW_SEMIBOLD.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            0x0200,
            PCWSTR(face.as_ptr()),
        );
        let old_font = SelectObject(mem, HGDIOBJ(font.0));
        let mut u: Vec<u16> = text.encode_utf16().collect();
        let mut calc = RECT { left: 0, top: 0, right: w, bottom: h };
        let _ = DrawTextW(mem, &mut u, &mut calc, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
        let tw = (calc.right - calc.left).min(w);
        let th = (calc.bottom - calc.top).min(h);
        let m = (px / 3).max(4);
        let (x, y) = match wm.pos {
            0 => (m, m),
            1 => ((w - tw - m).max(m), m),
            2 => (m, (h - th - m).max(m)),
            4 => ((w - tw) / 2, (h - th) / 2),
            _ => ((w - tw - m).max(m), (h - th - m).max(m)),
        };
        let mut r = RECT { left: x, top: y, right: x + tw, bottom: y + th };
        let _ = DrawTextW(mem, &mut u, &mut r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        // 叠回原图
        let op = wm.opacity.min(100) as f32 / 100.0;
        let (cr, cg, cb) = (
            ((wm.color >> 16) & 0xff) as f32,
            ((wm.color >> 8) & 0xff) as f32,
            (wm.color & 0xff) as f32,
        );
        let src = bits as *const u8;
        let dst = img.as_mut();
        for yy in 0..h as usize {
            for xx in 0..w as usize {
                let i = (yy * w as usize + xx) * 4;
                // DIB 是 BGRA
                let cov = (*src.add(i)).max(*src.add(i + 1)).max(*src.add(i + 2)) as f32 / 255.0;
                let a = cov * op;
                if a <= 0.0 {
                    continue;
                }
                for c in 0..3 {
                    let target = match c {
                        0 => cr,
                        1 => cg,
                        _ => cb,
                    };
                    let cur = dst[i + c] as f32;
                    dst[i + c] = (cur * (1.0 - a) + target * a).clamp(0.0, 255.0) as u8;
                }
            }
        }
        let _ = SelectObject(mem, old_font);
        let _ = SelectObject(mem, old_bmp);
        let _ = DeleteObject(HGDIOBJ(font.0));
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
    }
}

/// 目标路径：撞名时按策略跳过 / 覆盖 / 自动加序号。
/// 源文件和目标是同一个文件（例如 jpg 转 jpg 原地）不算撞名，照常写。
fn target_path(
    src: &str,
    out_dir: &str,
    stem: &str,
    ext: &str,
    policy: Collision,
) -> Result<Option<PathBuf>, String> {
    let first = Path::new(out_dir).join(format!("{stem}.{ext}"));
    if !first.exists() || same_file(src, &first) {
        return Ok(Some(first));
    }
    match policy {
        Collision::Overwrite => Ok(Some(first)),
        Collision::Skip => Ok(None),
        Collision::Rename => {
            for i in 2..=9999 {
                let p = Path::new(out_dir).join(format!("{stem} ({i}).{ext}"));
                if !p.exists() {
                    return Ok(Some(p));
                }
            }
            Err("同名文件太多，找不到空号".to_string())
        }
    }
}

fn same_file(a: &str, b: &Path) -> bool {
    a.eq_ignore_ascii_case(&b.display().to_string())
}

/// 编辑窗口保存用：按扩展名选格式，先写 .part 再替换原文件。
pub fn save_rgba(img: &RgbaImage, path: &str) -> Result<(), String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let fmt = match ext.as_str() {
        "jpg" | "jpeg" | "jpe" | "jfif" => 0,
        "png" => 1,
        "bmp" => 2,
        "tif" | "tiff" => 3,
        "gif" => 4,
        _ => return Err(format!("这个格式不能直接保存：.{ext}")),
    };
    let mut opt = Opt::for_fmt(fmt);
    opt.quality = 95;
    let dir = Path::new(path).parent().unwrap_or_else(|| Path::new("."));
    let part = dir.join(format!(
        ".{}.{}.part",
        Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("save"),
        std::process::id()
    ));
    encode_to(img, &part, fmt, &opt, Some(path))?;
    if Path::new(path).exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&part, path).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        format!("保存失败：{e}")
    })
}

/// 直接编码一块 RGBA 到指定路径。force_src 用来取源文件的 EXIF。
fn encode_to(
    img: &RgbaImage,
    path: &Path,
    fmt: usize,
    opt: &Opt,
    exif_src: Option<&str>,
) -> Result<(), String> {
    match fmt {
        0 => encode_jpeg(img, path, opt, exif_src),
        3 => encode_tiff(img, path, opt),
        _ => encode_std(img, path, fmt, opt),
    }
}

fn encode_jpeg(
    img: &RgbaImage,
    path: &Path,
    opt: &Opt,
    exif_src: Option<&str>,
) -> Result<(), String> {
    use jpeg_encoder::{ColorType, Encoder, SamplingFactor};
    if img.width() > 65535 || img.height() > 65535 {
        return Err("JPEG 最大只到 65535 像素".to_string());
    }
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut enc = Encoder::new(&mut buf, opt.quality.clamp(1, 100));
        let sampling = match (opt.sample_h, opt.sample_v) {
            (true, true) => SamplingFactor::F_2_2,
            (true, false) => SamplingFactor::F_2_1,
            (false, true) => SamplingFactor::F_1_2,
            (false, false) => SamplingFactor::F_1_1,
        };
        enc.set_sampling_factor(sampling);
        enc.set_progressive(opt.progressive);
        enc.set_optimized_huffman_tables(opt.optimize_huffman);
        if let Some(exif) = jpeg_exif(img, exif_src, opt) {
            enc.add_app_segment(1, exif)
                .map_err(|e| format!("EXIF 写入失败：{e}"))?;
        }
        enc.encode(img.as_raw(), img.width() as u16, img.height() as u16, ColorType::Rgba)
            .map_err(|e| format!("JPEG 编码失败：{e}"))?;
    }
    let mut f = File::create(path).map_err(|e| format!("建不了 {}：{e}", path.display()))?;
    f.write_all(&buf).map_err(|e| e.to_string())?;
    f.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn encode_std(img: &RgbaImage, path: &Path, fmt: usize, opt: &Opt) -> Result<(), String> {
    use image::ImageEncoder;
    let f = File::create(path).map_err(|e| format!("建不了 {}：{e}", path.display()))?;
    let mut w = BufWriter::new(f);
    if fmt == 1 {
        use image::codecs::png::{CompressionType, FilterType, PngEncoder};
        let comp = match opt.png_level {
            0 => CompressionType::Uncompressed,
            1..=3 => CompressionType::Fast,
            4..=6 => CompressionType::Default,
            _ => CompressionType::Best,
        };
        PngEncoder::new_with_quality(&mut w, comp, FilterType::Adaptive)
            .write_image(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgba8)
            .map_err(|e| format!("PNG 编码失败：{e}"))?;
    } else {
        let format = match fmt {
            2 => ImageFormat::Bmp,
            4 => ImageFormat::Gif,
            _ => return Err("未知格式".to_string()),
        };
        let dynimg = DynamicImage::ImageRgba8(img.clone());
        dynimg
            .write_to(&mut w, format)
            .map_err(|e| format!("{} 编码失败：{e}", FORMATS.get(fmt).map(|f| f.0).unwrap_or("图片")))?;
    }
    w.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn encode_tiff(img: &RgbaImage, path: &Path, opt: &Opt) -> Result<(), String> {
    use tiff::encoder::compression::DeflateLevel;
    use tiff::encoder::{colortype, Compression, TiffEncoder};
    let f = File::create(path).map_err(|e| format!("建不了 {}：{e}", path.display()))?;
    let mut w = BufWriter::new(f);
    let comp = match opt.tiff_comp {
        0 => Compression::Uncompressed,
        1 => Compression::Lzw,
        2 => Compression::Deflate(DeflateLevel::Balanced),
        _ => Compression::Packbits,
    };
    let mut enc = TiffEncoder::new(&mut w)
        .map_err(|e| format!("TIFF 编码失败：{e}"))?
        .with_compression(comp);
    enc.write_image::<colortype::RGBA8>(img.width(), img.height(), img.as_raw())
        .map_err(|e| format!("TIFF 编码失败：{e}"))?;
    w.flush().map_err(|e| e.to_string())?;
    Ok(())
}

// ---- EXIF：复制源文件的 EXIF 并把方向归零；需要缩略图时另拼一张小的 ----

fn jpeg_exif(img: &RgbaImage, src: Option<&str>, opt: &Opt) -> Option<Vec<u8>> {
    let want_thumb = opt.thumb == 2 || opt.dcf || (opt.thumb == 1 && src.and_then(source_exif).is_some());
    if want_thumb {
        return Some(build_exif(img));
    }
    let mut e = source_exif(src?)?;
    neutralize_orientation(&mut e);
    Some(e)
}

/// 从源文件前 1 MB 里找 APP1 Exif 段，返回含 "Exif\0\0" 的整段负载。
fn source_exif(src: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut f = File::open(src).ok()?;
    let mut data = vec![0u8; 1024 * 1024];
    let n = f.read(&mut data).ok()?;
    data.truncate(n);
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            break;
        }
        let marker = data[i + 1];
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if len < 2 || i + 2 + len > data.len() {
            break;
        }
        let seg = &data[i + 4..i + 2 + len];
        if marker == 0xE1 && seg.len() > 6 && &seg[..6] == b"Exif\0\0" {
            return Some(seg.to_vec());
        }
        i += 2 + len;
    }
    None
}

/// 把 IFD0 里的 Orientation(0x0112) 改成 1。像素已经摆正，方向标记不能再留着。
fn neutralize_orientation(exif: &mut [u8]) {
    if exif.len() < 14 || &exif[..6] != b"Exif\0\0" {
        return;
    }
    let t = &mut exif[6..];
    let le = match &t[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return,
    };
    let rd16 = |b: &[u8], o: usize| -> u16 {
        if le {
            u16::from_le_bytes([b[o], b[o + 1]])
        } else {
            u16::from_be_bytes([b[o], b[o + 1]])
        }
    };
    let rd32 = |b: &[u8], o: usize| -> u32 {
        if le {
            u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
        } else {
            u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
        }
    };
    if t.len() < 8 {
        return;
    }
    let off = rd32(t, 4) as usize;
    if off + 2 > t.len() {
        return;
    }
    let n = rd16(t, off) as usize;
    for k in 0..n {
        let e = off + 2 + k * 12;
        if e + 12 > t.len() {
            return;
        }
        if rd16(t, e) == 0x0112 {
            let val = rd16(t, e + 8);
            let one = if le { 1u16.to_le_bytes() } else { 1u16.to_be_bytes() };
            let _ = val;
            t[e + 8..e + 10].copy_from_slice(&one);
            return;
        }
    }
}

/// 拼一份最小 EXIF：IFD0 只放方向和分辨率，IFD1 挂一张 160×120 的 JPEG 缩略图。
fn build_exif(img: &RgbaImage) -> Vec<u8> {
    let thumb = make_thumb(img);
    let mut t: Vec<u8> = Vec::new();
    t.extend_from_slice(b"II*\0");
    t.extend_from_slice(&8u32.to_le_bytes());
    // IFD0：4 个条目
    let ifd0 = 8usize;
    let entries0 = 4usize;
    let data0 = ifd0 + 2 + entries0 * 12 + 4;
    let xres = data0;
    let yres = data0 + 8;
    let ifd1 = data0 + 16;
    t.resize(ifd0, 0);
    t.extend_from_slice(&(entries0 as u16).to_le_bytes());
    let mut ent = |tag: u16, ty: u16, count: u32, val: [u8; 4]| {
        t.extend_from_slice(&tag.to_le_bytes());
        t.extend_from_slice(&ty.to_le_bytes());
        t.extend_from_slice(&count.to_le_bytes());
        t.extend_from_slice(&val);
    };
    ent(0x0112, 3, 1, [1, 0, 0, 0]);
    ent(0x011A, 5, 1, (xres as u32).to_le_bytes());
    ent(0x011B, 5, 1, (yres as u32).to_le_bytes());
    ent(0x0128, 3, 1, [2, 0, 0, 0]);
    t.extend_from_slice(&(ifd1 as u32).to_le_bytes());
    t.resize(xres, 0);
    t.extend_from_slice(&1u32.to_le_bytes());
    t.extend_from_slice(&1u32.to_le_bytes());
    t.extend_from_slice(&1u32.to_le_bytes());
    t.extend_from_slice(&1u32.to_le_bytes());
    // IFD1：6 个条目，缩略图数据在 IFD1 之后
    let entries1 = 6usize;
    let data1 = ifd1 + 2 + entries1 * 12 + 4;
    let thumb_at = data1;
    t.resize(ifd1, 0);
    t.extend_from_slice(&(entries1 as u16).to_le_bytes());
    let xres1 = ifd1 + 2 + entries1 * 12 + 4;
    let _ = xres1;
    let mut ent = |tag: u16, ty: u16, count: u32, val: [u8; 4]| {
        t.extend_from_slice(&tag.to_le_bytes());
        t.extend_from_slice(&ty.to_le_bytes());
        t.extend_from_slice(&count.to_le_bytes());
        t.extend_from_slice(&val);
    };
    ent(0x0103, 3, 1, [6, 0, 0, 0]);
    // IFD1 的分辨率直接指回 IFD0 那两份有理数，省两份数据
    ent(0x011A, 5, 1, (xres as u32).to_le_bytes());
    ent(0x011B, 5, 1, (yres as u32).to_le_bytes());
    ent(0x0128, 3, 1, [2, 0, 0, 0]);
    ent(0x0201, 4, 1, (thumb_at as u32).to_le_bytes());
    ent(0x0202, 4, 1, (thumb.len() as u32).to_le_bytes());
    t.extend_from_slice(&0u32.to_le_bytes());
    t.resize(thumb_at, 0);
    t.extend_from_slice(&thumb);
    let mut out = Vec::with_capacity(t.len() + 6);
    out.extend_from_slice(b"Exif\0\0");
    out.extend_from_slice(&t);
    out
}

fn make_thumb(img: &RgbaImage) -> Vec<u8> {
    let (w, h) = (img.width().max(1), img.height().max(1));
    let scale = (160.0 / w as f64).min(120.0 / h as f64).min(1.0);
    let tw = ((w as f64 * scale).round() as u32).max(1);
    let th = ((h as f64 * scale).round() as u32).max(1);
    let small = if tw == w && th == h {
        img.clone()
    } else {
        image::imageops::resize(img, tw, th, image::imageops::FilterType::Triangle)
    };
    let mut out = Vec::new();
    let _ = jpeg_encoder::Encoder::new(&mut out, 70).encode(
        small.as_raw(),
        tw as u16,
        th as u16,
        jpeg_encoder::ColorType::Rgba,
    );
    out
}

//! 文件格式牌：竖版圆角牌 + 大写 X + 下方的格式名。
//!
//! 浏览窗口的列表/详细两档、以及资源管理器里的文件图标都从这一套绘制里出图，
//! 牌面的形状和颜色只有一份。系统图标的牌占得更满，字号跟着放大，格式名在
//! 48 像素的中图标视图里也读得清；列表档图标只有二十来像素，只留格式名并把牌压扁。

use std::cell::RefCell;
use std::collections::HashMap;

use windows::core::PCWSTR;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLIP_DEFAULT_PRECIS, CreateFontW,
    CreatePen, CreateSolidBrush, DEFAULT_CHARSET, DIB_RGB_COLORS, DeleteObject, FW_BOLD,
    FW_EXTRABOLD, GGO_METRICS, GLYPHMETRICS, GetGlyphOutlineW, GetTextExtentPoint32W, HDC, HFONT,
    HGDIOBJ, MAT2, OUT_DEFAULT_PRECIS, PS_GEOMETRIC, RoundRect, SelectObject, SetBkMode,
    SetDIBitsToDevice, SetTextAlign, SetTextColor, TA_BASELINE, TA_LEFT, TEXT_ALIGN_OPTIONS,
    TextOutW, TRANSPARENT, FIXED,
};

use crate::theme::{cref, lum, mix};

pub struct Fmt {
    pub label: &'static str,
    /// 浅底上的标准色。
    pub ink: u32,
    /// 深底（选中行、深色皮肤）上用同一色相亮一档，不然整块糊在背景里。
    pub lit: u32,
}

/// 顺序就是资源号 101.. 的顺序，改顺序会让已经设好的系统图标认错格式。
pub const FMTS: [Fmt; 10] = [
    Fmt { label: "JPG", ink: 0xc2762c, lit: 0xd9924d },
    Fmt { label: "PNG", ink: 0x2e8b74, lit: 0x3fae99 },
    Fmt { label: "PSD", ink: 0x5566b8, lit: 0x7a8bda },
    Fmt { label: "GIF", ink: 0xa5568f, lit: 0xc77fb3 },
    Fmt { label: "BMP", ink: 0x3f7fa8, lit: 0x6aa3c8 },
    Fmt { label: "TIF", ink: 0x6f7d86, lit: 0x9aa8b1 },
    Fmt { label: "WEBP", ink: 0x1f8a99, lit: 0x49b0bf },
    Fmt { label: "HEIC", ink: 0x7a68a8, lit: 0xa494d0 },
    Fmt { label: "ICO", ink: 0xb35c4e, lit: 0xd88a7d },
    Fmt { label: "RAW", ink: 0x6b7355, lit: 0x98a07c },
];

/// 扩展名 → 格式牌。RAW 家族共用一个牌，具体后缀不另起颜色。
pub fn index(ext: &str) -> Option<usize> {
    Some(match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "jpe" | "jfif" => 0,
        "png" => 1,
        "psd" | "psb" => 2,
        "gif" => 3,
        "bmp" => 4,
        "tif" | "tiff" => 5,
        "webp" => 6,
        "heic" => 7,
        "ico" => 8,
        "nef" | "cr2" | "cr3" | "arw" | "rw2" | "dng" | "orf" | "raf" | "pef" | "srw" | "x3f"
        | "3fr" | "fff" | "iiq" => 9,
        _ => return None,
    })
}

/// 按底色挑标准色还是亮一档。
pub fn pick(idx: usize, bg: u32) -> u32 {
    if lum(bg) < 0.18 { FMTS[idx].lit } else { FMTS[idx].ink }
}

/// 一套版式，坐标都在 24 的网格里。
struct Geo {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    rad: f64,
    /// X 的宽度，按软件图标那枚的宽高比往下排。
    x_w: f64,
    name_em: f64,
    /// X 的墨迹顶、格式名的墨迹底。
    x_top: f64,
    name_bot: f64,
    has_x: bool,
    /// 格式名超出牌内可用宽度时按比例缩字号。
    fit: bool,
}

/// 窗口里的定稿版式。
const GEO_UI: Geo = Geo {
    left: 3.2,
    top: 2.4,
    right: 20.8,
    bottom: 21.6,
    rad: 8.0,
    x_w: 13.5,
    name_em: 4.5,
    x_top: 4.35,
    name_bot: 19.44,
    has_x: true,
    fit: false,
};

/// 浏览窗口的列表档：行内图标只有二十来像素，X 会把格式名挤到看不清；
/// 这一档只留格式名居中，牌也压扁一截，行与行之间不再贴在一起。
const GEO_LIST: Geo = Geo {
    left: 3.2,
    top: 4.2,
    right: 20.8,
    bottom: 19.8,
    rad: 5.5,
    x_w: 0.0,
    name_em: 9.0,
    x_top: 0.0,
    name_bot: 0.0,
    has_x: false,
    fit: true,
};

/// 系统图标的版式：构图和窗口里一样（牌 + X + 格式名），但牌放大到接近满格，
/// 字号跟着抬一档。资源管理器里同一格中，它和其他满格图标一样大。
const GEO_ICON: Geo = Geo {
    left: 1.6,
    top: 1.2,
    right: 22.4,
    bottom: 22.8,
    rad: 9.0,
    x_w: 14.8,
    name_em: 6.6,
    x_top: 3.15,
    name_bot: 20.9,
    has_x: true,
    fit: true,
};

/// 画一枚格式牌。`bg` 只用来把牌面的 10% 填色预混成实色，
/// 牌外一个像素都不碰（缓存层用哨兵色抠底）。
pub unsafe fn draw(hdc: HDC, x: i32, y: i32, sz: i32, idx: usize, ink: u32, bg: u32) {
    paint(hdc, x, y, sz, idx, ink, bg, &GEO_UI);
}

/// 资源管理器里的格式图标。牌比窗口里那版占得满，不再分尺寸换构图。
pub unsafe fn draw_icon(hdc: HDC, x: i32, y: i32, sz: i32, idx: usize, ink: u32, bg: u32) {
    paint(hdc, x, y, sz, idx, ink, bg, &GEO_ICON);
}

/// 列表档的格式牌：没有 X，只有居中放大的格式名。
pub unsafe fn draw_list(hdc: HDC, x: i32, y: i32, sz: i32, idx: usize, ink: u32, bg: u32) {
    paint(hdc, x, y, sz, idx, ink, bg, &GEO_LIST);
}

unsafe fn paint(hdc: HDC, x: i32, y: i32, sz: i32, idx: usize, ink: u32, bg: u32, g: &Geo) {
    let u = sz as f64 / 24.0;
    let left = x + (g.left * u).round() as i32;
    let top = y + (g.top * u).round() as i32;
    // RoundRect 的右/下是开区间，补一格才和网格里的牌面对齐。
    let right = x + (g.right * u).round() as i32 + 1;
    let bottom = y + (g.bottom * u).round() as i32 + 1;
    let rad = (g.rad * u).round().max(2.0) as i32;
    let brush = CreateSolidBrush(cref(mix(ink, bg, 0.90)));
    let pen = CreatePen(PS_GEOMETRIC, (1.5 * u).round().max(1.0) as i32, cref(ink));
    let ob = SelectObject(hdc, HGDIOBJ(brush.0));
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let _ = RoundRect(hdc, left, top, right, bottom, rad, rad);
    SelectObject(hdc, op);
    SelectObject(hdc, ob);
    let _ = DeleteObject(HGDIOBJ(pen.0));
    let _ = DeleteObject(HGDIOBJ(brush.0));

    if g.has_x {
        // 牌上的 X 直接照软件图标里那枚抠下来，只换颜色：形状、宽高比、
        // 两笔的深浅和交叉处的叠色都原样保留。
        stamp_x(hdc, x, y, u, ink, bg, g);
    }

    let label = FMTS[idx].label;
    let maxw = if g.fit {
        // 有 X 的档左右各留 1.8，只有名字的档留 1.0，名字再大也不碰边框。
        let pad = if g.has_x { 3.6 } else { 2.0 };
        (((g.right - g.left) - pad) * u).round() as i32
    } else {
        0
    };
    let fnm = if g.fit {
        fit_font(hdc, cell(g.name_em / 24.0 * sz as f64), label, maxw)
    } else {
        font(cell(g.name_em / 24.0 * sz as f64), FW_BOLD.0 as i32)
    };
    if !fnm.is_invalid() {
        let center = y + (12.0 * u).round() as i32;
        let base = match ink_box(hdc, fnm, label) {
            Some((t, b)) => {
                if g.has_x {
                    y + (g.name_bot * u).round() as i32 - b
                } else {
                    center + (t + b) / 2
                }
            }
            None => {
                if g.has_x {
                    y + (g.name_bot * u).round() as i32
                } else {
                    center
                }
            }
        };
        put(hdc, fnm, label, x, sz, base, ink);
    }
}

/// 软件图标那枚 X 原样采下来的蒙版，宽高 220×186。每像素两个字节：
/// 覆盖度、深浅（0 = 亮笔，255 = 暗笔）。
static XMASK: &[u8] = include_bytes!("../resources/xmask.bin");
const XMASK_W: i32 = 220;
const XMASK_H: i32 = 186;

/// 蒙版双线性取样，返回（覆盖度，深浅）。深浅按覆盖度加权，边缘不串色。
fn mask_at(mx: f64, my: f64) -> (f64, f64) {
    let fx = mx.clamp(0.0, (XMASK_W - 1) as f64);
    let fy = my.clamp(0.0, (XMASK_H - 1) as f64);
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let x1 = (x0 + 1).min(XMASK_W - 1);
    let y1 = (y0 + 1).min(XMASK_H - 1);
    let tx = fx - x0 as f64;
    let ty = fy - y0 as f64;
    let mut cov = 0.0;
    let mut tone = 0.0;
    for (yy, wy) in [(y0, 1.0 - ty), (y1, ty)] {
        for (xx, wx) in [(x0, 1.0 - tx), (x1, tx)] {
            let i = ((yy * XMASK_W + xx) * 2) as usize;
            let c = XMASK[i] as f64 / 255.0;
            let w = wx * wy;
            cov += c * w;
            tone += c * (XMASK[i + 1] as f64 / 255.0) * w;
        }
    }
    (cov, if cov > 1e-6 { tone / cov } else { 0.5 })
}

fn split3(c: u32) -> (i32, i32, i32) {
    (((c >> 16) & 255) as i32, ((c >> 8) & 255) as i32, (c & 255) as i32)
}

/// 直线插值。蒙版每一格的颜色都靠它混出来，走线性空间才不会把过渡压暗。
fn lerp(a: u32, b: u32, t: f64) -> u32 {
    let (ar, ag, ab) = split3(a);
    let (br, bg, bb) = split3(b);
    let m = |p: i32, q: i32| (p as f64 + (q - p) as f64 * t).round().clamp(0.0, 255.0) as u32;
    (m(ar, br) << 16) | (m(ag, bg) << 8) | m(ab, bb)
}

/// 把 X 按目标尺寸重采样成一块 32 位点阵直接贴上去。缩小时按盒式平均取样，
/// 48 像素的牌上边才不锯齿；牌底是实色，边缘可以直接混进底色，不用再描一遍。
unsafe fn stamp_x(hdc: HDC, x: i32, y: i32, u: f64, ink: u32, bg: u32, g: &Geo) {
    let w = (g.x_w * u).round().max(2.0) as i32;
    let h = ((w as f64) * XMASK_H as f64 / XMASK_W as f64).round().max(2.0) as i32;
    let x0 = x + (12.0 * u).round() as i32 - w / 2;
    let y0 = y + (g.x_top * u).round() as i32;
    let sx = XMASK_W as f64 / w as f64;
    let sy = XMASK_H as f64 / h as f64;
    let steps = sx.max(sy).ceil().clamp(1.0, 10.0) as i32;
    let plate = mix(ink, bg, 0.90);
    // 亮笔就是格式的标准色，暗笔压到六成，深浅比照软件图标那两笔。
    let dark = lerp(ink, 0, 0.40);
    let mut buf = vec![0u32; (w * h) as usize];
    for dy in 0..h {
        for dx in 0..w {
            let mut cov = 0.0;
            let mut tone = 0.0;
            for j in 0..steps {
                for i in 0..steps {
                    let mx = (dx as f64 + (i as f64 + 0.5) / steps as f64) * sx;
                    let my = (dy as f64 + (j as f64 + 0.5) / steps as f64) * sy;
                    let (c, t) = mask_at(mx, my);
                    cov += c;
                    tone += c * t;
                }
            }
            let n = (steps * steps) as f64;
            cov /= n;
            let t = if cov > 1e-6 { tone / (cov * n) } else { 0.5 };
            buf[(dy * w + dx) as usize] = lerp(plate, lerp(ink, dark, t), cov);
        }
    }
    let bmi = BITMAPINFO {
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
    let _ = SetDIBitsToDevice(
        hdc,
        x0,
        y0,
        w as u32,
        h as u32,
        0,
        0,
        0,
        h as u32,
        buf.as_ptr() as *const core::ffi::c_void,
        &bmi,
        DIB_RGB_COLORS,
    );
}

/// 字号直接就是 lfHeight：GDI 这个值量出来就是字形的 em 大小，
/// 早先按 ascent+descent 乘 1.33 会把整块放大三成。
fn cell(em: f64) -> i32 {
    (em.round() as i32).max(5)
}

/// 名字放不下时按宽度缩一档；16 像素档的四个字母要靠这步留住牌内。
unsafe fn fit_font(hdc: HDC, em: i32, s: &str, maxw: i32) -> HFONT {
    let f = font(em, FW_EXTRABOLD.0 as i32);
    if maxw <= 0 || f.is_invalid() {
        return f;
    }
    let w = text_w(hdc, f, s);
    if w <= maxw {
        return f;
    }
    let n = (em as f64 * maxw as f64 / w as f64).floor() as i32;
    font(n.max(4), FW_EXTRABOLD.0 as i32)
}

thread_local! {
    static FONTS: RefCell<HashMap<(i32, i32), HFONT>> = RefCell::new(HashMap::new());
}

static SEGOE: [u16; 9] = [0x53, 0x65, 0x67, 0x6f, 0x65, 0x20, 0x55, 0x49, 0];

unsafe fn font(cell: i32, weight: i32) -> HFONT {
    if let Some(f) = FONTS.with(|c| c.borrow().get(&(cell, weight)).copied()) {
        return f;
    }
    // 换缩放会攒出新字号，表大了就清一遍旧的；字体很便宜，进程里也不会同时用很多档。
    if FONTS.with(|c| c.borrow().len()) >= 64 {
        FONTS.with(|c| {
            for (_, f) in c.borrow_mut().drain() {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        });
    }
    let f = CreateFontW(
        -cell,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        // 图标缓存要整块抠底贴到各种底色上，ClearType 的彩边会留在像素里。
        ANTIALIASED_QUALITY,
        0,
        PCWSTR(SEGOE.as_ptr()),
    );
    FONTS.with(|c| c.borrow_mut().insert((cell, weight), f));
    f
}

/// 一个字的墨迹：相对基线的上沿距离（正数往上）、墨迹高。
unsafe fn glyph(hdc: HDC, f: HFONT, ch: char) -> (i32, u32) {
    let old = SelectObject(hdc, HGDIOBJ(f.0));
    let mut gm = GLYPHMETRICS::default();
    let mat = MAT2 {
        eM11: FIXED { fract: 0, value: 1 },
        eM12: FIXED { fract: 0, value: 0 },
        eM21: FIXED { fract: 0, value: 0 },
        eM22: FIXED { fract: 0, value: 1 },
    };
    let r = GetGlyphOutlineW(hdc, ch as u32, GGO_METRICS, &mut gm, 0, None, &mat);
    SelectObject(hdc, old);
    if r == u32::MAX || gm.gmBlackBoxY == 0 {
        (0, 0)
    } else {
        (gm.gmptGlyphOrigin.y, gm.gmBlackBoxY)
    }
}

/// 一整串字的墨迹上下沿（相对基线，向上为正；下沿一般是负数）。
unsafe fn ink_box(hdc: HDC, f: HFONT, s: &str) -> Option<(i32, i32)> {
    let mut top = 0i32;
    let mut bot = i32::MAX;
    let mut any = false;
    for ch in s.chars() {
        let (oy, bh) = glyph(hdc, f, ch);
        if bh == 0 {
            continue;
        }
        any = true;
        top = top.max(oy);
        bot = bot.min(oy - bh as i32);
    }
    any.then_some((top, bot))
}

unsafe fn text_w(hdc: HDC, f: HFONT, s: &str) -> i32 {
    let old = SelectObject(hdc, HGDIOBJ(f.0));
    let w: Vec<u16> = s.encode_utf16().collect();
    let mut size = SIZE::default();
    let _ = GetTextExtentPoint32W(hdc, &w, &mut size);
    SelectObject(hdc, old);
    size.cx
}

/// 按基线居中写一行字。字号小到只有几个像素，位置只能用基线锁，不能用格顶。
unsafe fn put(hdc: HDC, f: HFONT, s: &str, x: i32, sz: i32, base: i32, ink: u32) {
    let old = SelectObject(hdc, HGDIOBJ(f.0));
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, cref(ink));
    let olda = SetTextAlign(hdc, TA_BASELINE | TA_LEFT);
    let mut w: Vec<u16> = s.encode_utf16().collect();
    let mut size = SIZE::default();
    let _ = GetTextExtentPoint32W(hdc, &w, &mut size);
    let _ = TextOutW(hdc, x + (sz - size.cx) / 2, base, &w);
    let _ = SetTextAlign(hdc, TEXT_ALIGN_OPTIONS(olda));
    SelectObject(hdc, old);
}

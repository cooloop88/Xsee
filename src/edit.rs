//! 编辑器的像素运算。全部在 RGBA8 上做，alpha 原样保留；
//! 参数含义按 nomacs 3.22.4 的 DkImageStorage / DkManipulatorsIpl 对齐。

use image::imageops::FilterType;
use image::RgbaImage;

/// 灰度：按亮度加权，alpha 不动。
pub fn grayscale(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let [r, g, b, a] = p.0;
        let y = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32).round() as u8;
        p.0 = [y, y, y, a];
    }
}

pub fn invert(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let [r, g, b, a] = p.0;
        p.0 = [255 - r, 255 - g, 255 - b, a];
    }
}

/// 自动调整：每个通道各自拉满直方图；本来就从 0 铺到 255 的通道不动。
pub fn auto_adjust(img: &mut RgbaImage) {
    let mut lo = [255u8; 3];
    let mut hi = [0u8; 3];
    for p in img.pixels() {
        for c in 0..3 {
            lo[c] = lo[c].min(p.0[c]);
            hi[c] = hi[c].max(p.0[c]);
        }
    }
    let mut keep = [false; 3];
    for c in 0..3 {
        keep[c] = hi[c] == lo[c] || (lo[c] == 0 && hi[c] == 255);
    }
    if keep.iter().all(|k| *k) {
        return;
    }
    let lut = channel_lut(lo, hi, keep);
    apply_lut(img, &lut);
}

/// 标准化：三个通道共用一组最小/最大值，只拉伸整体明暗。
pub fn normalize(img: &mut RgbaImage) {
    let mut lo = 255u8;
    let mut hi = 0u8;
    for p in img.pixels() {
        for c in 0..3 {
            lo = lo.min(p.0[c]);
            hi = hi.max(p.0[c]);
        }
    }
    if hi == lo || (lo == 0 && hi == 255) {
        return;
    }
    let lut = channel_lut([lo; 3], [hi; 3], [false; 3]);
    apply_lut(img, &lut);
}

fn channel_lut(lo: [u8; 3], hi: [u8; 3], keep: [bool; 3]) -> [[u8; 256]; 3] {
    let mut lut = [[0u8; 256]; 3];
    for c in 0..3 {
        for v in 0..256usize {
            lut[c][v] = if keep[c] {
                v as u8
            } else {
                let d = (hi[c] as f32 - lo[c] as f32).max(1.0);
                (255.0 * (v as f32 - lo[c] as f32) / d).round().clamp(0.0, 255.0) as u8
            };
        }
    }
    lut
}

fn apply_lut(img: &mut RgbaImage, lut: &[[u8; 256]; 3]) {
    for p in img.pixels_mut() {
        for c in 0..3 {
            p.0[c] = lut[c][p.0[c] as usize];
        }
    }
}

/// 曝光：ev 是光圈档，+1 = 亮一倍；gamma 用查表折算。
pub fn exposure(img: &mut RgbaImage, ev: f64, gamma: f64) {
    let gain = 2f64.powf(ev);
    let inv = if gamma.is_finite() && gamma > 0.01 { 1.0 / gamma } else { 1.0 };
    let mut lut = [0u8; 256];
    for v in 0..256usize {
        let x = (v as f64 / 255.0 * gain).clamp(0.0, 1.0);
        lut[v] = (255.0 * x.powf(inv)).round().clamp(0.0, 255.0) as u8;
    }
    for p in img.pixels_mut() {
        for c in 0..3 {
            p.0[c] = lut[p.0[c] as usize];
        }
    }
}

/// 色相/饱和度：色相加角度（-180..180），饱和度乘系数，亮度加偏移（-1..1）。
pub fn hue_sat(img: &mut RgbaImage, hue_deg: f64, sat: f64, bright: f64) {
    let hs = hue_deg / 360.0;
    let bo = bright;
    for p in img.pixels_mut() {
        let r = p.0[0] as f64 / 255.0;
        let g = p.0[1] as f64 / 255.0;
        let b = p.0[2] as f64 / 255.0;
        let (mut h, mut s, mut l) = rgb_to_hsl(r, g, b);
        h = (h + hs).rem_euclid(1.0);
        s = (s * sat).clamp(0.0, 1.0);
        l = (l + bo).clamp(0.0, 1.0);
        let (nr, ng, nb) = hsl_to_rgb(h, s, l);
        let a = p.0[3];
        p.0 = [
            (nr * 255.0).round() as u8,
            (ng * 255.0).round() as u8,
            (nb * 255.0).round() as u8,
            a,
        ];
    }
}

/// 颜色均衡：三个通道各加一个偏移，-100..100 映射到 -255..255。
pub fn color_balance(img: &mut RgbaImage, dr: f64, dg: f64, db: f64) {
    let sh = [dr, dg, db].map(|d| (d / 100.0 * 255.0).round() as i32);
    for p in img.pixels_mut() {
        for c in 0..3 {
            p.0[c] = (p.0[c] as i32 + sh[c]).clamp(0, 255) as u8;
        }
    }
}

pub fn threshold(img: &mut RgbaImage, thr: u8) {
    grayscale(img);
    for p in img.pixels_mut() {
        let v = if p.0[0] > thr { 255 } else { 0 };
        p.0[0] = v;
        p.0[1] = v;
        p.0[2] = v;
    }
}

/// 方框模糊做三遍，近似高斯；半径按像素算。
pub fn blur(img: &mut RgbaImage, radius: f32) {
    let r = radius.round().clamp(0.0, 200.0) as usize;
    if r == 0 {
        return;
    }
    for _ in 0..3 {
        box_pass(img, r, true);
        box_pass(img, r, false);
    }
}

fn box_pass(img: &mut RgbaImage, r: usize, horiz: bool) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let src = img.as_raw().clone();
    let mut out = src.clone();
    if horiz {
        for y in 0..h {
            for c in 0..4 {
                let mut sum = 0u32;
                for x in 0..=r.min(w - 1) {
                    sum += src[(y * w + x) * 4 + c] as u32;
                }
                for x in 0..w {
                    let n = (x + r).min(w - 1) - x.saturating_sub(r) + 1;
                    out[(y * w + x) * 4 + c] = (sum / n as u32).min(255) as u8;
                    if x >= r {
                        sum -= src[(y * w + x - r) * 4 + c] as u32;
                    }
                    let add = x + r + 1;
                    if add < w {
                        sum += src[(y * w + add) * 4 + c] as u32;
                    }
                }
            }
        }
    } else {
        for x in 0..w {
            for c in 0..4 {
                let mut sum = 0u32;
                for y in 0..=r.min(h - 1) {
                    sum += src[(y * w + x) * 4 + c] as u32;
                }
                for y in 0..h {
                    let n = (y + r).min(h - 1) - y.saturating_sub(r) + 1;
                    out[(y * w + x) * 4 + c] = (sum / n as u32).min(255) as u8;
                    if y >= r {
                        sum -= src[((y - r) * w + x) * 4 + c] as u32;
                    }
                    let add = y + r + 1;
                    if add < h {
                        sum += src[(add * w + x) * 4 + c] as u32;
                    }
                }
            }
        }
    }
    img.copy_from_slice(&out);
}

/// USM：原图与模糊图按权重混合，weight 1 = 原图，越大越锐。
pub fn sharpen(img: &mut RgbaImage, sigma: f32, amount: f32) {
    let mut g = img.clone();
    blur(&mut g, sigma.max(0.5));
    let a = amount.clamp(0.0, 3.0);
    for (p, q) in img.pixels_mut().zip(g.pixels()) {
        for c in 0..3 {
            let v = p.0[c] as f32 + a * (p.0[c] as f32 - q.0[c] as f32);
            p.0[c] = v.round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// 消除斑点：3×3 中值，只动 RGB。
pub fn despeckle(img: &mut RgbaImage) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w < 3 || h < 3 {
        return;
    }
    let src = img.as_raw().clone();
    let mut out = src.clone();
    let mut v = [0u8; 9];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            for c in 0..3 {
                let mut k = 0;
                for dy in 0..3 {
                    for dx in 0..3 {
                        v[k] = src[((y + dy - 1) * w + (x + dx - 1)) * 4 + c];
                        k += 1;
                    }
                }
                v.sort_unstable();
                out[(y * w + x) * 4 + c] = v[4];
            }
        }
    }
    img.copy_from_slice(&out);
}

pub fn rotate90(img: &RgbaImage, times: u8) -> RgbaImage {
    let t = times % 4;
    let (w, h) = (img.width(), img.height());
    let mut out = if t % 2 == 1 {
        RgbaImage::new(h, w)
    } else {
        RgbaImage::new(w, h)
    };
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            let (nx, ny) = match t {
                1 => (h - 1 - y, x),
                2 => (w - 1 - x, h - 1 - y),
                3 => (y, w - 1 - x),
                _ => (x, y),
            };
            out.put_pixel(nx, ny, *p);
        }
    }
    out
}

pub fn flip_h(img: &RgbaImage) -> RgbaImage {
    image::imageops::flip_horizontal(img)
}

pub fn flip_v(img: &RgbaImage) -> RgbaImage {
    image::imageops::flip_vertical(img)
}

/// 任意角度旋转，双线性取样，画布扩大到刚好装下，空白处透明。
pub fn rotate_any(img: &RgbaImage, deg: f64) -> RgbaImage {
    let a = deg.to_radians();
    let (s, c) = (a.sin().abs(), a.cos().abs());
    let nw = (img.width() as f64 * c + img.height() as f64 * s).ceil().max(1.0) as u32;
    let nh = (img.width() as f64 * s + img.height() as f64 * c).ceil().max(1.0) as u32;
    let (cx, cy) = (img.width() as f64 / 2.0, img.height() as f64 / 2.0);
    let (ncx, ncy) = (nw as f64 / 2.0, nh as f64 / 2.0);
    let (sin, cos) = (a.sin(), a.cos());
    let mut out = RgbaImage::new(nw, nh);
    for y in 0..nh {
        for x in 0..nw {
            let dx = x as f64 + 0.5 - ncx;
            let dy = y as f64 + 0.5 - ncy;
            let sx = cos * dx + sin * dy + cx - 0.5;
            let sy = -sin * dx + cos * dy + cy - 0.5;
            if sx < 0.0 || sy < 0.0 || sx >= img.width() as f64 - 1.0 || sy >= img.height() as f64 - 1.0 {
                continue;
            }
            let x0 = sx.floor() as u32;
            let y0 = sy.floor() as u32;
            let fx = sx - x0 as f64;
            let fy = sy - y0 as f64;
            let p00 = img.get_pixel(x0, y0).0;
            let p10 = img.get_pixel(x0 + 1, y0).0;
            let p01 = img.get_pixel(x0, y0 + 1).0;
            let p11 = img.get_pixel(x0 + 1, y0 + 1).0;
            let mut q = [0u8; 4];
            for k in 0..4 {
                let v = p00[k] as f64 * (1.0 - fx) * (1.0 - fy)
                    + p10[k] as f64 * fx * (1.0 - fy)
                    + p01[k] as f64 * (1.0 - fx) * fy
                    + p11[k] as f64 * fx * fy;
                q[k] = v.round().clamp(0.0, 255.0) as u8;
            }
            out.put_pixel(x, y, image::Rgba(q));
        }
    }
    out
}

pub fn resize(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    image::imageops::resize(img, w.max(1), h.max(1), FilterType::Lanczos3)
}

pub fn resize_filter(img: &RgbaImage, w: u32, h: u32, filter: usize) -> RgbaImage {
    let f = match filter {
        1 => FilterType::CatmullRom,
        2 => FilterType::Triangle,
        3 => FilterType::Nearest,
        _ => FilterType::Lanczos3,
    };
    image::imageops::resize(img, w.max(1), h.max(1), f)
}

pub fn crop(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
    let x = x.min(img.width().saturating_sub(1));
    let y = y.min(img.height().saturating_sub(1));
    let w = w.max(1).min(img.width() - x);
    let h = h.max(1).min(img.height() - y);
    image::imageops::crop_imm(img, x, y, w, h).to_image()
}

/// 透明底压成指定颜色。
pub fn flatten_bg(img: &RgbaImage, c: [u8; 3]) -> RgbaImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let a = p.0[3] as f32 / 255.0;
        for i in 0..3 {
            p.0[i] = (p.0[i] as f32 * a + c[i] as f32 * (1.0 - a)).round() as u8;
        }
        p.0[3] = 255;
    }
    out
}

fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-9 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, s, l)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    if s <= 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |mut t: f64| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0))
}

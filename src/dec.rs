use windows::core::Result;
use windows::Win32::Graphics::Imaging::*;

use crate::Rgba;

/// 实测四种格式两个目标尺寸得出来的两档，不是按画质常识挑的：
/// 缩到 512 以内时三次和线性同速（tif 20 vs 10ms、jpeg 418 vs 417ms），那就留画质；
/// 预览尺寸三次和 Fant 都要贵一倍（jpeg 1029 / 530 vs 线性 515ms），改线性。
/// Fant 在 TIF 上是陷阱：同一张 1.9GB TIF 出 173px 缩略图 1197ms，线性只要 10ms。
fn pick_mode(tw: u32, th: u32) -> WICBitmapInterpolationMode {
    if tw.max(th) <= 512 {
        WICBitmapInterpolationModeCubic
    } else {
        WICBitmapInterpolationModeLinear
    }
}

/// 目标框内接尺寸。小图不放大。
pub fn fit(iw: u32, ih: u32, bw: u32, bh: u32) -> (u32, u32) {
    let s = (bw as f64 / iw as f64).min(bh as f64 / ih as f64);
    if s >= 1.0 {
        return (iw.max(1), ih.max(1));
    }
    (
        ((iw as f64 * s).round() as u32).max(1),
        ((ih as f64 * s).round() as u32).max(1),
    )
}

/// 解到目标框：WIC 缩放 -> 统一 32bppBGRA -> 紧凑 RGBA。
/// 挂 scaler 而不是自己按 DCT 档分两步，是要验 WIC 会不会把 JPEG 的 DCT 缩放
/// 并进这条管线里；并进来了才不会落地全尺寸位图。
pub fn source_to_box(
    factory: &IWICImagingFactory,
    src: &IWICBitmapSource,
    bw: u32,
    bh: u32,
) -> Result<Rgba> {
    let (tw, th, buf) = source_to_box_bgra(factory, src, bw, bh)?;
    Ok(Rgba {
        w: tw,
        h: th,
        px: bgra_to_rgba(&buf, tw * 4, tw, th),
    })
}

/// 同上，但保留 GDI 直接能用的 BGRA 行对齐缓冲，不做通道翻转。插值档自动挑。
pub fn source_to_box_bgra(
    factory: &IWICImagingFactory,
    src: &IWICBitmapSource,
    bw: u32,
    bh: u32,
) -> Result<(u32, u32, Vec<u8>)> {
    source_to_box_bgra_mode(factory, src, bw, bh, None)
}

/// 同上，可以钉死插值档做对比测量；传 None 走自动档。
pub fn source_to_box_bgra_mode(
    factory: &IWICImagingFactory,
    src: &IWICBitmapSource,
    bw: u32,
    bh: u32,
    mode: Option<WICBitmapInterpolationMode>,
) -> Result<(u32, u32, Vec<u8>)> {
    let (mut nw, mut nh) = (0u32, 0u32);
    unsafe {
        src.GetSize(&mut nw, &mut nh)?;
    }
    let (tw, th) = fit(nw, nh, bw, bh);
    let mode = mode.unwrap_or_else(|| pick_mode(tw, th));
    let stride = tw * 4;
    let mut buf = vec![0u8; stride as usize * th as usize];
    unsafe {
        let scaler = factory.CreateBitmapScaler()?;
        scaler.Initialize(src, tw, th, mode)?;
        let conv = factory.CreateFormatConverter()?;
        conv.Initialize(
            &scaler,
            &GUID_WICPixelFormat32bppBGRA,
            WICBitmapDitherTypeNone,
            None::<&IWICPalette>,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
        let s: &IWICBitmapSource = &conv;
        s.CopyPixels(std::ptr::null(), stride, &mut buf)?;
    }
    Ok((tw, th, buf))
}

/// 按 EXIF 方向档把出图摆正：4 字节一像素的紧凑缓冲，整颗像素搬，
/// 所以 RGBA（缩略图）和 BGRA（看图）两条线共用这一个实现。宽高就地改。
/// 档 5/7 按 PIL/ExifTool 的口径：5 是主对角线镜像，7 是副对角线镜像。
pub fn orient(px: &mut Vec<u8>, w: &mut u32, h: &mut u32, o: u16) {
    if o <= 1 || o > 8 || px.is_empty() {
        return;
    }
    let (sw, sh) = (*w as usize, *h as usize);
    let (dw, dh) = if o >= 5 { (sh, sw) } else { (sw, sh) };
    if sw * sh * 4 != px.len() {
        return;
    }
    let mut out = vec![0u8; px.len()];
    for y in 0..sh {
        for x in 0..sw {
            let d = match o {
                2 => y * dw + (sw - 1 - x),
                3 => (sh - 1 - y) * dw + (sw - 1 - x),
                4 => (sh - 1 - y) * dw + x,
                5 => x * dw + y,
                6 => x * dw + (sh - 1 - y),
                7 => (dh - 1 - x) * dw + (sh - 1 - y),
                8 => (dh - 1 - x) * dw + y,
                _ => return,
            } * 4;
            let s = (y * sw + x) * 4;
            out[d..d + 4].copy_from_slice(&px[s..s + 4]);
        }
    }
    *px = out;
    *w = dw as u32;
    *h = dh as u32;
}

/// BGRA 行对齐缓冲 -> 紧凑 RGBA。尺寸已经很小，逐像素换即可。
pub(crate) fn bgra_to_rgba(buf: &[u8], stride: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0u8; w as usize * h as usize * 4];
    for y in 0..h as usize {
        let row = &buf[y * stride as usize..][..w as usize * 4];
        let dst = &mut out[y * w as usize * 4..][..w as usize * 4];
        for (d, s) in dst.chunks_exact_mut(4).zip(row.chunks_exact(4)) {
            d[0] = s[2];
            d[1] = s[1];
            d[2] = s[0];
            d[3] = 0xFF;
        }
    }
    out
}

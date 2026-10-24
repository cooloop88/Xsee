use std::error::Error;
use std::time::Instant;

use windows::core::{Interface, GUID, HSTRING};
use windows::Win32::Foundation::GENERIC_READ;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};

fn scale_to(w: u32, h: u32, long_edge: u32) -> (u32, u32) {
    if w >= h {
        (long_edge, (h as u64 * long_edge as u64 / w as u64) as u32)
    } else {
        ((w as u64 * long_edge as u64 / h as u64) as u32, long_edge)
    }
}

fn bytes_of(g: Option<GUID>) -> u32 {
    match g {
        Some(x) if x == GUID_WICPixelFormat24bppRGB => 3,
        Some(x) if x == GUID_WICPixelFormat24bppBGR => 3,
        _ => 4,
    }
}

unsafe fn probe(path: &str) -> Result<(), Box<dyn Error>> {
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    let factory: IWICImagingFactory =
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
    let dec = factory.CreateDecoderFromFilename(
        &HSTRING::from(path),
        None,
        GENERIC_READ,
        WICDecodeMetadataCacheOnDemand,
    )?;
    let frame = dec.GetFrame(0)?;
    let (mut nw, mut nh) = (0u32, 0u32);
    frame.GetSize(&mut nw, &mut nh)?;
    println!("  native {}x{}", nw, nh);

    let t = Instant::now();
    let stride = nw * 4;
    let mut buf = vec![0u8; (stride * nh) as usize];
    frame.CopyPixels(std::ptr::null(), stride, &mut buf)?;
    println!(
        "  WIC full          {:>5}x{:<5} {:>7.1} MB  {:>8.1} ms",
        nw,
        nh,
        buf.len() as f64 / 1048576.0,
        t.elapsed().as_secs_f64() * 1000.0
    );

    let tr: IWICBitmapSourceTransform = frame.cast()?;
    for target in [173u32, 769u32] {
        let (dw, dh) = scale_to(nw, nh, target);
        let (mut aw, mut ah) = (dw, dh);
        tr.GetClosestSize(&mut aw, &mut ah)?;
        println!("  --- ask {}x{} -> closest {}x{} ---", dw, dh, aw, ah);
        let cands: [Option<GUID>; 4] = [
            Some(GUID_WICPixelFormat24bppRGB),
            Some(GUID_WICPixelFormat32bppBGR),
            Some(GUID_WICPixelFormatDontCare),
            None,
        ];
        for cand in cands {
            let pfmt = match &cand {
                Some(g) => g as *const GUID,
                None => std::ptr::null(),
            };
            let bps = bytes_of(cand);
            let stride = (aw * bps + 3) & !3;
            let mut buf = vec![0u8; (stride * ah) as usize];
            let t = Instant::now();
            let r = tr.CopyPixels(
                std::ptr::null(),
                aw,
                ah,
                pfmt,
                WICBitmapTransformRotate0,
                stride,
                &mut buf,
            );
            match r {
                Ok(()) => {
                    let ms = t.elapsed().as_secs_f64() * 1000.0;
                    let mut rgb: Vec<u8> = Vec::with_capacity((aw * ah * 3) as usize);
                    if bps == 3 {
                        for p in buf.chunks_exact(stride as usize) {
                            for q in p[..(aw * 3) as usize].chunks_exact(3) {
                                rgb.extend_from_slice(&[q[2], q[1], q[0]]);
                            }
                        }
                    } else {
                        for p in buf.chunks_exact(stride as usize) {
                            for q in p[..(aw * 4) as usize].chunks_exact(4) {
                                rgb.extend_from_slice(&[q[2], q[1], q[0]]);
                            }
                        }
                    }
                    let img = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(aw, ah, rgb)
                        .ok_or("from_raw")?;
                    let out = std::env::temp_dir()
                        .join(format!("wic{target}_{bps}.png"))
                        .display()
                        .to_string();
                    image::DynamicImage::ImageRgb8(img).save(&out)?;
                    println!(
                        "      OK {}bpp stride={:<6} {:>6.1} MB  {:>7.1} ms  -> {}",
                        bps * 8,
                        stride,
                        buf.len() as f64 / 1048576.0,
                        ms,
                        out.rsplit('\\').next().unwrap()
                    );
                    break;
                }
                Err(e) => {
                    let tag = match cand {
                        Some(g) if g == GUID_WICPixelFormat24bppRGB => "24bppRGB",
                        Some(g) if g == GUID_WICPixelFormat32bppBGR => "32bppBGR",
                        Some(_) => "dontcare",
                        None => "null",
                    };
                    println!("      {} -> {}", tag, e)
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    // 用法：wic <图片>。取不到图片就不动，出的图都落在系统临时目录。
    let Some(path) = args.get(1).cloned() else {
        eprintln!("用法：wic <图片>");
        return Ok(());
    };
    println!("WIC probe: {path}");
    unsafe { probe(&path) }?;
    Ok(())
}

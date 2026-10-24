//! 生成 `resources/fmt/*.ico`：文件关联之后资源管理器里显示的格式图标。
//!
//! 图形和浏览窗口里那套格式牌同源，`draw_icon` 只按尺寸换版式：
//! 大图标和窗口一致，中图标放大格式名，小图标只留格式名。
//! 用法：`cargo run --release --bin mkicons`，产物随 xsee.rc 编进 exe。

use std::fs;
use std::path::Path;

use xsee_host::fmticon;
use xsee_host::theme::cref;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::*;

/// 资源管理器会在这些档里挑：16/20/24 是列表和小图标，48 是"中图标"，
/// 96 往上是"大图标"和超大图标。256 那帧 Win 要求是 PNG，这里全是 PNG。
const SIZES: [u32; 9] = [16, 20, 24, 32, 40, 48, 64, 128, 256];

/// 建图时的底：画完当透明色抠掉，牌外面的圆角才透得过桌面。
const SENTINEL: u32 = 0x00ff00ff;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("fmt");
    fs::create_dir_all(&dir).unwrap();
    for (idx, f) in fmticon::FMTS.iter().enumerate() {
        let mut frames = Vec::new();
        for s in SIZES {
            frames.push((s, unsafe { render(idx, s) }));
        }
        let name = f.label.to_ascii_lowercase();
        write_ico(&dir.join(format!("{name}.ico")), &frames);
        println!("fmt/{name}.ico");
    }
}

unsafe fn render(idx: usize, s: u32) -> Vec<u8> {
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: s as i32,
            biHeight: -(s as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let bmp = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
    let mem = CreateCompatibleDC(None);
    let old = SelectObject(mem, HGDIOBJ(bmp.0));

    let all = RECT { left: 0, top: 0, right: s as i32, bottom: s as i32 };
    let br = CreateSolidBrush(cref(SENTINEL));
    FillRect(mem, &all, br);
    let _ = DeleteObject(HGDIOBJ(br.0));

    // 牌面在资源管理器里是自带底色的：按白底预混，深浅主题下都清楚。
    fmticon::draw_icon(mem, 0, 0, s as i32, idx, fmticon::FMTS[idx].ink, 0xffffff);

    let px = std::slice::from_raw_parts(bits as *const u8, (s * s * 4) as usize);
    let mut rgba = Vec::with_capacity((s * s * 4) as usize);
    for i in 0..(s * s) as usize {
        let b = px[i * 4];
        let g = px[i * 4 + 1];
        let r = px[i * 4 + 2];
        let c = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        if c == SENTINEL {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }

    SelectObject(mem, old);
    let _ = DeleteObject(HGDIOBJ(bmp.0));
    let _ = DeleteDC(mem);

    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&rgba, s, s, ExtendedColorType::Rgba8)
        .unwrap();
    png
}

/// ICO 头 + 目录 + 每帧 PNG。所有帧都是 32 位 PNG 帧，Vista 以后都认。
fn write_ico(path: &Path, frames: &[(u32, Vec<u8>)]) {
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(frames.len() as u16).to_le_bytes());
    let mut off = 6 + 16 * frames.len() as u32;
    for (s, png) in frames {
        let side = if *s >= 256 { 0 } else { *s as u8 };
        out.push(side);
        out.push(side);
        out.push(0);
        out.push(0);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&off.to_le_bytes());
        off += png.len() as u32;
    }
    for (_, png) in frames {
        out.extend_from_slice(png);
    }
    fs::write(path, out).unwrap();
}

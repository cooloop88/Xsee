//! 把一扇自绘窗口整张拍成 PNG。命令行口和测试台都走这一份。
//! PrintWindow 不等重绘：按完动作要隔一会儿再拍，隔多久由调用方定。
use std::ffi::c_void;

use image::ImageEncoder;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

// windows 那套绑定把 PrintWindow 挂在 Win32_Storage_Xps 特性下，只为拍一张图
// 多开一整个特性不划算，这里直接接 user32。
#[link(name = "user32")]
unsafe extern "system" {
    fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> i32;
}

/// 窗口在屏幕上的矩形：左上 x,y + 宽高（含它自己画的那条标题栏）。
pub fn rect(hwnd: HWND) -> (i32, i32, i32, i32) {
    let mut rc = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &mut rc) };
    (rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top)
}

/// 拍一扇窗。`crop` 是窗口坐标下要留的那一块（x,y,宽,高），`zoom` 是整数倍放大
/// （1 = 原尺寸；量 1 像素的边线、看图标糊不糊得靠放大才判得出来）。
/// 返回落盘那张的宽高。
pub unsafe fn window(
    hwnd: HWND,
    out: &str,
    crop: Option<(i32, i32, i32, i32)>,
    zoom: i32,
) -> Result<(i32, i32), String> {
    let (_, _, ww, wh) = rect(hwnd);
    if ww <= 0 || wh <= 0 {
        return Err(format!("窗口矩形是空的（{ww}x{wh}）"));
    }
    let screen = GetDC(Some(hwnd));
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: ww,
            // 负数 = 顶朝下，行序和窗口坐标一行对一行
            biHeight: -wh,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    let dib = CreateDIBSection(Some(screen), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
        .map_err(|e| format!("建位图失败 {e}"))?;
    let mem = CreateCompatibleDC(Some(screen));
    let _ = SelectObject(mem, HGDIOBJ(dib.0));
    // 2 = PW_RENDERFULLCONTENT：自绘那面也要拿全
    let ok = PrintWindow(hwnd, mem, 2);
    let raw: Vec<u8> = if ok != 0 {
        std::slice::from_raw_parts(bits as *const u8, (ww * wh * 4) as usize).to_vec()
    } else {
        Vec::new()
    };
    let _ = DeleteDC(mem);
    let _ = DeleteObject(HGDIOBJ(dib.0));
    let _ = ReleaseDC(Some(hwnd), screen);
    if ok == 0 {
        return Err("PrintWindow 返回 0（窗口可能已经没了）".to_string());
    }
    let (cx, cy, cw, ch) = match crop {
        Some((x, y, w, h)) => {
            let (x, y) = (x.clamp(0, ww - 1), y.clamp(0, wh - 1));
            let (w, h) = (w.max(1).min(ww - x), h.max(1).min(wh - y));
            (x, y, w, h)
        }
        None => (0, 0, ww, wh),
    };
    let z = zoom.clamp(1, 8) as usize;
    let row = (cw as usize) * 4;
    // BGRA（预乘）→ RGBA，alpha 一律钉成不透：分层窗口那层半透背景拍出来会连
    // 底下的桌面一起混进来，看界面不该看那个。
    let mut base = vec![0u8; row * ch as usize];
    for y in 0..ch as usize {
        for x in 0..cw as usize {
            let s = &raw[((cy as usize + y) * ww as usize + cx as usize + x) * 4..][..4];
            let d = &mut base[y * row + x * 4..][..4];
            d.copy_from_slice(&[s[2], s[1], s[0], 255]);
        }
    }
    let mut px: Vec<u8> = Vec::with_capacity(row * z * ch as usize * z);
    for y in 0..ch as usize {
        let r = &base[y * row..(y + 1) * row];
        let line: Vec<u8> = if z == 1 {
            r.to_vec()
        } else {
            let mut v = Vec::with_capacity(row * z);
            for p in r.chunks(4) {
                for _ in 0..z {
                    v.extend_from_slice(p);
                }
            }
            v
        };
        // 行内放大完了，整行再纵向重复 z 遍
        for _ in 0..z {
            px.extend_from_slice(&line);
        }
    }
    let (ow, oh) = (cw * z as i32, ch * z as i32);
    // 只挂 PNG 那一个编码器。走 DynamicImage::save 会把 image 那套所有格式的
    // 编码器全拽进来（实测 exe 胖 4 MB），这程序体积优先。
    let mut f = std::io::BufWriter::new(
        std::fs::File::create(out).map_err(|e| format!("建 {out} 失败 {e}"))?,
    );
    image::codecs::png::PngEncoder::new(&mut f)
        .write_image(&px, ow as u32, oh as u32, image::ExtendedColorType::Rgba8)
        .map_err(|e| format!("写 {out} 失败 {e}"))?;
    Ok((ow, oh))
}

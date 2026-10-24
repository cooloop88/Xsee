#![allow(unsafe_op_in_unsafe_fn)]
/// 启动时间轴量具：照 view.exe main() 的顺序走一遍，每走完一步把步名和累计毫秒写进标题，
/// 外部用 work/vt2.py 轮询标题就能看出打开这一下卡在哪一步。不动 view.exe。
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use xsee_host::{job, scan};
use windows::core::{PCWSTR, HSTRING};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

static T0: LazyLock<Instant> = LazyLock::new(Instant::now);

fn pw(s: &str) -> &'static [u16] {
    Box::leak(
        s.encode_utf16()
            .chain([0])
            .collect::<Vec<u16>>()
            .into_boxed_slice(),
    )
}

unsafe fn mark(hwnd: HWND, s: &str) {
    static LAST: LazyLock<Mutex<Instant>> = LazyLock::new(|| Mutex::new(Instant::now()));
    let (d, c) = {
        let mut l = LAST.lock().unwrap();
        let c = T0.elapsed().as_secs_f64() * 1000.0;
        let d = l.elapsed().as_secs_f64() * 1000.0;
        *l = Instant::now();
        (d, c)
    };
    let t = format!("+{d:.0} (共 {c:.0}) ms · {s}");
    let _ = SetWindowTextW(hwnd, &HSTRING::from(t));
}

unsafe extern "system" fn wp(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_DESTROY {
        let _ = PostQuitMessage(0);
        LRESULT(0)
    } else {
        DefWindowProcW(hwnd, msg, w, l)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let some = args.first().cloned().unwrap_or_default();
    let dir = some
        .rsplit_once('\\')
        .map(|(a, _)| a.to_string())
        .unwrap_or(".".into());

    unsafe {
        use windows::Win32::UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let files: Vec<String> = scan::list(&dir, false)
        .iter()
        .filter(|e| !e.is_dir && scan::is_image(&e.path))
        .map(|e| e.path.clone())
        .collect();
    job::init_com();

    unsafe {
        let hinst = GetModuleHandleW(None).unwrap();
        let hinst = HINSTANCE(hinst.0);
        let cls = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wp),
            hInstance: hinst,
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszClassName: PCWSTR(pw("XseeViewWnd").as_ptr()),
            ..Default::default()
        };
        if RegisterClassExW(&cls) == 0 {
            return;
        }
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(pw("XseeViewWnd").as_ptr()),
            PCWSTR(pw("看图").as_ptr()),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1200,
            800,
            None,
            None,
            Some(hinst),
            None,
        )
        .unwrap();
        mark(hwnd, &format!("A 建窗口 (共 {} 张)", files.len()));
        ShowWindow(hwnd, SW_SHOWMAXIMIZED);
        mark(hwnd, "B 第一次 SW_SHOWMAXIMIZED");
        ShowWindow(hwnd, SW_HIDE);
        mark(hwnd, "C SW_HIDE");
        ShowWindow(hwnd, SW_SHOW);
        mark(hwnd, "D SW_SHOW");
        ShowWindow(hwnd, SW_SHOWMAXIMIZED);
        mark(hwnd, "E 第二次 SW_SHOWMAXIMIZED");
        let mut rc = RECT::default();
        let _ = GetClientRect(hwnd, &mut rc);
        let cw = (rc.right - rc.left).max(1) as u32;
        let ch = (rc.bottom - rc.top).max(1) as u32;
        mark(hwnd, &format!("F 客户区 {cw}x{ch}"));
        let size = job::open_size(&some);
        mark(hwnd, &format!("G 读尺寸 {:?}", size.map(|(a, b)| format!("{a}x{b}"))));
        let q = job::embedded_view(&some, cw, ch);
        mark(hwnd, &format!("H 档内预览 {}", q.is_some()));
        let v = job::decode_view(&some, cw, ch);
        mark(hwnd, &format!("I 全尺寸解码 {}", v.is_ok()));
        let mut m = MSG::default();
        while GetMessageW(&mut m, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }
}

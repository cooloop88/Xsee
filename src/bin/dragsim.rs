//! 模拟从外部程序拖动文件：按下左键 -> DoDragDrop -> 移到目标 -> 松手。
//! 用法: dragsim.exe 源x 源y 目标x 目标y 文件...
#![allow(unsafe_op_in_unsafe_fn)]

use std::thread;
use std::time::Duration;

use windows::Win32::System::Ole::OleInitialize;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, mouse_event,
};
use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 5 {
        eprintln!("用法: dragsim 源x 源y 目标x 目标y 文件...");
        return;
    }
    let (sx, sy, tx, ty) = (
        a[0].parse::<i32>().unwrap(),
        a[1].parse::<i32>().unwrap(),
        a[2].parse::<i32>().unwrap(),
        a[3].parse::<i32>().unwrap(),
    );
    let paths: Vec<String> = a[4..].to_vec();
    unsafe {
        let _ = OleInitialize(None);
        let _ = SetCursorPos(sx, sy);
        thread::sleep(Duration::from_millis(150));
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(250));
            for i in 1..=12 {
                let x = sx + (tx - sx) * i / 12;
                let y = sy + (ty - sy) * i / 12;
                let _ = SetCursorPos(x, y);
                thread::sleep(Duration::from_millis(50));
            }
            thread::sleep(Duration::from_millis(500));
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        });
        xsee_host::drag::start(paths);
        thread::sleep(Duration::from_millis(600));
    }
}

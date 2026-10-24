//! 任务栏、Alt+Tab 和标题栏角上那个程序图标。
//!
//! 图稿在 `work\ui\png`（xsee_icon_1024 母版 + 16–256 七档），由 `work\ui\make_ico.py`
//! 打包成 `host\resources\xsee.ico`，`build.rs` 调 rc.exe 编进 exe 资源段；运行时按资源名
//! `IDI_XSEE` 取回来，挂到窗口类上给任务栏，标题栏那份是画出来的。
//! 界面里不再重画一遍这个图形。
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    IMAGE_ICON, LR_DEFAULTCOLOR, SM_CXICON, SM_CXSMICON, HICON, GetSystemMetrics, LoadImageW,
};
use windows::core::PCWSTR;

/// 取一对（大、小）图标。取不到就还回空句柄，注册类时留空，系统继续用默认图标。
pub fn load() -> (HICON, HICON) {
    unsafe {
        (
            load_sized(GetSystemMetrics(SM_CXICON)),
            load_sized(GetSystemMetrics(SM_CXSMICON)),
        )
    }
}

/// 按指定像素边长取一份。系统会在 .ico 里挑最接近的那一档再缩放到这个尺寸。
/// 调用方用 DestroyIcon 还。
pub fn load_sized(px: i32) -> HICON {
    let wide: Vec<u16> = "IDI_XSEE\0".encode_utf16().collect();
    let name = PCWSTR(wide.as_ptr());
    unsafe {
        // hModule 不能给 NULL：给了就是查系统那份资源，自己那份直接 1813
        // （ERROR_RESOURCE_NAME_NOT_FOUND）失败，图标静默没了。
        let hmod = GetModuleHandleW(None).unwrap_or_default();
        let hinst = HINSTANCE(hmod.0);
        LoadImageW(Some(hinst), name, IMAGE_ICON, px.max(1), px.max(1), LR_DEFAULTCOLOR)
            .map(|hd| HICON(hd.0))
            .unwrap_or_default()
    }
}

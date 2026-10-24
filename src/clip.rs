//! 文件剪贴板（CF_HDROP + Preferred DropEffect）、纯文本、回收站删除、位图剪贴板。
#![allow(unsafe_op_in_unsafe_fn)]

use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::{CF_DIB, CF_HDROP, CF_UNICODETEXT, DROPEFFECT_COPY, DROPEFFECT_MOVE};
use windows::Win32::UI::Shell::{
    CFSTR_PREFERREDDROPEFFECT, DROPFILES, DragQueryFileW, FO_COPY, FO_DELETE, FO_MOVE,
    FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOCONFIRMMKDIR, FOF_NOERRORUI, FOF_SILENT, HDROP,
    SHFileOperationW, SHFILEOPSTRUCTW,
};

fn wide_list(paths: &[String]) -> Vec<u16> {
    let mut w = Vec::new();
    for p in paths {
        w.extend(p.encode_utf16());
        w.push(0);
    }
    w.push(0);
    w
}

unsafe fn global_u32(v: u32) -> Option<HANDLE> {
    let h = GlobalAlloc(GMEM_MOVEABLE, 4).ok()?;
    let p = GlobalLock(h);
    if p.is_null() {
        return None;
    }
    (p as *mut u32).write(v);
    let _ = GlobalUnlock(h);
    Some(HANDLE(h.0))
}

/// 把文件放进剪贴板；cut 决定资源管理器里粘贴时是移动还是复制。
pub fn copy_paths(paths: &[String], cut: bool) -> Result<(), String> {
    if paths.is_empty() {
        return Err("没有选中文件".into());
    }
    unsafe {
        OpenClipboard(None).map_err(|e| e.to_string())?;
        let r = (|| -> Result<(), String> {
            EmptyClipboard().map_err(|e| e.to_string())?;
            let wide = wide_list(paths);
            let head = std::mem::size_of::<DROPFILES>();
            let bytes = head + wide.len() * 2;
            let h = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|e| e.to_string())?;
            let p = GlobalLock(h);
            if p.is_null() {
                return Err("GlobalLock 失败".into());
            }
            let df = p as *mut DROPFILES;
            (*df).pFiles = head as u32;
            (*df).fWide = windows::core::BOOL(1);
            std::ptr::copy_nonoverlapping(wide.as_ptr(), (p as *mut u8).add(head) as *mut u16, wide.len());
            let _ = GlobalUnlock(h);
            SetClipboardData(CF_HDROP.0 as u32, Some(HANDLE(h.0))).map_err(|e| e.to_string())?;
            let fmt = RegisterClipboardFormatW(CFSTR_PREFERREDDROPEFFECT);
            if fmt != 0 {
                let eff = if cut { DROPEFFECT_MOVE.0 } else { DROPEFFECT_COPY.0 };
                if let Some(mem) = global_u32(eff) {
                    let _ = SetClipboardData(fmt, Some(mem));
                }
            }
            Ok(())
        })();
        let _ = CloseClipboard();
        r
    }
}

/// CF_UNICODETEXT：地址栏里选中的那串路径走这条路进剪贴板。
pub fn copy_text(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err("没有选中文字".into());
    }
    unsafe {
        OpenClipboard(None).map_err(|e| e.to_string())?;
        let r = (|| -> Result<(), String> {
            EmptyClipboard().map_err(|e| e.to_string())?;
            let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0u16)).collect();
            let h = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2).map_err(|e| e.to_string())?;
            let p = GlobalLock(h);
            if p.is_null() {
                return Err("GlobalLock 失败".into());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), p as *mut u16, wide.len());
            let _ = GlobalUnlock(h);
            SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(h.0))).map_err(|e| e.to_string())?;
            Ok(())
        })();
        let _ = CloseClipboard();
        r
    }
}

/// 读剪贴板里的文件列表和第二字段（是否剪切）。
pub fn clipboard_paths() -> (Vec<String>, bool) {
    unsafe {
        if OpenClipboard(None).is_err() {
            return (Vec::new(), false);
        }
        let mut out = Vec::new();
        let mut cut = false;
        if let Ok(h) = GetClipboardData(CF_HDROP.0 as u32) {
            let hd = HDROP(h.0);
            let n = DragQueryFileW(hd, u32::MAX, None);
            for i in 0..n {
                let len = DragQueryFileW(hd, i, None) as usize;
                if len == 0 {
                    continue;
                }
                let mut buf = vec![0u16; len + 1];
                if DragQueryFileW(hd, i, Some(&mut buf)) > 0 {
                    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                    out.push(String::from_utf16_lossy(&buf[..end]));
                }
            }
            let fmt = RegisterClipboardFormatW(CFSTR_PREFERREDDROPEFFECT);
            if fmt != 0 {
                if let Ok(m) = GetClipboardData(fmt) {
                    let p = GlobalLock(windows::Win32::Foundation::HGLOBAL(m.0));
                    if !p.is_null() {
                        cut = (p as *const u32).read_unaligned() == DROPEFFECT_MOVE.0;
                        let _ = GlobalUnlock(windows::Win32::Foundation::HGLOBAL(m.0));
                    }
                }
            }
        }
        let _ = CloseClipboard();
        (out, cut)
    }
}

/// 粘贴到目录，不覆盖同名文件。返回 (旧路径 -> 新路径 的成功列表, 跳过数)。
/// 调用方拿这份映射去同步分类库和缩略图缓存。
pub fn paste_into(paths: &[String], dir: &str, cut: bool) -> (Vec<(String, String)>, usize) {
    let mut from: Vec<String> = Vec::new();
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut skip = 0;
    for p in paths {
        let Some(name) = Path::new(p).file_name() else {
            skip += 1;
            continue;
        };
        let dst = Path::new(dir).join(name);
        if dst.exists() {
            skip += 1;
            continue;
        }
        from.push(p.clone());
        pairs.push((p.clone(), dst.display().to_string()));
    }
    if from.is_empty() {
        return (Vec::new(), skip);
    }
    // 交给 shell 搬/拷：文件夹、跨盘都由它按资源管理器那一套处理。以前拿
    // fs::copy 自己拷，碰到文件夹必失败，还一声不响地记成“跳过”。
    let src = wide_list(&from);
    let dst = wide_list(&[dir.to_string()]);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: if cut { FO_MOVE } else { FO_COPY },
        pFrom: PCWSTR(src.as_ptr()),
        pTo: PCWSTR(dst.as_ptr()),
        fFlags: (FOF_NOCONFIRMATION.0 | FOF_NOERRORUI.0 | FOF_SILENT.0 | FOF_NOCONFIRMMKDIR.0) as u16,
        ..Default::default()
    };
    let r = unsafe { SHFileOperationW(&mut op) };
    if r == 0 {
        (pairs, skip)
    } else {
        (Vec::new(), skip + from.len())
    }
}

/// 删除到回收站，确认框由调用方先问过。
pub fn delete_to_recycle(paths: &[String]) -> Result<usize, String> {
    if paths.is_empty() {
        return Ok(0);
    }
    let from = wide_list(paths);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO.0 | FOF_NOCONFIRMATION.0 | FOF_NOERRORUI.0 | FOF_SILENT.0) as u16,
        ..Default::default()
    };
    let r = unsafe { SHFileOperationW(&mut op) };
    if r == 0 { Ok(paths.len()) } else { Err(format!("删除失败，代码 {r}")) }
}

/// 永久删除：不留回收站那份，删了找不回。确认框由调用方先问过，
/// 这里照样带 NOCONFIRMATION，免得系统再叠一层自己的框。
pub fn delete_permanent(paths: &[String]) -> Result<usize, String> {
    if paths.is_empty() {
        return Ok(0);
    }
    let from = wide_list(paths);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_NOCONFIRMATION.0 | FOF_NOERRORUI.0 | FOF_SILENT.0) as u16,
        ..Default::default()
    };
    let r = unsafe { SHFileOperationW(&mut op) };
    if r == 0 { Ok(paths.len()) } else { Err(format!("删除失败，代码 {r}")) }
}

/// 把图片塞进剪贴板当 CF_DIB，Word/Photoshop 可以直接粘贴。
pub fn copy_image(path: &str) -> Result<(), String> {
    let img = crate::encode::decode(path, true)?;
    let (w, h) = (img.width(), img.height());
    let stride = (w * 4) as usize;
    let head = std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAPINFOHEADER>();
    let bytes = head + stride * h as usize;
    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|e| e.to_string())?;
        let p = GlobalLock(mem);
        if p.is_null() {
            return Err("GlobalLock 失败".into());
        }
        let bi = p as *mut windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
        (*bi).biSize = head as u32;
        (*bi).biWidth = w as i32;
        (*bi).biHeight = h as i32;
        (*bi).biPlanes = 1;
        (*bi).biBitCount = 32;
        (*bi).biCompression = windows::Win32::Graphics::Gdi::BI_RGB.0;
        (*bi).biSizeImage = (stride * h as usize) as u32;
        let dst = (p as *mut u8).add(head);
        for y in 0..h as usize {
            let sy = h as usize - 1 - y;
            let srow = &img.as_raw()[sy * stride..sy * stride + stride];
            let drow = dst.add(y * stride);
            for x in 0..w as usize {
                let s = &srow[x * 4..x * 4 + 4];
                *drow.add(x * 4) = s[2];
                *drow.add(x * 4 + 1) = s[1];
                *drow.add(x * 4 + 2) = s[0];
                *drow.add(x * 4 + 3) = 0xFF;
            }
        }
        let _ = GlobalUnlock(mem);
        OpenClipboard(None).map_err(|e| e.to_string())?;
        EmptyClipboard().map_err(|e| e.to_string())?;
        SetClipboardData(CF_DIB.0 as u32, Some(HANDLE(mem.0))).map_err(|e| e.to_string())?;
        let _ = CloseClipboard();
    }
    Ok(())
}

/// 内存里那张贴回剪贴板当 CF_DIB。画布上从剪贴板贴进来的那张没有文件可解，
/// 走 copy_image 等于跟它要一个不存在的.path。
pub fn copy_view(v: &crate::View) -> Result<(), String> {
    let (w, h) = (v.w as usize, v.h as usize);
    let stride = w * 4;
    let head = std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAPINFOHEADER>();
    if w == 0 || h == 0 || v.px.len() < stride * h {
        return Err("那张图手上还没有像素".into());
    }
    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, head + stride * h).map_err(|e| e.to_string())?;
        let p = GlobalLock(mem);
        if p.is_null() {
            return Err("GlobalLock 失败".into());
        }
        let bi = p as *mut windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
        (*bi).biSize = head as u32;
        (*bi).biWidth = w as i32;
        (*bi).biHeight = h as i32;
        (*bi).biPlanes = 1;
        (*bi).biBitCount = 32;
        (*bi).biCompression = windows::Win32::Graphics::Gdi::BI_RGB.0;
        (*bi).biSizeImage = (stride * h) as u32;
        // 画布那边是顶朝下，DIB 这一档要底朝上，逐行倒着搬一遍
        let dst = (p as *mut u8).add(head);
        for y in 0..h {
            std::ptr::copy_nonoverlapping(
                v.px.as_ptr().add((h - 1 - y) * stride),
                dst.add(y * stride),
                stride,
            );
        }
        let _ = GlobalUnlock(mem);
        OpenClipboard(None).map_err(|e| e.to_string())?;
        EmptyClipboard().map_err(|e| e.to_string())?;
        SetClipboardData(CF_DIB.0 as u32, Some(HANDLE(mem.0))).map_err(|e| e.to_string())?;
        let _ = CloseClipboard();
    }
    Ok(())
}

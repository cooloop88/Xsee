//! 文件剪贴板（CF_HDROP + Preferred DropEffect）、纯文本、回收站删除、位图剪贴板。
#![allow(unsafe_op_in_unsafe_fn)]

use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HGLOBAL, HANDLE, GlobalFree};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::{CF_DIB, CF_HDROP, CF_UNICODETEXT, DROPEFFECT_COPY, DROPEFFECT_MOVE};
use windows::Win32::UI::Shell::{
    CFSTR_PREFERREDDROPEFFECT, DROPFILES, DragQueryFileW, FO_COPY, FO_DELETE, FO_MOVE,
    FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOCONFIRMMKDIR, FOF_NOERRORUI, FOF_SILENT, HDROP,
    SHFileOperationW, SHFILEOPSTRUCTW,
};

fn wide_list(paths: &[String]) -> Vec<u16> {
    let mut w = Vec::new();
    for p in paths {
        // SHFileOperationW 只认反斜杠：pFrom 带正斜杠时它连源文件都找不到，
        // 回一个 183（"已存在"），看着就是粘贴/删除没反应。
        // 地址栏手敲的路径、剪贴板里的旧记录、命令行参数都可能带正斜杠。
        let p = p.replace('/', "\\");
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
        let _ = GlobalFree(Some(h));
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
            let inner = (|| -> Result<(), String> {
                EmptyClipboard().map_err(|e| e.to_string())?;
                let wide = wide_list(paths);
                let head = std::mem::size_of::<DROPFILES>();
                let bytes = head + wide.len() * 2;
                // ZEROINIT：DROPFILES 的 pt/fNC 也算在结构里，别把堆上的脏字节发给系统。
                let h = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes).map_err(|e| e.to_string())?;
                let res = (|| -> Result<(), String> {
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
                            if SetClipboardData(fmt, Some(mem)).is_err() {
                                // 投放失败系统不接管这块，自己还回去。
                                let _ = GlobalFree(Some(HGLOBAL(mem.0)));
                            }
                        }
                    }
                    Ok(())
                })();
                if res.is_err() {
                    // 投放失败系统不接管这块，得自己还回去；成功则归剪贴板。
                    let _ = GlobalFree(Some(h));
                }
                res
            })();
            let _ = CloseClipboard();
            inner
        })();
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
            let inner = (|| -> Result<(), String> {
                EmptyClipboard().map_err(|e| e.to_string())?;
                let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0u16)).collect();
                let h = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2).map_err(|e| e.to_string())?;
                let res = (|| -> Result<(), String> {
                    let p = GlobalLock(h);
                    if p.is_null() {
                        return Err("GlobalLock 失败".into());
                    }
                    std::ptr::copy_nonoverlapping(wide.as_ptr(), p as *mut u16, wide.len());
                    let _ = GlobalUnlock(h);
                    SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(h.0))).map_err(|e| e.to_string())?;
                    Ok(())
                })();
                if res.is_err() {
                    // 投放失败系统不接管这块，得自己还回去；成功则归剪贴板。
                    let _ = GlobalFree(Some(h));
                }
                res
            })();
            let _ = CloseClipboard();
            inner
        })();
        r
    }
}

/// 读剪贴板里的文件列表和第二字段（是否剪切）。
pub fn clipboard_paths() -> (Vec<String>, bool) {
    unsafe {
        // 刚从资源管理器复制完那一下，它自己还占着剪贴板，一次 OpenClipboard 常回不来，
        // 读不到就成了"剪贴板里没有文件"。小步重试几回再放弃。
        let mut opened = false;
        for _ in 0..8 {
            if OpenClipboard(None).is_ok() {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if !opened {
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
/// 目标目录里已经有同名东西时，照资源管理器那样改出"名字 - 副本.扩展名"。
pub fn copy_name(dir: &Path, name: &str) -> String {
    if !dir.join(name).exists() {
        return name.to_string();
    }
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string());
    let ext = Path::new(name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for i in 1u32.. {
        let cand = if i == 1 {
            format!("{stem} - 副本{ext}")
        } else {
            format!("{stem} - 副本 ({i}){ext}")
        };
        if !dir.join(&cand).exists() {
            return cand;
        }
    }
    unreachable!()
}

/// `SHFileOperationW` 的返回码翻成一句人话。分开"这里不让写"和"同名已存在"这两类，
/// 不然用户看到的都是同一句"没反应"。
pub fn shell_err(r: i32) -> String {
    match r {
        0 => String::new(),
        2 => "找不到源文件".to_string(),
        5 | 70 | 120 => "这个目录不允许写入（要管理员权限），换个普通文件夹".to_string(),
        124 => "目标路径不对".to_string(),
        112 => "目标盘满了".to_string(),
        1223 => "取消了".to_string(),
        _ => format!("系统返回错误码 {r}"),
    }
}

/// 目标就是源待着的那个文件夹。资源管理器只有在这一种情况下直接落"xxx - 副本"，
/// 别的撞名都是弹框问替换还是跳过——我们照这个分界走。
/// 两边的写法不保证一样：地址栏手敲、剪贴板里的旧记录、树行都可能是正斜杠或带尾分隔符的。
/// 不先抹平就直接比字符串，同目录会被判成"不同目录"，落出来的目标就是源文件自己，
/// shell 回 183，看着就是"粘贴没反应"。
pub fn same_folder(src: &str, dir: &str) -> bool {
    fn norm(p: &str) -> String {
        let s = p.replace('/', "\\");
        let s = if s.len() > 3 { s.trim_end_matches('\\') } else { s.as_str() };
        s.to_ascii_lowercase()
    }
    // 文件夹带尾分隔符时 Path::parent 会认错上一级，先去掉尾分隔符再取父目录。
    let bare = src.trim_end_matches(['\\', '/']);
    let bare = if bare.len() == 2 && bare.ends_with(':') { src } else { bare };
    let Some(par) = Path::new(bare).parent() else {
        return false;
    };
    let a = norm(&par.to_string_lossy());
    !a.is_empty() && a == norm(dir)
}

/// 一次一个文件地交给 shell：`pTo` 写成完整目标路径，落出来的名字才是我们算好的那个
/// （"名字 - 副本.ext"）。整批一次拷时 shell 只认目录、按原名落，撞名就悄悄不动，
/// 同目录粘贴看着就像"没反应"。
/// 标志只压掉"建目录确认"，进度框、出错框、要管理员权限那一律留给系统弹——
/// 资源管理器怎么做，这里就怎么做。
pub fn shell_move(src: &str, dst: &str, cut: bool) -> Result<(), String> {
    let from = wide_list(&[src.to_string()]);
    let to = wide_list(&[dst.to_string()]);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: if cut { FO_MOVE } else { FO_COPY },
        pFrom: PCWSTR(from.as_ptr()),
        pTo: PCWSTR(to.as_ptr()),
        fFlags: (FOF_NOCONFIRMMKDIR.0 | FOF_ALLOWUNDO.0) as u16,
        ..Default::default()
    };
    let r = unsafe { SHFileOperationW(&mut op) };
    if r != 0 {
        return Err(shell_err(r));
    }
    // 用户在"替换/跳过"那类系统弹框里点取消时，返回值照样是 0，
    // 真相在这个标志位上——不查它，没发生的搬运会被当成成功记账。
    if op.fAnyOperationsAborted.as_bool() {
        return Err("取消了".to_string());
    }
    Ok(())
}

/// 把剪贴板里那批文件落进 `dir`。返回（成功的 源→目标 对，跳过的个数，失败原因）。
pub fn paste_into(paths: &[String], dir: &str, cut: bool) -> (Vec<(String, String)>, usize, String) {
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut skip = 0;
    let out = Path::new(dir);
    for p in paths {
        let Some(name) = Path::new(p).file_name().map(|n| n.to_string_lossy().to_string()) else {
            skip += 1;
            continue;
        };
        // 同目录粘贴 = 资源管理器那句"粘贴"：直接落一个副本出来。
        // 别的目录撞名不改名，交给系统弹框问（替换/跳过）。
        let dst_name = if !cut && same_folder(p, dir) {
            copy_name(out, &name)
        } else {
            name.clone()
        };
        let dst = out.join(dst_name).display().to_string();
        pairs.push((p.clone(), dst));
    }
    let mut done: Vec<(String, String)> = Vec::new();
    let mut err = String::new();
    for (src, dst) in pairs {
        match shell_move(&src, &dst, cut) {
            Ok(()) => done.push((src, dst)),
            Err(e) => {
                skip += 1;
                if err.is_empty() {
                    err = e;
                }
            }
        }
    }
    (done, skip, err)
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
    if r != 0 {
        return Err(format!("删除失败，代码 {r}"));
    }
    // 中途被打断（文件被占用跳过一类）时按失败报：宁可在库里留下几条死路径等
    // "检查数据库"清，也不能把还活着的文件从分类里摘掉。
    if op.fAnyOperationsAborted.as_bool() {
        return Err("删除没有全部完成，文件可能有被占用的".to_string());
    }
    Ok(paths.len())
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
    if r != 0 {
        return Err(format!("删除失败，代码 {r}"));
    }
    if op.fAnyOperationsAborted.as_bool() {
        return Err("删除没有全部完成，文件可能有被占用的".to_string());
    }
    Ok(paths.len())
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
            let _ = GlobalFree(Some(mem));
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
        put_clipboard(mem)
    }
}

/// 把一块已经填好的 HGLOBAL 投进剪贴板。打开/清空/投放三步任何一步失败都不能把
/// 剪贴板留成打开状态（会锁死全系统的剪贴板直到本进程退出）；投放失败的块系统
/// 不接管，要自己 GlobalFree 还回去，成功则所有权归剪贴板。
unsafe fn put_clipboard(mem: HGLOBAL) -> Result<(), String> {
    OpenClipboard(None).map_err(|e| e.to_string())?;
    let inner = (|| -> Result<(), String> {
        EmptyClipboard().map_err(|e| e.to_string())?;
        SetClipboardData(CF_DIB.0 as u32, Some(HANDLE(mem.0))).map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = CloseClipboard();
    if inner.is_err() {
        let _ = GlobalFree(Some(mem));
    }
    inner
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
            let _ = GlobalFree(Some(mem));
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
        put_clipboard(mem)
    }
}

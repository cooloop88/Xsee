//! 文件剪贴板（CF_HDROP + Preferred DropEffect）、纯文本、回收站删除、位图剪贴板。
#![allow(unsafe_op_in_unsafe_fn)]

use std::path::Path;

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HGLOBAL, HANDLE, GlobalFree};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::{CF_DIB, CF_HDROP, CF_UNICODETEXT, DROPEFFECT_COPY, DROPEFFECT_MOVE};
use windows::Win32::UI::Shell::{
    CFSTR_PREFERREDDROPEFFECT, DROPFILES, DragQueryFileW, FILEOPERATION_FLAGS, FO_DELETE,
    FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOCONFIRMMKDIR, FOF_NOERRORUI, FOF_SILENT, FileOperation,
    HDROP, IFileOperation, IFileOperationProgressSink, IShellItem, SHCreateItemFromParsingName,
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

/// `IFileOperation` 抛出来的是 HRESULT，不是 `SHFileOperationW` 那套返回码，
/// 常见几种翻成人话，别让用户对着 0x8007… 猜。
fn op_err(e: &windows::core::Error) -> String {
    match e.code().0 as u32 {
        0x8007_0002 => "找不到源文件".to_string(),
        0x8007_0005 => "这个位置不允许写入（要管理员权限），换个普通文件夹".to_string(),
        0x8007_0020 => "文件正被占用".to_string(),
        0x8007_0070 => "目标盘满了".to_string(),
        0x8007_04c7 | 0x8027_0000 => "取消了".to_string(),
        _ => format!("{e}"),
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

/// 一批文件一次交给系统搬：资源管理器自己的"粘贴"走的就是这套 `IFileOperation`，
/// 建一次操作实例、把每项排进队列、最后 `PerformOperations` 一次性执行。
/// 以前是每个文件单独调一次 `SHFileOperationW`，那样每项都要重新拉起一套 shell 拷贝引擎，
/// 几十个小文件光这笔固定开销就好几秒（实测每个 0.13~0.6 秒，比拷字节本身贵得多），
/// 所以同目录粘贴一堆小图比资源管理器慢一大截。
/// `items` 是 (源, 目标完整路径) 对；返回实际落地的那些、没落地的个数、首条错误。
pub fn transfer_many(items: &[(String, String)], cut: bool) -> (Vec<(String, String)>, usize, String) {
    if items.is_empty() {
        return (Vec::new(), 0, String::new());
    }
    // 操作前记下源的"大小 + 修改时间"。系统拷完是原样抄这两个值，事后靠它分清
    // "目标上落了新拷贝"和"撞名框里点了跳过"——跳过时目标还是老内容，不能当成搬成功。
    let stamp: Vec<Option<(u64, std::time::SystemTime)>> = items
        .iter()
        .map(|(s, _)| {
            std::fs::metadata(s)
                .ok()
                .and_then(|m| m.modified().ok().map(|t| (m.len(), t)))
        })
        .collect();
    let mut err = String::new();
    let mut aborted = false;
    unsafe {
        // 界面线程早就 OleInitialize 成了 STA，这里只是配个对，让后台拖放线程也能用。
        let com = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let op: Option<IFileOperation> = match CoCreateInstance(&FileOperation, None, CLSCTX_ALL) {
            Ok(op) => Some(op),
            Err(e) => {
                err = format!("系统文件操作初始化失败（{}）", op_err(&e));
                None
            }
        };
        if let Some(op) = op {
            if let Err(e) =
                op.SetOperationFlags(FILEOPERATION_FLAGS(FOF_ALLOWUNDO.0 | FOF_NOCONFIRMMKDIR.0))
            {
                err = format!("设置文件操作参数失败（{}）", op_err(&e));
            } else {
                let mut cur_dir = String::new();
                let mut dir_item: Option<IShellItem> = None;
                for (src, dst) in items {
                    let dst_path = Path::new(dst);
                    let Some(name) = dst_path.file_name().map(|n| n.to_string_lossy().to_string())
                    else {
                        if err.is_empty() {
                            err = "目标名不合法".to_string();
                        }
                        continue;
                    };
                    let Some(parent) = dst_path.parent() else {
                        if err.is_empty() {
                            err = "目标路径不合法".to_string();
                        }
                        continue;
                    };
                    let parent = parent.display().to_string();
                    if dir_item.is_none() || parent != cur_dir {
                        let w = HSTRING::from(parent.replace('/', "\\"));
                        let it: windows::core::Result<IShellItem> =
                            SHCreateItemFromParsingName(PCWSTR(w.as_ptr()), None);
                        match it {
                            Ok(it) => {
                                dir_item = Some(it);
                                cur_dir = parent;
                            }
                            Err(e) => {
                                if err.is_empty() {
                                    err = format!("找不到目标文件夹（{}）", op_err(&e));
                                }
                                dir_item = None;
                                continue;
                            }
                        }
                    }
                    let folder = dir_item.as_ref().expect("上面刚建好");
                    let sw = HSTRING::from(src.replace('/', "\\"));
                    let item: windows::core::Result<IShellItem> =
                        SHCreateItemFromParsingName(PCWSTR(sw.as_ptr()), None);
                    let item = match item {
                        Ok(it) => it,
                        Err(e) => {
                            if err.is_empty() {
                                err = format!("找不到源文件（{}）", op_err(&e));
                            }
                            continue;
                        }
                    };
                    let nw = HSTRING::from(name);
                    let queued = if cut {
                        op.MoveItem(
                            &item,
                            folder,
                            PCWSTR(nw.as_ptr()),
                            None::<&IFileOperationProgressSink>,
                        )
                    } else {
                        op.CopyItem(
                            &item,
                            folder,
                            PCWSTR(nw.as_ptr()),
                            None::<&IFileOperationProgressSink>,
                        )
                    };
                    if let Err(e) = queued {
                        if err.is_empty() {
                            err = format!("文件操作排队失败（{}）", op_err(&e));
                        }
                    }
                }
                if let Err(e) = op.PerformOperations() {
                    if err.is_empty() {
                        err = format!("文件操作失败（{}）", op_err(&e));
                    }
                }
                aborted = op
                    .GetAnyOperationsAborted()
                    .map(|b| b.as_bool())
                    .unwrap_or(false);
            }
        }
        if com.is_ok() {
            CoUninitialize();
        }
    }
    // 排进队列不等于真落了盘：撞名框点跳过、单个文件被占用，整批照样回成功。
    // 按文件系统实际状态逐项认账。
    let mut done = Vec::new();
    let mut skip = 0;
    for (i, (src, dst)) in items.iter().enumerate() {
        let m = std::fs::metadata(dst).ok();
        let landed = match (&m, &stamp[i]) {
            (None, _) => false,
            // 搬走 = 源没了、目标在；源还在说明这项没动
            (Some(_), _) if cut => !Path::new(src).exists(),
            (Some(m), Some((len, t))) => {
                m.len() == *len && m.modified().ok().as_ref() == Some(t)
            }
            (Some(_), None) => true,
        };
        if landed {
            done.push((src.clone(), dst.clone()));
        } else {
            skip += 1;
        }
    }
    if err.is_empty() && aborted {
        err = "取消了".to_string();
    }
    (done, skip, err)
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
    let (done, more, err) = transfer_many(&pairs, cut);
    (done, skip + more, err)
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

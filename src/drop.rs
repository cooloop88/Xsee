#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]
//! 接收拖放：资源管理器或别的程序拖过来的文件在这里变成一串本地路径，
//! 再交给窗口自己判断是落进某个文件夹还是直接打开。

use windows::core::{implement, Ref, Result};
use windows::Win32::Foundation::{HWND, POINT, POINTL, S_OK};
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL};
use windows::Win32::System::Ole::{
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_MOVE, IDropTarget, IDropTarget_Impl,
    RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
};
use windows::Win32::System::SystemServices::{MK_CONTROL, MODIFIERKEYS_FLAGS};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

/// 落盘时带上客户区坐标；copy=true 表示按住 Ctrl 复制，否则落进文件夹就是移动。
pub type DropFn = fn(HWND, Vec<String>, i32, i32, bool);
/// 指针底下是不是一个能放文件的文件夹。
pub type FolderFn = fn(HWND, i32, i32) -> bool;
/// 拖拽经过本窗口时报告落点坐标；None 表示当前没有可放的文件夹。
pub type HoverFn = fn(HWND, Option<(i32, i32)>);

#[implement(IDropTarget)]
struct Target {
    hwnd: isize,
    on_drop: DropFn,
    over_folder: FolderFn,
    hover: HoverFn,
}

fn fmtetc() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

fn read_paths(data: &IDataObject) -> Vec<String> {
    unsafe {
        let f = fmtetc();
        if data.QueryGetData(&f) != S_OK {
            return Vec::new();
        }
        let Ok(mut m) = data.GetData(&f) else {
            return Vec::new();
        };
        let h = HDROP(m.u.hGlobal.0);
        let n = DragQueryFileW(h, u32::MAX, None);
        let mut out = Vec::with_capacity(n as usize);
        for i in 0..n {
            let len = DragQueryFileW(h, i, None) as usize;
            if len == 0 {
                continue;
            }
            let mut buf = vec![0u16; len + 1];
            let got = DragQueryFileW(h, i, Some(&mut buf)) as usize;
            if got > 0 {
                buf.truncate(got);
                out.push(String::from_utf16_lossy(&buf));
            }
        }
        ReleaseStgMedium(&mut m);
        out
    }
}

impl Target {
    fn client(&self, pt: &POINTL) -> (i32, i32) {
        let hwnd = HWND(self.hwnd as *mut core::ffi::c_void);
        let mut p = POINT { x: pt.x, y: pt.y };
        unsafe {
            let _ = ScreenToClient(hwnd, &mut p);
        }
        (p.x, p.y)
    }

    fn folder(&self, pt: &POINTL) -> bool {
        let hwnd = HWND(self.hwnd as *mut core::ffi::c_void);
        let (x, y) = self.client(pt);
        (self.over_folder)(hwnd, x, y)
    }

    /// 判定的同时把落点报给窗口，让它画/擦高亮。
    fn tell(&self, pt: &POINTL) -> bool {
        let hwnd = HWND(self.hwnd as *mut core::ffi::c_void);
        let f = self.folder(pt);
        let xy = self.client(pt);
        (self.hover)(hwnd, if f { Some(xy) } else { None });
        f
    }

    fn clear(&self) {
        let hwnd = HWND(self.hwnd as *mut core::ffi::c_void);
        (self.hover)(hwnd, None);
    }
}

impl IDropTarget_Impl for Target_Impl {
    fn DragEnter(
        &self,
        pdataobj: Ref<IDataObject>,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> Result<()> {
        let ok = pdataobj
            .as_ref()
            .map(|d| unsafe { d.QueryGetData(&fmtetc()) == S_OK })
            .unwrap_or(false);
        unsafe {
            *pdweffect = if ok {
                if self.tell(pt) {
                    DROPEFFECT_MOVE | DROPEFFECT_COPY
                } else {
                    DROPEFFECT_COPY
                }
            } else {
                DROPEFFECT(0)
            };
        }
        Ok(())
    }

    fn DragOver(
        &self,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> Result<()> {
        unsafe {
            *pdweffect = if self.tell(pt) {
                DROPEFFECT_MOVE | DROPEFFECT_COPY
            } else {
                DROPEFFECT_COPY
            };
        }
        Ok(())
    }

    fn DragLeave(&self) -> Result<()> {
        self.clear();
        Ok(())
    }

    fn Drop(
        &self,
        pdataobj: Ref<IDataObject>,
        grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> Result<()> {
        let Some(data) = pdataobj.as_ref() else {
            return Ok(());
        };
        let paths = read_paths(data);
        let hwnd = HWND(self.hwnd as *mut core::ffi::c_void);
        let (x, y) = self.client(pt);
        let folder = self.tell(pt);
        self.clear();
        // 落进文件夹就是搬走：默认移动，按住 Ctrl 才复制。空白处只是打开，不动文件
        let copy = !folder || (grfkeystate.0 & MK_CONTROL.0) != 0;
        if !paths.is_empty() {
            (self.on_drop)(hwnd, paths, x, y, copy);
        }
        unsafe {
            *pdweffect = if folder && !copy { DROPEFFECT_MOVE } else { DROPEFFECT_COPY };
        }
        Ok(())
    }
}

/// 注册成某个窗口的拖放接收方。调用线程必须先 OleInitialize（STA）。
pub unsafe fn register(hwnd: HWND, on_drop: DropFn, over_folder: FolderFn, hover: HoverFn) -> bool {
    let t: IDropTarget = Target {
        hwnd: hwnd.0 as isize,
        on_drop,
        over_folder,
        hover,
    }
    .into();
    RegisterDragDrop(hwnd, &t).is_ok()
}

pub unsafe fn unregister(hwnd: HWND) {
    let _ = RevokeDragDrop(hwnd);
}

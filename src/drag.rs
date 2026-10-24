#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]
//! 把选中的文件拖给资源管理器或别的程序。数据格式就是标准的 CF_HDROP，
//! 拖出的是真实路径，目标程序按普通文件处理，不需要知道我们是谁。

use windows::core::{implement, PCWSTR, Interface, BOOL, HRESULT, Result};
use windows::Win32::Foundation::{
    DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, DV_E_FORMATETC,
    E_NOTIMPL, OLE_E_ADVISENOTSUPPORTED, POINT, S_OK,
};
use windows::Win32::System::Com::{
    DATADIR_GET, DVASPECT_CONTENT, FORMATETC, IAdviseSink, IDataObject, IDataObject_Impl,
    IEnumFORMATETC, IEnumSTATDATA, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::{
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, DoDragDrop,
    IDropSource, IDropSource_Impl,
};
use windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use windows::Win32::UI::Shell::{
    CFSTR_PERFORMEDDROPEFFECT, CFSTR_PREFERREDDROPEFFECT, DROPFILES, SHCreateStdEnumFmtEtc,
};

/// 目标程序拿这个决定"不按 Ctrl 时算复制还是剪切"。往别的文件夹或桌面拖
/// 默认是复制，原文件留在原处；按住 Shift 由目标程序自己改成移动。
const PREF: PCWSTR = CFSTR_PREFERREDDROPEFFECT;
const PERF: PCWSTR = CFSTR_PERFORMEDDROPEFFECT;

fn fmt_id(name: PCWSTR) -> u16 {
    unsafe { RegisterClipboardFormatW(name) as u16 }
}

#[implement(IDataObject, IDropSource)]
struct FileDrop {
    paths: Vec<String>,
    performed: std::sync::atomic::AtomicU32,
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

fn effect_etc(name: PCWSTR) -> FORMATETC {
    FORMATETC {
        cfFormat: fmt_id(name),
        ..fmtetc()
    }
}

fn is_hdrop(f: &FORMATETC) -> bool {
    f.cfFormat == CF_HDROP.0 && (f.tymed & TYMED_HGLOBAL.0 as u32) != 0
}

fn is_name(f: &FORMATETC, name: PCWSTR) -> bool {
    f.cfFormat == fmt_id(name) && (f.tymed & TYMED_HGLOBAL.0 as u32) != 0
}

/// 效果标志就是一个 4 字节 DWORD，装在 HGLOBAL 里递出去。
unsafe fn dword(v: u32) -> Result<STGMEDIUM> {
    let h = GlobalAlloc(GMEM_MOVEABLE, 4)?;
    let p = GlobalLock(h);
    if p.is_null() {
        return Err(E_NOTIMPL.into());
    }
    (p as *mut u32).write(v);
    let _ = GlobalUnlock(h);
    Ok(STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: h },
        pUnkForRelease: Default::default(),
    })
}

impl IDataObject_Impl for FileDrop_Impl {
    fn GetData(&self, pformatetcin: *const FORMATETC) -> Result<STGMEDIUM> {
        unsafe {
            let f = &*pformatetcin;
            if is_name(f, PREF) {
                return dword(DROPEFFECT_COPY.0);
            }
            if is_name(f, PERF) {
                return dword(self.performed.load(std::sync::atomic::Ordering::SeqCst));
            }
            if !is_hdrop(f) {
                return Err(DV_E_FORMATETC.into());
            }
            let mut wide: Vec<u16> = Vec::new();
            for s in &self.paths {
                wide.extend(s.encode_utf16());
                wide.push(0);
            }
            wide.push(0);
            let head = std::mem::size_of::<DROPFILES>();
            let bytes = head + wide.len() * 2;
            let h = GlobalAlloc(GMEM_MOVEABLE, bytes)?;
            let p = GlobalLock(h);
            if p.is_null() {
                return Err(E_NOTIMPL.into());
            }
            let df = p as *mut DROPFILES;
            df.write(DROPFILES {
                pFiles: head as u32,
                pt: POINT { x: 0, y: 0 },
                fNC: BOOL(0),
                fWide: BOOL(1),
            });
            std::ptr::copy_nonoverlapping(
                wide.as_ptr(),
                (p as *mut u8).add(head) as *mut u16,
                wide.len(),
            );
            let _ = GlobalUnlock(h);
            Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: h },
                pUnkForRelease: Default::default(),
            })
        }
    }

    fn GetDataHere(&self, _f: *const FORMATETC, _m: *mut STGMEDIUM) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        unsafe {
            let f = &*pformatetc;
            if is_hdrop(f) || is_name(f, PREF) || is_name(f, PERF) {
                S_OK
            } else {
                DV_E_FORMATETC
            }
        }
    }

    fn GetCanonicalFormatEtc(
        &self,
        inp: *const FORMATETC,
        out: *mut FORMATETC,
    ) -> HRESULT {
        unsafe {
            *out = *inp;
            (*out).ptd = std::ptr::null_mut();
        }
        S_OK
    }

    /// 目标程序 drop 完会写回它实际做了什么效果。这里必须收下，否则 OLE 报错。
    fn SetData(&self, f: *const FORMATETC, m: *const STGMEDIUM, _r: BOOL) -> Result<()> {
        unsafe {
            let fm = &*f;
            if is_name(fm, PERF) && (*m).tymed == TYMED_HGLOBAL.0 as u32 {
                let h = (*m).u.hGlobal;
                if !h.0.is_null() {
                    let p = GlobalLock(h);
                    if !p.is_null() {
                        self.performed
                            .store((p as *const u32).read_unaligned(), std::sync::atomic::Ordering::SeqCst);
                        let _ = GlobalUnlock(h);
                    }
                }
            }
            Ok(())
        }
    }

    fn EnumFormatEtc(&self, dwdirection: u32) -> Result<IEnumFORMATETC> {
        if dwdirection != DATADIR_GET.0 as u32 {
            return Err(E_NOTIMPL.into());
        }
        unsafe { SHCreateStdEnumFmtEtc(&[fmtetc(), effect_etc(PREF), effect_etc(PERF)]) }
    }

    fn DAdvise(
        &self,
        _f: *const FORMATETC,
        _a: u32,
        _s: windows::core::Ref<IAdviseSink>,
    ) -> Result<u32> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn DUnadvise(&self, _c: u32) -> Result<()> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn EnumDAdvise(&self) -> Result<IEnumSTATDATA> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
}

impl IDropSource_Impl for FileDrop_Impl {
    fn QueryContinueDrag(
        &self,
        fescapepressed: BOOL,
        grfkeystate: MODIFIERKEYS_FLAGS,
    ) -> HRESULT {
        if fescapepressed.as_bool() {
            DRAGDROP_S_CANCEL
        } else if (grfkeystate.0 & MK_LBUTTON.0) == 0 {
            DRAGDROP_S_DROP
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, _dweffect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

/// 一直阻塞到用户松手；返回 DoDragDrop 的返回码和目标最后执行的效果，
/// 0 开头的 0x000401xx 是 OLE 的正常结局（放下/取消），失败才是错误码。
pub fn start(paths: Vec<String>) -> (i32, u32) {
    if paths.is_empty() {
        return (0, 0);
    }
    unsafe {
        let data: IDataObject = FileDrop {
            paths,
            performed: Default::default(),
        }
        .into();
        let Ok(src) = data.cast::<IDropSource>() else {
            return (-1, 0);
        };
        let mut effect = DROPEFFECT(0);
        let r = DoDragDrop(
            &data,
            &src,
            DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK,
            &mut effect,
        );
        (r.0 as i32, effect.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_copy() {
        let data: IDataObject = FileDrop {
            paths: vec![r"G:\q\x.jpg".to_string()],
            performed: Default::default(),
        }
        .into();
        let f = effect_etc(PREF);
        let m = unsafe {
            assert_eq!(data.QueryGetData(&f), S_OK);
            data.GetData(&f).unwrap()
        };
        unsafe {
            let p = GlobalLock(m.u.hGlobal) as *const u32;
            assert_eq!(*p, DROPEFFECT_COPY.0);
            let _ = GlobalUnlock(m.u.hGlobal);
        }
    }

    #[test]
    fn hdrop_carries_all_paths() {
        let paths = vec![r"C:\a b\x.jpg".to_string(), r"G:\q\y.png".to_string()];
        let data: IDataObject = FileDrop {
            paths: paths.clone(),
            performed: Default::default(),
        }
        .into();
        let f = fmtetc();
        let m = unsafe {
            assert_eq!(data.QueryGetData(&f), S_OK);
            data.GetData(&f).unwrap()
        };
        assert_eq!(m.tymed, TYMED_HGLOBAL.0 as u32);
        unsafe {
            let p = GlobalLock(m.u.hGlobal) as *const u8;
            assert!(!p.is_null());
            let df = p as *const DROPFILES;
            assert!(df.read().fWide.as_bool());
            let mut w = p.add(df.read().pFiles as usize) as *const u16;
            let mut got: Vec<String> = Vec::new();
            loop {
                let mut s = Vec::new();
                while *w != 0 {
                    s.push(*w);
                    w = w.add(1);
                }
                if s.is_empty() {
                    break;
                }
                got.push(String::from_utf16(&s).unwrap());
                w = w.add(1);
            }
            assert_eq!(got, paths);
            let _ = GlobalUnlock(m.u.hGlobal);
        }
    }
}

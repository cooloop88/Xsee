//! 非图片文件的系统图标。
//!
//! 图标按扩展名向 shell 要（`SHGFI_USEFILEATTRIBUTES`，不碰盘）；`.exe/.lnk`
//! 这类每个文件图标都可能不同的按完整路径要。索引只查一次，好结果和空结果都记。
//! 实际出图按（键, 边长, 底色）合成到一张实底 32 位点阵上缓存：格子底色就
//! 那么几种，滚动时只剩一次整块拷贝。

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, CreateSolidBrush,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, FillRect, HALFTONE, HBITMAP, HDC, HGDIOBJ, SelectObject,
    SetBrushOrgEx, SetDIBitsToDevice, SetStretchBltMode,
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::UI::Controls::{IImageList, ILD_TRANSPARENT};
use windows::Win32::UI::Shell::{
    SHFILEINFOW, SHGFI_SYSICONINDEX, SHGFI_USEFILEATTRIBUTES, SHGetFileInfoW, SHGetImageList,
    SHIL_EXTRALARGE, SHIL_LARGE, SHIL_SMALL,
};
use windows::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DestroyIcon, DrawIconEx, HICON};

use crate::theme::cref;

thread_local! {
    /// 键 → 系统图像列表里的下标。取不到也记 None，免得每帧重问一次。
    static IDX: RefCell<HashMap<String, Option<i32>>> = RefCell::new(HashMap::new());
    /// 键 →（边长, 底色）→ 已合成的位图。外层用 &str 查，命中路径不分配。
    static TILES: RefCell<HashMap<String, HashMap<(i32, u32), Tile>>> = RefCell::new(HashMap::new());
}

/// 每个文件图标都可能不同的类型：按完整路径要，才能看到各自的图标。
const PER_FILE: [&str; 7] = ["exe", "lnk", "ico", "cur", "msi", "cpl", "url"];

struct Tile {
    bmp: HBITMAP,
    bits: *mut c_void,
    side: i32,
}

impl Drop for Tile {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.bmp.0));
        }
    }
}

fn key_for(path: &str) -> Option<(String, bool)> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    if ext.is_empty() {
        return None;
    }
    if PER_FILE.contains(&ext.as_str()) {
        Some((path.to_ascii_lowercase(), true))
    } else {
        Some((ext, false))
    }
}

/// 画一枚系统图标。取不到就返回 false，调用方退回原来那枚单色文件标。
pub unsafe fn draw(hdc: HDC, path: &str, x: i32, y: i32, sz: i32, bg: u32) -> bool {
    let side = sz.max(2);
    let Some((key, per_file)) = key_for(path) else {
        return false;
    };
    let hit = TILES.with(|c| {
        c.borrow()
            .get(key.as_str())
            .and_then(|m| m.get(&(side, bg)))
            .map(|t| (t.bits, t.side))
    });
    if let Some((bits, s)) = hit {
        blit(hdc, bits, s, x, y);
        return true;
    }
    let Some(idx) = index(&key, per_file, path) else {
        return false;
    };
    let Some(h) = hicon(idx, side) else {
        return false;
    };
    let tile = build(hdc, h, side, bg);
    let _ = DestroyIcon(h);
    let Some(tile) = tile else {
        return false;
    };
    let (bits, s) = (tile.bits, tile.side);
    TILES.with(|c| {
        let mut m = c.borrow_mut();
        // 换肤、换缩放会攒出新键；图标本身很便宜，越界整表重来。
        if m.len() >= 128 {
            m.clear();
        }
        m.entry(key).or_default().insert((side, bg), tile);
    });
    blit(hdc, bits, s, x, y);
    true
}

unsafe fn index(key: &str, per_file: bool, path: &str) -> Option<i32> {
    if let Some(v) = IDX.with(|c| c.borrow().get(key).copied()) {
        return v;
    }
    let v = if per_file {
        // 文件不在（网络盘断线、右栏残留的旧路径）就退回这个扩展名的通用图标。
        sh_index(path, false).or_else(|| sh_index(&format!("x.{key}"), true))
    } else {
        sh_index(&format!("x.{key}"), true)
    };
    IDX.with(|c| {
        let mut m = c.borrow_mut();
        if m.len() >= 512 {
            m.clear();
        }
        m.insert(key.to_string(), v);
    });
    v
}

unsafe fn sh_index(name: &str, use_attrs: bool) -> Option<i32> {
    let w: Vec<u16> = name.encode_utf16().chain([0]).collect();
    let flags = if use_attrs {
        SHGFI_SYSICONINDEX | SHGFI_USEFILEATTRIBUTES
    } else {
        SHGFI_SYSICONINDEX
    };
    let mut fi = SHFILEINFOW::default();
    let n = SHGetFileInfoW(
        PCWSTR(w.as_ptr()),
        FILE_ATTRIBUTE_NORMAL,
        Some(&mut fi as *mut SHFILEINFOW),
        std::mem::size_of::<SHFILEINFOW>() as u32,
        flags,
    );
    (n != 0).then_some(fi.iIcon)
}

/// 按目标边长挑最接近且不小于它的那档：列表档二十来像素拿 32 缩下来才清楚，
/// 图标档 60 拿 48 略微放大，正好盖住格子。
unsafe fn hicon(idx: i32, side: i32) -> Option<HICON> {
    let shil = if side <= 16 {
        SHIL_SMALL
    } else if side <= 24 {
        SHIL_LARGE
    } else {
        SHIL_EXTRALARGE
    };
    let list = SHGetImageList::<IImageList>(shil as i32).ok()?;
    let h = list.GetIcon(idx, ILD_TRANSPARENT.0).ok()?;
    (!h.0.is_null()).then_some(h)
}

unsafe fn build(hdc: HDC, h: HICON, side: i32, bg: u32) -> Option<Tile> {
    let bmi = dib_bmi(side);
    let mut bits: *mut c_void = std::ptr::null_mut();
    let bmp = CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
    let mem = CreateCompatibleDC(Some(hdc));
    if mem.0.is_null() {
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        return None;
    }
    let old = SelectObject(mem, HGDIOBJ(bmp.0));
    let br = CreateSolidBrush(cref(bg));
    let r = RECT { left: 0, top: 0, right: side, bottom: side };
    FillRect(mem, &r, br);
    let _ = DeleteObject(HGDIOBJ(br.0));
    // 60 这一档要把 48 撑大，半色调比最近邻少一圈锯齿；HUGE 调色板时代的老图标
    // 本来就有锯齿，这里只求过渡别太硬。
    let _ = SetStretchBltMode(mem, HALFTONE);
    let _ = SetBrushOrgEx(mem, 0, 0, None);
    let drawn = DrawIconEx(mem, 0, 0, h, side, side, 0, None, DI_NORMAL).is_ok();
    SelectObject(mem, old);
    let _ = DeleteDC(mem);
    if !drawn {
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        return None;
    }
    Some(Tile { bmp, bits, side })
}

fn dib_bmi(side: i32) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: side,
            biHeight: -side,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

unsafe fn blit(hdc: HDC, bits: *mut c_void, side: i32, x: i32, y: i32) {
    let bmi = dib_bmi(side);
    let _ = SetDIBitsToDevice(
        hdc,
        x,
        y,
        side as u32,
        side as u32,
        0,
        0,
        0,
        side as u32,
        bits as *const c_void,
        &bmi,
        DIB_RGB_COLORS,
    );
}

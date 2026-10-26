#![allow(unsafe_op_in_unsafe_fn)]
#![windows_subsystem = "windows"]
//! Xsee 图像编辑：独立的编辑窗口。菜单/工具条/对话框全部按 skin 自绘，
//! 像素运算在 xsee_host::edit 里（移植自 nomacs 3.22.4 的同名功能）。
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use xsee_host::theme::{self, cref, Look, Theme};
use xsee_host::{edit, encode};
use image::RgbaImage;
use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Graphics::Dwm::{
    DWMNCRENDERINGPOLICY, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY,
    OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::*;

const CLASS: &str = "XseeEditWnd";
const TITLE: &str = "Xsee 图像编辑";
const WM_NOTIFY_EDIT: u32 = WM_APP + 9;

const M_OPEN: usize = 500;
const M_SAVE: usize = 501;
const M_SAVEAS: usize = 502;
const M_CLOSE: usize = 503;
const M_UNDO: usize = 510;
const M_REDO: usize = 511;
const M_REVERT: usize = 512;
const M_FIT: usize = 520;
const M_ACTUAL: usize = 521;
const M_ZOOMIN: usize = 522;
const M_ZOOMOUT: usize = 523;
const M_PAN: usize = 524;
const M_GRAY: usize = 530;
const M_INVERT: usize = 531;
const M_THRESHOLD: usize = 532;
const M_AUTOLEVEL: usize = 533;
const M_HUE: usize = 534;
const M_BALANCE: usize = 535;
const M_AUTOADJ: usize = 540;
const M_NORMALIZE: usize = 541;
const M_EXPOSURE: usize = 542;
const M_RESIZE: usize = 543;
const M_ROT90: usize = 544;
const M_ROT180: usize = 545;
const M_ROT270: usize = 546;
const M_FLIPH: usize = 547;
const M_FLIPV: usize = 548;
const M_CROP: usize = 549;
const M_ROTATE: usize = 553;
const M_BLUR: usize = 550;
const M_SHARPEN: usize = 551;
const M_DESPECKLE: usize = 552;
const M_BG: usize = 560;
const M_ABOUT: usize = 570;

const T_MIN: usize = 600;
const T_MAX: usize = 601;
const T_CLOSE: usize = 602;

const D_OK: usize = 700;
const D_CANCEL: usize = 701;
const D_CLOSE: usize = 702;
const D_SLIDER: usize = 750;
const D_TITLEBAR: usize = 759;
const D_SAVE: usize = 730;
const D_DISCARD: usize = 731;
const D_STAY: usize = 732;
const D_KEEP: usize = 740;
const D_FW: usize = 742;
const D_FH: usize = 743;

struct Mi {
    label: &'static str,
    key: &'static str,
    id: usize,
}

const fn mi(label: &'static str, key: &'static str, id: usize) -> Mi {
    Mi { label, key, id }
}

const FILE_MENU: &[Mi] = &[
    mi("打开(O)...", "Ctrl+O", M_OPEN),
    mi("保存(S)", "Ctrl+S", M_SAVE),
    mi("另存为(A)...", "Ctrl+Shift+S", M_SAVEAS),
    mi("关闭(X)", "Esc", M_CLOSE),
];
const EDIT_MENU: &[Mi] = &[
    mi("撤销(U)", "Ctrl+Z", M_UNDO),
    mi("重做(R)", "Ctrl+Y", M_REDO),
    mi("还原为原图", "", M_REVERT),
];
const ZOOM_MENU: &[Mi] = &[
    mi("适应窗口(F)", "Ctrl+0", M_FIT),
    mi("实际大小(100%)", "Ctrl+1", M_ACTUAL),
    mi("放大(+)", "Ctrl++", M_ZOOMIN),
    mi("缩小(-)", "Ctrl+-", M_ZOOMOUT),
];
const COLOR_MENU: &[Mi] = &[
    mi("灰度(G)", "", M_GRAY),
    mi("反转图像(I)", "", M_INVERT),
    mi("阈值(T)...", "", M_THRESHOLD),
    mi("自动色阶调整(A)", "", M_AUTOLEVEL),
    mi("色调/饱和度(H)...", "", M_HUE),
    mi("颜色均衡(B)...", "", M_BALANCE),
];
const ADJUST_MENU: &[Mi] = &[
    mi("自动调整(A)", "", M_AUTOADJ),
    mi("标准化图像(N)", "", M_NORMALIZE),
    mi("曝光(E)...", "", M_EXPOSURE),
    mi("调整大小(R)...", "", M_RESIZE),
    mi("旋转角度(A)...", "", M_ROTATE),
    mi("向左旋转 90°", "", M_ROT270),
    mi("向右旋转 90°", "", M_ROT90),
    mi("旋转 180°", "", M_ROT180),
    mi("水平翻转（左右）", "", M_FLIPH),
    mi("垂直翻转（上下）", "", M_FLIPV),
    mi("裁剪(C)", "Enter/双击 应用", M_CROP),
];
const TB_ROT_MENU: &[Mi] = &[
    mi("向左旋转 90°", "", M_ROT270),
    mi("向右旋转 90°", "", M_ROT90),
];
const TB_FLIP_MENU: &[Mi] = &[
    mi("水平翻转（左右）", "", M_FLIPH),
    mi("垂直翻转（上下）", "", M_FLIPV),
];

fn tb_items(id: usize) -> &'static [Mi] {
    match id {
        M_ROT90 => TB_ROT_MENU,
        M_FLIPH => TB_FLIP_MENU,
        _ => &[],
    }
}
const FILTER_MENU: &[Mi] = &[
    mi("模糊(B)...", "", M_BLUR),
    mi("锐化(S)...", "", M_SHARPEN),
    mi("消除斑点(D)", "", M_DESPECKLE),
];
const EFFECT_MENU: &[Mi] = &[mi("背景颜色(B)...", "", M_BG)];
const HELP_MENU: &[Mi] = &[mi("关于(A)", "", M_ABOUT)];

const MENUS: &[(&str, &[Mi])] = &[
    ("文件(F)", FILE_MENU),
    ("编辑(E)", EDIT_MENU),
    ("缩放至(Z)", ZOOM_MENU),
    ("颜色(C)", COLOR_MENU),
    ("调整(A)", ADJUST_MENU),
    ("过滤(L)", FILTER_MENU),
    ("效果(X)", EFFECT_MENU),
    ("帮助(H)", HELP_MENU),
];

struct Tb {
    id: usize,
    label: &'static str,
    icon: u8,
}

const TOOLS: &[Tb] = &[
    Tb { id: M_SAVE, label: "保存", icon: 0 },
    Tb { id: M_UNDO, label: "撤销", icon: 1 },
    Tb { id: M_REDO, label: "重做", icon: 2 },
    Tb { id: 0, label: "", icon: 99 },
    Tb { id: M_PAN, label: "滚动", icon: 15 },
    Tb { id: M_CROP, label: "裁剪", icon: 3 },
    Tb { id: M_ZOOMOUT, label: "缩小", icon: 4 },
    Tb { id: M_ZOOMIN, label: "放大", icon: 5 },
    Tb { id: M_RESIZE, label: "调整大小", icon: 6 },
    Tb { id: M_ROT90, label: "旋转", icon: 7 },
    Tb { id: M_FLIPH, label: "翻转", icon: 8 },
    Tb { id: M_BALANCE, label: "颜色均衡", icon: 9 },
    Tb { id: M_HUE, label: "色调调整", icon: 10 },
    Tb { id: M_AUTOLEVEL, label: "自动色阶", icon: 11 },
    Tb { id: M_BLUR, label: "模糊", icon: 12 },
    Tb { id: M_SHARPEN, label: "锐化", icon: 13 },
    Tb { id: M_DESPECKLE, label: "消除斑点", icon: 14 },
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PK {
    Resize,
    Rotate,
    Exposure,
    Hue,
    Threshold,
    Blur,
    Sharpen,
    Balance,
    Bg,
    Exit,
}

impl PK {
    fn title(self) -> &'static str {
        match self {
            PK::Resize => "调整大小",
            PK::Rotate => "旋转",
            PK::Exposure => "曝光",
            PK::Hue => "色调/饱和度",
            PK::Threshold => "阈值",
            PK::Blur => "模糊",
            PK::Sharpen => "锐化",
            PK::Balance => "颜色均衡",
            PK::Bg => "背景颜色",
            PK::Exit => "保存更改",
        }
    }
    fn labels(self) -> &'static [&'static str] {
        match self {
            PK::Resize => &["宽度", "高度"],
            PK::Rotate => &["角度"],
            PK::Exposure => &["曝光", "伽马"],
            PK::Hue => &["色相", "饱和度", "亮度"],
            PK::Threshold => &["阈值"],
            PK::Blur => &["半径"],
            PK::Sharpen => &["半径", "强度"],
            PK::Balance => &["红", "绿", "蓝"],
            PK::Bg => &["红", "绿", "蓝"],
            PK::Exit => &[],
        }
    }
    fn steps(self) -> &'static [f64] {
        match self {
            PK::Resize => &[1.0, 1.0],
            PK::Rotate => &[15.0],
            PK::Exposure => &[0.5, 0.1],
            PK::Hue => &[5.0, 0.05, 0.02],
            PK::Threshold => &[5.0],
            PK::Blur => &[1.0],
            PK::Sharpen => &[0.5, 0.1],
            PK::Balance => &[5.0, 5.0, 5.0],
            PK::Bg => &[5.0, 5.0, 5.0],
            PK::Exit => &[],
        }
    }
    /// 每一行拖得到的上下限。滑块的头出不了这一档，值也一律夹在它里面。
    fn ranges(self) -> &'static [(f64, f64)] {
        match self {
            PK::Resize => &[(1.0, 20000.0), (1.0, 20000.0)],
            PK::Rotate => &[(0.0, 360.0)],
            PK::Exposure => &[(-3.0, 3.0), (0.2, 3.0)],
            PK::Hue => &[(-180.0, 180.0), (0.0, 3.0), (-1.0, 1.0)],
            PK::Threshold => &[(0.0, 255.0)],
            PK::Blur => &[(0.0, 20.0)],
            PK::Sharpen => &[(0.0, 10.0), (0.0, 3.0)],
            PK::Balance => &[(-100.0, 100.0), (-100.0, 100.0), (-100.0, 100.0)],
            PK::Bg => &[(0.0, 255.0), (0.0, 255.0), (0.0, 255.0)],
            PK::Exit => &[],
        }
    }
    /// 这一页拖滑块时要不要当场把效果算到图上给他看。
    /// 逐像素那几页一趟几十毫秒，拖一路都算跟得上；模糊/锐化/旋转是按半径铺开的重活，
    /// 一路算会把界面拖死，那几页改成松手算一次。
    fn live(self) -> bool {
        matches!(self, PK::Exposure | PK::Hue | PK::Threshold | PK::Balance | PK::Bg)
    }
}

struct Dlg {
    kind: PK,
    rect: RECT,
    vals: [f64; 3],
    focus: usize,
    buf: String,
    keep: bool,
    aspect: f64,
}

// 编辑器只在一个 UI 线程里碰 GDI 句柄；这里让全局状态能放进 OnceLock<Mutex<_>>。
unsafe impl Send for Ed {}

struct Ed {
    th: Theme,
    scale: f64,
    skin: String,
    density: String,
    look: Look,
    hwnd: HWND,
    path: PathBuf,
    img: RgbaImage,
    orig: RgbaImage,
    bgra: Vec<u8>,
    iw: u32,
    ih: u32,
    zoom: f64,
    fit: bool,
    pan: (i32, i32),
    undo: Vec<RgbaImage>,
    redo: Vec<RgbaImage>,
    crop_mode: bool,
    crop: Option<(i32, i32, i32, i32)>,
    crop_seed: Option<(i32, i32)>,
    drag_pan: Option<(i32, i32, i32, i32)>,
    font: HFONT,
    /// 标题栏左角那枚图标：(尺寸, 句柄)。尺寸变了才重取一份，取到就还旧的。
    tb_icon: (i32, HICON),
    font_sm: HFONT,
    font_b: HFONT,
    font_num: HFONT,
    menu_open: Option<usize>,
    tb_open: Option<usize>,
    mb_rows: Vec<(usize, RECT)>,
    pop_rows: Vec<(usize, RECT)>,
    tb_rows: Vec<(usize, RECT)>,
    title_rows: Vec<(usize, RECT)>,
    dlg: Option<Dlg>,
    dlg_rows: Vec<(usize, RECT)>,
    /// 打开参数框时那张图。拖滑块时的预览一律从它身上算，取消就整张还回它
    dlg_base: Option<RgbaImage>,
    /// 按住框的标题条在拖这扇框：按下点相对框左上的偏移
    dlg_drag: Option<(i32, i32)>,
    /// 按住第几行滑块在拖
    row_drag: Option<usize>,
    pt: (i32, i32),
    hint: String,
    status: String,
    notify: Option<isize>,
    dirty: bool,
}

fn ed() -> &'static Mutex<Ed> {
    static E: OnceLock<Mutex<Ed>> = OnceLock::new();
    E.get_or_init(|| {
        Mutex::new(Ed {
            th: Theme::new(&theme::default_name(), "normal", 1.0),
            scale: 1.0,
            skin: theme::default_name(),
            density: "normal".into(),
            look: Look::default(),
            hwnd: HWND::default(),
            path: PathBuf::new(),
            img: RgbaImage::new(1, 1),
            orig: RgbaImage::new(1, 1),
            bgra: vec![0; 4],
            iw: 1,
            ih: 1,
            zoom: 1.0,
            fit: true,
            pan: (0, 0),
            undo: Vec::new(),
            redo: Vec::new(),
            crop_mode: false,
            crop: None,
            crop_seed: None,
            drag_pan: None,
            font: HFONT::default(),
            tb_icon: (0, HICON::default()),
            font_sm: HFONT::default(),
            font_b: HFONT::default(),
            font_num: HFONT::default(),
            menu_open: None,
            tb_open: None,
            mb_rows: Vec::new(),
            pop_rows: Vec::new(),
            tb_rows: Vec::new(),
            title_rows: Vec::new(),
            dlg: None,
            dlg_rows: Vec::new(),
            dlg_base: None,
            dlg_drag: None,
            row_drag: None,
            pt: (-1, -1),
            hint: String::new(),
            status: String::new(),
            notify: None,
            dirty: false,
        })
    })
}

fn pw(s: &str) -> &'static [u16] {
    // 字体名就那几样：按值缓存住。原来每次 Box::leak 一份，跨 DPI 显示器
    // 拖几个来回就攒几份永远不还的内存。
    static SEEN: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, &'static [u16]>>> =
        std::sync::OnceLock::new();
    let mut m = SEEN
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
        .lock()
        .unwrap();
    if let Some(v) = m.get(s) {
        return *v;
    }
    let leaked: &'static [u16] =
        Box::leak(s.encode_utf16().chain([0]).collect::<Vec<u16>>().into_boxed_slice());
    m.insert(s.to_string(), leaked);
    leaked
}

fn stem(path: &Path) -> String {
    path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string()
}

fn over(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

unsafe fn fill(hdc: HDC, r: &RECT, c: u32) {
    let b = CreateSolidBrush(cref(c));
    let _ = FillRect(hdc, r, b);
    let _ = DeleteObject(HGDIOBJ(b.0));
}

unsafe fn frame(hdc: HDC, r: &RECT, c: u32) {
    let b = CreateSolidBrush(cref(c));
    let _ = FrameRect(hdc, r, b);
    let _ = DeleteObject(HGDIOBJ(b.0));
}

// ── 下面这组条带工艺与主窗口同一套：尺寸令牌、底色、蚀刻边、悬停态全部照搬，
//    编辑器不再自成一套比例。

fn bands(th: &Theme) -> bool {
    th.style != theme::Style::Plain
}

/// 三套完整版式的判据全部走 theme::Style，和主窗口问的是同一句：
/// 同一套皮肤在两扇窗里不会走出两个答案。
fn fine(th: &Theme) -> bool {
    th.style.is_fine()
}

fn draft(th: &Theme) -> bool {
    th.style.is_draft()
}

fn quiet(th: &Theme) -> bool {
    th.style.is_quiet()
}

/// 圆角裁剪。GDI 的裁剪区没有栈：先把当前那份取出来存着，画完再放回去。
unsafe fn round_clip(hdc: HDC, r: &RECT, rad: i32) -> Option<HRGN> {
    let keep = CreateRectRgn(0, 0, 0, 0);
    let had = GetClipRgn(hdc, keep) == 1;
    let rgn = CreateRoundRectRgn(r.left, r.top, r.right, r.bottom, rad * 2, rad * 2);
    let _ = ExtSelectClipRgn(hdc, Some(rgn), RGN_AND);
    let _ = DeleteObject(HGDIOBJ(rgn.0));
    if had {
        Some(keep)
    } else {
        let _ = DeleteObject(HGDIOBJ(keep.0));
        None
    }
}

unsafe fn unround(hdc: HDC, keep: Option<HRGN>) {
    match keep {
        Some(r) => {
            let _ = SelectClipRgn(hdc, Some(r));
            let _ = DeleteObject(HGDIOBJ(r.0));
        }
        None => {
            let _ = SelectClipRgn(hdc, None);
        }
    }
}

/// 圆角描边一圈。GDI 描不了半透明，颜色由调用方按底下那层换算好。
unsafe fn rring(hdc: HDC, r: &RECT, rad: i32, c: u32) {
    rring_w(hdc, r, rad, c, 1);
}

/// 圆角框带笔宽。GDI 的笔压在路径正中，要让它整条落在 r 里面，
/// 传进来的 r 得先往里缩 w / 2。
unsafe fn rring_w(hdc: HDC, r: &RECT, rad: i32, c: u32, w: i32) {
    let pen = CreatePen(PS_SOLID, w, cref(c));
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let ob = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    let _ = RoundRect(hdc, r.left, r.top, r.right, r.bottom, rad * 2, rad * 2);
    SelectObject(hdc, ob);
    SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(pen.0));
}

unsafe fn hline(hdc: HDC, x0: i32, x1: i32, y: i32, c: u32) {
    fill(hdc, &RECT { left: x0, top: y, right: x1.max(x0 + 1), bottom: y + 1 }, c);
}

unsafe fn vline(hdc: HDC, x: i32, y0: i32, y1: i32, c: u32) {
    fill(hdc, &RECT { left: x, top: y0, right: x + 1, bottom: y1.max(y0 + 1) }, c);
}

unsafe fn veil(hdc: HDC, r: &RECT, c: u32, alpha: u8) {
    let w = r.right - r.left;
    let h = r.bottom - r.top;
    if w <= 0 || h <= 0 || alpha == 0 {
        return;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let Ok(dib) = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
        return;
    };
    let src = CreateCompatibleDC(Some(hdc));
    let old = SelectObject(src, HGDIOBJ(dib.0));
    fill(src, &RECT { left: 0, top: 0, right: w, bottom: h }, c);
    let bf = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: alpha,
        AlphaFormat: 0,
    };
    let _ = AlphaBlend(hdc, r.left, r.top, w, h, src, 0, 0, w, h, bf);
    SelectObject(src, old);
    let _ = DeleteObject(HGDIOBJ(dib.0));
    let _ = DeleteDC(src);
}

unsafe fn shell(hdc: HDC, g: &RECT, th: &Theme, base: u32, lit: bool, border_top: bool) {
    let s = th.skin;
    fill(hdc, g, if lit { theme::mix(base, s.chrome_hi, 0.10) } else { base });
    let bc = if bands(th) { s.line_soft } else { s.line };
    if border_top {
        hline(hdc, g.left, g.right, g.top, bc);
    } else {
        hline(hdc, g.left, g.right, g.bottom - 1, bc);
    }
}

/// 弹层投影，跟主窗口同一套：同尺寸灰纱往右下让 6 像素、AlphaBlend 50%。
const SHADOW_GRAY: u32 = 0x009E9E9E;

unsafe fn drop_shadow(hdc: HDC, r: &RECT, k: f64) {
    let off = (4.0 * k) as i32 + 1;
    let q = RECT { left: r.left + off, top: r.top + off, right: r.right + off, bottom: r.bottom + off };
    veil(hdc, &q, SHADOW_GRAY, 128);
}

unsafe fn xmark(hdc: HDC, cx: i32, cy: i32, d: i32, col: u32) {
    let a = [POINT { x: cx - d, y: cy - d }, POINT { x: cx + d, y: cy + d }];
    let b = [POINT { x: cx + d, y: cy - d }, POINT { x: cx - d, y: cy + d }];
    line(hdc, col, 1, a[0].x, a[0].y, a[1].x, a[1].y);
    line(hdc, col, 1, b[0].x, b[0].y, b[1].x, b[1].y);
}

unsafe fn wbtn_glyph(hdc: HDC, id: usize, r: &RECT, col: u32, k: f64) {
    let (cx, cy) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
    match id {
        T_MIN => hline(hdc, cx - (5.0 * k) as i32, cx + (5.0 * k) as i32, cy, col),
        T_MAX => frame(
            hdc,
            &RECT {
                left: cx - (4.0 * k) as i32,
                top: cy - (4.0 * k) as i32,
                right: cx + (4.0 * k) as i32,
                bottom: cy + (3.0 * k) as i32,
            },
            col,
        ),
        _ => xmark(hdc, cx, cy, (5.0 * k) as i32, col),
    }
}

unsafe fn txt(hdc: HDC, r: &RECT, s: &str, c: u32, font: HFONT, extra: DRAW_TEXT_FORMAT) {
    if s.is_empty() {
        return;
    }
    SelectObject(hdc, HGDIOBJ(font.0));
    SetTextColor(hdc, cref(c));
    SetBkMode(hdc, TRANSPARENT);
    let mut u: Vec<u16> = s.encode_utf16().collect();
    let mut rc = *r;
    let _ = DrawTextW(hdc, &mut u, &mut rc, DT_NOPREFIX | extra);
}

unsafe fn text_w(hdc: HDC, s: &str, font: HFONT) -> i32 {
    SelectObject(hdc, HGDIOBJ(font.0));
    let mut u: Vec<u16> = s.encode_utf16().collect();
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    let _ = DrawTextW(hdc, &mut u, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
    r.right - r.left
}

unsafe fn mk_font(px: i32, bold: bool, face: &str) -> HFONT {
    CreateFontW(
        -px.max(9),
        0,
        0,
        0,
        if bold { FW_SEMIBOLD.0 as i32 } else { FW_NORMAL.0 as i32 },
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        0x0200,
        PCWSTR(pw(face).as_ptr()),
    )
}

unsafe fn line(hdc: HDC, c: u32, w: i32, x0: i32, y0: i32, x1: i32, y1: i32) {
    let p = CreatePen(PS_SOLID, w, cref(c));
    let old = SelectObject(hdc, HGDIOBJ(p.0));
    let _ = MoveToEx(hdc, x0, y0, None);
    let _ = LineTo(hdc, x1, y1);
    SelectObject(hdc, old);
    let _ = DeleteObject(HGDIOBJ(p.0));
}

unsafe fn glyph(hdc: HDC, f: HFONT, r: &RECT, s: &str, c: u32) {
    txt(hdc, r, s, c, f, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
}

/// SVG 路径 → 折线。和主窗口同一套 24 格解析：M/L/H/V/C/S/A/Z（含相对形式），
/// 弧线按 15° 采样，27px 的图标上看不出多边形。
fn svg_toks(d: &str) -> Vec<(char, f64)> {
    let b = d.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_alphabetic() {
            out.push((c, 0.0));
            i += 1;
        } else if c.is_ascii_digit() || c == '.' || c == '-' {
            let st = i;
            if c == '-' {
                i += 1;
            }
            while i < b.len() && (b[i] as char).is_ascii_digit() {
                i += 1;
            }
            if i < b.len() && b[i] == b'.' {
                i += 1;
                while i < b.len() && (b[i] as char).is_ascii_digit() {
                    i += 1;
                }
            }
            out.push((' ', d[st..i].parse().unwrap_or(0.0)));
        } else {
            i += 1;
        }
    }
    out
}

fn arc_pts(out: &mut Vec<(f64, f64)>, p0: (f64, f64), rx: f64, ry: f64, large: bool, sweep: bool, p1: (f64, f64)) {
    let ((x0, y0), (x1, y1)) = (p0, p1);
    if rx == 0.0 || ry == 0.0 {
        out.push(p1);
        return;
    }
    let (rx, ry) = (rx.abs(), ry.abs());
    let (dx, dy) = ((x0 - x1) / 2.0, (y0 - y1) / 2.0);
    let lam = dx * dx / (rx * rx) + dy * dy / (ry * ry);
    let (rx, ry) = if lam > 1.0 {
        let s = lam.sqrt();
        (rx * s, ry * s)
    } else {
        (rx, ry)
    };
    let num = (rx * rx * ry * ry - rx * rx * dy * dy - ry * ry * dx * dx).max(0.0);
    let den = rx * rx * dy * dy + ry * ry * dx * dx;
    let coef = if large != sweep { 1.0 } else { -1.0 } * (num / den.max(1e-12)).sqrt();
    let (cxp, cyp) = (coef * rx * dy / ry, coef * -ry * dx / rx);
    let (cx, cy) = ((x0 + x1) / 2.0 + cxp, (y0 + y1) / 2.0 + cyp);
    let ang = |ux: f64, uy: f64, vx: f64, vy: f64| {
        let dot = (ux * vx + uy * vy) / ((ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt()).max(1e-12);
        let mut a = dot.clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 {
            a = -a;
        }
        a
    };
    let th1 = ang(1.0, 0.0, (dx - cxp) / rx, (dy - cyp) / ry);
    let mut dth = ang((dx - cxp) / rx, (dy - cyp) / ry, (-dx - cxp) / rx, (-dy - cyp) / ry);
    if !sweep && dth > 0.0 {
        dth -= std::f64::consts::TAU;
    }
    if sweep && dth < 0.0 {
        dth += std::f64::consts::TAU;
    }
    let n = (dth.abs() / (std::f64::consts::PI / 12.0)).ceil().max(2.0) as usize;
    for k in 1..=n {
        let t = th1 + dth * k as f64 / n as f64;
        out.push((cx + rx * t.cos(), cy + ry * t.sin()));
    }
}

fn parse_path(d: &str) -> Vec<Vec<(f64, f64)>> {
    let t = svg_toks(d);
    let mut subs: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut cur: Vec<(f64, f64)> = Vec::new();
    let (mut cx, mut cy) = (0.0, 0.0);
    let (mut sx, mut sy) = (0.0, 0.0);
    let mut ctrl: Option<(f64, f64)> = None;
    let mut i = 0;
    let mut cmd = 'M';
    let num = |t: &[(char, f64)], i: &mut usize| -> f64 {
        let v = t.get(*i).map(|x| x.1).unwrap_or(0.0);
        *i += 1;
        v
    };
    while i < t.len() {
        if t[i].0 != ' ' {
            cmd = t[i].0;
            i += 1;
        }
        let rel = cmd.is_ascii_lowercase();
        let up = cmd.to_ascii_uppercase();
        let has = |t: &[(char, f64)], i: usize| t.get(i).map(|x| x.0 == ' ').unwrap_or(false);
        // Z 不吃参数，它后面紧跟的必然是下一个命令字母。这道"必须有数字"的闸要对 Z 放行，
        // 否则一条 d 里第一个 z 就把整串截断，多段路径只剩第一段。
        if up != 'Z' && !has(&t, i) {
            break;
        }
        match up {
            'M' => {
                let (mut x, mut y) = (num(&t, &mut i), num(&t, &mut i));
                if rel {
                    x += cx;
                    y += cy;
                }
                if !cur.is_empty() {
                    subs.push(std::mem::take(&mut cur));
                }
                cx = x;
                cy = y;
                (sx, sy) = (x, y);
                cur.push((x, y));
                cmd = if rel { 'l' } else { 'L' };
                ctrl = None;
            }
            'L' => {
                let (mut x, mut y) = (num(&t, &mut i), num(&t, &mut i));
                if rel {
                    x += cx;
                    y += cy;
                }
                cx = x;
                cy = y;
                cur.push((x, y));
                ctrl = None;
            }
            'H' => {
                let mut x = num(&t, &mut i);
                if rel {
                    x += cx;
                }
                cx = x;
                cur.push((cx, cy));
                ctrl = None;
            }
            'V' => {
                let mut y = num(&t, &mut i);
                if rel {
                    y += cy;
                }
                cy = y;
                cur.push((cx, cy));
                ctrl = None;
            }
            'C' | 'S' => {
                let (x1, y1) = if up == 'C' {
                    let (mut a, mut b) = (num(&t, &mut i), num(&t, &mut i));
                    if rel {
                        a += cx;
                        b += cy;
                    }
                    (a, b)
                } else {
                    ctrl.map(|(a, b)| (2.0 * cx - a, 2.0 * cy - b)).unwrap_or((cx, cy))
                };
                let (mut x2, mut y2) = (num(&t, &mut i), num(&t, &mut i));
                let (mut x, mut y) = (num(&t, &mut i), num(&t, &mut i));
                if rel {
                    x2 += cx;
                    y2 += cy;
                    x += cx;
                    y += cy;
                }
                let (ox, oy) = (cx, cy);
                let n = ((x - ox).hypot(y - oy) * 0.5).clamp(6.0, 24.0) as usize;
                for k in 1..=n {
                    let t = k as f64 / n as f64;
                    let m = 1.0 - t;
                    let px = m * m * m * ox + 3.0 * m * m * t * x1 + 3.0 * m * t * t * x2 + t * t * t * x;
                    let py = m * m * m * oy + 3.0 * m * m * t * y1 + 3.0 * m * t * t * y2 + t * t * t * y;
                    cur.push((px, py));
                }
                ctrl = Some((x2, y2));
                cx = x;
                cy = y;
            }
            'A' => {
                let (rx, ry) = (num(&t, &mut i), num(&t, &mut i));
                let _rot = num(&t, &mut i);
                let lf = num(&t, &mut i) != 0.0;
                let sf = num(&t, &mut i) != 0.0;
                let (mut x, mut y) = (num(&t, &mut i), num(&t, &mut i));
                if rel {
                    x += cx;
                    y += cy;
                }
                arc_pts(&mut cur, (cx, cy), rx, ry, lf, sf, (x, y));
                cx = x;
                cy = y;
                ctrl = None;
            }
            'Z' => {
                if let Some(&f) = cur.first() {
                    if cur.last().map(|p| (p.0 - f.0).abs() > 0.01 || (p.1 - f.1).abs() > 0.01).unwrap_or(true) {
                        cur.push(f);
                    }
                }
                if !cur.is_empty() {
                    subs.push(std::mem::take(&mut cur));
                }
                (cx, cy) = (sx, sy);
                ctrl = None;
            }
            _ => break,
        }
    }
    if cur.len() > 1 {
        subs.push(cur);
    }
    subs
}

unsafe fn ibrush(hdc: HDC, col: u32) -> (HGDIOBJ, HGDIOBJ, HBRUSH) {
    let br = CreateSolidBrush(cref(col));
    let ob = SelectObject(hdc, HGDIOBJ(br.0));
    let op = SelectObject(hdc, GetStockObject(NULL_PEN));
    (ob, op, br)
}

unsafe fn irestore(hdc: HDC, ob: HGDIOBJ, op: HGDIOBJ, br: HBRUSH) {
    SelectObject(hdc, ob);
    SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(br.0));
}

/// 实心多边形。SVG 的 opacity 用 mix(col,bg) 现算，换皮肤时底下那层跟着换。
unsafe fn fill_svg(hdc: HDC, x: i32, y: i32, u: f64, col: u32, bg: u32, alpha: f64, d: &str) {
    let c = if alpha >= 0.999 { col } else { theme::mix(col, bg, 1.0 - alpha) };
    for pts in parse_path(d) {
        if pts.len() < 3 {
            continue;
        }
        let v: Vec<POINT> = pts
            .iter()
            .map(|(a, b)| POINT { x: x + (a * u).round() as i32, y: y + (b * u).round() as i32 })
            .collect();
        let (ob, op, br) = ibrush(hdc, c);
        let _ = Polygon(hdc, &v);
        irestore(hdc, ob, op, br);
    }
}

unsafe fn stroke_svg(hdc: HDC, x: i32, y: i32, u: f64, col: u32, w: f64, d: &str) {
    let pen = CreatePen(PS_GEOMETRIC, (w * u).round().max(1.0) as i32, cref(col));
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let ob = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    for pts in parse_path(d) {
        if pts.len() < 2 {
            continue;
        }
        let v: Vec<POINT> = pts
            .iter()
            .map(|(a, b)| POINT { x: x + (a * u).round() as i32, y: y + (b * u).round() as i32 })
            .collect();
        let _ = Polyline(hdc, &v);
    }
    SelectObject(hdc, ob);
    SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(pen.0));
}

unsafe fn ell_svg(hdc: HDC, x: i32, y: i32, u: f64, col: u32, bg: u32, alpha: f64, cx: f64, cy: f64, r: f64) {
    let c = if alpha >= 0.999 { col } else { theme::mix(col, bg, 1.0 - alpha) };
    let (px, py, pr) = (x + (cx * u).round() as i32, y + (cy * u).round() as i32, (r * u).round().max(1.0) as i32);
    let (ob, op, br) = ibrush(hdc, c);
    let _ = Ellipse(hdc, px - pr, py - pr, px + pr, py + pr);
    irestore(hdc, ob, op, br);
}

unsafe fn ell_stroke_svg(hdc: HDC, x: i32, y: i32, u: f64, col: u32, w: f64, cx: f64, cy: f64, r: f64) {
    let (px, py, pr) = (x + (cx * u).round() as i32, y + (cy * u).round() as i32, (r * u).round().max(1.0) as i32);
    let pen = CreatePen(PS_GEOMETRIC, (w * u).round().max(1.0) as i32, cref(col));
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let ob = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    let _ = Ellipse(hdc, px - pr, py - pr, px + pr, py + pr);
    SelectObject(hdc, ob);
    SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(pen.0));
}

/// 工具条图标：24 格网 + SVG 折线，工艺照主窗口 browse.rs 的 icon()。
/// 线宽以格为单位，随按钮尺寸缩放；实心块用 alpha 叠到底色上，和设计稿的 opacity 对齐。
unsafe fn icon(hdc: HDC, code: u8, r: &RECT, bg: u32, c: u32) {
    let (x, y) = (r.left, r.top);
    let u = (r.right - r.left).min(r.bottom - r.top) as f64 / 24.0;
    match code {
        0 => {
            // 保存：软盘
            fill_svg(hdc, x, y, u, c, bg, 0.34, "M5 4h10.5L19 7.5V20H5z");
            stroke_svg(hdc, x, y, u, c, 2.0, "M5 4h10.5L19 7.5V20H5z");
            stroke_svg(hdc, x, y, u, c, 2.0, "M8 4v5h6.5V4");
            fill_svg(hdc, x, y, u, c, bg, 0.55, "M8 13h8v7H8z");
        }
        1 => {
            // 撤销：回旋箭头
            stroke_svg(hdc, x, y, u, c, 2.4, "M4.5 10h9.5a5 5 0 0 1 0 10H10");
            stroke_svg(hdc, x, y, u, c, 2.4, "M4.5 10l4-4M4.5 10l4 4");
        }
        2 => {
            stroke_svg(hdc, x, y, u, c, 2.4, "M19.5 10H10a5 5 0 0 0 0 10h4");
            stroke_svg(hdc, x, y, u, c, 2.4, "M19.5 10l-4-4M19.5 10l-4 4");
        }
        3 => {
            // 裁剪：两条拐角线
            stroke_svg(hdc, x, y, u, c, 2.2, "M7 3v14h14");
            stroke_svg(hdc, x, y, u, c, 2.2, "M3 7h14v14");
        }
        4 | 5 => {
            // 缩小 / 放大：放大镜 + 中间符号，和主窗口"查找"同一画法
            ell_stroke_svg(hdc, x, y, u, c, 2.4, 10.5, 10.5, 6.4);
            stroke_svg(hdc, x, y, u, c, 2.4, "M15.2 15.2 20 20");
            stroke_svg(hdc, x, y, u, c, 2.0, "M7.4 10.5h6.2");
            if code == 5 {
                stroke_svg(hdc, x, y, u, c, 2.0, "M10.5 7.4v6.2");
            }
        }
        6 => {
            // 调整大小：对角双箭头
            stroke_svg(hdc, x, y, u, c, 2.2, "M14.5 4H20v5.5M9.5 20H4v-5.5M20 4l-6 6M4 20l6-6");
        }
        7 => {
            // 旋转：右上开口的弧形箭头
            stroke_svg(hdc, x, y, u, c, 2.4, "M21 4.5V10h-5.5");
            stroke_svg(hdc, x, y, u, c, 2.4, "M19 15a8 8 0 1 1-2-8.5L21 10");
        }
        8 => {
            // 翻转：左右镜像，带中轴
            fill_svg(hdc, x, y, u, c, bg, 1.0, "M10 5v14l-7-7z");
            fill_svg(hdc, x, y, u, c, bg, 0.55, "M14 5v14l7-7z");
            stroke_svg(hdc, x, y, u, c, 1.8, "M12 3v18");
        }
        9 => {
            // 颜色均衡：三条滑轨 + 滑块
            stroke_svg(hdc, x, y, u, c, 2.2, "M4 7h16M4 12h16M4 17h16");
            ell_svg(hdc, x, y, u, c, bg, 1.0, 8.0, 7.0, 2.3);
            ell_svg(hdc, x, y, u, c, bg, 1.0, 15.5, 12.0, 2.3);
            ell_svg(hdc, x, y, u, c, bg, 1.0, 11.0, 17.0, 2.3);
        }
        10 => {
            // 色调调整：色相环，右半格填色
            ell_stroke_svg(hdc, x, y, u, c, 2.2, 12.0, 12.0, 8.0);
            fill_svg(hdc, x, y, u, c, bg, 0.3, "M12 4a8 8 0 0 1 0 16z");
            stroke_svg(hdc, x, y, u, c, 2.0, "M12 4v16");
        }
        11 => {
            // 自动色阶：直方图
            fill_svg(hdc, x, y, u, c, bg, 1.0, "M4 20v-7h2.6v7zM9 20V8h2.6v12zM14 20v-9h2.6v9zM19 20V5h2.6v15z");
            stroke_svg(hdc, x, y, u, c, 2.0, "M3 21h18");
        }
        12 => {
            // 模糊：三层同心圆
            ell_stroke_svg(hdc, x, y, u, c, 2.0, 12.0, 12.0, 8.0);
            ell_stroke_svg(hdc, x, y, u, c, 2.2, 12.0, 12.0, 4.5);
            ell_svg(hdc, x, y, u, c, bg, 1.0, 12.0, 12.0, 1.7);
        }
        13 => {
            // 锐化：四角星
            fill_svg(hdc, x, y, u, c, bg, 1.0, "M12 3l2.2 6.8L21 12l-6.8 2.2L12 21l-2.2-6.8L3 12l6.8-2.2z");
        }
        14 => {
            // 消除斑点：圈住散点
            ell_stroke_svg(hdc, x, y, u, c, 2.2, 12.0, 12.0, 8.0);
            ell_svg(hdc, x, y, u, c, bg, 1.0, 9.0, 9.0, 1.5);
            ell_svg(hdc, x, y, u, c, bg, 1.0, 15.0, 10.0, 1.2);
            ell_svg(hdc, x, y, u, c, bg, 1.0, 10.5, 15.0, 1.6);
            ell_svg(hdc, x, y, u, c, bg, 1.0, 14.5, 15.0, 1.0);
        }
        15 => {
            // 滚动：四向平移箭头
            stroke_svg(hdc, x, y, u, c, 2.2, "M12 4v16M4 12h16");
            fill_svg(hdc, x, y, u, c, bg, 1.0, "M12 2.5l3.2 4.5H8.8zM12 21.5l3.2-4.5H8.8zM2.5 12l4.5-3.2v6.4zM21.5 12l-4.5-3.2v6.4z");
        }
        _ => {}
    }
}

#[derive(Clone, Copy)]
struct Lay {
    client: RECT,
    title: RECT,
    menu: RECT,
    bar: RECT,
    canvas: RECT,
    status: RECT,
}

fn lay(rc: RECT, th: &Theme) -> Lay {
    let title_h = th.title;
    let menu_h = th.menu;
    let bar_h = th.bar;
    let status_h = th.status;
    Lay {
        client: rc,
        title: RECT { left: rc.left, top: rc.top, right: rc.right, bottom: rc.top + title_h },
        menu: RECT { left: rc.left, top: rc.top + title_h, right: rc.right, bottom: rc.top + title_h + menu_h },
        bar: RECT {
            left: rc.left,
            top: rc.top + title_h + menu_h,
            right: rc.right,
            bottom: rc.top + title_h + menu_h + bar_h,
        },
        canvas: RECT {
            left: rc.left,
            top: rc.top + title_h + menu_h + bar_h,
            right: rc.right,
            bottom: rc.bottom - status_h,
        },
        status: RECT { left: rc.left, top: rc.bottom - status_h, right: rc.right, bottom: rc.bottom },
    }
}

fn zoom_at(g: &Ed, cv: RECT) -> f64 {
    if g.fit {
        let z = (cv.right - cv.left) as f64 / g.iw.max(1) as f64;
        let z2 = (cv.bottom - cv.top) as f64 / g.ih.max(1) as f64;
        z.min(z2).max(0.01)
    } else {
        g.zoom.clamp(0.02, 32.0)
    }
}

fn img_rect(g: &Ed, cv: RECT) -> RECT {
    let z = zoom_at(g, cv);
    let w = (g.iw as f64 * z).round() as i32;
    let h = (g.ih as f64 * z).round() as i32;
    let x = cv.left + (cv.right - cv.left - w) / 2 + g.pan.0;
    let y = cv.top + (cv.bottom - cv.top - h) / 2 + g.pan.1;
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

fn pop_geom(g: &Ed, l: &Lay, idx: usize) -> Vec<(usize, RECT)> {
    let k = g.scale;
    let th = &g.th;
    let items = MENUS.get(idx).map(|m| m.1).unwrap_or(&[]);
    let mut lw = 0;
    let mut kw = 0;
    for it in items {
        lw = lw.max(it.label.chars().count() as i32 * (th.font / 2 + 2));
        if !it.key.is_empty() {
            kw = kw.max(it.key.chars().count() as i32 * (th.font_sm / 2 + 1));
        }
    }
    let w = ((18.0 * k) as i32 + lw + (14.0 * k) as i32 + kw + (16.0 * k) as i32 + (16.0 * k) as i32).max((150.0 * k) as i32);
    let h = items.len() as i32 * th.row + (4.0 * k) as i32;
    let ax = l.menu.left + g.mb_rows.iter().find(|(i, _)| *i == idx).map(|(_, r)| r.left - l.menu.left).unwrap_or(0);
    let ay = l.menu.bottom;
    let mut x = ax;
    let mut y = ay;
    if x + w > l.client.right - 2 {
        x = (l.client.right - w - 2).max(l.client.left);
    }
    if y + h > l.client.bottom - 2 {
        y = (ay - h).max(l.client.top);
    }
    let mut out = Vec::new();
    let mut cy = y + (2.0 * k) as i32;
    for it in items {
        out.push((it.id, RECT { left: x, top: cy, right: x + w, bottom: cy + th.row }));
        cy += th.row;
    }
    out
}

fn tb_pop_geom(g: &Ed, l: &Lay, id: usize) -> Vec<(usize, RECT)> {
    let k = g.scale;
    let th = &g.th;
    let items = tb_items(id);
    if items.is_empty() {
        return Vec::new();
    }
    let mut lw = 0;
    for it in items {
        lw = lw.max(it.label.chars().count() as i32 * (th.font / 2 + 2));
    }
    let w = ((18.0 * k) as i32 + lw + (32.0 * k) as i32).max((130.0 * k) as i32);
    let h = items.len() as i32 * th.row + (4.0 * k) as i32;
    let Some(br) = g.tb_rows.iter().find(|(i, _)| *i == id).map(|(_, r)| *r) else {
        return Vec::new();
    };
    let mut x = br.left;
    let mut y = br.bottom + (2.0 * k) as i32;
    if x + w > l.client.right - 2 {
        x = (l.client.right - w - 2).max(l.client.left);
    }
    if y + h > l.client.bottom - 2 {
        y = (br.top - h - (2.0 * k) as i32).max(l.client.top);
    }
    let mut out = Vec::new();
    let mut cy = y + (2.0 * k) as i32;
    for it in items {
        out.push((it.id, RECT { left: x, top: cy, right: x + w, bottom: cy + th.row }));
        cy += th.row;
    }
    out
}

fn fmt_val(kind: PK, i: usize, v: f64) -> String {
    match kind {
        PK::Resize => format!("{v:.0}"),
        PK::Rotate => format!("{v:.0}°"),
        PK::Exposure => {
            if i == 0 {
                format!("{v:+.1} EV")
            } else {
                format!("{v:.2}")
            }
        }
        PK::Hue => {
            if i == 0 {
                format!("{v:+.0}°")
            } else if i == 1 {
                format!("{v:.2}×")
            } else {
                format!("{v:+.2}")
            }
        }
        PK::Threshold | PK::Blur => format!("{v:.0}"),
        PK::Sharpen => {
            if i == 0 {
                format!("{v:.1}")
            } else {
                format!("{v:.2}")
            }
        }
        PK::Balance => format!("{v:+.0}"),
        PK::Bg => format!("{v:.0}"),
        PK::Exit => String::new(),
    }
}

unsafe fn dlg_rect_for(g: &Ed, l: &Lay, kind: PK) -> RECT {
    let k = g.scale;
    let n = kind.labels().len();
    let w = (390.0 * k) as i32;
    let h = if kind == PK::Exit {
        (168.0 * k) as i32
    } else if kind == PK::Resize {
        (218.0 * k) as i32
    } else {
        (118.0 * k + n as f64 * 38.0 * k) as i32
    };
    let cx = (l.client.left + l.client.right) / 2;
    let cy = (l.client.top + l.client.bottom) / 2;
    RECT { left: cx - w / 2, top: cy - h / 2, right: cx + w / 2, bottom: cy + h / 2 }
}

unsafe fn open_param(hwnd: HWND, g: &mut Ed, kind: PK) {
    let rc = client(hwnd);
    let l = lay(rc, &g.th);
    let vals = match kind {
        PK::Resize => [g.iw as f64, g.ih as f64, 0.0],
        PK::Rotate => [90.0, 0.0, 0.0],
        PK::Exposure => [0.0, 1.0, 0.0],
        PK::Hue => [0.0, 1.0, 0.0],
        PK::Threshold => [128.0, 0.0, 0.0],
        PK::Blur => [3.0, 0.0, 0.0],
        PK::Sharpen => [1.5, 0.6, 0.0],
        PK::Balance => [0.0, 0.0, 0.0],
        PK::Bg => [255.0, 255.0, 255.0],
        PK::Exit => [0.0; 3],
    };
    let rect = dlg_rect_for(&g, &l, kind);
    g.menu_open = None;
    g.tb_open = None;
    let aspect = g.iw as f64 / g.ih.max(1) as f64;
    g.dlg = Some(Dlg { kind, rect, vals, focus: usize::MAX, buf: String::new(), keep: true, aspect });
    // 记下这一张：拖滑块时的预览都从它身上算，取消就还回它。
    // 关闭询问那一页没有参数也不预览，没必要为此复制一整张图。
    g.dlg_base = if kind == PK::Exit { None } else { Some(g.img.clone()) };
    g.dlg_drag = None;
    g.row_drag = None;
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn refresh_dib(g: &mut Ed) {
    g.iw = g.img.width();
    g.ih = g.img.height();
    g.bgra.clear();
    g.bgra.reserve((g.iw as usize) * (g.ih as usize) * 4);
    for p in g.img.pixels() {
        g.bgra.extend_from_slice(&[p.0[2], p.0[1], p.0[0], p.0[3]]);
    }
}

unsafe fn push_state(g: &mut Ed, img: RgbaImage) {
    g.undo.push(img);
    if g.undo.len() > 12 {
        g.undo.remove(0);
    }
    g.redo.clear();
}

unsafe fn push_undo(g: &mut Ed) {
    let cur = g.img.clone();
    push_state(g, cur);
}

unsafe fn set_img(g: &mut Ed, img: RgbaImage) {
    g.img = img;
    refresh_dib(g);
    g.dirty = true;
    g.crop = None;
    g.crop_mode = false;
}

/// 把这一页的参数算到 base 那张上：还回新图 + 一句要说的话。
/// 图为 None 表示这一下不该动图（角度归 0、尺寸没变、目标过大那一类），话照样得说。
/// 确定时算一次、拖滑块预览时也走这一趟，两边看到的效果必然同一份。
fn render_param(base: &RgbaImage, d: &Dlg) -> (Option<RgbaImage>, String) {
    let v = d.vals;
    match d.kind {
        PK::Resize => {
            let w = v[0].round().clamp(1.0, 20000.0) as u32;
            let h = v[1].round().clamp(1.0, 20000.0) as u32;
            if w == base.width() && h == base.height() {
                return (None, "尺寸没有变化".to_string());
            }
            if w as u64 * h as u64 > 200_000_000 {
                return (None, "目标尺寸过大，没有执行".to_string());
            }
            let img = edit::resize(base, w, h);
            (Some(img), format!("已调整为 {w} × {h}"))
        }
        PK::Rotate => {
            let a = v[0].rem_euclid(360.0);
            if (a - 90.0).abs() < 0.01 || (a - 180.0).abs() < 0.01 || (a - 270.0).abs() < 0.01 {
                (Some(edit::rotate90(base, (a / 90.0).round() as u8)), String::new())
            } else if a.abs() > 0.01 {
                (Some(edit::rotate_any(base, a)), String::new())
            } else {
                (None, String::new())
            }
        }
        PK::Exposure => {
            let mut img = base.clone();
            edit::exposure(&mut img, v[0], v[1]);
            (Some(img), String::new())
        }
        PK::Hue => {
            let mut img = base.clone();
            edit::hue_sat(&mut img, v[0], v[1], v[2]);
            (Some(img), String::new())
        }
        PK::Threshold => {
            let mut img = base.clone();
            edit::threshold(&mut img, v[0].clamp(0.0, 255.0) as u8);
            (Some(img), String::new())
        }
        PK::Blur => {
            let mut img = base.clone();
            edit::blur(&mut img, v[0] as f32);
            (Some(img), String::new())
        }
        PK::Sharpen => {
            let mut img = base.clone();
            edit::sharpen(&mut img, v[0] as f32, v[1] as f32);
            (Some(img), String::new())
        }
        PK::Balance => {
            let mut img = base.clone();
            edit::color_balance(&mut img, v[0], v[1], v[2]);
            (Some(img), String::new())
        }
        PK::Bg => {
            (Some(edit::flatten_bg(base, [v[0] as u8, v[1] as u8, v[2] as u8])), String::new())
        }
        PK::Exit => (None, String::new()),
    }
}

/// 确定：算到"打开框时那张"上，改之前那张进撤销栈，算出来那张换进显示。
/// 预览画到屏上的那一层在这里整个作废重算，所以拖的过程中怎么中断都不会留下半成品。
unsafe fn commit_dlg(g: &mut Ed, d: &Dlg) {
    let base = g.dlg_base.take().unwrap_or_else(|| g.img.clone());
    let (img, status) = render_param(&base, d);
    if !status.is_empty() {
        g.status = status;
    }
    if let Some(img) = img {
        push_state(g, base);
        set_img(g, img);
    }
    update_title(g);
}

/// 取消 / ✕ / Esc：预览那一层抹掉，图回到打开框时那张
unsafe fn close_dlg(g: &mut Ed) {
    g.dlg = None;
    g.dlg_drag = None;
    g.row_drag = None;
    if let Some(base) = g.dlg_base.take() {
        g.img = base;
        refresh_dib(g);
    }
}

/// 拖滑块途中把效果叠到显示上：不进撤销栈、不置脏标记，确定那一刻才算真改过
unsafe fn preview(g: &mut Ed) {
    let out = match (g.dlg_base.as_ref(), g.dlg.as_ref()) {
        (Some(base), Some(d)) => render_param(base, d).0,
        _ => None,
    };
    if let Some(img) = out {
        g.img = img;
        refresh_dib(g);
    }
}

unsafe fn update_title(g: &Ed) {
    let name = if g.path.as_os_str().is_empty() {
        "未命名".to_string()
    } else {
        stem(&g.path)
    };
    let s = format!("{TITLE} — {name}{}", if g.dirty { " *" } else { "" });
    let _ = SetWindowTextW(g.hwnd, &HSTRING::from(s));
}

unsafe fn load_file(g: &mut Ed, path: &Path) -> Result<(), String> {
    // 换文件前把开着的参数框整个丢掉（不还原）：dlg_base 还指着上一张图，
    // 留着的话 Esc 一还原就把旧图挂在新路径上，再一保存就是张冠李戴。
    g.dlg = None;
    g.dlg_base = None;
    g.dlg_drag = None;
    g.row_drag = None;
    let img = encode::decode(&path.to_string_lossy(), false)?;
    g.orig = img.clone();
    g.img = img;
    g.path = path.to_path_buf();
    g.undo.clear();
    g.redo.clear();
    g.fit = true;
    g.pan = (0, 0);
    g.dirty = false;
    g.crop = None;
    g.crop_mode = false;
    refresh_dib(g);
    update_title(g);
    Ok(())
}

unsafe fn save_as_dialog(hwnd: HWND, cur: &Path) -> Option<PathBuf> {
    let mut buf = vec![0u16; 1024];
    let name: Vec<u16> = stem(cur).encode_utf16().collect();
    for (i, c) in name.iter().take(1000).enumerate() {
        buf[i] = *c;
    }
    let filter: Vec<u16> = "图像文件\0*.jpg;*.jpeg;*.png;*.bmp;*.tif;*.tiff;*.gif\0JPEG\0*.jpg;*.jpeg\0PNG\0*.png\0BMP\0*.bmp\0TIFF\0*.tif;*.tiff\0GIF\0*.gif\0\0"
        .encode_utf16()
        .collect();
    let init_dir: Vec<u16> = cur
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
        .encode_utf16()
        .chain([0])
        .collect();
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        nFilterIndex: 1,
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrInitialDir: PCWSTR(init_dir.as_ptr()),
        lpstrTitle: PCWSTR(pw("另存为").as_ptr()),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT | OFN_HIDEREADONLY,
        lpstrDefExt: PCWSTR(pw("jpg").as_ptr()),
        ..Default::default()
    };
    if !GetSaveFileNameW(&mut ofn).as_bool() {
        return None;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(PathBuf::from(String::from_utf16_lossy(&buf[..end])))
}

fn save_ext_ok(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("jpg" | "jpeg" | "jpe" | "jfif" | "png" | "bmp" | "tif" | "tiff" | "gif")
    )
}

// 文件对话框是模态的，会泵本窗口消息；持锁弹框必然和 paint 抢同一把锁，
// 所以先在锁外选路径，再进来保存。
unsafe fn save_entry(hwnd: HWND, pick: bool) -> bool {
    let cur = ed().lock().unwrap().path.clone();
    let path = if pick || cur.as_os_str().is_empty() || !save_ext_ok(&cur) {
        match save_as_dialog(hwnd, &cur) {
            Some(p) => p,
            None => return false,
        }
    } else {
        cur
    };
    let mut g = ed().lock().unwrap();
    save_to(hwnd, &mut g, path)
}

unsafe fn save_to(hwnd: HWND, g: &mut Ed, path: PathBuf) -> bool {
    match encode::save_rgba(&g.img, &path.to_string_lossy()) {
        Ok(()) => {
            g.path = path.clone();
            g.dirty = false;
            g.status = format!("已保存 {}", stem(&path));
            update_title(g);
            if let Some(h) = g.notify {
                let s = path.to_string_lossy().to_string();
                let cds = COPYDATASTRUCT {
                    dwData: WM_NOTIFY_EDIT as usize,
                    cbData: s.len() as u32,
                    lpData: s.as_ptr() as *mut core::ffi::c_void,
                };
                let _ = SendMessageW(
                    HWND(h as *mut core::ffi::c_void),
                    WM_COPYDATA,
                    Some(WPARAM(g.hwnd.0 as usize)),
                    Some(LPARAM(&cds as *const _ as isize)),
                );
            }
            true
        }
        Err(e) => {
            g.status = e;
            false
        }
    }
}

unsafe fn pick_open_path(hwnd: HWND) -> Option<PathBuf> {
    let mut buf = vec![0u16; 1024];
    let filter: Vec<u16> = "图像文件\0*.jpg;*.jpeg;*.png;*.bmp;*.gif;*.tif;*.tiff;*.webp;*.psd;*.heic\0所有文件\0*.*\0\0"
        .encode_utf16()
        .collect();
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        nFilterIndex: 1,
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrTitle: PCWSTR(pw("打开图像").as_ptr()),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_HIDEREADONLY,
        ..Default::default()
    };
    if !GetOpenFileNameW(&mut ofn).as_bool() {
        return None;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(PathBuf::from(String::from_utf16_lossy(&buf[..end])))
}

unsafe fn open_path(hwnd: HWND, p: &Path) {
    let mut g = ed().lock().unwrap();
    match load_file(&mut g, p) {
        Ok(()) => g.status = format!("已打开 {}", stem(p)),
        Err(e) => g.status = e,
    }
    drop(g);
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn request_exit(hwnd: HWND) {
    let mut g = ed().lock().unwrap();
    // 参数框还开着就先收掉（预览作废、回到打开框前那张），再问要不要保存。
    // 不然关闭询问这一页会把没提交的预览定格成工作图，取消后撤销链直接跳档。
    if g.dlg.is_some() {
        close_dlg(&mut g);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    if g.dirty {
        open_param(hwnd, &mut g, PK::Exit);
        return;
    }
    drop(g);
    let _ = DestroyWindow(hwnd);
}

unsafe fn do_command(hwnd: HWND, id: usize) {
    match id {
        M_OPEN => {
            if let Some(p) = pick_open_path(hwnd) {
                open_path(hwnd, &p);
            }
            return;
        }
        M_SAVE | M_SAVEAS => {
            save_entry(hwnd, id == M_SAVEAS);
            let _ = InvalidateRect(Some(hwnd), None, false);
            return;
        }
        _ => {}
    }
    let mut g = ed().lock().unwrap();
    match id {
        M_CLOSE => {
            drop(g);
            request_exit(hwnd);
            return;
        }
        M_UNDO => {
            if let Some(prev) = g.undo.pop() {
                let cur = g.img.clone();
                g.redo.push(cur);
                g.img = prev;
                refresh_dib(&mut g);
                // 撤回到和原图一模一样就不算脏了，关闭时不再问"保存更改"。
                g.dirty = g.orig != g.img;
            } else {
                g.status = "没有可撤销的操作".to_string();
            }
        }
        M_REDO => {
            if let Some(next) = g.redo.pop() {
                let cur = g.img.clone();
                g.undo.push(cur);
                g.img = next;
                refresh_dib(&mut g);
                g.dirty = g.orig != g.img;
            } else {
                g.status = "没有可重做的操作".to_string();
            }
        }
        M_REVERT => {
            if g.orig == g.img {
                g.status = "已经是原图".to_string();
            } else {
                push_undo(&mut g);
                let o = g.orig.clone();
                set_img(&mut g, o);
                // 还原后就是盘上那张：不置脏，标题不带星号。
                g.dirty = false;
                update_title(&g);
            }
        }
        M_FIT => {
            g.fit = true;
            g.pan = (0, 0);
        }
        M_ACTUAL => {
            g.fit = false;
            g.zoom = 1.0;
            g.pan = (0, 0);
        }
        M_ZOOMIN | M_ZOOMOUT => {
            let rc = client(hwnd);
            let l = lay(rc, &g.th);
            let z = zoom_at(&g, l.canvas);
            let nz = if id == M_ZOOMIN { z * 1.25 } else { z / 1.25 };
            g.fit = false;
            g.zoom = nz.clamp(0.02, 32.0);
        }
        M_GRAY => {
            let mut img = g.img.clone();
            edit::grayscale(&mut img);
            push_undo(&mut g);
            set_img(&mut g, img);
        }
        M_INVERT => {
            let mut img = g.img.clone();
            edit::invert(&mut img);
            push_undo(&mut g);
            set_img(&mut g, img);
        }
        M_AUTOLEVEL => {
            let mut img = g.img.clone();
            edit::auto_adjust(&mut img);
            push_undo(&mut g);
            set_img(&mut g, img);
        }
        M_AUTOADJ => {
            let mut img = g.img.clone();
            edit::auto_adjust(&mut img);
            edit::normalize(&mut img);
            push_undo(&mut g);
            set_img(&mut g, img);
        }
        M_NORMALIZE => {
            let mut img = g.img.clone();
            edit::normalize(&mut img);
            push_undo(&mut g);
            set_img(&mut g, img);
        }
        M_FLIPH | M_FLIPV => {
            let img = if id == M_FLIPH { edit::flip_h(&g.img) } else { edit::flip_v(&g.img) };
            push_undo(&mut g);
            set_img(&mut g, img);
        }
        M_ROT90 | M_ROT180 | M_ROT270 => {
            let t = if id == M_ROT90 { 1 } else if id == M_ROT180 { 2 } else { 3 };
            let img = edit::rotate90(&g.img, t);
            push_undo(&mut g);
            set_img(&mut g, img);
        }
        M_DESPECKLE => {
            let mut img = g.img.clone();
            edit::despeckle(&mut img);
            push_undo(&mut g);
            set_img(&mut g, img);
        }
        M_PAN => {
            g.status = "按住左键拖动画面滚动，方向键也可以移动".to_string();
        }
        M_CROP => {
            if g.crop.is_some() {
                apply_crop(&mut g);
            } else {
                g.crop_mode = !g.crop_mode;
                g.crop = None;
                g.status = if g.crop_mode {
                    "拖动框选要保留的区域，Enter 或双击应用，Esc 取消".to_string()
                } else {
                    String::new()
                };
            }
        }
        M_THRESHOLD => open_param(hwnd, &mut g, PK::Threshold),
        M_BLUR => open_param(hwnd, &mut g, PK::Blur),
        M_SHARPEN => open_param(hwnd, &mut g, PK::Sharpen),
        M_RESIZE => open_param(hwnd, &mut g, PK::Resize),
        M_ROTATE => open_param(hwnd, &mut g, PK::Rotate),
        M_EXPOSURE => open_param(hwnd, &mut g, PK::Exposure),
        M_HUE => open_param(hwnd, &mut g, PK::Hue),
        M_BALANCE => open_param(hwnd, &mut g, PK::Balance),
        M_BG => open_param(hwnd, &mut g, PK::Bg),
        M_ABOUT => {
            g.status = "Xsee 图像编辑 · 自绘界面 · 像素运算参考 nomacs 3.22.4".to_string();
        }
        _ => {}
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn apply_crop(g: &mut Ed) {
    let Some((x0, y0, x1, y1)) = g.crop else { return };
    let rc = client(g.hwnd);
    let l = lay(rc, &g.th);
    let ir = img_rect(g, l.canvas);
    let z = zoom_at(g, l.canvas);
    let cx0 = ((x0.min(x1) - ir.left) as f64 / z).round().max(0.0) as u32;
    let cy0 = ((y0.min(y1) - ir.top) as f64 / z).round().max(0.0) as u32;
    let cx1 = ((x0.max(x1) - ir.left) as f64 / z).round().clamp(0.0, g.iw as f64) as u32;
    let cy1 = ((y0.max(y1) - ir.top) as f64 / z).round().clamp(0.0, g.ih as f64) as u32;
    if cx1.saturating_sub(cx0) < 2 || cy1.saturating_sub(cy0) < 2 {
        g.status = "裁切区域太小".to_string();
        return;
    }
    let img = edit::crop(&g.img, cx0, cy0, cx1 - cx0, cy1 - cy0);
    push_undo(g);
    set_img(g, img);
    g.crop = None;
    g.crop_mode = false;
    g.status = "已裁切".to_string();
}

unsafe fn client(hwnd: HWND) -> RECT {
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    rc
}

unsafe fn paint(hdc: HDC, hwnd: HWND) {
    let mut g = ed().lock().unwrap();
    let rc = client(hwnd);
    let th = g.th.clone();
    let l = lay(rc, &th);
    let s = th.skin;
    let k = g.scale;
    fill(hdc, &rc, s.win);

    // 标题栏：底色、底缘那条线、按钮几何、四套版式的层次，全部取自 theme::Strip，
    // 和主窗口那一条是同一份算法算出来的，不再这里另定一套。整条铺纯色，不做渐变。
    let st = theme::Strip::top(&th);
    let bar = st.bar;
    let g0 = &l.title;
    fill(hdc, g0, st.bar);
    hline(hdc, g0.left, g0.right, g0.bottom - 1, st.edge);
    if st.ruler {
        // 制图台：左下压一道短尺，和主窗口标题栏同一个记号
        fill(
            hdc,
            &RECT {
                left: g0.left + (8.0 * k) as i32,
                top: g0.bottom - 3,
                right: g0.left + (72.0 * k) as i32,
                bottom: g0.bottom - 1,
            },
            st.dim,
        );
    }
    // 左角那枚图标：和主窗口同一份图稿，尺寸跟着条高走
    let isz = (g0.bottom - g0.top - (6.0 * k).round() as i32).max((12.0 * k) as i32);
    let mx = g0.left + (8.0 * k).round() as i32;
    if g.tb_icon.0 != isz {
        if !g.tb_icon.1.0.is_null() {
            let _ = DestroyIcon(g.tb_icon.1);
        }
        g.tb_icon = (isz, xsee_host::appicon::load_sized(isz));
    }
    if !g.tb_icon.1.0.is_null() {
        let _ = DrawIconEx(
            hdc,
            mx,
            g0.top + (g0.bottom - g0.top - isz) / 2,
            g.tb_icon.1,
            isz,
            isz,
            0,
            None,
            DI_NORMAL,
        );
    }
    g.title_rows.clear();
    let bw = st.bw;
    let bh = st.bh;
    let by = g0.top + (g0.bottom - g0.top - bh) / 2;
    let mut bx = g0.right - st.pad;
    for (id, gl) in [(T_CLOSE, "✕"), (T_MAX, "□"), (T_MIN, "─")] {
        let r = RECT { left: bx - bw, top: by, right: bx, bottom: by + bh };
        let hot = over(&r, g.pt.0, g.pt.1);
        let lay1 = if id == T_CLOSE { st.hot } else { st.hov };
        if fine(&th) {
            if hot {
                let hr = RECT { left: r.left + 2, top: r.top + 2, right: r.right - 2, bottom: r.bottom - 2 };
                let rad = (4.0 * k).round() as i32;
                let keep = round_clip(hdc, &hr, rad);
                fill(hdc, &hr, lay1);
                unround(hdc, keep);
                rring(hdc, &hr, rad, theme::mix(bar, st.fg, 0.45));
            }
            wbtn_glyph(hdc, id, &r, if hot { st.fg } else { st.dim }, k);
        } else if bands(&th) {
            // 静谧玻璃只给一层很淡的雾（st.hot 那一档已经是淡的）；制图台在悬停那两颗底下补一道索引线
            if hot {
                fill(hdc, &r, lay1);
            }
            if draft(&th) && hot && id != T_CLOSE {
                fill(
                    hdc,
                    &RECT { left: r.left + 4, top: r.bottom - 2, right: r.right - 4, bottom: r.bottom },
                    st.dim,
                );
            }
            wbtn_glyph(hdc, id, &r, if hot { st.fg } else { st.dim }, k);
        } else {
            if hot {
                fill(hdc, &r, lay1);
                frame(hdc, &r, st.ring);
            }
            // 极简档这三颗用的是字形，和主窗口同一句；字形换字体就换粗细，
            // 但这一档本来就没有线条可配。
            txt(
                hdc,
                &r,
                gl,
                if hot { st.fg } else { st.dim },
                g.font_sm,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
        }
        g.title_rows.push((id, r));
        bx = r.left - st.gap;
    }
    let name = if g.path.as_os_str().is_empty() {
        String::new()
    } else {
        format!(" — {}", stem(&g.path))
    };
    g.hint = String::new();
    txt(
        hdc,
        &RECT { left: mx + isz + (6.0 * k) as i32, top: g0.top, right: bx - (8.0 * k) as i32, bottom: g0.bottom },
        &format!("{TITLE}{name}{}", if g.dirty { " *" } else { "" }),
        st.ink,
        g.font,
        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
    );

    // 菜单条：读字的一行，底色是独立的 menu 槽，和浏览窗跟着同一套皮肤走
    let mbase = s.menu;
    shell(hdc, &l.menu, &th, mbase, false, false);
    g.mb_rows.clear();
    let pad = (8.0 * k).round() as i32;
    let mut mx = l.menu.left + (2.0 * k).round() as i32;
    for (i, (label, _)) in MENUS.iter().enumerate() {
        let w = text_w(hdc, label, g.font) + pad * 2;
        let r = RECT { left: mx, top: l.menu.top + 1, right: mx + w, bottom: l.menu.bottom - 1 };
        let open = g.menu_open == Some(i);
        let hot = open || over(&r, g.pt.0, g.pt.1);
        if bands(&th) {
            if hot {
                fill(hdc, &r, s.hov);
            }
            if open {
                fill(hdc, &RECT { left: r.left, top: r.bottom - 2, right: r.right, bottom: r.bottom }, s.acc);
            }
            txt(hdc, &r, label, if hot { s.ink } else { s.ink_dim }, g.font, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
        } else {
            if hot {
                fill(hdc, &r, if open { s.sel } else { s.hov });
                frame(hdc, &r, s.acc);
            }
            txt(hdc, &r, label, if open { s.sel_ink } else { s.ink }, g.font, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
        }
        g.mb_rows.push((i, r));
        mx = r.right;
    }

    // 工具条：与主窗口 paint_bar 的 below 排法逐项对齐——按钮宽按文字量，
    // 图标贴按钮顶、标签贴按钮底，起点 4k、按钮间距 1px、分隔线带一道亮边。
    if bands(&th) {
        shell(hdc, &l.bar, &th, s.chrome, true, false);
    } else {
        fill(hdc, &l.bar, s.chrome);
    }
    g.tb_rows.clear();
    let lab_h = th.font_sm + 2;
    let isz = th.icon.min((l.bar.bottom - l.bar.top - 6 - lab_h - 2).max(12));
    let p = (5.0 * k).round() as i32;
    let mut tx = l.bar.left + (4.0 * k).round() as i32;
    for t in TOOLS {
        if t.icon == 99 {
            let y0 = l.bar.top + (4.0 * k) as i32;
            let y1 = l.bar.bottom - (4.0 * k) as i32;
            let m = (3.0 * k).round() as i32;
            vline(hdc, tx + m, y0, y1, s.line_soft);
            if fine(&th) {
                vline(hdc, tx + m + 1, y0, y1, theme::mix(s.chrome_hi, s.canvas, 0.22));
            } else if !bands(&th) {
                vline(hdc, tx + m + 1, y0, y1, s.chrome_hi);
            }
            tx += m * 2 + 2;
            continue;
        }
        let tw = text_w(hdc, t.label, g.font_sm);
        let w = isz.max(tw) + p * 2;
        let r = RECT { left: tx, top: l.bar.top + 2, right: tx + w, bottom: l.bar.bottom - 2 };
        let hot = over(&r, g.pt.0, g.pt.1);
        let on = t.id == M_CROP && g.crop_mode;
        // 悬停底色和主窗口 paint_bar 同一档换算：精细/制图/静谧各有掺法，平阶直接用悬停色。
        // 裁切激活那一下仍铺浅主题色，只是往里再叠档位的框。
        let bgc = if on {
            s.acc_soft
        } else if hot {
            if draft(&th) {
                theme::mix(s.chrome_hi, s.hov, 0.72)
            } else if quiet(&th) {
                theme::mix(s.chrome, s.hov, 0.42)
            } else if fine(&th) {
                theme::mix(s.chrome, s.hov, 0.55)
            } else {
                s.hov
            }
        } else {
            s.chrome
        };
        let ink = if bands(&th) && !(on || hot) { s.ink_dim } else { s.ink };
        // 图标比文字再收一档：线条和字重不是一个量级，同色会显得图标更粗；
        // 制图台悬停那下往主色偏半格，和主窗口同一句。
        let iic = if draft(&th) && hot {
            theme::mix(s.acc, ink, 0.20)
        } else if bands(&th) && !(on || hot) {
            theme::mix(ink, bgc, 0.08)
        } else {
            ink
        };
        // 外框按版式分档，和主窗口 paint_bar 同一套：制图台方角主色框加底缘索引线，
        // 精细档圆角加两像素环，静谧档小圆角细环，平阶只描一条浅线。
        if draft(&th) {
            if hot || on {
                fill(hdc, &r, bgc);
                frame(hdc, &r, s.acc);
                fill(
                    hdc,
                    &RECT { left: r.left + 3, top: r.bottom - 2, right: r.right - 3, bottom: r.bottom },
                    s.acc,
                );
            }
        } else if fine(&th) {
            if hot || on {
                let rad = (6.0 * k).round() as i32;
                let keep = round_clip(hdc, &r, rad);
                fill(hdc, &r, bgc);
                unround(hdc, keep);
                // 两像素的框往里缩一像素再描，整条线落在按钮格内。
                let rr = RECT { left: r.left + 1, top: r.top + 1, right: r.right - 1, bottom: r.bottom - 1 };
                rring_w(hdc, &rr, rad, theme::mix(s.line_soft, s.acc, 0.34), 2);
            }
        } else if quiet(&th) {
            if hot || on {
                let rad = (4.0 * k).round() as i32;
                let keep = round_clip(hdc, &r, rad);
                fill(hdc, &r, bgc);
                unround(hdc, keep);
                rring(hdc, &r, rad, theme::mix(s.line_soft, s.acc, 0.22));
            }
        } else {
            fill(hdc, &r, bgc);
            if hot && !bands(&th) {
                frame(hdc, &r, s.line_soft);
            }
        }
        // 图标顶上、文字贴底，空档两边各让 3/8，中间剩的间隙是全空档的 1/4；
        // 整组在按钮里保持居中——和主窗口 below 排法同一份几何。
        let free = (r.bottom - r.top - lab_h - isz).max(0);
        let d = free / 8 * 3;
        let ir = RECT {
            left: r.left + (r.right - r.left - isz) / 2,
            top: r.top + d,
            right: r.left + (r.right - r.left + isz) / 2,
            bottom: r.top + d + isz,
        };
        icon(hdc, t.icon, &ir, bgc, iic);
        let lr = RECT {
            left: r.left + 1,
            top: r.bottom - lab_h - d - (2.0 * k).round() as i32,
            right: r.right - 1,
            bottom: r.bottom - d - (2.0 * k).round() as i32,
        };
        txt(hdc, &lr, t.label, ink, g.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
        if hot {
            g.hint = t.label.to_string();
        }
        g.tb_rows.push((t.id, r));
        tx = r.right + 1;
    }
    if !bands(&th) {
        hline(hdc, l.bar.left, l.bar.right, l.bar.bottom - 1, s.line);
    }
    if draft(&th) {
        // 图框双线：工具栏底缘之上再压一道细线，和主窗口、标题栏那道对上。
        hline(hdc, l.bar.left, l.bar.right, l.bar.bottom - 4, theme::mix(s.chrome, s.line_soft, 0.50));
    }

    // 画布
    fill(hdc, &l.canvas, s.canvas);
    let dc = SaveDC(hdc);
    let _ = IntersectClipRect(hdc, l.canvas.left, l.canvas.top, l.canvas.right, l.canvas.bottom);
    let ir = img_rect(&g, l.canvas);
    if g.iw > 1 || g.ih > 1 || g.img.as_raw().len() > 4 {
        let bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: g.iw as i32,
                biHeight: -(g.ih as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        // 整张图源原样铺到 ir 那一块，超出画布的部分交给上面设好的裁剪区切。
        // 别先裁源行号再按裁过的源高铺进画布：那样纵向等于"贴下边对齐"，
        // 图往下挪、画出来的内容反而往上走。
        if ir.right > l.canvas.left && ir.left < l.canvas.right
            && ir.bottom > l.canvas.top && ir.top < l.canvas.bottom
        {
            let _ = SetStretchBltMode(hdc, HALFTONE);
            let _ = StretchDIBits(
                hdc,
                ir.left,
                ir.top,
                ir.right - ir.left,
                ir.bottom - ir.top,
                0,
                0,
                g.iw as i32,
                g.ih as i32,
                Some(g.bgra.as_ptr() as *const core::ffi::c_void),
                &bi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
    }
    frame(hdc, &ir, s.line);

    // 裁切框
    if let Some((x0, y0, x1, y1)) = g.crop {
        let r = RECT { left: x0.min(x1), top: y0.min(y1), right: x0.max(x1), bottom: y0.max(y1) };
        // 框外压暗：原图还在，只是蒙暗一层，跟框内一比就分得出来
        let sh = theme::mix(s.canvas, 0x000000, 0.80);
        let parts = [
            RECT { left: l.canvas.left, top: l.canvas.top, right: l.canvas.right, bottom: r.top },
            RECT { left: l.canvas.left, top: r.bottom, right: l.canvas.right, bottom: l.canvas.bottom },
            RECT { left: l.canvas.left, top: r.top, right: r.left, bottom: r.bottom },
            RECT { left: r.right, top: r.top, right: l.canvas.right, bottom: r.bottom },
        ];
        for p in parts {
            if p.right > p.left && p.bottom > p.top {
                veil(hdc, &p, sh, 105);
            }
        }
        let p = CreatePen(PS_SOLID, 2, cref(s.acc));
        let ob = SelectObject(hdc, HGDIOBJ(p.0));
        let _ = SelectObject(hdc, GetStockObject(NULL_BRUSH));
        let _ = Rectangle(hdc, r.left, r.top, r.right, r.bottom);
        SelectObject(hdc, ob);
        let _ = DeleteObject(HGDIOBJ(p.0));
        for i in 1..3 {
            let x = r.left + (r.right - r.left) * i / 3;
            let y = r.top + (r.bottom - r.top) * i / 3;
            line(hdc, s.acc, 1, x, r.top, x, r.bottom);
            line(hdc, s.acc, 1, r.left, y, r.right, y);
        }
    }
    let _ = RestoreDC(hdc, dc);

    // 状态栏：装饰档跟主窗口一样走外壳工艺；极简档铺一块纯色，
    // 高度、内边距、字号（等宽数字）与主窗口一致。
    if bands(&th) {
        shell(hdc, &l.status, &th, s.chrome, false, true);
    } else {
        fill(hdc, &l.status, s.chrome);
        hline(hdc, l.status.left, l.status.right, l.status.top, s.line_soft);
    }
    let z = zoom_at(&g, l.canvas) * 100.0;
    let dim = format!("{} × {}  ·  {z:.0}%", g.iw, g.ih);
    let pad7 = (7.0 * k).round() as i32;
    let sw = text_w(hdc, &dim, g.font_num) + pad7 * 2;
    txt(hdc, &RECT { left: l.status.left + pad7, top: l.status.top, right: l.status.right - sw, bottom: l.status.bottom }, &g.status, s.ink_dim, g.font_num, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    txt(hdc, &RECT { left: l.status.right - sw, top: l.status.top, right: l.status.right - pad7, bottom: l.status.bottom }, &dim, s.ink_dim, g.font_num, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);

    // 菜单弹层：宽按内容、行高用主题的 row、热点用选中色，都跟主窗口弹层一致
    if let Some(idx) = g.menu_open {
        let rows = pop_geom(&g, &l, idx);
        if !rows.is_empty() {
            let pad2 = (2.0 * k) as i32;
            let pr = RECT {
                left: rows[0].1.left,
                top: rows[0].1.top - pad2,
                right: rows[0].1.right,
                bottom: rows[rows.len() - 1].1.bottom + pad2,
            };
            drop_shadow(hdc, &pr, k);
            fill(hdc, &pr, s.popup);
            frame(hdc, &pr, s.line);
            for (id, r) in &rows {
                let hot = over(r, g.pt.0, g.pt.1);
                if hot {
                    fill(hdc, r, s.sel);
                }
                let ink = if hot { s.sel_ink } else { s.ink };
                let item = MENUS[idx].1.iter().find(|m| m.id == *id);
                let (label, key) = item.map(|m| (m.label, m.key)).unwrap_or(("", ""));
                txt(hdc, &RECT { left: r.left + (18.0 * k) as i32, top: r.top, right: r.right - (16.0 * k) as i32, bottom: r.bottom }, label, ink, g.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                let kc = if hot {
                    theme::mix(s.sel, s.sel_ink, 0.8)
                } else if bands(&th) {
                    s.ink_off
                } else {
                    s.ink_dim
                };
                txt(hdc, &RECT { left: r.right - (16.0 * k) as i32 - (110.0 * k) as i32, top: r.top, right: r.right - (16.0 * k) as i32, bottom: r.bottom }, key, kc, g.font_sm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
            }
            g.pop_rows = rows;
        }
    }

    // 工具条下拉：点「旋转」「翻转」在这里选方向
    if let Some(id) = g.tb_open {
        let rows = tb_pop_geom(&g, &l, id);
        let items = tb_items(id);
        if !rows.is_empty() {
            let pad2 = (2.0 * k) as i32;
            let pr = RECT {
                left: rows[0].1.left,
                top: rows[0].1.top - pad2,
                right: rows[0].1.right,
                bottom: rows[rows.len() - 1].1.bottom + pad2,
            };
            drop_shadow(hdc, &pr, k);
            fill(hdc, &pr, s.popup);
            frame(hdc, &pr, s.line);
            for (rid, r) in &rows {
                let hot = over(r, g.pt.0, g.pt.1);
                if hot {
                    fill(hdc, r, s.sel);
                }
                let ink = if hot { s.sel_ink } else { s.ink };
                let label = items.iter().find(|m| m.id == *rid).map(|m| m.label).unwrap_or("");
                txt(hdc, &RECT { left: r.left + (18.0 * k) as i32, top: r.top, right: r.right - (16.0 * k) as i32, bottom: r.bottom }, label, ink, g.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            }
            g.pop_rows = rows;
        }
    }

    // 参数对话框
    g.dlg_rows.clear();
    if let Some((kind, dr, vals, focus, buf, keep)) = g.dlg.as_ref().map(|d| (d.kind, d.rect, d.vals, d.focus, d.buf.clone(), d.keep)) {
        drop_shadow(hdc, &dr, k);
        fill(hdc, &dr, s.chrome_hi);
        frame(hdc, &dr, s.line);
        // 浮层的标题条：底色仍是中性那两档，取色规则和 ✕ 那颗的画法走 theme::Strip，
        // 和主窗口标题栏右上角那颗同一套（以前这里是硬编码的一枚红底白叉）。
        let st = theme::Strip::float(&th);
        let tb = RECT { left: dr.left, top: dr.top, right: dr.right, bottom: dr.top + st.h };
        fill(hdc, &tb, st.bar);
        hline(hdc, tb.left, tb.right, tb.bottom - 1, st.edge);
        let txr = RECT { left: dr.left + (12.0 * k) as i32, top: tb.top, right: tb.right - (st.bw + st.pad + (6.0 * k) as i32), bottom: tb.bottom };
        txt(hdc, &txr, kind.title(), st.ink, g.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
        let ty = tb.top + (tb.bottom - tb.top - st.bh) / 2;
        let xr = RECT { left: tb.right - st.pad - st.bw, top: ty, right: tb.right - st.pad, bottom: ty + st.bh };
        let xhot = over(&xr, g.pt.0, g.pt.1);
        if xhot {
            if fine(&th) {
                let hr = RECT { left: xr.left + 2, top: xr.top + 2, right: xr.right - 2, bottom: xr.bottom - 2 };
                let rad = (4.0 * k).round() as i32;
                let keep = round_clip(hdc, &hr, rad);
                fill(hdc, &hr, st.hot);
                unround(hdc, keep);
                rring(hdc, &hr, rad, theme::mix(st.bar, st.fg, 0.45));
            } else {
                fill(hdc, &xr, st.hot);
                if !bands(&th) {
                    frame(hdc, &xr, st.ring);
                }
            }
        }
        if bands(&th) {
            xmark(hdc, (xr.left + xr.right) / 2, (xr.top + xr.bottom) / 2, (5.0 * k) as i32, if xhot { st.fg } else { st.dim });
        } else {
            glyph(hdc, g.font_sm, &xr, "✕", if xhot { st.fg } else { st.dim });
        }
        g.dlg_rows.push((D_CLOSE, xr));
        // 整条标题都登记成可拖（✕ 先登记的，命中轮得到它）
        g.dlg_rows.push((D_TITLEBAR, tb));

        if kind == PK::Exit {
            txt(hdc, &RECT { left: dr.left + (18.0 * k) as i32, top: tb.bottom + (14.0 * k) as i32, right: dr.right - (18.0 * k) as i32, bottom: tb.bottom + (60.0 * k) as i32 }, "图像已经修改，关闭前要保存吗？", s.ink, g.font, DT_LEFT | DT_WORDBREAK);
            let bw = (86.0 * k) as i32;
            let by = dr.bottom - (42.0 * k) as i32;
            dlg_btn(hdc, &mut g, s, RECT { left: dr.right - (16.0 * k) as i32 - bw * 3 - (16.0 * k) as i32, top: by, right: dr.right - (16.0 * k) as i32 - bw * 2 - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_SAVE, "保存", true);
            dlg_btn(hdc, &mut g, s, RECT { left: dr.right - (16.0 * k) as i32 - bw * 2 - (8.0 * k) as i32, top: by, right: dr.right - (16.0 * k) as i32 - bw - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_DISCARD, "不保存", false);
            dlg_btn(hdc, &mut g, s, RECT { left: dr.right - (16.0 * k) as i32 - bw, top: by, right: dr.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_STAY, "取消", false);
        } else if kind == PK::Resize {
            let mut y = tb.bottom + (12.0 * k) as i32;
            let field_w = (140.0 * k) as i32;
            for (i, label) in ["宽度(W)", "高度(H)"].iter().enumerate() {
                let lr = RECT { left: dr.left + (18.0 * k) as i32, top: y, right: dr.left + (128.0 * k) as i32, bottom: y + (26.0 * k) as i32 };
                txt(hdc, &lr, label, s.ink, g.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                let fr = RECT { left: dr.left + (134.0 * k) as i32, top: y, right: dr.left + (134.0 * k) as i32 + field_w, bottom: y + (26.0 * k) as i32 };
                fill(hdc, &fr, s.canvas);
                frame(hdc, &fr, if focus == i { s.acc } else { s.line });
                let shown = if focus == i { buf.clone() } else { format!("{:.0}", vals[i]) };
                txt(hdc, &fr, &shown, s.ink, g.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
                g.dlg_rows.push((if i == 0 { D_FW } else { D_FH }, fr));
                txt(hdc, &RECT { left: fr.right + (8.0 * k) as i32, top: y, right: fr.right + (50.0 * k) as i32, bottom: fr.bottom }, "像素", s.ink_dim, g.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                y += (34.0 * k) as i32;
            }
            let cb = RECT { left: dr.left + (18.0 * k) as i32, top: y + (4.0 * k) as i32, right: dr.left + (36.0 * k) as i32, bottom: y + (22.0 * k) as i32 };
            fill(hdc, &cb, s.canvas);
            frame(hdc, &cb, s.line);
            if keep {
                fill(hdc, &cb, s.acc);
                glyph(hdc, g.font_sm, &cb, "✓", s.ink_inv);
            }
            txt(hdc, &RECT { left: cb.right + (8.0 * k) as i32, top: y, right: dr.right - (18.0 * k) as i32, bottom: y + (26.0 * k) as i32 }, "保持纵横比(M)", s.ink, g.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            g.dlg_rows.push((D_KEEP, RECT { left: cb.left - (4.0 * k) as i32, top: y, right: dr.left + (140.0 * k) as i32, bottom: y + (26.0 * k) as i32 }));
            let by = dr.bottom - (40.0 * k) as i32;
            let bw = (74.0 * k) as i32;
            dlg_btn(hdc, &mut g, s, RECT { left: dr.right - (16.0 * k) as i32 - bw * 2 - (8.0 * k) as i32, top: by, right: dr.right - (16.0 * k) as i32 - bw - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_OK, "确定", true);
            dlg_btn(hdc, &mut g, s, RECT { left: dr.right - (16.0 * k) as i32 - bw, top: by, right: dr.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_CANCEL, "取消", false);
        } else {
            let labels = kind.labels();
            let ranges = kind.ranges();
            let mut y = tb.bottom + (12.0 * k) as i32;
            for (i, label) in labels.iter().enumerate() {
                let (lo, hi) = ranges.get(i).copied().unwrap_or((0.0, 100.0));
                let row = RECT { left: dr.left + (18.0 * k) as i32, top: y, right: dr.right - (18.0 * k) as i32, bottom: y + (26.0 * k) as i32 };
                let lr = RECT { left: row.left, top: row.top, right: row.left + (86.0 * k) as i32, bottom: row.bottom };
                txt(hdc, &lr, label, s.ink, g.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                let vr = RECT { left: row.right - (60.0 * k) as i32, top: row.top, right: row.right, bottom: row.bottom };
                let tr = RECT { left: lr.right + (10.0 * k) as i32, top: row.top, right: vr.left - (10.0 * k) as i32, bottom: row.bottom };
                dlg_slider(hdc, s, k, &tr, vals[i], lo, hi);
                // 命中登记轨道那一段的横向范围、整行高（不是 6 像素的细轨道）：
                // 横向必须和画出来的轨道一模一样，不然头的位置和他拖出来的值对不上。
                g.dlg_rows.push((D_SLIDER + i, RECT { left: tr.left, top: row.top, right: tr.right, bottom: row.bottom }));
                txt(hdc, &vr, &fmt_val(kind, i, vals[i]), s.ink, g.font_sm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
                y += (34.0 * k) as i32;
            }
            let by = dr.bottom - (40.0 * k) as i32;
            let bw = (74.0 * k) as i32;
            dlg_btn(hdc, &mut g, s, RECT { left: dr.right - (16.0 * k) as i32 - bw * 2 - (8.0 * k) as i32, top: by, right: dr.right - (16.0 * k) as i32 - bw - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_OK, "确定", true);
            dlg_btn(hdc, &mut g, s, RECT { left: dr.right - (16.0 * k) as i32 - bw, top: by, right: dr.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_CANCEL, "取消", false);
        }
    }
    let _ = hwnd;
}

/// 框里的一根滑块：一条细轨道 + 从左边铺到头的那一截 + 一枚压在轨道正中的小圆头。
/// 值按 lo..hi 归一化，头压在轨道两端就是那两个极值。
unsafe fn dlg_slider(hdc: HDC, s: theme::Skin, k: f64, r: &RECT, v: f64, lo: f64, hi: f64) {
    let mid = (r.top + r.bottom) / 2;
    let tt = (3.0 * k).round().max(2.0) as i32;
    let t = ((v - lo) / (hi - lo).max(1e-6)).clamp(0.0, 1.0);
    let pos = (r.left as f64 + t * (r.right - r.left - 1) as f64).round() as i32;
    let track = RECT { left: r.left, top: mid - tt / 2, right: r.right, bottom: mid - tt / 2 + tt };
    pill(hdc, &track, tt / 2, s.line_soft);
    pill(
        hdc,
        &RECT { left: track.left, top: track.top, right: pos.max(track.left + 1), bottom: track.bottom },
        tt / 2,
        s.acc,
    );
    let kr = (5.0 * k).round() as i32;
    let hr = RECT { left: pos - kr, top: mid - kr, right: pos + kr + 1, bottom: mid + kr + 1 };
    pill(hdc, &hr, kr, s.chrome_hi);
    rring(hdc, &hr, kr, s.acc);
}

/// 一颗圆角块：半径取到高度一半就是一枚胶囊，取到半边长就是一枚圆。
unsafe fn pill(hdc: HDC, r: &RECT, rad: i32, c: u32) {
    let keep = round_clip(hdc, r, rad.max(1));
    fill(hdc, r, c);
    unround(hdc, keep);
}

unsafe fn dlg_btn(hdc: HDC, g: &mut Ed, s: theme::Skin, r: RECT, id: usize, label: &str, primary: bool) {
    let hot = over(&r, g.pt.0, g.pt.1);
    fill(hdc, &r, if primary { s.acc } else if hot { s.hov } else { s.chrome });
    frame(hdc, &r, if primary { s.acc } else { s.line });
    txt(hdc, &r, label, if primary { s.ink_inv } else { s.ink }, g.font, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    g.dlg_rows.push((id, r));
}

unsafe fn on_mouse_down(hwnd: HWND, x: i32, y: i32) {
    let mut g = ed().lock().unwrap();
    g.pt = (x, y);
    // 对话框优先
    if g.dlg.is_some() {
        let hit = g.dlg_rows.iter().find(|(_, r)| over(r, x, y)).map(|(id, _)| *id);
        if let Some(id) = hit {
            // 保存/放弃要先放锁：保存可能弹模态的文件对话框
            if id == D_SAVE {
                close_dlg(&mut g);
                drop(g);
                if save_entry(hwnd, false) {
                    let _ = DestroyWindow(hwnd);
                } else {
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                return;
            }
            if id == D_DISCARD {
                close_dlg(&mut g);
                drop(g);
                let _ = DestroyWindow(hwnd);
                return;
            }
            if id == D_TITLEBAR {
                // 按住标题条：整扇框跟着手走
                if let Some(d) = g.dlg.as_ref() {
                    g.dlg_drag = Some((x - d.rect.left, y - d.rect.top));
                }
                let _ = SetCapture(hwnd);
                let _ = InvalidateRect(Some(hwnd), None, false);
                return;
            }
            if (D_SLIDER..=D_SLIDER + 2).contains(&id) {
                g.row_drag = Some(id - D_SLIDER);
                let _ = SetCapture(hwnd);
            }
            handle_dlg(hwnd, &mut g, id);
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
        return;
    }
    // 工具条下拉（旋转/翻转选方向）
    if g.tb_open.is_some() {
        let hit = g.pop_rows.iter().find(|(_, r)| over(r, x, y)).map(|(id, _)| *id);
        if let Some(id) = hit {
            g.tb_open = None;
            drop(g);
            do_command(hwnd, id);
            let _ = InvalidateRect(Some(hwnd), None, false);
            return;
        }
        g.tb_open = None;
        // 点在旋转/翻转按钮上就是收起，别再走下面的打开分支
        if let Some((id, _)) = g.tb_rows.iter().find(|(_, r)| over(r, x, y)).cloned() {
            if id == M_ROT90 || id == M_FLIPH {
                let _ = InvalidateRect(Some(hwnd), None, false);
                return;
            }
        }
    }
    // 菜单弹层
    if let Some(_) = g.menu_open {
        let hit = g.pop_rows.iter().find(|(_, r)| over(r, x, y)).map(|(id, _)| *id);
        if let Some(id) = hit {
            g.menu_open = None;
            drop(g);
            do_command(hwnd, id);
            let _ = InvalidateRect(Some(hwnd), None, false);
            return;
        }
        // 点在菜单条上就换一栏
        if let Some((i, _)) = g.mb_rows.iter().find(|(_, r)| over(r, x, y)) {
            g.menu_open = Some(*i);
            let _ = InvalidateRect(Some(hwnd), None, false);
            return;
        }
        g.menu_open = None;
    }
    // 标题栏按钮
    if let Some((id, _)) = g.title_rows.iter().find(|(_, r)| over(r, x, y)).cloned() {
        match id {
            T_MIN => {
                // ShowWindow 同步回调 WM_GETMINMAXINFO / WM_NCHITTEST，它们也要这把锁，持锁调用必卡死
                drop(g);
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
            T_MAX => {
                drop(g);
                let _ = ShowWindow(hwnd, if IsZoomed(hwnd).as_bool() { SW_RESTORE } else { SW_MAXIMIZE });
            }
            T_CLOSE => {
                drop(g);
                request_exit(hwnd);
                return;
            }
            _ => {}
        }
        return;
    }
    // 菜单条
    if let Some((i, _)) = g.mb_rows.iter().find(|(_, r)| over(r, x, y)).cloned() {
        let same = g.menu_open == Some(i);
        g.tb_open = None;
        g.menu_open = if same { None } else { Some(i) };
        if !same {
            let l = lay(client(hwnd), &g.th);
            g.pop_rows = pop_geom(&g, &l, i);
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
        return;
    }
    // 工具条
    if let Some((id, _)) = g.tb_rows.iter().find(|(_, r)| over(r, x, y)).cloned() {
        if id != 0 {
            if id == M_ROT90 || id == M_FLIPH {
                g.tb_open = Some(id);
                g.menu_open = None;
                let l = lay(client(hwnd), &g.th);
                g.pop_rows = tb_pop_geom(&g, &l, id);
                let _ = InvalidateRect(Some(hwnd), None, false);
            } else {
                g.tb_open = None;
                drop(g);
                do_command(hwnd, id);
            }
        }
        return;
    }
    // 画布
    let l = lay(client(hwnd), &g.th);
    if over(&l.canvas, x, y) {
        if g.crop_mode {
            // 只记起点，先不动已有的框：双击确认的前一下也走这里，
            // 当场抹成零点的话后面那下双击就再也找不到框了。
            g.crop_seed = Some((x, y));
        } else {
            g.drag_pan = Some((x, y, g.pan.0, g.pan.1));
            SetCapture(hwnd);
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

/// 平移能挪到哪儿：图比画布大，挪到"图边贴住画布边"为止；图比画布小就锁在正中。
/// 拖动、滚轮、方向键三条路都走这一句，不然它们各自停在不同的边界上，
/// 下一次动鼠标会被硬拉回去，看着就是图自己反向弹一下。
fn clamp_pan(g: &mut Ed, canvas: RECT) {
    let z = zoom_at(g, canvas);
    let iw = (g.iw as f64 * z) as i32;
    let ih = (g.ih as f64 * z) as i32;
    let cw = canvas.right - canvas.left;
    let ch = canvas.bottom - canvas.top;
    g.pan.0 = g.pan.0.clamp(-((iw - cw) / 2).max(0), ((iw - cw) / 2).max(0));
    g.pan.1 = g.pan.1.clamp(-((ih - ch) / 2).max(0), ((ih - ch) / 2).max(0));
}

fn sync_resize(d: &mut Dlg, edited: usize) {
    if d.kind != PK::Resize || !d.keep {
        return;
    }
    if edited == 0 {
        d.vals[1] = (d.vals[0] / d.aspect).round().clamp(1.0, 20000.0);
    } else {
        d.vals[0] = (d.vals[1] * d.aspect).round().clamp(1.0, 20000.0);
    }
}

/// 第 i 根滑块此刻鼠标压在轨道的哪一档上，折成值（吸附到这一行的步长，两头夹进值域）。
fn slider_at(g: &Ed, i: usize) -> Option<f64> {
    let d = g.dlg.as_ref()?;
    let (lo, hi) = d.kind.ranges().get(i).copied()?;
    let st = d.kind.steps().get(i).copied().unwrap_or(1.0);
    let r = g.dlg_rows.iter().find(|(id, _)| *id == D_SLIDER + i).map(|(_, r)| *r)?;
    let t = ((g.pt.0 - r.left) as f64 / (r.right - r.left).max(1) as f64).clamp(0.0, 1.0);
    let raw = lo + t * (hi - lo);
    Some(((raw / st).round() * st).clamp(lo.min(hi), lo.max(hi)))
}

/// 把第 i 行按鼠标位置落值。逐像素那几页顺手把效果算到屏上给他看，重活留到松手那一下。
unsafe fn slide_to(g: &mut Ed, i: usize) {
    let live = g.dlg.as_ref().map(|d| d.kind.live()).unwrap_or(false);
    let Some(v) = slider_at(g, i) else { return };
    {
        let Some(d) = g.dlg.as_mut() else { return };
        if (d.vals[i] - v).abs() < 1e-9 {
            return;
        }
        d.vals[i] = v;
        d.focus = usize::MAX;
    }
    if live {
        preview(g);
    }
}

unsafe fn handle_dlg(hwnd: HWND, g: &mut Ed, id: usize) {
    match id {
        D_CLOSE | D_CANCEL | D_STAY => {
            close_dlg(g);
        }
        D_OK => {
            g.dlg_drag = None;
            g.row_drag = None;
            if let Some(d) = g.dlg.take() {
                commit_dlg(g, &d);
            }
        }
        id if (D_SLIDER..=D_SLIDER + 2).contains(&id) => {
            slide_to(g, id - D_SLIDER);
        }
        D_FW | D_FH => {
            if let Some(d) = g.dlg.as_mut() {
                let i = if id == D_FW { 0 } else { 1 };
                d.focus = i;
                d.buf = format!("{:.0}", d.vals[i]);
            }
        }
        D_KEEP => {
            if let Some(d) = g.dlg.as_mut() {
                d.keep = !d.keep;
                if d.keep {
                    d.vals[1] = (d.vals[0] / d.aspect).round().clamp(1.0, 20000.0);
                }
                d.focus = usize::MAX;
            }
        }
        _ => {}
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn on_mouse_move(hwnd: HWND, x: i32, y: i32) {
    let mut g = ed().lock().unwrap();
    let old = g.pt;
    g.pt = (x, y);
    let mut dirty = old != g.pt;
    if let Some((sx, sy, px, py)) = g.drag_pan {
        g.pan = (px + x - sx, py + y - sy);
        let l = lay(client(hwnd), &g.th);
        clamp_pan(&mut g, l.canvas);
        dirty = true;
    }
    if let Some((ox, oy)) = g.dlg_drag {
        let rc = client(hwnd);
        let keep = (90.0 * g.scale) as i32;
        if let Some(d) = g.dlg.as_mut() {
            let (w, h) = (d.rect.right - d.rect.left, d.rect.bottom - d.rect.top);
            // 至少留一小截在客户区里，别一路拖到找不回来
            let nx = (x - ox).clamp(rc.left - w + keep, rc.right - keep);
            let ny = (y - oy).clamp(rc.top, rc.bottom - keep);
            d.rect = RECT { left: nx, top: ny, right: nx + w, bottom: ny + h };
        }
        dirty = true;
    }
    if let Some(i) = g.row_drag {
        slide_to(&mut g, i);
        dirty = true;
    }
    if let Some((sx, sy)) = g.crop_seed {
        g.crop = Some((sx, sy, x, y));
        dirty = true;
    }
    // 悬停变化也重画
    let hot = g
        .tb_rows
        .iter()
        .chain(g.mb_rows.iter())
        .chain(g.title_rows.iter())
        .chain(g.pop_rows.iter())
        .any(|(_, r)| over(r, x, y));
    if hot {
        dirty = true;
    }
    if dirty {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

unsafe fn on_mouse_up(hwnd: HWND, x: i32, y: i32) {
    let mut g = ed().lock().unwrap();
    if g.drag_pan.take().is_some() {
        let _ = ReleaseCapture();
    }
    if g.dlg_drag.take().is_some() {
        let _ = ReleaseCapture();
    }
    if g.row_drag.take().is_some() {
        let _ = ReleaseCapture();
        // 模糊/锐化/旋转这几页拖的一路不算，松手这一下算一次给他看
        let live = g.dlg.as_ref().map(|d| d.kind.live()).unwrap_or(true);
        if !live {
            preview(&mut g);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
    if let Some((sx, sy)) = g.crop_seed.take() {
        // 拖出尺寸才算新的一框；没动的这一下（双击的前一半）保留原框
        if (x - sx).abs() > 2 || (y - sy).abs() > 2 {
            g.crop = Some((sx, sy, x, y));
            g.status = "按 Enter 或双击框内应用裁切，Esc 取消".to_string();
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
}

/// 双击框内 = 应用裁切；框外或没框时跟普通按下一样，重新起一框。
unsafe fn on_mouse_double(hwnd: HWND, x: i32, y: i32) {
    let mut g = ed().lock().unwrap();
    let l = lay(client(hwnd), &g.th);
    let inside = g.crop.map(|(x0, y0, x1, y1)| {
        over(
            &RECT { left: x0.min(x1), top: y0.min(y1), right: x0.max(x1), bottom: y0.max(y1) },
            x,
            y,
        )
    });
    if g.crop_mode && over(&l.canvas, x, y) && inside == Some(true) {
        apply_crop(&mut g);
        let _ = InvalidateRect(Some(hwnd), None, false);
        return;
    }
    drop(g);
    on_mouse_down(hwnd, x, y);
}

unsafe fn on_char(hwnd: HWND, ch: u32) {
    let mut g = ed().lock().unwrap();
    let editing = g.dlg.as_ref().map(|d| d.kind == PK::Resize && d.focus <= 1).unwrap_or(false);
    if !editing {
        return;
    }
    if ch == 0x0D {
        if let Some(d) = g.dlg.take() {
            commit_dlg(&mut g, &d);
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
        return;
    }
    let mut changed = false;
    if let Some(d) = g.dlg.as_mut() {
        match ch {
            0x08 => {
                d.buf.pop();
                changed = true;
            }
            c if (c as u8 as char).is_ascii_digit() => {
                if d.buf.len() < 5 {
                    d.buf.push(c as u8 as char);
                    changed = true;
                }
            }
            _ => {}
        }
        if changed && !d.buf.is_empty() {
            let raw = d.buf.parse::<f64>().unwrap_or(d.vals[d.focus]);
            let (i, n) = (d.focus, raw.clamp(1.0, 20000.0));
            d.vals[i] = n;
            if (raw - n).abs() > 0.5 {
                d.buf = format!("{n:.0}");
            }
            sync_resize(d, i);
        }
    }
    if changed {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

unsafe fn on_key(hwnd: HWND, vk: u32, ctrl: bool, shift: bool) {
    // 参数框开着时只认 Esc/Enter：这时 Ctrl+O 会换文件而 dlg_base 还指着旧图，
    // Esc 一还原就把旧图挂在新路径上；Ctrl+S 会把没提交的预览态写盘。
    let dlg_open = ed().lock().unwrap().dlg.is_some();
    if dlg_open && !matches!(vk, 0x1B | 0x0D) {
        return;
    }
    match (vk, ctrl, shift) {
        (0x53, true, false) => do_command(hwnd, M_SAVE),
        (0x53, true, true) => do_command(hwnd, M_SAVEAS),
        (0x4F, true, _) => do_command(hwnd, M_OPEN),
        (0x5A, true, _) => do_command(hwnd, M_UNDO),
        (0x59, true, _) => do_command(hwnd, M_REDO),
        (0x30, true, _) => do_command(hwnd, M_FIT),
        (0x31, true, _) => do_command(hwnd, M_ACTUAL),
        (0xBB, true, _) => do_command(hwnd, M_ZOOMIN),
        (0xBD, true, _) => do_command(hwnd, M_ZOOMOUT),
        (0x1B, _, _) => {
            let mut g = ed().lock().unwrap();
            if g.dlg.is_some() {
                close_dlg(&mut g);
            } else if g.menu_open.is_some() {
                g.menu_open = None;
            } else if g.tb_open.is_some() {
                g.tb_open = None;
            } else if g.crop_mode || g.crop.is_some() {
                g.crop_mode = false;
                g.crop = None;
                g.status = "已取消裁切".to_string();
            }
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        (0x0D, _, _) => {
            let mut g = ed().lock().unwrap();
            // 参数框开着时 Enter 不往下落：裁切状态是另一套，别隔着对话框应用裁切。
            if g.dlg.is_some() {
                return;
            }
            let has = g.crop.is_some();
            if has {
                apply_crop(&mut g);
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
        }
        (0x25..=0x28, _, _) => {
            let mut g = ed().lock().unwrap();
            // 对话框开着时方向键不动背景，免得滑块还没拖明白图先跑了
            if g.dlg.is_some() {
                return;
            }
            let step = 48;
            // 符号跟拖动手势一致：按左 = 把图往左推。以前这几条反着写，
            // 于是"按住左键往左拖"和"按左键"两个方向打架。
            match vk {
                0x25 => g.pan.0 -= step,
                0x27 => g.pan.0 += step,
                0x26 => g.pan.1 -= step,
                _ => g.pan.1 += step,
            }
            let l = lay(client(hwnd), &g.th);
            clamp_pan(&mut g, l.canvas);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        _ => {}
    }
}

unsafe fn on_wheel(hwnd: HWND, x: i32, y: i32, delta: i32) {
    let mut g = ed().lock().unwrap();
    // 光标压在浮着的对话框上时，滚轮不该去动背后的图
    if g.dlg.as_ref().map(|d| over(&d.rect, x, y)).unwrap_or(false) {
        return;
    }
    let l = lay(client(hwnd), &g.th);
    if !over(&l.canvas, x, y) {
        return;
    }
    let z = zoom_at(&g, l.canvas);
    let nz = (z * if delta > 0 { 1.25 } else { 0.8 }).clamp(0.02, 32.0);
    let ir = img_rect(&g, l.canvas);
    let ix = (x - ir.left) as f64 / z;
    let iy = (y - ir.top) as f64 / z;
    g.fit = false;
    g.zoom = nz;
    let w = (g.iw as f64 * nz).round() as i32;
    let h = (g.ih as f64 * nz).round() as i32;
    let centered_x = l.canvas.left + (l.canvas.right - l.canvas.left - w) / 2;
    let centered_y = l.canvas.top + (l.canvas.bottom - l.canvas.top - h) / 2;
    g.pan.0 = x - (ix * nz).round() as i32 - centered_x;
    g.pan.1 = y - (iy * nz).round() as i32 - centered_y;
    clamp_pan(&mut g, l.canvas);
    let _ = InvalidateRect(Some(hwnd), None, false);
}

fn title_hit(g: &Ed, rc: RECT, x: i32, y: i32) -> bool {
    // 和画出来的那条一样高（以前写死 32，绘制走 th.title，两者会错开一截）
    y >= rc.top && y < rc.top + g.th.title
}

unsafe fn hit_test(g: &Ed, hwnd: HWND, sx: i32, sy: i32) -> LRESULT {
    let mut rc = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rc);
    let x = sx - rc.left;
    let y = sy - rc.top;
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
    let edge = (6.0 * g.scale) as i32;
    if IsZoomed(hwnd).as_bool() {
        return LRESULT(HTCLIENT as isize);
    }
    if x < edge && y < edge {
        return LRESULT(HTTOPLEFT as isize);
    }
    if x >= w - edge && y < edge {
        return LRESULT(HTTOPRIGHT as isize);
    }
    if x < edge && y >= h - edge {
        return LRESULT(HTBOTTOMLEFT as isize);
    }
    if x >= w - edge && y >= h - edge {
        return LRESULT(HTBOTTOMRIGHT as isize);
    }
    if x < edge {
        return LRESULT(HTLEFT as isize);
    }
    if x >= w - edge {
        return LRESULT(HTRIGHT as isize);
    }
    if y < edge {
        return LRESULT(HTTOP as isize);
    }
    if y >= h - edge {
        return LRESULT(HTBOTTOM as isize);
    }
    if title_hit(g, client(hwnd), x, y) {
        // 标题栏按钮让客户区自己处理
        if g.title_rows.iter().any(|(_, r)| over(r, x, y)) {
            return LRESULT(HTCLIENT as isize);
        }
        return LRESULT(HTCAPTION as isize);
    }
    let _ = (sx, sy);
    LRESULT(HTCLIENT as isize)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let dpi = GetDpiForWindow(hwnd);
            let k = dpi as f64 / 96.0;
            let mut g = ed().lock().unwrap();
            g.hwnd = hwnd;
            g.scale = k;
            g.th = Theme::new_with(&g.skin, &g.density, k, &g.look);
            let face = g.th.face.clone();
            g.font = mk_font(g.th.font, false, &face);
            g.font_sm = mk_font(g.th.font_sm, false, &face);
            g.font_b = mk_font(g.th.font, true, &face);
            g.font_num = mk_font(g.th.font_sm, false, "Consolas");
            drop(g);
            update_title(&ed().lock().unwrap());
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let dpi = ((wp.0 >> 16) & 0xffff) as u32;
            let k = dpi as f64 / 96.0;
            let mut g = ed().lock().unwrap();
            g.scale = k;
            g.th = Theme::new_with(&g.skin, &g.density, k, &g.look);
            // 重建前先把旧的四支字体还回去：GDI 句柄有一万的上限，多屏来回拖会攒。
            for h in [g.font, g.font_sm, g.font_b, g.font_num] {
                if !h.is_invalid() {
                    let _ = DeleteObject(HGDIOBJ(h.0));
                }
            }
            let face = g.th.face.clone();
            g.font = mk_font(g.th.font, false, &face);
            g.font_sm = mk_font(g.th.font_sm, false, &face);
            g.font_b = mk_font(g.th.font, true, &face);
            g.font_num = mk_font(g.th.font_sm, false, "Consolas");
            drop(g);
            let pr = &*(lp.0 as *const RECT);
            let _ = SetWindowPos(hwnd, None, pr.left, pr.top, pr.right - pr.left, pr.bottom - pr.top, SWP_NOZORDER | SWP_NOACTIVATE);
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let rc = client(hwnd);
            let mem = CreateCompatibleDC(Some(hdc));
            let bmp = CreateCompatibleBitmap(hdc, (rc.right - rc.left).max(1), (rc.bottom - rc.top).max(1));
            let old = SelectObject(mem, HGDIOBJ(bmp.0));
            paint(mem, hwnd);
            let _ = BitBlt(hdc, 0, 0, rc.right, rc.bottom, Some(mem), 0, 0, SRCCOPY);
            SelectObject(mem, old);
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            let _ = DeleteDC(mem);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        // 与主窗口一致：WS_THICKFRAME 默认会被系统描一圈非客户区边框，客户区已铺满，别画
        WM_NCPAINT => LRESULT(0),
        WM_NCACTIVATE => LRESULT(1),
        // 客户区盖满整个窗口；最大化时收进工作区，别压任务栏
        WM_NCCALCSIZE => {
            if wp.0 != 0 && IsZoomed(hwnd).as_bool() {
                let p = lp.0 as *mut NCCALCSIZE_PARAMS;
                let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi = MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(mon, &mut mi).as_bool() {
                    let rc = &mut (*p).rgrc[0];
                    rc.left = rc.left.max(mi.rcWork.left);
                    rc.top = rc.top.max(mi.rcWork.top);
                    rc.right = rc.right.min(mi.rcWork.right);
                    rc.bottom = rc.bottom.min(mi.rcWork.bottom);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let x = (lp.0 & 0xffff) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xffff) as i16 as i32;
            on_mouse_down(hwnd, x, y);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let x = (lp.0 & 0xffff) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xffff) as i16 as i32;
            on_mouse_up(hwnd, x, y);
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            let x = (lp.0 & 0xffff) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xffff) as i16 as i32;
            on_mouse_double(hwnd, x, y);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let x = (lp.0 & 0xffff) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xffff) as i16 as i32;
            on_mouse_move(hwnd, x, y);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 >> 16) & 0xffff) as i16 as i32;
            let pt = POINT { x: (lp.0 & 0xffff) as i16 as i32, y: ((lp.0 >> 16) & 0xffff) as i16 as i32 };
            let mut c = pt;
            let _ = ScreenToClient(hwnd, &mut c);
            on_wheel(hwnd, c.x, c.y, delta);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let vk = wp.0 as u32;
            let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
            let shift = GetKeyState(VK_SHIFT.0 as i32) < 0;
            on_key(hwnd, vk, ctrl, shift);
            LRESULT(0)
        }
        WM_CHAR => {
            on_char(hwnd, wp.0 as u32);
            LRESULT(0)
        }
        WM_COMMAND => {
            if (wp.0 >> 16) & 0xffff == 0 {
                do_command(hwnd, wp.0 & 0xffff);
            }
            LRESULT(0)
        }
        WM_NCHITTEST => {
            let x = (lp.0 & 0xffff) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xffff) as i16 as i32;
            let g = ed().lock().unwrap();
            hit_test(&g, hwnd, x, y)
        }
        WM_SETCURSOR => {
            let g = ed().lock().unwrap();
            let crop = g.crop_mode && over(&lay(client(hwnd), &g.th).canvas, g.pt.0, g.pt.1);
            drop(g);
            if crop {
                if let Ok(c) = LoadCursorW(None, IDC_CROSS) {
                    let _ = SetCursor(Some(c));
                    return LRESULT(1);
                }
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_GETMINMAXINFO => {
            let mm = &mut *(lp.0 as *mut MINMAXINFO);
            let k = ed().lock().unwrap().scale;
            mm.ptMinTrackSize = POINT { x: (680.0 * k) as i32, y: (480.0 * k) as i32 };
            let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(mon, &mut mi).as_bool() {
                let (wr, mr) = (mi.rcWork, mi.rcMonitor);
                mm.ptMaxPosition.x = wr.left - mr.left;
                mm.ptMaxPosition.y = wr.top - mr.top;
                mm.ptMaxSize.x = wr.right - wr.left;
                mm.ptMaxSize.y = wr.bottom - wr.top;
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            request_exit(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn arg_val(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).cloned()
}

fn main() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.iter().find(|a| !a.starts_with("--") && !a.starts_with('-')).cloned();
    let skin = arg_val(&args, "--skin").unwrap_or_else(theme::default_name);
    let density = arg_val(&args, "--density").unwrap_or_else(|| "normal".into());
    let notify = arg_val(&args, "--notify").and_then(|s| s.parse::<isize>().ok());
    {
        let mut g = ed().lock().unwrap();
        g.skin = skin;
        g.density = density;
        g.look = Look::load(&g.skin);
        g.notify = notify;
        if let Some(p) = &path {
            match unsafe { load_file(&mut g, Path::new(p)) } {
                Ok(()) => g.status = format!("{} × {} · {}", g.iw, g.ih, stem(&g.path)),
                Err(e) => g.status = e,
            }
        } else {
            g.status = "文件 → 打开 选择一张图片".to_string();
        }
    }
    unsafe {
        let hinst = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let arrow = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let cls = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            hCursor: arrow,
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszClassName: PCWSTR(pw(CLASS).as_ptr()),
            ..Default::default()
        };
        if RegisterClassExW(&cls) == 0 {
            std::process::exit(1);
        }
        // 落在调它的那扇窗所在屏幕的正中。WS_POPUP 不认 CW_USEDEFAULT 的层叠定位，
        // 不自己算就会甩到屏幕外去。
        let mut anchor = notify
            .map(|h| HWND(h as *mut core::ffi::c_void))
            .unwrap_or_default();
        // 调用方给的那扇窗可能已经关了（或压根没经浏览器启动），句柄不作数就退回当前前台窗。
        if !IsWindow(Some(anchor)).as_bool() {
            anchor = GetForegroundWindow();
        }
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let mon = MonitorFromWindow(anchor, MONITOR_DEFAULTTONEAREST);
        if !GetMonitorInfoW(mon, &mut mi).as_bool() {
            let mut wa = RECT::default();
            let _ = SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut wa as *mut _ as *mut core::ffi::c_void),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
            mi.rcWork = wa;
        }
        let (mut ww, mut wh, mut wx, mut wy) = (1180i32, 820i32, 0i32, 0i32);
        let rc = mi.rcWork;
        let (aw, ah) = (rc.right - rc.left, rc.bottom - rc.top);
        if aw > 0 && ah > 0 {
            ww = ww.min(aw);
            wh = wh.min(ah);
            wx = rc.left + (aw - ww) / 2;
            wy = rc.top + (ah - wh) / 2;
        }
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(pw(CLASS).as_ptr()),
            PCWSTR(pw(TITLE).as_ptr()),
            WS_POPUP | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU | WS_CLIPCHILDREN,
            wx,
            wy,
            ww,
            wh,
            None,
            None,
            Some(hinst),
            None,
        );
        let Ok(hwnd) = hwnd else {
            std::process::exit(1);
        };
        let dark = windows::core::BOOL::from(true);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const _ as *const core::ffi::c_void, std::mem::size_of::<windows::core::BOOL>() as u32);
        let nc = DWMNCRP_DISABLED;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_NCRENDERING_POLICY, &nc as *const _ as *const core::ffi::c_void, std::mem::size_of::<DWMNCRENDERINGPOLICY>() as u32);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);
        let mut msg = MSG::default();
        loop {
            let r = GetMessageW(&mut msg, None, 0, 0);
            if !r.as_bool() {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#![allow(unsafe_op_in_unsafe_fn)]
/// 大图查看态。它不是另一个窗口，而是浏览窗口换了一张脸：
/// 两行工具条 + 画布 + 底部状态栏这一套结构。
/// 界面怎么画、鼠标点了哪，全在这里；换目录、删文件、开设置这些动作交回宿主去做。
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::{
    job, scan,
    theme::{cref, Theme},
    View,
};
use windows::core::{PCWSTR, HSTRING};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

/// 后台解码好了往宿主窗口发这两条。
pub const WM_VCLEAR: u32 = WM_APP + 21;
pub const WM_VQUICK: u32 = WM_APP + 22;

/// 查看器做不了、要宿主去做的事。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Open,
    Explore,
    Edit,
    MoveTo,
    CopyTo,
    Delete,
    Prop,
    Up,
    Look,
    List,
    Detail,
    Slide,
    Browse,
}

/// 一次交互的结论：没变化 / 回浏览态 / 切全屏 / 让宿主办事。
#[derive(PartialEq, Eq, Debug)]
pub enum Act {
    Nothing,
    Leave,
    Full,
    Do(Cmd),
}

/// 工具条按钮的动作种类。能自己办的自己办，办不了的变成 Cmd。
#[derive(Clone, Copy, PartialEq, Eq)]
enum K {
    Prev,
    Next,
    ZoomOut,
    ZoomIn,
    Fit,
    Actual,
    Thumb,
    Big,
    Refresh,
    C(Cmd),
}

struct Btn {
    label: &'static str,
    row: u32,
    k: K,
    sep: bool,
}

use Cmd as C;
use K::C as KC;

/// 第一行是查看器主工具条，第二行是"上一级 + 视图方式 + 工具"。
const BTNS: &[Btn] = &[
    Btn { label: "浏览", row: 0, k: KC(C::Browse), sep: false },
    Btn { label: "打开", row: 0, k: KC(C::Open), sep: false },
    Btn { label: "上一个", row: 0, k: K::Prev, sep: true },
    Btn { label: "下一个", row: 0, k: K::Next, sep: false },
    Btn { label: "幻灯显示", row: 0, k: KC(C::Slide), sep: true },
    Btn { label: "缩小", row: 0, k: K::ZoomOut, sep: true },
    Btn { label: "放大", row: 0, k: K::ZoomIn, sep: false },
    Btn { label: "编辑", row: 0, k: KC(C::Edit), sep: true },
    Btn { label: "移到", row: 0, k: KC(C::MoveTo), sep: false },
    Btn { label: "复制到", row: 0, k: KC(C::CopyTo), sep: false },
    Btn { label: "删除", row: 0, k: KC(C::Delete), sep: false },
    Btn { label: "属性", row: 0, k: KC(C::Prop), sep: true },
    Btn { label: "上一级", row: 1, k: KC(C::Up), sep: false },
    Btn { label: "缩略图", row: 1, k: K::Thumb, sep: true },
    Btn { label: "大图", row: 1, k: K::Big, sep: false },
    Btn { label: "列表", row: 1, k: KC(C::List), sep: false },
    Btn { label: "详情", row: 1, k: KC(C::Detail), sep: false },
    Btn { label: "刷新", row: 1, k: K::Refresh, sep: true },
    Btn { label: "看整夹", row: 1, k: KC(C::Look), sep: false },
    Btn { label: "定位", row: 1, k: KC(C::Explore), sep: false },
];

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Fit,
    Actual,
}

struct Cur {
    idx: usize,
    view: View,
    files: Vec<String>,
    mode: Mode,
    /// 整图倍率：相对原图宽高的倍数。刻意不跟"这次解码解出多大"挂钩，
    /// 否则换一次画布就换一次解码尺寸，图会凭空变大变小。
    scale: f64,
    panx: i32,
    pany: i32,
    /// 左键拖出来的选框，客户区坐标：起点 + 当前终点。
    sel: Option<(i32, i32, i32, i32)>,
    /// 左键是不是还按着。松手之后鼠标移动不能再拖着选框跑。
    drag: bool,
    /// 放大态按住左键平移：起点、按下时的 panx/pany。
    pan: Option<(i32, i32, i32, i32)>,
    rc: RECT,
    /// 进全屏前那一屏的视图：模式 + 倍率 + 平移。退出全屏原样还回去。
    saved: Option<(Mode, f64, i32, i32)>,
    /// 画完缓存下来的按钮矩形，鼠标命中直接查。
    btns: Vec<(usize, RECT)>,
    hot: usize,
    ms: f64,
    /// 文件字节数，状态栏用。换图时取一次，不在绘制里反复 stat。
    bytes: u64,
    /// 全屏时鼠标指着哪边的翻页箭头：-1 左、1 右、0 没指着。大图模式恒为 0。
    nav: i8,
    /// 全屏箭头这一会儿是不是画出来。离开箭头一秒后转假。
    nav_show: bool,
    /// 手上这张 view 是不是全尺寸解出来的。快速帧不能把清晰图盖回去。
    src_full: bool,
    /// 手上这张全尺寸 view 当初是按哪个盒子解的。同图同盒再次 show() 时
    /// 解码线程直接跳过，缩放键/F↔1/改窗口尺寸不再每次都重解一张大图。
    full_box: (u32, u32),
}

struct Zones {
    bar0: RECT,
    bar1: RECT,
    status: RECT,
    area: RECT,
}

/// 宿主窗口。查看器不再自己 CreateWindow，只往这个句柄发重绘。
static HOST: AtomicIsize = AtomicIsize::new(0);
static ON: AtomicBool = AtomicBool::new(false);
/// 全屏时宿主把自绘标题栏收掉了，查看器得跟着把工具栏顶到窗口边。
static TITLED: AtomicBool = AtomicBool::new(true);
static QUICK: Mutex<Option<(usize, View)>> = Mutex::new(None);
static APP: OnceLock<Mutex<Option<Cur>>> = OnceLock::new();
static CACHE: OnceLock<Mutex<Option<(usize, u32, u32, View, f64)>>> = OnceLock::new();
static THEME: OnceLock<Mutex<Option<Theme>>> = OnceLock::new();
static FONTS: OnceLock<Mutex<HashMap<i32, isize>>> = OnceLock::new();
/// 后台线程只追这一张：翻页翻过去的直接弃算，同一时刻最多一张在全尺寸解码。
static WANT: AtomicUsize = AtomicUsize::new(usize::MAX);
static BUSY: AtomicBool = AtomicBool::new(false);
static QWANT: AtomicUsize = AtomicUsize::new(usize::MAX);
static QBUSY: AtomicBool = AtomicBool::new(false);
/// 全屏箭头的显隐代数：进出箭头区域各加一次，挂着的"一秒后隐藏"对不上号就作废。
static NAV_GEN: AtomicUsize = AtomicUsize::new(0);

fn app() -> &'static Mutex<Option<Cur>> {
    APP.get_or_init(|| Mutex::new(None))
}
fn cache() -> &'static Mutex<Option<(usize, u32, u32, View, f64)>> {
    CACHE.get_or_init(|| Mutex::new(None))
}
fn theme() -> &'static Mutex<Option<Theme>> {
    THEME.get_or_init(|| Mutex::new(None))
}
fn fonts() -> &'static Mutex<HashMap<i32, isize>> {
    FONTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn host() -> Option<HWND> {
    let h = HOST.load(Ordering::SeqCst);
    if h == 0 {
        None
    } else {
        Some(HWND(h as *mut core::ffi::c_void))
    }
}

fn kick() {
    if let Some(h) = host() {
        unsafe {
            let _ = InvalidateRect(Some(h), None, false);
        }
    }
}

fn th() -> Option<Theme> {
    theme().lock().unwrap().clone()
}

/// 同一号字体只建一次；粗体用负数那一格记。
unsafe fn fnt(px: i32, bold: bool) -> HFONT {
    let key = px * 2 + bold as i32;
    if let Some(h) = fonts().lock().unwrap().get(&key).copied() {
        return HFONT(h as *mut core::ffi::c_void);
    }
    let face: Vec<u16> = "Microsoft YaHei UI".encode_utf16().chain([0]).collect();
    let f = CreateFontW(
        -px.abs().max(9),
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
        PCWSTR(face.as_ptr()),
    );
    fonts().lock().unwrap().insert(key, f.0 as isize);
    f
}

fn box_for(mode: Mode, cw: u32, ch: u32, nw: u32, nh: u32) -> (u32, u32) {
    match mode {
        Mode::Fit => (cw.max(1), ch.max(1)),
        Mode::Actual => (nw, nh),
    }
}

/// 这次出图对应的整图像素尺寸。解码还没回来时退到位图自身尺寸，别拿 0 去除。
fn base_size(v: &View) -> (f64, f64) {
    if v.nw > 0 && v.nh > 0 {
        (v.nw as f64, v.nh as f64)
    } else {
        (v.w.max(1) as f64, v.h.max(1) as f64)
    }
}

/// 整张图铺进画布的倍率：等比内接、小图不放大，和解码那一档同口径。
fn fit_zoom(aw: f64, ah: f64, nw: f64, nh: f64) -> f64 {
    (aw / nw).min(ah / nh).min(1.0)
}

/// 当前这一帧真正生效的倍率。整张态是算出来的，放大态才是手上那个数。
fn cur_zoom(c: &Cur, aw: f64, ah: f64) -> f64 {
    let (nw, nh) = base_size(&c.view);
    match c.mode {
        Mode::Fit => fit_zoom(aw, ah, nw, nh),
        Mode::Actual => c.scale,
    }
}

fn zones(rc: RECT) -> Zones {
    // 全屏只留图：工具条和状态栏的矩形都收成零宽，所有命中测试自然落空。
    if fullscreen() {
        let z = RECT { left: rc.left, top: rc.top, right: rc.left, bottom: rc.top };
        return Zones { bar0: z, bar1: z, status: z, area: rc };
    }
    let t = th().unwrap_or_else(|| Theme::new(&crate::theme::default_name(), "", 1.0));
    let h0 = (t.bar * 3 / 4).max(24);
    let h1 = (t.row + 6).max(20);
    let hs = (t.status + 6).max(20);
    // 顶上让出自绘标题栏：那块归宿主窗口画，标题栏按钮的拖动/关闭走宿主。
    let ht = title_h();
    let top = rc.top + ht + h0 + h1;
    let bot = (rc.bottom - hs).max(top + 1);
    Zones {
        bar0: RECT { left: rc.left, top: rc.top + ht, right: rc.right, bottom: rc.top + ht + h0 },
        bar1: RECT {
            left: rc.left,
            top: rc.top + ht + h0,
            right: rc.right,
            bottom: top,
        },
        status: RECT { left: rc.left, top: bot, right: rc.right, bottom: rc.bottom },
        area: RECT { left: rc.left, top, right: rc.right.max(rc.left + 1), bottom: bot },
    }
}

pub fn is_open() -> bool {
    ON.load(Ordering::SeqCst)
}

fn fullscreen() -> bool {
    !TITLED.load(Ordering::SeqCst)
}

/// 顶部自绘标题栏占多高。全屏时为 0。
pub fn title_h() -> i32 {
    if !TITLED.load(Ordering::SeqCst) {
        return 0;
    }
    th().map(|t| t.title.max(0)).unwrap_or(0)
}

pub fn set_titled(on: bool) {
    TITLED.store(on, Ordering::SeqCst);
}

/// 进全屏：先记下当前这一屏的视图，再改成整张图按新画布铺满。
/// 调用方要先把窗口摆成全屏、set_titled(false) 之后再叫这一个，倍率是按新画布算的。
pub fn enter_full() {
    {
        let mut g = app().lock().unwrap();
        let Some(c) = g.as_mut() else { return };
        c.saved = Some((c.mode, c.scale, c.panx, c.pany));
        c.mode = Mode::Fit;
        c.panx = 0;
        c.pany = 0;
        c.nav = 0;
        c.nav_show = false;
        if let Some(h) = host() {
            c.rc = client_of(h);
        }
    }
    show(current());
}

/// 退全屏：把进全屏前那一屏原样还回去，包括当时放大到哪、看到哪一块。
pub fn exit_full() {
    {
        let mut g = app().lock().unwrap();
        let Some(c) = g.as_mut() else { return };
        c.nav = 0;
        c.nav_show = false;
        if let Some((m, s, px, py)) = c.saved.take() {
            c.mode = m;
            c.scale = s;
            c.panx = px;
            c.pany = py;
        }
        if let Some(h) = host() {
            c.rc = client_of(h);
        }
    }
    show(current());
}

pub fn set_host(hwnd: HWND) {
    HOST.store(hwnd.0 as isize, Ordering::SeqCst);
}

pub fn set_theme(t: &Theme) {
    *theme().lock().unwrap() = Some(t.clone());
}

pub fn current() -> usize {
    app().lock().unwrap().as_ref().map(|c| c.idx).unwrap_or(0)
}

pub fn total() -> usize {
    app().lock().unwrap().as_ref().map(|c| c.files.len()).unwrap_or(0)
}

pub fn shown() -> Option<String> {
    app().lock().unwrap().as_ref().and_then(|c| c.files.get(c.idx).cloned())
}

pub fn title() -> String {
    let g = app().lock().unwrap();
    let Some(c) = g.as_ref() else { return String::new() };
    let name = c
        .files
        .get(c.idx)
        .and_then(|p| p.rsplit('\\').next())
        .unwrap_or("")
        .to_string();
    format!("Xsee - [{}] ({}/{})", name, c.idx + 1, c.files.len())
}

fn set_title() {
    let t = title();
    if let Some(h) = host() {
        unsafe {
            let _ = SetWindowTextW(h, &HSTRING::from(t));
        }
    }
}

/// 界面线程只切状态、贴手上已有的画面；查尺寸、取预览、全尺寸解码全在后台。
fn show(idx: usize) {
    let ok = app()
        .lock()
        .unwrap()
        .as_ref()
        .map(|c| idx < c.files.len())
        .unwrap_or(false);
    if !ok {
        return;
    }
    {
        let mut g = app().lock().unwrap();
        if let Some(c) = g.as_mut() {
            // 换图就回到"整张铺进窗口"，不然上一张的放大倍率会让新图只剩一角
            let moved = c.idx != idx;
            if moved {
                c.mode = Mode::Fit;
                c.scale = 1.0;
                c.panx = 0;
                c.pany = 0;
                c.ms = 0.0;
                c.src_full = false;
            }
            c.idx = idx;
            c.sel = None;
            c.drag = false;
            c.pan = None;
            c.bytes = c
                .files
                .get(idx)
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len())
                .unwrap_or(0);
        }
    }
    WANT.store(idx, Ordering::SeqCst);
    QWANT.store(idx, Ordering::SeqCst);
    set_title();
    kick();
    quick_follow();
    follow();
}

fn client_of(h: HWND) -> RECT {
    let mut rc = RECT::default();
    unsafe {
        let _ = GetClientRect(h, &mut rc);
    }
    rc
}

/// 档内预览那条快路。和 follow 分开，不让全尺寸解码把第一帧堵在后面。
fn quick_follow() {
    if QBUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let Some(h) = host() else {
        QBUSY.store(false, Ordering::SeqCst);
        return;
    };
    let hp = h.0 as isize;
    std::thread::spawn(move || {
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        job::init_com();
        let mut done = usize::MAX;
        loop {
            let want = QWANT.load(Ordering::SeqCst);
            if want == usize::MAX {
                break;
            }
            let snap = app()
                .lock()
                .unwrap()
                .as_ref()
                .map(|c| (c.files.clone(), c.mode, zones(client_of(hwnd)).area));
            let Some((files, mode, a)) = snap else { break };
            let Some(path) = files.get(want).cloned() else { break };
            let (nw, nh) = job::open_size(&path).unwrap_or((1, 1));
            let (bw, bh) = box_for(mode, (a.right - a.left).max(1) as u32, (a.bottom - a.top).max(1) as u32, nw, nh);
            let Some(mut v) = job::embedded_view(&path, bw, bh) else {
                done = want;
                break;
            };
            // 内嵌预览比原图小、又没被缩进画布：这就是那张一百来像素的缩略图，
            // 贴上去只会在屏幕中间露出一张小图，等全尺寸解码到了再跳一下。
            if (v.nw < nw || v.nh < nh) && v.w == v.nw && v.h == v.nh {
                // 记成"这张的快速帧不要了"，否则收尾会认为还没处理过、再起一轮
                done = want;
                break;
            }
            // 摆位按原图尺寸算，预览帧和后面的清晰图是同一块画布，不会先小后大跳一下
            if nw > 1 && nh > 1 {
                v.nw = nw;
                v.nh = nh;
            }
            done = want;
            if QWANT.load(Ordering::SeqCst) == want {
                *QUICK.lock().unwrap() = Some((want, v));
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_VQUICK, WPARAM(want), LPARAM(0));
                }
                break;
            }
        }
        QBUSY.store(false, Ordering::SeqCst);
        let w = QWANT.load(Ordering::SeqCst);
        if w != done && w != usize::MAX {
            quick_follow();
        }
    });
}

/// 串行解码线程。抢不到 BUSY 就说明有人还在跑，他收尾时会重看 WANT，不用再起一个。
fn follow() {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let Some(h) = host() else {
        BUSY.store(false, Ordering::SeqCst);
        return;
    };
    let hp = h.0 as isize;
    std::thread::spawn(move || {
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        job::init_com();
        let mut done = usize::MAX;
        loop {
            let want = WANT.load(Ordering::SeqCst);
            let snap = app()
                .lock()
                .unwrap()
                .as_ref()
                .map(|c| (c.files.clone(), c.mode, zones(client_of(hwnd)).area));
            let Some((files, mode, a)) = snap else { break };
            let Some(path) = files.get(want).cloned() else { break };
            let (nw, nh) = job::open_size(&path).unwrap_or((1, 1));
            let (bw, bh) = box_for(mode, (a.right - a.left).max(1) as u32, (a.bottom - a.top).max(1) as u32, nw, nh);
            // 同一张图、同一个解码盒，手上那张清晰 view 就是这个产物：直接跳过。
            // 缩放键、F↔1、改窗口尺寸全走 show()，没有这道检查每次都是一次全尺寸重解码。
            let have = {
                let g = app().lock().unwrap();
                g.as_ref().is_some_and(|c| c.idx == want && c.src_full && c.full_box == (bw, bh))
            };
            if have {
                done = want;
                break;
            }
            let t = Instant::now();
            let Ok(v) = job::decode_view(&path, bw, bh) else {
                // 这张解不开就记下，别让收尾把它当成没处理而一直重试
                done = want;
                break;
            };
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            done = want;
            *cache().lock().unwrap() = Some((want, bw, bh, v, ms));
            if WANT.load(Ordering::SeqCst) == want {
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_VCLEAR, WPARAM(want), LPARAM(0));
                }
                break;
            }
        }
        BUSY.store(false, Ordering::SeqCst);
        if WANT.load(Ordering::SeqCst) != done {
            follow();
        }
    });
}

/// 清晰图到了才换画面；已经翻到别张就丢弃（结果留在 CACHE 里，翻回去不用重解）。
pub fn clear_msg(idx: usize) -> bool {
    let Some((bw, bh, view, ms)) = cache()
        .lock()
        .unwrap()
        .take_if(|c| c.0 == idx)
        .map(|c| (c.1, c.2, c.3, c.4))
    else {
        return false;
    };
    let mut g = app().lock().unwrap();
    let Some(c) = g.as_mut() else { return false };
    if c.idx != idx {
        return false;
    }
    c.view = view;
    c.ms = ms;
    c.src_full = true;
    c.full_box = (bw, bh);
    drop(g);
    set_title();
    true
}

pub fn quick_msg(idx: usize) -> bool {
    if current() != idx {
        return false;
    }
    let Some(v) = QUICK.lock().unwrap().take_if(|q| q.0 == idx).map(|q| q.1) else {
        return false;
    };
    let mut g = app().lock().unwrap();
    let Some(c) = g.as_mut() else { return false };
    // 清晰图已经贴上去了，晚到的预览帧不能再把画面盖回小图
    if c.src_full {
        return false;
    }
    c.view = v;
    true
}

fn fmt_size(n: u64) -> String {
    const U: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < 3 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}

// ---------------------------------------------------------------- 绘制

unsafe fn fill(hdc: HDC, r: &RECT, c: u32) {
    let b = CreateSolidBrush(cref(c));
    let _ = FillRect(hdc, r, b);
    let _ = DeleteObject(HGDIOBJ(b.0));
}

unsafe fn hline(hdc: HDC, x0: i32, x1: i32, y: i32, c: u32) {
    fill(hdc, &RECT { left: x0, top: y, right: x1.max(x0 + 1), bottom: y + 1 }, c);
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
    let mut r = RECT { left: 0, top: 0, right: 2000, bottom: 200 };
    let _ = DrawTextW(hdc, &mut u, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
    (r.right - r.left).max(8)
}

unsafe fn dib(hdc: HDC, dst: &RECT, src: &View) {
    if src.w == 0 || src.h == 0 || dst.right <= dst.left || dst.bottom <= dst.top {
        return;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: src.w as i32,
            biHeight: -(src.h as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = SetStretchBltMode(hdc, HALFTONE);
    let mut org = POINT::default();
    let _ = SetBrushOrgEx(hdc, 0, 0, Some(&mut org));
    let _ = StretchDIBits(
        hdc,
        dst.left,
        dst.top,
        dst.right - dst.left,
        dst.bottom - dst.top,
        0,
        0,
        src.w as i32,
        src.h as i32,
        Some(src.px.as_ptr() as *const core::ffi::c_void),
        &bmi,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

/// 按钮排布：文字宽度 + 左右留白，组与组之间画一条竖线。
unsafe fn paint_bar(hdc: HDC, row: u32, r: RECT) {
    let t = th().unwrap();
    let s = &t.skin;
    fill(hdc, &r, s.chrome);
    hline(hdc, r.left, r.right, r.bottom - 1, s.line_soft);
    let font = fnt(t.font, false);
    let pad = (6.0 * t.scale) as i32;
    let mut x = r.left + pad;
    let mut btns: Vec<(usize, RECT)> = Vec::new();
    for (i, b) in BTNS.iter().enumerate() {
        if b.row != row {
            continue;
        }
        if b.sep && x > r.left + pad {
            x += pad / 2;
        }
        let w = text_w(hdc, b.label, font) + pad * 2;
        let hot = app().lock().unwrap().as_ref().map(|c| c.hot == i).unwrap_or(false);
        let br = RECT { left: x, top: r.top + 3, right: (x + w).min(r.right - 2), bottom: r.bottom - 4 };
        if hot {
            fill(hdc, &br, s.hov);
        }
        txt(hdc, &br, b.label, s.ink, font, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
        btns.push((i, br));
        x += w + 1;
    }
    if let Some(c) = app().lock().unwrap().as_mut() {
        if row == 0 {
            c.btns = btns;
        } else {
            c.btns.extend(btns);
        }
    }
}

unsafe fn paint_status(hdc: HDC, z: &Zones) {
    let t = th().unwrap();
    let s = &t.skin;
    fill(hdc, &z.status, s.chrome);
    hline(hdc, z.status.left, z.status.right, z.status.top, s.line_soft);
    let (idx, n, ms, name, bytes, nw, nh) = {
        let g = app().lock().unwrap();
        match g.as_ref() {
            Some(c) => (
                c.idx,
                c.files.len(),
                c.ms,
                c.files.get(c.idx).and_then(|p| p.rsplit('\\').next()).unwrap_or("").to_string(),
                c.bytes,
                c.view.nw,
                c.view.nh,
            ),
            None => (0, 0, 0.0, String::new(), 0, 0, 0),
        }
    };
    let f = fnt(t.font_sm, false);
    let pad = (6.0 * t.scale) as i32;
    let left = format!("{}/{}", idx + 1, n);
    let r = RECT { left: z.status.left + pad, top: z.status.top, right: z.status.left + pad + 90, bottom: z.status.bottom };
    txt(hdc, &r, &left, s.ink, f, DT_VCENTER | DT_SINGLELINE);
    // 像素尺寸、文件大小、解码耗时挪到状态栏右边：右侧那一栏已经按他要求撤掉了
    let mut meta: Vec<String> = Vec::new();
    if nw > 0 && nh > 0 {
        meta.push(format!("{nw} x {nh}"));
    }
    if bytes > 0 {
        meta.push(fmt_size(bytes));
    }
    if ms > 0.0 {
        meta.push(format!("{ms:.0} ms"));
    }
    let right = meta.join("   ");
    let track = RECT { left: r.right + 6, top: z.status.top, right: z.status.right - pad, bottom: z.status.bottom };
    let rw = if right.is_empty() { 0 } else { text_w(hdc, &right, f) + 2 };
    let rr = RECT { left: track.right - rw, top: track.top, right: track.right, bottom: track.bottom };
    txt(hdc, &rr, &right, s.ink, f, DT_VCENTER | DT_SINGLELINE);
    let nm = RECT { left: track.left, top: track.top, right: (rr.left - 8).max(track.left), bottom: track.bottom };
    txt(hdc, &nm, &name, s.ink_dim, f, DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
}

/// 全屏画布底色。固定深灰，皮肤换成什么都还是这一块。
const FULL_BG: u32 = 0x00242424;

/// 全屏两侧的翻页箭头：位置钉在画布左右边缘的垂直中线上，不跟图片走。
const NAV_W: i32 = 54;
const NAV_H: i32 = 88;
const NAV_MARGIN: i32 = 26;
const NAV_BG: u32 = 0x003A3A3A;
const NAV_BG_HOT: u32 = 0x00565656;
const NAV_EDGE: u32 = 0x004C4C4C;
const NAV_FG: u32 = 0x00DCDCDC;

fn can_step(dir: i8) -> bool {
    let i = current();
    if dir < 0 { i > 0 } else { i + 1 < total() }
}

fn nav_rect(z: &Zones, dir: i8) -> RECT {
    let cy = (z.area.top + z.area.bottom) / 2;
    let left = if dir < 0 {
        z.area.left + NAV_MARGIN
    } else {
        z.area.right - NAV_MARGIN - NAV_W
    };
    RECT { left, top: cy - NAV_H / 2, right: left + NAV_W, bottom: cy - NAV_H / 2 + NAV_H }
}

fn nav_hit(z: &Zones, x: i32, y: i32) -> i8 {
    if !fullscreen() {
        return 0;
    }
    for d in [-1i8, 1] {
        if can_step(d) && over(&nav_rect(z, d), x, y) {
            return d;
        }
    }
    0
}

/// 鼠标离开箭头一秒后把箭头收起来。进出一次就让旧的那条计时作废。
fn arm_nav_hide() {
    let tag = NAV_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if NAV_GEN.load(Ordering::SeqCst) != tag {
            return;
        }
        {
            let mut g = app().lock().unwrap();
            let Some(c) = g.as_mut() else { return };
            if c.nav != 0 {
                return;
            }
            c.nav_show = false;
        }
        kick();
    });
}

unsafe fn paint_nav(hdc: HDC, z: &Zones) {
    let (show, hot) = app()
        .lock()
        .unwrap()
        .as_ref()
        .map(|c| (c.nav_show, c.nav))
        .unwrap_or((false, 0));
    if !show {
        return;
    }
    for d in [-1i8, 1] {
        if !can_step(d) {
            continue;
        }
        let r = nav_rect(z, d);
        let on = hot == d;
        let br = CreateSolidBrush(cref(if on { NAV_BG_HOT } else { NAV_BG }));
        let pen = CreatePen(PS_SOLID, 1, cref(NAV_EDGE));
        let ob = SelectObject(hdc, HGDIOBJ(br.0));
        let op = SelectObject(hdc, HGDIOBJ(pen.0));
        let _ = RoundRect(hdc, r.left, r.top, r.right, r.bottom, 18, 18);
        let _ = SelectObject(hdc, ob);
        let _ = SelectObject(hdc, op);
        let _ = DeleteObject(HGDIOBJ(br.0));
        let _ = DeleteObject(HGDIOBJ(pen.0));
        let (cx, cy) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
        let k = (NAV_H / 8).max(7);
        let tip = if d < 0 { cx - k / 2 } else { cx + k / 2 };
        let back = if d < 0 { cx + k / 2 } else { cx - k / 2 };
        let pts = [
            POINT { x: back, y: cy - k },
            POINT { x: tip, y: cy },
            POINT { x: back, y: cy + k },
        ];
        let p = CreatePen(PS_SOLID, 3, cref(if on { 0x00FFFFFF } else { NAV_FG }));
        let op = SelectObject(hdc, HGDIOBJ(p.0));
        let _ = Polyline(hdc, &pts);
        let _ = SelectObject(hdc, op);
        let _ = DeleteObject(HGDIOBJ(p.0));
    }
}

unsafe fn paint_image(hdc: HDC, z: &Zones) {
    let s = th().unwrap().skin;
    // 全屏的底色是固定的深灰，不跟皮肤走
    fill(hdc, &z.area, if fullscreen() { FULL_BG } else { s.canvas });
    let g = app().lock().unwrap();
    let Some(c) = g.as_ref() else { return };
    let a = z.area;
    let aw = (a.right - a.left) as f64;
    let ah = (a.bottom - a.top) as f64;
    let (nw, nh) = base_size(&c.view);
    let zoom = cur_zoom(&c, aw, ah);
    let dw = (nw * zoom) as i32;
    let dh = (nh * zoom) as i32;
    let dx = (if c.mode == Mode::Fit {
        a.left as f64 + (aw - dw as f64) / 2.0
    } else {
        a.left as f64 + c.panx as f64
    }) as i32;
    let dy = (if c.mode == Mode::Fit {
        a.top as f64 + (ah - dh as f64) / 2.0
    } else {
        a.top as f64 + c.pany as f64
    }) as i32;
    let dst = RECT { left: dx, top: dy, right: dx + dw, bottom: dy + dh };
    dib(hdc, &dst, &c.view);
    if let Some((x0, y0, x1, y1)) = c.sel {
        let pen = CreatePen(PS_SOLID, 1, COLORREF(0x00FFFFFF));
        let op = SelectObject(hdc, HGDIOBJ(pen.0));
        let ob = SelectObject(hdc, GetStockObject(NULL_BRUSH));
        let _ = Rectangle(hdc, x0.min(x1), y0.min(y1), x0.max(x1) + 1, y0.max(y1) + 1);
        let _ = SelectObject(hdc, ob);
        let _ = SelectObject(hdc, op);
        let _ = DeleteObject(HGDIOBJ(pen.0));
    }
}

/// 宿主在自己的双缓冲上直接画这一整张脸。
pub unsafe fn paint(hdc: HDC, rc: RECT) {
    if !is_open() {
        return;
    }
    let z = zones(rc);
    if let Some(c) = app().lock().unwrap().as_mut() {
        c.btns.clear();
    }
    paint_image(hdc, &z);
    if fullscreen() {
        paint_nav(hdc, &z);
    } else {
        paint_status(hdc, &z);
        paint_bar(hdc, 0, z.bar0);
        paint_bar(hdc, 1, z.bar1);
    }
}

pub fn resize(rc: RECT) {
    let again = {
        let mut g = app().lock().unwrap();
        match g.as_mut() {
            Some(c) => {
                let old = c.rc;
                c.rc = rc;
                // 只比宽不够：最大化↔全屏两次的客户区一样宽，变的全在高度上
                // （还有工具条、状态栏让出来的那一截），一比宽就判定"没变"，图一直按旧画布显示。
                let (o, n) = (zones(old).area, zones(rc).area);
                c.mode == Mode::Fit
                    && (n.right - n.left != o.right - o.left || n.bottom - n.top != o.bottom - o.top)
            }
            None => false,
        }
    };
    if again {
        show(current());
    }
}

// ---------------------------------------------------------------- 交互

fn over(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

fn run(k: K) -> Act {
    match k {
        K::Prev => {
            let i = current();
            if i > 0 {
                show(i - 1);
            }
            Act::Nothing
        }
        K::Next => {
            let i = current();
            if i + 1 < total() {
                show(i + 1);
            }
            Act::Nothing
        }
        K::ZoomIn => {
            zoom_by(1.25);
            Act::Nothing
        }
        K::ZoomOut => {
            zoom_by(0.8);
            Act::Nothing
        }
        K::Fit => {
            set_mode(Mode::Fit);
            show(current());
            Act::Nothing
        }
        K::Actual => {
            if let Some(c) = app().lock().unwrap().as_mut() {
                c.mode = Mode::Actual;
                c.scale = 1.0;
            }
            show(current());
            Act::Nothing
        }
        K::Thumb => Act::Leave,
        K::Big => {
            set_mode(Mode::Fit);
            show(current());
            Act::Nothing
        }
        K::Refresh => {
            *cache().lock().unwrap() = None;
            *QUICK.lock().unwrap() = None;
            // 命中检查认的是手上那张 view；刷新要真重解，就得把这个记录一并抹掉。
            if let Some(c) = app().lock().unwrap().as_mut() {
                c.full_box = (0, 0);
            }
            show(current());
            Act::Nothing
        }
        K::C(c) => Act::Do(c),
    }
}

fn set_mode(m: Mode) {
    if let Some(c) = app().lock().unwrap().as_mut() {
        c.mode = m;
    }
}

pub fn zoom_by(f: f64) {
    let area = host().map(|h| zones(client_of(h)).area);
    {
        let mut g = app().lock().unwrap();
        let Some(c) = g.as_mut() else { return };
        // 整张态下手上那个数还没落地（是每帧按画布算出来的），先把它取成数字再乘，
        // 不然从整张态按一次放大会直接跳到 1.25 倍原图。
        if c.mode == Mode::Fit {
            let Some(r) = area else { return };
            let z = cur_zoom(c, (r.right - r.left) as f64, (r.bottom - r.top) as f64);
            c.scale = z;
        }
        c.mode = Mode::Actual;
        c.scale = (c.scale * f).clamp(0.02, 40.0);
    }
    show(current());
}

/// 全屏滚轮缩放：以鼠标指着的那个点为锚，只改倍率和平移，不重新解码。
/// 屏上这张位图本来就是按画布解出来的，重解码也是同一张，直接拉伸就够。
fn zoom_at(f: f64, x: i32, y: i32) {
    let Some(h) = host() else { return };
    let a = zones(client_of(h)).area;
    let mut g = app().lock().unwrap();
    let Some(c) = g.as_mut() else { return };
    let aw = (a.right - a.left) as f64;
    let ah = (a.bottom - a.top) as f64;
    let (nw, nh) = base_size(&c.view);
    let z = cur_zoom(c, aw, ah);
    if z <= 0.0 {
        return;
    }
    let dx = if c.mode == Mode::Fit {
        a.left as f64 + (aw - nw * z) / 2.0
    } else {
        a.left as f64 + c.panx as f64
    };
    let dy = if c.mode == Mode::Fit {
        a.top as f64 + (ah - nh * z) / 2.0
    } else {
        a.top as f64 + c.pany as f64
    };
    let z2 = (z * f).clamp(0.02, 40.0);
    if (z2 - z).abs() < 1e-9 {
        return;
    }
    // 鼠标底下那个图像坐标点，缩放后还让它待在原处
    let ux = (x as f64 - dx) / z;
    let uy = (y as f64 - dy) / z;
    c.mode = Mode::Actual;
    c.scale = z2;
    c.panx = (x as f64 - ux * z2 - a.left as f64).round() as i32;
    c.pany = (y as f64 - uy * z2 - a.top as f64).round() as i32;
    drop(g);
    kick();
}

pub fn step(dir: i32) {
    let i = current() as i64 + dir as i64;
    if i >= 0 && (i as usize) < total() {
        show(i as usize);
    }
}

/// 把左键拖出来的选框放大到铺满画布。没有有效选框就什么也不做。
fn zoom_to_sel() -> bool {
    let Some(h) = host() else { return false };
    let a = zones(client_of(h)).area;
    let cw = (a.right - a.left).max(1) as f64;
    let ch = (a.bottom - a.top).max(1) as f64;
    let mut g = app().lock().unwrap();
    let Some(c) = g.as_mut() else { return false };
    let Some((sx, sy, ex, ey)) = c.sel.take() else {
        return false;
    };
    let (l, r) = (sx.min(ex) as f64, sx.max(ex) as f64);
    let (t, b) = (sy.min(ey) as f64, sy.max(ey) as f64);
    if r - l < 8.0 || b - t < 8.0 {
        drop(g);
        kick();
        return false;
    }
    let (nw, nh) = base_size(&c.view);
    let zoom = cur_zoom(c, cw, ch);
    let dw = nw * zoom;
    let dh = nh * zoom;
    let dx = if c.mode == Mode::Fit { ((a.right - a.left) as f64 - dw) / 2.0 + a.left as f64 } else { a.left as f64 + c.panx as f64 };
    let dy = if c.mode == Mode::Fit { ((a.bottom - a.top) as f64 - dh) / 2.0 + a.top as f64 } else { a.top as f64 + c.pany as f64 };
    // 选框四角落在图上的位置（0..1）。框到图外的部分得剪掉，不剪就会把
    // 中心算到图外面去，一放大整张图直接推出屏幕，看着像图丢了。
    let u0 = ((l - dx) / dw).clamp(0.0, 1.0);
    let u1 = ((r - dx) / dw).clamp(0.0, 1.0);
    let v0 = ((t - dy) / dh).clamp(0.0, 1.0);
    let v1 = ((b - dy) / dh).clamp(0.0, 1.0);
    if u1 - u0 < 0.002 || v1 - v0 < 0.002 {
        drop(g);
        kick();
        return false;
    }
    // 选框的长宽比和图不一定一致，按"整框都装得下"取较小的放大倍数
    let fit = (cw / ((u1 - u0) * nw)).min(ch / ((v1 - v0) * nh)).clamp(0.05, 16.0);
    c.scale = fit;
    c.mode = Mode::Actual;
    let ndw = nw * fit;
    let ndh = nh * fit;
    c.panx = (cw / 2.0 - ((u0 + u1) / 2.0) * ndw) as i32;
    c.pany = (ch / 2.0 - ((v0 + v1) / 2.0) * ndh) as i32;
    drop(g);
    kick();
    true
}

/// 鼠标离开窗口：箭头不再算指着，一秒后照常收起来。
pub fn mouse_leave() {
    if !is_open() {
        return;
    }
    let show = {
        let mut g = app().lock().unwrap();
        let Some(c) = g.as_mut() else { return };
        if c.nav == 0 {
            return;
        }
        c.nav = 0;
        c.nav_show
    };
    if show {
        arm_nav_hide();
    }
    kick();
}

pub fn mouse(msg: u32, wparam: WPARAM, x: i32, y: i32) -> Act {
    if !is_open() {
        return Act::Nothing;
    }
    let Some(h) = host() else { return Act::Nothing };
    // 滚轮消息带的是屏幕坐标，后面一律按客户区算
    let (x, y) = if msg == WM_MOUSEWHEEL {
        let mut p = POINT { x, y };
        unsafe {
            let _ = ScreenToClient(h, &mut p);
        }
        (p.x, p.y)
    } else {
        (x, y)
    };
    let z = zones(client_of(h));
    match msg {
        WM_MOUSEMOVE => {
            let hot = {
                let g = app().lock().unwrap();
                match g.as_ref() {
                    Some(c) => c.btns.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i).unwrap_or(usize::MAX),
                    None => usize::MAX,
                }
            };
            let nav = nav_hit(&z, x, y);
            let mut need = false;
            {
                let mut g = app().lock().unwrap();
                if let Some(c) = g.as_mut() {
                    if c.hot != hot {
                        c.hot = hot;
                        need = true;
                    }
                    if c.nav != nav {
                        c.nav = nav;
                        if nav != 0 {
                            NAV_GEN.fetch_add(1, Ordering::SeqCst);
                            c.nav_show = true;
                        } else if c.nav_show {
                            arm_nav_hide();
                        }
                        need = true;
                    }
                    if let Some((sx, sy, px, py)) = c.pan {
                        let (nx, ny) = (px + x - sx, py + y - sy);
                        if nx != c.panx || ny != c.pany {
                            c.panx = nx;
                            c.pany = ny;
                            need = true;
                        }
                    } else if c.drag {
                        if let Some((x0, y0, x1, y1)) = c.sel {
                            if x != x1 || y != y1 {
                                c.sel = Some((x0, y0, x, y));
                                need = true;
                            }
                        }
                    }
                }
            }
            if need {
                kick();
            }
            Act::Nothing
        }
        WM_LBUTTONDOWN => {
            let hit = {
                let g = app().lock().unwrap();
                match g.as_ref() {
                    Some(c) => c.btns.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i),
                    None => None,
                }
            };
            if let Some(i) = hit {
                if let Some(b) = BTNS.get(i) {
                    return run(b.k);
                }
                return Act::Nothing;
            }
            // 全屏两侧的箭头：左上一张、右下一张
            if fullscreen() {
                let d = nav_hit(&z, x, y);
                if d != 0 {
                    return run(if d < 0 { K::Prev } else { K::Next });
                }
            }
            if over(&z.area, x, y) {
                {
                    let mut g = app().lock().unwrap();
                    if let Some(c) = g.as_mut() {
                        // 放大态：左键按住拖图；整张态：左键拖出选框，松手放大。
                        if c.mode == Mode::Actual {
                            c.pan = Some((x, y, c.panx, c.pany));
                        } else {
                            c.sel = Some((x, y, x, y));
                            c.drag = true;
                        }
                    }
                }
                // SetCapture 挪在放锁之后：捕获转移会同步发 WM_CAPTURECHANGED，
                // 哪天有人让它重入拿这把锁就是当场自锁（refwin 那边是拿锁前调的）。
                unsafe {
                    let _ = SetCapture(h);
                }
            }
            Act::Nothing
        }
        WM_LBUTTONUP => {
            let mut zoom = false;
            {
                let mut g = app().lock().unwrap();
                if let Some(c) = g.as_mut() {
                    if c.pan.take().is_none() {
                        c.drag = false;
                        // 只是单击没拖出框：别把一个小点留成选框
                        if let Some((x0, y0, x1, y1)) = c.sel {
                            if (x1 - x0).abs() < 4 && (y1 - y0).abs() < 4 {
                                c.sel = None;
                            } else {
                                zoom = true;
                            }
                        }
                    }
                }
            }
            unsafe {
                let _ = ReleaseCapture();
            }
            if zoom {
                zoom_to_sel();
            } else {
                kick();
            }
            Act::Nothing
        }
        // 双击画布就是看完退回缩略图那一屏；翻页交给滚轮和方向键
        WM_LBUTTONDBLCLK => {
            // 双击落在翻页箭头上不能当成"看完退出"
            if nav_hit(&z, x, y) != 0 {
                return Act::Nothing;
            }
            if over(&z.area, x, y) {
                Act::Leave
            } else {
                Act::Nothing
            }
        }
        // 中键只管全屏
        WM_MBUTTONDOWN => Act::Full,
        WM_MOUSEWHEEL => {
            if !over(&z.area, x, y) {
                return Act::Nothing;
            }
            let d = ((wparam.0 >> 16) & 0xffff) as u16 as i16;
            if fullscreen() {
                // 全屏滚轮是缩放；大图模式还是翻页
                zoom_at(if d > 0 { 1.25 } else { 0.8 }, x, y);
            } else {
                step(if d > 0 { -1 } else { 1 });
            }
            Act::Nothing
        }
        _ => Act::Nothing,
    }
}

pub fn key(vk: u32) -> Act {
    if !is_open() {
        return Act::Nothing;
    }
    match vk {
        0x1B | 0x51 | 0x0D => Act::Leave,
        0x25 | 0x21 => {
            step(-1);
            Act::Nothing
        }
        0x27 | 0x22 | 0x20 => {
            step(1);
            Act::Nothing
        }
        0x24 => {
            show(0);
            Act::Nothing
        }
        0x23 => {
            let n = total();
            show(n.saturating_sub(1));
            Act::Nothing
        }
        0x46 => run(K::Fit),
        0x31 => run(K::Actual),
        0xBB => run(K::ZoomIn),
        0x6B | 0x4C => run(K::ZoomIn),
        0xBD | 0x6D | 0x4D => run(K::ZoomOut),
        _ => Act::Nothing,
    }
}

// ---------------------------------------------------------------- 进出

fn list_files(dir: &str) -> Vec<String> {
    scan::list(dir, false)
        .iter()
        .filter(|e| !e.is_dir && scan::is_image(&e.path))
        .map(|e| e.path.clone())
        .collect()
}

/// 挑出这一张该用的文件表：宿主递来的能用就用，否则只扫它所在的那个目录。
fn pick_files(path: &str, given: Option<Vec<String>>) -> (Vec<String>, usize) {
    let p = std::path::Path::new(path);
    let dir = if p.is_dir() {
        path.trim_end_matches('\\').to_string()
    } else {
        p.parent().map(|d| d.display().to_string()).unwrap_or_default()
    };
    let files = match given {
        Some(fs) if fs.iter().any(|f| std::path::Path::new(f) == p) => fs,
        _ => list_files(&dir),
    };
    let idx = files
        .iter()
        .position(|f| std::path::Path::new(f) == p)
        .unwrap_or(0)
        .min(files.len().saturating_sub(1));
    (files, idx)
}

/// 进入大图态。宿主已经把文件表、皮肤和窗口句柄备好了。
/// 首帧不拿列表里那张缩略图凑数：它只有一两百像素，铺满画布就是一团糊，
/// 宁可先留一块底色，等接近实尺寸的图到了再画。
pub fn open(path: &str, files: Option<Vec<String>>) -> bool {
    let (files, idx) = pick_files(path, files);
    if files.is_empty() {
        return false;
    }
    let Some(h) = host() else { return false };
    let rc = client_of(h);
    *app().lock().unwrap() = Some(Cur {
        idx,
        view: View { w: 0, h: 0, nw: 0, nh: 0, px: vec![0; 4] },
        files,
        mode: Mode::Fit,
        scale: 1.0,
        panx: 0,
        pany: 0,
        sel: None,
        drag: false,
        pan: None,
        rc,
        saved: None,
        btns: Vec::new(),
        hot: usize::MAX,
        ms: 0.0,
        bytes: 0,
        nav: 0,
        nav_show: false,
        src_full: false,
        full_box: (0, 0),
    });
    *cache().lock().unwrap() = None;
    *QUICK.lock().unwrap() = None;
    ON.store(true, Ordering::SeqCst);
    WANT.store(idx, Ordering::SeqCst);
    QWANT.store(idx, Ordering::SeqCst);
    show(idx);
    true
}

/// 退出大图态：把后台两条线叫停，状态清空。
pub fn close() {
    ON.store(false, Ordering::SeqCst);
    WANT.store(usize::MAX, Ordering::SeqCst);
    QWANT.store(usize::MAX, Ordering::SeqCst);
    *cache().lock().unwrap() = None;
    *QUICK.lock().unwrap() = None;
    *app().lock().unwrap() = None;
}

/// 已经在大图态里换一张（拖进来、外部再次双击同一程序时用）。
pub fn switch_to(path: &str, files: Option<Vec<String>>) {
    if !is_open() {
        let _ = open(path, files);
        return;
    }
    let (fs, idx) = pick_files(path, files);
    if fs.is_empty() {
        return;
    }
    {
        let mut g = app().lock().unwrap();
        if let Some(c) = g.as_mut() {
            c.files = fs;
            // 手上那张是上一张的，先清掉，别让它在新图到位前挂在画布上
            c.view = View { w: 0, h: 0, nw: 0, nh: 0, px: vec![0; 4] };
            c.src_full = false;
        }
    }
    *cache().lock().unwrap() = None;
    *QUICK.lock().unwrap() = None;
    show(idx);
}

// ---------------------------------------------------------------- 命令行壳

/// view.exe 还留着：没有宿主窗口时自己开一个窗口把上面这套画出来。
mod shell {
    use super::*;

    const CLASS: &str = "XseeViewWnd";

    unsafe fn paint_here(hwnd: HWND) {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let rc = client_of(hwnd);
        let w = (rc.right - rc.left).max(1);
        let h = (rc.bottom - rc.top).max(1);
        let mem = CreateCompatibleDC(Some(hdc));
        let bmp = CreateCompatibleBitmap(hdc, w, h);
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        super::paint(mem, rc);
        let _ = BitBlt(hdc, 0, 0, w, h, Some(mem), 0, 0, SRCCOPY);
        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        let _ = EndPaint(hwnd, &ps);
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        let x = (lp.0 & 0xffff) as u16 as i16 as i32;
        let y = ((lp.0 >> 16) & 0xffff) as u16 as i16 as i32;
        match msg {
            WM_PAINT => {
                paint_here(hwnd);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_SIZE => {
                resize(client_of(hwnd));
                LRESULT(0)
            }
            WM_VCLEAR => {
                if clear_msg(wp.0 as usize) {
                    kick();
                }
                LRESULT(0)
            }
            WM_VQUICK => {
                if quick_msg(wp.0 as usize) {
                    kick();
                }
                LRESULT(0)
            }
            WM_KEYDOWN => match key(wp.0 as u32) {
                Act::Leave => {
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                    LRESULT(0)
                }
                Act::Full => LRESULT(0),
                _ => LRESULT(0),
            },
            WM_SETCURSOR => {
                SetCursor(Some(LoadCursorW(None, IDC_ARROW).unwrap_or_default()));
                LRESULT(1)
            }
            WM_DESTROY => {
                close();
                let _ = PostQuitMessage(0);
                LRESULT(0)
            }
            WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_MOUSEMOVE | WM_MBUTTONDOWN
            | WM_RBUTTONUP | WM_MOUSEWHEEL => match mouse(msg, wp, x, y) {
                Act::Leave => {
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                    LRESULT(0)
                }
                _ => LRESULT(0),
            },
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }

    fn pw(s: &str) -> &'static [u16] {
        Box::leak(s.encode_utf16().chain([0]).collect::<Vec<u16>>().into_boxed_slice())
    }

    pub fn run(path: &str) {
        unsafe {
            use windows::Win32::UI::HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
            };
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let hinst = GetModuleHandleW(None).unwrap();
            let hinst = HINSTANCE(hinst.0);
            let (hicon, hicon_sm) = crate::appicon::load();
            let cls = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
                lpfnWndProc: Some(wndproc),
                hInstance: hinst,
                hIcon: hicon,
                hIconSm: hicon_sm,
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: PCWSTR(pw(CLASS).as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassExW(&cls);
            let mon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
            let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            let _ = GetMonitorInfoW(mon, &mut mi);
            let ww = ((mi.rcWork.right - mi.rcWork.left) as f64 * 0.85) as i32;
            let wh = ((mi.rcWork.bottom - mi.rcWork.top) as f64 * 0.85) as i32;
            let wx = mi.rcWork.left + ((mi.rcWork.right - mi.rcWork.left) - ww) / 2;
            let wy = mi.rcWork.top + ((mi.rcWork.bottom - mi.rcWork.top) - wh) / 2;
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(pw(CLASS).as_ptr()),
                PCWSTR(pw("Xsee 看图").as_ptr()),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                wx,
                wy,
                ww,
                wh,
                None,
                None,
                Some(hinst),
                None,
            );
            let Ok(hwnd) = hwnd else { return };
            set_host(hwnd);
            set_theme(&Theme::new(&crate::theme::default_name(), "", 1.25));
            if !open(path, None) {
                return;
            }
            let mut m = MSG::default();
            while GetMessageW(&mut m, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&m);
                DispatchMessageW(&m);
            }
            set_host(HWND::default());
        }
    }
}

pub fn run_cli(path: &str) {
    shell::run(path);
}

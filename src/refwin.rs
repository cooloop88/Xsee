#![allow(unsafe_op_in_unsafe_fn)]
//! 参考图画布（菜单上叫参考图画布模式）。
//!
//! 一块置顶的黑底画布，顶上只有一条标题条，别的一概没有：参考图是开着别的软件
//! 时拿来对照的，所以它不挂成主窗口的跟班（一挂 owner，点它就会把主窗口一起抬到
//! 前台，把正在用的那扇盖住），也不开第二扇——再点"参考图画布模式"是往这块画布上加一张。
//! 每张图自己拖，鼠标压到图片四边或四角能等比改尺寸；滚轮不管指针在图上还是空白，
//! 一律整块画布一起缩放（所有图按同一倍率一起变大变小，相对位置不散）。
//! 剪贴板里出现图片时标题条会写出来；右键弹一块自绘弹层，复制、粘贴、删除都在
//! 那一排里，Ctrl+V 是不用开菜单的那条近路。
//!
//! 这扇是分层窗口：整面内容由我们自己填一张 32 位图交给系统合成，标题条也是画的。
//! 走这条路的唯一理由是背景能透——把标题条上那个滑块往右拖，底就越来越看得见底下
//! 的软件，而图片那一片的 alpha 一律钉成不透，一点不跟着变。
//! 代价是系统那套标题栏（包括深色描边和关闭键）在分层窗口上挂不住，
//! 于是拖动、关窗、改大小都得自己接管：WM_NCCALCSIZE 让客户区盖满整窗，
//! WM_NCHITTEST 报边、报标题条。alpha 留了个下限，不留 0——
//! 全透的像素会被系统当成点不着，那样画布就拖不动了。
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::{appicon, job, View};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HGLOBAL, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, GetClipboardData, IsClipboardFormatAvailable,
    OpenClipboard, RemoveClipboardFormatListener,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::{CF_BITMAP, CF_DIB};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_C, VK_CONTROL, VK_DELETE, VK_ESCAPE, VK_RETURN,
    VK_SPACE, VK_V,
};
use windows::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, DragQueryPoint, HDROP,
};
use windows::Win32::UI::WindowsAndMessaging::*;

const CLASS: &str = "XseeRefWnd";
/// 后台那张解码回来了
const WM_IMG: u32 = WM_APP + 30;
/// 这扇被系统最小化了，回头自己弹回来（不能就地还原：见 WM_UNMIN 那段）
const WM_UNMIN: u32 = WM_APP + 31;
const TITLE: &str = "参考图模式";
const TITLE_CLIP: &str = "参考图模式 · 剪贴板有图片";
/// 偏好键在主题模块那一处（`theme::ui_get/ui_put`），这边只借它记一条
/// "说明卡看过了"；画布别的状态都不落盘。
const INTRO_KEY: &str = "ref_intro";
/// 第一次开画布时压在面上的使用说明：小标题 + 正文。正文按卡片宽度自然折行，
/// 不手写断句——窗口拉宽拉窄都不会排出一半空白。
const INTRO: &[(&str, &str)] = &[
    (
        "一、把图摆上来",
        "在主窗口选中图片，右键点「参考图画布模式」，图就摆到这块板上；再点一次是再添一张。\
         也可以从资源管理器直接把图片拖进来，或复制图片后按 Ctrl+V 贴上来。",
    ),
    (
        "二、位置和大小",
        "按住图片拖动可以挪位置；按住空白拖动，是整块画布一起移。\
         鼠标移到图片的四边或四角，拖动可以等比改大小。\
         滚轮压在哪儿都行，所有图一起放大缩小，相互位置不变。\
         右键双击图片，或把鼠标压在图上按 Delete，删掉那一张。",
    ),
    (
        "三、拿来对照",
        "画布总在最前面。拖住顶部标题条就能把它挪开，不挡正在用的软件。\
         顶部「透明」滑块往右拖，画布底越透，能看到下面的软件，图片本身不变。\
         在图片上右键有「复制图像」「粘贴剪贴板图片」「删除这张」，Esc 收起菜单。",
    ),
];
const INTRO_FOOT: &str = "支持：拖入、粘贴、多图、单独缩放、整块缩放、背景透明、复制图像、置顶";
/// 滚轮一格
const STEP: f64 = 1.1;
const SCALE_MIN: f64 = 0.02;
const SCALE_MAX: f64 = 40.0;
/// 解码缓冲的上限。超过它先等比压到这一档，不然几张大图就把内存吃穿。
const MAX_PIX: u64 = 16_000_000;
/// 一张图刚摆进画布时最多占掉视口的这一成，剩下的留白给他拖
const OCCUPY: f64 = 0.7;
/// 排图的边距
const PAD: i32 = 16;
/// 背景最不透明 / 最透明那一档。24 那档太粗，拉到底看着还是蒙着一层，改成 1：
/// 千分之一的黑，肉眼分不出和全透的差别。不能再往下 —— alpha 归 0 的那块像素
/// 会被系统当成点不着，鼠标就穿到底下软件里去了，画布拖不动、空白处也贴不成图。
const BG_SOLID: u8 = 255;
const BG_CLEAR: u8 = 1;
/// 标题条那一整套（底色、字色、关闭那颗的几何和悬停画法）都从主窗口喂过来。
/// 正常情况下宿主每画一次主窗口的标题栏就更新一次，换肤、换缩放这条跟着走。
/// 只有直接命令行起画布、宿主还没画过第一帧时它是空的，那时按默认皮肤现算一条 ——
/// 画出来仍是同一套画法，不再自己定一套深灰。
static STRIP: Mutex<Option<crate::theme::Strip>> = Mutex::new(None);

/// 对话浮层那层中性配色。说明卡照它画：以图搜图那个提示窗也是这套底，
/// 不再拿标题栏的红主色铺满一整块。
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Face {
    pub bg: u32,
    pub line: u32,
    pub ink: u32,
    pub dim: u32,
    pub hov: u32,
    /// 下面四样是右键弹层那套，和主窗口的菜单同源：面板底 `skin.popup`、
    /// 点不动那行的字色 `skin.ink_off`、分隔线 `skin.line_soft`。
    pub popup: u32,
    pub off: u32,
    pub sep: u32,
    /// 主窗口弹层一行多高（物理像素）。画布这扇照着它排行。
    pub menu_row: i32,
}

impl Face {
    fn of(th: &crate::theme::Theme) -> Face {
        Face {
            bg: th.skin.chrome_hi,
            line: th.skin.line,
            ink: th.skin.ink,
            dim: th.skin.ink_dim,
            hov: th.skin.hov,
            popup: th.skin.popup,
            off: th.skin.ink_off,
            sep: th.skin.line_soft,
            menu_row: crate::theme::popup_row(th),
        }
    }
}

static FACE: Mutex<Option<Face>> = Mutex::new(None);

struct Card {
    id: usize,
    /// 剪贴板贴进来的那张没有路径
    path: String,
    view: Option<View>,
    /// 画布坐标；屏幕上看到的是它加上画布平移量
    x: i32,
    y: i32,
    /// 相对 base 那对尺寸的倍率
    scale: f64,
    /// 倍率是照着多大的位图定的。解码回来若被压过一档，按这个比例把倍率折算掉，
    /// 屏幕上的占位一分不变，不然图会突然变大把旁边那张压住。
    base: (u32, u32),
}

/// 图片边上能抓住的八条（四角 + 四边）
#[derive(Clone, Copy, PartialEq, Eq)]
enum Grip {
    Nw,
    N,
    Ne,
    W,
    E,
    Sw,
    S,
    Se,
}

/// 正在拖某张图的边改尺寸。起始位置和尺寸都按客户区坐标记：
/// 拖的过程中每一下都从这一份重算，不累加，所以再快的移动也不会漂。
#[derive(Clone, Copy)]
struct Resize {
    id: usize,
    grip: Grip,
    x0: i32,
    y0: i32,
    w0: i32,
    h0: i32,
    scale0: f64,
}

/// 右键弹层里那三项的编号
const MI_COPY: usize = 1;
const MI_PASTE: usize = 2;
const MI_DEL: usize = 3;

/// 画布上那块自绘弹层。颜色全部借标题条那一套（`strip_of`），所以它跟着皮肤走。
struct Menu {
    /// 右键按下那一点（客户区坐标）：画的时候按窗口尺寸把它收进来
    x: i32,
    y: i32,
    /// 这一右键压在哪个卡片上；没有就是落在空白，只有粘贴点得动
    card: Option<usize>,
    /// 开菜单那一刻剪贴板里到底有没有能贴的东西（位图或图片文件都算）。
    /// 不现算：标题条上那份 `clip` 只跟位图，从资源管理器复制文件时它是灭的。
    paste: bool,
    /// 量过一次就存下来的盘宽（0 = 还没量）
    mw: i32,
    /// 悬停在哪一行（-1 = 没有）
    hot: i32,
    /// 画出来才知道的那几行：(编号, 那一行的位置)
    rows: Vec<(usize, RECT)>,
}

/// 这一项此刻点得动吗：复制和删除要压在图上，粘贴要看剪贴板有没有货
fn menu_on(id: usize, card: Option<usize>, paste: bool) -> bool {
    match id {
        MI_COPY | MI_DEL => card.is_some(),
        _ => paste,
    }
}

/// 开弹层：记下压在哪个卡片上，顺便现查一次剪贴板。
fn open_menu(s: &mut State, x: i32, y: i32) {
    let card = hit(s, x, y).map(|i| s.cards[i].id);
    s.menu = Some(Menu { x, y, card, paste: clip_ready(), mw: 0, hot: -1, rows: Vec::new() });
}

/// 收起弹层，有收才重推一面
unsafe fn close_menu(hwnd: HWND) {
    let had = {
        let mut g = state().lock().unwrap();
        match g.as_mut() {
            Some(s) => s.menu.take().is_some(),
            None => false,
        }
    };
    if had {
        push(hwnd);
    }
}

struct State {
    cards: Vec<Card>,
    panx: i32,
    pany: i32,
    /// 下一张往哪儿落：相对视口左上的偏移 + 这一行已经多高
    cur: (i32, i32),
    row_h: i32,
    /// 按住左键在拖的东西：卡片 id（-1 是空白，拖整块画布）、按下点、按下时的那对坐标
    drag: Option<(i64, i32, i32, i32, i32)>,
    /// 按住图片边缘在改尺寸（和上面那个互斥）
    resize: Option<Resize>,
    /// 剪贴板此刻有没有图片，只在标题栏那行字上生效
    clip: bool,
    rc: RECT,
    /// 上一次看到的客户区左上角在屏幕上的位置。改大小和挪窗口都会让它动，
    /// 靠它和上面 rc 的变化量分辨是哪一种。
    ox: i32,
    oy: i32,
    /// 常驻后备位图（内存 DC + 它身上的 32 位节 + 那位图的第一像素）
    buf_mem: isize,
    buf_bmp: isize,
    /// 位图像素首地址（当作 isize 存，指针本身不能进静态）
    bits: isize,
    buf_w: i32,
    buf_h: i32,
    /// 背景那一档不透明度：255 = 实心，24 = 最透。图片不跟着走。
    bg: u8,
    /// 标题条多高（物理像素，跟着这块屏的缩放走）
    strip: i32,
    /// 抓边改大小的感应宽度
    edge: i32,
    /// 标题条上那两块能点的地方：滑块感应区、滑块轨道、关闭键
    sld: RECT,
    trk: RECT,
    cls: RECT,
    /// 按住了滑块
    drag_sld: bool,
    font: isize,
    /// 这块屏的缩放，条上那点东西都按它折算
    dpi: i32,
    /// 鼠标最后停在哪儿（客户区坐标）：Del 和右键双击删的是压在这一底下的那张
    hover: (i32, i32),
    /// 右键弹层，开着才有
    menu: Option<Menu>,
    /// 第一次开这扇时压在上面的使用说明卡；点「知道了」以后不再出现
    intro: bool,
    /// 说明卡上「知道了」那颗的位置（画的时候填，点的时候用）
    intro_btn: RECT,
    /// 指针压在「知道了」上
    intro_hot: bool,
}

fn state() -> &'static Mutex<Option<State>> {
    static S: OnceLock<Mutex<Option<State>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None))
}

fn theme_of(dpi: i32) -> crate::theme::Theme {
    crate::theme::Theme::new(&crate::theme::default_name(), "", dpi as f64 / 96.0)
}

/// 这条多高、什么颜色：全从主窗口那套标题栏口径拿（`set_strip` 喂进来的那份）。
fn strip_of(dpi: i32) -> crate::theme::Strip {
    if let Some(s) = *STRIP.lock().unwrap() {
        return s;
    }
    crate::theme::Strip::top(&theme_of(dpi))
}

/// 说明卡那套中性底。主窗口还没画过第一帧的时候照默认皮肤现算一份。
fn face_of(dpi: i32) -> Face {
    if let Some(f) = *FACE.lock().unwrap() {
        return f;
    }
    Face::of(&theme_of(dpi))
}

fn strip_h(dpi: i32) -> i32 {
    strip_of(dpi).h
}

/// 宿主每画一次主窗口的标题栏就把那套取色喂过来：换肤、换缩放，画布这条跟着变，
/// 不再自己定一套深灰。一个值都没变就当没事发生 —— 这句每重画一次主窗口都会叫，
/// 画布不能跟着一直推面。
pub fn set_strip(st: crate::theme::Strip, face: Face) {
    {
        let mut g = STRIP.lock().unwrap();
        if *g == Some(st) && *FACE.lock().unwrap() == Some(face) {
            return;
        }
        *g = Some(st);
        *FACE.lock().unwrap() = Some(face);
    }
    if let Some(h) = hwnd_of() {
        unsafe { push(h) };
    }
}

/// 命令行口用：这扇在不在（在的话给它句柄，好拍图、好投消息）。
pub fn window() -> Option<HWND> {
    hwnd_of()
}

/// 命令行口跑测试时置真：建这扇就摆到屏幕外、也不抢前台。PrintWindow 拍得到
/// 屏幕外的窗，所以图照样出，他手上的软件不受打扰。
pub fn set_headless(on: bool) {
    HEADLESS.store(on, Ordering::SeqCst);
}

fn headless() -> bool {
    HEADLESS.load(Ordering::SeqCst)
}

static HEADLESS: AtomicBool = AtomicBool::new(false);

/// 字符串进 JSON 之前过一遍：路径里的反斜杠和引号不转义，读的那头就散了。
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// 命令行口用：把画布此刻的状态写成一段 JSON。哪张图落在哪儿、解码回来没有、
/// 弹层开在哪、那几行各占哪一块 —— 全都报出去，脚本照着行心点，不用猜坐标。
pub fn probe_json() -> String {
    let Some(g) = state().lock().ok() else { return "{}".into() };
    let Some(s) = g.as_ref() else { return "{\"open\":false}".into() };
    let (w, h) = (s.rc.right - s.rc.left, s.rc.bottom - s.rc.top);
    let mut o = format!(
        "{{\"open\":true,\"client\":[{w},{h}],\"dpi\":{},\"strip\":{},\"bg\":{},\"intro\":{},\"clip\":{}",
        s.dpi,
        s.strip,
        s.bg,
        s.intro,
        s.clip
    );
    o.push_str(",\"cards\":[");
    for (i, c) in s.cards.iter().enumerate() {
        let (x, y, cw, ch) = match on_screen(c, s) {
            Some(r) => (r.left, r.top, r.right - r.left, r.bottom - r.top),
            None => (c.x, c.y, 0, 0),
        };
        if i > 0 {
            o.push(',');
        }
        o.push_str(&format!(
            "{{\"id\":{},\"x\":{x},\"y\":{y},\"w\":{cw},\"h\":{ch},\"scale\":{:.4},\"ready\":{},\"path\":\"{}\"}}",
            c.id,
            c.scale,
            c.view.is_some(),
            esc(&c.path)
        ));
    }
    o.push(']');
    match s.menu.as_ref() {
        Some(m) => {
            let card = match m.card {
                Some(id) => id.to_string(),
                None => "null".to_string(),
            };
            o.push_str(&format!(
                ",\"menu\":{{\"x\":{},\"y\":{},\"card\":{card},\"paste\":{},\"hot\":{},\"rows\":[",
                m.x, m.y, m.paste, m.hot
            ));
            for (i, (id, r)) in m.rows.iter().enumerate() {
                if i > 0 {
                    o.push(',');
                }
                o.push_str(&format!(
                    "[{id},[{}, {}, {}, {}]]",
                    r.left, r.top, r.right, r.bottom
                ));
            }
            o.push_str("]}");
        }
        None => o.push_str(",\"menu\":null"),
    }
    o.push('}');
    o
}

/// 全局只有这一扇
static WIN: AtomicIsize = AtomicIsize::new(0);static REGISTERED: AtomicBool = AtomicBool::new(false);
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn hwnd_of() -> Option<HWND> {
    let h = WIN.load(Ordering::SeqCst);
    if h == 0 {
        return None;
    }
    let h = HWND(h as *mut core::ffi::c_void);
    unsafe { IsWindow(Some(h)).as_bool().then_some(h) }
}

/// 主窗口那边滚了滚轮、光标却压在这块画布上：把这一下交给画布处理，还回画布吃没吃掉。
/// 滚轮只发给有焦点的那一扇（系统设置里"悬停在窗口上时滚动非活动窗口"关着的话更是如此），
/// 焦点还在主窗口上时画布根本收不到这消息，看着就是"滚了没反应"。
pub fn wheel_here(wparam: WPARAM, lparam: LPARAM) -> bool {
    let Some(hwnd) = hwnd_of() else { return false };
    let sx = (lparam.0 & 0xffff) as u16 as i16 as i32;
    let sy = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    let mut wr = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut wr);
        if sx < wr.left || sx >= wr.right || sy < wr.top || sy >= wr.bottom {
            return false;
        }
        let _ = SendMessageW(hwnd, WM_MOUSEWHEEL, Some(wparam), Some(lparam));
    }
    true
}

fn pw(s: &str) -> &'static [u16] {
    Box::leak(s.encode_utf16().chain([0]).collect::<Vec<u16>>().into_boxed_slice())
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

fn client(hwnd: HWND) -> RECT {
    let mut rc = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    rc
}

unsafe fn register() {
    if REGISTERED.load(Ordering::SeqCst) {
        return;
    }
    REGISTERED.store(true, Ordering::SeqCst);
    let hinst = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
    let (hicon, hicon_sm) = appicon::load();
    let cls = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        // CS_DBLCLKS：右键双击删图那一下得收得到
        style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
        lpfnWndProc: Some(wndproc),
        hInstance: hinst,
        hIcon: hicon,
        hIconSm: hicon_sm,
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
        lpszClassName: PCWSTR(pw(CLASS).as_ptr()),
        ..Default::default()
    };
    if RegisterClassExW(&cls) == 0 {
        REGISTERED.store(false, Ordering::SeqCst);
    }
}

/// 把这张图摆进参考图画布：窗口没开就先开一扇，开着就只加一张。
/// `pick` 只用来挑屏幕——在哪块屏上点的就摆到哪块屏。
pub fn open(path: &str, pick: HWND) {
    let p = path.to_string();
    if hwnd_of().is_none() {
        ensure(pick);
    }
    add_file(&p, None);
    if let Some(h) = hwnd_of() {
        // 命令口跑测试时不抢前台：他那边正打字，前台被这扇抢走算事故
        if !headless() {
            unsafe {
                let _ = SetForegroundWindow(h);
            }
        }
    }
}

/// 建那一扇画布窗口。已经在了就直接还回现有的那扇。
fn ensure(pick: HWND) -> Option<HWND> {
    if let Some(h) = hwnd_of() {
        return Some(h);
    }
    // 无边框 + 可拖边：客户区盖满整窗，标题条自己画，这样背景才透得出去。
    // TOOLWINDOW：不占任务栏那一格，从任务栏上"最小化所有窗口"就只收得到主窗口，
    // 不会把这板对照图一起收走（真被收走了，还有 WM_UNMIN 那趟兜着）。
    let style = WS_POPUP | WS_THICKFRAME | WS_SYSMENU;
    let ex = WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW;
    unsafe {
        register();
        let mon = MonitorFromWindow(pick, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO::default();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let work = if GetMonitorInfoW(mon, &mut mi).as_bool() {
            mi.rcWork
        } else {
            RECT { left: 0, top: 0, right: 1280, bottom: 1024 }
        };
        // 画布默认吃掉这块屏工作区的六成，剩下四成留给他正在用的软件
        let ww = ((work.right - work.left) * 6 / 10).max(480).min(820);
        let wh = ((work.bottom - work.top) * 6 / 10).max(360).min(620);
        let x = work.left + (work.right - work.left - ww) / 2;
        let y = work.top + (work.bottom - work.top - wh) / 2;
        // 命令口跑测试：摆到屏幕外，真画但看不见，也不挡住他手上的活
        let (x, y) = if headless() { (-32000, -32000) } else { (x, y) };
        let t = wide(TITLE);
        let hinst = HINSTANCE(GetModuleHandleW(None).unwrap_or_default().0);
        let hwnd = CreateWindowExW(
            ex,
            PCWSTR(pw(CLASS).as_ptr()),
            PCWSTR(t.as_ptr()),
            style,
            x,
            y,
            ww,
            wh,
            None,
            None,
            Some(hinst),
            None,
        )
        .ok()?;
        // 剪贴板变了由系统发消息过来，不靠自己定时去翻
        let _ = AddClipboardFormatListener(hwnd);
        // 从资源管理器那边把图片拖过来也能进画布
        let _ = DragAcceptFiles(hwnd, true);
        let rc = client(hwnd);
        let mut org = POINT { x: 0, y: 0 };
        let _ = ClientToScreen(hwnd, &mut org);
        let dpi = GetDpiForWindow(hwnd).max(96) as i32;
        let has = clipboard_has_image();
        let face = wide("Segoe UI");
        let font = CreateFontW(
            -(12 * dpi / 96),
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
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
        WIN.store(hwnd.0 as isize, Ordering::SeqCst);
        *state().lock().unwrap() = Some(State {
            cards: Vec::new(),
            panx: 0,
            pany: 0,
            cur: (0, 0),
            row_h: 0,
            drag: None,
            resize: None,
            clip: has,
            rc,
            ox: org.x,
            oy: org.y,
            buf_mem: 0,
            buf_bmp: 0,
            bits: 0,
            buf_w: 0,
            buf_h: 0,
            bg: BG_SOLID,
            strip: strip_h(dpi),
            edge: (6 * dpi / 96).max(4),
            sld: RECT::default(),
            trk: RECT::default(),
            cls: RECT::default(),
            drag_sld: false,
            font: font.0 as isize,
            dpi,
            hover: (0, 0),
            menu: None,
            intro: !intro_seen(),
            intro_btn: RECT::default(),
            intro_hot: false,
        });
        if has {
            set_title(true);
        }
        let _ = ShowWindow(hwnd, SW_SHOW);
        // 分层窗口不会自己画第一面：不推这张面上去，屏上什么都看不见
        push(hwnd);
        Some(hwnd)
    }
}

/// 往画布上加一个文件。`at` 给了（拖放松手那点，客户区坐标）就把这张摆在那儿，
/// 不给就按排图规则往空处叠。已经在画布上的那张不再重复解码、不再摆第二遍。
fn add_file(path: &str, at: Option<(i32, i32)>) {
    let Some(hwnd) = hwnd_of() else { return };
    let Some((nw, nh)) = job::open_size(path).ok().filter(|(w, h)| *w > 0 && *h > 0) else {
        return;
    };
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    {
        let mut g = state().lock().unwrap();
        let Some(s) = g.as_mut() else { return };
        if s.cards.iter().any(|c| c.path == path) {
            return;
        }
        let (mut x, mut y, scale) = slot(s, nw, nh);
        if let Some((ax, ay)) = at {
            // 落点当这张图的中心：人家拖到这儿松的手，要的就是这一处
            let dw = (nw as f64 * scale).round() as i32;
            let dh = (nh as f64 * scale).round() as i32;
            x = ax - dw / 2 - s.panx;
            y = ay - dh / 2 - s.pany;
        }
        s.cards.push(Card { id, path: path.to_string(), view: None, x, y, scale, base: (nw, nh) });
    }
    let hp = hwnd.0 as isize;
    let p = path.to_string();
    std::thread::spawn(move || decode(id, hp, &p));
}

/// 删掉压在这一底下的那一张（叠着的只删最上面）。返回有没有删成。
fn drop_card(s: &mut State, x: i32, y: i32) -> bool {
    match hit(s, x, y) {
        Some(i) => {
            s.cards.remove(i);
            true
        }
        None => false,
    }
}

/// 剪贴板此刻有没有能贴进画布的东西：截图那种位图，或者复制的图片文件。
fn clip_ready() -> bool {
    clipboard_has_image()
        || crate::clip::clipboard_paths()
            .0
            .iter()
            .any(|p| crate::scan::is_image(p))
}

/// 剪贴板里那张贴进画布。位图先试（截图工具给的走这条），再试文件列表——
/// 从资源管理器复制一张图片文件，系统剪贴板里给的是路径，那也是他要的"贴进来"。
fn paste_clip() {
    if hwnd_of().is_none() {
        return;
    }
    if let Some(v) = clip_view() {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let base = (v.w, v.h);
        {
            let mut g = state().lock().unwrap();
            let Some(s) = g.as_mut() else { return };
            let (x, y, scale) = slot(s, base.0, base.1);
            s.cards.push(Card { id, path: String::new(), view: Some(v), x, y, scale, base });
        }
        kick();
        return;
    }
    let (paths, _) = crate::clip::clipboard_paths();
    for p in paths.iter().filter(|p| crate::scan::is_image(p)) {
        add_file(p, None);
    }
}

/// 弹层里点下去的那一项。`card` 是这一右键压在上的那一张。
unsafe fn menu_act(hwnd: HWND, id: usize, card: Option<usize>) {
    match id {
        MI_PASTE => paste_clip(),
        MI_COPY => {
            let Some(want) = card else { return };
            let mut path = String::new();
            let mut done = false;
            {
                let g = state().lock().unwrap();
                if let Some(c) = g.as_ref().and_then(|s| s.cards.iter().find(|c| c.id == want)) {
                    // 画布上手上那份像素优先：现成的，点一下就写完，不用再等一次全尺寸解码。
                    // 只有还没解回来的那张才回头读文件。
                    if let Some(v) = &c.view {
                        let _ = crate::clip::copy_view(v);
                        done = true;
                    } else {
                        path = c.path.clone();
                    }
                }
            }
            if !done && !path.is_empty() {
                let _ = crate::clip::copy_image(&path);
            }
        }
        MI_DEL => {
            let Some(want) = card else { return };
            {
                let mut g = state().lock().unwrap();
                let Some(s) = g.as_mut() else { return };
                if let Some(p) = s.cards.iter().position(|c| c.id == want) {
                    s.cards.remove(p);
                }
            }
            push(hwnd);
        }
        _ => {}
    }
}

/// 这一张落在画布哪儿、按视口该铺多大。顶上让开标题条那一条。
fn slot(s: &mut State, bw: u32, bh: u32) -> (i32, i32, f64) {
    let vw = (s.rc.right - s.rc.left).max(1);
    let vh = (s.rc.bottom - s.rc.top - s.strip).max(1);
    let max_w = (vw as f64 * OCCUPY).max(64.0);
    let max_h = (vh as f64 * OCCUPY).max(64.0);
    let scale = (max_w / bw.max(1) as f64)
        .min(max_h / bh.max(1) as f64)
        .min(1.0)
        .clamp(SCALE_MIN, SCALE_MAX);
    let dw = (bw.max(1) as f64 * scale).round() as i32;
    let dh = (bh.max(1) as f64 * scale).round() as i32;
    let mut nx = s.cur.0;
    let mut ny = s.cur.1;
    if nx + dw > vw - PAD {
        nx = 0;
        ny += s.row_h + PAD;
        s.row_h = 0;
    }
    if ny + dh > vh - PAD {
        // 排到底了就回到视口左上接着摆，挤不挤由他自己拖开
        nx = 0;
        ny = 0;
        s.row_h = 0;
    }
    s.cur = (nx + dw + PAD, ny);
    s.row_h = s.row_h.max(dh);
    (-s.panx + PAD + nx, -s.pany + s.strip + PAD + ny, scale)
}

/// 后台解码。全尺寸出图，只在像素数超线时等比压一档。
fn decode(id: usize, hp: isize, path: &str) {
    let hwnd = HWND(hp as *mut core::ffi::c_void);
    job::init_com();
    let (nw, nh) = job::open_size(path).unwrap_or((0, 0));
    let (bw, bh) = if nw == 0 || nh == 0 {
        (0u32, 0u32)
    } else {
        let over = (nw as u64 * nh as u64) as f64 / MAX_PIX as f64;
        if over > 1.0 {
            let k = over.sqrt();
            (
                ((nw as f64 / k).round() as u32).max(1),
                ((nh as f64 / k).round() as u32).max(1),
            )
        } else {
            (nw, nh)
        }
    };
    let Some(v) = job::decode_view(path, bw, bh).ok() else { return };
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return;
    }
    {
        let mut g = state().lock().unwrap();
        let Some(s) = g.as_mut() else { return };
        let Some(c) = s.cards.iter_mut().find(|c| c.id == id) else { return };
        if c.base.0 > 0 && v.w > 0 {
            c.scale = (c.scale * c.base.0 as f64 / v.w as f64).clamp(SCALE_MIN, SCALE_MAX);
            c.base = (v.w, v.h);
        }
        c.view = Some(v);
    }
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_IMG, WPARAM(0), LPARAM(0));
    }
}

/// 一张图在屏幕上占的那块
fn on_screen(c: &Card, s: &State) -> Option<RECT> {
    let v = c.view.as_ref()?;
    let w = (v.w.max(1) as f64 * c.scale).round() as i32;
    let h = (v.h.max(1) as f64 * c.scale).round() as i32;
    Some(RECT {
        left: c.x + s.panx,
        top: c.y + s.pany,
        right: c.x + s.panx + w,
        bottom: c.y + s.pany + h,
    })
}

fn over(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

/// 命中最上面那张：后摆的压先摆的，所以从尾巴往前找，找到的是卡片自己的下标
fn hit(s: &State, x: i32, y: i32) -> Option<usize> {
    let n = s.cards.len();
    s.cards
        .iter()
        .rev()
        .position(|c| on_screen(c, s).map(|r| over(&r, x, y)).unwrap_or(false))
        .map(|i| n - 1 - i)
}

/// 指针压在哪张图的哪条边上。只认最上面那张；太小的图不给边，
/// 中间要留得下拖整张的空当。
fn grip_at(s: &State, x: i32, y: i32) -> Option<(usize, Grip)> {
    let i = hit(s, x, y)?;
    let r = on_screen(&s.cards[i], s)?;
    let g = s.edge.max(4);
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let l = x - r.left < g && w > g * 2 + 12;
    let rr = r.right - x <= g && w > g * 2 + 12;
    let t = y - r.top < g && h > g * 2 + 12;
    let b = r.bottom - y <= g && h > g * 2 + 12;
    let grip = match (l, rr, t, b) {
        (true, _, true, _) => Grip::Nw,
        (_, true, true, _) => Grip::Ne,
        (true, _, _, true) => Grip::Sw,
        (_, true, _, true) => Grip::Se,
        (true, _, _, _) => Grip::W,
        (_, true, _, _) => Grip::E,
        (_, _, true, _) => Grip::N,
        (_, _, _, true) => Grip::S,
        _ => return None,
    };
    Some((i, grip))
}

/// 那条边该配哪种双向箭头。
fn grip_cursor(g: Grip) -> PCWSTR {
    match g {
        Grip::Nw | Grip::Se => IDC_SIZENWSE,
        Grip::Ne | Grip::Sw => IDC_SIZENESW,
        Grip::W | Grip::E => IDC_SIZEWE,
        Grip::N | Grip::S => IDC_SIZENS,
    }
}

/// 拖边那一趟：按抓住的那条边算新倍率，对边（或对角）钉住不动，
/// 宽高始终按原图比例一起走。每一步都从按下那一刻重算，不累加。
fn apply_resize(s: &mut State, rz: Resize, mx: i32, my: i32) {
    let (px, py) = (s.panx, s.pany);
    let l = (rz.x0 + px) as f64;
    let t = (rz.y0 + py) as f64;
    let w0 = rz.w0.max(1) as f64;
    let h0 = rz.h0.max(1) as f64;
    let r = l + w0;
    let b = t + h0;
    let mx = mx as f64;
    let my = my as f64;
    // k = 新尺寸相对按下那一刻的倍数。角上取两个方向里幅度大的那个，
    // 指针往哪个方向拉都跟手。
    let k = match rz.grip {
        Grip::E => (mx - l) / w0,
        Grip::W => (r - mx) / w0,
        Grip::S => (my - t) / h0,
        Grip::N => (b - my) / h0,
        Grip::Se => ((mx - l) / w0).max((my - t) / h0),
        Grip::Sw => ((r - mx) / w0).max((my - t) / h0),
        Grip::Ne => ((mx - l) / w0).max((b - my) / h0),
        Grip::Nw => ((r - mx) / w0).max((b - my) / h0),
    };
    let ns = (rz.scale0 * k).clamp(SCALE_MIN, SCALE_MAX);
    let k = ns / rz.scale0;
    let nw = w0 * k;
    let nh = h0 * k;
    let (nl, nt) = match rz.grip {
        Grip::E => (l, (t + b - nh) / 2.0),
        Grip::W => (r - nw, (t + b - nh) / 2.0),
        Grip::S => ((l + r - nw) / 2.0, t),
        Grip::N => ((l + r - nw) / 2.0, b - nh),
        Grip::Se => (l, t),
        Grip::Sw => (r - nw, t),
        Grip::Ne => (l, b - nh),
        Grip::Nw => (r - nw, b - nh),
    };
    if let Some(c) = s.cards.iter_mut().find(|c| c.id == rz.id) {
        c.x = nl.round() as i32 - px;
        c.y = nt.round() as i32 - py;
        c.scale = ns;
    }
}

/// 整块画布一起缩放：所有卡片按同一倍率放大/缩小，`(x, y)` 那一点压着的内容缩完还压着。
/// 还回这一下有没有真动。
///
/// 有一张已经到倍率上限/下限时，把整档倍率往 1 那边压回去，而不是只缩它以外的那些：
/// 一半动一半不动的话，图与图之间摆好的位置就散了。
fn zoom_all(s: &mut State, x: i32, y: i32, f: f64) -> bool {
    let mut k = f;
    for c in &s.cards {
        if c.scale <= 0.0 {
            continue;
        }
        let ns = c.scale * f;
        if ns > SCALE_MAX {
            k = k.min(SCALE_MAX / c.scale);
        } else if ns < SCALE_MIN {
            k = k.max(SCALE_MIN / c.scale);
        }
    }
    if (k - 1.0).abs() < 1e-9 {
        return false;
    }
    let (px, py) = (s.panx, s.pany);
    for c in &mut s.cards {
        let l = c.x + px;
        let t = c.y + py;
        c.scale = (c.scale * k).clamp(SCALE_MIN, SCALE_MAX);
        c.x = (x as f64 - (x - l) as f64 * k).round() as i32 - px;
        c.y = (y as f64 - (y - t) as f64 * k).round() as i32 - py;
    }
    true
}

/// 画布内容有变，重推一面。分层窗口没有 WM_PAINT 可等，改完就得自己交上去。
fn kick() {
    if let Some(h) = hwnd_of() {
        unsafe {
            push(h);
        }
    }
}

/// 窗口标题（任务栏和 EnumWindows 找得到那一份）。画在标题条上的那行字
/// 跟着 s.clip 走，不靠这句。
fn set_title(clip: bool) {
    let Some(h) = hwnd_of() else { return };
    let t = wide(if clip { TITLE_CLIP } else { TITLE });
    unsafe {
        let _ = SetWindowTextW(h, PCWSTR(t.as_ptr()));
    }
}

/// 说明卡看过没有。读不出来（没这条、打不开键）就按没看过算 —— 宁可多讲一遍。
fn intro_seen() -> bool {
    crate::theme::ui_get(INTRO_KEY).is_some_and(|v| v != "0")
}

/// 记下"说明卡看过了"。主窗口那份偏好是全量写自己名单的，不删名单外的值，
/// 这一条和它们放在一块不会互相冲掉。
fn intro_done() {
    crate::theme::ui_put(INTRO_KEY, Some("1"));
}

/// 收起说明卡，重推一面。画布上的图一张都不动。
unsafe fn close_intro(hwnd: HWND) {
    let had = {
        let mut g = state().lock().unwrap();
        match g.as_mut() {
            Some(s) => std::mem::replace(&mut s.intro, false),
            None => false,
        }
    };
    if !had {
        return;
    }
    intro_done();
    push(hwnd);
}

fn clipboard_has_image() -> bool {
    unsafe {
        IsClipboardFormatAvailable(CF_DIB.0 as u32).is_ok()
            || IsClipboardFormatAvailable(CF_BITMAP.0 as u32).is_ok()
    }
}

/// 剪贴板里的位图取成画布用得上的 BGRA。
fn clip_view() -> Option<View> {
    unsafe {
        OpenClipboard(None).ok()?;
        // 绝大多数来源都给 CF_DIB；只给 GDI 位图那类老规矩的用第二条路兜住，
        // 不然是标题栏写着"有图片"、右键却什么也不来。
        let r = read_clip_dib().or(read_clip_bitmap());
        let _ = CloseClipboard();
        r
    }
}

/// 得在剪贴板开着的时候读，锁要在 CloseClipboard 之前解
unsafe fn read_clip_dib() -> Option<View> {
    let h = GetClipboardData(CF_DIB.0 as u32).ok()?;
    let hg = HGLOBAL(h.0);
    let size = GlobalSize(hg);
    if size < 40 {
        return None;
    }
    let p = GlobalLock(hg) as *const u8;
    if p.is_null() {
        return None;
    }
    let r = dib_to_view(std::slice::from_raw_parts(p, size));
    let _ = GlobalUnlock(hg);
    r
}

/// CF_BITMAP：让 GDI 把它拷成 32 位 BGRA
unsafe fn read_clip_bitmap() -> Option<View> {
    let h = GetClipboardData(CF_BITMAP.0 as u32).ok()?;
    let hb = HBITMAP(h.0);
    let mut bm = BITMAP::default();
    let got = GetObjectW(
        HGDIOBJ(hb.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut BITMAP as *mut core::ffi::c_void),
    );
    if got == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
        return None;
    }
    let (w, hh) = (bm.bmWidth, bm.bmHeight);
    if w > 20000 || hh > 20000 {
        return None;
    }
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            // 负数 = 顶朝下，拿回来的行序才和画布一致
            biHeight: -hh,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut px = vec![0u8; (w as usize) * (hh as usize) * 4];
    let hdc = GetDC(None);
    let n = GetDIBits(hdc, hb, 0, hh as u32, Some(px.as_mut_ptr() as *mut core::ffi::c_void), &mut bmi, DIB_RGB_COLORS);
    let _ = ReleaseDC(None, hdc);
    if n == 0 {
        return None;
    }
    // GDI 位图那一路 alpha 全空，不填满是画到黑底上和没画一样
    for p in px.chunks_exact_mut(4) {
        p[3] = 255;
    }
    Some(View { w: w as u32, h: hh as u32, nw: w as u32, nh: hh as u32, px })
}

/// 头里那几个字段按小端手动取，不拿指针去猜对齐
fn u32_at(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn i32_at(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn u16_at(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(b[off..off + 2].try_into().unwrap())
}

/// 只吃 24/32 位那份；调色板图和 16 位的不接（截图工具给的都在这一档里）
fn dib_to_view(b: &[u8]) -> Option<View> {
    let hdr = u32_at(b, 0) as usize;
    let w = i32_at(b, 4);
    let hraw = i32_at(b, 8);
    let bits = u16_at(b, 14) as usize;
    let comp = u32_at(b, 16);
    if w <= 0 || hraw == 0 || w > 20000 || hraw.abs() > 20000 {
        return None;
    }
    if bits != 24 && bits != 32 {
        return None;
    }
    // BI_RGB(0) 和 BI_BITFIELDS(3)：32 位那份就是 BGRA，掩码不再单独认
    if comp != 0 && comp != 3 {
        return None;
    }
    let h = hraw.unsigned_abs() as usize;
    let w = w as usize;
    let stride = (w * bits / 8 + 3) & !3;
    let pal = hdr;
    let need = pal + stride * h;
    if b.len() < need || hdr < 40 {
        return None;
    }
    let mut px = vec![0u8; w * h * 4];
    for row in 0..h {
        // biHeight 为正 = 底朝上，倒着读才对
        let src = if hraw > 0 { h - 1 - row } else { row };
        let s = &b[pal + src * stride..pal + src * stride + stride];
        let d = &mut px[row * w * 4..row * w * 4 + w * 4];
        if bits == 32 {
            d.copy_from_slice(&s[..w * 4]);
            for p in d.chunks_exact_mut(4) {
                p[3] = 255;
            }
        } else {
            for x in 0..w {
                let q = x * 3;
                d[x * 4] = s[q];
                d[x * 4 + 1] = s[q + 1];
                d[x * 4 + 2] = s[q + 2];
                d[x * 4 + 3] = 255;
            }
        }
    }
    Some(View { w: w as u32, h: h as u32, nw: w as u32, nh: h as u32, px })
}

/// 整面重画一次，交给系统合成。分层窗口不发 WM_PAINT，看到的和透出去的都出自这张面。
///
/// 底的 alpha 由标题条上那个滑块定，图片那一片一律钉成不透 —— 他要的是"调画布
/// 的透明度"，不是"把参考图也一起调淡"。
unsafe fn push(hwnd: HWND) {
    let rc = client(hwnd);
    let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
    if w <= 0 || h <= 0 {
        return;
    }
    let mut org = POINT { x: 0, y: 0 };
    let _ = ClientToScreen(hwnd, &mut org);
    let screen = GetDC(Some(hwnd));
    let mut g = state().lock().unwrap();
    let Some(s) = g.as_mut() else {
        let _ = ReleaseDC(Some(hwnd), screen);
        return;
    };
    if !ensure_surf(s, screen, w, h) {
        // 面建不出来（GDI 资源枯竭那一类）：这一下不重画，留着上一面继续用
        let _ = ReleaseDC(Some(hwnd), screen);
        return;
    }
    s.ox = org.x;
    s.oy = org.y;
    s.rc = rc;
    let hdc = HDC(s.buf_mem as *mut core::ffi::c_void);
    let px = s.bits as *mut u32;
    if px.is_null() {
        let _ = ReleaseDC(Some(hwnd), screen);
        return;
    }
    // 底：纯黑 + 那一档不透明度。alpha 有下限、不留 0，不然这块点不着。
    let n = (w as usize) * (h as usize);
    std::slice::from_raw_parts_mut(px, n).fill((s.bg as u32) << 24);
    let mut boxes: Vec<RECT> = Vec::with_capacity(s.cards.len());
    for c in &s.cards {
        let (Some(v), Some(d)) = (&c.view, on_screen(c, s)) else { continue };
        dib(hdc, &d, v);
        boxes.push(d);
    }
    let _ = GdiFlush();
    // GDI 那条路对 alpha 那四个字节各版本走法不一致，图片写完再逐张钉回不透。
    // 带 alpha 的 PNG 也走这一句：它的透明像素照样不许透到背景那层去。
    for d in &boxes {
        opaque(px, w, h, d);
    }
    paint_strip(hdc, s, px, w, h);
    paint_menu(hdc, s, px, w, h);
    paint_intro(hdc, s, px, w, h);
    let _ = GdiFlush();
    let size = SIZE { cx: w, cy: h };
    let pos = POINT { x: org.x, y: org.y };
    let src = POINT { x: 0, y: 0 };
    let bf = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        // 255 = 不整面统一压档，逐像素都按面上各自那份 alpha 合成
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    let _ = UpdateLayeredWindow(
        hwnd,
        Some(screen),
        Some(&pos),
        Some(&size),
        Some(hdc),
        Some(&src),
        COLORREF(0),
        Some(&bf),
        ULW_ALPHA,
    );
    let _ = ReleaseDC(Some(hwnd), screen);
}

/// 要用的那张后备面。尺寸没变就用现成的，变了才重开一张。
/// 必须是 32 位的 DIB section：既要能拿 GDI 往上面贴图，又要能自己逐像素写 alpha。
unsafe fn ensure_surf(s: &mut State, screen: HDC, w: i32, h: i32) -> bool {
    if s.buf_mem != 0 && s.buf_bmp != 0 && s.bits != 0 && s.buf_w == w && s.buf_h == h {
        return true;
    }
    free_surf(s);
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            // 负数 = 顶朝下，像素行序和客户区 y 一行对一行
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let Ok(bmp) = CreateDIBSection(Some(screen), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
        return false;
    };
    if bits.is_null() {
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        return false;
    }
    let mem = CreateCompatibleDC(Some(screen));
    if mem.0.is_null() {
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        return false;
    }
    let _ = SelectObject(mem, HGDIOBJ(bmp.0));
    s.buf_mem = mem.0 as isize;
    s.buf_bmp = bmp.0 as isize;
    s.bits = bits as isize;
    s.buf_w = w;
    s.buf_h = h;
    true
}

unsafe fn free_surf(s: &mut State) {
    if s.buf_mem != 0 {
        let _ = DeleteDC(HDC(s.buf_mem as *mut core::ffi::c_void));
        s.buf_mem = 0;
    }
    if s.buf_bmp != 0 {
        let _ = DeleteObject(HGDIOBJ(s.buf_bmp as *mut core::ffi::c_void));
        s.buf_bmp = 0;
    }
    s.bits = 0;
    s.buf_w = 0;
    s.buf_h = 0;
}

/// 往后备面上直接写一块实色：连 alpha 一起写。GDI 的 FillRect 只给 RGB，
/// alpha 那四个字节它抹成 0，那一块就跟着背景一起透出去。
/// 传进来的颜色是主题那套 COLORREF（alpha 字节天生是 0），这一句负责补成不透：
/// 标题条是操作件，透了就等于既看不见也按不住。
unsafe fn put(px: *mut u32, w: i32, h: i32, r: &RECT, argb: u32) {
    if px.is_null() {
        return;
    }
    let argb = argb | 0xFF00_0000;
    let (x0, y0) = (r.left.max(0), r.top.max(0));
    let (x1, y1) = (r.right.min(w), r.bottom.min(h));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let row = (x1 - x0) as usize;
    for y in y0..y1 {
        let p = px.add(y as usize * w as usize + x0 as usize);
        std::slice::from_raw_parts_mut(p, row).fill(argb);
    }
}

/// 在分层窗口的后备面上盖一层半透明灰（预乘合成，跟 UpdateLayeredWindow 那套一致）。
/// 图片那种 alpha=255 的地方就是普通插值；背景那条 RGB 是 0，按预乘补上。
unsafe fn veil_px(px: *mut u32, w: i32, h: i32, r: &RECT, col: u32, alpha: u8) {
    if px.is_null() || alpha == 0 {
        return;
    }
    let (x0, y0) = (r.left.max(0), r.top.max(0));
    let (x1, y1) = (r.right.min(w), r.bottom.min(h));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let a = alpha as u32;
    let ia = 255 - a;
    let sr = ((col >> 16) & 0xff) * a / 255;
    let sg = ((col >> 8) & 0xff) * a / 255;
    let sb = (col & 0xff) * a / 255;
    for y in y0..y1 {
        let p = px.add(y as usize * w as usize + x0 as usize);
        for i in 0..(x1 - x0) as usize {
            let d = *p.add(i);
            let da = d >> 24;
            let dr = ((d >> 16) & 0xff) * ia / 255;
            let dg = ((d >> 8) & 0xff) * ia / 255;
            let db = (d & 0xff) * ia / 255;
            let na = (a + da * ia / 255).min(255);
            *p.add(i) = (na << 24) | ((sr + dr) << 16) | ((sg + dg) << 8) | (sb + db);
        }
    }
}

/// 图片占的那一片钉成不透。写在 GDI 贴完之后：不管那一路把 alpha 落成了什么，
/// 底透多少都不该透到图片身上。
unsafe fn opaque(px: *mut u32, w: i32, h: i32, r: &RECT) {
    if px.is_null() {
        return;
    }
    let (x0, y0) = (r.left.max(0), r.top.max(0));
    let (x1, y1) = (r.right.min(w), r.bottom.min(h));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    for y in y0..y1 {
        let p = px.add(y as usize * w as usize + x0 as usize);
        for i in 0..(x1 - x0) as usize {
            *p.add(i) |= 0xFF00_0000;
        }
    }
}

/// 滑块那一点折成背景的不透明度：最左实心，最右最透。
fn alpha_at(trk: &RECT, x: i32) -> u8 {
    let w = (trk.right - trk.left).max(1) as f64;
    let f = ((x - trk.left) as f64 / w).clamp(0.0, 1.0);
    (BG_SOLID as f64 - (BG_SOLID - BG_CLEAR) as f64 * f).round() as u8
}

/// 顶上那一条：底、名字、透明度滑块、关闭。整条 alpha 拉满 —— 它是操作件，
/// 跟着背景一起透就按不住了。取色、条高、关闭那颗的几何和悬停画法全部照主窗口
/// 那一条走（`strip_of` 那份就是主窗口画它自己那条时用的同一份）。
unsafe fn paint_strip(hdc: HDC, s: &mut State, px: *mut u32, w: i32, h: i32) {
    use crate::theme as th;
    let st = strip_of(s.dpi);
    let sh = st.h.max(12);
    s.strip = sh;
    let k = |v: i32| v * s.dpi / 96;
    let top = RECT { left: 0, top: 0, right: w, bottom: sh };
    // 底：整条铺一块纯色。这块不能交给 GDI 的填充 —— 它只写 RGB，
    // alpha 那字节抹成 0，整条就跟着背景一起透出去了。
    put(px, w, h, &top, st.bar);
    put(px, w, h, &RECT { left: 0, top: sh - 1, right: w, bottom: sh }, st.edge);
    let f = HFONT(s.font as *mut core::ffi::c_void);
    SelectObject(hdc, HGDIOBJ(f.0));
    let _ = SetBkMode(hdc, TRANSPARENT);
    // 从右往左排：关闭、滑块、"透明"那两个字，剩下的都给名字。
    // 关闭那颗用主窗口那三颗的几何：离右缘留 pad、条内居中，不再顶满一角。
    let mid = sh / 2;
    let cy = (sh - st.bh) / 2;
    s.cls = RECT { left: w - st.pad - st.bw, top: cy, right: w - st.pad, bottom: cy + st.bh };
    let tw = k(120);
    let tr_left = (s.cls.left - k(12) - tw).max(k(120));
    let tt = (k(3)).max(2);
    s.trk = RECT { left: tr_left, top: mid - tt / 2, right: tr_left + tw, bottom: mid - tt / 2 + tt };
    s.sld = RECT { left: tr_left - k(6), top: 0, right: s.cls.left - k(4), bottom: sh };
    let lab_w = k(28);
    // 轨道、走过的这一截、滑块头不再各定一枚灰：全都从底色往字色方向掺出来，
    // 皮肤一换它们自己跟着换，深浅也始终咬得住在同一档上。
    put(px, w, h, &s.trk, th::mix(st.bar, st.fg, 0.28));
    let pos = (BG_SOLID - s.bg) as f64 / (BG_SOLID - BG_CLEAR) as f64;
    let kx = (s.trk.left as f64 + pos * (s.trk.right - s.trk.left - 1) as f64).round() as i32;
    put(
        px,
        w,
        h,
        &RECT { left: s.trk.left, top: s.trk.top, right: kx, bottom: s.trk.bottom },
        th::mix(st.bar, st.fg, 0.72),
    );
    // 头：一枚小圆点压在轨道正中。原来那截 13×9 的方块比字还抢眼。
    let kr = (k(4)).max(3);
    block(px, w, h, &RECT { left: kx - kr, top: mid - kr, right: kx + kr + 1, bottom: mid + kr + 1 }, kr - 1, st.fg);
    // 悬停只落在关闭键上：鼠标跨过那一格就要重推一面，别的一路都不管
    let hot = over(&s.cls, s.hover.0, s.hover.1);
    if hot {
        if st.tag == th::TAG_FINE {
            let hr = RECT { left: s.cls.left + 2, top: s.cls.top + 2, right: s.cls.right - 2, bottom: s.cls.bottom - 2 };
            let rad = (k(4)).max(2);
            block(px, w, h, &hr, rad, st.hot);
            ring(px, w, h, &hr, rad, th::mix(st.bar, st.fg, 0.45));
        } else {
            put(px, w, h, &s.cls, st.hot);
            if st.tag == th::TAG_PLAIN {
                ring(px, w, h, &s.cls, 0, st.ring);
            }
        }
    }
    let _ = SetTextColor(hdc, th::cref(st.dim));
    let mut u: Vec<u16> = "透明".encode_utf16().collect();
    let mut lr = RECT { left: tr_left - k(10) - lab_w, top: 0, right: tr_left - k(10), bottom: sh };
    let _ = DrawTextW(hdc, &mut u, &mut lr, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    let _ = SetTextColor(hdc, th::cref(st.ink));
    let mut u: Vec<u16> = if s.clip { TITLE_CLIP } else { TITLE }.encode_utf16().collect();
    let mut nr = RECT {
        left: k(12),
        top: 0,
        right: (tr_left - k(14) - lab_w).max(k(12)),
        bottom: sh,
    };
    let _ = DrawTextW(hdc, &mut u, &mut nr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
    // ✕ 那颗：平阶以上画两条 1px 对角线（和主窗口那三颗同一个画法，换字体不会换粗细），
    // 最素那一档仍用字形。
    let col = if hot { st.fg } else { st.dim };
    if st.tag == th::TAG_PLAIN {
        let mut u: Vec<u16> = "✕".encode_utf16().collect();
        let mut cr = s.cls;
        let _ = SetTextColor(hdc, th::cref(col));
        let _ = DrawTextW(hdc, &mut u, &mut cr, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    } else {
        let (cx, cyy) = ((s.cls.left + s.cls.right) / 2, (s.cls.top + s.cls.bottom) / 2);
        xmark_px(px, w, h, cx, cyy, (k(5)).max(3), col);
    }
    // 上面那些字是 GDI 写的，alpha 落成了什么都可能；整条钉回不透。
    opaque(px, w, h, &top);
}

/// 用当前选中的那支笔量一行字有多宽。量字和画字同一支笔，才不会量窄了裁字。
unsafe fn text_w(hdc: HDC, t: &str) -> i32 {
    let mut u: Vec<u16> = t.encode_utf16().collect();
    let mut r = RECT { left: 0, top: 0, right: 4000, bottom: 4000 };
    let _ = DrawTextW(hdc, &mut u, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
    r.right
}

/// 右键弹层。设计就是系统菜单那一套，尺寸读主窗口菜单同一组令牌
/// （`theme::MENU_*`）：中性面板底、一圈 accent 细框、左缘留出图标列、
/// 快捷键贴右对齐、点不动的那行只把字压灰不铺底；只有悬停那一行铺
/// 标题栏那一档底色。底色和字色由宿主画标题栏时喂进来的 `Face` 出，
/// 皮肤一换整套跟着换。
unsafe fn paint_menu(hdc: HDC, s: &mut State, px: *mut u32, w: i32, h: i32) {
    use crate::theme as th;
    let (mx, my, card, hot, mw0) = match s.menu.as_ref() {
        Some(m) => (m.x, m.y, m.card, m.hot, m.mw),
        None => return,
    };
    let paste = s.menu.as_ref().is_some_and(|m| m.paste);
    let st = strip_of(s.dpi);
    let fc = face_of(s.dpi);
    let k = s.dpi as f64 / 96.0;
    // (那一行的字, 快捷键, 编号, 上面要不要一道分隔线)
    let items = [
        ("复制图像", "Ctrl+C", MI_COPY, false),
        ("粘贴剪贴板图片", "Ctrl+V", MI_PASTE, false),
        ("删除这张", "Del", MI_DEL, true),
    ];
    let rh = fc.menu_row.max(1);
    let sep_h = (th::MENU_SEP_H * k) as i32;
    let seps = items.iter().filter(|(_, _, _, sp)| *sp).count() as i32;
    let padx = th::menu_text_x(k);
    let pr = (th::MENU_RIGHT * k) as i32;
    let inset = (th::MENU_INSET * k) as i32;
    let f = HFONT(s.font as *mut core::ffi::c_void);
    SelectObject(hdc, HGDIOBJ(f.0));
    let _ = SetBkMode(hdc, TRANSPARENT);
    // 盘子宽度按最宽那行量一次就存下来：菜单开着时鼠标每跨过一行都要推一面，
    // 不能每帧回头把字再量一遍。
    let (mut lw, mut kw) = (0i32, 0i32);
    if mw0 == 0 {
        for (t, key, _, _) in items {
            lw = lw.max(text_w(hdc, t));
            kw = kw.max(text_w(hdc, key));
        }
    }
    let mw = if mw0 > 0 { mw0 } else { th::menu_width(k, lw, kw) };
    let mh = rh * items.len() as i32 + sep_h * seps + (th::MENU_INSET_Y * k) as i32;
    // 贴着右边或下边右键时往回收，这块画布比菜单大不了多少的时候也不能出去
    let x = mx.clamp(2, (w - mw - 2).max(2));
    let y = my.clamp(s.strip + 2, (h - mh - 2).max(s.strip + 2));
    let panel = RECT { left: x, top: y, right: x + mw, bottom: y + mh };
    // 跟主窗口那几类弹层同一套投法：同尺寸灰纱往右下让 6 像素，同一处按 50% 混，
    // 底下压着的图透得出来。
    let off = (4.0 * k) as i32 + 1;
    let sh = RECT {
        left: panel.left + off,
        top: panel.top + off,
        right: panel.right + off,
        bottom: panel.bottom + off,
    };
    veil_px(px, w, h, &sh, 0x009E9E9E, 128);
    put(px, w, h, &panel, fc.popup);
    let mut rows: Vec<(usize, RECT)> = Vec::with_capacity(items.len());
    let mut cy = y + inset;
    for (i, (t, key, id, sp)) in items.into_iter().enumerate() {
        if sp {
            let spad = (th::MENU_SEP_PAD * k) as i32;
            let mid = cy + sep_h / 2;
            put(
                px,
                w,
                h,
                &RECT { left: x + spad, top: mid, right: x + mw - spad, bottom: mid + 1 },
                fc.sep,
            );
            cy += sep_h;
        }
        let r = RECT { left: x, top: cy, right: x + mw, bottom: cy + rh };
        cy += rh;
        let on = menu_on(id, card, paste);
        let picked = on && hot == i as i32;
        if picked {
            put(px, w, h, &r, st.bar);
        }
        let col = if !on {
            fc.off
        } else if picked {
            st.fg
        } else {
            fc.ink
        };
        let _ = SetTextColor(hdc, th::cref(col));
        let mut tr = RECT { left: x + padx, top: r.top, right: r.right - pr, bottom: r.bottom };
        let mut u: Vec<u16> = t.encode_utf16().collect();
        let _ = DrawTextW(hdc, &mut u, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        // 快捷键贴右那一列，和主窗口弹层同一条线
        let _ = SetTextColor(hdc, th::cref(if picked { st.dim } else { fc.dim }));
        let mut kr = RECT {
            left: r.right - pr - (th::MENU_KEY_MAX * k) as i32,
            top: r.top,
            right: r.right - pr,
            bottom: r.bottom,
        };
        let mut v: Vec<u16> = key.encode_utf16().collect();
        let _ = DrawTextW(hdc, &mut v, &mut kr, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        rows.push((id, r));
    }
    // 一圈 1 像素实框：accent，和主窗口弹层同一色、同一做法
    for r in [
        RECT { left: x, top: y, right: x + mw, bottom: y + 1 },
        RECT { left: x, top: y + mh - 1, right: x + mw, bottom: y + mh },
        RECT { left: x, top: y, right: x + 1, bottom: y + mh },
        RECT { left: x + mw - 1, top: y, right: x + mw, bottom: y + mh },
    ] {
        put(px, w, h, &r, st.bar);
    }
    // 框和字都画完了：这一整块钉成不透，不然半透的背景会从菜单底下透出来糊掉字
    opaque(px, w, h, &panel);
    if let Some(m) = s.menu.as_mut() {
        m.rows = rows;
        m.mw = mw;
    }
}

/// 第一次开画布时压在面上的使用说明卡。卡片里的字全部按宽度自然折行，
/// 面板高度照量出来的定；窗口太矮时少画几段，按钮始终留在底下。
unsafe fn paint_intro(hdc: HDC, s: &mut State, px: *mut u32, w: i32, h: i32) {
    use crate::theme as th;
    if !s.intro {
        return;
    }
    let face = face_of(s.dpi);
    let k = |v: i32| v * s.dpi / 96;
    let f = HFONT(s.font as *mut core::ffi::c_void);
    SelectObject(hdc, HGDIOBJ(f.0));
    let _ = SetBkMode(hdc, TRANSPARENT);

    // 标题条以下压一层黑纱：底下的图退到背景里，说明字才读得清。
    // 标题条那一条不动，关窗和拖窗照常按得着。
    veil_px(px, w, h, &RECT { left: 0, top: s.strip, right: w, bottom: h }, 0x00000000, 128);

    let padx = k(22);
    let pady = k(18);
    let gap = k(10);
    let title_h = k(24);
    let btn_w = k(96);
    let btn_h = k(30);
    let margin = k(24);
    let top0 = s.strip + k(8);
    let cw = k(660).min((w - margin * 2).max(k(300)));
    let body_w = (cw - padx * 2).max(k(120));

    // 按正文宽把每段量一遍，面板高度照量出来的定，不做滚动条。
    let mut hs: Vec<(i32, i32)> = Vec::with_capacity(INTRO.len());
    let mut content = title_h + k(12);
    for (head, body) in INTRO {
        let mut u: Vec<u16> = head.encode_utf16().collect();
        let mut r = RECT { left: 0, top: 0, right: body_w, bottom: 4000 };
        let _ = DrawTextW(hdc, &mut u, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
        let hh = (r.bottom - r.top).max(k(16));
        let mut u: Vec<u16> = body.encode_utf16().collect();
        let mut r = RECT { left: 0, top: 0, right: body_w, bottom: 4000 };
        let _ = DrawTextW(hdc, &mut u, &mut r, DT_CALCRECT | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX);
        let bh = (r.bottom - r.top).max(k(16));
        hs.push((hh, bh));
        content += hh + k(3) + bh + gap;
    }
    content += k(6) + btn_h;
    let max_h = (h - top0 - margin).max(k(120));
    let ch = (content + pady * 2).min(max_h);
    let cx = (w - cw) / 2;
    let cy = top0 + ((h - margin - top0 - ch) / 2).max(0);
    let panel = RECT { left: cx, top: cy, right: cx + cw, bottom: cy + ch };

    // 跟别的弹层同一套投影：同尺寸灰纱、右下让 6 像素、50% 混。
    let off = (4.0 * (s.dpi as f64 / 96.0)) as i32 + 1;
    let sh = RECT {
        left: panel.left + off,
        top: panel.top + off,
        right: panel.right + off,
        bottom: panel.bottom + off,
    };
    veil_px(px, w, h, &sh, 0x009E9E9E, 128);
    // 底和框取对话框那层（以图搜图提示窗用的也是这一套），不拿标题栏主色
    put(px, w, h, &panel, face.bg);
    let line = face.line;
    for r in [
        RECT { left: panel.left, top: panel.top, right: panel.right, bottom: panel.top + 1 },
        RECT { left: panel.left, top: panel.bottom - 1, right: panel.right, bottom: panel.bottom },
        RECT { left: panel.left, top: panel.top, right: panel.left + 1, bottom: panel.bottom },
        RECT { left: panel.right - 1, top: panel.top, right: panel.right, bottom: panel.bottom },
    ] {
        put(px, w, h, &r, line);
    }

    // 窗口太矮时正文会被面板底边截住：先裁进面板，再逐段判断放不放得下。
    let dc = SaveDC(hdc);
    let _ = IntersectClipRect(hdc, panel.left + 1, panel.top + 1, panel.right - 1, panel.bottom - 1);

    let mut y = panel.top + pady;
    let _ = SetTextColor(hdc, th::cref(face.ink));
    let mut u: Vec<u16> = "参考图画布模式 · 使用方法".encode_utf16().collect();
    let mut tr = RECT { left: panel.left + padx, top: y, right: panel.right - padx, bottom: y + title_h };
    let _ = DrawTextW(hdc, &mut u, &mut tr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
    y += title_h;
    put(
        px,
        w,
        h,
        &RECT { left: panel.left + padx, top: y + k(5), right: panel.right - padx, bottom: y + k(5) + 1 },
        th::mix(face.bg, face.ink, 0.18),
    );
    y += k(12);

    let btn_top = panel.bottom - pady - btn_h;
    for (i, (head, body)) in INTRO.iter().enumerate() {
        let (hh, bh) = hs[i];
        if y + hh + k(3) + bh > btn_top - k(8) {
            break;
        }
        let _ = SetTextColor(hdc, th::cref(face.ink));
        let mut u: Vec<u16> = head.encode_utf16().collect();
        let mut r = RECT { left: panel.left + padx, top: y, right: panel.right - padx, bottom: y + hh };
        let _ = DrawTextW(hdc, &mut u, &mut r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        y += hh + k(3);
        // 正文比小标题淡一档，段落之间才有层次
        let _ = SetTextColor(hdc, th::cref(th::mix(face.ink, face.bg, 0.12)));
        let mut u: Vec<u16> = body.encode_utf16().collect();
        let mut r = RECT { left: panel.left + padx, top: y, right: panel.right - padx, bottom: y + bh };
        let _ = DrawTextW(hdc, &mut u, &mut r, DT_LEFT | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX);
        y += bh + gap;
    }

    // 底行：左边列一眼能用的功能，右边「知道了」。
    let btn = RECT {
        left: panel.right - padx - btn_w,
        top: btn_top,
        right: panel.right - padx,
        bottom: btn_top + btn_h,
    };
    let _ = SetTextColor(hdc, th::cref(face.dim));
    let mut u: Vec<u16> = INTRO_FOOT.encode_utf16().collect();
    let mut fr = RECT { left: panel.left + padx, top: btn_top, right: btn.left - k(12), bottom: btn_top + btn_h };
    let _ = DrawTextW(hdc, &mut u, &mut fr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
    let bcol = if s.intro_hot { face.hov } else { face.bg };
    put(px, w, h, &btn, bcol);
    for r in [
        RECT { left: btn.left, top: btn.top, right: btn.right, bottom: btn.top + 1 },
        RECT { left: btn.left, top: btn.bottom - 1, right: btn.right, bottom: btn.bottom },
        RECT { left: btn.left, top: btn.top, right: btn.left + 1, bottom: btn.bottom },
        RECT { left: btn.right - 1, top: btn.top, right: btn.right, bottom: btn.bottom },
    ] {
        put(px, w, h, &r, line);
    }
    let _ = SetTextColor(hdc, th::cref(face.ink));
    let mut u: Vec<u16> = "知道了".encode_utf16().collect();
    let mut br = btn;
    let _ = DrawTextW(hdc, &mut u, &mut br, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    s.intro_btn = btn;

    let _ = RestoreDC(hdc, dc);
    // 面板整块钉成不透：分层面上 GDI 写完字 alpha 不可靠，透出去会糊。
    opaque(px, w, h, &panel);
}

/// 一块实色，四角各切掉 `cut` 像素（0 = 方角，取到一半就是一枚圆）。
/// 逐行铺而不是 GDI 圆角裁剪：那条路要在分层窗面上动裁剪区，切不干净会在 alpha 上留边。
unsafe fn block(px: *mut u32, w: i32, h: i32, r: &RECT, cut: i32, col: u32) {
    for y in r.top.max(0)..r.bottom.min(h) {
        let i = (y - r.top).min(r.bottom - 1 - y);
        let c = (cut - i).max(0);
        put(px, w, h, &RECT { left: r.left + c, top: y, right: r.right - c, bottom: y + 1 }, col);
    }
}

/// 上面那块的一圈 1px 边，跟着同一档切角。
unsafe fn ring(px: *mut u32, w: i32, h: i32, r: &RECT, cut: i32, col: u32) {
    for y in r.top.max(0)..r.bottom.min(h) {
        let i = (y - r.top).min(r.bottom - 1 - y);
        let c = (cut - i).max(0);
        let (l, rr) = (r.left + c, r.right - c);
        if y == r.top || y == r.bottom - 1 {
            put(px, w, h, &RECT { left: l, top: y, right: rr, bottom: y + 1 }, col);
        } else if rr > l {
            put(px, w, h, &RECT { left: l, top: y, right: l + 1, bottom: y + 1 }, col);
            put(px, w, h, &RECT { left: rr - 1, top: y, right: rr, bottom: y + 1 }, col);
        }
    }
}

/// 一个 ✕：两条 1px 对角线。和主窗口那枚同名的一模一样，只是这边只能逐像素写。
unsafe fn xmark_px(px: *mut u32, w: i32, h: i32, cx: i32, cy: i32, d: i32, col: u32) {
    for i in -d..=d {
        put(px, w, h, &RECT { left: cx + i, top: cy + i, right: cx + i + 1, bottom: cy + i + 1 }, col);
        put(px, w, h, &RECT { left: cx + i, top: cy - i, right: cx + i + 1, bottom: cy - i + 1 }, col);
    }
}

/// 客户区左上角在屏幕上的位置动了要不要补。
///
/// 拖左边框或上边框改大小会把这个原点一起推着走，而卡片坐标写的是客户区，
/// 不补这一步图就贴着边框一起跑；他要么调不动视口，要么得重摆一遍图。
/// 补上它，改窗口大小就等于改看得到的画面，图在屏幕上不动。
/// 拖标题条挪窗口是另一种：整扇一起走，那种不补，只更新基准。
/// `rc` 一起变了才判为改大小，所以 WM_MOVE 和 WM_SIZE 谁先到都只补一遍。
/// 还回这次到底是不是改大小 —— 两条消息只该有一趟去重推那张面。
unsafe fn sync_origin(hwnd: HWND, s: &mut State) -> bool {
    let mut p = POINT { x: 0, y: 0 };
    let _ = ClientToScreen(hwnd, &mut p);
    let rc = client(hwnd);
    let resized = (rc.right - rc.left) != (s.rc.right - s.rc.left)
        || (rc.bottom - rc.top) != (s.rc.bottom - s.rc.top);
    if resized {
        s.panx -= p.x - s.ox;
        s.pany -= p.y - s.oy;
    }
    s.ox = p.x;
    s.oy = p.y;
    s.rc = rc;
    resized
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

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let x = (lp.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lp.0 >> 16) & 0xffff) as u16 as i16 as i32;
    // 说明卡开着时它就是这扇窗的全部界面：底下的图不许被拖、被删、被滚。
    // 标题条那一条放行 —— 关窗、拖窗、调透明还得照常按得着。
    let (intro, strip, on_sld) = match state().lock().unwrap().as_ref() {
        Some(s) => (s.intro, s.strip, s.drag_sld),
        None => (false, 0, false),
    };
    if intro {
        match msg {
            // 滑块拖出标题条时这一趟还得归它，不然拖到一半就卡住
            WM_MOUSEMOVE if y >= strip && !on_sld => {
                let mut dirty = false;
                if let Some(s) = state().lock().unwrap().as_mut() {
                    let was = s.intro_hot;
                    s.intro_hot = over(&s.intro_btn, x, y);
                    dirty = was != s.intro_hot;
                }
                if dirty {
                    push(hwnd);
                }
                return LRESULT(0);
            }
            WM_LBUTTONDOWN if y >= strip => {
                let on = state().lock().unwrap().as_ref().is_some_and(|s| over(&s.intro_btn, x, y));
                if on {
                    close_intro(hwnd);
                }
                return LRESULT(0);
            }
            // 标题条上的滑块按下时抓过鼠标，这一下松开得把抓取和拖动一起收掉，
            // 不然它会一直跟着指针跑
            WM_LBUTTONUP => {
                let _ = ReleaseCapture();
                if let Some(s) = state().lock().unwrap().as_mut() {
                    s.drag = None;
                    s.resize = None;
                    s.drag_sld = false;
                }
                return LRESULT(0);
            }
            WM_RBUTTONUP | WM_RBUTTONDBLCLK | WM_MOUSEWHEEL | WM_DROPFILES => {
                return LRESULT(0);
            }
            // 卡片盖着的图不该冒出"改尺寸"的双向箭头，卡片区域一律保持箭头
            WM_SETCURSOR if (lp.0 & 0xffff) as u32 == HTCLIENT => {
                if let Ok(c) = LoadCursorW(None, IDC_ARROW) {
                    SetCursor(Some(c));
                }
                return LRESULT(1);
            }
            WM_KEYDOWN => {
                if wp.0 == VK_ESCAPE.0 as usize
                    || wp.0 == VK_RETURN.0 as usize
                    || wp.0 == VK_SPACE.0 as usize
                {
                    close_intro(hwnd);
                }
                return LRESULT(0);
            }
            _ => {}
        }
    }
    match msg {
        WM_PAINT => {
            push(hwnd);
            // 纯 ULW 窗口不经过 BeginPaint，更新区没人清；类样式里的 CS_HREDRAW/VREDRAW
            // 每次改尺寸整窗失效，理论上会陷入"WM_PAINT→重画→仍脏"的自旋。补一道验证。
            let _ = ValidateRect(Some(hwnd), None);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        // 客户区盖满整窗：那一面就是这扇窗口的全部内容，边上不会多出系统画的实色框，
        // 背景透出去也只透这一面，不透出一条框。
        WM_NCCALCSIZE => LRESULT(0),
        WM_NCHITTEST => {
            let sx = (lp.0 & 0xffff) as u16 as i16 as i32;
            let sy = ((lp.0 >> 16) & 0xffff) as u16 as i16 as i32;
            let mut wr = RECT::default();
            let _ = GetWindowRect(hwnd, &mut wr);
            let (edge, strip, sld, cls) = match state().lock().unwrap().as_ref() {
                Some(s) => (s.edge, s.strip, s.sld, s.cls),
                None => (6, 30, RECT::default(), RECT::default()),
            };
            let (l, r, t, b) =
                (sx - wr.left < edge, wr.right - sx < edge, sy - wr.top < edge, wr.bottom - sy < edge);
            let grip = match (l, r, t, b) {
                (true, _, true, _) => Some(HTTOPLEFT),
                (_, true, true, _) => Some(HTTOPRIGHT),
                (true, _, _, true) => Some(HTBOTTOMLEFT),
                (_, true, _, true) => Some(HTBOTTOMRIGHT),
                (true, _, _, _) => Some(HTLEFT),
                (_, true, _, _) => Some(HTRIGHT),
                (_, _, true, _) => Some(HTTOP),
                (_, _, _, true) => Some(HTBOTTOM),
                _ => None,
            };
            if let Some(g) = grip {
                return LRESULT(g as isize);
            }
            // 客户区盖满整窗，窗口坐标就是客户区坐标
            let (cx, cy) = (sx - wr.left, sy - wr.top);
            if cy < strip {
                // 条上那两块得留成客户区，才吃得到鼠标的按下和移动；其余按标题条拖走
                if over(&sld, cx, cy) || over(&cls, cx, cy) {
                    return LRESULT(HTCLIENT as isize);
                }
                return LRESULT(HTCAPTION as isize);
            }
            LRESULT(HTCLIENT as isize)
        }
        WM_SETCURSOR => {
            // 指针压在图片四边/四角上换成双向箭头；标题条、窗口边框照旧交给系统。
            // 正在拖边时按抓住的那条算，图变大后指针已经在图里头了，箭头也不该跳回默认。
            if (lp.0 & 0xffff) as u32 == HTCLIENT {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let _ = ScreenToClient(hwnd, &mut pt);
                let g = {
                    let g = state().lock().unwrap();
                    match g.as_ref() {
                        Some(s) => s
                            .resize
                            .map(|r| r.grip)
                            .or_else(|| grip_at(s, pt.x, pt.y).map(|(_, g)| g)),
                        None => None,
                    }
                };
                if let Some(g) = g {
                    if let Ok(c) = LoadCursorW(None, grip_cursor(g)) {
                        SetCursor(Some(c));
                    }
                    return LRESULT(1);
                }
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_SIZE => {
            // 被系统最小化那一刻：客户区变成 0、窗口原点被推到屏幕外去了。
            // 这一趟什么都不记、什么都不画（记了就等于把基准弄脏，弹回来时图会跑位），
            // 只排一趟消息把自己还原。
            if wp.0 == SIZE_MINIMIZED as usize {
                let _ = PostMessageW(Some(hwnd), WM_UNMIN, WPARAM(0), LPARAM(0));
                return DefWindowProcW(hwnd, msg, wp, lp);
            }
            if IsIconic(hwnd).as_bool() {
                return DefWindowProcW(hwnd, msg, wp, lp);
            }
            {
                let mut g = state().lock().unwrap();
                if let Some(s) = g.as_mut() {
                    sync_origin(hwnd, s);
                }
            }
            push(hwnd);
            LRESULT(0)
        }
        WM_UNMIN => {
            // 这板是拿来对着别的软件看的，收进任务栏就没意义了：谁把它最小化都当场弹回来。
            // 用 SHOWNOACTIVATE 不抢前台，他正在用的那扇不受打扰。
            // 还原只能在这一趟做：WM_SIZE 里直接 ShowWindow 会当场又发一趟 WM_SIZE，
            // 那趟进来要拿状态锁，而锁还在上一次手里，就地卡死。
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            LRESULT(0)
        }
        WM_MOVE => {
            // 最小化/还原路上那些把窗口挪到屏幕外的消息，一律不认
            if IsIconic(hwnd).as_bool() {
                return LRESULT(0);
            }
            // 挪窗口只更新基准、不重画：面本来就跟着窗口一起走
            let mut g = state().lock().unwrap();
            let mut resized = false;
            if let Some(s) = g.as_mut() {
                resized = sync_origin(hwnd, s);
            }
            drop(g);
            // 系统按"移动过就发 WM_MOVE、改过尺寸就发 WM_SIZE"分别通知，谁先到不一定。
            // 这一趟尺寸也变了，说明 WM_SIZE 还没走过来，就在这里把面重推一次，
            // 免得同一帧合成两遍、透底那一层闪一下。
            if resized {
                push(hwnd);
            }
            LRESULT(0)
        }
        WM_IMG => {
            push(hwnd);
            LRESULT(0)
        }
        WM_DROPFILES => {
            // 从资源管理器把图片拖进来
            let hd = HDROP(wp.0 as *mut core::ffi::c_void);
            let mut pt = POINT::default();
            let _ = DragQueryPoint(hd, &mut pt);
            let mut got: Vec<String> = Vec::new();
            let n = DragQueryFileW(hd, u32::MAX, None);
            for i in 0..n {
                let len = DragQueryFileW(hd, i, None) as usize;
                if len == 0 {
                    continue;
                }
                let mut buf = vec![0u16; len + 1];
                if DragQueryFileW(hd, i, Some(&mut buf)) > 0 {
                    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                    got.push(String::from_utf16_lossy(&buf[..end]));
                }
            }
            let _ = DragFinish(hd);
            for (i, p) in got.iter().enumerate() {
                // 一起拖好几张：只有第一张压在松手那点，其余照排图规则往后叠
                add_file(p, if i == 0 { Some((pt.x, pt.y)) } else { None });
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            // Ctrl+V：把剪贴板里那张贴成画布上的一张。开着弹层也认这一条，顺手把
            // 菜单收掉——右键看完菜单再按键，不该还要先点一下空白。
            if wp.0 == VK_V.0 as usize && GetKeyState(VK_CONTROL.0 as i32) < 0 {
                close_menu(hwnd);
                paste_clip();
                return LRESULT(0);
            }
            // Ctrl+C：复制鼠标底下那一张，和弹层那一行、和 Del 都是同一个落点。
            if wp.0 == VK_C.0 as usize && GetKeyState(VK_CONTROL.0 as i32) < 0 {
                close_menu(hwnd);
                let card = {
                    let g = state().lock().unwrap();
                    g.as_ref().and_then(|s| hit(s, s.hover.0, s.hover.1).map(|i| s.cards[i].id))
                };
                menu_act(hwnd, MI_COPY, card);
                return LRESULT(0);
            }
            let menu_open = state().lock().unwrap().as_ref().is_some_and(|s| s.menu.is_some());
            if menu_open {
                // Esc 只收菜单；别的键（Del 那一类）照常往下走，别把按键一起吞了
                if wp.0 == VK_ESCAPE.0 as usize {
                    close_menu(hwnd);
                    return LRESULT(0);
                }
                close_menu(hwnd);
            }
            // VK_DELETE：删掉鼠标底下那一张。这扇自己收键，主窗口那边的加速键
            // 表已经按窗口句柄挡住了，不会把主窗口的选中项一起删了。
            if wp.0 != VK_DELETE.0 as usize {
                return LRESULT(0);
            }
            let mut g = state().lock().unwrap();
            let Some(s) = g.as_mut() else { return LRESULT(0) };
            let (hx, hy) = s.hover;
            if !drop_card(s, hx, hy) {
                return LRESULT(0);
            }
            drop(g);
            push(hwnd);
            LRESULT(0)
        }
        WM_ACTIVATE => {
            // 抬到别处去了就收掉弹层（WA_INACTIVE = 0）：这一扇没了焦点还留着它，
            // 下一次点击会被当成选项
            if wp.0 as u16 == WA_INACTIVE as u16 {
                close_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_CLIPBOARDUPDATE => {
            let has = clipboard_has_image();
            let mut changed = false;
            if let Some(s) = state().lock().unwrap().as_mut() {
                changed = s.clip != has;
                s.clip = has;
            }
            if changed {
                set_title(has);
                push(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            // 弹层开着：这一下先看落点。压在行上就收菜单、办那一行的事；
            // 落在行外只把菜单抹掉，这一顿不吞掉点击——点了两下才拖得动，读起来就是卡。
            let mut on_row: Option<(usize, Option<usize>, bool)> = None;
            let mut was_menu = false;
            {
                let mut g = state().lock().unwrap();
                if let Some(s) = g.as_mut() {
                    if let Some(m) = s.menu.take() {
                        was_menu = true;
                        if let Some(id) = m.rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
                            on_row = Some((id, m.card, menu_on(id, m.card, m.paste)));
                        }
                    }
                }
            }
            if was_menu {
                push(hwnd);
            }
            if let Some((id, card, usable)) = on_row {
                if usable {
                    menu_act(hwnd, id, card);
                }
                return LRESULT(0);
            }
            let _ = SetCapture(hwnd);
            let mut act = None;
            {
                let mut g = state().lock().unwrap();
                if let Some(s) = g.as_mut() {
                    s.hover = (x, y);
                    if over(&s.sld, x, y) {
                        s.drag_sld = true;
                        s.bg = alpha_at(&s.trk, x);
                        act = Some(0);
                    } else if over(&s.cls, x, y) {
                        act = Some(1);
                    } else if y >= s.strip {
                        // 压在图边上：这一下拿来改那张的尺寸，不整张拖走
                        if let Some((i, grip)) = grip_at(s, x, y) {
                            if let Some(r) = on_screen(&s.cards[i], s) {
                                let c = &s.cards[i];
                                s.resize = Some(Resize {
                                    id: c.id,
                                    grip,
                                    x0: c.x,
                                    y0: c.y,
                                    w0: r.right - r.left,
                                    h0: r.bottom - r.top,
                                    scale0: c.scale,
                                });
                            }
                        } else {
                            // 条底下才算画布：压着图拖那一张，压着空白拖整块画布
                            let d = match hit(s, x, y) {
                                Some(i) => Some((s.cards[i].id as i64, x, y, s.cards[i].x, s.cards[i].y)),
                                None => Some((-1, x, y, s.panx, s.pany)),
                            };
                            s.drag = d;
                        }
                    }
                }
            }
            match act {
                Some(0) => push(hwnd),
                Some(1) => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let mut moved = false;
            {
                let mut g = state().lock().unwrap();
                if let Some(s) = g.as_mut() {
                    if s.menu.is_some() {
                        // 弹层开着：这一趟只用来挪那一行的高亮，底下的图一张都不动
                        let hot = s
                            .menu
                            .as_ref()
                            .map(|m| {
                                let (card, paste) = (m.card, m.paste);
                                m.rows
                                    .iter()
                                    .position(|(i, r)| over(r, x, y) && menu_on(*i, card, paste))
                                    .map(|i| i as i32)
                                    .unwrap_or(-1)
                            })
                            .unwrap_or(-1);
                        if let Some(m) = s.menu.as_mut() {
                            if m.hot != hot {
                                m.hot = hot;
                                moved = true;
                            }
                        }
                    } else {
                        // 只在跨过关闭键那一格时补一趟：它悬停要换底色，别的悬停什么都不换
                        let was = over(&s.cls, s.hover.0, s.hover.1);
                        s.hover = (x, y);
                        if was != over(&s.cls, x, y) {
                            moved = true;
                        }
                        if s.drag_sld {
                            s.bg = alpha_at(&s.trk, x);
                            moved = true;
                        } else if let Some(rz) = s.resize {
                            apply_resize(s, rz, x, y);
                            moved = true;
                        } else if let Some((id, dx, dy, sx, sy)) = s.drag {
                            let (cx, cy) = (sx + (x - dx), sy + (y - dy));
                            if id >= 0 {
                                if let Some(c) = s.cards.iter_mut().find(|c| c.id as i64 == id) {
                                    c.x = cx;
                                    c.y = cy;
                                }
                            } else {
                                s.panx = cx;
                                s.pany = cy;
                            }
                            moved = true;
                        }
                    }
                }
            }
            if moved {
                push(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let _ = ReleaseCapture();
            let mut g = state().lock().unwrap();
            if let Some(s) = g.as_mut() {
                s.drag = None;
                s.resize = None;
                s.drag_sld = false;
            }
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            // 右键一律开弹层：压在图上时那一张就是这一排要动的对象，落在空白只有粘贴点得动。
            // 标题条那一条不归画布，不在那儿开。
            let mut opened = false;
            {
                let mut g = state().lock().unwrap();
                if let Some(s) = g.as_mut() {
                    if y >= s.strip {
                        s.hover = (x, y);
                        open_menu(s, x, y);
                        opened = true;
                    }
                }
            }
            if opened {
                push(hwnd);
            }
            LRESULT(0)
        }
        WM_RBUTTONDBLCLK => {
            let mut g = state().lock().unwrap();
            let Some(s) = g.as_mut() else { return LRESULT(0) };
            // 弹层正开着，这一下算"换个地方再开一次"，不能顺手把底下那张删掉
            if s.menu.is_some() {
                open_menu(s, x, y);
                drop(g);
                push(hwnd);
                return LRESULT(0);
            }
            // 右键双击图上：删掉这一张（叠着的只删最上面那张）
            if !drop_card(s, x, y) {
                return LRESULT(0);
            }
            drop(g);
            push(hwnd);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let d = ((wp.0 >> 16) & 0xffff) as u16 as i16 as f64;
            if d == 0.0 {
                return LRESULT(0);
            }
            // 滚轮的坐标是屏幕的，换成客户区才压得住指针底下那个点
            let mut pt = POINT {
                x: (lp.0 & 0xffff) as u16 as i16 as i32,
                y: ((lp.0 >> 16) & 0xffff) as u16 as i16 as i32,
            };
            let _ = ScreenToClient(hwnd, &mut pt);
            let f = if d > 0.0 { STEP } else { 1.0 / STEP };
            let mut g = state().lock().unwrap();
            let Some(s) = g.as_mut() else { return LRESULT(0) };
            if pt.y < s.strip {
                // 标题条那一条上滚滚轮不动画布：那儿是滑块和名字的地盘
                return LRESULT(0);
            }
            // 不论指针压着图还是落在空白，滚轮都缩整块画布：所有图按同一倍率
            // 一起动，图与图之间摆好的位置不散。
            if !zoom_all(s, pt.x, pt.y, f) {
                return LRESULT(0);
            }
            drop(g);
            push(hwnd);
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            let mmi = lp.0 as *mut MINMAXINFO;
            if !mmi.is_null() {
                unsafe {
                    // 不能再窄了：标题条上名字 + 滑块 + 关闭，挤到一起就没法点
                    (*mmi).ptMinTrackSize.x = 460;
                    (*mmi).ptMinTrackSize.y = 180;
                }
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_DESTROY => {
            let _ = RemoveClipboardFormatListener(hwnd);
            WIN.store(0, Ordering::SeqCst);
            let mut g = state().lock().unwrap();
            if let Some(s) = g.as_mut() {
                // 后备面和字体都是 GDI 对象，不跟着窗口放掉就漏在进程里
                free_surf(s);
                if s.font != 0 {
                    let _ = DeleteObject(HGDIOBJ(s.font as *mut core::ffi::c_void));
                    s.font = 0;
                }
            }
            *g = None;
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

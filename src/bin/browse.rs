#![allow(unsafe_op_in_unsafe_fn)]
#![windows_subsystem = "windows"]
/// 浏览窗口。六个区：菜单条 / 工具条 / 路径条 / 左文件夹格 /
/// 中间缩略图格 / 右预览信息格 / 状态栏。颜色和尺寸全部取自 theme，这里不写死任何 RGB。
/// 缩略图走 pool 的可见优先队列：只排看得见的那一屏加往后一页，滚过就弃算。
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use std::sync::atomic::AtomicU64;
use windows::Win32::Foundation::HANDLE;
use xsee_host::{
    appicon, assoc, cache, clip, catdb, encode, fmticon, glyphs, job, menuicons,
    pool::Pool,
    refwin,
    scan,
    sim, sysicon, Rgba,
    theme::{self, mix, Style, Theme, cref, MENU_IC_W, MENU_IC_X},
};
use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    COLORREF, FILETIME, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SYSTEMTIME, WPARAM,
};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Graphics::Dwm::{
    DWMNCRENDERINGPOLICY, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_SYSTEM,
    FileTimeToLocalFileTime, GetLogicalDrives,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::SystemServices::{MK_CONTROL, MK_LBUTTON, MK_SHIFT};
use windows::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, INFINITE, SetWaitableTimer, TIMER_ALL_ACCESS,
};
use windows::Win32::System::Time::FileTimeToSystemTime;
use windows::Win32::UI::Controls::{EM_SETSEL, WM_MOUSELEAVE};
use windows::Win32::UI::Controls::Dialogs::{
    CC_FULLOPEN, CC_RGBINIT, CF_INITTOLOGFONTSTRUCT, CF_SCREENFONTS, CHOOSECOLORW, CHOOSEFONTW,
    ChooseColorW, ChooseFontW, GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER,
    OFN_FILEMUSTEXIST, OFN_HIDEREADONLY, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Shell::{
    BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, DefSubclassProc, SHBrowseForFolderW,
    SHGetPathFromIDListW, SetWindowSubclass,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
    SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, GetFocus, GetKeyState, ReleaseCapture, SetCapture, SetFocus,
    TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_CONTROL, VK_LBUTTON, VK_MENU, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::*;

// JSON 命令口（xsee --cmd '...'）。单独一个文件，别往这九十万行里堆。
// 路径要点名：bin 根文件的子模块默认往 src/bin/ 那一层找，会撞成另一个 bin。
#[path = "browse/drive.rs"]
mod drive;

const CLASS: &str = "XseeBrowseWnd";
const TITLE: &str = "Xsee 浏览";
const WM_PV: u32 = WM_APP + 1;
const WM_DROP_DONE: u32 = WM_APP + 2;
const TMR: usize = 1;
/// 滚动帧节拍的定时器号。建得出高精度等待定时器就用不上它，建不出才退回 SetTimer。
const TMR_ANIM: usize = 2;
/// 滚动帧节拍：16 毫秒一帧（约 60 帧/秒）。SetTimer 会被系统合并到 15.6 毫秒的
/// 节拍上、33 毫秒实际要等到 48 毫秒，所以这里不用系统定时器打底，帧距要准。
const FRAME_MS: u64 = 16;
/// 惯性滚动的三件套：指数摩擦系数（每秒速度乘 exp(-9)≈零头）、停下的速度门槛、
/// 速度上限。滚轮给冲量，帧节拍按速度积分走位，松手后自己滑一段再停。
const SCROLL_FRICTION: f64 = 9.0;
const SCROLL_STOP: f64 = 24.0;
const SCROLL_MAX_V: f64 = 12_000.0;
/// 剪切/复制之后的整窗提示纱：持续时间、最浓那一档的不透明度（0–255）。
/// 颜色不在这里，走皮肤槽 `flash`，换皮肤跟着换。
const FLASH_MS: Duration = Duration::from_millis(220);
const FLASH_ALPHA: u8 = 72;
/// 正在从本窗口往外拖东西。松手没落到文件夹上时，这一趟就地结束，什么都不做。
static SELF_DRAG: AtomicBool = AtomicBool::new(false);
/// 小键盘排序键按下时先立一个旗，紧跟其后的 WM_CHAR 直接吞掉：不吞的话那个数字
/// 会被“打字跳到同名条目”接走，列表刚排完又跳到别处。
static CHAR_SWALLOW: AtomicBool = AtomicBool::new(false);

const M_OPEN: usize = 101;
const M_EXPL: usize = 102;
const M_RESCAN: usize = 103;
const M_QUIT: usize = 104;
const M_BACK: usize = 105;
const M_FWD: usize = 106;
const M_UP: usize = 107;
/// 树右键：只重扫右键那一行的子节点（格子的「刷新」是 M_RESCAN，管当前目录）。
const M_TREE_REFRESH: usize = 108;
const M_FIND: usize = 109;
const M_ALL: usize = 111;
const M_NONE: usize = 112;
const M_VIEWER: usize = 113;
const M_EDIT: usize = 115;
const M_MOVE: usize = 116;
const M_COPY: usize = 117;
const M_DEL: usize = 118;
/// Shift+Del：不进回收站，删了找不回，所以要过确认框
const M_DEL_PERM: usize = 124;
const M_PROP: usize = 119;
const M_VGRID: usize = 120;
const M_VSMALL: usize = 121;
const M_VLIST: usize = 122;
const M_VDETAIL: usize = 123;
const M_LOOK: usize = 125;
const M_SKIN_SET: usize = 126;
const M_ABOUT: usize = 161;
/// 皮肤菜单项：每套占一个号，从 M_SKIN 起连续排。原来挤在 200 段只留了
/// 20 个号，套数一过 20 后面的项点了没反应；5000 段又被路径历史（3000+）截走，
/// 所以放在 1500–2999 这段空号里。
const M_SKIN: usize = 1500;
const M_DENS: usize = 220;
/// 外观菜单里的版式选择：每套开源图标库占一个号，从 M_LAYOUT + 1 起
/// （顺序同 glyphs::all()）。整段放在 1000：400 那一段全被对话框按钮占着
/// （D_OK=400、D_CANCEL=401、D_BROWSE=402…），版式点下去会被当成按了对话框的按钮。
const M_LAYOUT: usize = 1000;
/// 路径栏上的视图组按钮，从 M_VGRID 起四个；缩略图高度用一对 −/+ 调。
const M_ZOOM_DN: usize = 240;
const M_ZOOM_UP: usize = 242;
const M_ADDR: usize = 241;
/// 排列图标：按逆向出来的菜单 20111 的条目走。
const M_SORT_NAME: usize = 260;
const M_SORT_EXT: usize = 261;
const M_SORT_SIZE: usize = 262;
const M_SORT_TYPE: usize = 263;
const M_SORT_DATE: usize = 264;
const M_SORT_IMAGE: usize = 265;
const M_SORT_DESC: usize = 266;
const M_SORT_FWD: usize = 267;
const M_SORT_REV: usize = 268;
/// 按入库时间排：用的是我们自己库里那一列时间（第一次见到那张图时盖当下时间，
/// 之后不再改）。排在末尾，不改动上面那批命令号。
const M_SORT_ADDED: usize = 276;
const M_NEWDIR: usize = 269;
const M_PASTE: usize = 270;
const M_CUT: usize = 271;
const M_COPYF: usize = 272;
const M_RENAME: usize = 273;
const M_COPYIMG: usize = 274;
const M_CONV: usize = 275;
const M_HELP: usize = 277;
const M_NEWWIN: usize = 278;
const M_EDITOR: usize = 279;
const M_EXTAPP: usize = 280;
/// 壁纸那三条占连续三个号，整段挪到 346：留在 281 时「平铺」正好撞在 M_BATCH(282) 上，
/// 点平铺弹出的是批量重命名。
const M_WALL: usize = 346;
const M_BATCH: usize = 282;
const M_ORG: usize = 284;
const M_ORG_CLOSE: usize = 285;
const M_PAGE_PREV: usize = 286;
const M_PAGE_NEXT: usize = 287;
const M_CAT_DEL: usize = 288;
const M_CAT_COPY: usize = 289;
const M_CAT_PASTE: usize = 290;
const M_CAT_RENAME: usize = 291;
const M_CAT_NEW: usize = 292;

/// 查看菜单里那几个窗格/工具条开关。关掉的那块在 `zones()` 里直接归零，
/// 画和点共用同一份区域，不会出现画没了还点得着。
const M_PANE_TREE: usize = 300;
const M_PANE_PV: usize = 301;
const M_TB_TEXT: usize = 303;
const M_TB_NOTEXT: usize = 304;
const M_TB_RIGHT: usize = 305;
const M_TB_BELOW: usize = 306;
const M_TB_HIDE: usize = 307;
const M_BAR_PATH: usize = 308;
const M_BAR_STATUS: usize = 309;
const M_DBBACKUP: usize = 310;
const M_DBIMPORT: usize = 311;
/// 分类数据库自己的入口；缩略图数据库另有一个，两页彻底分开互不掺和。
const M_CATDB: usize = 312;
const M_THUMBDB: usize = 321;
/// 中间列表的过滤开关。分类时把已经分过的、非图片和文件夹先藏掉，眼前只剩没干的活。
const M_FILT_ALL: usize = 313;
const M_FILT_DIRS: usize = 314;
const M_FILT_NONIMG: usize = 315;
const M_FILT_CAT: usize = 316;
/// 格子右键：拿这一张去指纹库里找长得像的。
const M_FINDSIM: usize = 317;
/// 右侧栏的批量转换页。和整理页共用一栏宽，同开时靠页签切换。
const M_CONV_PANEL: usize = 318;
const M_CONV_PANEL_CLOSE: usize = 319;
/// 格子右键：把这张图摆进参考图画布（画布没开就先开一扇）。
const M_REF: usize = 331;
/// 工具菜单里的文件关联：勾要登记的格式，把 Xsee 写进 HKCU 的候选列表。
const M_ASSOC: usize = 332;
/// 右键菜单：把落点的路径按纯文本放进剪贴板（快捷键之外另给一个入口）。
const M_COPYPATH: usize = 333;

/// 批量转换面板上的控件 id（800 起，避开对话框那一段 400–789）。
const C_FMT: usize = 800; // 800..805 依次是五种目标格式
const C_QUAL: usize = 810;
const C_BROWSE_OUT: usize = 811;
const C_OW: usize = 813; // 813..816 三格，摆的顺序见 OW_SLOTS
/// 面板「同名文件」三格的文字和存值。编号挪到头一格，存值仍按对话框那套
/// 0 取代 / 1 跳过 / 2 重命名 / 3 询问——面板只改显示顺序，两边共用同一个号。
const OW_SLOTS: [(&str, u8); 3] = [("编号", 2), ("跳过", 1), ("覆盖", 0)];
const C_FIT: usize = 820; // 820..825 原样 / 适应 / 填充 / 拉伸 / 缩小
const C_ROT: usize = 826; // 826..830 0 / 90 / 180 / 270
const C_FLIP_H: usize = 831;
const C_FLIP_V: usize = 832;
const C_SHARP: usize = 833;
const C_WM_ON: usize = 834;
const C_WM_POS: usize = 835; // 835..839 左上 / 右上 / 左下 / 右下
const C_WM_SIZE: usize = 840;
const C_WM_OPAC: usize = 841;
const C_RN_DN: usize = 842;
const C_RN_UP: usize = 843;
const C_START: usize = 844;
const C_STOP: usize = 845;
const C_TAB_ORG: usize = 846;
const C_TAB_CONV: usize = 847;
const C_CLOSE: usize = 848;
const C_SYNC: usize = 849;
/// 允许放大那个勾选。
const C_UP: usize = 850;
const C_RESET: usize = 851;
/// 转换后改名那个勾选。放在 850 之后的空档，别和 C_START=844 撞。
const C_RN_ON: usize = 852;
/// 锐化半径（编辑窗口那页的第一个参数）。
const C_SHARP_R: usize = 853;

/// 面板上五个原生输入框的控件 id，WM_COMMAND 靠它认出是哪一格。
const E_TW: usize = 900;
const E_TH: usize = 901;
const E_OUT: usize = 902;
const E_WM: usize = 903;
const E_RN: usize = 904;
const E_N: usize = 5;

/// 自绘对话框里的控件 id，和命令 id 共用 mouse_down 的分发。
const D_OK: usize = 400;
const D_CANCEL: usize = 401;
const D_BROWSE: usize = 402;
const D_FMT: usize = 410;
const D_SCOPE_CUR: usize = 430;
const D_SCOPE_DB: usize = 431;
const D_START: usize = 432;
const D_CLEAR: usize = 433;
const D_CLOSE: usize = 434;
const D_DELSRC: usize = 491;
const D_QUAL: usize = 492;
const D_FMTSET: usize = 493;
const D_OW: usize = 494;
const D_OW_ITEM: usize = 495;
const D_OPT_PROG: usize = 500;
const D_OPT_HUFF: usize = 501;
const D_OPT_SH: usize = 502;
const D_OPT_SV: usize = 503;
const D_THUMB: usize = 504;
const D_DCF: usize = 510;
const D_SAVEDEF: usize = 511;
const D_FMT_RESET: usize = 512;
const D_OPT_PNG: usize = 513;
const D_OPT_TIFF: usize = 514;
const D_RN_START_DN: usize = 520;
const D_RN_START_UP: usize = 521;
const D_RN_CASE: usize = 522;
const D_RN_HELP: usize = 526;
/// 文件关联页：扩展名勾选从 D_ASSOC 起连续排，一格一个号。
/// 1060 段是版式（1001..1048）和皮肤（1500..）之间的空档，别挪回 450——
/// 那边紧挨着格式设置页的 D_OPT_*，扩展名一多就会撞号。
const D_ASSOC: usize = 1060;
const ASSOC_MAX: usize = 96;
/// 应用勾选 / 打开系统默认应用设置。
const D_ASSOC_APPLY: usize = 1160;
const D_ASSOC_DEFAULTS: usize = 1161;
/// 启动复查的开关。
const D_ASSOC_CHECK: usize = 1162;

/// 皮肤设置窗：三个页签 + 字体/字号 + 配色槽 + 皮肤目录。
const D_SKIN_TAB: usize = 600;
const D_FACE: usize = 610;
/// 目录页：路径框/“选择文件夹…”共用一号，“用默认目录”单独一号。
const D_SKIN_DIR: usize = 620;
const D_SKIN_DIR_DEF: usize = 621;
const D_DBCHECK: usize = 640;
/// 维护页上的两个缩略图动作：重建当前列出的、重做名单上全部看过的。
const D_THUMB_FIX: usize = 641;
const D_THUMB_ALL: usize = 642;
/// 维护页上换数据库位置那一下。
const D_DB_MOVE: usize = 643;
/// 维护页上清指纹库：把原图已不在盘上的条目从全部档位里摘掉（从以图搜图页挪过来的）。
const D_DB_PRUNE: usize = 644;
/// 维护页的扫描文件夹：名单增删、开扫（所选/全部）、停止，行按 `D_SCAN_ROW + 可见行号`。
const D_SCAN_ADD: usize = 645;
const D_SCAN_DEL: usize = 646;
const D_SCAN_RUN: usize = 647;
const D_SCAN_ALL: usize = 648;
const D_SCAN_STOP: usize = 649;
const D_SCAN_ROW: usize = 652;
/// 扫描名单一次画五行，再多滚（滚轮认 paint 登记的那块 scan_list）。
const SCAN_VIS: usize = 5;
/// 扫描名单的规则编辑：按钮在维护页，对话框压在维护页上（dlg_hold 那套）。
const D_SCAN_RULES: usize = 658;
/// 规则对话框里的「保存并清理已入账」。
const D_RULE_SAVE: usize = 659;
/// 配色页每个颜色占连续编号；放在 800 段，避开维护页的 640..652。
const D_COLOR: usize = 800;
const D_RESET_LOOK: usize = 690;
/// 皮肤设置窗右下角那排：改动实时叠到界面上，点确定才问要不要留在皮肤文件里。
const D_LOOK_OK: usize = 700;
/// 以当前颜色另存成一套新皮肤（点一下才出名字输入框）。
const D_SKIN_ASAVE: usize = 702;
/// 删除当前这套皮肤：连文件带它的个人改动一起删，要先按一次武装。
const D_SKIN_DEL: usize = 703;
/// 点确定弹出的那句"是否覆盖当前皮肤"。确认框借用 ask_cmd 认这条，点"是"走 skin_commit。
const D_COVER_YES: usize = 704;
/// 字体页十块逐块一档：每块三个命中区，`D_ZONE + 块序*3` 依次是 减 / 加 / 加粗。
const D_ZONE: usize = 760;
/// 字体页每个版块行上的“这一块的字色”色块。870 起这一段没人用（转换面板只到 852）。
const D_ZONEC: usize = 870;
const ZR_DN: usize = 0;
const ZR_UP: usize = 1;
const ZR_BOLD: usize = 2;

/// 以图搜图那一页。710 起整段留给它，结果行按 `D_SIM_ROW + 可见行号` 排。
const D_SIM_PICK: usize = 711;
const D_SIM_CLIP: usize = 712;
const D_SIM_SLIDER: usize = 713;
const D_SIM_ALGO: usize = 714;
/// 下拉展开的三行：只看骨架 / 只看内容 / 两个都看。
const D_SIM_ALGO_ITEM: usize = 715;
const D_SIM_ROT: usize = 719;
const D_SIM_FLIP: usize = 720;
const D_SIM_GO: usize = 721;
const D_SIM_ROW: usize = 722;
/// 结果表一次最多画这么多行，再多就滚。命中 240 张，行号不会超过 id 区间。
const SIM_ROWS: usize = 24;
const D_SIM_AUTO: usize = 753;
/// 浮层自己那条标题条：按住它能把这一页拖走。所有对话框共用这一个号。
const D_TITLEBAR: usize = 759;

const WM_DB: u32 = WM_APP + 3;
const WM_SEARCH_DONE: u32 = WM_APP + 4;
const WM_CONV_DONE: u32 = WM_APP + 5;
const WM_DB_DONE: u32 = WM_APP + 6;
const WM_SIM_DONE: u32 = WM_APP + 8;
/// 那两张预览解好了，只负责重画一下；相似页的库统计算完也借它刷一帧。
const WM_SIM_PV: u32 = WM_APP + 10;

const PAGE_N: usize = 120;

/// 查找的代数：只认最后一次搜索的结果，前面慢的那条回来就丢掉。
static SEARCH_GEN: AtomicUsize = AtomicUsize::new(0);
static SEARCH_RES: OnceLock<Mutex<Option<(usize, String, Vec<String>)>>> = OnceLock::new();
/// 找相似也只认最后一次，前面慢的那条回来就丢掉。带回来的是耗时毫秒和整条结果行。
static SIM_GEN: AtomicUsize = AtomicUsize::new(0);
static SIM_RES: OnceLock<Mutex<Option<(usize, u128, Vec<SimHit>, Option<String>)>>> = OnceLock::new();
/// 两张预览也讲代数：连点几行只画最后点的那一张。
static SIM_PV_GEN: AtomicUsize = AtomicUsize::new(0);
/// 正拖着浮层。这是 App.dlg_drag 的一个镜像，只为让 refresh() 问一句就能走、
/// 不必去拿那把锁（有的调用点已经攥着它，拿了就是死锁）。
static DRAGGING: AtomicBool = AtomicBool::new(false);
/// 一次只跑一个转换批次；进度写在 msg 里由提示条刷新。
static CONV_RUN: AtomicBool = AtomicBool::new(false);
/// 转换中途按 Esc 置位。worker 每取一个文件先看它，队列里剩下的原样留着。
static CONV_STOP: AtomicBool = AtomicBool::new(false);
/// 数据库检查/备份/导入的结果：(给用户看的话, 要不要重载数据库)。
static DB_RESULT: Mutex<(u8, String, bool)> = Mutex::new((0, String::new(), false));
static INIT_WALL: OnceLock<Mutex<String>> = OnceLock::new();

/// 这一帧是被谁引起的：1=滚轮，2=60 毫秒心跳，0=别处。paint 收尾时读一次并清零。
static PAINT_SRC: AtomicU8 = AtomicU8::new(0);

/// 一帧各段耗时累计（微秒）：建后备位图/上栏/树/网格/预览/侧栏/弹层纱/上屏。
/// 只记滚轮帧，滚轮帧那批 flush 时才取走清零，免得别的帧把钱算混。
static SEG_TOT: Mutex<[u32; 8]> = Mutex::new([0; 8]);
static SEG_N: AtomicU32 = AtomicU32::new(0);
/// 刚画完这一帧的各段耗时。中位再低也看不出"咯噔"那一下是怎么来的，
/// 所以每帧的明细也要留一份，攒到批末尾跟汇总行一起写出去。
static LAST_SEG: Mutex<[u32; 8]> = Mutex::new([0; 8]);
static LAST_SUB: Mutex<[u32; 10]> = Mutex::new([0; 10]);

fn seg_tracing() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("XSEE_TRACE").is_some())
}

fn seg_add(v: &[u32; 8]) {
    if let Ok(mut t) = SEG_TOT.lock() {
        for i in 0..8 {
            t[i] = t[i].saturating_add(v[i]);
        }
        SEG_N.fetch_add(1, Ordering::Relaxed);
    }
}

/// 把累计的分段耗时折成平均每帧，交出去时清零。
fn seg_take() -> Option<[f64; 8]> {
    let n = SEG_N.swap(0, Ordering::Relaxed);
    if n == 0 {
        return None;
    }
    let mut t = SEG_TOT.lock().ok()?;
    let mut out = [0.0f64; 8];
    for i in 0..8 {
        out[i] = t[i] as f64 / n as f64;
        t[i] = 0;
    }
    drop(t);
    Some(out)
}

/// 网格那一段里再拆三层：地面（钉在窗口上）/ 逐格（钉在内容上）/ 尾巴（列头+滚动条+外框）。
/// 要判断滚动时哪些能整体搬进位图、哪些必须每帧重画，得先知道这两层各吃掉多少。
static SUB_TOT: Mutex<[u32; 10]> = Mutex::new([0; 10]);
static SUB_N: AtomicU32 = AtomicU32::new(0);

fn sub_add(v: &[u32; 10]) {
    if let Ok(mut t) = SUB_TOT.lock() {
        for i in 0..10 {
            t[i] = t[i].saturating_add(v[i]);
        }
        SUB_N.fetch_add(1, Ordering::Relaxed);
    }
}

fn sub_take() -> Option<[f64; 10]> {
    let n = SUB_N.swap(0, Ordering::Relaxed);
    if n == 0 {
        return None;
    }
    let mut t = SUB_TOT.lock().ok()?;
    let mut out = [0.0f64; 10];
    for i in 0..10 {
        out[i] = t[i] as f64 / n as f64;
        t[i] = 0;
    }
    drop(t);
    Some(out)
}

/// 逐格这一层里再按调用点拆开量：画刷起落 / 取图缩放 / 圆角裁剪 / 排字 / 圆环。
/// 只在滚轮帧的网格循环里开着（SUB_DEEP），关掉就只剩一次原子读，生产路径不吃这个钱。
static SUB_DEEP: AtomicBool = AtomicBool::new(false);
static DEEP_TOT: Mutex<[u32; 7]> = Mutex::new([0; 7]);
/// 一格排两行字：尺寸行、名字行。两行的画法不一样（一个带省略号一个不带），
/// 钱到底花在哪一行得分开看，所以排字这一档画之前先拨一下归哪一档。
static TXT_SLOT: AtomicUsize = AtomicUsize::new(3);

fn deep_snap() -> [u32; 7] {
    DEEP_TOT.lock().map(|t| *t).unwrap_or([0; 7])
}

/// 开着才取时间戳；`on` 由调用点传 SUB_DEEP 那一次读的结果，一次调用只读一遍。
fn deep_add(i: usize, t: Option<Instant>) {
    if let Some(t) = t {
        if let Ok(mut d) = DEEP_TOT.lock() {
            d[i] = d[i].saturating_add(t.elapsed().as_micros() as u32);
        }
    }
}

/// 帧计时。设了 XSEE_TRACE 才记，攒满一批才落文件：逐帧写的话文件 IO 会混进
/// 帧间隔里，"每帧画多久"就量不准了。滚轮帧和其余帧分两桶，滚动卡不卡只看前者。
static FRAMES: OnceLock<Mutex<FrameAcc>> = OnceLock::new();
const FRAME_BATCH: usize = 40;
const FRAME_HOLD_US: u64 = 1_200_000;

struct FrameAcc {
    last_end: Option<Instant>,
    last_flush: Instant,
    w_used: Vec<u32>,
    w_gap: Vec<u32>,
    w_seg: Vec<[u32; 8]>,
    w_sub: Vec<[u32; 10]>,
    r_used: Vec<u32>,
    r_gap: Vec<u32>,
    cells: u32,
    bw: i32,
    bh: i32,
    sty: &'static str,
}

fn frames() -> &'static Mutex<FrameAcc> {
    FRAMES.get_or_init(|| {
        Mutex::new(FrameAcc {
            last_end: None,
            last_flush: Instant::now(),
            w_used: Vec::new(),
            w_gap: Vec::new(),
            w_seg: Vec::new(),
            w_sub: Vec::new(),
            r_used: Vec::new(),
            r_gap: Vec::new(),
            cells: 0,
            bw: 0,
            bh: 0,
            sty: "-",
        })
    })
}

/// 分位数：先排序再按下标取，四舍五入到边界。
fn pct(v: &mut [u32], q: f64) -> u32 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    let i = ((v.len() as f64 - 1.0) * q).round() as usize;
    v[i.min(v.len() - 1)]
}

fn frame_begin() -> (Instant, u32) {
    let now = Instant::now();
    let mut gap = 0u32;
    if let Ok(a) = frames().lock() {
        if let Some(e) = a.last_end {
            gap = now.duration_since(e).as_micros() as u32;
        }
    }
    (now, gap)
}

/// 把一桶帧样本汇成一行写进 XSEE_TRACE 那个文件。
fn frame_emit(tag: &str, mut used_v: Vec<u32>, mut gap_v: Vec<u32>, cells: u32, bw: i32, bh: i32, sty: &str) {
    let n = used_v.len();
    if n == 0 {
        return;
    }
    let paint_avg = used_v.iter().sum::<u32>() as f64 / n as f64;
    let gap_avg = gap_v.iter().sum::<u32>() as f64 / n as f64;
    let (u_min, u_p50, u_p95, u_max) = (
        pct(&mut used_v, 0.0),
        pct(&mut used_v, 0.5),
        pct(&mut used_v, 0.95),
        pct(&mut used_v, 1.0),
    );
    let (g_min, g_p50, g_p95, g_max) = (
        pct(&mut gap_v, 0.0),
        pct(&mut gap_v, 0.5),
        pct(&mut gap_v, 0.95),
        pct(&mut gap_v, 1.0),
    );
    if std::env::var_os("XSEE_TRACE").is_none() {
        return;
    }
    // 分段耗时只属于滚轮帧那一行；other 那行不能把它们取走清零。
    let seg = if tag == "wheel" {
        seg_take().map(|s| {
            format!(
                " seg_back={:.0} seg_bar={:.0} seg_tree={:.0} seg_grid={:.0} seg_pv={:.0} seg_side={:.0} seg_pop={:.0} seg_blt={:.0}",
                s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]
            )
        }).unwrap_or_default()
    } else {
        String::new()
    };
    let sub = if tag == "wheel" {
        sub_take().map(|s| format!(
            " seg_ground={:.0} seg_cells={:.0} seg_tail={:.0} seg_fill={:.0} seg_dib={:.0} seg_clip={:.0} seg_txt={:.0} seg_ring={:.0} seg_dim={:.0} seg_grad={:.0}",
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7], s[8], s[9]
        ))
        .unwrap_or_default()
    } else {
        String::new()
    };
    // 这一批里真重缩了几次：正常只有第一次遇到那一格才缩，
    // 数字贴着格子数就说明在每帧重缩，那这缓存白做。
    let reb = if tag == "wheel" {
        format!(" reb={}", RESAMPLES.swap(0, Ordering::Relaxed))
    } else {
        String::new()
    };
    // 列表帧里有多少走了条带、多少整块重画：band=0/N 就是条带一次都没生效。
    let band = if tag == "wheel" {
        format!(
            " band={}/{}",
            BAND_FRAMES.swap(0, Ordering::Relaxed),
            GRID_FRAMES.swap(0, Ordering::Relaxed)
        )
    } else {
        String::new()
    };
    let Some(mut guard) = trace_file() else { return };
    let Some(f) = guard.as_mut() else { return };
    let _ = writeln!(
        f,
        "ev=fps src={} sty={} n={} cells={} buf={}x{} paint_avg={:.0} paint_min={} paint_p50={} paint_p95={} paint_max={} gap_avg={:.0} gap_min={} gap_p50={} gap_p95={} gap_max={} fps={:.1}{}{}{}{}",
        tag, sty, n, cells, bw, bh,
        paint_avg, u_min, u_p50, u_p95, u_max,
        gap_avg, g_min, g_p50, g_p95, g_max,
        if gap_avg > 0.0 { 1_000_000.0 / gap_avg } else { 0.0 },
        seg,
        sub,
        reb,
        band,
    );
    let _ = f.flush();
}

/// 遥测文件：设了 XSEE_TRACE 才开，开了就一次握到底，批行和每帧明细共用这一份。
fn trace_file() -> Option<std::sync::MutexGuard<'static, Option<std::fs::File>>> {
    static F: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
    let slot = F.get_or_init(|| match std::env::var_os("XSEE_TRACE") {
        Some(p) => Mutex::new(
            std::fs::OpenOptions::new().create(true).append(true).open(&p).ok(),
        ),
        None => Mutex::new(None),
    });
    slot.lock().ok()
}

/// 把这一批滚轮帧逐帧落一行。查"最差那一帧贵在哪一段"只能靠它，
/// 汇总行里的 p95 只会告诉你有个洞，不会说是哪个洞。
fn frame_rows(seg: &[[u32; 8]], sub: &[[u32; 10]], used: &[u32], gap: &[u32], sty: &str) {
    let Some(mut guard) = trace_file() else { return };
    let Some(f) = guard.as_mut() else { return };
    for i in 0..used.len().min(seg.len()).min(sub.len()) {
        let s = &seg[i];
        let c = &sub[i];
        let _ = writeln!(
            f,
            "ev=frm sty={} paint={} gap={} back={} bar={} tree={} grid={} pv={} side={} pop={} blt={} \
             ground={} cells={} tail={} fill={} dib={} clip={} txt={} ring={} dim={} grad={}",
            sty, used[i], gap[i],
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
            c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8], c[9],
        );
    }
    let _ = f.flush();
}

/// 攒够一批、或者压着不满批已经过了一阵，就落一行。
/// 快滚时多次滚轮会被并成一帧，只按固定帧数攒的话一批都出不来。
fn frame_flush_due(a: &mut FrameAcc, force: bool) {
    let nw = a.w_used.len();
    let nr = a.r_used.len();
    if nw + nr == 0 {
        a.last_flush = Instant::now();
        return;
    }
    let held = a.last_flush.elapsed().as_micros() as u64;
    if !force && nw < FRAME_BATCH && nr < FRAME_BATCH && held < FRAME_HOLD_US {
        return;
    }
    let (cells, bw, bh, sty) = (a.cells, a.bw, a.bh, a.sty);
    if nw > 0 {
        let (used, gapv) = (std::mem::take(&mut a.w_used), std::mem::take(&mut a.w_gap));
        let (sg, sb) = (std::mem::take(&mut a.w_seg), std::mem::take(&mut a.w_sub));
        // 明细行要用未排序的样本，汇总行排序时会就地打乱，所以先写明细。
        frame_rows(&sg, &sb, &used, &gapv, sty);
        frame_emit("wheel", used, gapv, cells, bw, bh, sty);
    }
    if nr > 0 {
        frame_emit("other", std::mem::take(&mut a.r_used), std::mem::take(&mut a.r_gap), cells, bw, bh, sty);
    }
    a.last_flush = Instant::now();
}

unsafe fn frame_end(t0: Instant, gap: u32, hwnd: HWND) {
    if std::env::var_os("XSEE_TRACE").is_none() {
        return;
    }
    let used = t0.elapsed().as_micros() as u32;
    let src = PAINT_SRC.swap(0, Ordering::Relaxed);
    let rc = client(hwnd);
    let (cells, sty) = app()
        .lock()
        .ok()
        .map(|g| ((g.vis.1.saturating_sub(g.vis.0)) as u32, g.th.style.label()))
        .unwrap_or((0, "-"));
    let Ok(mut a) = frames().lock() else { return };
    a.last_end = Some(Instant::now());
    a.cells = cells;
    a.sty = sty;
    a.bw = rc.right.max(0);
    a.bh = rc.bottom.max(0);
    if src == 1 {
        a.w_used.push(used);
        a.w_gap.push(gap);
        if let (Ok(s), Ok(c)) = (LAST_SEG.lock(), LAST_SUB.lock()) {
            a.w_seg.push(*s);
            a.w_sub.push(*c);
        }
    } else {
        a.r_used.push(used);
        a.r_gap.push(gap);
    }
    frame_flush_due(&mut a, false);
}

/// 心跳里补一句：滚完停手时最后一批不满整数的帧也要落文件。
fn frame_tick() {
    if std::env::var_os("XSEE_TRACE").is_none() {
        return;
    }
    let Ok(mut a) = frames().lock() else { return };
    frame_flush_due(&mut a, false);
}

#[derive(Clone, Copy, PartialEq)]
enum SortKey {
    Name,
    Ext,
    Size,
    Type,
    Date,
    Image,
    Desc,
    /// 库里记的入库时间。排在最后，前面几档的编号（会存进界面数）不动。
    Added,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Ik {
    Folder,
    Drive,
    Pc,
    Star,
    Pic,
    File,
    Up,
    Prev,
    Next,
    Find,
    Get,
    Print,
    Edit,
    Move,
    Copy,
    Del,
    Prop,
    Opt,
    Skin,
    View,
    Net,
    Clock,
    /// 路径栏右侧四个视图档的图标：原来是块字符（▦▪☰≡），字号一换就歪。
    VThb,
    VSm,
    VList,
    VDetail,
}

struct BtnDef {
    id: usize,
    label: &'static str,
    icon: Ik,
    sep: bool,
    /// 设计稿里 移到/复制到/删除 是灰显档，点了不做事。
    off: bool,
    /// 标签后面带 ▾ 的下拉档。
    caret: bool,
}

const BTNS: &[BtnDef] = &[
    BtnDef { id: M_BACK, label: "上一文件夹", icon: Ik::Prev, sep: false, off: false, caret: false },
    BtnDef { id: M_FWD, label: "下一文件夹", icon: Ik::Next, sep: false, off: false, caret: false },
    BtnDef { id: M_UP, label: "上一层", icon: Ik::Up, sep: false, off: false, caret: false },
    BtnDef { id: M_FIND, label: "查找", icon: Ik::Find, sep: true, off: false, caret: false },
    BtnDef { id: M_EDIT, label: "编辑", icon: Ik::Edit, sep: true, off: false, caret: true },
    BtnDef { id: M_MOVE, label: "移到", icon: Ik::Move, sep: true, off: false, caret: false },
    BtnDef { id: M_COPY, label: "复制到", icon: Ik::Copy, sep: false, off: false, caret: false },
    BtnDef { id: M_DEL, label: "删除", icon: Ik::Del, sep: false, off: false, caret: false },
    BtnDef { id: M_PROP, label: "属性", icon: Ik::Prop, sep: false, off: false, caret: false },
    BtnDef { id: M_LOOK, label: "外观", icon: Ik::Skin, sep: false, off: false, caret: true },
];

const M_MIN: usize = 250;
const M_MAX: usize = 251;
const M_CLOSE: usize = 252;

#[derive(Clone, Copy)]
struct MenuItem {
    label: &'static str,
    key: &'static str,
    id: usize,
    check: bool,
    off: bool,
    sep: bool,
    sub: Option<&'static [MenuItem]>,
}

const fn mi(label: &'static str, key: &'static str, id: usize) -> MenuItem {
    MenuItem { label, key, id, check: false, off: false, sep: false, sub: None }
}
const fn mick(label: &'static str, key: &'static str, id: usize, check: bool) -> MenuItem {
    MenuItem { label, key, id, check, off: false, sep: false, sub: None }
}
const fn msep() -> MenuItem {
    MenuItem { label: "", key: "", id: 0, check: false, off: false, sep: true, sub: None }
}
const fn msub(label: &'static str, sub: &'static [MenuItem]) -> MenuItem {
    MenuItem { label, key: "", id: 0, check: false, off: false, sep: false, sub: Some(sub) }
}

/// 工具栏 ▾ 菜单的一项。和标题栏那套 MenuItem 分开：皮肤名、历史路径都是
/// 打开菜单那一刻才排的，标签得是 String。
#[derive(Clone)]
struct BItem {
    label: String,
    id: usize,
    check: bool,
    off: bool,
    sep: bool,
    sub: Vec<BItem>,
}

impl BItem {
    fn new(label: impl Into<String>, id: usize) -> BItem {
        BItem { label: label.into(), id, check: false, off: false, sep: false, sub: Vec::new() }
    }
    fn checked(label: impl Into<String>, id: usize, on: bool) -> BItem {
        BItem { check: on, ..BItem::new(label, id) }
    }
    fn off(label: impl Into<String>) -> BItem {
        BItem { off: true, ..BItem::new(label, 0) }
    }
    fn sep() -> BItem {
        BItem { label: String::new(), id: 0, check: false, off: false, sep: true, sub: Vec::new() }
    }
    fn submenu(label: impl Into<String>, sub: Vec<BItem>) -> BItem {
        BItem { sub, ..BItem::new(label, 0) }
    }
}

/// 工具栏 ▾ 按钮弹出的自绘菜单。同一时刻只有一张；配色、行高、图标列都照
/// 标题栏那套弹层来，皮肤一换它跟着换。
struct BMenu {
    /// 按钮矩形（客户区坐标），面板挂在它左下角
    anchor: RECT,
    items: Vec<BItem>,
    rect: RECT,
    rows: Vec<(usize, RECT)>,
    sub_open: Option<usize>,
    sub_rect: RECT,
    sub_rows: Vec<(usize, RECT)>,
    /// 指针压在哪一行（-1 = 没有）；换行才重画
    hover: i32,
    sub_hover: i32,
}

const VIEW_PANES: &[MenuItem] = &[
    mick("文件夹(F)", "Ctrl+Shift+F", M_PANE_TREE, true),
    mick("预览", "Ctrl+Shift+E", M_PANE_PV, true),
];
const MAIN_TOOLBAR: &[MenuItem] = &[
    mick("按钮文字(L)", "", M_TB_TEXT, true),
    mi("无文字(N)", "", M_TB_NOTEXT),
    mi("标签在右侧(R)", "", M_TB_RIGHT),
    mick("标签在下方(B)", "", M_TB_BELOW, true),
    msep(),
    mi("隐藏工具栏(H)", "", M_TB_HIDE),
];
const THUMB_VIEWS: &[MenuItem] = &[
    mick("缩略图(T)", "F8", M_VGRID, true),
    mi("图标(M)", "F9", M_VSMALL),
    mi("列表(L)", "F10", M_VLIST),
    mi("详细资料(D)", "F11", M_VDETAIL),
];
const ARRANGE: &[MenuItem] = &[
    mick("按名称", "NumPad 1", M_SORT_NAME, true),
    mi("按扩展名(E)", "NumPad 2", M_SORT_EXT),
    mi("按大小", "NumPad 3", M_SORT_SIZE),
    mi("按类型", "NumPad 4", M_SORT_TYPE),
    mi("按日期", "NumPad 5", M_SORT_DATE),
    mi("按图像属性", "NumPad 6", M_SORT_IMAGE),
    mi("按入库时间", "", M_SORT_ADDED),
    mi("按描述", "NumPad 0", M_SORT_DESC),
    msep(),
    mick("正向放映", "", M_SORT_FWD, true),
    mi("反向放映", "", M_SORT_REV),
];
const GO_MENU: &[MenuItem] = &[
    mi("后退", "Alt+←", M_BACK),
    mi("前进", "Alt+→", M_FWD),
    mi("上一层", "Backspace", M_UP),
];
const WALLPAPER: &[MenuItem] = &[
    mi("居中(C)", "", M_WALL),
    mi("平铺(T)", "", M_WALL + 1),
    mi("还原(R)", "", M_WALL + 2),
];

const FILE_MENU: &[MenuItem] = &[
    mi("新建文件夹", "Ctrl+Shift+N", M_NEWDIR),
    mi("新建窗口(W)", "Ctrl+N", M_NEWWIN),
    msep(),
    mi("打开(O)...", "Enter", M_OPEN),
    mi("预览", "Ctrl+Shift+V", M_VIEWER),
    msep(),
    mi("属性...", "Alt+Enter", M_PROP),
    mi("结束(X)", "Alt+X", M_QUIT),
];
const EDIT_MENU: &[MenuItem] = &[
    mi("剪切(T)", "Ctrl+X", M_CUT),
    mi("复制(C)", "Ctrl+C", M_COPYF),
    mi("复制图像(Y)", "Ctrl+Shift+C", M_COPYIMG),
    mi("复制到文件夹(O)...", "Alt+C", M_COPY),
    mi("移到文件夹(M)...", "Alt+M", M_MOVE),
    mi("粘贴文件(P)", "Ctrl+V", M_PASTE),
    mi("删除(D)", "DEL", M_DEL),
    msep(),
    mi("重命名(R)", "F2", M_RENAME),
    mi("重命名系列...", "", M_BATCH),
    msep(),
    mi("选取全部文件(L)", "Ctrl+A", M_ALL),
];
const VIEW_MENU: &[MenuItem] = &[
    msub("浏览器窗格(B)", VIEW_PANES),
    mick("图片分类(R)", "Ctrl+Shift+O", M_ORG, true),
    mick("批量图片转换(B)", "Ctrl+B", M_CONV_PANEL, false),
    msub("主工具栏(O)", MAIN_TOOLBAR),
    mick("路径栏(A)", "", M_BAR_PATH, true),
    mick("状态栏(B)", "", M_BAR_STATUS, true),
    msep(),
    msub("缩略图(T)", THUMB_VIEWS),
    msub("排列图标(I)", ARRANGE),
    msub("转到(G)", GO_MENU),
];
const TOOL_MENU: &[MenuItem] = &[
    mi("格式转换(E)...", "Ctrl+E", M_CONV),
    mi("查找(F)...", "Ctrl+F", M_FIND),
    mi("文件关联(F)...", "", M_ASSOC),
    msep(),
    mi("在编辑器中打开(O)", "", M_EDITOR),
    mi("以外部程序打开(P)...", "", M_EXTAPP),
    msep(),
    msub("壁纸(W)", WALLPAPER),
];
/// 中间列表的过滤，挂在缩略图右键菜单里。分类时把已经分过的、非图片和文件夹先藏掉，
/// 眼前只剩没干的活。勾选状态不写死在这里，由绘制那侧按 App 上的开关现算。
const FILTER_MENU: &[MenuItem] = &[
    mi("显示全部(A)", "", M_FILT_ALL),
    msep(),
    mick("隐藏文件夹(D)", "", M_FILT_DIRS, false),
    mick("隐藏非图片文件(N)", "", M_FILT_NONIMG, false),
    mick("隐藏已分类图片(C)", "", M_FILT_CAT, false),
];
/// 库并成一份 catalog.tsv 之后，这几条的口径：导出=把这一份连上一版打包，
/// 导入=整份覆盖（确认框里写明白，不是合并），维护页管统计/检查/缩略图/位置。
const DB_MENU: &[MenuItem] = &[
    mi("立即查找(F)", "Ctrl+F", M_FIND),
    mi("分类数据库...", "", M_CATDB),
    mi("缩略图数据库...", "", M_THUMBDB),
    mi("重建页面缩略图", "F5", M_RESCAN),
    msep(),
    mi("导出数据库备份...", "", M_DBBACKUP),
    mi("导入备份(覆盖当前)...", "", M_DBIMPORT),
];
const HELP_MENU: &[MenuItem] = &[
    mi("帮助(H)", "F1", M_HELP),
    msep(),
    mi("关于 Xsee", "", M_ABOUT),
];

const MENUS: &[(&str, &[MenuItem])] = &[
    ("文件(F)", FILE_MENU),
    ("编辑(E)", EDIT_MENU),
    ("查看(V)", VIEW_MENU),
    ("工具(T)", TOOL_MENU),
    ("数据库(D)", DB_MENU),
    ("帮助(H)", HELP_MENU),
];

// 缩略图右键菜单照逆向的 #20111 排，文案和顺序与原来一致，只是改走自绘弹层。
const CTX_ARRANGE: &[MenuItem] = &[
    mi("按名称", "", M_SORT_NAME),
    mi("按大小", "", M_SORT_SIZE),
    mi("按类型", "", M_SORT_TYPE),
    mi("按日期", "", M_SORT_DATE),
    mi("按入库时间", "", M_SORT_ADDED),
    msep(),
    mi("正向放映", "", M_SORT_FWD),
    mi("反向放映", "", M_SORT_REV),
];
/// 整理面板的类别右键菜单。原版 # 项里只留这几样，其余按用户要求裁掉。
const CAT_CTX: &[MenuItem] = &[
    mi("新建类别(&N)", "", M_CAT_NEW),
    msep(),
    mi("复制(&C)", "", M_CAT_COPY),
    mi("粘贴(&P)", "", M_CAT_PASTE),
    msep(),
    mi("删除(&D)", "", M_CAT_DEL),
    mi("编辑名称(&R)", "", M_CAT_RENAME),
];
const CTX_MENU: &[MenuItem] = &[
    mi("打开(&O)", "Enter", M_OPEN),
    msep(),
    msub("查看(&V)", THUMB_VIEWS),
    msub("排列图标(&I)", CTX_ARRANGE),
    msub("过滤(&F)", FILTER_MENU),
    msep(),
    mi("转到文件", "", M_EXPL),
    msep(),
    mi("剪切(&T)", "Ctrl+X", M_CUT),
    mi("复制(&C)", "Ctrl+C", M_COPYF),
    mi("粘贴(&P)", "Ctrl+V", M_PASTE),
    mi("删除(&D)", "Del", M_DEL),
    mi("重命名(&R)", "F2", M_RENAME),
    mi("批量重命名(&N)...", "", M_BATCH),
    msep(),
    mi("参考图画布模式", "", M_REF),
    mi("以图搜图(&S)", "", M_FINDSIM),
    mi("编辑图像(&E)...", "", M_EDITOR),
    msep(),
    mi("格式转换(&E)...", "Ctrl+E", M_CONV),
    msep(),
    mi("选取全部文件(&L)", "Ctrl+A", M_ALL),
    mi("新建文件夹", "", M_NEWDIR),
    msep(),
    mi("复制路径(&C)", "", M_COPYPATH),
    mi("刷新(&F)", "F5", M_RESCAN),
    mi("属性(&P)", "", M_PROP),
];

/// 目录树右键：只做文件夹本身的操作。查看/排列/过滤属于中间列表，
/// 画布/找相似/编辑/格式转换/批量改名属于图片文件，都不在这张菜单上。
const TREE_CTX: &[MenuItem] = &[
    mi("新建文件夹(&N)", "Ctrl+Shift+N", M_NEWDIR),
    mi("粘贴(&P)", "Ctrl+V", M_PASTE),
    msep(),
    mi("剪切(&T)", "Ctrl+X", M_CUT),
    mi("复制(&C)", "Ctrl+C", M_COPYF),
    mi("删除(&D)", "Del", M_DEL),
    mi("重命名(&R)", "F2", M_RENAME),
    mi("复制路径(&C)", "", M_COPYPATH),
    mi("属性(&P)", "", M_PROP),
    mi("转到文件", "", M_EXPL),
    msep(),
    mi("刷新(&F)", "", M_TREE_REFRESH),
];

/// 目录树右键那张菜单里哪些条目该画灰：盘符、主目录不能剪、删、改名；
/// 盘根没有上一级，“转到文件”没地方可去。
/// 绘制和点击都走这一条判断，免得画灰了还能点，或者能点的反而画灰。
fn tree_ctx_off(a: &App, id: usize) -> bool {
    let Some(d) = a.ctx_dir.as_deref() else { return false };
    match id {
        M_RENAME | M_CUT | M_DEL => !a.tree_editable(d),
        // 盘根没有上一级，“转到文件”没地方可去。
        M_EXPL => Path::new(d).parent().is_none(),
        _ => false,
    }
}

/// 这次文件操作落在哪儿：(是不是树落点, 树上的路径)。右键那一行最优先，
/// 其次是键盘焦点所在的树行。焦点在树上但那一行没有文件落点（“此电脑”）时
/// 返回 (true, None)——不许回落去动格子选区，那正是“树上按 Del 删别处文件”的老毛病。
fn tree_target() -> (bool, Option<String>) {
    let g = app().lock().unwrap();
    if g.ctx_dir.is_some() {
        return (true, g.ctx_dir.clone());
    }
    if !g.tree_kb {
        return (false, None);
    }
    (true, g.tree_focus_dir())
}

/// 中间列表右键：图片专用的那几条只有落点真是一张图时才亮；文件夹、非图片文件
/// 一律画灰，点了也不执行。绘制和点击共用这一条，跟 tree_ctx_off 同一个道理。
/// 空白处不动图片项——那几条按格子里的选区走，选区还在就该能用。
/// 顶栏和工具栏那些菜单也用同一个绘制函数，靠 ctx_at 把范围圈在右键弹层里。
fn grid_ctx_off(a: &App, id: usize) -> bool {
    if a.ctx_at.is_none() || a.ctx_tree {
        return false;
    }
    if id == M_OPEN {
        return a.ctx_blank;
    }
    let image_only = matches!(id, M_REF | M_FINDSIM | M_EDITOR | M_CONV | M_BATCH);
    image_only && !a.ctx_blank && a.ctx_img.is_none()
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Grid,
    Small,
    List,
    Detail,
}

/// 缩略图高度的下限，逻辑像素。定在 100 是为了压住以图搜图那条"短边 ≥96"的线
/// （`sim::MIN_SIDE`）——矮过它，看过的图会静默记不进指纹库。
const THUMB_H_MIN: i32 = 100;
const THUMB_H_MAX: i32 = 150;
const THUMB_H_STEP: i32 = 5;
/// 内存里最多留多少缩略图像素（原尺寸那份 + 按显示尺寸缩好的那份）。
/// 按字节算不按张数算：默认缩略图档一张约 60 KB，256 MB 足够一两千张的目录
/// 整个常驻，滚回之前看过的地方不用再从缓存里读一遍。目录更大、缩略图调得更大
/// 或者一张特别占地方时才退回"只留视区附近"。
const THUMB_MEM_MAX: usize = 256 * 1024 * 1024;
/// 图标档那枚格式牌/文件夹图标在屏幕上的边长，物理像素。
const SMALL_ICON: i32 = 60;

impl Mode {
    /// 只有缩略图档只列图片；图标、列表、详细资料都要连非图片文件一起列。
    fn all_files(self) -> bool {
        !matches!(self, Mode::Grid)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Src {
    Folder,
    Category(i64),
    Search,
}

#[derive(Clone, Copy, PartialEq)]
enum DlgKind {
    Convert,
    /// 某种目标格式的详细选项（截图里的 JPEG 对话框）
    FmtOpt,
    Search,
    /// 批量重命名：模板 + 编号 + 大小写 + 预览表
    BatchRename,
    NewCat,
    RenameCat,
    /// 皮肤设置：字体、字号、配色槽（文字色在字体页按版块调）。
    Skin,
    /// 分类数据库：类别/归类/入库时间的统计和检查，独立一页。
    CatDb,
    /// 缩略图数据库：缓存、重建、清指纹、扫描文件夹，独立一页（以图搜图的账在这里）。
    ThumbDb,
    /// 扫描名单某一项的排除规则编辑：两个输入框 + 保存清理。压在缩略图数据库页上。
    ScanRule,
    /// 以图搜图：指纹库管理 + 检索条件 + 结果表 + 双预览，一整页。
    SimSearch,
    /// 文件关联：勾要登记的格式，写 HKCU 的候选列表。
    Assoc,
    /// 确认框和提示框。皮肤配色自绘，取代系统 MessageBox 的浅色窗。
    Ask,
}

struct Dlg {
    kind: DlgKind,
    rect: RECT,
    /// 转换格式下标，或设置窗口的选项卡下标。
    fmt: usize,
    /// 格式设置窗口里正在编辑的那份副本；取消就整个丢掉。
    opt: encode::Opt,
    scope_db: bool,
    /// 覆盖下拉框展开着；批量重命名的帮助页也用它。
    combo_open: bool,
    /// 以下四项只有 Ask 用得上：标题、正文、有没有"否"、点确认后要重发的命令。
    ask_title: String,
    ask_body: String,
    ask_yes: bool,
    ask_cmd: usize,
    /// 皮肤设置窗的草稿：改动先落这里，同时实时叠到界面上给他看。
    look_draft: theme::Look,
    /// 打开皮肤设置窗那一刻的个人覆盖。没覆盖皮肤就退出时退回这一份。
    skin_before: theme::Look,
    /// 这次退出保不保住草稿：只有"覆盖当前皮肤"和"另存为"置真，其余出口一律退回 skin_before。
    look_keep: bool,
    /// 另存为的名字输入框开着。开着时点确定=先存成新皮肤再应用。
    save_as: bool,
    /// 删除皮肤要先按一次武装、第二次才真删：对话框是模态的，叠不了确认框。
    arm_del: bool,
}

#[derive(Clone)]
struct Node {
    path: String,
    name: String,
    open: bool,
    loaded: bool,
    kids: Vec<Node>,
}

/// 拖放落点画在哪：格子视图的第几项，或左树第几行。
#[derive(PartialEq, Clone, Copy)]
enum Hl {
    Grid(usize),
    Tree(usize),
    /// 列表区本身，也就是当前正在看的那个文件夹。
    Here,
}

/// 按显示尺寸缩好的那一份。缩一次存下来，滚动时每帧只贴不重算。
#[derive(Clone)]
struct Fit {
    w: i32,
    h: i32,
    px: Vec<u8>,
}

/// 到位的缩略图：GDI 直接能贴的 BGRA 紧凑缓冲。
#[derive(Clone)]
struct Img {
    w: u32,
    h: u32,
    px: Vec<u8>,
    fit: Option<Fit>,
}

impl Img {
    /// 这一张实际占的内存：解码出来的那份，加上按显示尺寸缩好的那份。
    fn bytes(&self) -> usize {
        self.px.len() + self.fit.as_ref().map(|f| f.px.len()).unwrap_or(0)
    }
}

/// 以图搜图结果表的一行。差几位、大小、改期在结果回来那一下一次取好，
/// 绘制时每帧只读内存，不再碰盘。
#[derive(Clone)]
struct SimHit {
    path: String,
    dist: u32,
    size: u64,
    mtime: u32,
}

struct App {
    th: Theme,
    /// 当前配色。只决定颜色，不再决定版式。
    skin: String,
    /// 版式：None = 内置的圆角仪表，Some = 某套开源图标库的 key。
    layout: Option<String>,
    density: String,
    /// 当前皮肤的个人覆盖：字体、字号、槽位颜色。按皮肤名分文件存。
    look: theme::Look,
    scale: f64,
    mode: Mode,
    sort: SortKey,
    sort_desc: bool,
    /// 缩略图高度（逻辑像素）。宽度按格子的宽:高比例跟着它走，所以只有这一个数可调。
    thumb_h: i32,
    dir: PathBuf,
    entries: Vec<scan::Entry>,
    /// entries[0] 是不是那个"返回上一级"的虚拟条目。
    has_up: bool,
    sel: usize,
    /// 多选集合按**文件路径**记，不按 entries 下标。换视图时列表范围会变（缩略图档
    /// 只列图片，其余三档连非图片一起列），按下标记一重列就串到别的文件上，
    /// 看着就是"缩略图里框选的，切到列表没框选"。
    marked: HashSet<String>,
    /// Shift 点选 / Shift 方向键量范围的那一头。点了别处、换了目录就跟着光标走。
    anchor: usize,
    /// 光标那一项是不是真亮着：Ctrl 点掉最后一个文件后焦点还在，但已不算选中。
    sel_lit: bool,
    /// 键盘焦点在文件夹树上（点了树，或 Tab/F6 切过去）；这时方向键走树不走格子。
    tree_kb: bool,
    /// 格子空白处拖出来的框选：(起 x, 起 y, 当前 x, 当前 y)。
    band: Option<(i32, i32, i32, i32)>,
    /// Ctrl 框选按下那一刻已有的选区，抬起时并进去。
    band_add: HashSet<String>,
    hover: usize,
    /// 指针底下压着的是哪个按钮命中区。按钮高亮靠它，不在格子里也要重画。
    hover_btn: Option<usize>,
    /// 鼠标停在哪一项顶层菜单上。菜单条不在 a.btns 里，不单独记这个的话
    /// 鼠标扫过去 hover 没人发现，界面就"移上去没反应"。
    hover_menu: Option<usize>,
    scroll: i32,
    /// 上一帧画进常驻位图的那个滚动位置。这一帧只列表在动、而且动的距离不到一屏，
    /// 就把位图里那一屏整体搬到新位置，只补新露出来的那一条 —— 靠的就是这两个数之差。
    paint_scroll: i32,
    /// 常驻位图里那一屏还作不作数：刚重建过、或者别的东西也改了（新缩略图到齐），
    /// 就不作数，这一帧整块列表从头画。
    paint_ok: bool,
    /// 置真表示上一帧存下来的格子内容已经过时（出图陆续到齐就是它），下一帧不许走条带。
    grid_stale: bool,
    /// 滚动速度（像素/秒）。滚轮往这里加冲量，帧节拍按它积分走位、乘摩擦衰减，
    /// 松手后自己滑一段再停；别的来源动 scroll 就把它作废。
    scroll_vel: f64,
    /// 最后一次滚动的时刻。停下后这 400 毫秒里心跳只攒刷新不整窗刷，免得刚停就被打断。
    scroll_until: Instant,
    /// 滚动动画期间被按下不表的整窗刷新（出图到齐、进度走光），滚停那一帧一次补齐。
    anim_defer: bool,
    tscroll: i32,
    thumbs: HashMap<usize, Img>,
    dims: HashMap<usize, (u32, u32)>,
    bpps: HashMap<usize, u32>,
    /// 详细资料档每一行的大小、修改时间。按完整路径记，画的时候不再每帧碰盘。
    stats: HashMap<String, (u64, String)>,
    queued: HashSet<usize>,
    pool: Option<Pool>,
    got: usize,
    err: usize,
    tree: Vec<Node>,
    tsel: String,
    home: String,
    /// 这一次换目录别动树的滚动位置：树里点出来的导航要让那一行待在原地。
    tree_noscroll: bool,
    /// 按住树滚动条滑块拖动时记的（指针与滑块顶的差、槽顶、滑块可行程、滚动上限）。
    tree_drag: Option<(i32, i32, i32, i32)>,
    /// 缩略图区滚动条拖动，记法同 tree_drag。
    grid_drag: Option<(i32, i32, i32, i32)>,
    /// 详细资料档五条列宽（逻辑像素，从左到右：名称、类型、尺寸、修改时间、大小）。
    /// 名称、大小两格 0 = 没调过，画的时候铺满剩下的宽度；按下任意一条列线时定格成
    /// 眼前看到的宽，之后五列各认各的，拖哪条都只改它自己。
    col_w: [i32; 5],
    /// 正在拖的列边界：（这条线左边那一列在 col_w 里的序号、按下时的 x、按下时那一列的宽）。
    col_drag: Option<(usize, i32, i32)>,
    hist: Vec<String>,
    hpos: usize,
    left_w: i32,
    /// 查看菜单的窗格开关：左栏那两块，两块都关时整条左栏收掉。
    pane_tree: bool,
    pane_pv: bool,
    /// 三条横条：主工具栏、路径栏、状态栏。关掉的那条在 zones() 里高度归零。
    bar_main: bool,
    bar_path: bool,
    bar_status: bool,
    /// 工具栏按钮的文字开关，和文字摆在图标右边还是下边。
    /// 密度只决定"放不放得下"（`bar_below` 那条判据），不决定人的选择。
    tb_text: bool,
    tb_below: bool,
    /// 标题栏角上那份图标：(像素边长, 句柄)。尺寸随密度/缩放变，变了才重取一份。
    tb_icon: (i32, HICON),
    /// 左窗格下半（预览）的整块高度，含窗格头，可拖。
    pv_h: i32,
    /// 上一次画预览时那个图盒的实际宽度，取图线程按它要像素。
    pv_w: i32,
    /// 这一层合计字节数，后台线程点好了才填。没点到之前这格只有张数。
    total_sz: Option<u64>,
    /// 第几次数总量的活，用来认后台交回来的那份结果是不是当前这一层的。
    sz_seq: usize,
    /// 这一池缩略图开跑的时刻，只用来算"每秒出几张"。
    t0: Instant,
    /// 本目录开始加载的时刻。缩放重排不动它，所以用时不会被滚没。
    t_dir: Instant,
    /// 全部就绪那一刻的用时；就绪之后冻住，换目录才重新起表。
    ready_sec: Option<f64>,
    /// 进本目录时全局命中计数的快照。状态栏要说的是"这一目录命中几张"，
    /// 直接印全局累计会让人以为缓存里只有那么点。
    hit0: u64,
    drag: Option<(i32, i32, i32)>,
    /// 在格子上按下的位置和条目；移动超过系统拖拽阈值就转成拖文件。
    drag_seed: Option<(i32, i32, usize)>,
    /// 树行拖起来的起点：按下时记下面包路径，拖过系统阈值就真的把这一个文件夹拖出去。
    tree_seed: Option<(i32, i32, String)>,
    /// 正在往外拖时，指针底下那个能收文件的文件夹；画出来给人当落点提示。
    drop_hl: Option<Hl>,
    btns: Vec<(usize, RECT)>,
    menus: Vec<(usize, RECT)>,
    pop_rows: Vec<(usize, RECT)>,
    sub_rows: Vec<(usize, RECT)>,
    open_menu: Option<usize>,
    open_sub: Option<usize>,
    /// 弹层里指针这一刻压着的主层行和子层行。高亮本身按指针现算，但窗口不知道
    /// "指针换了一行"，不重画就停在旧行上——菜单看着就像慢半拍、不跟手。
    pop_hover: Option<usize>,
    sub_hover: Option<usize>,
    pop_rect: RECT,
    sub_rect: RECT,
    /// 右键弹层：锚点是客户区坐标，rows 和 rect 由绘制时算好，命中直接用。
    ctx_at: Option<(i32, i32)>,
    ctx_rect: RECT,
    ctx_rows: Vec<(usize, RECT)>,
    ctx_sub: Option<usize>,
    ctx_sub_rect: RECT,
    ctx_sub_rows: Vec<(usize, RECT)>,
    /// 这次右键是不是冲着目录树某个文件夹来的：树上右键时记那行的路径，
    /// 复制/粘贴/新建/删除四条命令按文件夹本体算。弹层一关就清掉。
    ctx_dir: Option<String>,
    /// 这次右键落在中间列表哪张图上：以图搜图从右键进来时直接拿它当查询图，
    /// 不经过全局选区——从右键到点菜单项之间选区被谁动了都不碍事。
    /// 弹层一关就清掉，跟 ctx_dir 同一个寿命。
    ctx_img: Option<String>,
    /// 这次中间列表右键落在空白处：没有可打开的东西，“打开”跟着画灰，
    /// 图片专用那几条也不对着别处的选区亮着。和 ctx_img 同一个寿命。
    ctx_blank: bool,
    /// 这次右键弹的是树那张菜单还是中间列表那张。两张菜单的条目不同，
    /// 绘制、悬停、点击都要按同一个开关取，不然点到的行和看到的那行会错位。
    ctx_tree: bool,
    /// 类别行的右键菜单，和缩略图右键分开存，互不覆盖。
    ctx_cat: Option<(i32, i32)>,
    ctx_cat_rect: RECT,
    ctx_cat_rows: Vec<(usize, RECT)>,
    /// 工具栏 ▾ 那几个按钮弹出的自绘菜单，开着才有。
    bmenu: Option<BMenu>,
    bar: RECT,
    path_bar: RECT,
    /// 地址栏文字矩形：指针落在这里面才当"选路径"，外面照旧走按钮和格子。
    addr_tr: RECT,
    /// 逐字量出来的 (UTF-16 偏移, 这个字的左边界 x)，首项是行首、末项是行尾。
    addr_xs: Vec<(usize, i32)>,
    /// 量边界时那串文字和它占的框宽，两个都对不上才重算。
    addr_src: String,
    addr_w: i32,
    /// addr_src 的 UTF-16 长度，选区偏移的上界。
    addr_n: usize,
    /// 整条放得下才给逐字选；被省略号截断时中间那截画的是别的内容，只能整条选中。
    addr_full: bool,
    /// 选区 = UTF-16 偏移区间 (起, 止)，起 == 止 视为没选中任何东西。
    addr_sel: Option<(usize, usize)>,
    /// 拖选按下的那一头（偏移），抬起就清。
    addr_anchor: Option<usize>,
    vis: (usize, usize),
    pt: (i32, i32),
    stamp: usize,
    font: HFONT,
    font_sm: HFONT,
    font_b: HFONT,
    font_num: HFONT,
    /// 地址栏那串路径：比小字正文再大四号，小了看不清目录名
    font_path: HFONT,
    /// 十块各自的四支笔，下标同 theme::ZONES。上面那五个是"正在画的那一块"用的笔，
    /// 每个 paint_ 进自己那块时换一次，几百处 txt() 就不用一个个写字号。
    zf: [ZoneFont; theme::NZ],
    msg: String,
    /// 提示纱的起点：剪切/复制那一刻开始，画到点自己收。
    flash: Option<Instant>,
    src: Src,
    view_title: String,
    org_open: bool,
    org_w: i32,
    org_scroll: i32,
    org_sel: i64,
    org_open_ids: HashSet<i64>,
    org_hits: Vec<(i64, RECT)>,
    org_checks: Vec<(i64, RECT)>,
    org_up: RECT,
    org_dn: RECT,
    /// 可编辑的类别树，整份自含在 data\catalog.tsv 里。
    cat: Option<catdb::CatEdit>,
    org_drag: Option<(i32, i32, i32, i32)>,
    /// 类别行拖到别的类别上就是改父级：记下被拖的 id 和起拖点，抬起时按落点执行。
    org_cat_drag: Option<(i64, i32, i32)>,
    org_cat_to: i64,
    /// 批量转换页开着。和整理页共用右侧那一条栏。
    conv_open: bool,
    /// 栏里当前是哪一页：0 整理，1 批量转换
    dock_tab: u8,
    /// 打开转换页时为放得下临时加宽的量和原宽；用户自己拖过分隔条就不还了
    conv_w_prev: i32,
    conv_w_auto: bool,
    conv_scroll: i32,
    conv_max_scroll: i32,
    conv_drag: Option<(i32, i32, i32, i32)>,
    /// 按住滑条一路拖：记的是哪一条（控件号）和轨道的左右两端，拖动与单击共用同一条换算。
    conv_sdrag: Option<(usize, i32, i32)>,
    /// 面板上五个原生输入框：目标宽、目标高、输出目录、水印文字、改名模板
    conv_edits: [HWND; E_N],
    conv_edit_rects: [RECT; E_N],
    conv_tw: String,
    conv_th: String,
    conv_fit: u8,
    conv_up: bool,
    conv_rot: u16,
    conv_fliph: bool,
    conv_flipv: bool,
    /// 强度 ×10，0 关。上限 30 = 编辑窗口那页的 3.0
    conv_sharp: u8,
    /// 半径 ×2，上限 20 = 编辑窗口那页的 10.0；默认 3 = 1.5，和那页开框的默认值一致
    conv_sharp_r: u8,
    conv_wm: bool,
    conv_wm_pos: u8,
    conv_wm_size: u8,
    conv_wm_opac: u8,
    conv_wm_text: String,
    conv_rn: bool,
    conv_rn_tpl: String,
    conv_rn_start: i32,
    /// 面板底部的即时报错/提示（状态栏那行太窄，点开始没反应时看不见原因）
    conv_msg: String,
    /// 这一批的总数和已完成数，面板进度条直接读
    conv_total: Arc<AtomicUsize>,
    conv_done: Arc<AtomicUsize>,
    /// 分类库这次是从备份/临时文件恢复的，载入完成后要在状态栏说明。
    db_note: String,
    search_kw: String,
    search_files: Vec<String>,
    page: usize,
    page_btns: Vec<(usize, RECT)>,
    /// 已经在确认框里点过"是"的命令 id；删除类命令靠它判断这次该弹框还是该执行。
    ask_go: usize,
    dlg: Option<Dlg>,
    /// 压在确认框底下的那层窗（只有一层）：继续画但不吃点击，确认框关掉后原样换回 dlg。
    dlg_hold: Option<Dlg>,
    dlg_btns: Vec<(usize, RECT)>,
    dlg_rows: Vec<(usize, RECT)>,
    /// 正按住浮层标题条拖它：记的是按下那一下离浮层左上角差多少，松手就清。
    dlg_drag: Option<(i32, i32)>,
    dlg_edit: Option<HWND>,
    /// 规则对话框的两个输入框（排除文件夹关键字、排除文件格式）。
    /// 只有 ScanRule 那层开着才有柄，关闭随 close_dlg 一起销毁。
    rule_edits: [HWND; 2],
    /// 文件关联页的草稿：每个扩展名勾没勾，点「应用」才写注册表。
    assoc: Vec<(String, bool)>,
    /// 关联页底部那行结果。写失败的项目也在这儿列出来，不弹框。
    assoc_msg: String,
    /// 「每次启动检查」那颗勾的草稿，点「应用」才落注册表——和扩展名勾选一个节奏。
    assoc_check: bool,
    /// 启动后还没复查过。第一次心跳里跑一次，不塞到首帧前面去。
    assoc_probe: bool,
    /// 原地改名：编辑框直接挂在名字那一格上，没有对话框。记的是这一格、它原来的路径。
    ren_edit: Option<HWND>,
    ren_idx: usize,
    ren_path: String,
    /// 这次改名挂在目录树的行上，不是格子：收尾走文件夹那一套，编辑框也跟着树行走。
    ren_tree: bool,
    /// 慢速双击的头一下：只记"这一下落在文件名那一格上"的那一项，第二下再接上才算改名。
    ren_arm: Option<String>,
    /// 树行的同款备记：头一下落在行名上的时刻。第二下距它 500~1000 毫秒
    /// 是改名，更快是双击开合，更慢就重新备记。
    ren_arm_t: Option<(String, Instant)>,
    /// 编辑框上一次待的位置：每帧重画都要对一次位，位置没变就不去动它，免得白擦一遍。
    ren_box: (i32, i32, i32, i32),
    conv_fmt: usize,
    /// 每种目标格式各存一份设置；切换格式不会把别的格式的选项冲掉。
    conv_opts: Vec<encode::Opt>,
    conv_out: String,
    /// 输出文件夹还是"跟着当前文件夹走"的默认态。用户自己填过路径（编辑框输入、
    /// 浏览按钮、对话框里改过）就落定为 false，之后不再跟手。
    /// 不跟手的话，开面板之后换过文件夹，转换就悄悄落进上一个文件夹。
    conv_out_auto: bool,
    /// 0 取代 / 1 跳过 / 2 重命名 / 3 询问（批量里等同跳过）
    conv_overwrite: u8,
    conv_del: bool,
    /// 批量重命名的模板、起始编号、大小写（0 不动 / 1 小写 / 2 大写）。
    rn_tpl: String,
    rn_start: i32,
    rn_case: u8,
    /// 数据库后台任务正在跑（检查/备份/导入），避免同时开两份。
    db_busy: bool,
    /// 已经选定、正在等确认框的那份导入源。
    db_import_src: Option<PathBuf>,
    /// 维护页上最近一次检查的结果；空表示还没查过。
    db_maint: String,
    /// 缩略图数据库页上最近一次「移除无效索引」的结果，和检查各走各的页。
    db_prune: String,
    /// 维护页上最近一次缩略图动作的结果。和检查结果各记各的，不互相盖掉。
    db_thumb: String,
    /// 维护页上最近一次扫描文件夹的结果。三样各记各的。
    db_scan: String,
    /// 扫描文件夹的名单，落盘在缓存目录 scan.tsv。每行一个根：路径 + 上次扫完
    /// 的秒数 + 排除规则（文件夹关键字、文件格式），规则随名单一起存、一起用。
    scan_dirs: Vec<ScanRoot>,
    scan_sel: usize,
    /// 名单画在哪一块，滚轮只认它；paint 时登记，滚轮照 sim_list 同一个办法读。
    scan_list: RECT,
    scan_scroll: usize,
    /// 扫描进行中的计数，心跳和结果框都从这几个现读。
    scan_busy: bool,
    scan_total: usize,
    scan_done: Arc<AtomicUsize>,
    scan_added: Arc<AtomicUsize>,
    scan_skip: Arc<AtomicUsize>,
    scan_err: Arc<AtomicUsize>,
    scan_stop: Arc<AtomicBool>,
    /// 中间列表的三个过滤开关。只在浏览文件夹时生效；是"暂时藏起来"，不是设定，
    /// 所以不落盘，重启全显示。
    filt_dirs: bool,
    filt_nonimg: bool,
    filt_cat: bool,
    /// 缩略图重建：池子按重建档跑着，跑完自己关掉。计数是这一轮的。
    fix_mode: bool,
    fix_total: usize,
    fix_done: usize,
    /// 这一轮里有多少张缓存中和原图对不上（错位）。
    fix_bad: usize,
    /// 全部重建：库认识的每张图都绕开缓存重解一遍。跑在后台线程上，
    /// 进度由界面每 60 毫秒心跳从这两个计数里现读，不靠消息推。
    thumb_busy: bool,
    thumb_total: usize,
    thumb_done: Arc<AtomicUsize>,
    thumb_bad: Arc<AtomicUsize>,
    thumb_stop: Arc<AtomicBool>,
    /// 找相似：后台先拿缩略图补指纹库，补完扫一遍指纹。库越大这一步越久，
    /// 所以进度照样由心跳从这两个计数里现读。
    sim_busy: bool,
    sim_total: usize,
    sim_done: Arc<AtomicUsize>,
    /// 这次相似结果是从哪张图找的，标题栏用它。空表示当前视图不是找相似。
    sim_q: String,
    // 以下都是"以图搜图"那一页的状态。放这儿而不是放 Dlg 里：Dlg 有六处字面量构造，
    // 每处都补十几个初值更容易漏，而这些参数本来就跟着进程走、不跟着窗口开合走。
    /// 拿来当查询的那张图，空表示还没选。
    sim_path: String,
    /// 相似度 50~100；换算成"允许差几位"由 sim 那边按所选算法的总位数做。
    sim_pct: u32,
    sim_algo: u8,
    sim_rot: bool,
    sim_flip: bool,
    /// 查找之前先拿缩略图缓存把这一档的指纹补齐。
    sim_auto: bool,
    /// 头一页说明看过没有。只弹一次，落注册表跟着进程走。
    sim_intro: bool,
    sim_hits: Vec<SimHit>,
    sim_sel: usize,
    sim_scroll: i32,
    /// 结果表那一块的矩形，滚轮只认它。
    sim_list: RECT,
    sim_ms: u128,
    sim_lib_msg: String,
    sim_stop: Arc<AtomicBool>,
    /// 底下那两块预览：查询图，和表里选中的那张。
    sim_src_img: Option<Img>,
    sim_hit_img: Option<Img>,
    /// 查询图的原图尺寸，只在换图时取一次（每帧开一次文件头同样拖慢拖动）。
    sim_qinfo_path: String,
    sim_qinfo: String,
    brush_edit: HBRUSH,
}

/// 数据库位置可以换：默认跟着主文件同级的 data；主文件装在系统目录写不动时退回
/// 缓存目录；他在分类数据库页换过位置就认注册表里那条 db_dir。
static DATA_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

fn data_dir() -> PathBuf {
    let mut g = DATA_DIR.lock().unwrap();
    if g.is_none() {
        *g = Some(pick_data_dir());
    }
    (*g).clone().unwrap_or_default()
}

/// 本该放这一份的位置：exe 旁边的 data。拿不到 exe 路径时才退回缓存目录。
fn default_data_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("data")))
        .unwrap_or_else(cache::dir)
}

/// 这个目录能不能真拿来放库：建得出目录、还得写得进文件才算。
/// 有些系统目录允许建目录、拒绝建文件，只查 create_dir_all 会看走眼。
fn dir_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".probe");
    let ok = std::fs::File::create(&probe).is_ok();
    if ok {
        let _ = std::fs::remove_file(&probe);
    }
    ok
}

fn pick_data_dir() -> PathBuf {
    let def = default_data_dir();
    if let Some(d) = ui_str("db_dir").map(PathBuf::from) {
        // 记过位置但那儿现在写不动（盘拔了、目录删了）：回默认，别把库丢在半路。
        if d.is_absolute() && dir_writable(&d) {
            bring_db_along(&d, &def);
            return d;
        }
    }
    // 默认位置不在启动时建目录：分发包旁边只该在真建类别、写库那一刻长出 data。
    // 写库那条路（write_atomic）自己会 create_dir_all。
    if !def.exists() {
        return def;
    }
    if dir_writable(&def) {
        return def;
    }
    cache::dir()
}

/// 换到新位置后头一次启动，把原来那份库原样复制过去（原件不动）。
fn bring_db_along(to: &Path, from: &Path) {
    if to == from || to.join("catalog.tsv").is_file() {
        return;
    }
    for name in ["catalog.tsv", "catalog.bak", "catalog.tmp"] {
        let src = from.join(name);
        if src.is_file() {
            copy_synced(&src, &to.join(name));
        }
    }
}

/// 维护页上「更改位置」：新目录写得动、而且那儿没有另一套库，才把这一份复制过去、
/// 认它，并把位置记进注册表。旧目录那份只复制不删——搬错了还能回去。
fn move_db_dir(to: &Path) -> String {
    let cur = data_dir();
    let norm =
        |p: &Path| p.to_string_lossy().trim_end_matches(|c| c == '\\' || c == '/').to_lowercase();
    if to.as_os_str().is_empty() {
        return "没有填位置，没动。".to_string();
    }
    if norm(to) == norm(&cur) {
        return format!("数据库本来就在 {}，没动。", cur.display());
    }
    if !dir_writable(to) {
        return format!("换不到 {}：那个目录建不出来，或者不让我们往里写文件。换硬盘上一个普通文件夹再试。", to.display());
    }
    if to.join("catalog.tsv").is_file() {
        return format!(
            "{} 里已经有一份 catalog.tsv，那是另一套分类库，我没盖它、也没认它。\n\n要用那一份的话：先把现在这套「导出数据库」成压缩包，再用「导入数据库」灌进去。",
            to.display()
        );
    }
    if !cur.join("catalog.tsv").is_file() {
        // 还没建过类别，库文件本来就一份没有：直接改认新目录，没东西可复制。
        *DATA_DIR.lock().unwrap() = Some(to.to_path_buf());
        unsafe { ui_put_str("db_dir", &to.display().to_string()) };
        return format!(
            "数据库位置已改到 {}。\n\n当前一套类别都还没建，所以那还是个空库，第一次归类时才会写出文件。",
            to.display()
        );
    }
    for name in ["catalog.tsv", "catalog.bak", "catalog.tmp"] {
        let src = cur.join(name);
        if src.is_file() {
            copy_synced(&src, &to.join(name));
        }
    }
    let dst = to.join("catalog.tsv");
    let ok = std::fs::read_to_string(&dst)
        .map(|t| catdb::catalog_ok(&t))
        .unwrap_or(false);
    if !ok {
        return format!(
            "复制过来的那份读不出来（{}），位置没改，还在 {}。",
            dst.display(),
            cur.display()
        );
    }
    *DATA_DIR.lock().unwrap() = Some(to.to_path_buf());
    unsafe { ui_put_str("db_dir", &to.display().to_string()) };
    format!(
        "数据库已改到 {}。\n\n原来那份还留在 {}，确认新位置没问题再自己删。\n以后每次启动都认这个位置。",
        to.display(),
        cur.display()
    )
}

fn copy_synced(src: &Path, dst: &Path) {
    let mut name = dst.as_os_str().to_os_string();
    name.push(".migrating");
    let tmp = PathBuf::from(name);
    if std::fs::copy(src, &tmp).is_ok() {
        if let Ok(f) = std::fs::File::open(&tmp) {
            let _ = f.sync_all();
        }
        let _ = std::fs::rename(&tmp, dst);
    }
}

/// 自含全量的分类库：主程序旁边的 data\catalog.tsv，运行时只认这一份。
fn catalog_file() -> PathBuf {
    data_dir().join("catalog.tsv")
}

/// 老格式那份"只记改动"的叠加层 categories.tsv——它只记改动，得并进一份自含全量库才算完整。
/// 只在第一次翻成自含库时读它，读完改名留档。更早的版本把它存在缩略图缓存目录里，
/// 那边有、data 里没有就先原样搬过来。
fn legacy_overlay_file() -> PathBuf {
    static CHOSEN: OnceLock<PathBuf> = OnceLock::new();
    CHOSEN
        .get_or_init(|| {
            let p = data_dir().join("categories.tsv");
            let old = cache::dir().join("categories.tsv");
            if old != p && !p.exists() && old.exists() {
                copy_synced(&old, &p);
                for ext in ["bak", "tmp"] {
                    let from = old.with_extension(ext);
                    let to = p.with_extension(ext);
                    if from.exists() && !to.exists() {
                        copy_synced(&from, &to);
                    }
                }
            }
            p
        })
        .clone()
}

/// 分类库的表文件和叠加层一起平铺在主文件旁边的 data 里，整份跟着 exe 目录走。
fn db_dir() -> PathBuf {
    data_dir()
}

/// 转换设置的落盘位置。光标一类的临时状态不放这里，只放各格式的那几个开关。
fn conv_cfg_file() -> PathBuf {
    cache::dir().join("convert.tsv")
}

fn load_conv_cfg() -> Vec<encode::Opt> {
    let mut v: Vec<encode::Opt> = (0..encode::FORMATS.len()).map(|_| encode::Opt::default()).collect();
    let Ok(s) = std::fs::read_to_string(conv_cfg_file()) else { return v };
    for line in s.lines() {
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() < 10 {
            continue;
        }
        let Ok(i) = p[0].parse::<usize>() else { continue };
        if i >= v.len() {
            continue;
        }
        let b = |x: &str| x == "1";
        v[i] = encode::Opt {
            quality: p[1].parse().unwrap_or(90).clamp(1, 100),
            progressive: b(p[2]),
            optimize_huffman: b(p[3]),
            sample_h: b(p[4]),
            sample_v: b(p[5]),
            thumb: p[6].parse().unwrap_or(0).min(2),
            dcf: b(p[7]),
            png_level: p[8].parse().unwrap_or(9).min(9),
            tiff_comp: p[9].parse().unwrap_or(1).min(3),
        };
    }
    v
}

fn save_conv_cfg(opts: &[encode::Opt]) {
    let mut s = String::new();
    for (i, o) in opts.iter().enumerate() {
        s.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            i,
            o.quality,
            o.progressive as u8,
            o.optimize_huffman as u8,
            o.sample_h as u8,
            o.sample_v as u8,
            o.thumb,
            o.dcf as u8,
            o.png_level,
            o.tiff_comp
        ));
    }
    let _ = std::fs::write(conv_cfg_file(), s);
}

/// 扫描名单的一行：根路径、上次扫完的时间、这条根自己的排除规则。
/// 规则三处生效：扫描走查剪枝、浏览顺手记指纹、补齐指纹——出处见 scan::excluded。
#[derive(Clone)]
struct ScanRoot {
    path: String,
    last: u64,
    /// 排除文件夹关键字（模糊：文件夹名包含即命中，不分大小写）。
    dirs: Vec<String>,
    /// 排除文件格式（后缀精确，不带点）。
    exts: Vec<String>,
}

/// 扫描文件夹的名单跟着缩略图库走，所以放缓存目录（convert.tsv 同一个先例）。
/// 一行一个文件夹：`上次扫完的秒数 \t 完整路径 \t 排除文件夹关键字 \t 排除格式`，
/// 关键字/格式内部用逗号分隔。后两列是后来加的，旧行没有就当没有规则。
fn scan_cfg_file() -> PathBuf {
    cache::dir().join("scan.tsv")
}

/// 逗号分隔的一串词拆成表，去空去重（存小写，匹配时两边都小写）。
fn rule_words(s: Option<&str>) -> Vec<String> {
    let mut v: Vec<String> = s
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(|x| x.to_ascii_lowercase())
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// 把现行名单换算成指纹关口的规则（scan::set_rules）。名单加载、规则保存、
/// 名单增删之后都要同步一次。
fn sync_scan_rules(dirs: &[ScanRoot]) {
    let rules = dirs
        .iter()
        .map(|r| scan::ExclRule::new(&r.path, &r.dirs, &r.exts))
        .collect();
    scan::set_rules(rules);
}

fn load_scan_cfg() -> Vec<ScanRoot> {
    let mut v: Vec<ScanRoot> = Vec::new();
    if let Ok(s) = std::fs::read_to_string(scan_cfg_file()) {
        for line in s.lines() {
            if line.is_empty() {
                continue;
            }
            let mut f = line.splitn(4, '\t');
            let (Some(t), Some(p)) = (f.next(), f.next()) else {
                continue;
            };
            if p.is_empty() {
                continue;
            }
            let (dirs, exts) = (rule_words(f.next()), rule_words(f.next()));
            // 同一个文件夹大小写不同的两行只留后写的，时间取新的那个。
            if let Some(old) = v.iter_mut().find(|o| o.path.eq_ignore_ascii_case(p)) {
                old.last = old.last.max(t.parse::<u64>().unwrap_or(0));
                continue;
            }
            v.push(ScanRoot {
                path: p.to_string(),
                last: t.parse::<u64>().unwrap_or(0),
                dirs,
                exts,
            });
        }
    }
    // 名单进了内存就把规则同步给指纹关口，启动即生效。
    sync_scan_rules(&v);
    v
}

fn save_scan_cfg(dirs: &[ScanRoot]) {
    let mut s = String::new();
    for r in dirs {
        s.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            r.last,
            r.path,
            r.dirs.join(","),
            r.exts.join(",")
        ));
    }
    let _ = std::fs::write(scan_cfg_file(), s);
}

// ── 界面摆位：调一次记一次 ─────────────────────────────────────
/// 界面上能调的那几十个数控在一条注册表键下，一个数一条 DWORD。
/// 只记"界面摆成什么样"，不记"这一次在看什么"（树展开到哪、滚到哪）。
/// 窗口位置也故意不记：两块屏并排时记下坐标，哪天拔了屏窗口就落在看不见的地方，
/// 看着像程序没起来；所以只记宽高，位置每次让系统摆。
/// 唯一的例外是 `last_dir`：那条是路径，DWORD 装不下，单独走 REG_SZ。
const UI_KEY: &str = "Software\\Xsee\\UI";

/// pw() 是把字符串常驻下来的，一次写二十几条不能那么用；这个是要完就丢的临时宽字。
fn uw(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// 启动时整条键读一次，之后全进程共用这一份。第一次运行或键没了就是空表，
/// 每个数各自退回代码里那份缺省。
fn ui_vals() -> &'static HashMap<String, u32> {
    static V: OnceLock<HashMap<String, u32>> = OnceLock::new();
    V.get_or_init(|| {
        let mut out = HashMap::new();
        unsafe {
            let key = uw(UI_KEY);
            let mut hk = HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), None, KEY_READ, &mut hk).0 != 0
            {
                return out;
            }
            let mut name = vec![0u16; 256];
            let mut data = [0u8; 4];
            for i in 0.. {
                let mut cch = name.len() as u32;
                let mut cb = data.len() as u32;
                let mut kind = 0u32;
                let e = RegEnumValueW(
                    hk,
                    i,
                    Some(PWSTR(name.as_mut_ptr())),
                    &mut cch,
                    None,
                    Some(&mut kind),
                    Some(data.as_mut_ptr()),
                    Some(&mut cb),
                );
                // 259 = ERROR_NO_MORE_ITEMS，到尾巴了才收工。字符串值（last_dir、
                // skin）塞不进 4 字节，枚举会回 ERROR_MORE_DATA——那条跳过往下走，
                // 不能 break，否则字母序排在它后面的那串 DWORD 全读不到。
                if e.0 == 259 {
                    break;
                }
                if e.0 != 0 || kind != REG_DWORD.0 || cb != 4 {
                    continue;
                }
                out.insert(String::from_utf16_lossy(&name[..cch as usize]), u32::from_ne_bytes(data));
            }
            let _ = RegCloseKey(hk);
        }
        out
    })
}

fn ui_num(name: &str, def: u32) -> u32 {
    ui_vals().get(name).copied().unwrap_or(def)
}

fn ui_flag(name: &str, def: bool) -> bool {
    ui_vals().get(name).map(|v| *v != 0).unwrap_or(def)
}

/// 读一条 REG_SZ。没这条键、值为空都算"没记过"，交回调用方退缺省。
fn ui_str(name: &str) -> Option<String> {
    unsafe {
        let key = uw(UI_KEY);
        let mut hk = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), None, KEY_READ, &mut hk).0 != 0 {
            return None;
        }
        let wn = uw(name);
        let mut buf = [0u8; 8192];
        let mut sz = buf.len() as u32;
        let r = RegQueryValueExW(
            hk,
            PCWSTR(wn.as_ptr()),
            None,
            None,
            Some(buf.as_mut_ptr()),
            Some(&mut sz),
        );
        let _ = RegCloseKey(hk);
        if r.0 != 0 || sz < 2 {
            return None;
        }
        let wide: Vec<u16> = buf[..sz as usize]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&c| c != 0)
            .collect();
        let s = String::from_utf16_lossy(&wide);
        (!s.is_empty()).then_some(s)
    }
}

/// 桌面只有一个算法：用户目录下的 Desktop。左树的第一项和没记过目录时的启动目录
/// 都从这里取，两边对不上就会出现"打开的是桌面、树里亮的不是那一项"。
fn desktop_dir() -> Option<String> {
    let home = user_home()?;
    let d = format!("{home}\\Desktop");
    Path::new(&d).is_dir().then_some(d)
}

/// 不带尾斜杠的用户目录。
fn user_home() -> Option<String> {
    let h = std::env::var("USERPROFILE").ok()?;
    let h = h.trim_end_matches('\\').to_string();
    (!h.is_empty()).then_some(h)
}

/// 没带路径参数时开哪个目录：上次关窗时看的那个。记不到就退桌面（新机器上第一启
/// 就是这个），桌面也没有就退用户目录。记下但如今已经不存在的目录（删了、盘拔了）
/// 也当没记过，不然启动就停在一个打不开的地方。
fn startup_dir() -> String {
    if let Some(d) = ui_str("last_dir") {
        if Path::new(&d).is_dir() {
            return d;
        }
    }
    desktop_dir().or_else(user_home).unwrap_or_else(|| "C:\\".into())
}

/// 密度三档。皮肤设置窗里那排按钮和注册表里存的下标共用这一份名单。
const DENS: [&str; 3] = ["compact", "normal", "loose"];
/// 详细资料档五条列宽在注册表里的值名，下标同 `App::col_w`
/// （名称、类型、尺寸、修改时间、大小）。
/// 换过一套值名：旧的 col_w0~col_w3 是"从右往左排的四条定宽列"，含义对不上了，
/// 读回来时只拿来给新键垫缺省（见 App 构造里那一段）。
const COLW_NAMES: [&str; 5] = ["colw_name", "colw_type", "colw_dim", "colw_date", "colw_size"];

/// 注册表里存的是枚举下标。对不上（改过档序、或值被人手改过）退回默认那一档，
/// 所以读回来的值不需要再各自判越界。
fn mode_at(i: u32) -> Mode {
    match i {
        1 => Mode::Small,
        2 => Mode::List,
        3 => Mode::Detail,
        _ => Mode::Grid,
    }
}

fn sort_at(i: u32) -> SortKey {
    match i {
        1 => SortKey::Ext,
        2 => SortKey::Size,
        3 => SortKey::Type,
        4 => SortKey::Date,
        5 => SortKey::Image,
        6 => SortKey::Desc,
        7 => SortKey::Added,
        _ => SortKey::Name,
    }
}

fn dens_at(i: u32) -> String {
    DENS.get(i as usize).copied().unwrap_or("normal").to_string()
}

/// 能调的界面数一次全摊平。写的时候只认这一份名单，加一项不用两头各记一遍。
fn ui_items(a: &App) -> Vec<(&'static str, u32)> {
    let b = |v: bool| v as u32;
    let px = |v: i32| v.max(0) as u32;
    let mut v = vec![
        ("mode", a.mode as u32),
        ("sort", a.sort as u32),
        ("sort_desc", b(a.sort_desc)),
        ("thumb_h", px(a.thumb_h)),
        ("pane_tree", b(a.pane_tree)),
        ("pane_pv", b(a.pane_pv)),
        ("bar_main", b(a.bar_main)),
        ("bar_path", b(a.bar_path)),
        ("bar_status", b(a.bar_status)),
        ("tb_text", b(a.tb_text)),
        ("tb_below", b(a.tb_below)),
        ("org_open", b(a.org_open)),
        ("conv_open", b(a.conv_open)),
        ("sim_intro", b(a.sim_intro)),
        ("dock_tab", a.dock_tab as u32),
        ("density", DENS.iter().position(|d| *d == a.density).unwrap_or(1) as u32),
        ("left_w", px(a.left_w)),
        ("pv_h", px(a.pv_h)),
        ("org_w", px(a.org_w)),
    ];
    for (i, w) in a.col_w.iter().enumerate() {
        v.push((COLW_NAMES[i], px(*w)));
    }
    v
}

/// 窗口那三条：正常位置的宽高 + 是不是最大化着。取的是"正常位置"那个矩形，
/// 所以最大化时关掉再开，记下的仍是还原之后该摆的那么大。
fn ui_win(hwnd: HWND) -> [(&'static str, u32); 3] {
    let mut wp = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
    let (mut w, mut h, mut max) = (0u32, 0u32, 0u32);
    unsafe {
        if GetWindowPlacement(hwnd, &mut wp).is_ok() {
            let r = wp.rcNormalPosition;
            w = (r.right - r.left).max(0) as u32;
            h = (r.bottom - r.top).max(0) as u32;
            max = u32::from(wp.showCmd == SW_SHOWMAXIMIZED.0 as u32);
        }
    }
    [("win_w", w), ("win_h", h), ("win_max", max)]
}

/// 上一次记的时候窗口是不是最大化着。WM_SIZE 只在它翻档的那一刻才写，
/// 拉伸边框过程中每条 WM_SIZE 都写等于拿注册表刷屏幕。
static UI_ZOOM: AtomicU32 = AtomicU32::new(0);

/// 全量写一次。每个"这一次调整结束"的时刻调一次，攒到关窗才写的话崩一次就全丢。
unsafe fn ui_save(a: &App, hwnd: HWND) {
    // 命令口跑测试那一趟不写盘：跑完就退，把测试摆的位置和目录记到他偏好里，
    // 下次他自己打开就发现软件"变了"。
    if drive::quiet() {
        return;
    }
    let mut items = ui_items(a);
    items.extend(ui_win(hwnd));
    let key = uw(UI_KEY);
    let mut hk = HKEY::default();
    let e = RegCreateKeyExW(
        HKEY_CURRENT_USER,
        PCWSTR(key.as_ptr()),
        None,
        PCWSTR::null(),
        REG_OPTION_NON_VOLATILE,
        KEY_SET_VALUE,
        None,
        &mut hk,
        None,
    );
    if e.0 != 0 {
        return;
    }
    for (name, val) in items {
        let wn = uw(name);
        let bytes = val.to_ne_bytes();
        let _ = RegSetValueExW(hk, PCWSTR(wn.as_ptr()), None, REG_DWORD, Some(&bytes));
    }
    // 目录一条。还没落到真实目录时（dir 仍是 "." 那个占位）不写，免得下次启动
    // 把相对占位当成上次的目录读回来。
    if a.dir.is_absolute() {
        let d = a.dir.display().to_string();
        let wide: Vec<u16> = d.encode_utf16().chain([0]).collect();
        let bytes: Vec<u8> = wide.iter().flat_map(|w| w.to_le_bytes()).collect();
        let wn = uw("last_dir");
        let _ = RegSetValueExW(hk, PCWSTR(wn.as_ptr()), None, REG_SZ, Some(&bytes));
    }
    // 皮肤一条：关窗再兜一次底，换肤那几处已经即时写过，这里保证漏不掉。
    {
        let wide: Vec<u16> = a.skin.encode_utf16().chain([0]).collect();
        let bytes: Vec<u8> = wide.iter().flat_map(|w| w.to_le_bytes()).collect();
        let wn = uw("skin");
        let _ = RegSetValueExW(hk, PCWSTR(wn.as_ptr()), None, REG_SZ, Some(&bytes));
    }
    let _ = RegCloseKey(hk);
}

/// 单写一条字符串设置就走这一笔，不等关窗才存——数据库位置这种，改完立刻要落。
unsafe fn ui_put_str(name: &str, val: &str) {
    let key = uw(UI_KEY);
    let mut hk = HKEY::default();
    if RegCreateKeyExW(
        HKEY_CURRENT_USER,
        PCWSTR(key.as_ptr()),
        None,
        PCWSTR::null(),
        REG_OPTION_NON_VOLATILE,
        KEY_SET_VALUE,
        None,
        &mut hk,
        None,
    )
    .0
        != 0
    {
        return;
    }
    let wn = uw(name);
    let wide: Vec<u16> = val.encode_utf16().chain([0]).collect();
    let bytes: Vec<u8> = wide.iter().flat_map(|w| w.to_le_bytes()).collect();
    let _ = RegSetValueExW(hk, PCWSTR(wn.as_ptr()), None, REG_SZ, Some(&bytes));
    let _ = RegCloseKey(hk);
}

/// 类别剪贴板只在本程序里用：复制的是整棵子树的快照。
static CAT_CLIP: Mutex<Option<catdb::CatSnap>> = Mutex::new(None);

fn cat_clip_empty() -> bool {
    CAT_CLIP.lock().map(|c| c.is_none()).unwrap_or(true)
}

/// 后台搬运线程把成功的 (旧路径, 新路径) 放这里，界面线程落完分类再取走。
static MOVED_IMGS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
/// 落文件夹那趟失败的第一句原因，随 WM_DROP_DONE 一起给界面线程读。
static DROP_ERR: Mutex<String> = Mutex::new(String::new());
/// 落文件夹那趟动过的目录（目标 + 各个源的上层），界面线程拿它们重扫树。
static DROP_DIRS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// 把搬过家的图片路径同步进分类库；返回保存失败时的提示。
fn sync_cat_paths(g: &mut App, pairs: &[(String, String)]) -> Option<String> {
    let cat = g.cat.as_mut()?;
    for (old, new) in pairs {
        cat.rename_path(old, new);
    }
    match cat.save_full(&catalog_file()) {
        Ok(()) => None,
        Err(e) => Some(format!("类别路径已改，但保存失败：{e}")),
    }
}

/// App 里唯一不天生 Send 的是两个 HFONT 句柄——它们只在界面线程被 GDI 用到，
/// 工作线程从不碰，跨线程搬运的只有 String 和像素缓冲。
unsafe impl Send for App {}

fn app() -> &'static Mutex<App> {
    static A: OnceLock<Mutex<App>> = OnceLock::new();
    A.get_or_init(|| {
        // 上次穿哪套皮肤记在注册表里；那套被改名或删掉了才退回默认，
        // 不然每次开窗都从默认那套重来一遍。
        let d = ui_str("skin").filter(|n| theme::has_skin(n)).unwrap_or_else(theme::default_name);
        let look = theme::Look::load(&d);
        // 没写过图标偏好也有一套完整的：默认用内置皮肤配 lucide 那套图标。
        let layout = theme::startup_layout();
        let dens = dens_at(ui_num("density", 1));
        Mutex::new(App {
            th: Theme::new_with_layout(&d, &dens, 1.0, &look, layout.as_deref()),
            skin: d,
            layout,
            density: dens,
            look,
            scale: 1.0,
            mode: mode_at(ui_num("mode", 0)),
            sort: sort_at(ui_num("sort", 0)),
            sort_desc: ui_flag("sort_desc", false),
            thumb_h: ui_num("thumb_h", THUMB_H_MIN as u32).clamp(THUMB_H_MIN as u32, THUMB_H_MAX as u32) as i32,
            dir: PathBuf::from("."),
            entries: Vec::new(),
            has_up: false,
            sel: usize::MAX,
            marked: HashSet::new(),
            anchor: usize::MAX,
            sel_lit: true,
            tree_kb: false,
            band: None,
            band_add: HashSet::new(),
            hover: usize::MAX,
            hover_btn: None,
            hover_menu: None,
            scroll: 0,
            scroll_vel: 0.0,
            scroll_until: Instant::now(),
            anim_defer: false,
            paint_scroll: 0,
            paint_ok: false,
            grid_stale: false,
            tscroll: 0,
            thumbs: HashMap::new(),
            dims: HashMap::new(),
            bpps: HashMap::new(),
            stats: HashMap::new(),
            queued: HashSet::new(),
            pool: None,
            got: 0,
            err: 0,
            tree: Vec::new(),
            tsel: String::new(),
            home: user_home().unwrap_or_else(|| "C:\\".into()),
            tree_noscroll: false,
            tree_drag: None,
            grid_drag: None,
            col_w: {
                // 拖边界按 48~1400 夹，读回来再夹一遍：值被人手改过也不会把列排歪。
                // 名称、大小两格 0 = 没调过（铺满剩下的宽度）。类型/尺寸/修改时间三列的
                // 缺省拿旧键 col_w0~col_w2 垫上，这一版之前调过的列宽不会白调。
                let def: [u32; 5] =
                    [0, ui_num("col_w0", 96), ui_num("col_w1", 104), ui_num("col_w2", 132), 0];
                [0usize, 1, 2, 3, 4].map(|i| {
                    let v = ui_num(COLW_NAMES[i], def[i]);
                    if i == 0 || i == 4 {
                        v.min(1400) as i32
                    } else {
                        v.clamp(48, 1400) as i32
                    }
                })
            },
            col_drag: None,
            hist: Vec::new(),
            hpos: 0,
            left_w: ui_num("left_w", 0) as i32,
            pane_tree: ui_flag("pane_tree", true),
            pane_pv: ui_flag("pane_pv", true),
            bar_main: ui_flag("bar_main", true),
            bar_path: ui_flag("bar_path", true),
            bar_status: ui_flag("bar_status", true),
            tb_text: ui_flag("tb_text", true),
            tb_below: ui_flag("tb_below", true),
            tb_icon: (0, HICON::default()),
            pv_h: ui_num("pv_h", 0) as i32,
            pv_w: 0,
            total_sz: None,
            sz_seq: 0,
            t0: Instant::now(),
            t_dir: Instant::now(),
            ready_sec: None,
            hit0: 0,
            drag: None,
            drag_seed: None,
            tree_seed: None,
            drop_hl: None,
            btns: Vec::new(),
            menus: Vec::new(),
            pop_rows: Vec::new(),
            sub_rows: Vec::new(),
            open_menu: None,
            open_sub: None,
            pop_hover: None,
            sub_hover: None,
            pop_rect: RECT::default(),
            sub_rect: RECT::default(),
            ctx_at: None,
            ctx_rect: RECT::default(),
            ctx_rows: Vec::new(),
            ctx_sub: None,
            ctx_sub_rect: RECT::default(),
            ctx_dir: None,
            ctx_img: None,
            ctx_blank: true,
            ctx_tree: false,
            ctx_sub_rows: Vec::new(),
            ctx_cat: None,
            ctx_cat_rect: RECT::default(),
            ctx_cat_rows: Vec::new(),
            bmenu: None,
            bar: RECT::default(),
            path_bar: RECT::default(),
            addr_tr: RECT::default(),
            addr_xs: Vec::new(),
            addr_src: String::new(),
            addr_w: 0,
            addr_n: 0,
            addr_full: false,
            addr_sel: None,
            addr_anchor: None,
            vis: (0, 0),
            pt: (-1, -1),
            stamp: 0,
            font: HFONT::default(),
            font_sm: HFONT::default(),
            font_b: HFONT::default(),
            font_num: HFONT::default(),
            font_path: HFONT::default(),
            zf: [ZoneFont::default(); theme::NZ],
            msg: String::new(),
            flash: None,
            src: Src::Folder,
            view_title: String::new(),
            org_open: ui_flag("org_open", true),
            org_w: ui_num("org_w", 0) as i32,
            org_scroll: 0,
            org_sel: 0,
            org_open_ids: HashSet::new(),
            org_hits: Vec::new(),
            org_checks: Vec::new(),
            org_up: RECT::default(),
            org_dn: RECT::default(),
            cat: None,
            org_drag: None,
            org_cat_drag: None,
            org_cat_to: -1,
            conv_open: ui_flag("conv_open", false),
            dock_tab: ui_num("dock_tab", 0).min(1) as u8,
            conv_w_prev: 0,
            conv_w_auto: false,
            conv_scroll: 0,
            conv_max_scroll: 0,
            conv_drag: None,
            conv_sdrag: None,
            conv_edits: [HWND::default(); E_N],
            conv_edit_rects: [RECT::default(); E_N],
            conv_tw: String::new(),
            conv_th: String::new(),
            conv_fit: 1,
            conv_up: false,
            conv_rot: 0,
            conv_fliph: false,
            conv_flipv: false,
            conv_sharp: 0,
            conv_sharp_r: 3,
            conv_wm: false,
            conv_wm_pos: 3,
            conv_wm_size: 4,
            conv_wm_opac: 55,
            conv_wm_text: String::new(),
            conv_rn: false,
            conv_rn_tpl: "*".into(),
            conv_rn_start: 1,
            conv_msg: String::new(),
            conv_total: Arc::new(AtomicUsize::new(0)),
            conv_done: Arc::new(AtomicUsize::new(0)),
            db_note: String::new(),
            search_kw: String::new(),
            search_files: Vec::new(),
            page: 0,
            page_btns: Vec::new(),
            ask_go: 0,
            dlg: None,
            dlg_hold: None,
            dlg_btns: Vec::new(),
            dlg_rows: Vec::new(),
            dlg_drag: None,
            dlg_edit: None,
            rule_edits: [HWND::default(); 2],
            assoc: Vec::new(),
            assoc_msg: String::new(),
            assoc_check: assoc::check_on(),
            assoc_probe: true,
            ren_edit: None,
            ren_idx: usize::MAX,
            ren_path: String::new(),
            ren_tree: false,
            ren_arm: None,
            ren_arm_t: None,
            ren_box: (0, 0, 0, 0),
            conv_fmt: 0,
            conv_opts: load_conv_cfg(),
            conv_out: String::new(),
            conv_out_auto: true,
            conv_overwrite: 2,
            conv_del: false,
            rn_tpl: "*".into(),
            rn_start: 1,
            rn_case: 0,
            db_busy: false,
            db_import_src: None,
            db_maint: String::new(),
            db_prune: String::new(),
            db_thumb: String::new(),
            db_scan: String::new(),
            scan_dirs: load_scan_cfg(),
            scan_sel: 0,
            scan_list: RECT::default(),
            scan_scroll: 0,
            scan_busy: false,
            scan_total: 0,
            scan_done: Arc::new(AtomicUsize::new(0)),
            scan_added: Arc::new(AtomicUsize::new(0)),
            scan_skip: Arc::new(AtomicUsize::new(0)),
            scan_err: Arc::new(AtomicUsize::new(0)),
            scan_stop: Arc::new(AtomicBool::new(false)),
            filt_dirs: false,
            filt_nonimg: false,
            filt_cat: false,
            fix_mode: false,
            fix_total: 0,
            fix_done: 0,
            fix_bad: 0,
            thumb_busy: false,
            thumb_total: 0,
            thumb_done: Arc::new(AtomicUsize::new(0)),
            thumb_bad: Arc::new(AtomicUsize::new(0)),
            thumb_stop: Arc::new(AtomicBool::new(false)),
            sim_busy: false,
            sim_total: 0,
            sim_done: Arc::new(AtomicUsize::new(0)),
            sim_q: String::new(),
            sim_path: String::new(),
            sim_pct: 87,
            sim_algo: sim::ALGO_ALL,
            sim_rot: false,
            sim_flip: false,
            sim_auto: true,
            sim_intro: ui_flag("sim_intro", false),
            sim_hits: Vec::new(),
            sim_sel: 0,
            sim_scroll: 0,
            sim_list: RECT::default(),
            sim_ms: 0,
            sim_lib_msg: String::new(),
            sim_stop: Arc::new(AtomicBool::new(false)),
            sim_src_img: None,
            sim_hit_img: None,
            sim_qinfo_path: String::new(),
            sim_qinfo: String::new(),
            brush_edit: HBRUSH::default(),
        })
    })
}

/// 右格预览：后台线程只追最新那一次请求，过期结果直接丢。
/// (代数, 位图宽, 位图高, BGRA, 原图宽, 原图高)
type Pv = (usize, u32, u32, Vec<u8>, u32, u32, u32);
static PV: OnceLock<Mutex<Option<Pv>>> = OnceLock::new();
fn pv() -> &'static Mutex<Option<Pv>> {
    PV.get_or_init(|| Mutex::new(None))
}

fn pw(s: &str) -> &'static [u16] {
    Box::leak(s.encode_utf16().chain([0]).collect::<Vec<u16>>().into_boxed_slice())
}

fn stem(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// 排列图标用的三把钥匙：文件名、扩展名、类型说明。
/// 扩展名 → 类型列上那句。列头显示和"按类型排序"的键都从这一张表取，
/// 免得出现"看着是 JPEG 图像、排到别处去"的两套说法。
fn type_label(ext: &str) -> String {
    let named = match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "jpe" | "jfif" => Some("JPEG 图像"),
        "png" => Some("PNG 图像"),
        "gif" => Some("GIF 图像"),
        "bmp" => Some("BMP 图像"),
        "tif" | "tiff" => Some("TIFF 图像"),
        "webp" => Some("WebP 图像"),
        "heic" => Some("HEIC 图像"),
        "psd" | "psb" => Some("Photoshop 图像"),
        "ico" => Some("图标"),
        "avif" => Some("AVIF 图像"),
        "tga" => Some("Targa 图像"),
        "dds" => Some("DDS 纹理"),
        "qoi" => Some("QOI 图像"),
        "exr" => Some("OpenEXR 图像"),
        "hdr" => Some("HDR 图像"),
        "ff" => Some("Farbfeld 图像"),
        "pbm" | "pgm" | "ppm" | "pnm" | "pam" => Some("Netpbm 图像"),
        "wdp" | "hdp" | "jxr" => Some("JPEG XR 图像"),
        _ => None,
    };
    match named {
        Some(t) => t.to_string(),
        None if ext.is_empty() => "文件".to_string(),
        None => format!("{} 文件", ext.to_ascii_uppercase()),
    }
}

fn sort_labels(path: &str) -> (String, String, String) {
    let name = stem(path).to_lowercase();
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_string()).unwrap_or_default();
    let typ = type_label(&ext).to_lowercase();
    (name, ext, typ)
}

fn fmt_size(n: u64) -> String {
    if n >= 1 << 30 {
        format!("{:.2} GB", n as f64 / 1073741824.0)
    } else if n >= 1 << 20 {
        format!("{:.1} MB", n as f64 / 1048576.0)
    } else if n >= 1024 {
        format!("{:.0} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

/// epoch 秒 -> 本地 "2026-10-19 14:22"。走系统的 FILETIME 换算，自己算时区会漏夏令时。
fn local(epoch: u64) -> String {
    const TO1601: u64 = 116_444_736_000_000_000;
    let ticks = epoch.saturating_mul(10_000_000).saturating_add(TO1601);
    let ft = FILETIME {
        dwLowDateTime: (ticks & 0xffff_ffff) as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut loc = FILETIME::default();
    let mut st = SYSTEMTIME::default();
    unsafe {
        if FileTimeToLocalFileTime(&ft, &mut loc).is_err() {
            return String::new();
        }
        let _ = FileTimeToSystemTime(&loc, &mut st);
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute
    )
}

/// 文件系统给的时间 -> 本地 "2026-10-19 14:22"。取不到（卷不支持、时间为空）就返回空串。
fn fmt_sys_time(t: std::io::Result<std::time::SystemTime>) -> String {
    t.ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| local(d.as_secs()))
        .unwrap_or_default()
}

/// 结果表"所在文件夹"那一列：只留最后一级目录名，整条路径在窄列里全是省略号。
fn sim_parent(path: &str) -> String {
    match Path::new(path).parent() {
        Some(p) => p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.to_string_lossy().into_owned()),
        None => String::new(),
    }
}

fn stat_of(path: &str) -> (u64, String) {
    match std::fs::metadata(path) {
        Ok(m) => {
            let t = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| local(d.as_secs()))
                .unwrap_or_default();
            (m.len(), t)
        }
        Err(_) => (0, String::new()),
    }
}

/// 列标题和单元格的横坐标，画的和点的共用一份。
/// 详细资料档的分列线（物理像素）。name/typ/dim/date/size 是各列左缘，end 是大小那列的右缘，
/// right 是列表可见区的右缘（拖宽以后超出它的格子，画的时候裁掉）。
/// 五列各自有宽，拖一条边界只改自己那一列，别的列只跟着挪位置、宽度不动。
struct Cols {
    name: i32,
    typ: i32,
    dim: i32,
    date: i32,
    size: i32,
    end: i32,
    right: i32,
}

impl App {
    /// 缩略图真正显示的那块（边距之外）：缩放条只归缩略图档，
    /// 图标档用基准高度，宽度按格子的宽:高比例跟着走。
    fn thumb_box(&self) -> (u32, u32) {
        let s = &self.th;
        let base = if self.mode == Mode::Grid { self.thumb_h } else { THUMB_H_MIN };
        let h = (base as f64 * self.scale).round().max(1.0) as u32;
        let w = (h as f64 * s.cell_w as f64 / s.cell_h.max(1) as f64).round().max(1.0) as u32;
        (w, h)
    }

    fn box_size(&self) -> (i32, i32) {
        match self.mode {
            // 缩略图档和图标档共用同一套格子比例；图标档画 48 像素图标，
            // 但不靠压扁格子去凑，框的比例保持原来的样子。
            Mode::Grid | Mode::Small => {
                let (w, h) = self.thumb_box();
                (w as i32 + 2 * self.th.pad, h as i32 + 2 * self.th.pad)
            }
            Mode::List | Mode::Detail => {
                // 这两档的行高固定，不跟缩略图缩放条走。
                (self.th.cell_w, self.th.row + 4)
            }
        }
    }

    /// 整格高 = 缩略图盒 + 尺寸行 + 文件名条，照设计稿的 .cell 三行结构。
    fn cell(&self) -> (i32, i32) {
        match self.mode {
            Mode::List | Mode::Detail => self.box_size(),
            _ => {
                let (w, h) = self.box_size();
                (w, h + self.th.meta + self.th.label)
            }
        }
    }

    fn row_h(&self) -> i32 {
        match self.mode {
            Mode::List | Mode::Detail => self.cell().1,
            _ => self.cell().1 + self.th.gap,
        }
    }

    /// 列表档一列多宽：只放图标和文件名，跟缩略图格子同宽，窗口够宽就多排几列。
    fn list_col_w(&self) -> i32 {
        self.th.cell_w.max(1)
    }

    fn cols_n(&self, gw: i32) -> usize {
        if self.mode == Mode::Detail {
            return 1;
        }
        if self.mode == Mode::List {
            let pad = self.grid_pad();
            return ((gw - pad * 2).max(1) / self.list_col_w()).max(1) as usize;
        }
        let pad = (6.0 * self.scale).round() as i32;
        ((gw - pad * 2).max(1) / self.box_size().0.max(1)).max(1) as usize
    }

    /// 缩略图向解码器要的目标框：格子扣掉边距和文件名行。
    /// 图标档、列表档不生成缩略图，这个尺寸只给找相似那几块预览用，照格子档给。
    fn tbox(&self) -> (u32, u32) {
        match self.mode {
            Mode::Grid | Mode::Small => self.thumb_box(),
            Mode::List | Mode::Detail => {
                let (_, ch) = self.box_size();
                let s = (ch - 2).clamp(16, 64) as u32;
                (s, s)
            }
        }
    }

    /// 这一档要不要开缩略图池。图标档、列表档只画图标，不开；
    /// 详细资料档也只画图标，但"尺寸"那一列要原图尺寸，还得靠池子带回来。
    fn wants_thumb(&self) -> bool {
        !matches!(self.mode, Mode::Small | Mode::List)
    }

    fn pending_n(&self) -> usize {
        if self.wants_thumb() { self.n_images() } else { 0 }
    }

    fn cols(&self, g: RECT) -> Cols {
        let s = &self.th;
        let k = self.scale;
        let right = g.right - s.gap;
        let name0 = g.left + s.pad + s.icon + s.icon / 2 + s.gap;
        let cw = |i: usize| ((self.col_w[i] as f64 * k).round() as i32).max((28.0 * k) as i32);
        // 五列从左往右排，各自多宽就是多宽：拖一条边界只改它左边那一列，
        // 右边那几列只跟着挪位置，宽度一格也不动。以前四条定宽列是从右往左锚的，
        // 窗口一窄还一起等比缩，所以调完一条发现别条也变了。
        // 名称、大小这两列没调过（记 0）时铺满剩下的宽度：两列都 0 时名称铺到
        // 离右缘 min_size 为止、大小吃满最后那一段，第一次打开看着跟以前一样。
        // 按下任意一条列线时会先把这两列定格成眼前的宽，从此五列都只认自己的数。
        let min_size = (78.0 * k) as i32;
        let fixed_after_name =
            cw(1) + cw(2) + cw(3) + if self.col_w[4] > 0 { cw(4) } else { min_size };
        let name_w = if self.col_w[0] > 0 {
            cw(0)
        } else {
            (right - name0 - fixed_after_name).max((140.0 * k) as i32)
        };
        let typ = name0 + name_w;
        let dim = typ + cw(1);
        let date = dim + cw(2);
        let size = date + cw(3);
        // 大小那列没调过就铺到最右缘；调过就按自己那份宽走，哪怕右边空出一截。
        let end = if self.col_w[4] > 0 { size + cw(4) } else { right.max(size) };
        Cols { name: name0, typ, dim, date, size, end, right }
    }

    /// 第 i 条可拖的列边界线的 x，序号和 `col_w` 一一对得上：
    /// 0=名称右、1=类型右、2=尺寸右、3=修改时间右、4=大小右。
    /// 这几条线都是从左往右排的，所以线往右拖就是把它左边那一列加宽。
    fn col_edge(&self, g: RECT, i: usize) -> i32 {
        let c = self.cols(g);
        [c.typ, c.dim, c.date, c.size, c.end][i]
    }

    /// 当前滚动位置下看得见的条目区间 [first, last)。
    fn range(&self, g: RECT) -> (usize, usize) {
        let pitch = self.row_h().max(1);
        let cols = self.cols_n(g.right - g.left);
        let lines = ((g.bottom - g.top) / pitch + 2).max(1) as usize;
        let top = (self.scroll / pitch).max(0) as usize;
        let n = self.entries.len();
        if self.mode == Mode::Detail {
            (top, (top + lines).min(n))
        } else {
            (top * cols, ((top + lines) * cols).min(n))
        }
    }

    /// 视口底下还有没有条目。走条带的那一帧靠它确认视口最下面不是露出地面的空白：
    /// 内容到底了，底下那一块是地面，而地面钉在窗口上、不能跟着搬。
    fn has_below(&self, g: RECT) -> bool {
        self.range(g).1 < self.entries.len()
    }

    fn grid_pad(&self) -> i32 {
        (6.0 * self.scale).round() as i32
    }

    fn grid_x0(&self, g: RECT) -> i32 {
        let cols = self.cols_n(g.right - g.left).max(1) as i32;
        let pad = self.grid_pad();
        let avail = (g.right - g.left - 2 * pad).max(0);
        g.left + pad + ((avail - cols * self.cell().0).max(0)) / 2
    }

    fn max_scroll(&self, g: RECT) -> i32 {
        let pitch = self.row_h().max(1);
        let cols = self.cols_n(g.right - g.left).max(1);
        let lines = self.entries.len().div_ceil(cols).max(1) as i32;
        (lines * pitch - self.th.gap + self.grid_pad() * 2 - (g.bottom - g.top)).max(0)
    }

    /// 换视图、换窗口大小之后行高就变了，旧的滚动量可能已经超出范围，
    /// 不夹住的话第一行会画到工具条上面去。
    fn clamp_scroll(&mut self, g: RECT) {
        self.scroll = self.scroll.clamp(0, self.max_scroll(g));
    }

    /// 不走帧节拍、直接把列表滚到某个位置（滚动条、键盘、进目录）。惯性速度就地
    /// 作废，两条来源不会互相打架；宽限照记，刚滚完心跳别急着整窗刷。
    fn scroll_set(&mut self, v: i32) {
        self.scroll = v;
        self.scroll_vel = 0.0;
        self.scroll_until = Instant::now() + Duration::from_millis(400);
    }

    /// 速度还在，或者刚滚完不久：这段时间内 60ms 心跳只攒刷新、不整窗刷。
    fn scrolling(&self) -> bool {
        self.scroll_vel != 0.0 || Instant::now() < self.scroll_until
    }

    fn n_images(&self) -> usize {
        self.entries.iter().filter(|e| e.is_image).count()
    }

    fn cell_rect(&self, i: usize, g: RECT) -> RECT {
        match self.mode {
            Mode::Detail => {
                let y = g.top + i as i32 * self.row_h() - self.scroll;
                RECT { left: g.left, top: y, right: g.right, bottom: y + self.row_h() - 1 }
            }
            Mode::List => {
                let cw = self.list_col_w();
                let cols = self.cols_n(g.right - g.left).max(1) as i32;
                let c = (i as i32) % cols;
                let r = (i as i32) / cols;
                let x = g.left + self.grid_pad() + c * cw;
                let y = g.top + r * self.row_h() - self.scroll;
                RECT { left: x, top: y, right: x + cw, bottom: y + self.row_h() - 1 }
            }
            _ => {
                let (cw, ch) = self.cell();
                let cols = self.cols_n(g.right - g.left).max(1) as i32;
                let c = (i as i32) % cols;
                let r = (i as i32) / cols;
                let x = self.grid_x0(g) + c * cw;
                let y = g.top + self.grid_pad() + r * self.row_h() - self.scroll;
                RECT { left: x, top: y, right: x + cw, bottom: y + ch }
            }
        }
    }

    /// 框选：矩形碰到的文件（文件夹不参与）。传进来的是客户区坐标，
    /// 框本身按内容坐标记，滚动时换算一下就能跟住内容。
    /// 这一项能不能进选区：只有虚拟的 `..` 不算，文件夹和资源管理器一样可以选中。
    fn pickable(&self, i: usize) -> bool {
        !(self.has_up && i == 0)
    }

    /// 树上这一行能不能改名、剪切、删除：盘符、“此电脑”这个虚拟根、用户主目录
    /// 都不在其列。右键菜单和慢两连击共用这一条，口径不会各说一套。
    fn tree_editable(&self, path: &str) -> bool {
        let pt = path.trim_end_matches('\\');
        pt != "此电脑"
            && !pt.eq_ignore_ascii_case(self.home.trim_end_matches('\\'))
            && pt.len() > 3
    }

    /// 键盘焦点在树上时，文件命令（F2/Del/复制/剪切/属性/转到文件）认的是这一行。
    /// 盘根保留结尾反斜杠，其余摘掉，和右键落点同一个口径。
    /// “此电脑”没有文件层面的落点，返回 None——调用方按“什么都不做”处理。
    fn tree_focus_dir(&self) -> Option<String> {
        if !self.tree_kb || self.tsel == "此电脑" {
            return None;
        }
        let p = self.tsel.as_str();
        Some(if p.len() > 3 { p.trim_end_matches('\\').to_string() } else { p.to_string() })
    }

    fn band_pick(&self, g: RECT, x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<usize> {
        let (bx0, bx1) = (x0.min(x1), x0.max(x1));
        let (by0, by1) = (y0.min(y1), y0.max(y1));
        let mut hits = Vec::new();
        for i in 0..self.entries.len() {
            if !self.pickable(i) {
                continue;
            }
            let r = self.cell_rect(i, g);
            if r.left < bx1 && r.right > bx0 && r.top < by1 && r.bottom > by0 {
                hits.push(i);
            }
        }
        hits
    }

    fn hit(&self, g: RECT, x: i32, y: i32) -> usize {
        if x < g.left || x >= g.right || y < g.top || y >= g.bottom {
            return usize::MAX;
        }
        let n = self.entries.len();
        match self.mode {
            Mode::Detail => {
                let r = (y - g.top + self.scroll) / self.row_h().max(1);
                if r < 0 || r as usize >= n {
                    usize::MAX
                } else {
                    r as usize
                }
            }
            Mode::List => {
                let cw = self.list_col_w();
                let cols = self.cols_n(g.right - g.left).max(1) as i32;
                let x0 = g.left + self.grid_pad();
                if x < x0 {
                    return usize::MAX;
                }
                let c = (x - x0) / cw;
                let r = (y - g.top + self.scroll) / self.row_h().max(1);
                if c < 0 || c >= cols || r < 0 {
                    return usize::MAX;
                }
                let i = (r as usize) * cols as usize + c as usize;
                if i >= n {
                    usize::MAX
                } else {
                    i
                }
            }
            _ => {
                let cw = self.cell().0;
                let x0 = self.grid_x0(g);
                if x < x0 {
                    return usize::MAX;
                }
                let cols = self.cols_n(g.right - g.left).max(1) as i32;
                let c = (x - x0) / cw.max(1);
                let r = (y - g.top - self.grid_pad() + self.scroll) / self.row_h().max(1);
                if c < 0 || c >= cols || r < 0 {
                    return usize::MAX;
                }
                let xc = x0 + c * cw;
                if x - xc >= cw {
                    return usize::MAX;
                }
                let i = (r as usize) * cols as usize + c as usize;
                if i >= n || r == 0 && y < g.top + self.grid_pad() - self.scroll {
                    usize::MAX
                } else {
                    i
                }
            }
        }
    }

    fn set_sel(&mut self, i: usize) {
        self.sel = i;
        self.sel_lit = true;
        self.stamp += 1;
    }

    /// 文件名那一块。慢速再点只有落在字上才进改名，点缩略图不算。
    fn name_rect(&self, i: usize, g: RECT) -> RECT {
        let r = self.cell_rect(i, g);
        match self.mode {
            Mode::Detail => {
                let cx = self.cols(g);
                let right = (cx.typ - 4).min(cx.right).max(cx.name);
                RECT { left: cx.name, top: r.top, right, bottom: r.bottom }
            }
            Mode::List => {
                let isz = self.th.icon.max(14);
                let left = r.left + self.th.pad + isz + isz / 2 + self.th.gap;
                RECT { left, top: r.top, right: r.right - self.th.pad, bottom: r.bottom }
            }
            _ => {
                let (_, bh) = self.box_size();
                let top = r.top + bh + self.th.meta;
                RECT { left: r.left, top, right: r.right, bottom: top + self.th.label }
            }
        }
    }

    /// 本目录里有多少张是直接命中缓存的。缓存计数是进程累计的，
    /// 直接印出来会让人以为整库只有这么点，所以要减掉进目录前的底数。
    fn dir_hits(&self) -> u64 {
        cache::hits().saturating_sub(self.hit0)
    }

    /// 遥测：设了 XSEE_TRACE 就把关键计数按行追加进那个文件。
    /// "命中为什么只有 100 多张""用时有没有自己涨"这类账，看文件比看截图准。
    fn trace(&self, ev: &str) {
        let Some(p) = std::env::var_os("XSEE_TRACE") else { return };
        static F: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
        let slot = F.get_or_init(|| {
            Mutex::new(std::fs::OpenOptions::new().create(true).append(true).open(&p).ok())
        });
        let Ok(mut guard) = slot.lock() else { return };
        let Some(f) = guard.as_mut() else { return };
        let (bw, bh) = self.tbox();
        let _ = writeln!(
            f,
            "ev={} box={}x{} n={} got={} err={} q={} hits={} dir_hits={} mk={} sel={} anch={} t0={:.2} tdir={:.2} ready={:.2} dir={}",
            ev,
            bw,
            bh,
            self.n_images(),
            self.got,
            self.err,
            self.queued.len(),
            cache::hits(),
            self.dir_hits(),
            self.marked.len(),
            if self.sel == usize::MAX {
                -1isize
            } else {
                self.sel as isize
            },
            if self.anchor == usize::MAX {
                -1isize
            } else {
                self.anchor as isize
            },
            self.t0.elapsed().as_secs_f64(),
            self.t_dir.elapsed().as_secs_f64(),
            self.ready_sec.unwrap_or(-1.0),
            self.dir.display(),
        );
    }

    fn restart_pool(&mut self) {
        if let Some(o) = self.pool.take() {
            o.shutdown();
        }
        self.thumbs.clear();
        self.queued.clear();
        self.dims.clear();
        self.bpps.clear();
        self.stats.clear();
        self.got = 0;
        self.err = 0;
        self.t0 = Instant::now();
        // 图标档、列表档只画图标、一张缩略图也不生成，池子索性不开。
        if !self.wants_thumb() {
            self.trace("pool");
            return;
        }
        let paths: Vec<String> = self.entries.iter().map(|e| e.path.clone()).collect();
        let (bw, bh) = self.tbox();
        self.pool = Some(Pool::start(
            paths,
            bw,
            bh,
            4,
            true,
            self.fix_mode,
            self.mode == Mode::Detail,
        ));
        self.thumbs.clear();
        self.queued.clear();
        self.dims.clear();
        self.bpps.clear();
        self.stats.clear();
        self.got = 0;
        self.err = 0;
        self.t0 = Instant::now();
        self.trace("pool");
    }

    /// 按三个过滤开关把列表筛一遍。第一格那个"上一级"是回家的路，藏着就回不去了，永远留着。
    /// 只管当前目录这一份列表：类别视图里的图本来就全部分过类，跟着筛等于把画面清空。
    fn apply_filters(&mut self) {
        if !(self.filt_dirs || self.filt_nonimg || self.filt_cat) {
            return;
        }
        let base = usize::from(self.has_up);
        let mut kept: Vec<scan::Entry> = Vec::with_capacity(self.entries.len().saturating_sub(base));
        for i in base..self.entries.len() {
            let e = &self.entries[i];
            if e.is_dir && self.filt_dirs {
                continue;
            }
            if !e.is_dir && !e.is_image && self.filt_nonimg {
                continue;
            }
            if e.is_image
                && self.filt_cat
                && self.cat.as_ref().is_some_and(|c| c.any_cat(&e.path))
            {
                continue;
            }
            kept.push(self.entries[i].clone());
        }
        let mut next: Vec<scan::Entry> = self.entries[..base].to_vec();
        next.extend(kept);
        self.entries = next;
    }

    /// 这一屏里第一次见到的图，当场把入库时间记进库；已经记过的不动（那是它当年进库的时间）。
    /// 只在真记了新名字时写盘——整份库是几兆字节，不能每进一个目录都空写一遍。
    fn note_seen(&mut self) {
        let Some(cat) = self.cat.as_mut() else { return };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut n = 0usize;
        for e in self.entries.iter().filter(|e| e.is_image) {
            if cat.note_time(&e.path, now) {
                n += 1;
            }
        }
        // 还没建过类别（库文件不在）就先只在内存里记，别为浏览一个目录就在
        // 分发包旁边凭空建出 data；等第一次写库把文件建出来以后再补记。
        if n > 0 && catalog_file().exists() {
            let _ = cat.save_full(&catalog_file());
        }
    }

    /// 缩略图重建：把当前列出的每张图绕开缓存重解一遍，顺手挑出对不上的那些。
    /// 返回这一轮要重做几张。
    fn rebuild_thumbs(&mut self) -> usize {
        if !self.wants_thumb() {
            // 图标档、列表档只画图标：这一轮没东西可重建，挂着"重建中"就永远等不到收尾。
            let name = if self.mode == Mode::List { "列表视图" } else { "图标视图" };
            self.msg = format!("{name}不生成缩略图，按 F8 换成缩略图视图再重建");
            return 0;
        }
        let idxs: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_image)
            .map(|(i, _)| i)
            .collect();
        if idxs.is_empty() {
            // 一张图都没列出就别挂着"重建中"：这一轮没有结果会回来，等不到收尾。
            self.end_rebuild();
            return 0;
        }
        self.fix_total = idxs.len();
        self.fix_done = 0;
        self.fix_bad = 0;
        self.fix_mode = true;
        self.restart_pool();
        // 这一轮整份列表都要重做，先全部占住名额：不然滚动时 queue_visible 会把同一张再排一遍，
        // 一张图回两条结果，账就数超了。
        self.queued.extend(idxs.iter().copied());
        if let Some(p) = &self.pool {
            p.submit(&idxs);
        }
        idxs.len()
    }

    /// 换目录/换视图就把重建收掉：那一轮是按老列表排的，接着跑只会算错账。
    fn end_rebuild(&mut self) {
        self.fix_mode = false;
        self.fix_total = 0;
        self.fix_done = 0;
        self.fix_bad = 0;
    }

    /// 重数这一层的总字节：先让状态栏空着，后台数完心跳再补上。
    fn restart_size(&mut self) {
        self.sz_seq += 1;
        self.total_sz = None;
        let paths: Vec<String> = self
            .entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.path.clone())
            .collect();
        spawn_size(paths, self.sz_seq);
    }

    /// 换目录。push=false 给后退/前进用，不能把历史越推越长。
    fn goto(&mut self, dir: &str, hwnd: HWND, push: bool) {
        let mut d = dir.to_string();
        if d.len() > 3 && d.ends_with('\\') {
            d.pop();
        }
        self.dir = PathBuf::from(&d);
        self.src = Src::Folder;
        self.view_title = d.clone();
        self.end_rebuild();
        self.entries = scan::list(&d, self.mode.all_files());
        self.ren_arm = None;
        // 第一格永远是"回到上一级"那个文件夹条目；盘根没有上级就不放。
        self.has_up = false;
        if let Some(up) = self.parent() {
            self.entries.insert(0, scan::Entry { path: up, is_dir: true, is_image: false });
            self.has_up = true;
        }
        self.apply_filters();
        self.note_seen();
        self.restart_size();
        // 目录枚举回来永远是按名的顺序。不补这一刀，刷新或换目录会把人选的
        // 排列方式打回按名，而菜单上的勾还停在原来那一档。
        self.apply_sort();
        self.sel = if self.entries.is_empty() {
            usize::MAX
        } else {
            self.entries.iter().position(|e| !e.is_dir).unwrap_or(0)
        };
        self.anchor = self.sel;
        self.sel_lit = true;
        self.marked.clear();
        self.hover = usize::MAX;
        self.scroll_set(0);
        self.stamp += 1;
        *pv().lock().unwrap() = None;
        // 用时口径：从进这个目录起表，跟缩放/滚动无关，所以只在换目录时重置。
        self.t_dir = Instant::now();
        self.ready_sec = None;
        self.hit0 = cache::hits();
        self.restart_pool();
        self.trace("dir");
        self.sync_tree();
        let title = format!("Xsee · 浏览 — {}", self.dir.display());
        unsafe {
            let _ = SetWindowTextW(hwnd, &HSTRING::from(title));
        }
        if push {
            if self.hpos + 1 < self.hist.len() {
                self.hist.truncate(self.hpos + 1);
            }
            if self.hist.last().map(|p| p != &d).unwrap_or(true) {
                self.hist.push(d.clone());
            }
            self.hpos = self.hist.len().saturating_sub(1);
        }
        self.tsel = format!("{d}\\");
        unsafe {
            self.feed(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }

    /// 按当前排列方式重排。文件夹永远在前，缩略图/尺寸表按新下标重建。
    fn apply_sort(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let sel_path = self.entries.get(self.sel).map(|e| e.path.clone());
        let anchor_path = self.entries.get(self.anchor).map(|e| e.path.clone());
        let up = if self.has_up { self.entries.first().cloned() } else { None };
        let base = usize::from(self.has_up);
        let need_stat = matches!(self.sort, SortKey::Size | SortKey::Date);
        let need_time = self.sort == SortKey::Added;
        // (原下标, 名字, 扩展名, 类型, 大小, 日期, 像素数, 入库时间)
        let mut rows: Vec<(usize, String, String, String, u64, u64, u64, u64)> = Vec::new();
        for i in base..self.entries.len() {
            let e = &self.entries[i];
            let (name, ext, typ) = sort_labels(&e.path);
            let (size, date) = if need_stat {
                match std::fs::metadata(&e.path) {
                    Ok(m) => (
                        m.len(),
                        m.modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    ),
                    Err(_) => (0, 0),
                }
            } else {
                (0, 0)
            };
            let pix = self.dims.get(&i).map(|(w, h)| *w as u64 * *h as u64).unwrap_or(0);
            // 库里没记过时间的排到最后（正向时），不拿 0 冒充 1970 年
            let added = if need_time {
                self.cat.as_ref().and_then(|c| c.time_of(&e.path)).map(|t| t.max(0) as u64).unwrap_or(u64::MAX)
            } else {
                0
            };
            rows.push((i, name, ext, typ, size, date, pix, added));
        }
        let key = self.sort;
        let desc = self.sort_desc;
        let ent = &self.entries;
        rows.sort_by(|a, b| {
            let mut o = match (ent[a.0].is_dir, ent[b.0].is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            };
            if o == std::cmp::Ordering::Equal {
                o = match key {
                    SortKey::Name | SortKey::Desc => a.1.cmp(&b.1),
                    SortKey::Ext => a.2.cmp(&b.2).then(a.1.cmp(&b.1)),
                    SortKey::Size => a.4.cmp(&b.4).then(a.1.cmp(&b.1)),
                    SortKey::Type => a.3.cmp(&b.3).then(a.1.cmp(&b.1)),
                    SortKey::Date => a.5.cmp(&b.5).then(a.1.cmp(&b.1)),
                    SortKey::Image => a.6.cmp(&b.6).then(a.1.cmp(&b.1)),
                    SortKey::Added => a.7.cmp(&b.7).then(a.1.cmp(&b.1)),
                };
                if desc {
                    o = o.reverse();
                }
            }
            o
        });
        let mut next = Vec::with_capacity(rows.len() + 1);
        if let Some(u) = up {
            next.push(u);
        }
        for r in &rows {
            next.push(self.entries[r.0].clone());
        }
        self.entries = next;
        // 本来就没选中（sel=MAX，类别视图的口径）排序后也保持没选中：
        // 落成 0 会凭空把第一张变成光标项，批量操作会误伤这张从没点过的图。
        let was_none = sel_path.is_none() && self.sel == usize::MAX;
        self.sel = sel_path
            .and_then(|p| self.entries.iter().position(|e| e.path == p))
            .unwrap_or_else(|| {
                if was_none || self.entries.is_empty() {
                    usize::MAX
                } else {
                    0
                }
            });
        self.anchor = anchor_path
            .and_then(|p| self.entries.iter().position(|e| e.path == p))
            .unwrap_or(self.sel);
        // 选区本来就是按路径记的，重排不动下标含义，不用再映射一遍。
        self.hover = usize::MAX;
        self.scroll_set(0);
        self.stamp += 1;
        self.restart_pool();
    }

    /// 换目录/换皮肤之后必须把看得见的那几张重新排进池子。restart_pool 把队列和
    /// 已到的图全清了，不在这儿补一笔，池子就空转——一整屏缩略图全白，
    /// 只有滚动或改窗口大小才会重新排，看上去像"缓存没生效"。
    unsafe fn feed(&mut self, hwnd: HWND) {
        let rc = client(hwnd);
        let z = zones(self, rc, menu_h(hwnd));
        self.clamp_scroll(z.grid);
        self.queue_visible(z.grid);
    }

    /// 换皮肤/密度/视图/个人覆盖：尺寸、字体、画刷、查看器一起重算。
    unsafe fn restyle(&mut self, hwnd: HWND) {
        self.th = Theme::new_with_layout(
            &self.skin,
            &self.density,
            self.scale,
            &self.look,
            self.layout.as_deref(),
        );
        xsee_host::viewer::set_theme(&self.th);
        // 图标笔风跟着版式变了，缓存里那批小位图是旧笔画的，整表丢掉重画。
        ik_tiles_reset();
        self.refresh_visual();
        self.restart_pool();
        self.stamp += 1;
        self.feed(hwnd);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }

    /// 菜单悬停预览：只换外观，不碰缩略图池。指针扫过几十套皮肤时，
    /// 每换一套就关池重开、把已解的缩略图全丢掉，菜单会卡得按不动。
    unsafe fn restyle_preview(&mut self, hwnd: HWND) {
        self.th = Theme::new_with_layout(
            &self.skin,
            &self.density,
            self.scale,
            &self.look,
            self.layout.as_deref(),
        );
        xsee_host::viewer::set_theme(&self.th);
        ik_tiles_reset();
        self.refresh_visual();
        self.stamp += 1;
        self.feed(hwnd);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }

    /// 皮肤设置窗改了一处草稿：马上把草稿当成本皮肤的个人覆盖重画一遍，主窗口当场就能
    /// 看到效果。只动内存、不写盘——最后要不要留下，看"是否覆盖当前皮肤"那一句。
    unsafe fn apply_draft(&mut self, hwnd: HWND) {
        let Some(draft) = self
            .dlg
            .as_ref()
            .filter(|d| d.kind == DlgKind::Skin)
            .map(|d| d.look_draft.clone())
        else {
            return;
        };
        self.look = draft;
        self.restyle(hwnd);
    }

    /// 重造 GDI 字体和输入框画刷。只有字号/字体/皮肤色变化时才需要。
    unsafe fn refresh_visual(&mut self) {
        let face = self.look.face().to_string();
        for z in self.zf.iter() {
            for h in [z.f, z.sm, z.b, z.num] {
                if !h.is_invalid() {
                    let _ = DeleteObject(HGDIOBJ(h.0));
                }
            }
        }
        let mut zf = [ZoneFont::default(); theme::NZ];
        for i in 0..theme::NZ {
            let (px, bold) = self.th.zf[i];
            // 次要字比正文小一号；地址栏那一格的小字是右边的尺寸读数，
            // 原来就是"路径字号减 4"，照旧留 4 才和过去一个像素不差。
            let sm = (px - if i == theme::Z_PATH { 4 } else { 1 }).max(8);
            // 地址栏那一整块就是等宽字，别的块只有数字列换 Consolas。
            let main_face = if i == theme::Z_PATH { "Consolas" } else { &face };
            zf[i] = ZoneFont {
                f: mk_font(px, bold, main_face),
                sm: mk_font(sm, bold, &face),
                b: mk_font(px, true, main_face),
                num: mk_font(sm, bold, "Consolas"),
            };
        }
        self.zf = zf;
        // 刚删掉的旧字体还被原生输入框攥着，句柄已悬空：把各自的对应新字体
        // 重新发一遍，不然换肤/改字号后输入框回退系统字体，直到控件重建。
        unsafe {
            for ed in self.conv_edits {
                if !ed.0.is_null() {
                    let _ = SendMessageW(ed, WM_SETFONT, Some(WPARAM(self.zf[theme::Z_ORG].f.0 as usize)), Some(LPARAM(1)));
                }
            }
            if let Some(ed) = self.dlg_edit {
                let _ = SendMessageW(ed, WM_SETFONT, Some(WPARAM(self.zf[theme::Z_DLG].f.0 as usize)), Some(LPARAM(1)));
            }
            if let Some(ed) = self.ren_edit {
                // 改名框创建时拿的是当时的 g.font——一帧画完停在对话框那一块上，取同一支。
                let _ = SendMessageW(ed, WM_SETFONT, Some(WPARAM(zf[theme::Z_DLG].f.0 as usize)), Some(LPARAM(1)));
            }
        }
        self.zone(theme::Z_DLG);
        let brush = CreateSolidBrush(cref(self.th.skin.canvas));
        if !self.brush_edit.0.is_null() {
            let _ = DeleteObject(HGDIOBJ(self.brush_edit.0));
        }
        self.brush_edit = brush;
        // 字体全换新了，存着的那些字块跟着作废。
        dim_tiles_reset();
    }

    /// 进哪一块就把当前几支笔换成那一块的：画法一处没动，字号按版块走。
    /// 路径那支不跟着块换——地址栏那一块就是它，别的块画同一条路径要一样宽。
    fn zone(&mut self, i: usize) {
        let z = self.zf[i];
        self.font = z.f;
        self.font_sm = z.sm;
        self.font_b = z.b;
        self.font_num = z.num;
        self.font_path = self.zf[theme::Z_PATH].f;
    }

    /// 把看得见的顶到队头，再往滚动方向多排两屏。只排没排过的。
    fn queue_visible(&mut self, g: RECT) {
        let (first, last) = self.range(g);
        let span = last.saturating_sub(first).max(1);
        // 惯性还在跑也要排：条带帧只补新露出来的那一格，缩略图不在内存里就只剩空白。
        // 以前速度不为零就整段不排，滚停前扫过的格子只能等停稳后重画。滚动中按滚动
        // 方向往前排两屏（滚轮往下就排下面、往上就排上面），停稳后同样补两屏；留两屏
        // 是因为一屏的提前量在滚得快时经常还没解完就进屏了。详细资料档这一趟要开文件
        // 读头，滚动中仍旧不排，免得慢盘 IO 拖住帧。
        let moving = self.scroll_vel != 0.0;
        let ahead = (span * 2).max(24);
        let mut fresh = Vec::new();
        if !(moving && self.mode == Mode::Detail) {
            let (from, to) = if self.scroll_vel < 0.0 {
                (first.saturating_sub(ahead), last)
            } else {
                (first, (last + ahead).min(self.entries.len()))
            };
            for i in from..to {
                if !self.entries[i].is_image || self.thumbs.contains_key(&i) || self.queued.contains(&i) {
                    continue;
                }
                self.queued.insert(i);
                fresh.push(i);
            }
        }
        let Some(p) = &self.pool else { return };
        if !fresh.is_empty() {
            p.submit(&fresh);
        }
        let vis: Vec<usize> = (first..last).collect();
        p.prioritize(&vis);
    }

    fn pump(&mut self) -> bool {
        let mut got = Vec::new();
        if let Some(p) = &self.pool {
            while let Some(d) = p.recv(Duration::ZERO) {
                got.push(d);
            }
        }
        if got.is_empty() {
            return false;
        }
        for d in got {
            // 详细资料档只要原图尺寸，这一趟根本没解图：不算失败，进度按读到的张数走。
            if d.dims_only {
                if d.dims.0 > 0 {
                    self.dims.insert(d.idx, d.dims);
                }
                if d.bpp > 0 {
                    self.bpps.insert(d.idx, d.bpp);
                }
                self.queued.remove(&d.idx);
                self.got += 1;
                continue;
            }
            if self.fix_mode {
                self.fix_done += 1;
                if d.mismatch {
                    self.fix_bad += 1;
                }
            }
            let bad = d.res.is_err();
            match d.res {
                Ok(t) => {
                    let px = rgba_to_bgra(&t.img.px);
                    self.thumbs.insert(d.idx, Img { w: t.img.w, h: t.img.h, px, fit: None });
                    self.got += 1;
                }
                Err(_) => self.err += 1,
            }
            if d.dims.0 > 0 {
                self.dims.insert(d.idx, d.dims);
            }
            if d.bpp > 0 {
                self.bpps.insert(d.idx, d.bpp);
            }
            // 重建档里解不开的这张要一直占着名额：放开它就会每 60 毫秒重解一次，永远解不开。
            if !(self.fix_mode && bad) {
                self.queued.remove(&d.idx);
            }
        }
        if self.fix_mode {
            // 结果框就在维护页按钮上方，进度写那儿才看得见——状态条被对话框盖住了。
            let line = format!(
                "正在核对当前目录：{} / {}，已发现对不上 {} 张。",
                self.fix_done, self.fix_total, self.fix_bad
            );
            self.msg = line.clone();
            self.db_thumb = line;
            if self.fix_done >= self.fix_total {
                let line = format!(
                    "当前目录：核对了 {} 张，其中 {} 张和缓存里原来那张对不上，已全部换成重解的。",
                    self.fix_total, self.fix_bad
                );
                self.end_rebuild();
                self.db_thumb = line.clone();
                self.msg = line;
            }
        }
        // 最后一张落地就把用时钉死：之后滚动、缩放、重排队都不再改这个数字。
        let n = self.pending_n();
        if n > 0 && self.got + self.err >= n {
            if self.ready_sec.is_none() {
                self.ready_sec = Some(self.t_dir.elapsed().as_secs_f64());
                self.trace("ready");
            }
        }
        self.trace("pump");
        self.prune();
        true
    }

    /// 按内存预算留缩略图：放得下就整个目录都留，放不下才裁。
    /// 以前按"超过 600 张"硬砍，一千多张的目录滚到下半段，上半段就被丢掉，
    /// 再滚回去只能从缓存重读，滚动过程中就是一片空白格。
    fn prune(&mut self) {
        if self.thumbs.is_empty() {
            return;
        }
        let used: usize = self.thumbs.values().map(Img::bytes).sum();
        if used <= THUMB_MEM_MAX {
            return;
        }
        let (a, b) = self.vis;
        let span = b.saturating_sub(a).max(1);
        // 先把离屏一屏以外"按显示尺寸缩好的那份"放掉：再画到会重缩，留着只是白占。
        // 这一条只在超预算时才做——普通目录来回滚，附近那几屏的成品仍然原样留着。
        let near_lo = a.saturating_sub(span);
        let near_hi = b + span;
        for (i, im) in self.thumbs.iter_mut() {
            if *i < near_lo || *i >= near_hi {
                im.fit = None;
            }
        }
        let used: usize = self.thumbs.values().map(Img::bytes).sum();
        if used <= THUMB_MEM_MAX {
            return;
        }
        // 还超预算就把视区里的先全留，再从两侧由近到远往里收，收到预算为止。
        // 上万张的目录因此能留几千张的工作集，而不是固定前后 500 张。
        let mut all: Vec<(usize, Img)> = self.thumbs.drain().collect();
        all.sort_unstable_by_key(|(i, _)| {
            if *i < a {
                a - *i
            } else if *i >= b {
                *i - b + 1
            } else {
                0
            }
        });
        let mut used = 0usize;
        for (i, im) in all {
            let n = im.bytes();
            let in_vis = i >= a && i < b;
            if used + n > THUMB_MEM_MAX && !in_vis {
                continue;
            }
            used += n;
            self.thumbs.insert(i, im);
        }
        let keep: HashSet<usize> = self.thumbs.keys().copied().collect();
        self.dims.retain(|i, _| keep.contains(i));
        self.bpps.retain(|i, _| keep.contains(i));
    }

    /// 方向键 / Home / End / 翻页。Shift 从锚点量一段（两头都算），Ctrl 只挪光标不碰选区，
    /// 什么修饰键都不带就是“只选新的这一项”，和资源管理器一致。
    fn move_sel(&mut self, d: i32, g: RECT, shift: bool, ctrl: bool) {
        let n = self.entries.len();
        if n == 0 {
            return;
        }
        let cols = self.cols_n(g.right - g.left) as i32;
        let lines = (g.bottom - g.top) / self.row_h().max(1);
        let cur = if self.sel == usize::MAX { 0 } else { self.sel as i32 };
        let to = match d {
            -99 => 0,
            99 => n as i32 - 1,
            -98 => (cur - lines * cols).max(0),
            98 => (cur + lines * cols).min(n as i32 - 1),
            x => cur + x,
        };
        let to = to.clamp(0, n as i32 - 1) as usize;
        if shift {
            let a = if self.anchor == usize::MAX { self.sel } else { self.anchor };
            if a != usize::MAX {
                let (a, b) = (a.min(to), a.max(to));
                // 范围里连文件夹一起选，和资源管理器一样：按 Shift 点两下就是屏幕上这一段全部。
                // b 夹到表尾：锚点理论上已随视图重置，这里再兜一道底。
                let add: Vec<String> = (a..=b.min(n - 1))
                    .filter(|&j| self.pickable(j))
                    .map(|j| self.entries[j].path.clone())
                    .collect();
                self.marked = add.into_iter().collect();
            }
            self.sel_lit = true;
        } else if ctrl {
            // Ctrl+方向键只移光标，选区原样留着；光标那一项不画成选中。
            self.sel_lit = false;
        } else {
            let one = if self.pickable(to) { Some(self.entries[to].path.clone()) } else { None };
            self.marked.clear();
            if let Some(p) = one {
                self.marked.insert(p);
            }
            self.anchor = to;
            self.sel_lit = true;
        }
        self.sel = to;
        self.keep_visible(g);
        self.stamp += 1;
    }

    fn keep_visible(&mut self, g: RECT) {
        if self.sel == usize::MAX {
            return;
        }
        let cols = self.cols_n(g.right - g.left).max(1);
        let line = (self.sel / cols) as i32 * self.row_h();
        let h = g.bottom - g.top;
        if line < self.scroll {
            self.scroll_set(line);
        } else if line + self.row_h() > self.scroll + h {
            self.scroll_set(line + self.row_h() - h);
        }
        self.scroll_set(self.scroll.clamp(0, self.max_scroll(g)));
    }

    fn open_sel(&self) -> Option<PathBuf> {
        let e = self.entries.get(self.sel)?;
        (!e.is_dir).then(|| PathBuf::from(&e.path))
    }

    fn parent(&self) -> Option<String> {
        let d = self.dir.to_string_lossy().to_string();
        if d.len() <= 3 && d.ends_with(':') {
            return None;
        }
        let up = d.trim_end_matches('\\');
        let up = up.rsplit_once('\\').map(|(a, _)| a).unwrap_or("");
        if up.is_empty() {
            None
        } else if up.ends_with(':') {
            Some(format!("{up}\\"))
        } else {
            Some(up.to_string())
        }
    }

    fn label(&self) -> String {
        if self.src == Src::Folder {
            self.dir.display().to_string()
        } else {
            self.view_title.clone()
        }
    }

    /// 地址栏选中的那串字，Ctrl+C 走这个口。
    fn addr_sel_text(&self) -> Option<String> {
        let (a, b) = self.addr_sel?;
        if a >= b || b > self.addr_n {
            return None;
        }
        let u: Vec<u16> = self.addr_src.encode_utf16().collect();
        Some(String::from_utf16_lossy(&u[a..b]))
    }

    /// 选区在地址栏上占的那一段 x。省略号态只能整条选中，量出来的末尾夹到框边就行。
    fn addr_sel_x(&self) -> Option<(i32, i32)> {
        let (a, b) = self.addr_sel?;
        if a >= b {
            return None;
        }
        let x0 = self.addr_xs.iter().find(|(o, _)| *o == a).map(|(_, x)| *x)?;
        let x1 = self.addr_xs.iter().find(|(o, _)| *o == b).map(|(_, x)| *x)?;
        Some((x0.max(self.addr_tr.left), x1.min(self.addr_tr.right)))
    }

    /// 把任意来源的一组图片放进主网格（类别、搜索结果共用）。
    fn show_files(&mut self, files: Vec<String>, title: String, src: Src, hwnd: HWND) {
        self.entries = files
            .into_iter()
            .map(|p| scan::Entry { path: p, is_dir: false, is_image: true })
            .collect();
        self.has_up = false;
        // 类别视图只换列表、不替用户选图：没选图片，类别前的勾选框就保持全空
        self.sel = if self.entries.is_empty() || matches!(src, Src::Category(_)) {
            usize::MAX
        } else {
            0
        };
        // 锚点必须跟着重置：它还是上一个目录的下标，列表换短之后
        // Shift 点选/Shift 方向键拿它当 entries 的下标会越界 panic。
        self.anchor = self.sel;
        self.marked.clear();
        self.hover = usize::MAX;
        self.scroll_set(0);
        self.stamp += 1;
        *pv().lock().unwrap() = None;
        self.restart_size();
        self.src = src;
        if src != Src::Search {
            self.sim_q.clear();
        }
        self.view_title = title;
        self.t_dir = Instant::now();
        self.ready_sec = None;
        self.hit0 = cache::hits();
        self.restart_pool();
        self.trace("dir");
        unsafe {
            let _ = SetWindowTextW(
                hwnd,
                &HSTRING::from(format!("Xsee · 浏览 — {}", self.view_title)),
            );
            self.feed(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }

    fn load_category(&mut self, id: i64, hwnd: HWND) {
        let picked = self.cat.as_ref().map(|cat| {
            if id == 0 {
                // 和这一行后面的数字同一个口径：只列挂过类别的图，没分类的不掺进来。
                (cat.categorized_files(), CAT_ALL_ROW.to_string())
            } else {
                (cat.files(id), cat.name(id))
            }
        });
        let Some((files, name)) = picked else { return };
        self.org_sel = id;
        let title = format!("类别 · {name} · {} 张", files.len());
        self.show_files(files, title, Src::Category(id), hwnd);
    }

    fn page_count(&self) -> usize {
        self.search_files.len().div_ceil(PAGE_N).max(1)
    }

    fn show_page(&mut self, hwnd: HWND) {
        let n = self.search_files.len();
        let pages = self.page_count();
        self.page = self.page.min(pages.saturating_sub(1));
        let a = self.page * PAGE_N;
        let b = (a + PAGE_N).min(n);
        let files = self.search_files[a..b].to_vec();
        let title = if !self.sim_q.is_empty() {
            format!(
                "和“{}”相似 · 第 {}/{} 页 · 共 {n} 张",
                self.sim_q,
                self.page + 1,
                pages
            )
        } else if self.search_kw.is_empty() {
            format!("搜索 · 全部图片 · 第 {}/{} 页 · 共 {n} 张", self.page + 1, pages)
        } else {
            format!(
                "搜索“{}” · 第 {}/{} 页 · 共 {n} 张",
                self.search_kw,
                self.page + 1,
                pages
            )
        };
        self.show_files(files, title, Src::Search, hwnd);
    }

    /// 文件操作改完磁盘之后，按当前来源重新取一遍。
    fn refresh_src(&mut self, hwnd: HWND) {
        match self.src {
            Src::Folder => {
                let d = self.dir.display().to_string();
                self.goto(&d, hwnd, false);
            }
            Src::Category(id) => self.load_category(id, hwnd),
            Src::Search => self.show_page(hwnd),
        }
    }

    /// 当前选中（没有多选就是光标那一项）的真实路径；虚拟的"上一级"不算。
    /// Ctrl 把最后一项点掉之后光标还在，但 `sel_lit` 已经落下，这里就不能再把它当选中。
    fn selected_paths(&self) -> Vec<String> {
        let mut v: Vec<String> = if !self.marked.is_empty() {
            self.marked.iter().cloned().collect()
        } else if self.sel_lit {
            match self.entries.get(self.sel) {
                Some(e) if !(self.has_up && self.sel == 0) => vec![e.path.clone()],
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };
        v.sort();
        v
    }

    /// 当前选中的图片，勾选类别用；文件夹不算。
    fn selected_images(&self) -> Vec<String> {
        let mut v = self.selected_paths();
        v.retain(|p| scan::is_image(p));
        v
    }

    /// 只吃文件的那些批量操作的作用范围：转换、批量改名。选区里可以挂着文件夹
    /// （资源管理器也这么选），但那两类活儿对着文件夹使没有意义，先摘出去。
    fn selected_files(&self) -> Vec<String> {
        self.selected_paths()
            .into_iter()
            .filter(|p| !Path::new(p).is_dir())
            .collect()
    }
}

/// 类别和搜索视图不跟着筛——那本来就是一批筛好的图，再套一层只会把画面清空。
unsafe fn refilter(hwnd: HWND) {
    let d = {
        let g = app().lock().unwrap();
        if !matches!(g.src, Src::Folder) {
            return;
        }
        g.dir.to_string_lossy().to_string()
    };
    app().lock().unwrap().goto(&d, hwnd, false);
    request_pv(hwnd);
    refresh(hwnd);
}

/// 这一层的字节数在别的线程上逐个问文件大小：一千五百张在 G 盘上问一遍要一秒多，
/// 全压在点类别那一帧上就是明显的一顿。后台点完只把总数交回心跳，状态栏那一格补一下。
/// 名单越换越大也无所谓——每 32 张回看一眼有没有被更新的一次点击顶掉，顶掉就收手，
/// 免得连点十几个类别攒出一堆线程一起抢硬盘。
static SZ_TOTAL: AtomicU64 = AtomicU64::new(0);
static SZ_WANT: AtomicUsize = AtomicUsize::new(0);
static SZ_DONE: AtomicUsize = AtomicUsize::new(0);

fn spawn_size(paths: Vec<String>, seq: usize) {
    SZ_WANT.store(seq, Ordering::Release);
    std::thread::spawn(move || {
        let mut sum: u64 = 0;
        for (i, p) in paths.iter().enumerate() {
            if i % 32 == 0 && SZ_WANT.load(Ordering::Relaxed) != seq {
                return;
            }
            if let Ok(m) = std::fs::metadata(p) {
                sum = sum.saturating_add(m.len());
            }
        }
        SZ_TOTAL.store(sum, Ordering::Relaxed);
        SZ_DONE.store(seq, Ordering::Release);
    });
}

fn rgba_to_bgra(px: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; px.len()];
    for (d, s) in out.chunks_exact_mut(4).zip(px.chunks_exact(4)) {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = s[0];
        d[3] = 0xFF;
    }
    out
}

/// 区域划分：画和点都用同一份，两边不会算歪。
/// 摆放照设计稿 work/ui：左边一栏上下两格（文件夹树 + 预览），右边整块是文件列表，
/// 列表底下压一条进度提示，最底是六段状态栏。
/// 一个版块的笔：正文、比正文小一号的次要字、正文强制粗、等宽数字。
/// 地址栏那一块的正文本身就是等宽字体，建的时候单独给。
#[derive(Clone, Copy, Default)]
struct ZoneFont {
    f: HFONT,
    sm: HFONT,
    b: HFONT,
    num: HFONT,
}

#[derive(Clone, Copy)]
struct Zones {
    bar: RECT,
    path: RECT,
    /// 左上树格（含窗格头，画的时候自己让出头部高度）
    tree: RECT,
    /// 左下预览格整块（含窗格头）
    pv: RECT,
    grid: RECT,
    /// 缩略图区右侧的竖滚动条槽。
    gsb: RECT,
    hint: RECT,
    status: RECT,
    /// 左栏和列表之间的竖分隔
    spv: RECT,
    /// 左栏内树和预览之间的横分隔
    sph: RECT,
    /// 右侧"整理"面板；关掉时宽 0。
    org: RECT,
    /// 整理面板左侧的竖分隔：往左拖就是加宽类别栏。面板关掉时宽 0。
    spo: RECT,
}

fn zones(a: &App, rc: RECT, _menu_h: i32) -> Zones {
    let s = &a.th;
    let top = rc.top + s.title + s.menu;
    let bh = if a.bar_main { s.bar } else { 0 };
    let ph = if a.bar_path { s.path } else { 0 };
    let t = top + bh + ph;
    let bot = rc.bottom - if a.bar_status { s.status } else { 0 };
    // 左栏两块都关掉就没有这一栏了：竖分隔一起收，列表吃满整宽
    let col = a.pane_tree || a.pane_pv;
    let lr = rc.left + if col { a.left_w } else { 0 };
    let hint_h = (s.row as f64 * 0.9).round().max(16.0) as i32;
    // 预览块高度没初始化时按客户区的三成算，和设计稿的 clamp(150,30%,400) 同一条规则
    let min = (150.0 * s.scale).round() as i32;
    // 上半的树至少要留 110 逻辑像素，不然预览一拖就把树格压没了
    let max = (bot - t - s.split - s.pane_head_h * 2 - (110.0 * s.scale).round() as i32).max(min);
    let want = if a.pv_h > 0 { a.pv_h } else { ((bot - t) as f64 * 0.3).round() as i32 };
    // 只开一块时那一块吃满整栏，没有可拖的份，高度按 0 算
    let pv_h = if a.pane_tree && a.pane_pv { want.clamp(min, max) } else { 0 };
    let pv_top = bot - pv_h;
    let sbw = (10.0 * s.scale).max(8.0) as i32;
    let gl = lr + if col { s.split } else { 0 };
    let gh = bot - hint_h;
    // 右栏：整理和批量转换共用这一条。哪一页开着都得留出宽度。
    let dock = a.org_open || a.conv_open;
    // 客户区窄到放不下右栏时上界会算成负数，clamp 见到 min>max 直接 panic，
    // 而这里是从 WndProc 里调的，一 panic 就是整个进程 abort。上界收得住再夹。
    let org_lo = (170.0 * s.scale) as i32;
    let org_w = if dock {
        a.org_w.clamp(org_lo, ((rc.right - gl - s.split) / 2).max(org_lo))
    } else {
        0
    };
    let right = (rc.right - org_w).max(gl + 1);
    // 分隔条从列表这一侧让出来：右栏量到的还是 org_w，列表少一条的宽
    let seam = if dock { right - s.split } else { right };
    let grid_edge = seam.max(gl + 1);
    let grid_r = (grid_edge - sbw).max(gl + 1);
    // 详细资料档的列头常驻在列表顶上，从 grid 里扣掉最上面那一条。
    // 不扣的话第一行（"上一层"那一条）正好压在列头底下，点列头就成了点它，
    // 看着像"点名称跳上了上级"。扣掉之后行、命中、滚动量共用同一个 top。
    let head = if a.mode == Mode::Detail { s.pane_head_h } else { 0 };
    // 空区域：宽 0，over() 永不命中，画也是空操作。关掉的窗格占位用它。
    let none = RECT { left: rc.left, top, right: rc.left, bottom: top };
    Zones {
        bar: if a.bar_main {
            RECT { left: rc.left, top, right: rc.right, bottom: top + bh }
        } else {
            none
        },
        path: if a.bar_path {
            RECT { left: rc.left, top: top + bh, right: rc.right, bottom: t }
        } else {
            none
        },
        tree: if a.pane_tree {
            RECT { left: rc.left, top: t, right: lr, bottom: if a.pane_pv { pv_top - s.split } else { bot } }
        } else {
            none
        },
        pv: if a.pane_pv {
            RECT { left: rc.left, top: if a.pane_tree { pv_top } else { t }, right: lr, bottom: bot }
        } else {
            none
        },
        grid: RECT { left: gl, top: t + head, right: grid_r, bottom: gh },
        gsb: RECT { left: grid_r, top: t, right: grid_edge, bottom: gh },
        hint: RECT { left: gl, top: gh, right: grid_r, bottom: bot },
        status: if a.bar_status {
            RECT { left: rc.left, top: bot, right: rc.right, bottom: rc.bottom }
        } else {
            none
        },
        spv: if col { RECT { left: lr, top: t, right: lr + s.split, bottom: bot } } else { none },
        sph: if a.pane_tree && a.pane_pv {
            RECT { left: rc.left, top: pv_top - s.split, right: lr, bottom: pv_top }
        } else {
            none
        },
        org: RECT { left: right, top: t, right: rc.right, bottom: bot },
        spo: if dock {
            RECT { left: grid_edge, top: t, right: right, bottom: bot }
        } else {
            none
        },
    }
}

// 纯色刷子按颜色存下来反复用。FillRect / FrameRect 只是借刷子描一片色，从不把它
// 选进哪个 DC，所以缓存没有"正被选中的对象删不掉"那类麻烦。一帧要铺七百多次色，
// 每次都现建现销：建一个实测一块多微秒，铺色那一段一千八百微秒里大半是建销的钱。
thread_local! {
    static BRUSHES: std::cell::RefCell<std::collections::HashMap<u32, HBRUSH>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// 取这档纯色的刷子。表涨到 2048 色就整表换新：一个进程只有约一万个 GDI 对象的配额，
/// 皮肤来回换再多也不该把它吃光。
unsafe fn brush_cached(c: u32) -> HBRUSH {
    BRUSHES.with(|slot| {
        let mut m = slot.borrow_mut();
        if let Some(b) = m.get(&c) {
            return *b;
        }
        if m.len() >= 2048 {
            for (_, b) in m.drain() {
                let _ = DeleteObject(HGDIOBJ(b.0));
            }
        }
        let b = CreateSolidBrush(cref(c));
        m.insert(c, b);
        b
    })
}

unsafe fn fill(hdc: HDC, r: &RECT, c: u32) {
    let t = SUB_DEEP.load(Ordering::Relaxed).then(Instant::now);
    let _ = FillRect(hdc, r, brush_cached(c));
    deep_add(0, t);
}

unsafe fn frame(hdc: HDC, r: &RECT, c: u32) {
    let _ = FrameRect(hdc, r, brush_cached(c));
}

unsafe fn hline(hdc: HDC, x0: i32, x1: i32, y: i32, c: u32) {
    let r = RECT { left: x0, top: y, right: x1.max(x0 + 1), bottom: y + 1 };
    fill(hdc, &r, c);
}

/// 整块盖一层半透明色。FillRect 是不透明的，直接画会把底下的选框糊掉；
/// 提示纱要让人看见内容还在原地，只能走 AlphaBlend 的常量 alpha。
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
        // 0 = 不看源图逐像素 alpha，只用上面那个常量值
        AlphaFormat: 0,
    };
    let _ = AlphaBlend(hdc, r.left, r.top, w, h, src, 0, 0, w, h, bf);
    SelectObject(src, old);
    let _ = DeleteObject(HGDIOBJ(dib.0));
    let _ = DeleteDC(src);
}

/// 每个区开画前把自己夹进那一格里：换视图时行高突变，旧滚动量会把上一屏的
/// 格子顶到工具条和路径条上面去，夹住就只是看不见，不会画花。
unsafe fn clip(hdc: HDC, r: &RECT) {
    let rgn = CreateRectRgnIndirect(r as *const RECT);
    let _ = ExtSelectClipRgn(hdc, Some(rgn), RGN_AND);
    let _ = DeleteObject(HGDIOBJ(rgn.0));
}

unsafe fn unclip(hdc: HDC) {
    let _ = SelectClipRgn(hdc, None);
}

/// 字一律走透明底：底下是什么就混什么。给实心底（SetBkColor + OPAQUE）那条路
/// 实测过，一帧省不下半毫秒 —— GDI 该起的临时面照样起，只是拿它去垫色而不是去混色。
/// 既然没赚到，就不在字后面多盖一块底。
unsafe fn txt(hdc: HDC, r: &RECT, s: &str, c: u32, font: HFONT, extra: DRAW_TEXT_FORMAT) {
    let t = SUB_DEEP.load(Ordering::Relaxed).then(Instant::now);
    txt_raw(hdc, r, s, c, font, extra);
    deep_add(TXT_SLOT.load(Ordering::Relaxed), t);
}

/// 不计时的那一半，给自带计时的调用方用，免得同一笔钱记两遍。
unsafe fn txt_raw(hdc: HDC, r: &RECT, s: &str, c: u32, font: HFONT, extra: DRAW_TEXT_FORMAT) {
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

/// 单行文字宽度。工具条按这个决定按钮多宽，不然字会被裁。
unsafe fn text_w(hdc: HDC, s: &str, font: HFONT) -> i32 {
    SelectObject(hdc, HGDIOBJ(font.0));
    let mut u: Vec<u16> = s.encode_utf16().collect();
    let mut r = RECT { left: 0, top: 0, right: 2000, bottom: 200 };
    let _ = DrawTextW(hdc, &mut u, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
    (r.right - r.left).max(8)
}

unsafe fn dib(hdc: HDC, dst: &RECT, src: &Img) {
    if src.w == 0 || src.h == 0 || dst.right <= dst.left || dst.bottom <= dst.top {
        return;
    }
    let t = SUB_DEEP.load(Ordering::Relaxed).then(Instant::now);
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
    // 取样档次：HALFTONE 是加权平均，一张 0.19 毫秒；COLORONCOLOR 丢行抽样，快得多。
    // 降的只有取样档次 —— 显示尺寸、文件名、尺寸行一个字都不动。
    let _ = SetStretchBltMode(hdc, COLORONCOLOR);
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
    deep_add(1, t);
}

/// 这一批里后台工线真缩了几次（遥测用）：正常情况下每格只在头一次缩。
static RESAMPLES: AtomicU32 = AtomicU32::new(0);
/// 这一批里列表帧走了条带几次、整块重画几次（遥测用）：条带不生效就是白搬。
static BAND_FRAMES: AtomicU32 = AtomicU32::new(0);
static GRID_FRAMES: AtomicU32 = AtomicU32::new(0);

/// 32 位 BGRA 的 DIB 头；高取负数就是顶向下那套行序。
fn dib_info(w: i32, h: i32) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// 把缓存那张缩到图位里的显示尺寸，结果抓回内存。同一目录、同一窗口宽度下
/// 每格的显示尺寸帧帧一样，所以这张图只缩这一次；后面每帧贴的就是这份。
unsafe fn resample(hdc: HDC, src: &Img, dw: i32, dh: i32) -> Option<Vec<u8>> {
    RESAMPLES.fetch_add(1, Ordering::Relaxed);
    let sbmi = dib_info(src.w as i32, -(src.h as i32));
    let obmi = dib_info(dw, -dh);
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let Ok(dib) = CreateDIBSection(None, &obmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
        return None;
    };
    if bits.is_null() {
        let _ = DeleteObject(HGDIOBJ(dib.0));
        return None;
    }
    let mem = CreateCompatibleDC(Some(hdc));
    let old = SelectObject(mem, HGDIOBJ(dib.0));
    // 和逐帧重缩走的是同一档取样，画出来的像素没两样
    let _ = SetStretchBltMode(mem, COLORONCOLOR);
    let mut org = POINT::default();
    let _ = SetBrushOrgEx(mem, 0, 0, Some(&mut org));
    let _ = StretchDIBits(
        mem,
        0,
        0,
        dw,
        dh,
        0,
        0,
        src.w as i32,
        src.h as i32,
        Some(src.px.as_ptr() as *const core::ffi::c_void),
        &sbmi,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
    // 让系统把这块位图的像素写进我们自己分配的缓冲：行距由 GDI 算，不去裸读它的内存
    let mut out = vec![0u8; (dw as usize) * (dh as usize) * 4];
    let mut wbmi = obmi;
    let ok = GetDIBits(
        mem,
        dib,
        0,
        dh as u32,
        Some(out.as_mut_ptr() as *mut core::ffi::c_void),
        &mut wbmi,
        DIB_RGB_COLORS,
    );
    SelectObject(mem, old);
    let _ = DeleteDC(mem);
    let _ = DeleteObject(HGDIOBJ(dib.0));
    if ok == 0 {
        return None;
    }
    Some(out)
}

/// 网格里贴缩略图那一步：显示尺寸和上次一样就把存好的那份直接贴上去，
/// 变了（改窗口宽、换缩略图大小、新滚进来的）才重缩一次。
unsafe fn dib_cache(hdc: HDC, dst: &RECT, im: &mut Img) {
    let dw = dst.right - dst.left;
    let dh = dst.bottom - dst.top;
    if im.w == 0 || im.h == 0 || dw <= 0 || dh <= 0 {
        return;
    }
    let t = SUB_DEEP.load(Ordering::Relaxed).then(Instant::now);
    let same = im.fit.as_ref().map(|f| f.w == dw && f.h == dh).unwrap_or(false);
    if !same {
        if let Some(px) = resample(hdc, im, dw, dh) {
            im.fit = Some(Fit { w: dw, h: dh, px });
        }
    }
    match im.fit.as_ref() {
        Some(f) => {
            let bmi = dib_info(f.w, -f.h);
            let _ = StretchDIBits(
                hdc,
                dst.left,
                dst.top,
                dw,
                dh,
                0,
                0,
                f.w,
                f.h,
                Some(f.px.as_ptr() as *const core::ffi::c_void),
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            deep_add(1, t);
        }
        // 那份成品没抓住（内存吃紧）就退回逐帧重缩，画面不受影响
        None => dib(hdc, dst, im),
    }
}

/// 图在框内按原比例居中，小图不放大。
fn box_in(iw: u32, ih: u32, r: RECT) -> RECT {
    let bw = (r.right - r.left).max(1);
    let bh = (r.bottom - r.top).max(1);
    let s = (bw as f64 / iw as f64).min(bh as f64 / ih as f64).min(1.0);
    let w = (iw as f64 * s).round().max(1.0) as i32;
    let h = (ih as f64 * s).round().max(1.0) as i32;
    let x = r.left + (bw - w) / 2;
    let y = r.top + (bh - h) / 2;
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

/// SVG 路径 → 折线。设计稿里的 d 只用到 M/L/H/V/C/S/A/Z（含相对形式），
/// 弧线按 15° 采样，26px 的图标上看不出多边形。
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
        // 否则一条 d 里第一个 z 就把整串截断，四宫格只剩左上那一格。
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

thread_local! {
    /// 这一帧工具栏图标用哪套开源图标库；None = 内置那一套。
    /// 画图标的那一层手上没有 Theme，所以每帧开头把选中的集合记在这儿。
    static ICON_SET: std::cell::Cell<Option<&'static glyphs::Set>> = const { std::cell::Cell::new(None) };
}

/// 按网格单位建一支圆头图标笔。建不出来就退回普通笔，宁可笔风不对也不画空。
unsafe fn icon_pen(w: f64, u: f64, col: u32) -> HPEN {
    let width = (w * u).round().max(1.0) as u32;
    let style = PEN_STYLE(PS_GEOMETRIC.0 | PS_ENDCAP_ROUND.0 | PS_JOIN_ROUND.0);
    let lb = LOGBRUSH { lbStyle: BS_SOLID, lbColor: cref(col), ..Default::default() };
    let pen = ExtCreatePen(style, width, &lb, None);
    if pen.0.is_null() {
        CreatePen(PS_GEOMETRIC, width as i32, cref(col))
    } else {
        pen
    }
}

/// 实心多边形填充。SVG 的 opacity 用 mix(col,bg) 现算，皮肤换了它跟着换。
/// `even` 对应 SVG 的 fill-rule：evenodd 走 ALTERNATE，其余走 WINDING。
unsafe fn fill_svg_rule(
    hdc: HDC,
    x: i32,
    y: i32,
    u: f64,
    col: u32,
    bg: u32,
    alpha: f64,
    even: bool,
    d: &str,
) {
    let c = if alpha >= 0.999 { col } else { mix(col, bg, 1.0 - alpha) };
    let old = SetPolyFillMode(hdc, if even { ALTERNATE } else { WINDING });
    // 一条 path 里的多个子路径必须一次交出去，孔洞才挖得出来：逐条 Polygon
    // 会把每个子路径都当成实心形单独填，实心档的“i”“齿轮孔”就全糊成一块饼。
    let mut pts: Vec<POINT> = Vec::new();
    let mut counts: Vec<i32> = Vec::new();
    for sub in parse_path(d) {
        if sub.len() < 3 {
            continue;
        }
        counts.push(sub.len() as i32);
        pts.extend(
            sub.iter()
                .map(|(a, b)| POINT { x: x + (a * u).round() as i32, y: y + (b * u).round() as i32 }),
        );
    }
    if !counts.is_empty() {
        let (ob, op, br) = ibrush(hdc, c);
        let _ = PolyPolygon(hdc, pts.as_ptr(), &counts);
        irestore(hdc, ob, op, br);
    }
    let _ = SetPolyFillMode(hdc, CREATE_POLYGON_RGN_MODE(old));
}

/// 内置图形沿用的填充口径：GDI 默认的 ALTERNATE，和以前的 Polygon 一模一样。
unsafe fn fill_svg(hdc: HDC, x: i32, y: i32, u: f64, col: u32, bg: u32, alpha: f64, d: &str) {
    fill_svg_rule(hdc, x, y, u, col, bg, alpha, true, d);
}

unsafe fn stroke_svg(hdc: HDC, x: i32, y: i32, u: f64, col: u32, w: f64, d: &str) {
    let pen = icon_pen(w, u, col);
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
    let c = if alpha >= 0.999 { col } else { mix(col, bg, 1.0 - alpha) };
    let (px, py, pr) = (x + (cx * u).round() as i32, y + (cy * u).round() as i32, (r * u).round().max(1.0) as i32);
    let (ob, op, br) = ibrush(hdc, c);
    let _ = Ellipse(hdc, px - pr, py - pr, px + pr, py + pr);
    irestore(hdc, ob, op, br);
}

unsafe fn ell_stroke_svg(hdc: HDC, x: i32, y: i32, u: f64, col: u32, w: f64, cx: f64, cy: f64, r: f64) {
    let (px, py, pr) = (x + (cx * u).round() as i32, y + (cy * u).round() as i32, (r * u).round().max(1.0) as i32);
    let pen = icon_pen(w, u, col);
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let ob = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    let _ = Ellipse(hdc, px - pr, py - pr, px + pr, py + pr);
    SelectObject(hdc, ob);
    SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(pen.0));
}

/// 工具栏/树/路径栏的单色图标。树、路径栏和卡片照 browse.tpl.html；
/// 工具栏 11 个键沿用最早 reg/refine 稿的线性图标。半透明层按 mix(col,bg) 现算。
/// 树和分类用的线描图标：A/B/C 的树不再沿用旧版实心文件夹。
/// 卡片格里的文件夹仍用实心图标，避免把中间的缩略图区也一起改成线稿。
unsafe fn icon_line(hdc: HDC, k: Ik, x: i32, y: i32, sz: i32, col: u32, bg: u32) {
    let u = sz as f64 / 24.0;
    match k {
        Ik::Folder => {
            fill_svg(hdc, x, y, u, col, bg, 0.10, "M3.2 6.2a1.2 1.2 0 0 1 1.2-1.2h4.7l1.8 2h7.9a1.2 1.2 0 0 1 1.2 1.2v8.6a1.2 1.2 0 0 1-1.2 1.2H4.4a1.2 1.2 0 0 1-1.2-1.2z");
            stroke_svg(hdc, x, y, u, col, 1.45, "M3.2 6.2a1.2 1.2 0 0 1 1.2-1.2h4.7l1.8 2h7.9a1.2 1.2 0 0 1 1.2 1.2v8.6a1.2 1.2 0 0 1-1.2 1.2H4.4a1.2 1.2 0 0 1-1.2-1.2z");
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.44), 1.25, "M3.4 9.5h17.2");
        }
        Ik::Pc => {
            stroke_svg(hdc, x, y, u, col, 1.45, "M3.3 4.4h17.4v10.5H3.3z");
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.30), 1.35, "M9 17.7h6M7.2 20h9.6");
            fill_svg(hdc, x, y, u, col, bg, 0.16, "M3.3 4.4h17.4v10.5H3.3z");
        }
        Ik::Drive => {
            stroke_svg(hdc, x, y, u, col, 1.45, "M3.2 6.8h17.6v10.4H3.2z");
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.42), 1.25, "M6.3 14.2h2.2M15.5 14.2h2.2");
            fill_svg(hdc, x, y, u, col, bg, 0.10, "M3.2 6.8h17.6v10.4H3.2z");
        }
        Ik::File => {
            fill_svg(hdc, x, y, u, col, bg, 0.10, "M6.2 3.4h7.4l4.2 4.2v13H6.2z");
            stroke_svg(hdc, x, y, u, col, 1.45, "M6.2 3.4h7.4l4.2 4.2v13H6.2z");
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.34), 1.3, "M13.6 3.4v4.2h4.2");
        }
        Ik::Pic => {
            stroke_svg(hdc, x, y, u, col, 1.45, "M3.3 5.2h17.4v13.6H3.3z");
            stroke_svg(hdc, x, y, u, col, 1.45, "m5.2 16 4.5-5.4 3.1 3.8 2.1-2.3 4 3.9");
            ell_stroke_svg(hdc, x, y, u, col, 1.25, 8.2, 9.0, 1.6);
        }
        Ik::Star => {
            stroke_svg(hdc, x, y, u, col, 1.45, "m12 3.5 2.6 5.5 6 .8-4.4 4.1 1.1 6-5.3-2.9-5.3 2.9 1.1-6L3.4 9.8l6-.8z");
        }
        _ => icon(hdc, k, x, y, sz, col, bg),
    }
}

/// 列表/详细两档一屏两百多枚图标，只有三种：文件夹、图片、其他文件，选中再换一档字色。
/// 每帧把同样的 SVG 路径重新解析一遍、笔刷建了又删，一帧光这一项就要二十毫秒。
/// 同样的（种类、尺寸、字色、底色）画出来逐像素一样，所以第一枚画进一张小位图，
/// 之后把建图用的那种哨兵色抠掉、整块贴过去——镂空处透出的是行底下真正的地面，
/// 跟现画时一模一样。
const IK_FOLDER: u8 = 0;
const IK_PIC: u8 = 1;
const IK_FILE: u8 = 2;
/// 图片的格式牌从这一档起排：`IK_FMT + 格式表下标`。详细档用带 X 的版式。
const IK_FMT: u8 = 16;
/// 列表档的格式牌另起一档：同一张图在列表里只有格式名，不能和详细档共用缓存。
/// 起点必须让开 `IK_FMT + FMTS::len()` 整段——格式牌一多，两段就会撞号，
/// 缓存里取到的是另一种牌（曾经 32 起步，FF/PNM/JXR/SVG 在图标档显示成 JPG/PNG/PSD/GIF）。
const IK_FMT_LIST: u8 = 64;

thread_local! {
    static IK_TILES: std::cell::RefCell<std::collections::HashMap<(u8, i32, u32, u32), (HBITMAP, u32)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// 建图时铺底、贴的时候当透明色的那一种颜色。图标里画出来的颜色都落在
/// 底色与字色之间的连线上，这几档怪色碰不到它。
fn ik_sentinel(col: u32, bg: u32) -> u32 {
    for c in [0x00ff00ffu32, 0x0000ff00, 0x000000ff, 0x00ff8000] {
        if c != col && c != bg {
            return c;
        }
    }
    0x00000001
}

unsafe fn ik_kind(kind: u8, hdc: HDC, x: i32, y: i32, sz: i32, col: u32, bg: u32) {
    match kind {
        IK_FOLDER => icon(hdc, Ik::Folder, x, y, sz, col, bg),
        IK_PIC => icon_pic_solid(hdc, x, y, sz, col, bg),
        IK_FILE => icon(hdc, Ik::File, x, y, sz, col, bg),
        k if k >= IK_FMT_LIST => {
            fmticon::draw_list(hdc, x, y, sz, (k - IK_FMT_LIST) as usize, col, bg)
        }
        // 图标档直接画 48 像素那份文件图标：牌面占满、格式名放大，
        // 和资源管理器里看到的是同一张。
        k => fmticon::draw_icon(hdc, x, y, sz, (k - IK_FMT) as usize, col, bg),
    }
}

unsafe fn ik_tiles_reset() {
    // 删位图前先从常驻 DC 上摘下来，不然 DeleteObject 不认。
    scratch_drop();
    IK_TILES.with(|c| {
        let mut v = c.borrow_mut();
        for (b, _) in v.values() {
            let _ = DeleteObject(HGDIOBJ(b.0));
        }
        v.clear();
    });
}

unsafe fn icon_cached(hdc: HDC, kind: u8, x: i32, y: i32, sz: i32, col: u32, bg: u32) {
    let key = (kind, sz, col, bg);
    let hit = IK_TILES.with(|c| c.borrow().get(&key).map(|(b, t)| (*b, *t)));
    let (bmp, trans) = match hit {
        Some(v) => v,
        None => {
            let trans = ik_sentinel(col, bg);
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: sz.max(2),
                    biHeight: -(sz.max(2)),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let Ok(built) = CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
                ik_kind(kind, hdc, x, y, sz, col, bg);
                return;
            };
            let Some(mem) = scratch(hdc) else {
                let _ = DeleteObject(HGDIOBJ(built.0));
                ik_kind(kind, hdc, x, y, sz, col, bg);
                return;
            };
            let old = SelectObject(mem, HGDIOBJ(built.0));
            let side = sz.max(2);
            let local = RECT { left: 0, top: 0, right: side, bottom: side };
            fill(mem, &local, trans);
            ik_kind(kind, mem, 0, 0, sz, col, bg);
            SelectObject(mem, old);
            // 换肤、换缩放会攒出一批新键，越界就整表重来。格式牌一种颜色一张，
            // 十种格式再乘浅底/深底两档，门槛要留够，不然会边画边清。
            if IK_TILES.with(|c| c.borrow().len()) >= 128 {
                ik_tiles_reset();
            }
            IK_TILES.with(|c| c.borrow_mut().insert(key, (built, trans)));
            (built, trans)
        }
    };
    if let Some(mem) = scratch(hdc) {
        SelectObject(mem, HGDIOBJ(bmp.0));
        if TransparentBlt(hdc, x, y, sz, sz, mem, 0, 0, sz, sz, cref(trans).0).as_bool() {
            return;
        }
    }
    ik_kind(kind, hdc, x, y, sz, col, bg);
}

/// 图标档里的图片图标：外框描实一圈，视觉大小才和旁边那枚实心文件夹对得上；
/// 共用版 Ik::Pic 的框只有 0.28 透明度，远看只剩中间那座山，比文件夹小一圈。
unsafe fn icon_pic_solid(hdc: HDC, x: i32, y: i32, sz: i32, col: u32, bg: u32) {
    let u = sz as f64 / 24.0;
    fill_svg(hdc, x, y, u, col, bg, 0.30, "M3 5h18v14H3z");
    stroke_svg(hdc, x, y, u, col, 1.6, "M3 5h18v14H3z");
    fill_svg(hdc, x, y, u, col, bg, 1.0, "m5 17 5-6 3.2 4 2.3-2.6L19 17z");
    ell_svg(hdc, x, y, u, col, bg, 1.0, 8.5, 9.0, 1.6);
}

/// 图标槽位：和 glyphs::Set 里那十枚的顺序一一对应。
fn ik_slot(k: Ik) -> Option<usize> {
    Some(match k {
        Ik::Prev => 0,
        Ik::Next => 1,
        Ik::Up => 2,
        Ik::Find => 3,
        Ik::Edit => 4,
        Ik::Move => 5,
        Ik::Copy => 6,
        Ik::Del => 7,
        Ik::Prop => 8,
        Ik::Skin => 9,
        _ => return None,
    })
}

/// 工具栏那十枚图标按当前选中的开源库画：图形是库里的原件，只按网格单位换算到
/// sz 大小。fill 走多边形填充（evenodd 用 ALTERNATE），stroke 用库自带线宽。
unsafe fn set_icon(hdc: HDC, k: Ik, x: i32, y: i32, sz: i32, col: u32, bg: u32) -> bool {
    let Some(set) = ICON_SET.with(|c| c.get()) else { return false };
    let Some(g) = ik_slot(k).and_then(|i| set.glyphs[i]) else { return false };
    draw_glyph(hdc, g, x, y, sz, col, bg);
    true
}

/// 按图标自带的网格把一枚 Glyph 画进 sz×sz 的方格里。工具栏和菜单共用这一份算法。
unsafe fn draw_glyph(hdc: HDC, g: &glyphs::Glyph, x: i32, y: i32, sz: i32, col: u32, bg: u32) {
    let u = sz as f64 / g.grid as f64;
    for p in g.parts {
        let px = x + (p.dx as f64 * u).round() as i32;
        let py = y + (p.dy as f64 * u).round() as i32;
        if p.fill {
            fill_svg_rule(hdc, px, py, u, col, bg, p.alpha as f64, p.even, p.d);
        } else {
            stroke_svg(hdc, px, py, u, col, p.w as f64, p.d);
        }
    }
}

unsafe fn icon(hdc: HDC, k: Ik, x: i32, y: i32, sz: i32, col: u32, bg: u32) {
    if set_icon(hdc, k, x, y, sz, col, bg) {
        return;
    }
    let u = sz as f64 / 24.0;
    match k {
        Ik::Folder => {
            fill_svg(hdc, x, y, u, col, bg, 1.0, "M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v3H3z");
            fill_svg(hdc, x, y, u, col, bg, 0.72, "M3 10h18v8a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z");
        }
        Ik::Drive => {
            fill_svg(hdc, x, y, u, col, bg, 1.0, "M3 8h18v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z");
            ell_svg(hdc, x, y, u, 0xffffff, bg, 0.8, 18.0, 12.5, 1.2);
            fill_svg(hdc, x, y, u, col, bg, 0.6, "M5 5h9l2 3H4z");
        }
        Ik::Pc => {
            fill_svg(hdc, x, y, u, col, bg, 1.0, "M3 4h18v11H3z");
            fill_svg(hdc, x, y, u, col, bg, 0.6, "M9 17h6v2H9zM6 19h12v1.5H6z");
        }
        Ik::Star => fill_svg(hdc, x, y, u, col, bg, 1.0, "m12 3 2.7 5.7 6.3.8-4.6 4.3 1.2 6.2L12 17l-5.6 3 1.2-6.2L3 9.5l6.3-.8z"),
        Ik::Pic => {
            fill_svg(hdc, x, y, u, col, bg, 0.28, "M3 5h18v14H3z");
            fill_svg(hdc, x, y, u, col, bg, 1.0, "m5 17 5-6 3.2 4 2.3-2.6L19 17z");
            ell_svg(hdc, x, y, u, col, bg, 1.0, 8.5, 9.0, 1.6);
        }
        Ik::File => {
            fill_svg(hdc, x, y, u, col, bg, 0.35, "M6 3h8l4 4v14H6z");
            stroke_svg(hdc, x, y, u, col, 1.6, "M14 3v4h4");
        }
        // 工具栏沿用最初 reg/refine 稿的 1.7px 线条语言；树、卡片、分类仍用上面的图形。
        Ik::Up => {
            stroke_svg(hdc, x, y, u, col, 1.7, "M12 5.1 6.5 10.9h3.15v5.6h4.7v-5.6H17.5z");
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.45), 1.7, "M5.8 19.7h12.4");
        }
        Ik::Prev => {
            stroke_svg(hdc, x, y, u, col, 1.9, "M15.2 5 8.2 12l7 7");
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.5), 1.9, "M5.9 5.6v12.8");
        }
        Ik::Next => {
            stroke_svg(hdc, x, y, u, col, 1.9, "M8.8 5l7 7-7 7");
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.5), 1.9, "M18.1 5.6v12.8");
        }
        Ik::Find => {
            ell_stroke_svg(hdc, x, y, u, col, 1.7, 10.4, 10.4, 6.6);
            stroke_svg(hdc, x, y, u, col, 2.1, "M15.3 15.3 19.4 19.4");
        }
        Ik::Get => {
            fill_svg(hdc, x, y, u, col, bg, 0.9, "M10.9 3.6h2.2v6.6h3.1L12 15.5 7.8 10.2h3.1z");
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.25), 1.7,
                "M4.6 14.4v3.1c0 1 .8 1.8 1.8 1.8h11.2c1 0 1.8-.8 1.8-1.8v-3.1");
        }
        Ik::Print => {
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.4), 1.7, "M6.6 3.7h10.8v5.1H6.6z");
            fill_svg(hdc, x, y, u, col, bg, 0.9, "M3.2 8.8c0-.9.7-1.6 1.6-1.6h14.4c.9 0 1.6.7 1.6 1.6v5.7h-3.6v-1.9H6.8v1.9H3.2z");
            fill_svg(hdc, x, y, u, col, bg, 0.55, "M6.8 14.6h10.4v5.7H6.8z");
            fill_svg(hdc, x, y, u, col, bg, 0.35, "M17.2 10.9h1.7v1.7h-1.7z");
        }
        Ik::Edit => {
            stroke_svg(hdc, x, y, u, col, 1.7, "M4.6 19.4h3.3l10-10-3.3-3.3-10 10z");
            fill_svg(hdc, x, y, u, col, bg, 0.85,
                "m15.6 4.1 4.3 4.3 1.3-1.3c.45-.45.45-1.18 0-1.63l-2.67-2.67c-.45-.45-1.18-.45-1.63 0z");
        }
        Ik::Move => {
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.3), 1.7,
                "M2.7 7.3c0-1 .8-1.8 1.8-1.8h3.7l1.6 2h9.8c1 0 1.8.8 1.8 1.8v8.1c0 1-.8 1.8-1.8 1.8H4.5c-1 0-1.8-.8-1.8-1.8z");
            fill_svg(hdc, x, y, u, col, bg, 0.95,
                "M11.2 12.4h4.5v-2.3l4 3.5-4 3.5v-2.3h-4.5z");
        }
        Ik::Copy => {
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.5), 1.7,
                "M4.2 6.4c0-1 .8-1.8 1.8-1.8h8v2.2H6v8.2H4.2z");
            fill_svg(hdc, x, y, u, col, bg, 0.18,
                "M9 8.6h10.4c.9 0 1.6.7 1.6 1.6v10.4c0 .9-.7 1.6-1.6 1.6H9c-.9 0-1.6-.7-1.6-1.6V10.2c0-.9.7-1.6 1.6-1.6z");
            stroke_svg(hdc, x, y, u, col, 1.7,
                "M9 8.6h10.4c.9 0 1.6.7 1.6 1.6v10.4c0 .9-.7 1.6-1.6 1.6H9c-.9 0-1.6-.7-1.6-1.6V10.2c0-.9.7-1.6 1.6-1.6z");
        }
        Ik::Del => {
            stroke_svg(hdc, x, y, u, col, 1.7,
                "M9.4 4.8V3.4c0-.55.45-1 1-1h3.2c.55 0 1 .45 1 1v1.4");
            stroke_svg(hdc, x, y, u, col, 1.7,
                "M5.4 7.4h13.2l-.95 12.1c-.07.9-.82 1.6-1.72 1.6H8.07c-.9 0-1.65-.7-1.72-1.6z");
            fill_svg(hdc, x, y, u, col, bg, 0.85, "M3.4 4.8h17.2v2.3H3.4z");
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.5), 1.5, "M9.9 10.6v8M14.1 10.6v8");
        }
        Ik::Prop => {
            ell_stroke_svg(hdc, x, y, u, col, 1.7, 12.0, 12.0, 8.7);
            stroke_svg(hdc, x, y, u, col, 1.9, "M12 11.1v5.6");
            ell_svg(hdc, x, y, u, col, bg, 1.0, 12.0, 7.85, 1.2);
        }
        Ik::Opt => {
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.45), 1.7, "M3.4 8.3h17.2M3.4 15.7h17.2");
            ell_svg(hdc, x, y, u, col, bg, 1.0, 15.4, 8.3, 2.6);
            ell_svg(hdc, x, y, u, col, bg, 0.7, 8.6, 15.7, 2.6);
        }
        Ik::Skin => {
            fill_svg(hdc, x, y, u, col, bg, 0.42, "M3.2 3.4h7.4v7.4H3.2zM13.4 3.4h7.4v7.4h-7.4zM3.2 13.2h7.4v7.4H3.2z");
            fill_svg(hdc, x, y, u, col, bg, 0.95, "M13.4 13.2h7.4v7.4h-7.4z");
        }
        Ik::View => {
            fill_svg(hdc, x, y, u, col, bg, 0.45, "M2 12s4-6 10-6 10 6 10 6-4 6-10 6S2 12 2 12z");
            ell_svg(hdc, x, y, u, col, bg, 1.0, 12.0, 12.0, 3.0);
        }
        Ik::Net => {
            ell_stroke_svg(hdc, x, y, u, col, 1.8, 12.0, 12.0, 9.0);
            stroke_svg(hdc, x, y, u, col, 1.5, "M3 12h18");
            stroke_svg(hdc, x, y, u, col, 1.5, "M12 3c3 3 3 15 0 18");
            stroke_svg(hdc, x, y, u, col, 1.5, "M12 3c-3 3-3 15 0 18");
        }
        Ik::Clock => {
            ell_svg(hdc, x, y, u, col, bg, 0.3, 12.0, 12.0, 9.0);
            stroke_svg(hdc, x, y, u, col, 2.0, "M12 7v5l3.5 2");
        }
        Ik::VThb => fill_svg(hdc, x, y, u, col, bg, 1.0,
            "M3.6 3.6h7.6v7.6H3.6zM12.8 3.6h7.6v7.6h-7.6zM3.6 12.8h7.6v7.6H3.6zM12.8 12.8h7.6v7.6h-7.6z"),
        // 图标档：四格各画一枚"图片图标"（方框里一座山），跟格子里那枚通用图标对得上；
        // 上面那键四格实心，画的是真缩略图。原来这枚是 3×3 小方点，那是"小缩略图"的说法。
        Ik::VSm => {
            stroke_svg(hdc, x, y, u, col, 1.6,
                "M3.4 3.4h7.2v7.2H3.4zM12.6 3.4h7.2v7.2h-7.2zM3.4 12.6h7.2v7.2H3.4zM12.6 12.6h7.2v7.2h-7.2z");
            fill_svg(hdc, x, y, u, col, bg, 0.9,
                "M4.4 9.6 6.0 7.0 7.0 8.4 7.8 7.2 9.6 9.6zM13.6 9.6 15.2 7.0 16.2 8.4 17.0 7.2 18.8 9.6z\
                 M4.4 18.8 6.0 16.2 7.0 17.6 7.8 16.4 9.6 18.8zM13.6 18.8 15.2 16.2 16.2 17.6 17.0 16.4 18.8 18.8z");
        }
        Ik::VList => {
            for cy in [6.4, 12.0, 17.6] {
                ell_svg(hdc, x, y, u, col, bg, 1.0, 4.6, cy, 1.6);
            }
            stroke_svg(hdc, x, y, u, col, 1.9, "M9.4 6.4h10.8M9.4 12h10.8M9.4 17.6h10.8");
        }
        Ik::VDetail => {
            stroke_svg(hdc, x, y, u, mix(col, bg, 0.45), 1.7, "M3.6 4.2h16.8");
            for (cy, al) in [(9.2, 1.0), (14.0, 0.6), (18.8, 0.35)] {
                ell_svg(hdc, x, y, u, col, bg, al, 4.9, cy, 1.4);
            }
            stroke_svg(hdc, x, y, u, col, 1.8, "M9 9.2h11M9 14h8M9 18.8h11");
        }
    }
}

unsafe fn vline(hdc: HDC, x: i32, y0: i32, y1: i32, c: u32) {
    let r = RECT { left: x, top: y0, right: x + 1, bottom: y1.max(y0 + 1) };
    fill(hdc, &r, c);
}

// 贴图专用的常驻内存 DC。现建现销一个 DC 实测 8~10 微秒，一屏卡片要贴两百多次，
// 光建销就吃掉两毫秒半 —— 比省下来的插值还贵。这里只放自家缓存的位图，
// 选来选去碰不到别人的对象；要删位图前先把那张 1x1 垫图选回去，
// 正被人选中的位图 DeleteObject 是删不掉的。
thread_local! {
    static SCRATCH: std::cell::RefCell<Option<(HDC, HBITMAP)>> =
        const { std::cell::RefCell::new(None) };
}

unsafe fn scratch(hdc: HDC) -> Option<HDC> {
    if let Some(m) = SCRATCH.with(|c| c.borrow().map(|x| x.0)) {
        return Some(m);
    }
    let mem = CreateCompatibleDC(Some(hdc));
    if mem.is_invalid() {
        return None;
    }
    let dummy = CreateCompatibleBitmap(hdc, 1, 1);
    if dummy.is_invalid() {
        let _ = DeleteDC(mem);
        return None;
    }
    SelectObject(mem, HGDIOBJ(dummy.0));
    SCRATCH.with(|c| *c.borrow_mut() = Some((mem, dummy)));
    Some(mem)
}

/// 先把缓存位图从常驻 DC 上摘下来，再删。
unsafe fn scratch_drop() {
    SCRATCH.with(|c| {
        if let Some((mem, dummy)) = *c.borrow() {
            let _ = SelectObject(mem, HGDIOBJ(dummy.0));
        }
    });
}

/// 位图 1:1 贴到 hdc 上。贴不动就返回 false，让调用方照原路直接画。
unsafe fn blit_cached(hdc: HDC, x: i32, y: i32, w: i32, h: i32, bmp: HBITMAP) -> bool {
    let Some(mem) = scratch(hdc) else { return false };
    SelectObject(mem, HGDIOBJ(bmp.0));
    BitBlt(hdc, x, y, w, h, Some(mem), 0, 0, SRCCOPY).is_ok()
}

/// 尺寸行底下那块的纯色；NO_BK 表示底下不是纯色（极简档的字坐在柔光地面上），那种不存。
const NO_BK: u32 = u32::MAX;

/// 一屏七十二行尺寸字，来回滚动要重画几十遍，一行排字四十微秒就是三毫秒。
/// 同一串字、同一档字体、同一档字色、同一块纯色底，画出来逐像素一样，
/// 所以第一次画进一张小位图存着，之后同键直接贴过去 —— 省的全是重复排字。
/// 名字那行不在这儿：它坐在圆角裁剪里、顶上还压一道线，整块搬得走 AlphaBlend，
/// 先把这一行干净的搬掉。
#[derive(PartialEq, Eq, Hash)]
struct DimKey {
    s: String,
    c: u32,
    bk: u32,
    w: i32,
    h: i32,
    pad: i32,
}

thread_local! {
    static DIM_TILES: std::cell::RefCell<std::collections::HashMap<DimKey, HBITMAP>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// 字体、皮肤、缩放一变，存着的字块就不是原来那张了，整表丢掉重建。
unsafe fn dim_tiles_reset() {
    scratch_drop();
    DIM_TILES.with(|c| {
        let mut m = c.borrow_mut();
        for (_, b) in m.drain() {
            let _ = DeleteObject(HGDIOBJ(b.0));
        }
    });
    BAND_TILES.with(|c| {
        let mut m = c.borrow_mut();
        for (_, b) in m.drain() {
            let _ = DeleteObject(HGDIOBJ(b.0));
        }
    });
}

/// 照着主画面那条路画一张字块：先铺底色，再透明底排字。离屏和主画面都是内存 DC，
/// 抗锯齿走的是同一条路，贴回去一个像素都不差。
unsafe fn build_dim_tile(
    hdc: HDC,
    w: i32,
    h: i32,
    s: &str,
    c: u32,
    font: HFONT,
    bk: u32,
    pad: i32,
) -> Option<HBITMAP> {
    let built = CreateCompatibleBitmap(hdc, w, h);
    if built.is_invalid() {
        return None;
    }
    let mem = CreateCompatibleDC(Some(hdc));
    if mem.is_invalid() {
        let _ = DeleteObject(HGDIOBJ(built.0));
        return None;
    }
    let old = SelectObject(mem, HGDIOBJ(built.0));
    let local = RECT { left: 0, top: 0, right: w, bottom: h };
    let _ = FillRect(mem, &local, brush_cached(bk));
    // 排字矩形比位图左右各宽 pad：字仍按整行宽度居中、按整行宽度出省略号，
    // 只是落到缝和卡片描边那几列的笔画不存 —— 那几列主画面不归这一行管，
    // 贴回去会把格子缝和描边盖掉。
    let tr = RECT { left: -pad, top: 0, right: w + pad, bottom: h };
    txt_raw(mem, &tr, s, c, font, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    SelectObject(mem, old);
    let _ = DeleteDC(mem);
    Some(built)
}

unsafe fn dim_text(hdc: HDC, r: &RECT, s: &str, c: u32, font: HFONT, bk: u32, pad: i32) {
    if s.is_empty() {
        return;
    }
    let t = SUB_DEEP.load(Ordering::Relaxed).then(Instant::now);
    let extra = DT_CENTER | DT_SINGLELINE | DT_VCENTER;
    let w = r.right - r.left;
    let h = r.bottom - r.top;
    if bk == NO_BK || w <= 2 * pad || h <= 0 {
        txt_raw(hdc, r, s, c, font, extra);
        deep_add(TXT_SLOT.load(Ordering::Relaxed), t);
        return;
    }
    let bw = w - 2 * pad;
    let key = DimKey { s: s.to_string(), c, bk, w: bw, h, pad };
    let hit = DIM_TILES.with(|m| m.borrow().get(&key).copied());
    let bmp = match hit {
        Some(b) => b,
        None => {
            let Some(built) = build_dim_tile(hdc, bw, h, s, c, font, bk, pad) else {
                txt_raw(hdc, r, s, c, font, extra);
                deep_add(TXT_SLOT.load(Ordering::Relaxed), t);
                return;
            };
            DIM_TILES.with(|m| {
                let mut tb = m.borrow_mut();
                // 尺寸行的种类就那么几十种，涨到这个数说明皮肤在被人来回换，
                // 整表丢掉比留着占 GDI 配额划算。
                if tb.len() >= 1024 {
                    scratch_drop();
                    for (_, b) in tb.drain() {
                        let _ = DeleteObject(HGDIOBJ(b.0));
                    }
                }
                tb.insert(key, built);
            });
            built
        }
    };
    if !blit_cached(hdc, r.left + pad, r.top, bw, h, bmp) {
        txt_raw(hdc, r, s, c, font, extra);
    }
    deep_add(TXT_SLOT.load(Ordering::Relaxed), t);
}

/// 名字带整块（底 + 顶线 + 下划线 + 字）也存成小位图，跟尺寸行同一套路。
/// 左右各让开两列给卡片环：环在名字带之后画，正好盖住那两列，贴回去不碰它。
/// 底角在圆角档会出界，贴的时候套同一把圆角尺裁掉。
struct Band {
    r: RECT,
    s: String,
    c: u32,
    font: HFONT,
    bold: bool,
    bk: u32,
    line: u32,
    uline: u32,
    uw: i32,
    uh: i32,
    q: i32,
    card: RECT,
    rad: i32,
}

#[derive(PartialEq, Eq, Hash)]
struct BandKey {
    s: String,
    c: u32,
    bk: u32,
    line: u32,
    uline: u32,
    uw: i32,
    uh: i32,
    q: i32,
    w: i32,
    h: i32,
    bold: bool,
}

thread_local! {
    static BAND_TILES: std::cell::RefCell<std::collections::HashMap<BandKey, HBITMAP>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// 位图比名字带窄四像素（让开的那两列不存），所以排字矩形左右各补回两像素，
/// 居中位置和省略号宽度才跟整行现排时一个样。
unsafe fn build_band_tile(hdc: HDC, w: i32, h: i32, b: &Band) -> Option<HBITMAP> {
    let built = CreateCompatibleBitmap(hdc, w, h);
    if built.is_invalid() {
        return None;
    }
    let mem = CreateCompatibleDC(Some(hdc));
    if mem.is_invalid() {
        let _ = DeleteObject(HGDIOBJ(built.0));
        return None;
    }
    let old = SelectObject(mem, HGDIOBJ(built.0));
    let local = RECT { left: 0, top: 0, right: w, bottom: h };
    let _ = FillRect(mem, &local, brush_cached(b.bk));
    if b.line != NO_BK {
        hline(mem, 0, w, 0, b.line);
    }
    if b.uline != NO_BK {
        let ux = w / 2;
        fill(mem, &RECT { left: ux - b.uw / 2, top: h - b.uh - 1, right: ux + b.uw / 2, bottom: h - 1 }, b.uline);
    }
    let tr = RECT { left: b.q - 2, top: 0, right: w - b.q + 2, bottom: h };
    txt_raw(mem, &tr, &b.s, b.c, b.font, DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
    SelectObject(mem, old);
    let _ = DeleteDC(mem);
    Some(built)
}

/// 位图建不出来或贴不上时的兜底：就地画一遍，跟主流程原来那条路一样。
unsafe fn paint_band_now(hdc: HDC, b: &Band) {
    let keep = round_clip(hdc, &b.card, b.rad);
    let inner = RECT { left: b.r.left + 2, top: b.r.top, right: b.r.right - 2, bottom: b.r.bottom };
    fill(hdc, &inner, b.bk);
    if b.line != NO_BK {
        hline(hdc, inner.left, inner.right, b.r.top, b.line);
    }
    if b.uline != NO_BK {
        let ux = (b.r.left + b.r.right) / 2;
        fill(
            hdc,
            &RECT {
                left: ux - b.uw / 2,
                top: b.r.bottom - b.uh - 1,
                right: ux + b.uw / 2,
                bottom: b.r.bottom - 1,
            },
            b.uline,
        );
    }
    let tr = RECT { left: b.r.left + b.q, top: b.r.top, right: b.r.right - b.q, bottom: b.r.bottom };
    txt_raw(hdc, &tr, &b.s, b.c, b.font, DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
    unround(hdc, keep);
}

unsafe fn band_paint(hdc: HDC, b: &Band) {
    let t = SUB_DEEP.load(Ordering::Relaxed).then(Instant::now);
    let w = b.r.right - b.r.left;
    let h = b.r.bottom - b.r.top;
    let w4 = w - 4;
    if w4 <= 0 || h <= 0 {
        paint_band_now(hdc, b);
        deep_add(TXT_SLOT.load(Ordering::Relaxed), t);
        return;
    }
    let key = BandKey {
        s: b.s.clone(),
        c: b.c,
        bk: b.bk,
        line: b.line,
        uline: b.uline,
        uw: b.uw,
        uh: b.uh,
        q: b.q,
        w: w4,
        h,
        bold: b.bold,
    };
    let hit = BAND_TILES.with(|m| m.borrow().get(&key).copied());
    let bmp = match hit {
        Some(x) => x,
        None => {
            let Some(built) = build_band_tile(hdc, w4, h, b) else {
                paint_band_now(hdc, b);
                deep_add(TXT_SLOT.load(Ordering::Relaxed), t);
                return;
            };
            BAND_TILES.with(|m| {
                let mut tb = m.borrow_mut();
                if tb.len() >= 1024 {
                    scratch_drop();
                    for (_, x) in tb.drain() {
                        let _ = DeleteObject(HGDIOBJ(x.0));
                    }
                }
                tb.insert(key, built);
            });
            built
        }
    };
    let keep = round_clip(hdc, &b.card, b.rad);
    let ok = blit_cached(hdc, b.r.left + 2, b.r.top, w4, h, bmp);
    unround(hdc, keep);
    if !ok {
        paint_band_now(hdc, b);
    }
    deep_add(TXT_SLOT.load(Ordering::Relaxed), t);
}

/// 装饰档判据。三档共用一套配色，绘制代码只问这两句，不在每个函数里各 match 一遍。
/// 判据本身在 theme::Style 上，编辑器那扇问的是同一句，两边不会走出两个答案。
fn bands(s: &Theme) -> bool {
    s.style != Style::Plain
}

fn cards(s: &Theme) -> bool {
    s.style.is_cards()
}

/// A 静谧玻璃：低对比、薄雾、发丝线。
fn quiet(s: &Theme) -> bool {
    s.style.is_quiet()
}

/// B 制图台：结构线、分组框；标记一律用配色自带的主色，版式不夹带固定颜色。
fn draft(s: &Theme) -> bool {
    s.style.is_draft()
}

/// C 精密仪器：现有细作卡片，胶囊和圆环。
fn fine(s: &Theme) -> bool {
    s.style.is_fine()
}

/// 老版完整卡片只用于兼容旧存盘，菜单里不再出现。
fn rich(s: &Theme) -> bool {
    s.style == Style::CardsRich
}

/// 卡片圆角：只有精密仪器用圆角；静谧玻璃和制图台是方的。
fn card_rad(s: &Theme) -> i32 {
    if fine(s) {
        (7.0 * s.scale).round() as i32
    } else {
        (1.0 * s.scale).round() as i32
    }
}

/// 工具栏分组：相邻动作收进一个框。Find/Edit 共用一组。
const TB_GROUPS: &[(usize, usize)] = &[(0, 3), (3, 5), (5, 6), (6, 9), (9, 10)];

/// 外壳条带（标题 / 菜单 / 工具 / 路径 / 状态）共用一道工艺，只改颜色和边线，不动高度：
/// 底色铺平，工具栏那条亮一档，其余按传入的 base；边线统一用 line_soft。
unsafe fn shell(hdc: HDC, g: &RECT, s: &Theme, base: u32, lit: bool, border_top: bool) {
    fill(hdc, g, if lit { mix(base, s.skin.chrome_hi, 0.10) } else { base });
    let bc = if bands(s) { s.skin.line_soft } else { s.skin.line };
    if border_top {
        hline(hdc, g.left, g.right, g.top, bc);
    } else {
        hline(hdc, g.left, g.right, g.bottom - 1, bc);
    }
}

/// 内容区地面：整块纯色，不再铺对角柔光。
unsafe fn ground(hdc: HDC, r: &RECT, s: &Theme) {
    fill(hdc, r, s.skin.canvas);
}

unsafe fn ground_cached(hdc: HDC, r: &RECT, s: &Theme) {
    ground(hdc, r, s);
}

/// 把一块矩形裁进内容区后铺地面色。
unsafe fn ground_fill(hdc: HDC, g: &RECT, r: &RECT, c: u32) {
    let (l, t) = (r.left.max(g.left), r.top.max(g.top));
    let (rt, b) = (r.right.min(g.right), r.bottom.min(g.bottom));
    if rt > l && b > t {
        fill(hdc, &RECT { left: l, top: t, right: rt, bottom: b }, c);
    }
}

/// 条带帧的地面。整屏卡片是从上一帧搬过来的，卡片本身钉在内容上、搬得对；
/// 只有卡片缝里那一层地面钉在窗口上，跟着搬就挪了位。所以只补缝里那几条，
/// 地面是一整块纯色，直接按缝的位置铺回去。
unsafe fn ground_gaps(hdc: HDC, a: &App, g: &RECT, s: &Theme) -> bool {
    if !cards(s) {
        return false;
    }
    let c = s.skin.canvas;
    let (cw, ch) = a.cell();
    let pitch = a.row_h().max(1);
    let cols = a.cols_n(g.right - g.left).max(1) as i32;
    let x0 = a.grid_x0(*g);
    let edge = (3.0 * s.scale).round() as i32;
    ground_fill(hdc, g, &RECT { left: g.left, top: g.top, right: x0 + edge, bottom: g.bottom }, c);
    for ccol in 1..cols {
        let x = x0 + ccol * cw;
        ground_fill(hdc, g, &RECT { left: x - edge, top: g.top, right: x + edge, bottom: g.bottom }, c);
    }
    ground_fill(
        hdc,
        g,
        &RECT { left: x0 + cols * cw - edge, top: g.top, right: g.right, bottom: g.bottom },
        c,
    );
    // 行与行之间那一条缝（第一行顶上垫的那几像素也从这里一起补上，所以从上一行算起）
    let top = (a.scroll / pitch).max(0);
    let lines = ((g.bottom - g.top) / pitch + 3).max(1);
    for ri in top.saturating_sub(1)..(top + lines) {
        let dy = ri as i64 * pitch as i64 - a.scroll as i64;
        let y = g.top + a.grid_pad() + dy as i32;
        ground_fill(hdc, g, &RECT { left: g.left, top: y + ch, right: g.right, bottom: y + pitch }, c);
    }
    if top == 0 {
        // 第一行顶上垫的那几像素：列表刚滚开时它露着地面，上面那个循环从“上一行”起，
        // 头一行没有上一行，这一块得单独补。
        let y0 = g.top + a.grid_pad() - a.scroll;
        ground_fill(hdc, g, &RECT { left: g.left, top: g.top, right: g.right, bottom: y0 }, c);
    }
    true
}

/// 圆角裁剪。GDI 的裁剪区没有栈：先把当前那份取出来存着，画完再放回去，
/// 不然这一个格子画完，后面整片都画不出来。
unsafe fn round_clip(hdc: HDC, r: &RECT, rad: i32) -> Option<HRGN> {
    let t = SUB_DEEP.load(Ordering::Relaxed).then(Instant::now);
    let keep = CreateRectRgn(0, 0, 0, 0);
    let had = GetClipRgn(hdc, keep) == 1;
    let rgn = CreateRoundRectRgn(r.left, r.top, r.right, r.bottom, rad * 2, rad * 2);
    let _ = ExtSelectClipRgn(hdc, Some(rgn), RGN_AND);
    let _ = DeleteObject(HGDIOBJ(rgn.0));
    let out = if had { Some(keep) } else {
        let _ = DeleteObject(HGDIOBJ(keep.0));
        None
    };
    deep_add(2, t);
    out
}

unsafe fn unround(hdc: HDC, keep: Option<HRGN>) {
    let t = SUB_DEEP.load(Ordering::Relaxed).then(Instant::now);
    match keep {
        Some(r) => {
            let _ = SelectClipRgn(hdc, Some(r));
            let _ = DeleteObject(HGDIOBJ(r.0));
        }
        None => {
            let _ = SelectClipRgn(hdc, None);
        }
    }
    deep_add(2, t);
}

/// 圆角描边一圈，inner 是紧贴里面的第二圈。GDI 描不了半透明，
/// 第二圈的颜色由调用方按底下那层换算好（等价于 CSS 的 26%/30% alpha）。
unsafe fn rring(hdc: HDC, r: &RECT, rad: i32, c: u32, inner: Option<u32>) {
    rring_w(hdc, r, rad, c, 1, inner);
}

/// 圆角框。w 是笔宽：GDI 的笔压在路径正中，要让它整条落在 r 里面，
/// 传进来的 r 得先往里缩 w / 2。
unsafe fn rring_w(hdc: HDC, r: &RECT, rad: i32, c: u32, w: i32, inner: Option<u32>) {
    let t = SUB_DEEP.load(Ordering::Relaxed).then(Instant::now);
    let pen = CreatePen(PS_SOLID, w, cref(c));
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let ob = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    let _ = RoundRect(hdc, r.left, r.top, r.right, r.bottom, rad * 2, rad * 2);
    if let Some(ic) = inner {
        let p2 = CreatePen(PS_SOLID, 1, cref(ic));
        SelectObject(hdc, HGDIOBJ(p2.0));
        let d = (rad * 2 - 2).max(0);
        let _ = RoundRect(hdc, r.left + 1, r.top + 1, r.right - 1, r.bottom - 1, d, d);
        let _ = DeleteObject(HGDIOBJ(p2.0));
    }
    SelectObject(hdc, ob);
    SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(pen.0));
    deep_add(4, t);
}

/// 一个 ✕：两条对角线。字形 ✕ 换字体就换粗细，画出来才和 1px 的边线一个体系。
unsafe fn xmark(hdc: HDC, cx: i32, cy: i32, d: i32, col: u32) {
    let pen = CreatePen(PS_SOLID, 1, cref(col));
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let ob = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    let a = [POINT { x: cx - d, y: cy - d }, POINT { x: cx + d, y: cy + d }];
    let b = [POINT { x: cx + d, y: cy - d }, POINT { x: cx - d, y: cy + d }];
    let _ = Polyline(hdc, &a);
    let _ = Polyline(hdc, &b);
    SelectObject(hdc, ob);
    SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(pen.0));
}

/// 实心三角：树上的开合标记、工具栏和路径栏的下拉箭头。
/// 原来用 ▸ ▾ 两个字形，字号一变就和对齐线错开。
unsafe fn tri(hdc: HDC, cx: i32, cy: i32, d: i32, col: u32, down: bool) {
    let v = if down {
        [POINT { x: cx - d, y: cy - d / 2 }, POINT { x: cx + d, y: cy - d / 2 }, POINT { x: cx, y: cy + d / 2 }]
    } else {
        [POINT { x: cx - d / 2, y: cy - d }, POINT { x: cx - d / 2, y: cy + d }, POINT { x: cx + d / 2, y: cy }]
    };
    let (ob, op, br) = ibrush(hdc, col);
    let _ = Polygon(hdc, &v);
    irestore(hdc, ob, op, br);
}

/// 菜单里那个勾：8×5 的两笔，代替字形 ✓。
unsafe fn tick(hdc: HDC, cx: i32, cy: i32, col: u32, k: f64) {
    stroke_svg(hdc, cx - (4.5 * k) as i32, cy - (3.2 * k) as i32, k, col, 1.6, "M0 3.4 3.2 6.4 9 0.6");
}

/// 标签后面那个下拉箭头。极简档还是字形 ▾，平阶往上换成同体系的实心三角，
/// 颜色跟着按钮文字走（原来是死写 ink_dim，悬停时和文字脱开）。
unsafe fn caret(hdc: HDC, r: &RECT, s: &Theme, col: u32, font: HFONT) {
    if bands(s) {
        tri(hdc, (r.left + r.right) / 2, (r.top + r.bottom) / 2, (3.4 * s.scale) as i32, col, true);
    } else {
        txt(hdc, r, "▾", s.skin.ink_dim, font, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    }
}

/// 窗口钮的字形：最小化一道横、最大化一个框、关闭一个叉。
unsafe fn wbtn_glyph(hdc: HDC, id: usize, r: &RECT, col: u32, k: f64) {
    let (cx, cy) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
    match id {
        M_MIN => hline(hdc, cx - (5.0 * k) as i32, cx + (5.0 * k) as i32, cy, col),
        M_MAX => {
            frame(
                hdc,
                &RECT {
                    left: cx - (4.0 * k) as i32,
                    top: cy - (4.0 * k) as i32,
                    right: cx + (4.0 * k) as i32,
                    bottom: cy + (3.0 * k) as i32,
                },
                col,
            );
        }
        _ => xmark(hdc, cx, cy, (5.0 * k) as i32, col),
    }
}

/// 弹层的落地影：由内往外几层淡纱叠出来。原来是一整块偏移的硬影子，
/// 在深蓝上像贴了一张黑纸。只在弹层开着的时候画，一帧几层不心疼。
unsafe fn soft_shadow(hdc: HDC, r: &RECT, down: i32) {
    for i in 0..6 {
        let d = i * 2;
        let q = RECT { left: r.left - d, top: r.top + down - d, right: r.right + d, bottom: r.bottom + down + d };
        veil(hdc, &q, 0x000000, (26 - i * 4).max(4) as u8);
    }
}

/// 自绘标题栏：左边图标 + 标题，右边最小化/最大化/关闭。
/// 某一块单独设了字色时，换一份 Theme 给它：主字直接换成挑的颜色，
/// 次文/灰显按原来跟正文的通道差一起平移 —— 只换一块的主色，附带的两种小字
/// 仍跟它同族，不会又退回全局那一档。
fn zone_theme(base: &Theme, look: &theme::Look, i: usize) -> Theme {
    let Some(c) = look.zone_color(theme::ZONES[i].0) else {
        return base.clone();
    };
    let mut t = base.clone();
    t.skin.ink = c;
    t.skin.ink_dim = shift_text(base.skin.ink_dim, base.skin.ink, c);
    t.skin.ink_off = shift_text(base.skin.ink_off, base.skin.ink, c);
    t
}

/// 目录树和分类这两页的"选中"整块改走标题栏那一档主色：选中底就是 acc，
/// 字色按对比度在正文色/反白里挑一支，跟标题栏同一判据。
/// 槽位在进这两页之前一次换算好，画家各自的选中分支不用重写。
fn sel_as_bar(base: &Theme) -> Theme {
    let mut t = base.clone();
    let bg = t.skin.acc;
    t.skin.sel = bg;
    t.skin.sel_ink = if theme::contrast(t.skin.ink, bg) >= theme::contrast(t.skin.ink_inv, bg) {
        t.skin.ink
    } else {
        t.skin.ink_inv
    };
    t
}

fn shift_text(text: u32, old_ink: u32, new_ink: u32) -> u32 {
    let ch = |a: u32, b: u32, c: u32| (a as i32 + c as i32 - b as i32).clamp(0, 255) as u32;
    (ch((text >> 16) & 0xff, (old_ink >> 16) & 0xff, (new_ink >> 16) & 0xff) << 16)
        | (ch((text >> 8) & 0xff, (old_ink >> 8) & 0xff, (new_ink >> 8) & 0xff) << 8)
        | ch(text & 0xff, old_ink & 0xff, new_ink & 0xff)
}

/// 这一块现在正文用的到底是哪个色：单独设过就跟着那一档笔一起挪，没设过就是本皮肤这一档笔。
/// 字体页的色块和取色窗的初始色都从这一处算，界面上不再各猜各的。
fn zone_now_ink(i: usize, dk: &theme::Skin, over: Option<u32>) -> u32 {
    let cur = dk.slot(theme::ZONE_INK[i]);
    match over {
        Some(c) => shift_text(cur, dk.ink, c),
        None => cur,
    }
}

unsafe fn paint_title(hdc: HDC, a: &mut App, s: &Theme, rc: &RECT) {
    a.zone(theme::Z_MENU);
    let k = s.scale;
    // 取色、底缘那条线、按钮几何全部出自 theme::Strip，四扇窗共用一套。
    let st = theme::Strip::top(s);
    let g = RECT { left: rc.left, top: rc.top, right: rc.right, bottom: rc.top + st.h };
    fill(hdc, &g, st.bar);
    hline(hdc, g.left, g.right, g.bottom - 1, st.edge);
    if st.ruler {
        // 制图台：标题栏左下压一道短尺，和窗格头的主色索引线同一个记号。
        // 底子本身就是主色了，这道尺改用文字色压暗一档，否则和底色糊在一起。
        fill(
            hdc,
            &RECT {
                left: g.left + (8.0 * k) as i32,
                top: g.bottom - 3,
                right: g.left + (72.0 * k) as i32,
                bottom: g.bottom - 1,
            },
            st.dim,
        );
    }
    // 角上这个图标跟着标题栏高度走：原来写死 14 逻辑像素，缩放到 125% 才 17 高，
    // 比标题那行字还矮一截，看着像没放上去。上下各留 3 像素，其余给它。
    let isz = (g.bottom - g.top - (6.0 * k).round() as i32).max((12.0 * k) as i32);
    let mx = g.left + (8.0 * k).round() as i32;
    // 角上画的是 exe 里那份真图标（取景框那套图稿），不再用代码里凑的图形。
    // 密度或缩放变了尺寸才会变，变了才重取一份句柄，取到就还旧的。
    if a.tb_icon.0 != isz {
        if !a.tb_icon.1.0.is_null() {
            let _ = DestroyIcon(a.tb_icon.1);
        }
        a.tb_icon = (isz, xsee_host::appicon::load_sized(isz));
    }
    if !a.tb_icon.1.0.is_null() {
        let _ = DrawIconEx(
            hdc,
            mx,
            g.top + (g.bottom - g.top - isz) / 2,
            a.tb_icon.1,
            isz,
            isz,
            0,
            None,
            DI_NORMAL,
        );
    }
    let by = g.top + (g.bottom - g.top - st.bh) / 2;
    let mut x = g.right - st.pad;
    let mut rects = Vec::new();
    for (id, gl) in [(M_CLOSE, "✕"), (M_MAX, "□"), (M_MIN, "─")] {
        let r = RECT { left: x - st.bw, top: by, right: x, bottom: by + st.bh };
        let hot = over(&r, a.pt.0, a.pt.1);
        let lay1 = if id == M_CLOSE { st.hot } else { st.hov };
        if fine(s) {
            if hot {
                let hr = RECT { left: r.left + 2, top: r.top + 2, right: r.right - 2, bottom: r.bottom - 2 };
                let rad = (4.0 * k).round() as i32;
                let keep = round_clip(hdc, &hr, rad);
                fill(hdc, &hr, lay1);
                unround(hdc, keep);
                rring(hdc, &hr, rad, theme::mix(st.bar, st.fg, 0.45), None);
            }
            let col = if hot { st.fg } else { st.dim };
            wbtn_glyph(hdc, id, &r, col, k);
        } else if quiet(s) {
            // 静谧玻璃：按钮不描边，悬停只给一层很淡的雾。
            if hot {
                fill(hdc, &r, lay1);
            }
            let col = if hot { st.fg } else { st.dim };
            wbtn_glyph(hdc, id, &r, col, k);
        } else if bands(s) {
            if hot {
                fill(hdc, &r, lay1);
            }
            if draft(s) && hot && id != M_CLOSE {
                fill(
                    hdc,
                    &RECT { left: r.left + 4, top: r.bottom - 2, right: r.right - 4, bottom: r.bottom },
                    st.dim,
                );
            }
            let col = if hot { st.fg } else { st.dim };
            wbtn_glyph(hdc, id, &r, col, k);
        } else {
            if hot {
                fill(hdc, &r, lay1);
                frame(hdc, &r, st.ring);
            }
            txt(
                hdc,
                &r,
                gl,
                if hot { st.fg } else { st.dim },
                a.font_sm,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
        }
        rects.push((id, r));
        x = r.left - st.gap;
    }
    a.btns.extend(rects);
    let title = xsee_host::viewer::is_open()
        .then(xsee_host::viewer::title)
        .unwrap_or_else(|| format!("Xsee · 浏览 — {}", a.label()));
    let tr = RECT {
        left: mx + isz + (6.0 * k) as i32,
        top: g.top,
        right: x - (8.0 * k) as i32,
        bottom: g.bottom,
    };
    // 标题这行字不加粗，颜色按标题栏底色现挑（accent 有深浅，见 theme::Strip）。
    txt(
        hdc,
        &tr,
        &title,
        st.ink,
        a.font,
        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
    );
    // 画布那一扇的标题条跟着这扇走：换了皮肤、缩放了，它下一次推面就是新的一套。
    xsee_host::refwin::set_strip(
        st,
        xsee_host::refwin::Face {
            bg: s.skin.chrome_hi,
            line: s.skin.line,
            ink: s.skin.ink,
            dim: s.skin.ink_dim,
            hov: s.skin.hov,
            popup: s.skin.popup,
            off: s.skin.ink_off,
            sep: s.skin.line_soft,
            menu_row: theme::popup_row(s),
        },
    );
}

/// 顶部菜单条。它是“读字的一行”，亮度让给标题栏和工具栏：比上下都暗一档。
unsafe fn paint_menu_bar(hdc: HDC, a: &mut App, s: &Theme, rc: &RECT) {
    a.zone(theme::Z_MENU);
    let g = RECT {
        left: rc.left,
        top: rc.top + s.title,
        right: rc.right,
        bottom: rc.top + s.title + s.menu,
    };
    let base = s.skin.menu;
    shell(hdc, &g, s, base, false, false);
    let pad = (8.0 * s.scale).round() as i32;
    let mut x = g.left + (2.0 * s.scale).round() as i32;
    a.menus.clear();
    for (i, (label, _)) in MENUS.iter().enumerate() {
        let w = text_w(hdc, label, a.font) + pad * 2;
        let r = RECT { left: x, top: g.top + 1, right: x + w, bottom: g.bottom - 1 };
        let open = a.open_menu == Some(i);
        let hot = open || over(&r, a.pt.0, a.pt.1);
        let ink;
        if fine(s) {
            if hot {
                // 精修菜单的悬停只是一层薄雾，不铺一块实底色
                fill(
                    hdc,
                    &RECT { left: r.left + 2, top: r.top + 2, right: r.right - 2, bottom: r.bottom - 2 },
                    mix(base, s.skin.hov, 0.55),
                );
            }
            if open {
                // 打开态只留一条 1px accent，和卡片选中线同粗细
                fill(
                    hdc,
                    &RECT { left: r.left + 4, top: r.bottom - 1, right: r.right - 4, bottom: r.bottom },
                    s.skin.acc,
                );
            }
            ink = if hot { s.skin.ink } else { s.skin.ink_dim };
        } else if quiet(s) {
            if hot {
                fill(
                    hdc,
                    &RECT { left: r.left + 2, top: r.top + 2, right: r.right - 2, bottom: r.bottom - 2 },
                    mix(base, s.skin.hov, 0.32),
                );
            }
            if open {
                fill(
                    hdc,
                    &RECT { left: r.left + 4, top: r.bottom - 1, right: r.right - 4, bottom: r.bottom },
                    s.skin.line_soft,
                );
            }
            ink = if hot { s.skin.ink } else { s.skin.ink_dim };
        } else if bands(s) {
            if hot {
                fill(hdc, &r, s.skin.hov);
            }
            if draft(s) && hot {
                fill(
                    hdc,
                    &RECT { left: r.left + 3, top: r.bottom - 2, right: r.right - 3, bottom: r.bottom },
                    s.skin.acc,
                );
            }
            if open {
                // 打开态：只在底缘压一条 accent 轨，不整块反白
                fill(
                    hdc,
                    &RECT { left: r.left, top: r.bottom - 2, right: r.right, bottom: r.bottom },
                    s.skin.acc,
                );
            }
            ink = if hot { s.skin.ink } else { s.skin.ink_dim };
        } else {
            if hot {
                fill(hdc, &r, if open { s.skin.sel } else { s.skin.hov });
                frame(hdc, &r, s.skin.acc);
            }
            ink = if open { s.skin.sel_ink } else { s.skin.ink };
        }
        txt(hdc, &r, label, ink, a.font, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
        a.menus.push((i, r));
        x = r.right;
    }
}

/// 弹层的尺寸全部读 `theme::MENU_*` 那一组令牌，画布那扇右键菜单读的是同一处。
fn popup_metrics(_a: &App, s: &Theme, items: &[MenuItem]) -> (i32, i32, i32) {
    let k = s.scale;
    let mut lw = 0;
    let mut kw = 0;
    for it in items {
        if it.sep {
            continue;
        }
        lw = lw.max(it.label.chars().count() as i32 * (s.font / 2 + 2));
        if !it.key.is_empty() {
            kw = kw.max(it.key.chars().count() as i32 * (s.font_sm / 2 + 1));
        }
    }
    let w = theme::menu_width(k, lw, kw);
    let mut h = 0;
    for it in items {
        h += if it.sep {
            (theme::MENU_SEP_H * k) as i32
        } else {
            theme::popup_row(s)
        };
    }
    (w, h + (theme::MENU_INSET_Y * k) as i32, kw)
}

unsafe fn popup_layout(
    hdc: HDC,
    a: &App,
    s: &Theme,
    items: &[MenuItem],
    ax: i32,
    ay: i32,
    // 子层右侧放不下时翻到左边，贴的是父层的左缘；顶层菜单没有父层，传 None。
    flip_left: Option<i32>,
    rc: &RECT,
) -> (RECT, Vec<(usize, RECT)>) {
    let k = s.scale;
    let (_, h, _) = popup_metrics(a, s, items);
    // 精确宽度用实际字体量一遍，等宽估值只当兜底
    let mut lw = 0;
    let mut kw = 0;
    for it in items {
        if it.sep {
            continue;
        }
        lw = lw.max(text_w(hdc, it.label, a.font));
        if !it.key.is_empty() {
            kw = kw.max(text_w(hdc, it.key, a.font_sm));
        }
    }
    let w = theme::menu_width(k, lw, kw);
    let mut x = ax;
    let mut y = ay;
    if x + w > rc.right - 2 {
        // 子层往左夹会把整块面板压到父层上面，重叠区就成了两层抢一个鼠标，
        // 所以右侧放不下时翻到父层左边；左边也放不下才退回夹在窗口内。
        x = match flip_left {
            Some(edge) => (edge - w).max(rc.left),
            None => (rc.right - w - 2).max(rc.left),
        };
    }
    if y + h > rc.bottom - 2 {
        y = (ay - h).max(rc.top);
    }
    let rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
    let mut rows = Vec::new();
    let mut cy = rect.top + (theme::MENU_INSET * k) as i32;
    for (i, it) in items.iter().enumerate() {
        let rh = if it.sep {
            (theme::MENU_SEP_H * k) as i32
        } else {
            theme::popup_row(s)
        };
        rows.push((i, RECT { left: rect.left, top: cy, right: rect.right, bottom: cy + rh }));
        cy += rh;
    }
    (rect, rows)
}

unsafe fn paint_popup(
    hdc: HDC,
    a: &App,
    s: &Theme,
    items: &[MenuItem],
    rect: &RECT,
    rows: &[(usize, RECT)],
    keep: Option<usize>,
) {
    let k = s.scale;
    // 跟对话框同一套投法：同尺寸灰纱往右下让 6 像素，底下格子透得出来。
    drop_shadow(hdc, rect, k);
    fill(hdc, rect, s.skin.popup);
    if fine(s) {
        hline(
            hdc,
            rect.left + 1,
            rect.right - 1,
            rect.top + 1,
            mix(s.skin.popup, s.skin.line_soft, 0.55),
        );
    }
    frame(hdc, rect, s.skin.acc);
    let pad = (theme::MENU_SEP_PAD * k) as i32;
    for (idx, r) in rows {
        let it = &items[*idx];
        if it.sep {
            hline(hdc, r.left + pad, r.right - pad, (r.top + r.bottom) / 2, s.skin.line_soft);
            continue;
        }
        let hot = keep == Some(*idx) || over(r, a.pt.0, a.pt.1);
        let checked = match it.id {
            M_VGRID => a.mode == Mode::Grid,
            M_VSMALL => a.mode == Mode::Small,
            M_VLIST => a.mode == Mode::List,
            M_VDETAIL => a.mode == Mode::Detail,
            M_SORT_NAME => a.sort == SortKey::Name,
            M_SORT_EXT => a.sort == SortKey::Ext,
            M_SORT_SIZE => a.sort == SortKey::Size,
            M_SORT_TYPE => a.sort == SortKey::Type,
            M_SORT_DATE => a.sort == SortKey::Date,
            M_SORT_IMAGE => a.sort == SortKey::Image,
            M_SORT_ADDED => a.sort == SortKey::Added,
            M_SORT_DESC => a.sort == SortKey::Desc,
            M_SORT_FWD => !a.sort_desc,
            M_SORT_REV => a.sort_desc,
            M_ORG => a.org_open,
            M_CONV_PANEL => a.conv_open,
            M_PANE_TREE => a.pane_tree,
            M_PANE_PV => a.pane_pv,
            M_TB_TEXT => a.tb_text,
            M_TB_NOTEXT => !a.tb_text,
            M_TB_RIGHT => a.tb_text && !tb_below(a, s),
            M_TB_BELOW => a.tb_text && tb_below(a, s),
            M_TB_HIDE => !a.bar_main,
            M_BAR_PATH => a.bar_path,
            M_BAR_STATUS => a.bar_status,
            M_FILT_DIRS => a.filt_dirs,
            M_FILT_NONIMG => a.filt_nonimg,
            M_FILT_CAT => a.filt_cat,
            _ => it.check,
        };
        // 没复制过类别时“粘贴”是灰的；判断放在这里，菜单和点击用同一口径
        let off = it.off
            || (it.id == M_CAT_PASTE && cat_clip_empty())
            || tree_ctx_off(a, it.id)
            || grid_ctx_off(a, it.id);
        // 悬停底色直接用标题栏那一档（Strip::top 的 bar=skin.acc），文字跟着
        // 换标题栏的字色：菜单里指到哪一行，跟窗口头是同一套配色。
        let tst = theme::Strip::top(s);
        if hot {
            fill(hdc, r, tst.bar);
        }
        let ink = if off {
            if hot { tst.dim } else { s.skin.ink_off }
        } else if hot {
            tst.fg
        } else {
            s.skin.ink
        };
        let ck = RECT {
            left: r.left + (MENU_IC_X * k) as i32,
            top: r.top,
            right: r.left + ((MENU_IC_X + MENU_IC_W) * k) as i32,
            bottom: r.bottom,
        };
        if checked {
            // 勾在图标列里：悬停时底下换了标题栏色，勾跟着换标题栏字色才看得见。
            let cc = if hot {
                tst.fg
            } else if fine(s) || !bands(s) {
                s.skin.acc
            } else {
                ink
            };
            if bands(s) {
                tick(hdc, (ck.left + ck.right) / 2, (ck.top + ck.bottom) / 2, cc, k);
            } else {
                txt(hdc, &ck, "✓", cc, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
            }
        }
        let g = menuicons::find(it.id).or_else(|| menuicons::find_label(it.label));
        // 勾和图标共用左边那一列：勾上的行画勾，图标让位。
        if let Some(g) = g.filter(|_| !checked) {
            let isz = (MENU_IC_W * k).round() as i32;
            let ibg = if hot { tst.bar } else { s.skin.popup };
            draw_glyph(
                hdc,
                g,
                r.left + (MENU_IC_X * k) as i32,
                (r.top + r.bottom - isz) / 2,
                isz,
                ink,
                ibg,
            );
        }
        let lr = RECT {
            left: r.left + theme::menu_text_x(k),
            top: r.top,
            right: r.right - (theme::MENU_RIGHT * k) as i32,
            bottom: r.bottom,
        };
        txt(hdc, &lr, it.label, ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
        if !it.key.is_empty() {
            let kr = RECT {
                left: r.right - (theme::MENU_RIGHT * k) as i32 - (theme::MENU_KEY_MAX * k) as i32,
                top: r.top,
                right: r.right - (theme::MENU_RIGHT * k) as i32,
                bottom: r.bottom,
            };
            let kc = if hot {
                tst.dim
            } else if fine(s) {
                s.skin.ink_off
            } else if bands(s) {
                s.skin.ink_off
            } else {
                s.skin.ink_dim
            };
            txt(hdc, &kr, it.key, kc, a.font_sm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
        }
        if it.sub.is_some() {
            let ar = RECT { left: r.right - (14.0 * k) as i32, top: r.top, right: r.right - (2.0 * k) as i32, bottom: r.bottom };
            if fine(s) {
                tri(
                    hdc,
                    (ar.left + ar.right) / 2,
                    (ar.top + ar.bottom) / 2,
                    (3.2 * k) as i32,
                    if hot { tst.fg } else { s.skin.ink_off },
                    false,
                );
            } else if bands(s) {
                tri(hdc, (ar.left + ar.right) / 2, (ar.top + ar.bottom) / 2, (3.2 * k) as i32, if hot { tst.fg } else { s.skin.ink_off }, false);
            } else {
                txt(hdc, &ar, "▸", if hot { tst.fg } else { s.skin.ink_dim }, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
            }
        }
    }
}

unsafe fn paint_open_menu(hdc: HDC, a: &mut App, s: &Theme, rc: &RECT) {
    a.zone(theme::Z_POPUP);
    a.pop_rows.clear();
    a.sub_rows.clear();
    let Some(which) = a.open_menu else {
        // 关掉的这一帧顺手清悬停缓存：不清的话下次打开菜单，指针第一次移进
        // 上次停过的那一行会被当成"没换行"跳过重绘，看着就是移过去不亮、
        // 挪开再回来才亮。
        a.pop_hover = None;
        a.sub_hover = None;
        return;
    };
    let Some((_, items)) = MENUS.get(which) else {
        return;
    };
    let Some(anchor) = a.menus.iter().find(|(i, _)| *i == which).map(|(_, r)| *r) else {
        return;
    };
    let (rect, rows) = popup_layout(hdc, a, s, items, anchor.left, anchor.bottom, None, rc);
    a.pop_rect = rect;
    a.pop_rows = rows.clone();
    paint_popup(hdc, a, s, items, &rect, &rows, a.open_sub);
    let Some(si) = a.open_sub else {
        return;
    };
    let Some(sub) = items.get(si).and_then(|it| it.sub) else {
        return;
    };
    let Some(prow) = rows.iter().find(|(i, _)| *i == si).map(|(_, r)| *r) else {
        return;
    };
    let (srect, srows) = popup_layout(
        hdc,
        a,
        s,
        sub,
        prow.right - (2.0 * s.scale) as i32,
        prow.top - (3.0 * s.scale) as i32,
        Some(rect.left),
        rc,
    );
    a.sub_rect = srect;
    a.sub_rows = srows.clone();
    paint_popup(hdc, a, s, sub, &srect, &srows, None);
}

unsafe fn paint_ctx_menu(hdc: HDC, a: &mut App, s: &Theme, rc: &RECT) {
    a.zone(theme::Z_POPUP);
    a.ctx_rows.clear();
    a.ctx_sub_rows.clear();
    a.ctx_cat_rows.clear();
    if let Some((ax, ay)) = a.ctx_cat {
        let (rect, rows) = popup_layout(hdc, a, s, CAT_CTX, ax, ay, None, rc);
        a.ctx_cat_rect = rect;
        a.ctx_cat_rows = rows.clone();
        paint_popup(hdc, a, s, CAT_CTX, &rect, &rows, None);
        return;
    }
    let Some((ax, ay)) = a.ctx_at else {
        return;
    };
    let items = if a.ctx_tree { TREE_CTX } else { CTX_MENU };
    let (rect, rows) = popup_layout(hdc, a, s, items, ax, ay, None, rc);
    a.ctx_rect = rect;
    a.ctx_rows = rows.clone();
    paint_popup(hdc, a, s, items, &rect, &rows, a.ctx_sub);
    let Some(si) = a.ctx_sub else {
        return;
    };
    let Some(sub) = items.get(si).and_then(|it| it.sub) else {
        return;
    };
    let Some(prow) = rows.iter().find(|(i, _)| *i == si).map(|(_, r)| *r) else {
        return;
    };
    let (srect, srows) = popup_layout(
        hdc,
        a,
        s,
        sub,
        prow.right - (2.0 * s.scale) as i32,
        prow.top - (3.0 * s.scale) as i32,
        Some(rect.left),
        rc,
    );
    a.ctx_sub_rect = srect;
    a.ctx_sub_rows = srows.clone();
    paint_popup(hdc, a, s, sub, &srect, &srows, None);
}

/// 工具栏文字摆在图标下边还是右边：人的选择优先，可矮工具栏放不下两行，只能并排。
/// 这条判据只写一遍，画按钮和画菜单勾都用它，两边不会各说一套。
fn tb_below(a: &App, s: &Theme) -> bool {
    a.tb_below && s.bar > (30.0 * s.scale).round() as i32
}

unsafe fn paint_bar(hdc: HDC, a: &mut App, s: &Theme) {
    a.zone(theme::Z_BAR);
    let g = a.bar;
    if g.bottom - g.top < 8 {
        return;
    }
    if bands(s) {
        shell(hdc, &g, s, s.skin.chrome, true, false);
    } else {
        fill(hdc, &g, s.skin.chrome);
    }
    let below = tb_below(a, s);
    let lab_h = s.font_sm + 2;
    let isz = s
        .icon
        .min((g.bottom - g.top - 6 - if below { lab_h + 2 } else { 0 }).max(12));
    let p = (5.0 * s.scale).round() as i32;
    let car_w = |b: &BtnDef| if b.caret { (s.font_sm as f64 * 0.8).round() as i32 } else { 0 };
    let btn_w = |b: &BtnDef| {
        let tw = text_w(hdc, b.label, a.font_sm) + if b.caret { s.font_sm } else { 0 };
        if !a.tb_text {
            isz + p * 2 + car_w(b)
        } else if below {
            isz.max(tw) + p * 2
        } else {
            // 左内边距 + 图标 + 图标与文字的缝 + 文字（带下拉箭头的把箭头那份算进 tw）
            // + 右内边距。缝不给够就每个标签都要被省略号切掉一个字。
            isz + tw + p * 2 + 3
        }
    };
    let mut x = g.left + (4.0 * s.scale).round() as i32;
    for (bi, b) in BTNS.iter().enumerate() {
        // 制图台和精密仪器用分组框；组内不再画竖分隔线。
        let grouped = draft(s) || fine(s);
        if b.sep && !grouped {
            let y0 = g.top + (4.0 * s.scale) as i32;
            let y1 = g.bottom - (4.0 * s.scale) as i32;
            let m = (3.0 * s.scale).round() as i32;
            vline(hdc, x + m, y0, y1, s.skin.line_soft);
            if fine(s) {
                vline(hdc, x + m + 1, y0, y1, mix(s.skin.chrome_hi, s.skin.canvas, 0.22));
            } else if !bands(s) {
                // 极简档保留那道 1px 亮边（假的浮雕）；平阶往上一条线就够
                vline(hdc, x + m + 1, y0, y1, s.skin.chrome_hi);
            }
            x += m * 2 + 2;
        }
        if grouped {
            if let Some((_, end)) = TB_GROUPS.iter().find(|(start, _)| *start == bi) {
                let pad = (2.0 * s.scale).round() as i32;
                let w: i32 = (bi..*end).map(|j| btn_w(&BTNS[j])).sum::<i32>()
                    + (*end - bi - 1) as i32;
                let gr = RECT {
                    left: x - pad,
                    top: g.top + 3,
                    right: x + w + pad,
                    bottom: g.bottom - 3,
                };
                let rad = if fine(s) {
                    (8.0 * s.scale).round() as i32
                } else if draft(s) {
                    0
                } else {
                    (5.0 * s.scale).round() as i32
                };
                // 组框只留一圈框线，不铺底板：工具栏底色从头到尾都是 chrome 原色。
                rring(
                    hdc,
                    &gr,
                    rad,
                    if draft(s) {
                        s.skin.line
                    } else {
                        mix(s.skin.line_soft, s.skin.canvas, 0.28)
                    },
                    None,
                );
                if draft(s) {
                    hline(hdc, gr.left + 2, gr.right - 2, gr.top + 1, mix(s.skin.chrome_hi, s.skin.canvas, 0.48));
                    // 组框左上角压一道主色索引线：和窗格头、标题栏那把短尺是同一个记号，
                    // 不悬停也看得出这一档走的是图纸那套。
                    let iw = (((gr.right - gr.left) as f64) * 0.26).round().max(8.0) as i32;
                    hline(hdc, gr.left, (gr.left + iw).min(gr.right), gr.top, s.skin.acc);
                }
            }
        }
        let tw = text_w(hdc, b.label, a.font_sm) + if b.caret { s.font_sm } else { 0 };
        let w = btn_w(b);
        let r = RECT { left: x, top: g.top + 2, right: x + w, bottom: g.bottom - 2 };
        let hot = !b.off && over(&r, a.pt.0, a.pt.1);
        let bgc = if fine(s) || draft(s) || quiet(s) {
            if !b.off && hot {
                if draft(s) {
                    mix(s.skin.chrome_hi, s.skin.hov, 0.72)
                } else if quiet(s) {
                    mix(s.skin.chrome, s.skin.hov, 0.42)
                } else {
                    mix(s.skin.chrome, s.skin.hov, 0.55)
                }
            } else {
                s.skin.chrome
            }
        } else if !b.off && hot {
            s.skin.hov
        } else {
            s.skin.chrome
        };
        let ink;
        let iic;
        if bands(s) {
            ink = if b.off {
                mix(bgc, s.skin.ink_off, 0.82)
            } else if hot {
                s.skin.ink
            } else {
                s.skin.ink_dim
            };
            // 图标比文字再收一档：一笔一笔的线条和字重不是一个量级，同色会显得图标更粗
            iic = if draft(s) && hot && !b.off {
                mix(s.skin.acc, s.skin.ink, 0.20)
            } else if b.off || hot {
                ink
            } else {
                mix(ink, bgc, 0.08)
            };
        } else {
            ink = if b.off { s.skin.ink_off } else { s.skin.ink };
            iic = ink;
        }
        if draft(s) {
            if hot && !b.off {
                // 制图台：方角框 + 极淡底 + 底缘一道主色索引线，像在图纸上圈出一格。
                fill(hdc, &r, mix(s.skin.chrome, s.skin.hov, 0.34));
                frame(hdc, &r, s.skin.acc);
                fill(
                    hdc,
                    &RECT { left: r.left + 3, top: r.bottom - 2, right: r.right - 3, bottom: r.bottom },
                    s.skin.acc,
                );
            }
        } else if fine(s) {
            if hot && !b.off {
                let rad = (6.0 * s.scale).round() as i32;
                let keep = round_clip(hdc, &r, rad);
                fill(hdc, &r, bgc);
                unround(hdc, keep);
                // 两像素的框：往里缩一像素再描，整条线才落在按钮格内不压组框。
                let rr = RECT {
                    left: r.left + 1,
                    top: r.top + 1,
                    right: r.right - 1,
                    bottom: r.bottom - 1,
                };
                rring_w(hdc, &rr, rad, mix(s.skin.line_soft, s.skin.acc, 0.34), 2, None);
            }
        } else if quiet(s) {
            if hot && !b.off {
                let rad = (4.0 * s.scale).round() as i32;
                let keep = round_clip(hdc, &r, rad);
                fill(hdc, &r, bgc);
                unround(hdc, keep);
                rring(hdc, &r, rad, mix(s.skin.line_soft, s.skin.acc, 0.22), None);
            }
        } else {
            fill(hdc, &r, bgc);
            if hot && !bands(s) {
                frame(hdc, &r, s.skin.line_soft);
            }
        }
        if !a.tb_text {
            icon(hdc, b.icon, r.left + p, (r.top + r.bottom - isz) / 2, isz, iic, bgc);
            if b.caret {
                let cr = RECT { left: r.left + p + isz, top: r.top, right: r.right, bottom: r.bottom };
                caret(hdc, &cr, s, ink, a.font_sm);
            }
        } else if below {
            // 图标顶上、文字贴底，空档两边各让 3/8，中间剩的间隙是全空档
            // 的 1/4；整组在按钮里保持居中。
            let free = (r.bottom - r.top - lab_h - isz).max(0);
            let d = free / 8 * 3;
            icon(hdc, b.icon, r.left + (r.right - r.left - isz) / 2, r.top + d, isz, iic, bgc);
            let mut lr = RECT {
                left: r.left + 1,
                top: r.bottom - lab_h - d - (2.0 * s.scale).round() as i32,
                right: r.right - 1,
                bottom: r.bottom - d - (2.0 * s.scale).round() as i32,
            };
            if b.caret {
                let cw = text_w(hdc, b.label, a.font_sm);
                let cx = r.left + (r.right - r.left - (cw + s.font_sm)) / 2;
                let nr = RECT { left: cx + cw, top: lr.top, right: lr.right, bottom: lr.bottom };
                caret(hdc, &nr, s, ink, a.font_sm);
                lr.right = (cx + cw).min(lr.right);
            }
            txt(hdc, &lr, b.label, ink, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
        } else {
            icon(hdc, b.icon, r.left + p, (r.top + r.bottom - isz) / 2, isz, iic, bgc);
            let mut lr = RECT {
                left: r.left + p + isz + 3,
                top: r.top,
                right: r.right - p,
                bottom: r.bottom,
            };
            if b.caret {
                let tw0 = tw - s.font_sm;
                lr.right = (lr.left + tw0).min(r.right - p);
                let cr = RECT { left: lr.right, top: r.top, right: lr.right + s.font_sm, bottom: r.bottom };
                caret(hdc, &cr, s, ink, a.font_sm);
            }
            txt(hdc, &lr, b.label, ink, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
        }
        if draft(s) {
            // 组内一格一隔：相邻两颗按钮之间一道 1px 细线，组框里排成一格一格。
            let in_grp = TB_GROUPS.iter().find(|(st, en)| bi >= *st && bi < *en);
            let last_in_grp = in_grp.map(|(_, en)| bi + 1 >= *en).unwrap_or(true);
            if !last_in_grp {
                let y0 = r.top + (3.0 * s.scale) as i32;
                let y1 = r.bottom - (3.0 * s.scale) as i32;
                vline(hdc, r.right, y0, y1, mix(s.skin.chrome, s.skin.line_soft, 0.62));
            }
        }
        if !b.off {
            a.btns.push((b.id, r));
        }
        x = r.right + 1;
    }
    if !bands(s) {
        hline(hdc, g.left, g.right, g.bottom - 1, s.skin.line);
    }
    if draft(s) {
        // 图框双线：工具栏底缘之上再压一道细线，和标题栏那道对上。
        hline(hdc, g.left, g.right, g.bottom - 4, mix(s.skin.chrome, s.skin.line_soft, 0.50));
    }
}

/// 路径栏：地址框吃满中间，右边依次是五个视图字符钮和缩略图缩放。
/// 地址框带 1px 内阴影，路径用等宽字体。
unsafe fn paint_path(hdc: HDC, a: &mut App, s: &Theme) {
    a.zone(theme::Z_PATH);
    let g = a.path_bar;
    if g.right - g.left < 8 || g.bottom - g.top < 8 {
        // 这一帧没画地址框，命中表和选区一起清掉，别让上一帧的矩形接着吃鼠标
        a.addr_tr = RECT::default();
        a.addr_sel = None;
        a.addr_anchor = None;
        return;
    }
    if bands(s) {
        shell(hdc, &g, s, s.skin.win, false, false);
    } else {
        fill(hdc, &g, s.skin.win);
    }
    let bh = (s.path as f64 - 5.0 * s.scale).round().max(12.0) as i32;
    let bt = g.top + (g.bottom - g.top - bh) / 2;
    let cy = (g.top + g.bottom) / 2;
    let k = s.scale;
    let right0 = g.right - (5.0 * k).round() as i32;

    // 稿子 .vb 是 22×17 的钮坐一颗 15 的图，17 = d-path-5，也就是地址框实高。
    // 这边框比 17 高，钮宽和图标就得按框高同系数放大，不然框越高那几个看着越缩。
    let kf = bh as f64 / (17.0 * k);
    let vbn = (22.0 * k * kf).round() as i32;
    let vis = (15.0 * k * kf).round() as i32;
    let vg_w = 4 * vbn + 3 * (2.0 * k).round() as i32;
    let label = "缩略图";
    let lw = text_w(hdc, label, a.font_sm);
    // 一对 −/+ 跟视图组同一档钮宽，中间夹一条只读的滑条当刻度指示
    let zbn = vbn;
    let zg = (4.0 * k) as i32;
    let tw = (39.0 * k).round() as i32;
    let dim = format!("{} px", a.thumb_h);
    let nw = text_w(hdc, &dim, a.font_num);
    let zoom_w = lw + zg + zbn + zg + tw + zg + zbn + zg + nw;
    let zx = right0 - zoom_w;
    let vx0 = (zx - (4.0 * k) as i32 - vg_w).max(g.left);
    let ar = RECT {
        left: g.left + (5.0 * k).round() as i32,
        top: bt,
        right: (vx0 - (5.0 * k) as i32).max(g.left + (5.0 * k) as i32 + 40),
        bottom: bt + bh,
    };

    if fine(s) {
        fill(hdc, &ar, mix(s.skin.canvas, s.skin.pane, 0.38));
        frame(hdc, &ar, s.skin.line_soft);
        hline(
            hdc,
            ar.left + 1,
            ar.right - 1,
            ar.top + 1,
            mix(s.skin.canvas, s.skin.line_soft, 0.52),
        );
    } else if quiet(s) {
        // 静谧玻璃：地址框和玻璃面同色，只留一道发丝下沿。
        fill(hdc, &ar, mix(s.skin.canvas, s.skin.pane, 0.28));
        hline(hdc, ar.left, ar.right, ar.bottom - 1, s.skin.line_soft);
    } else if draft(s) {
        // 制图台：方格纸上的标题栏，左边压一条主色索引线。
        fill(hdc, &ar, mix(s.skin.canvas, s.skin.pane, 0.24));
        frame(hdc, &ar, s.skin.line);
        fill(
            hdc,
            &RECT { left: ar.left, top: ar.top + 2, right: ar.left + 2, bottom: ar.bottom - 2 },
            s.skin.acc,
        );
    } else if bands(s) {
        fill(hdc, &ar, s.skin.canvas);
        // 内阴影那两笔是假的凹陷，在深蓝上只是脏；一条弱边框就够
        frame(hdc, &ar, s.skin.line_soft);
    } else {
        fill(hdc, &ar, s.skin.canvas);
        frame(hdc, &ar, s.skin.line);
        vline(hdc, ar.left + 1, ar.top + 1, ar.bottom - 1, s.skin.line_soft);
        hline(hdc, ar.left + 1, ar.right - 1, ar.top + 1, s.skin.line_soft);
    }
    let isz = (14.0 * k * kf).round() as i32;
    icon(hdc, Ik::Folder, ar.left + (4.0 * k) as i32, cy - isz / 2, isz, s.skin.acc, s.skin.canvas);
    let pr = RECT {
        left: ar.left + (4.0 * k) as i32 + isz + (5.0 * k) as i32,
        top: ar.top,
        right: ar.right - (14.0 * k) as i32,
        bottom: ar.bottom,
    };
    let dt = DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_PATH_ELLIPSIS;
    let lbl = a.label();
    let pw = pr.right - pr.left;
    if a.addr_src != lbl || a.addr_w != pw {
        // 逐字量左边界：Consolas 撞上中文会换字宽，只能按前缀实测，不能拿字号乘字数
        let mut xs: Vec<(usize, i32)> = vec![(0usize, pr.left)];
        let mut uoff = 0usize;
        for (bi, ch) in lbl.char_indices() {
            uoff += ch.len_utf16() as usize;
            let w = text_w(hdc, &lbl[..bi + ch.len_utf8()], a.font_path);
            xs.push((uoff, pr.left + w));
        }
        a.addr_xs = xs;
        a.addr_n = uoff;
        a.addr_w = pw;
        a.addr_full = text_w(hdc, &lbl, a.font_path) <= pw;
        a.addr_src = lbl.clone();
        // 路径或框宽变了，旧偏移指不到新串上，选区直接作废
        a.addr_sel = None;
        a.addr_anchor = None;
    }
    a.addr_tr = RECT {
        left: ar.left + 1,
        top: ar.top + 1,
        right: pr.right,
        bottom: ar.bottom - 1,
    };
    let seg = a.addr_sel_x();
    if let Some((x0, x1)) = seg {
        fill(
            hdc,
            &RECT {
                left: x0,
                top: ar.top + 1,
                right: x1,
                bottom: ar.bottom - 1,
            },
            if fine(s) { mix(s.skin.canvas, s.skin.sel, 0.55) } else { s.skin.sel },
        );
    }
    txt(hdc, &pr, &lbl, s.skin.ink, a.font_path, dt);
    if let Some((x0, x1)) = seg {
        // 同一串字再画一遍、只把裁剪夹到选中那截：省略号在哪儿都由 GDI 自己算，
        // 高亮底下不会出现"底色的洞对不上文字"的错位。
        let keep = SaveDC(hdc);
        let _ = IntersectClipRect(hdc, x0, ar.top + 1, x1, ar.bottom - 1);
        txt(hdc, &pr, &lbl, if fine(s) { s.skin.ink } else { s.skin.sel_ink }, a.font_path, dt);
        let _ = RestoreDC(hdc, keep);
    }
    let cr = RECT {
        left: ar.right - (14.0 * k) as i32,
        top: ar.top,
        right: ar.right - 2,
        bottom: ar.bottom,
    };
    caret(hdc, &cr, s, s.skin.ink_off, a.font_sm);
    a.btns.push((M_ADDR, cr));

    // 视图组：四个按钮，钮宽和图标尺寸都在上面按框高算好的 vbn / vis
    vline(hdc, vx0 - (2.0 * k) as i32, bt, bt + bh, s.skin.line_soft);
    let mut vx = vx0;
    for (ik, m, id) in [
        (Ik::VThb, Mode::Grid, M_VGRID),
        (Ik::VSm, Mode::Small, M_VSMALL),
        (Ik::VList, Mode::List, M_VLIST),
        (Ik::VDetail, Mode::Detail, M_VDETAIL),
    ] {
        let on = a.mode == m;
        let r = RECT { left: vx, top: bt, right: vx + vbn, bottom: bt + bh };
        let hot = over(&r, a.pt.0, a.pt.1);
        let bgc = if draft(s) {
            if on {
                mix(s.skin.acc_soft, s.skin.canvas, 0.38)
            } else if hot {
                mix(s.skin.canvas, s.skin.hov, 0.55)
            } else {
                s.skin.canvas
            }
        } else if quiet(s) {
            if on {
                mix(s.skin.win, s.skin.hov, 0.40)
            } else if hot {
                mix(s.skin.win, s.skin.hov, 0.26)
            } else {
                s.skin.win
            }
        } else if on {
            s.skin.acc_soft
        } else if hot {
            s.skin.hov
        } else {
            s.skin.win
        };
        if fine(s) {
            let rad = (4.0 * k).round() as i32;
            let keep = round_clip(hdc, &r, rad);
            fill(hdc, &r, bgc);
            unround(hdc, keep);
            if on {
                rring(hdc, &r, rad, s.skin.acc, None);
            }
        } else {
            fill(hdc, &r, bgc);
            if on && !bands(s) {
                frame(hdc, &r, s.skin.acc);
            } else if draft(s) && on {
                fill(
                    hdc,
                    &RECT { left: r.left + 2, top: r.bottom - 2, right: r.right - 2, bottom: r.bottom },
                    s.skin.acc,
                );
            } else if draft(s) && hot {
                frame(hdc, &r, s.skin.line);
            }
        }
        let col = if on {
            if draft(s) { s.skin.ink } else if bands(s) { s.skin.acc } else { s.skin.ink }
        } else if hot {
            s.skin.ink
        } else if bands(s) {
            s.skin.ink_off
        } else {
            s.skin.ink_dim
        };
        icon(hdc, ik, (r.left + r.right - vis) / 2, (r.top + r.bottom - vis) / 2, vis, col, bgc);
        a.btns.push((id, r));
        vx = r.right + (2.0 * k).round() as i32;
    }

    let lr = RECT { left: zx, top: g.top, right: zx + lw, bottom: g.bottom };
    txt(
        hdc,
        &lr,
        label,
        if bands(s) { s.skin.ink_off } else { s.skin.ink_dim },
        a.font_sm,
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );
    let tx = zx + lw + zg;
    let at_min = a.thumb_h <= THUMB_H_MIN;
    let at_max = a.thumb_h >= THUMB_H_MAX;
    let pt = a.pt;
    let (gy0, gy1) = (g.top, g.bottom);
    let btn_r = |x: i32| RECT { left: x, top: bt, right: x + zbn, bottom: bt + bh };
    zbtn(hdc, btn_r(tx), "−", at_min, pt, a, s, M_ZOOM_DN);
    // 滑条只当刻度用：拖不动，位置跟着 thumb_h 走，所以按 −/+ 时它会自己挪。
    let th = (6.0 * k).round().max(4.0) as i32;
    let sx = tx + zbn + zg;
    let tr = RECT { left: sx, top: cy - th / 2, right: sx + tw, bottom: cy + th / 2 };
    let q = ((a.thumb_h - THUMB_H_MIN) as f64 / (THUMB_H_MAX - THUMB_H_MIN) as f64).clamp(0.0, 1.0);
    if bands(s) {
        // 滑槽收成 3px 一道弱线，滑块是一块 8×11 的实心把手：原来那圈边框在小尺寸下糊成一团
        let sl = (3.0 * k).round().max(2.0) as i32;
        let sr = RECT { left: tr.left, top: cy - sl / 2, right: tr.right, bottom: cy - sl / 2 + sl };
        fill(hdc, &sr, s.skin.line_soft);
        let fx = tr.left + ((tr.right - tr.left) as f64 * q).round() as i32;
        fill(hdc, &RECT { left: sr.left, top: sr.top, right: fx, bottom: sr.bottom }, s.skin.acc);
        let hw = (4.0 * k).round().max(3.0) as i32;
        let hh = (11.0 * k).round().max(7.0) as i32;
        fill(
            hdc,
            &RECT { left: fx - hw, top: cy - hh / 2, right: fx + hw, bottom: cy - hh / 2 + hh },
            s.skin.acc,
        );
    } else {
        fill(hdc, &tr, s.skin.canvas);
        frame(hdc, &tr, s.skin.line);
        let fill_r = RECT {
            left: tr.left + 1,
            top: tr.top + 1,
            right: tr.left + 1 + ((tr.right - tr.left - 2) as f64 * q).round() as i32,
            bottom: tr.bottom - 1,
        };
        fill(hdc, &fill_r, s.skin.acc);
        let hd = RECT {
            left: fill_r.right - 3,
            top: tr.top - (th / 2 + 1),
            right: fill_r.right + 3,
            bottom: tr.bottom + (th / 2 + 1),
        };
        fill(hdc, &hd, s.skin.chrome_hi);
        frame(hdc, &hd, s.skin.acc);
    }
    let px = sx + tw + zg;
    zbtn(hdc, btn_r(px), "+", at_max, pt, a, s, M_ZOOM_UP);
    let nr = RECT { left: px + zbn + zg, top: gy0, right: right0, bottom: gy1 };
    txt(hdc, &nr, &dim, s.skin.ink_dim, a.font_num, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    if !bands(s) {
        hline(hdc, g.left, g.right, g.bottom - 1, s.skin.line);
    }
}

/// 路径栏上那枚 −/+ 小方钮。到头了就画成灰的，也不登记命中区，点了没反应。
unsafe fn zbtn(
    hdc: HDC,
    r: RECT,
    glyph: &str,
    off: bool,
    pt: (i32, i32),
    a: &mut App,
    s: &Theme,
    id: usize,
) {
    let k = s.scale;
    let hot = over(&r, pt.0, pt.1) && !off;
    let bgc = if hot { s.skin.hov } else { s.skin.win };
    let edge = if hot { s.skin.acc } else { s.skin.line };
    // ± 的字色跟标题栏那一档红走（标题栏底色就是 acc），和滑条同色系；到头不能点时才转灰。
    let ink = if off { s.skin.ink_off } else { s.skin.acc };
    if fine(s) {
        let rad = (4.0 * k).round() as i32;
        let keep = round_clip(hdc, &r, rad);
        fill(hdc, &r, bgc);
        unround(hdc, keep);
        rring(hdc, &r, rad, edge, None);
    } else {
        fill(hdc, &r, bgc);
        frame(hdc, &r, edge);
    }
    txt(hdc, &r, glyph, ink, a.font_num, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    if !off {
        a.btns.push((id, r));
    }
}

unsafe fn pane_head(hdc: HDC, g: &RECT, title: &str, id: usize, a: &mut App, s: &Theme) {
    let head = RECT { left: g.left, top: g.top, right: g.right, bottom: g.top + s.pane_head_h };
    if fine(s) {
        fill(hdc, &head, s.skin.pane_head);
        hline(
            hdc,
            head.left,
            head.right,
            head.top + 1,
            mix(s.skin.pane_head, s.skin.chrome_hi, 0.38),
        );
    } else if quiet(s) {
        fill(hdc, &head, s.skin.pane_head);
        hline(hdc, head.left, head.right, head.bottom - 1, s.skin.line_soft);
    } else if draft(s) {
        fill(hdc, &head, mix(s.skin.pane_head, s.skin.chrome, 0.32));
        fill(
            hdc,
            &RECT { left: head.left, top: head.top, right: head.left + 2, bottom: head.bottom },
            s.skin.acc,
        );
        hline(hdc, head.left, head.right, head.bottom - 1, s.skin.line);
    } else {
        fill(hdc, &head, s.skin.pane_head);
    }
    let tr = RECT {
        left: head.left + if draft(s) { (10.0 * s.scale) as i32 } else { (6.0 * s.scale) as i32 },
        top: head.top,
        right: head.right - (18.0 * s.scale) as i32,
        bottom: head.bottom,
    };
    if fine(s) || quiet(s) || draft(s) {
        txt(hdc, &tr, title, s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    } else if bands(s) {
        // 窗格头是标签不是标题：字重压平、颜色退一档，把重量让给里面的内容
        txt(hdc, &tr, title, s.skin.ink_dim, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    } else {
        txt(hdc, &tr, title, s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    }
    let xr = RECT {
        left: head.right - (18.0 * s.scale) as i32,
        top: head.top,
        right: head.right - (3.0 * s.scale) as i32,
        bottom: head.bottom,
    };
    if fine(s) {
        let xh = over(&xr, a.pt.0, a.pt.1);
        if xh {
            let hr = RECT {
                left: xr.left + 2,
                top: xr.top + 3,
                right: xr.right - 2,
                bottom: xr.bottom - 3,
            };
            let rad = (4.0 * s.scale).round() as i32;
            let keep = round_clip(hdc, &hr, rad);
            fill(hdc, &hr, mix(s.skin.pane_head, s.skin.hov, 0.55));
            unround(hdc, keep);
            rring(hdc, &hr, rad, mix(s.skin.line_soft, s.skin.acc, 0.24), None);
        }
        xmark(
            hdc,
            (xr.left + xr.right) / 2,
            (xr.top + xr.bottom) / 2,
            (4.0 * s.scale) as i32,
            if xh { s.skin.acc } else { s.skin.ink_off },
        );
    } else if bands(s) {
        let xh = over(&xr, a.pt.0, a.pt.1);
        xmark(
            hdc,
            (xr.left + xr.right) / 2,
            (xr.top + xr.bottom) / 2,
            (4.0 * s.scale) as i32,
            if xh { s.skin.acc } else { s.skin.ink_off },
        );
    } else {
        txt(
            hdc,
            &xr,
            "✕",
            if over(&xr, a.pt.0, a.pt.1) { 0xc0392b } else { s.skin.ink_dim },
            a.font_sm,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
    }
    // 窗格头的 ✕ 和查看菜单那一项走同一个命令 id：一个语义一个出处
    a.btns.push((id, xr));
    hline(
        hdc,
        head.left,
        head.right,
        head.bottom - 1,
        if fine(s) { s.skin.line_soft } else { s.skin.line },
    );
}

/// 树滚动条：算出槽和滑块。行数没超出可视高度就不需要它，返回 None。
/// 返回（槽、滑块、槽宽）——槽宽还要拿去把行文字往左让开。
fn tree_sb(g: RECT, head_bot: i32, rows: usize, rh: i32, tscroll: i32, k: f64) -> Option<(RECT, RECT, i32)> {
    let h = g.bottom - head_bot;
    let total = rows as i32 * rh;
    if h < 24 || total <= h {
        return None;
    }
    let sbw = (10.0 * k).round().max(8.0) as i32;
    let track = RECT { left: g.right - sbw, top: head_bot, right: g.right, bottom: g.bottom };
    let tb = (h as f64 * h as f64 / total as f64).round().max(24.0) as i32;
    let span = (h - tb).max(1);
    let over = (total - h).max(1) as i64;
    let ty = head_bot + (tscroll as i64 * span as i64 / over).clamp(0, span as i64) as i32;
    Some((
        track,
        RECT {
            left: track.left + 2,
            top: ty,
            right: track.right - 2,
            bottom: (ty + tb).min(g.bottom),
        },
        sbw,
    ))
}

/// 详细资料档的列头那一条。高度是布局时从列表顶上扣出去的 `pane_head_h`，
/// 画和点都从这一个来源取，别在两边各写一遍。非详细档没有列头，调用方自己判 mode。
fn detail_head(z: &Zones, s: &Theme) -> RECT {
    RECT {
        left: z.grid.left,
        top: z.grid.top - s.pane_head_h,
        right: z.grid.right,
        bottom: z.grid.top,
    }
}

/// 缩略图区滚动条。槽在 z.gsb 那一竖条里，内容没超过一屏就不画滑块。
fn grid_sb(g: RECT, maxs: i32, scroll: i32, k: f64) -> Option<(RECT, RECT, i32)> {
    let h = g.bottom - g.top;
    if maxs <= 0 || h < 24 {
        return None;
    }
    let total = h + maxs;
    let tb = (h as f64 * h as f64 / total as f64).round().max(24.0) as i32;
    let span = (h - tb).max(1);
    let ty = g.top + (scroll as i64 * span as i64 / maxs as i64).clamp(0, span as i64) as i32;
    let sbw = (10.0 * k).round().max(8.0) as i32;
    Some((
        RECT { left: g.right - sbw, top: g.top, right: g.right, bottom: g.bottom },
        RECT {
            left: g.right - sbw + 2,
            top: ty,
            right: g.right - 2,
            bottom: (ty + tb).min(g.bottom),
        },
        tb,
    ))
}

/// 整理面板：只放类别这一组。类别树来自 data\catalog.tsv，
/// 计数是含子类别的去重张数。
#[derive(Clone, Copy, PartialEq)]
enum CheckState {
    Off,
    Part,
    On,
}

#[derive(Clone)]
struct OrgRow {
    id: i64,
    depth: i32,
    name: String,
    kids: bool,
    open: bool,
    count: usize,
    /// 当前选中图片在这个类别里的情况；"已分类图片"那行不适用。
    check: Option<CheckState>,
}

/// 整理面板第一行的名字。它只数、只列挂过类别的图，所以名字跟着口径走。
const CAT_ALL_ROW: &str = "已分类图片";

fn org_rows(a: &App) -> Vec<OrgRow> {
    let mut out = Vec::new();
    out.push(OrgRow {
        id: 0,
        depth: 0,
        name: CAT_ALL_ROW.to_string(),
        kids: false,
        open: false,
        // 只数分过类的：库里还有一大半图从没挂到任何类别上，把那些也算进来
        // 这一行就跟下面的类别行对不上账了。
        count: a.cat.as_ref().map(|c| c.n_categorized()).unwrap_or(0),
        check: None,
    });
    let Some(cat) = &a.cat else { return out };
    let sel = a.selected_images();
    fn push(v: &mut Vec<OrgRow>, a: &App, cat: &catdb::CatEdit, sel: &[String], id: i64, depth: i32) {
        let kids = cat.child_ids(id).to_vec();
        let open = a.org_open_ids.contains(&id);
        let check = if sel.is_empty() {
            Some(CheckState::Off)
        } else {
            let (n, total) = cat.state_sub(sel, id);
            Some(if n == 0 {
                CheckState::Off
            } else if n == total {
                CheckState::On
            } else {
                CheckState::Part
            })
        };
        v.push(OrgRow {
            id,
            depth,
            name: cat.name(id),
            kids: !kids.is_empty(),
            open,
            count: cat.count(id),
            check,
        });
        if open {
            for k in kids {
                push(v, a, cat, sel, k, depth + 1);
            }
        }
    }
    for r in &cat.roots {
        push(&mut out, a, cat, &sel, *r, 0);
    }
    out
}

/// 类别行前的勾选框：把当前选中的图片加入/移出该类别。
/// 类别行落到另一个类别上就是改父级；落到"已分类图片"那行表示回到顶层。
unsafe fn cat_drop(hwnd: HWND, cid: i64, to: i64) {
    if to < 0 || to == cid {
        return;
    }
    let (text, ok) = {
        let mut g = app().lock().unwrap();
        match g.cat.as_mut() {
            Some(cat) => {
                if !cat.set_parent(cid, to) {
                    ("不能把类别放进它自己的子类别里".to_string(), false)
                } else {
                    let name = cat.name(cid);
                    let dst = if to == 0 {
                        format!("{CAT_ALL_ROW}（顶层）")
                    } else {
                        cat.name(to)
                    };
                    let t = match cat.save_full(&catalog_file()) {
                        Ok(()) => format!("已把「{name}」放进「{dst}」"),
                        Err(e) => format!("位置已改，但保存失败：{e}"),
                    };
                    (t, true)
                }
            }
            None => ("分类数据库还没载入".to_string(), false),
        }
    };
    {
        let mut g = app().lock().unwrap();
        g.msg = text;
        if ok && to != 0 {
            g.org_open_ids.insert(to);
        }
    }
    refresh(hwnd);
}

unsafe fn toggle_cat_for_selection(hwnd: HWND, id: i64) {
    let (paths, full, cat_name) = {
        let g = app().lock().unwrap();
        let paths = g.selected_images();
        let cat = g.cat.as_ref();
        let full = !paths.is_empty()
            && cat
                .map(|c| {
                    let (n, total) = c.state_sub(&paths, id);
                    n == total
                })
                .unwrap_or(false);
        let name = cat.map(|c| c.name(id)).unwrap_or_default();
        (paths, full, name)
    };
    if paths.is_empty() {
        app().lock().unwrap().msg = "先在缩略图里选中图片".to_string();
        refresh(hwnd);
        return;
    }
    let (msg, reload) = {
        let mut g = app().lock().unwrap();
        // 父类别取消勾选会动到子树，当前看到的不一定就是被点的那一行，统一重取
        let reload = matches!(g.src, Src::Category(_));
        let res = if let Some(cat) = g.cat.as_mut() {
            cat.toggle_paths_sub(id, &paths, !full);
            Some(cat.save_full(&catalog_file()))
        } else {
            None
        };
        let msg = match res {
            Some(Ok(())) => format!(
                "{} 张图片{}类别「{}」",
                paths.len(),
                if full { "移出" } else { "加入" },
                cat_name
            ),
            Some(Err(e)) => format!("类别已改，但保存失败：{e}"),
            None => "分类数据库还没载入".to_string(),
        };
        (msg, reload)
    };
    {
        let mut g = app().lock().unwrap();
        g.msg = msg;
        if reload {
            if let Src::Category(cid) = g.src {
                g.load_category(cid, hwnd);
            }
        }
    }
    // "隐藏已分类图片"这条不当场生效：刚分完就撤掉，人就没法接着给它挂第二个类别了。
    // 什么时候撤看刷新（重新扫描、换目录），那一次会重新按过滤列一遍。
    refresh(hwnd);
}

/// 整理面板标题带（"整理"那一行）的高。比别的窗格头高一截，字底下才留得出空。
fn org_title_h(s: &Theme) -> i32 {
    s.pane_head_h + (6.0 * s.scale).round() as i32
}

/// 整理面板顶部整块占多高：标题带 + "类别"组头带。两带之间只一条横线，不留缝。
/// 画和点都取这一个数，两边不会各算一份算歪。
fn org_head_h(s: &Theme) -> i32 {
    org_title_h(s) + s.row
}

unsafe fn paint_org(hdc: HDC, a: &mut App, z: &Zones, s: &Theme) {
    a.zone(theme::Z_ORG);
    let g = z.org;
    if g.right <= g.left {
        return;
    }
    clip(hdc, &g);
    fill(hdc, &g, if draft(s) { mix(s.skin.pane, s.skin.canvas, 0.22) } else { s.skin.pane });
    vline(
        hdc,
        g.left,
        g.top,
        g.bottom,
        if fine(s) || quiet(s) || draft(s) { s.skin.line_soft } else { s.skin.line },
    );
    let head = RECT {
        left: g.left,
        top: g.top,
        right: g.right,
        bottom: g.top + org_title_h(s),
    };
    if fine(s) {
        fill(hdc, &head, s.skin.pane_head);
        hline(
            hdc,
            head.left,
            head.right,
            head.top + 1,
            mix(s.skin.pane_head, s.skin.chrome_hi, 0.38),
        );
    } else if quiet(s) {
        fill(hdc, &head, s.skin.pane_head);
    } else if draft(s) {
        fill(hdc, &head, mix(s.skin.pane_head, s.skin.chrome, 0.32));
        fill(
            hdc,
            &RECT { left: head.left, top: head.top, right: head.left + 2, bottom: head.bottom },
            s.skin.acc,
        );
    } else {
        fill(hdc, &head, s.skin.pane_head);
    }
    if a.conv_open {
        // 两页同开时标题带让给页签；✕ 关的是当前这一页
        paint_dock_tabs(hdc, a, s, &head, 0);
    } else {
    let tr = RECT {
        left: head.left + if draft(s) { (16.0 * s.scale) as i32 } else { (12.0 * s.scale) as i32 },
        top: head.top,
        right: head.right - (22.0 * s.scale) as i32,
        bottom: head.bottom,
    };
    if fine(s) || quiet(s) || draft(s) {
        txt(hdc, &tr, "图片分类", s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    } else if bands(s) {
        txt(hdc, &tr, "图片分类", s.skin.ink_dim, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    } else {
        txt(hdc, &tr, "图片分类", s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    }
    let xr = RECT {
        left: head.right - (22.0 * s.scale) as i32,
        top: head.top,
        right: head.right - (4.0 * s.scale) as i32,
        bottom: head.bottom,
    };
    if fine(s) {
        let xh = over(&xr, a.pt.0, a.pt.1);
        if xh {
            let hr = RECT {
                left: xr.left + 2,
                top: xr.top + 3,
                right: xr.right - 2,
                bottom: xr.bottom - 3,
            };
            let rad = (4.0 * s.scale).round() as i32;
            let keep = round_clip(hdc, &hr, rad);
            fill(hdc, &hr, mix(s.skin.pane_head, s.skin.hov, 0.55));
            unround(hdc, keep);
            rring(hdc, &hr, rad, mix(s.skin.line_soft, s.skin.acc, 0.24), None);
        }
        xmark(
            hdc,
            (xr.left + xr.right) / 2,
            (xr.top + xr.bottom) / 2,
            (4.0 * s.scale) as i32,
            if xh { s.skin.acc } else { s.skin.ink_off },
        );
    } else if bands(s) {
        let xh = over(&xr, a.pt.0, a.pt.1);
        if draft(s) && xh {
            fill(
                hdc,
                &RECT { left: xr.left + 2, top: xr.top + 3, right: xr.right - 2, bottom: xr.bottom - 3 },
                mix(s.skin.pane_head, s.skin.hov, 0.70),
            );
            fill(
                hdc,
                &RECT { left: xr.left + 4, top: xr.bottom - 4, right: xr.right - 4, bottom: xr.bottom - 2 },
                s.skin.acc,
            );
        }
        xmark(
            hdc,
            (xr.left + xr.right) / 2,
            (xr.top + xr.bottom) / 2,
            (4.0 * s.scale) as i32,
            if xh {
                s.skin.acc
            } else {
                s.skin.ink_off
            },
        );
    } else {
        txt(
            hdc,
            &xr,
            "✕",
            if over(&xr, a.pt.0, a.pt.1) { 0xc0392b } else { s.skin.ink_dim },
            a.font_sm,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
    }
    a.btns.push((M_ORG_CLOSE, xr));
    }
    hline(
        hdc,
        head.left,
        head.right,
        head.bottom - 1,
        if fine(s) { s.skin.line_soft } else { s.skin.line },
    );

    let grp = RECT {
        left: g.left,
        top: g.top + org_head_h(s) - s.row,
        right: g.right,
        bottom: g.top + org_head_h(s),
    };
    if fine(s) {
        fill(hdc, &grp, s.skin.pane_head);
        hline(
            hdc,
            grp.left,
            grp.right,
            grp.top + 1,
            mix(s.skin.pane_head, s.skin.chrome_hi, 0.34),
        );
    } else if quiet(s) {
        fill(hdc, &grp, s.skin.pane_head);
        hline(hdc, grp.left, grp.right, grp.top + 1, s.skin.line_soft);
    } else if draft(s) {
        fill(hdc, &grp, mix(s.skin.pane_head, s.skin.chrome, 0.38));
        fill(
            hdc,
            &RECT { left: grp.left, top: grp.top, right: grp.left + 2, bottom: grp.bottom },
            s.skin.acc,
        );
    } else {
        fill(hdc, &grp, s.skin.pane_head);
    }
    let gl = RECT {
        left: grp.left + if draft(s) { (16.0 * s.scale) as i32 } else { (12.0 * s.scale) as i32 },
        top: grp.top,
        right: grp.right - (8.0 * s.scale) as i32,
        bottom: grp.bottom,
    };
    if draft(s) {
        let d = (4.0 * s.scale).round().max(3.0) as i32;
        fill(
            hdc,
            &RECT {
                left: grp.left + (8.0 * s.scale) as i32,
                top: grp.top + (grp.bottom - grp.top - d) / 2,
                right: grp.left + (8.0 * s.scale) as i32 + d,
                bottom: grp.top + (grp.bottom - grp.top - d) / 2 + d,
            },
            s.skin.acc,
        );
    }
    txt(hdc, &gl, "类别", s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    // 组头右侧：上移、下移。"新建"已经并进类别右键菜单，这里不再摆按钮。
    // 矩形每次重画都重建，点击和看到的一致。
    {
        let bsz = (18.0 * s.scale) as i32;
        let by = grp.top + (grp.bottom - grp.top - bsz) / 2;
        let bt = |right: i32| RECT { left: right - bsz, top: by, right, bottom: by + bsz };
        let edge = (4.0 * s.scale) as i32;
        a.org_dn = bt(grp.right - edge);
        a.org_up = bt(a.org_dn.left - (2.0 * s.scale) as i32);
        for (r, label, enabled) in [
            (a.org_up, "↑", a.org_sel != 0),
            (a.org_dn, "↓", a.org_sel != 0),
        ] {
            let hot = over(&r, a.pt.0, a.pt.1);
            if hot && enabled {
                if fine(s) {
                    let hr = RECT {
                        left: r.left + 1,
                        top: r.top + 2,
                        right: r.right - 1,
                        bottom: r.bottom - 2,
                    };
                    let rad = (4.0 * s.scale).round() as i32;
                    let keep = round_clip(hdc, &hr, rad);
                    fill(hdc, &hr, mix(s.skin.pane_head, s.skin.hov, 0.55));
                    unround(hdc, keep);
                    rring(hdc, &hr, rad, mix(s.skin.line_soft, s.skin.acc, 0.22), None);
                } else {
                    fill(hdc, &r, s.skin.hov);
                }
            }
            txt(
                hdc,
                &r,
                label,
                if !enabled {
                    s.skin.ink_off
                } else if hot {
                    if fine(s) { s.skin.ink } else { s.skin.sel_ink }
                } else {
                    s.skin.ink_dim
                },
                a.font_b,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
        }
    }
    hline(hdc, grp.left, grp.right, grp.bottom - 1, s.skin.line_soft);

    let top = grp.bottom;
    // 裁剪区收到列表区自己那一格。上面 clip 的是整块面板，含"整理"标题和"类别"表头，
    // 滚动量不是整行高倍数时第一行会往上爬进表头那一行，图标和计数压在"类别"两个字上。
    clip(hdc, &RECT { left: g.left, top, right: g.right, bottom: g.bottom });
    let rh = s.row.max(1);
    let k = s.scale;
    let rows = org_rows(a);
    let h = (g.bottom - top).max(0);
    let maxs = (rows.len() as i32 * rh - h).max(0);
    a.org_scroll = a.org_scroll.max(0).min(maxs);
    let sb = tree_sb(
        RECT { left: g.left, top, right: g.right, bottom: g.bottom },
        top,
        rows.len(),
        rh,
        a.org_scroll,
        k,
    );
    let sbx = sb.as_ref().map(|(t, _, _)| t.left).unwrap_or(i32::MAX);
    let first = (a.org_scroll / rh).max(0) as usize;
    let last = ((a.org_scroll + h) / rh + 1) as usize;
    let ind = (13.0 * k).round() as i32;
    let arrow_w = (12.0 * k).round() as i32;
    a.org_hits.clear();
    a.org_checks.clear();
    for i in first..last.min(rows.len()) {
        let r = &rows[i];
        let y = top + i as i32 * rh - a.org_scroll;
        let rowr = if fine(s) {
            RECT {
                left: g.left + (4.0 * k) as i32,
                top: y + 1,
                right: g.right - (4.0 * k) as i32,
                bottom: y + rh - 1,
            }
        } else {
            RECT { left: g.left, top: y, right: g.right, bottom: y + rh }
        };
        // 命中区只算看得见那一段：不然鼠标停在"类别"表头上，会把爬到跟下的那一行
        // 当成悬停目标，点下去选中一张看不见的类别。
        let hitr = RECT { left: g.left, top: y.max(top), right: g.right, bottom: (y + rh).min(g.bottom) };
        let on = r.id == a.org_sel;
        let hot = over(&hitr, a.pt.0, a.pt.1) && a.pt.0 < sbx;
        let rowbg = if fine(s) {
            if on {
                s.skin.sel
            } else if hot {
                mix(s.skin.pane, s.skin.hov, 0.50)
            } else {
                s.skin.pane
            }
        } else if draft(s) {
            if on {
                s.skin.sel
            } else if hot {
                mix(s.skin.pane, s.skin.hov, 0.52)
            } else {
                s.skin.pane
            }
        } else if quiet(s) {
            if on {
                s.skin.sel
            } else if hot {
                mix(s.skin.pane, s.skin.hov, 0.40)
            } else {
                s.skin.pane
            }
        } else if on {
            s.skin.sel
        } else if hot {
            s.skin.hov
        } else {
            s.skin.pane
        };
        let keep = if fine(s) {
            round_clip(hdc, &rowr, (5.0 * k).round() as i32)
        } else {
            round_clip(hdc, &rowr, (1.0 * k).round() as i32)
        };
        if on {
            // 选中整行就是标题栏那一档颜色，整行铺纯色。
            fill(hdc, &rowr, s.skin.sel);
        } else if hot {
            fill(hdc, &rowr, rowbg);
        }
        if fine(s) && on {
            rring(hdc, &rowr, (5.0 * k).round() as i32, theme::darken(s.skin.sel, 0.30), None);
        }
        if on && (quiet(s) || draft(s) || fine(s)) {
            fill(
                hdc,
                &RECT { left: rowr.left, top: rowr.top + 2, right: rowr.left + 2, bottom: rowr.bottom - 2 },
                theme::darken(s.skin.acc, 0.30),
            );
        }
        // 拖类别改父级：指针停在哪个类别上，那个就是落点
        if a.org_cat_drag.is_some() && r.id == a.org_cat_to {
            fill(hdc, &rowr, s.skin.acc_soft);
            if fine(s) {
                rring(hdc, &rowr, (5.0 * k).round() as i32, s.skin.acc, None);
            } else {
                frame(hdc, &rowr, s.skin.acc);
            }
        }
        if (quiet(s) || draft(s)) && !on && !hot {
            hline(
                hdc,
                g.left,
                g.right,
                y + rh - 1,
                mix(s.skin.pane, s.skin.line_soft, if draft(s) { 0.40 } else { 0.22 }),
            );
        }
        let x = g.left + (6.0 * k) as i32 + r.depth * ind;
        let ar = RECT { left: x, top: y, right: x + arrow_w, bottom: y + rh };
        if r.kids {
            txt(
                hdc,
                &ar,
                if r.open { "▾" } else { "▸" },
                if on {
                    s.skin.sel_ink
                } else {
                    s.skin.ink_dim
                },
                a.font_sm,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
        }
        // 勾选框：表示当前选中的图片（可多张）在不在这个类别里。
        let cbox = (12.0 * k).round() as i32;
        let chx = ar.right + (2.0 * k) as i32;
        let chk = RECT {
            left: chx,
            top: y + (rh - cbox) / 2,
            right: chx + cbox,
            bottom: y + (rh - cbox) / 2 + cbox,
        };
        if r.id != 0 {
            let on_c = matches!(r.check, Some(CheckState::On));
            let part = matches!(r.check, Some(CheckState::Part));
            let cbg = if on_c {
                if on { s.skin.sel_ink } else { s.skin.acc }
            } else if part {
                if on { mix(s.skin.sel_ink, s.skin.acc, 0.45) } else { s.skin.acc_soft }
            } else {
                s.skin.canvas
            };
            let cbd = if on_c || part || hot {
                if on { s.skin.sel_ink } else { s.skin.acc }
            } else {
                s.skin.line_soft
            };
            if fine(s) {
                let rad = cbox / 2;
                let keep = round_clip(hdc, &chk, rad);
                fill(hdc, &chk, cbg);
                unround(hdc, keep);
                rring(hdc, &chk, rad, cbd, None);
            } else {
                fill(hdc, &chk, cbg);
                frame(hdc, &chk, cbd);
                if draft(s) && (on_c || part) {
                    hline(hdc, chk.left, chk.right, chk.top, s.skin.acc);
                }
            }
            if on_c {
                txt(
                    hdc,
                    &chk,
                    "✓",
                    if on { s.skin.acc } else { s.skin.ink_inv },
                    a.font_sm,
                    DT_CENTER | DT_SINGLELINE | DT_VCENTER,
                );
            } else if part {
                let bar = RECT {
                    left: chk.left + (3.0 * k) as i32,
                    top: (chk.top + chk.bottom) / 2 - 1,
                    right: chk.right - (3.0 * k) as i32,
                    bottom: (chk.top + chk.bottom) / 2 + 1,
                };
                fill(hdc, &bar, if on { s.skin.sel_ink } else { s.skin.acc });
            }
            a.org_checks.push((
                r.id,
                RECT { left: chx - (2.0 * k) as i32, top: hitr.top, right: chx + cbox + (2.0 * k) as i32, bottom: hitr.bottom },
            ));
        }
        let ix = chk.right + (4.0 * k) as i32;
        let isz = (13.0 * k).round() as i32;
        let ik = if r.id == 0 { Ik::Pic } else { Ik::Folder };
        if fine(s) || quiet(s) || draft(s) {
            icon_line(hdc, ik, ix, y + (rh - isz) / 2, isz, if on { s.skin.sel_ink } else { s.skin.acc }, rowbg);
        } else {
            icon(hdc, ik, ix, y + (rh - isz) / 2, isz, if on { s.skin.sel_ink } else { s.skin.acc }, rowbg);
        }
        let cr = RECT {
            left: g.right - (54.0 * k) as i32,
            top: y,
            right: g.right - 4 - sb.as_ref().map(|(_, _, w)| *w).unwrap_or(0),
            bottom: y + rh,
        };
        if fine(s) {
            let pill = RECT {
                left: cr.left - (4.0 * k) as i32,
                top: y + (2.0 * k) as i32,
                right: cr.right,
                bottom: y + rh - (2.0 * k) as i32,
            };
            let rad = (pill.bottom - pill.top) / 2;
            // 选中那一行的数字不再单独压一块底：整行就是标题栏那一档，数字坐在同一档上就够，
            // 加深或变浅都会让这一行看着比别处多一层。
            if !on {
                let keep = round_clip(hdc, &pill, rad);
                fill(hdc, &pill, mix(s.skin.pane, s.skin.canvas, 0.42));
                unround(hdc, keep);
                rring(hdc, &pill, rad, mix(s.skin.line_soft, s.skin.acc, 0.10), None);
            }
        } else if draft(s) {
            let cell = RECT {
                left: cr.left - (5.0 * k) as i32,
                top: y,
                right: cr.right,
                bottom: y + rh,
            };
            vline(hdc, cell.left, cell.top + 2, cell.bottom - 2, s.skin.line_soft);
        }
        // 圆角那一档数字要往左让开胶囊右边那道弧，字尾贴着边会被弧啃掉一截。
        // 选中那行不画胶囊，但同一列得跟着一起让，不然数字两边对不齐。
        let tr = if fine(s) {
            RECT { right: cr.right - (5.0 * k) as i32, ..cr }
        } else {
            cr
        };
        txt(
            hdc,
            &tr,
            &format!("{}", r.count),
            if on {
                s.skin.sel_ink
            } else if quiet(s) {
                s.skin.ink_dim
            } else {
                s.skin.ink_dim
            },
            a.font_num,
            DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
        );
        let nr = RECT {
            left: ix + isz + (4.0 * k) as i32,
            top: y,
            right: cr.left - 2,
            bottom: y + rh,
        };
        txt(
            hdc,
            &nr,
            &r.name,
            if on {
                // 选中行的字色跟目录树同一档：那一档是按标题栏底色的对比度挑出来的，
                // 这里再按版式家族分一支正文色，两页就会读成两种颜色。
                s.skin.sel_ink
            } else {
                s.skin.ink
            },
            if (fine(s) || draft(s)) && on { a.font_b } else { a.font },
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
        unround(hdc, keep);
        a.org_hits.push((r.id, hitr));
    }
    if let Some((track, thumb, _)) = &sb {
        let grab = over(track, a.pt.0, a.pt.1) || a.org_drag.is_some();
        if fine(s) {
            fill(hdc, track, mix(s.skin.pane, s.skin.canvas, 0.22));
            fill(hdc, thumb, if grab { s.skin.acc } else { mix(s.skin.chrome_hi, s.skin.acc, 0.26) });
        } else {
            fill(hdc, track, s.skin.win);
            fill(hdc, thumb, if grab { s.skin.acc } else { s.skin.chrome_hi });
        }
    }
    if a.cat.is_none() {
        let msg = "正在读取分类数据库…".to_string();
        let mr = RECT {
            left: g.left + (10.0 * k) as i32,
            top: g.bottom - (s.row * 4) as i32,
            right: g.right - (10.0 * k) as i32,
            bottom: g.bottom - (8.0 * k) as i32,
        };
        fill(hdc, &mr, s.skin.canvas);
        frame(hdc, &mr, s.skin.line_soft);
        txt(hdc, &mr, &msg, s.skin.ink_dim, a.font_sm, DT_LEFT | DT_WORDBREAK | DT_END_ELLIPSIS);
    }
    unclip(hdc);
}

// ---- 右侧栏第二页：批量转换 ----

/// 右栏标题带上的页签。每页一个 ✕，只关自己那一页；两页都关掉整条栏才收。
unsafe fn paint_dock_tabs(hdc: HDC, a: &mut App, s: &Theme, head: &RECT, active: u8) {
    let k = s.scale;
    let pad = (5.0 * k) as i32;
    let gap = (3.0 * k) as i32;
    let inner_l = head.left + pad;
    let inner_r = head.right - pad;
    // 只给开着的页画页签：关掉的那一页不在条上留空位。
    let mut pages: Vec<(usize, usize, &str)> = Vec::new();
    if a.org_open {
        pages.push((C_TAB_ORG, M_ORG_CLOSE, "图片分类"));
    }
    if a.conv_open {
        pages.push((C_TAB_CONV, C_CLOSE, "批量转换"));
    }
    if pages.is_empty() {
        return;
    }
    let n = pages.len() as i32;
    let tw = ((inner_r - inner_l - gap * (n - 1)) / n).max(10);
    let xw = (15.0 * k) as i32;
    for (i, &(id, close_id, label)) in pages.iter().enumerate() {
        let l = inner_l + i as i32 * (tw + gap);
        let r = RECT {
            left: l,
            top: head.top + (2.0 * k) as i32,
            right: l + tw,
            bottom: head.bottom - (3.0 * k) as i32,
        };
        let on = match id {
            C_TAB_ORG => active == 0,
            _ => active == 1,
        };
        let hot = over(&r, a.pt.0, a.pt.1);
        fill(
            hdc,
            &r,
            if on {
                s.skin.canvas
            } else if hot {
                mix(s.skin.pane_head, s.skin.hov, 0.55)
            } else {
                mix(s.skin.pane_head, s.skin.pane, 0.35)
            },
        );
        if on {
            fill(
                hdc,
                &RECT { left: r.left, top: r.bottom - (2.0 * k) as i32, right: r.right, bottom: r.bottom },
                s.skin.acc,
            );
        }
        // 每个页签自带一个小 ✕，只关这一页。
        let xr = RECT {
            left: r.right - xw - (2.0 * k) as i32,
            top: r.top + (2.0 * k) as i32,
            right: r.right - (2.0 * k) as i32,
            bottom: r.bottom - (2.0 * k) as i32,
        };
        let xh = over(&xr, a.pt.0, a.pt.1);
        if xh {
            fill(hdc, &xr, mix(s.skin.canvas, s.skin.hov, 0.7));
        }
        xmark(
            hdc,
            (xr.left + xr.right) / 2,
            (xr.top + xr.bottom) / 2,
            (3.2 * k) as i32,
            if xh { s.skin.acc } else { s.skin.ink_off },
        );
        // ✕ 先入表：它落在页签矩形里，点击时先命中关闭。
        a.btns.push((close_id, xr));
        let lbl = RECT { left: r.left + (2.0 * k) as i32, top: r.top, right: xr.left, bottom: r.bottom };
        txt(
            hdc,
            &lbl,
            label,
            if on { s.skin.ink } else { s.skin.ink_dim },
            if on { a.font_b } else { a.font_sm },
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
        a.btns.push((id, lbl));
    }
}

/// 分段按钮：整组里被选中的那块用强调色填满。
unsafe fn conv_seg(hdc: HDC, a: &mut App, s: &Theme, r: RECT, id: usize, label: &str, on: bool) {
    let hot = over(&r, a.pt.0, a.pt.1);
    fill(
        hdc,
        &r,
        if on {
            s.skin.acc
        } else if hot {
            s.skin.hov
        } else {
            s.skin.chrome_hi
        },
    );
    frame(hdc, &r, if on { s.skin.acc } else { s.skin.line });
    txt(
        hdc,
        &r,
        label,
        if on { s.skin.ink_inv } else { s.skin.ink },
        a.font_sm,
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );
    a.btns.push((id, r));
}

/// 普通按钮，主按钮用强调色。
unsafe fn conv_btn(
    hdc: HDC,
    a: &mut App,
    s: &Theme,
    r: RECT,
    id: usize,
    label: &str,
    primary: bool,
    enabled: bool,
) {
    let hot = enabled && over(&r, a.pt.0, a.pt.1);
    fill(
        hdc,
        &r,
        if !enabled {
            s.skin.chrome
        } else if primary {
            s.skin.acc
        } else if hot {
            s.skin.hov
        } else {
            s.skin.chrome_hi
        },
    );
    frame(hdc, &r, if primary { s.skin.acc } else { s.skin.line });
    let ink = if !enabled {
        s.skin.ink_off
    } else if primary {
        s.skin.ink_inv
    } else {
        s.skin.ink
    };
    txt(hdc, &r, label, ink, a.font, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    if enabled {
        a.btns.push((id, r));
    }
}

/// 勾选行：小方块 + 文字。
unsafe fn conv_check(
    hdc: HDC,
    a: &mut App,
    s: &Theme,
    r: RECT,
    id: usize,
    label: &str,
    on: bool,
) {
    let k = s.scale;
    let bx = (12.0 * k) as i32;
    let b = RECT {
        left: r.left,
        top: r.top + (r.bottom - r.top - bx) / 2,
        right: r.left + bx,
        bottom: r.top + (r.bottom - r.top + bx) / 2,
    };
    fill(hdc, &b, if on { s.skin.acc } else { s.skin.canvas });
    frame(hdc, &b, if over(&r, a.pt.0, a.pt.1) { s.skin.acc } else { s.skin.line });
    if on {
        txt(hdc, &b, "✓", s.skin.ink_inv, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    }
    let tr = RECT { left: b.right + (5.0 * k) as i32, top: r.top, right: r.right, bottom: r.bottom };
    txt(hdc, &tr, label, s.skin.ink, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    a.btns.push((id, r));
}

/// 细滑条。点击换算在 mouse_down 里按同一个矩形做。
unsafe fn conv_slider(hdc: HDC, a: &mut App, s: &Theme, r: RECT, id: usize, v: i32, lo: i32, hi: i32) {
    let k = s.scale;
    let h = (5.0 * k) as i32;
    let mid = (r.top + r.bottom) / 2;
    let track = RECT { left: r.left, top: mid - h / 2, right: r.right, bottom: mid + h / 2 };
    fill(hdc, &track, s.skin.canvas);
    frame(hdc, &track, s.skin.line);
    let t = ((v - lo) as f64 / (hi - lo).max(1) as f64).clamp(0.0, 1.0);
    let fr = RECT {
        left: track.left + 1,
        top: track.top + 1,
        right: track.left + 1 + ((track.right - track.left - 2) as f64 * t) as i32,
        bottom: track.bottom - 1,
    };
    fill(hdc, &fr, s.skin.acc);
    let hd = RECT {
        left: fr.right - (3.0 * k) as i32,
        top: track.top - (4.0 * k) as i32,
        right: fr.right + (3.0 * k) as i32,
        bottom: track.bottom + (4.0 * k) as i32,
    };
    fill(hdc, &hd, s.skin.chrome_hi);
    frame(hdc, &hd, s.skin.acc);
    a.btns.push((id, r));
}

/// 分组条：整栏宽，左侧强调色小标题。
unsafe fn conv_sec(hdc: HDC, a: &App, s: &Theme, r: RECT, label: &str) {
    fill(hdc, &r, mix(s.skin.pane_head, s.skin.canvas, 0.30));
    hline(hdc, r.left, r.right, r.top, s.skin.line_soft);
    let tr = RECT {
        left: r.left + (9.0 * s.scale) as i32,
        top: r.top,
        right: r.right - (6.0 * s.scale) as i32,
        bottom: r.bottom,
    };
    txt(hdc, &tr, label, s.skin.acc, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
}

/// 控件前面的小标题，右边可以跟一个当前值。
unsafe fn conv_cap(hdc: HDC, a: &App, s: &Theme, r: RECT, label: &str, value: &str) {
    txt(hdc, &r, label, s.skin.ink_dim, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    if !value.is_empty() {
        txt(hdc, &r, value, s.skin.ink, a.font_num, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
    }
}

thread_local! {
    /// 画帧全程攥着 app 锁，而藏一个正攥着焦点的输入框会同步触发 EN_KILLFOCUS
    /// 回到 WM_COMMAND，那里还要拿这把锁——所以隐藏/还焦点攒到这里，帧画完放锁再补做。
    static EDIT_OPS: std::cell::RefCell<Vec<(HWND, bool)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// 放锁后补做攒下的输入框操作。`hot` 的那颗先把焦点还给主窗。
unsafe fn flush_edit_ops(hwnd: HWND) {
    let ops = EDIT_OPS.with(|c| std::mem::take(&mut *c.borrow_mut()));
    for (ed, hot) in ops {
        if hot {
            let _ = SetFocus(Some(hwnd));
        }
        let _ = ShowWindow(ed, SW_HIDE);
    }
}

/// 面板打开/关闭时原生输入框跟着进退。
unsafe fn hide_conv_edits(a: &App) {
    let hot_focus = GetFocus();
    for ed in a.conv_edits {
        if !ed.0.is_null() {
            EDIT_OPS.with(|c| c.borrow_mut().push((ed, hot_focus == ed)));
        }
    }
}

/// 把五个输入框摆到这一帧算出来的位置上；滚出可视区就藏起来。
unsafe fn place_conv_edits(a: &mut App, body: &RECT, _hwnd: HWND) {
    let blocked = a.dlg.is_some() || a.open_menu.is_some() || a.bmenu.is_some();
    for i in 0..E_N {
        let ed = a.conv_edits[i];
        if ed.0.is_null() {
            continue;
        }
        let r = a.conv_edit_rects[i];
        let vis = !blocked && r.right > r.left && r.top >= body.top && r.bottom <= body.bottom;
        if vis {
            let _ = SetWindowPos(
                ed,
                None,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            let _ = ShowWindow(ed, SW_SHOWNOACTIVATE);
        } else {
            // 滚出可视区的那一格不能再攥着焦点，否则接着敲的字会写进看不见的框里。
            // 藏框/还焦点会同步触发 EN_KILLFOCUS，攒下放锁后再做。
            let hot = GetFocus() == ed;
            EDIT_OPS.with(|c| c.borrow_mut().push((ed, hot)));
        }
    }
}

unsafe fn paint_conv(_hwnd: HWND, hdc: HDC, a: &mut App, z: &Zones, s: &Theme) {
    a.zone(theme::Z_ORG);
    let g = z.org;
    if g.right <= g.left {
        hide_conv_edits(a);
        return;
    }
    clip(hdc, &g);
    fill(hdc, &g, if draft(s) { mix(s.skin.pane, s.skin.canvas, 0.22) } else { s.skin.pane });
    vline(
        hdc,
        g.left,
        g.top,
        g.bottom,
        if fine(s) || quiet(s) || draft(s) { s.skin.line_soft } else { s.skin.line },
    );
    // 标题带：和整理页同一套底，里面换成页签
    let head = RECT {
        left: g.left,
        top: g.top,
        right: g.right,
        bottom: g.top + org_title_h(s),
    };
    if fine(s) {
        fill(hdc, &head, s.skin.pane_head);
        hline(
            hdc,
            head.left,
            head.right,
            head.top + 1,
            mix(s.skin.pane_head, s.skin.chrome_hi, 0.38),
        );
    } else if quiet(s) {
        fill(hdc, &head, s.skin.pane_head);
    } else if draft(s) {
        fill(hdc, &head, mix(s.skin.pane_head, s.skin.chrome, 0.32));
        fill(
            hdc,
            &RECT { left: head.left, top: head.top, right: head.left + 2, bottom: head.bottom },
            s.skin.acc,
        );
    } else {
        fill(hdc, &head, s.skin.pane_head);
    }
    paint_dock_tabs(hdc, a, s, &head, 1);
    hline(
        hdc,
        head.left,
        head.right,
        head.bottom - 1,
        if fine(s) { s.skin.line_soft } else { s.skin.line },
    );

    let k = s.scale;
    let foot_h = (72.0 * k) as i32;
    let body = RECT {
        left: g.left,
        top: head.bottom,
        right: g.right,
        bottom: (g.bottom - foot_h).max(head.bottom),
    };
    clip(hdc, &body);

    let pad = (9.0 * k) as i32;
    let sbw = (10.0 * k).max(8.0) as i32;
    let x0 = body.left + pad;
    let xr = body.right - pad - sbw;
    let cw = (xr - x0).max(30);
    let gap = (5.0 * k) as i32;
    let rh = (s.row + (7.0 * k) as i32).max(22);
    let cap_h = s.row.max(16);
    let eh = (19.0 * k) as i32;
    let c3 = (cw - gap * 2) / 3;
    let c4 = (cw - gap * 3) / 4;
    let c2 = (cw - gap) / 2;
    let c5 = (cw - gap * 4) / 5;
    let top0 = body.top + (6.0 * k) as i32;
    let mut y = top0 - a.conv_scroll;
    a.conv_edit_rects = [RECT::default(); E_N];
    // 这一页从这里起推进命中表的矩形，到页脚前一段好整条挑出来裁（见下面 split_off）
    let hit_mark = a.btns.len();

    // —— 输出 ——
    conv_sec(hdc, a, s, RECT { left: g.left, top: y, right: g.right, bottom: y + cap_h }, "输出");
    y += cap_h + gap;
    conv_cap(hdc, a, s, RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h }, "格式", "");
    y += cap_h;
    let fmts = ["JPEG", "PNG", "BMP", "TIFF", "GIF"];
    for (i, name) in fmts.iter().enumerate() {
        let left = x0 + i as i32 * (c5 + gap);
        conv_seg(
            hdc,
            a,
            s,
            RECT { left, top: y, right: left + c5, bottom: y + rh },
            C_FMT + i,
            name,
            a.conv_fmt == i,
        );
    }
    y += rh + gap;
    let q = a.conv_opts.get(a.conv_fmt).map(|o| o.quality).unwrap_or(90);
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "质量",
        &format!("{q}"),
    );
    y += cap_h;
    let sr = RECT { left: x0, top: y, right: xr, bottom: y + (14.0 * k) as i32 };
    conv_slider(hdc, a, s, sr, C_QUAL, q as i32, 1, 100);
    y += sr.bottom - sr.top + gap;
    // 目录单独一行标签，下面整栏给输入框，条子才看得见
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "输出目录",
        "",
    );
    y += cap_h;
    a.conv_edit_rects[2] = RECT {
        left: x0,
        top: y,
        right: xr - (24.0 * k) as i32,
        bottom: y + eh,
    };
    conv_seg(
        hdc,
        a,
        s,
        RECT { left: xr - (22.0 * k) as i32, top: y, right: xr, bottom: y + eh },
        C_BROWSE_OUT,
        "…",
        false,
    );
    y += eh + gap;
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "同名文件",
        "",
    );
    y += cap_h;
    for (i, (name, val)) in OW_SLOTS.iter().enumerate() {
        let left = x0 + i as i32 * (c3 + gap);
        conv_seg(
            hdc,
            a,
            s,
            RECT { left, top: y, right: left + c3, bottom: y + rh },
            C_OW + i,
            name,
            a.conv_overwrite == *val,
        );
    }
    y += rh + gap;

    // —— 调整 ——
    conv_sec(hdc, a, s, RECT { left: g.left, top: y, right: g.right, bottom: y + cap_h }, "调整");
    y += cap_h + gap;
    conv_cap(hdc, a, s, RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h }, "缩放", "");
    y += cap_h;
    for (i, name) in ["原样", "适应", "填充", "拉伸", "缩小"].iter().enumerate() {
        let left = x0 + i as i32 * (c5 + gap);
        conv_seg(
            hdc,
            a,
            s,
            RECT { left, top: y, right: left + c5, bottom: y + rh },
            C_FIT + i,
            name,
            a.conv_fit as usize == i,
        );
    }
    y += rh + gap;
    // 尺寸这一排：标签一行，下面是 宽 × 高 + 按比例 + 允许放大
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "目标尺寸（宽 × 高）",
        "",
    );
    y += cap_h;
    let we = (54.0 * k) as i32;
    a.conv_edit_rects[0] = RECT { left: x0, top: y, right: x0 + we, bottom: y + eh };
    let xmul = x0 + we + (3.0 * k) as i32;
    txt(
        hdc,
        &RECT { left: xmul, top: y, right: xmul + (10.0 * k) as i32, bottom: y + eh },
        "×",
        s.skin.ink_dim,
        a.font_sm,
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );
    let hx = xmul + (10.0 * k) as i32;
    a.conv_edit_rects[1] = RECT { left: hx, top: y, right: hx + we, bottom: y + eh };
    let sy = hx + we + gap;
    conv_seg(
        hdc,
        a,
        s,
        RECT { left: sy, top: y, right: sy + (56.0 * k) as i32, bottom: y + eh },
        C_SYNC,
        "按比例",
        false,
    );
    conv_check(
        hdc,
        a,
        s,
        RECT { left: sy + (56.0 * k) as i32 + gap, top: y, right: xr, bottom: y + eh },
        C_UP,
        "允许放大",
        a.conv_up,
    );
    y += eh + gap;
    conv_cap(hdc, a, s, RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h }, "旋转", "");
    y += cap_h;
    for (i, name) in ["0°", "90°", "180°", "270°"].iter().enumerate() {
        let left = x0 + i as i32 * (c4 + gap);
        conv_seg(
            hdc,
            a,
            s,
            RECT { left, top: y, right: left + c4, bottom: y + rh },
            C_ROT + i,
            name,
            a.conv_rot == (i as u16 * 90),
        );
    }
    y += rh + gap;
    conv_check(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + c2, bottom: y + rh },
        C_FLIP_H,
        "水平翻转",
        a.conv_fliph,
    );
    conv_check(
        hdc,
        a,
        s,
        RECT { left: x0 + c2 + gap, top: y, right: xr, bottom: y + rh },
        C_FLIP_V,
        "垂直翻转",
        a.conv_flipv,
    );
    y += rh + gap;
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "锐化半径",
        &format!("{:.1}", a.conv_sharp_r as f64 / 2.0),
    );
    y += cap_h;
    let sr = RECT { left: x0, top: y, right: xr, bottom: y + (14.0 * k) as i32 };
    conv_slider(hdc, a, s, sr, C_SHARP_R, a.conv_sharp_r as i32, 0, 20);
    y += sr.bottom - sr.top + gap;
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "锐化强度",
        &format!("{:.1}", a.conv_sharp as f64 / 10.0),
    );
    y += cap_h;
    let sr = RECT { left: x0, top: y, right: xr, bottom: y + (14.0 * k) as i32 };
    conv_slider(hdc, a, s, sr, C_SHARP, a.conv_sharp as i32, 0, 30);
    y += sr.bottom - sr.top + gap;

    // —— 水印 ——
    conv_sec(hdc, a, s, RECT { left: g.left, top: y, right: g.right, bottom: y + cap_h }, "水印");
    // 单位说明挂在标题条右边，不再给「字号」单独开一行标题
    txt(
        hdc,
        &RECT { left: x0, top: y, right: xr, bottom: y + cap_h },
        "字号按长边百分比",
        s.skin.ink_dim,
        a.font_sm,
        DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
    );
    y += cap_h + gap;
    conv_check(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + (108.0 * k) as i32, bottom: y + rh },
        C_WM_ON,
        "加文字水印",
        a.conv_wm,
    );
    y += rh + gap;
    for (i, name) in ["左上", "右上", "左下", "右下"].iter().enumerate() {
        let left = x0 + i as i32 * (c4 + gap);
        conv_seg(
            hdc,
            a,
            s,
            RECT { left, top: y, right: left + c4, bottom: y + rh },
            C_WM_POS + i,
            name,
            a.conv_wm_pos as usize == i,
        );
    }
    y += rh + gap;
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "水印文字",
        "",
    );
    y += cap_h;
    a.conv_edit_rects[3] = RECT { left: x0, top: y, right: xr, bottom: y + eh };
    y += eh + gap;
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "字号",
        &format!("{}%", a.conv_wm_size),
    );
    y += cap_h;
    let sr = RECT { left: x0, top: y, right: xr, bottom: y + (14.0 * k) as i32 };
    conv_slider(hdc, a, s, sr, C_WM_SIZE, a.conv_wm_size as i32, 1, 20);
    y += sr.bottom - sr.top + gap;
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "不透明度",
        &format!("{}%", a.conv_wm_opac),
    );
    y += cap_h;
    let sr = RECT { left: x0, top: y, right: xr, bottom: y + (14.0 * k) as i32 };
    conv_slider(hdc, a, s, sr, C_WM_OPAC, a.conv_wm_opac as i32, 0, 100);
    y += sr.bottom - sr.top + gap;

    // —— 重命名 ——
    conv_sec(
        hdc,
        a,
        s,
        RECT { left: g.left, top: y, right: g.right, bottom: y + cap_h },
        "重命名",
    );
    txt(
        hdc,
        &RECT { left: x0, top: y, right: xr, bottom: y + cap_h },
        "* 原名，# 序号",
        s.skin.ink_dim,
        a.font_sm,
        DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
    );
    y += cap_h + gap;
    conv_check(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + (108.0 * k) as i32, bottom: y + rh },
        C_RN_ON,
        "转换后改名",
        a.conv_rn,
    );
    y += rh + gap;
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "模板",
        "",
    );
    y += cap_h;
    a.conv_edit_rects[4] = RECT { left: x0, top: y, right: xr, bottom: y + eh };
    y += eh + gap;
    conv_cap(
        hdc,
        a,
        s,
        RECT { left: x0, top: y, right: x0 + cw, bottom: y + cap_h },
        "起始编号",
        "",
    );
    y += cap_h;
    let nb = (rh as f64 * 0.7) as i32;
    let nsx = x0;
    conv_seg(
        hdc,
        a,
        s,
        RECT { left: nsx, top: y, right: nsx + nb, bottom: y + rh },
        C_RN_DN,
        "−",
        false,
    );
    txt(
        hdc,
        &RECT { left: nsx + nb + 2, top: y, right: nsx + nb + (54.0 * k) as i32, bottom: y + rh },
        &format!("{}", a.conv_rn_start),
        s.skin.ink,
        a.font_num,
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );
    conv_seg(
        hdc,
        a,
        s,
        RECT {
            left: nsx + nb + (56.0 * k) as i32,
            top: y,
            right: nsx + nb * 2 + (56.0 * k) as i32,
            bottom: y + rh,
        },
        C_RN_UP,
        "+",
        false,
    );
    y += rh + gap;

    let content_h = (y + a.conv_scroll - top0 + pad).max(1);
    let body_h = (body.bottom - body.top).max(1);
    a.conv_max_scroll = (content_h - body_h).max(0);
    if a.conv_scroll > a.conv_max_scroll {
        a.conv_scroll = a.conv_max_scroll;
    }
    if let Some((track, thumb, _)) = tree_sb(body, body.top, content_h as usize, 1, a.conv_scroll, k) {
        fill(hdc, &track, mix(s.skin.pane, s.skin.canvas, 0.25));
        let grab = over(&track, a.pt.0, a.pt.1) || a.conv_drag.is_some();
        fill(
            hdc,
            &thumb,
            if grab { s.skin.acc } else { mix(s.skin.chrome_hi, s.skin.acc, 0.30) },
        );
    }
    // 滚出可视区的行只是被裁掉不发给你看，矩形还留在命中表里，而且排在页脚那两个
    // 按钮前面——点开始/停止就先撞上一个看不见的设置，看着像按钮缺一块不能点。
    // 只留完整落在 body 里的那些，和上面原生输入框的显隐判据同一口径。
    let pushed = a.btns.split_off(hit_mark);
    a.btns.extend(
        pushed
            .into_iter()
            .filter(|(_, r)| r.top >= body.top && r.bottom <= body.bottom),
    );
    unclip(hdc);

    // 底部固定：进度 + 开始/停止 + 重置
    let foot = RECT { left: g.left, top: body.bottom, right: g.right, bottom: g.bottom };
    fill(hdc, &foot, mix(s.skin.pane_head, s.skin.pane, 0.40));
    hline(hdc, foot.left, foot.right, foot.top, s.skin.line);
    let total = a.conv_total.load(Ordering::Relaxed);
    let done = a.conv_done.load(Ordering::Relaxed);
    let running = CONV_RUN.load(Ordering::Relaxed);
    let pr = RECT {
        left: x0,
        top: foot.top + (8.0 * k) as i32,
        right: g.right - pad,
        bottom: foot.top + (18.0 * k) as i32,
    };
    fill(hdc, &pr, s.skin.canvas);
    frame(hdc, &pr, s.skin.line_soft);
    // 点开始却没动静时，原因必须写在这条上：状态栏那行太窄，看不到。
    // 上一次跑完留下的进度条会让新提示没地方显示，所以闲置时提示优先。
    let idle_msg = !running && !a.conv_msg.is_empty();
    if idle_msg {
        txt(
            hdc,
            &pr,
            &a.conv_msg.clone(),
            s.skin.acc,
            a.font_sm,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
    } else if total > 0 {
        let t = (done as f64 / total as f64).clamp(0.0, 1.0);
        let fillw = ((pr.right - pr.left - 2) as f64 * t) as i32;
        if fillw > 0 {
            fill(
                hdc,
                &RECT { left: pr.left + 1, top: pr.top + 1, right: pr.left + 1 + fillw, bottom: pr.bottom - 1 },
                s.skin.acc,
            );
        }
        txt(
            hdc,
            &pr,
            &format!("{done}/{total}"),
            s.skin.ink,
            a.font_sm,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
    } else {
        txt(
            hdc,
            &pr,
            "就绪",
            s.skin.ink_dim,
            a.font_sm,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
    }
    let by = foot.bottom - (30.0 * k) as i32;
    let bh = (22.0 * k) as i32;
    let fright = g.right - pad;
    let bw = ((fright - x0 - gap) * 2 / 3).max(40);
    let start_r = RECT { left: x0, top: by, right: x0 + bw, bottom: by + bh };
    let reset_r = RECT { left: start_r.right + gap, top: by, right: fright, bottom: by + bh };
    if running {
        conv_btn(hdc, a, s, start_r, C_STOP, "停止", true, true);
        conv_btn(hdc, a, s, reset_r, C_RESET, "重置", false, false);
    } else {
        conv_btn(hdc, a, s, start_r, C_START, "开始转换", true, true);
        conv_btn(hdc, a, s, reset_r, C_RESET, "重置", false, true);
    }

    place_conv_edits(a, &body, _hwnd);
    unclip(hdc);
}

/// 面板上的五个原生输入框。创建一次，开关面板时整组显示/隐藏。
unsafe fn create_conv_edits(hwnd: HWND) {
    let (texts, font) = {
        let g = app().lock().unwrap();
        if g.conv_edits.iter().any(|h| !h.0.is_null()) {
            return;
        }
        (
            [
                g.conv_tw.clone(),
                g.conv_th.clone(),
                g.conv_out.clone(),
                g.conv_wm_text.clone(),
                g.conv_rn_tpl.clone(),
            ],
            g.zf[theme::Z_ORG].f,
        )
    };
    let mut made = [HWND::default(); E_N];
    for i in 0..E_N {
        let htxt = HSTRING::from(texts[i].as_str());
        // 宽高只收数字；其余三格要能输入路径/中文。
        let num = if i < 2 { WINDOW_STYLE(0x2000) } else { WINDOW_STYLE(0) };
        let ed = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(pw("EDIT").as_ptr()),
            &htxt,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | WINDOW_STYLE(0x0080) | num,
            0,
            0,
            10,
            10,
            Some(hwnd),
            Some(HMENU((E_TW + i) as isize as *mut core::ffi::c_void)),
            None,
            None,
        );
        let Ok(ed) = ed else { continue };
        let _ = SendMessageW(ed, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
        let _ = SetWindowSubclass(ed, Some(edit_proc), 1, 0);
        made[i] = ed;
    }
    for ed in made {
        if !ed.0.is_null() {
            let _ = ShowWindow(ed, SW_HIDE);
        }
    }
    let mut g = app().lock().unwrap();
    for i in 0..E_N {
        if !made[i].0.is_null() {
            g.conv_edits[i] = made[i];
        }
    }
}

unsafe fn destroy_conv_edits() {
    let hs = {
        let mut g = app().lock().unwrap();
        std::mem::replace(&mut g.conv_edits, [HWND::default(); E_N])
    };
    for ed in hs {
        if !ed.0.is_null() {
            let _ = DestroyWindow(ed);
        }
    }
}

/// 切换右栏页签。整理页没开就顺手打开，转换页同理。
unsafe fn set_dock_tab(hwnd: HWND, tab: u8) {
    let create = {
        let mut g = app().lock().unwrap();
        if tab == 0 {
            g.org_open = true;
            g.dock_tab = 0;
        } else {
            if !g.conv_open {
                g.conv_w_prev = g.org_w;
                g.conv_w_auto = true;
                g.conv_open = true;
                let wide = (300.0 * g.scale).round() as i32;
                if g.org_w < wide {
                    g.org_w = wide;
                }
            }
            g.dock_tab = 1;
        }
        let create = g.conv_edits.iter().all(|h| h.0.is_null());
        g.feed(hwnd);
        create
    };
    if create {
        create_conv_edits(hwnd);
    }
    refresh(hwnd);
}

unsafe fn open_conv_panel(hwnd: HWND) {
    let create = {
        let mut g = app().lock().unwrap();
        if !g.conv_open {
            g.conv_w_prev = g.org_w;
            g.conv_w_auto = true;
            g.conv_open = true;
            let wide = (300.0 * g.scale).round() as i32;
            if g.org_w < wide {
                g.org_w = wide;
            }
        }
        g.dock_tab = 1;
        if g.conv_out.is_empty() {
            g.conv_out = g.dir.display().to_string();
        }
        let create = g.conv_edits.iter().all(|h| h.0.is_null());
        g.feed(hwnd);
        create
    };
    if create {
        create_conv_edits(hwnd);
    }
    refresh(hwnd);
}

unsafe fn close_conv_panel(hwnd: HWND) {
    {
        let mut g = app().lock().unwrap();
        g.conv_open = false;
        if g.org_open {
            g.dock_tab = 0;
        }
        // 只有为转换页自动加宽的那次才还回去；用户自己拖过的栏宽不动。
        if g.conv_w_auto {
            g.org_w = g.conv_w_prev;
            g.conv_w_auto = false;
        }
        g.feed(hwnd);
    }
    destroy_conv_edits();
    refresh(hwnd);
}

/// 面板上控件 id 的分发。命中返回 true，点击就不再落到后面的通用按钮表。
unsafe fn conv_command(hwnd: HWND, id: usize, x: i32, r: &RECT) -> bool {
    match id {
        C_TAB_ORG => {
            set_dock_tab(hwnd, 0);
            return true;
        }
        C_TAB_CONV => {
            set_dock_tab(hwnd, 1);
            return true;
        }
        C_CLOSE => {
            close_conv_panel(hwnd);
            return true;
        }
        C_BROWSE_OUT => {
            if let Some(dir) = browse_for_folder(hwnd, "选择输出文件夹") {
                let ed = {
                    let mut g = app().lock().unwrap();
                    g.conv_out = dir.clone();
                    g.conv_out_auto = false;
                    g.conv_edits[2]
                };
                if !ed.0.is_null() {
                    set_edit_text(ed, &dir);
                }
                refresh(hwnd);
            }
            return true;
        }
        C_SYNC => {
            let src = app()
                .lock()
                .unwrap()
                .selected_paths()
                .into_iter()
                .find(|p| scan::is_image(p));
            let dims = src.and_then(|p| job::open_meta(&p)).filter(|(w, h, _)| *w > 0 && *h > 0);
            let (tw, th) = {
                let g = app().lock().unwrap();
                (
                    g.conv_tw.trim().parse::<u32>().unwrap_or(0),
                    g.conv_th.trim().parse::<u32>().unwrap_or(0),
                )
            };
            if let Some((sw, sh, _)) = dims {
                let (nw, nh) = if tw > 0 {
                    (tw.to_string(), (tw as u64 * sh as u64 / sw as u64).max(1).to_string())
                } else if th > 0 {
                    ((th as u64 * sw as u64 / sh as u64).max(1).to_string(), th.to_string())
                } else {
                    (String::new(), String::new())
                };
                if !nw.is_empty() {
                    let (e0, e1) = {
                        let mut g = app().lock().unwrap();
                        g.conv_tw = nw.clone();
                        g.conv_th = nh.clone();
                        (g.conv_edits[0], g.conv_edits[1])
                    };
                    if !e0.0.is_null() {
                        set_edit_text(e0, &nw);
                    }
                    if !e1.0.is_null() {
                        set_edit_text(e1, &nh);
                    }
                }
            } else {
                app().lock().unwrap().msg = "先选一张图片，才知道原来的比例".to_string();
            }
            refresh(hwnd);
            return true;
        }
        C_START => {
            start_conv_panel(hwnd);
            return true;
        }
        C_STOP => {
            CONV_STOP.store(true, Ordering::SeqCst);
            app().lock().unwrap().msg = "正在停下…".to_string();
            refresh(hwnd);
            return true;
        }
        C_RESET => {
            {
                let mut g = app().lock().unwrap();
                g.conv_fit = 1;
                g.conv_up = false;
                g.conv_rot = 0;
                g.conv_fliph = false;
                g.conv_flipv = false;
                g.conv_sharp = 0;
                g.conv_sharp_r = 3;
                g.conv_wm = false;
                g.conv_wm_pos = 3;
                g.conv_wm_size = 4;
                g.conv_wm_opac = 55;
                g.conv_wm_text.clear();
                g.conv_rn = false;
                g.conv_rn_tpl = "*".into();
                g.conv_rn_start = 1;
                g.conv_tw.clear();
                g.conv_th.clear();
                g.conv_msg.clear();
            }
            let eds = app().lock().unwrap().conv_edits;
            for (i, t) in [(0usize, ""), (1, ""), (3, ""), (4, "*")] {
                if !eds[i].0.is_null() {
                    set_edit_text(eds[i], t);
                }
            }
            refresh(hwnd);
            return true;
        }
        _ => {}
    }
    if id >= C_FMT && id < C_FMT + encode::FORMATS.len() {
        app().lock().unwrap().conv_fmt = id - C_FMT;
        refresh(hwnd);
        return true;
    }
    if id >= C_OW && id < C_OW + OW_SLOTS.len() {
        app().lock().unwrap().conv_overwrite = OW_SLOTS[id - C_OW].1;
        refresh(hwnd);
        return true;
    }
    if id >= C_FIT && id < C_FIT + 5 {
        app().lock().unwrap().conv_fit = (id - C_FIT) as u8;
        refresh(hwnd);
        return true;
    }
    if id >= C_ROT && id < C_ROT + 4 {
        app().lock().unwrap().conv_rot = ((id - C_ROT) * 90) as u16;
        refresh(hwnd);
        return true;
    }
    if id >= C_WM_POS && id < C_WM_POS + 4 {
        app().lock().unwrap().conv_wm_pos = (id - C_WM_POS) as u8;
        refresh(hwnd);
        return true;
    }
    // 滑条：按点到的横坐标换算，和画的时候同一个矩形。
    if matches!(id, C_QUAL | C_SHARP | C_SHARP_R | C_WM_SIZE | C_WM_OPAC) {
        // 记下这一条和它的轨道两端，mouse_move 接着同一处往下算，拖动和单击不会两套口径
        app().lock().unwrap().conv_sdrag = Some((id, r.left, r.right));
    }
    let t = ((x - r.left) as f64 / (r.right - r.left).max(1) as f64).clamp(0.0, 1.0);
    match id {
        C_QUAL => {
            let mut g = app().lock().unwrap();
            let v = (1.0 + t * 99.0).round() as u8;
            let fmt = g.conv_fmt;
            if let Some(o) = g.conv_opts.get_mut(fmt) {
                o.quality = v.clamp(1, 100);
            }
            drop(g);
            refresh(hwnd);
            true
        }
        C_SHARP => {
            app().lock().unwrap().conv_sharp = (t * 30.0).round() as u8;
            refresh(hwnd);
            true
        }
        C_SHARP_R => {
            app().lock().unwrap().conv_sharp_r = (t * 20.0).round() as u8;
            refresh(hwnd);
            true
        }
        C_WM_SIZE => {
            app().lock().unwrap().conv_wm_size = (1.0 + t * 19.0).round() as u8;
            refresh(hwnd);
            true
        }
        C_WM_OPAC => {
            app().lock().unwrap().conv_wm_opac = (t * 100.0).round() as u8;
            refresh(hwnd);
            true
        }
        C_FLIP_H => {
            let mut g = app().lock().unwrap();
            g.conv_fliph = !g.conv_fliph;
            drop(g);
            refresh(hwnd);
            true
        }
        C_FLIP_V => {
            let mut g = app().lock().unwrap();
            g.conv_flipv = !g.conv_flipv;
            drop(g);
            refresh(hwnd);
            true
        }
        C_WM_ON => {
            let mut g = app().lock().unwrap();
            g.conv_wm = !g.conv_wm;
            drop(g);
            refresh(hwnd);
            true
        }
        C_UP => {
            let mut g = app().lock().unwrap();
            g.conv_up = !g.conv_up;
            drop(g);
            refresh(hwnd);
            true
        }
        C_RN_ON => {
            let mut g = app().lock().unwrap();
            g.conv_rn = !g.conv_rn;
            drop(g);
            refresh(hwnd);
            true
        }
        C_RN_DN => {
            let mut g = app().lock().unwrap();
            g.conv_rn_start = (g.conv_rn_start - 1).max(0);
            drop(g);
            refresh(hwnd);
            true
        }
        C_RN_UP => {
            let mut g = app().lock().unwrap();
            g.conv_rn_start = g.conv_rn_start.saturating_add(1);
            drop(g);
            refresh(hwnd);
            true
        }
        _ => false,
    }
}

/// 批量转换这一趟的经过，全落在缓存目录的 convert.log 上，一行一个事件。
/// 界面上只留得下一行提示，"为什么没出文件"只能靠它。和 XSEE_TRACE 那套分开：
/// 那个要预先设环境变量才开口，这个一直记，他自己点完就能看到。
fn conv_log(s: &str) {
    static F: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
    let slot = F.get_or_init(|| {
        Mutex::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(cache::dir().join("convert.log"))
                .ok(),
        )
    });
    let Ok(mut guard) = slot.lock() else { return };
    let Some(f) = guard.as_mut() else { return };
    let _ = writeln!(f, "t={} {s}", unsafe { stamp_now() });
}

/// 测试台用：把这一页当下能点的那些矩形，整份写进 XSEE_CONV_DUMP 指的文件。
/// 驱动脚本照这份坐标投鼠标，不用猜像素。内容变了才重写（先写临时名再改名，
/// 免得读的人撞上半截）；没设这个环境变量时一句都不写，正常用时不留痕。
fn conv_dump_hits(a: &App, body: &RECT) {
    let Some(p) = std::env::var_os("XSEE_CONV_DUMP") else { return };
    static LAST: Mutex<Option<String>> = Mutex::new(None);
    let mut s = format!(
        "# dlg={} menu={} bmenu={} tab={} conv={} org={} scroll={}/{} body={} {} {} {}\n",
        a.dlg.is_some() as u8,
        a.open_menu.is_some() as u8,
        a.bmenu.is_some() as u8,
        a.dock_tab,
        a.conv_open as u8,
        a.org_open as u8,
        a.conv_scroll,
        a.conv_max_scroll,
        body.left,
        body.top,
        body.right,
        body.bottom
    );
    s.push_str(
        &a.btns
            .iter()
            .filter(|(id, _)| (*id >= C_FMT && *id <= C_SHARP_R))
            .map(|(id, r)| format!("{id} {} {} {} {}\n", r.left, r.top, r.right, r.bottom))
            .collect::<String>(),
    );
    {
        let Ok(mut g) = LAST.lock() else { return };
        if g.as_deref() == Some(s.as_str()) {
            return;
        }
        *g = Some(s.clone());
    }
    let tmp = format!("{}.tmp", p.to_string_lossy());
    if std::fs::write(&tmp, &s).is_ok() {
        let _ = std::fs::rename(&tmp, &p);
    }
}

/// 面板里点开始：选中图按当前设置转一批。和对话框那条路共用同一套计数与收尾消息。
unsafe fn start_conv_panel(hwnd: HWND) {
    let (paths, fmt, opt, out, policy, xf, wm, rename, out_auto, typed_out) = {
        let g = app().lock().unwrap();
        let paths = g.selected_images();
        if paths.is_empty() {
            drop(g);
            let mut g = app().lock().unwrap();
            g.msg = "没有选中要转换的图片".to_string();
            g.conv_msg = "没有选中图片".to_string();
            conv_log(&format!(
                "ev=conv拒 原因=没有选中图片 当前目录={} 视图列出的图片={}",
                g.dir.display(),
                g.n_images()
            ));
            drop(g);
            refresh(hwnd);
            return;
        }
        let fmt = g.conv_fmt;
        let opt = g.conv_opts.get(fmt).cloned().unwrap_or_default();
        // 输出文件夹在默认态（用户没自己定过路径）时跟着当前文件夹走：面板开着
        // 换过文件夹，转换就该落进现在这个，而不是开面板那一刻的那个——不然
        // 文件悄悄落进上一个文件夹，看着就是"点了没反应"。
        let out_auto = g.conv_out_auto || g.conv_out.trim().is_empty();
        let out = if out_auto {
            g.dir.display().to_string()
        } else {
            g.conv_out.trim().to_string()
        };
        if !Path::new(&out).is_dir() {
            drop(g);
            let mut g = app().lock().unwrap();
            g.msg = "输出文件夹不存在".to_string();
            g.conv_msg = "输出文件夹不存在".to_string();
            conv_log(&format!("ev=conv拒 原因=输出文件夹不存在 输出={out} 跟随当前={out_auto} 当前目录={}", g.dir.display()));
            drop(g);
            refresh(hwnd);
            return;
        }
        let policy = match g.conv_overwrite {
            0 => encode::Collision::Overwrite,
            2 => encode::Collision::Rename,
            _ => encode::Collision::Skip,
        };
        let fit = match g.conv_fit {
            0 => encode::FitMode::None,
            1 => encode::FitMode::Contain,
            2 => encode::FitMode::Cover,
            3 => encode::FitMode::Stretch,
            _ => encode::FitMode::Fit,
        };
        let tw = g.conv_tw.trim().parse::<u32>().unwrap_or(0);
        let th = g.conv_th.trim().parse::<u32>().unwrap_or(0);
        // 只填一边也行：另一边在 encode 里按每张图自己的比例补。两边都空就当不缩放，
        // 不再拦着不让开始（拦下来只有状态栏一行，看着就是"点了没反应"）。
        let fit = if tw == 0 && th == 0 { encode::FitMode::None } else { fit };
        let xf = encode::Xform {
            fit,
            w: tw,
            h: th,
            allow_upscale: g.conv_up,
            rotate: g.conv_rot,
            flip_h: g.conv_fliph,
            flip_v: g.conv_flipv,
            sharpen: g.conv_sharp,
            sharpen_r: g.conv_sharp_r,
        };
        let wm = if g.conv_wm && !g.conv_wm_text.trim().is_empty() {
            Some(encode::Watermark {
                text: g.conv_wm_text.trim().to_string(),
                pos: g.conv_wm_pos,
                size_pct: g.conv_wm_size,
                opacity: g.conv_wm_opac,
                color: 0xFFFFFF,
            })
        } else {
            None
        };
        let rename = if g.conv_rn && !g.conv_rn_tpl.trim().is_empty() {
            Some((g.conv_rn_tpl.trim().to_string(), g.conv_rn_start))
        } else {
            None
        };
        (paths, fmt, opt, out, policy, xf, wm, rename, out_auto, g.conv_out.clone())
    };
    // 跟手态下把输出框的显示也对上当前文件夹。set_edit_text 会同步触发
    // EN_CHANGE 把 auto 落成 false——这一笔是程序自己写的，不是用户改的，补回来。
    if out_auto {
        let ed = app().lock().unwrap().conv_edits[2];
        if !ed.0.is_null() {
            set_edit_text(ed, &out);
            app().lock().unwrap().conv_out_auto = true;
        }
    }
    if CONV_RUN.swap(true, Ordering::SeqCst) {
        let mut g = app().lock().unwrap();
        g.msg = "上一批还在转换，等它跑完".to_string();
        g.conv_msg = "上一批还在转换".to_string();
        conv_log("ev=conv拒 原因=上一批还在转换");
        drop(g);
        refresh(hwnd);
        return;
    }
    let (ofmt, _, _) = *encode::FORMATS.get(fmt).unwrap_or(&("?", "?", "?"));
    let pol = match policy {
        encode::Collision::Overwrite => "取代",
        encode::Collision::Skip => "跳过",
        encode::Collision::Rename => "编号",
    };
    let fitn = match xf.fit {
        encode::FitMode::None => "原样",
        encode::FitMode::Contain => "适应",
        encode::FitMode::Cover => "填充",
        encode::FitMode::Stretch => "拉伸",
        encode::FitMode::Fit => "缩小",
    };
    conv_log(&format!(
        "ev=conv起 入口=面板 张={} 格式={ofmt} 质量={} 渐进={} 色度取样={}x{} 缩略图档={} DCF={} PNG压缩={} TIFF压缩={} \
         输出={out} 框里填的=[{typed_out}] 跟随当前目录={out_auto} 撞名={pol} 尺寸={fitn} {}/{} 允许放大={} 旋转={} 翻转={}x{} \
         锐化=半径{:.1}/强度{:.1} 水印={} 改名={}",
        paths.len(),
        opt.quality,
        opt.progressive as u8,
        opt.sample_h as u8,
        opt.sample_v as u8,
        opt.thumb,
        opt.dcf as u8,
        opt.png_level,
        opt.tiff_comp,
        xf.w,
        xf.h,
        xf.allow_upscale as u8,
        xf.rotate,
        xf.flip_h as u8,
        xf.flip_v as u8,
        xf.sharpen_r as f64 / 2.0,
        xf.sharpen as f64 / 10.0,
        wm.as_ref()
            .map(|w| format!("开 位置{} 字号{}% 不透明{}% 字数{}", w.pos, w.size_pct, w.opacity, w.text.chars().count()))
            .unwrap_or_else(|| "关".to_string()),
        rename
            .as_ref()
            .map(|(t, n)| format!("开 模板[{t}] 起始{n}"))
            .unwrap_or_else(|| "关".to_string()),
    ));
    for (i, p) in paths.iter().enumerate().take(20) {
        conv_log(&format!("ev=conv源 {i}={p}"));
    }
    let t0 = Instant::now();
    app().lock().unwrap().conv_msg.clear();
    CONV_STOP.store(false, Ordering::SeqCst);
    encode::begin_batch();
    if let Ok(mut v) = CONV_ERRS.lock() {
        v.clear();
    }
    let hp = hwnd.0 as isize;
    let total = paths.len();
    let prog_done = {
        let g = app().lock().unwrap();
        g.conv_total.store(total, Ordering::SeqCst);
        g.conv_done.store(0, Ordering::SeqCst);
        g.conv_done.clone()
    };
    app().lock().unwrap().msg = format!("开始转换 {total} 个文件…（Esc 停止）");
    refresh(hwnd);
    let paths = Arc::new(paths);
    let out = Arc::new(out);
    let opt = Arc::new(opt);
    let xf = Arc::new(xf);
    let wm = Arc::new(wm);
    let rename = Arc::new(rename);
    let batch = Arc::new(encode::batch_set(&paths));
    let next = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicUsize::new(0));
    let ok = Arc::new(AtomicUsize::new(0));
    let skip = Arc::new(AtomicUsize::new(0));
    let err = Arc::new(AtomicUsize::new(0));
    let workers = 4.min(total.max(1));
    let mut hs = Vec::new();
    for _ in 0..workers {
        let paths = paths.clone();
        let out = out.clone();
        let opt = opt.clone();
        let xf = xf.clone();
        let wm = wm.clone();
        let rename = rename.clone();
        let batch = batch.clone();
        let next = next.clone();
        let done = done.clone();
        let ok = ok.clone();
        let skip = skip.clone();
        let err = err.clone();
        let prog_done = prog_done.clone();
        let total_n = total;
        hs.push(std::thread::spawn(move || loop {
            if CONV_STOP.load(Ordering::SeqCst) {
                break;
            }
            let i = next.fetch_add(1, Ordering::SeqCst);
            if i >= paths.len() {
                break;
            }
            let p = &paths[i];
            let newname = (*rename).as_ref().map(|r| rn_stem(&r.0, r.1, i, p, 0));
            match encode::convert_file_x(
                p,
                &out,
                newname.as_deref(),
                fmt,
                &opt,
                policy,
                Some(&xf),
                wm.as_ref().as_ref(),
                &batch,
            ) {
                Ok(encode::Outcome::Done(op, n)) => {
                    let k = ok.fetch_add(1, Ordering::SeqCst) + 1;
                    // 成功的那几张也记目标路径，但只记前 200 张：几万张一批时
                    // 这本日志不能自己变成新的负担。
                    if k <= 200 {
                        conv_log(&format!("ev=conv张 结果=写出 源={p} 目标={op} 字节={n}"));
                    }
                }
                Ok(encode::Outcome::Skipped) => {
                    skip.fetch_add(1, Ordering::SeqCst);
                    conv_log(&format!("ev=conv张 结果=跳过 源={p}"));
                }
                Err(e) => {
                    err.fetch_add(1, Ordering::SeqCst);
                    conv_log(&format!("ev=conv张 结果=失败 源={p} 原因={e}"));
                    if let Ok(mut v) = CONV_ERRS.lock() {
                        if v.len() < 5 {
                            v.push(format!("{}：{e}", stem(p)));
                        }
                    }
                }
            }
            let n = done.fetch_add(1, Ordering::SeqCst) + 1;
            prog_done.store(n, Ordering::SeqCst);
            if n % 4 == 0 || n == total_n {
                let pct = n * 100 / total_n.max(1);
                app().lock().unwrap().msg = format!("正在转换 {n}/{total_n}（{pct}%，Esc 停止）：{}", stem(p));
                let hwnd = HWND(hp as *mut core::ffi::c_void);
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_PV, WPARAM(0), LPARAM(0));
                }
            }
        }));
    }
    std::thread::spawn(move || {
        for h in hs {
            let _ = h.join();
        }
        let stopped = CONV_STOP.load(Ordering::SeqCst);
        let (o, s, e) = (ok.load(Ordering::SeqCst), skip.load(Ordering::SeqCst), err.load(Ordering::SeqCst));
        if let Ok(mut l) = CONV_LAST.lock() {
            *l = (o, s, e, stopped);
        }
        conv_log(&format!(
            "ev=conv完 应转={} 写出={o} 跳过={s} 失败={e} 中断={} 用时={:.1}秒",
            total,
            stopped as u8,
            t0.elapsed().as_secs_f64()
        ));
        CONV_RUN.store(false, Ordering::SeqCst);
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CONV_DONE, WPARAM(0), LPARAM(0));
        }
    });
}

/// 展开/折叠时把被点的那一行钉在原地：改完树按它的现行号反推滚动量。
/// 不这么做的话，行数一变，绝对像素偏移会被夹到顶上，看着就是滚动条跳了一下。
/// `y_off` 必须是行顶相对内容顶的偏移（`k * rh - tscroll`），传鼠标位置会差 0~20 像素。
/// 行号而不是路径：同一个目录可能在树里出现两次（顶层「桌面」和 C 盘链下的桌面），
/// 按路径找只会开合到先碰上的那一个，另一处跟着一起动。
unsafe fn tree_toggle_keep(hwnd: HWND, row: usize, y_off: i32) {
    // 先量布局：layout 内部要同一把 app 锁，持锁再调就是自己等自己，界面当场卡死
    let z = layout(hwnd).0;
    let mut g = app().lock().unwrap();
    let mut before = Vec::new();
    flat_tree(&g.tree, 0, &mut before);
    let Some((_, path, _, _)) = before.get(row).cloned() else {
        return;
    };
    g.tsel = path;
    let mut seq = 0;
    toggle_at(&mut g.tree, row, &mut seq);
    let mut rows = Vec::new();
    flat_tree(&g.tree, 0, &mut rows);
    let rh = g.th.row.max(1);
    let h = (z.tree.bottom - z.tree.top - g.th.pane_head_h).max(0);
    let maxs = (rows.len() as i32 * rh - h).max(0);
    g.tscroll = (row as i32 * rh - y_off).clamp(0, maxs);
    drop(g);
    refresh(hwnd);
}

/// 树里当前该点亮哪一行：先找完全相同的路径，找不到就沿路径往上找最近的上级行。
/// 键盘接管时也按这个起点走，不这么做的话光标会从顶上那一行突然起跳。
fn tree_row(rows: &[(usize, String, bool, String)], tsel: &str) -> usize {
    let t = tsel.trim_end_matches('\\');
    let mut anchor = usize::MAX;
    for (i, row) in rows.iter().enumerate() {
        let p = row.1.trim_end_matches('\\');
        if p.eq_ignore_ascii_case(t) {
            return i;
        }
        if t.len() > p.len()
            && t.get(..p.len()).is_some_and(|pre| pre.eq_ignore_ascii_case(p))
            && t.get(p.len()..).is_some_and(|rest| rest.starts_with('\\'))
        {
            anchor = i;
        }
    }
    anchor
}

/// 键盘移到某一行之后把那一行滚进视区；树里点出来的滚动位置不动，
/// 这里只在行跑出上下沿时才推一点。
unsafe fn tree_show(g: &mut App, z: &Zones, row: usize) {
    let rh = g.th.row.max(1);
    let h = (z.tree.bottom - z.tree.top - g.th.pane_head_h).max(rh);
    let y = row as i32 * rh;
    if y < g.tscroll {
        g.tscroll = y;
    } else if y + rh > g.tscroll + h {
        g.tscroll = y + rh - h;
    }
    let mut rows = Vec::new();
    flat_tree(&g.tree, 0, &mut rows);
    let maxs = (rows.len() as i32 * rh - h).max(0);
    g.tscroll = g.tscroll.clamp(0, maxs);
}

/// 焦点在树上时的键盘：上下换目录、左右展开折叠、翻页/Home/End 跳着走，
/// 和资源管理器那棵树一致；走到的目录紧接着在右边列出来。
unsafe fn tree_key(hwnd: HWND, vk: u32) {
    let z = layout(hwnd).0;
    let mut g = app().lock().unwrap();
    let mut rows = Vec::new();
    flat_tree(&g.tree, 0, &mut rows);
    if rows.is_empty() {
        return;
    }
    let cur = {
        let r = tree_row(&rows, &g.tsel);
        if r == usize::MAX { 0 } else { r }
    };
    let rh = g.th.row.max(1);
    let h = (z.tree.bottom - z.tree.top - g.th.pane_head_h).max(rh);
    let vis = (h / rh).max(1) as usize;
    let page = vis.saturating_sub(1).max(1);
    let mut to = cur;
    let mut go = false;
    let mut redraw = false;
    match vk {
        0x26 => {
            to = cur.saturating_sub(1);
            go = true;
        }
        0x28 => {
            to = (cur + 1).min(rows.len() - 1);
            go = true;
        }
        0x21 => {
            to = cur.saturating_sub(page);
            go = true;
        }
        0x22 => {
            to = (cur + page).min(rows.len() - 1);
            go = true;
        }
        0x24 => {
            to = 0;
            go = true;
        }
        0x23 => {
            to = rows.len() - 1;
            go = true;
        }
        0x25 => {
            // 左：展着的收起来；已经收着的退到上一级那一行。
            if rows[cur].2 {
                let mut seq = 0;
                toggle_at(&mut g.tree, cur, &mut seq);
                redraw = true;
            } else {
                let d0 = rows[cur].0;
                if let Some(p) = (0..cur).rev().find(|&i| rows[i].0 < d0) {
                    to = p;
                    go = true;
                }
            }
        }
        0x27 => {
            // 右：展着的进第一个子项；收着的就展开。
            if rows[cur].2 {
                if rows.get(cur + 1).is_some_and(|r| r.0 > rows[cur].0) {
                    to = cur + 1;
                    go = true;
                }
            } else {
                let mut seq = 0;
                toggle_at(&mut g.tree, cur, &mut seq);
                redraw = true;
            }
        }
        0x0D => go = true,
        _ => {}
    }
    if go {
        let path = rows[to].1.clone();
        g.tsel = path.clone();
        if path != "此电脑" {
            // 单击切目录时就是这么做的：不替用户展开链子，行停在原地。
            g.tree_noscroll = true;
            g.goto(&path, hwnd, true);
        }
        tree_show(&mut g, &z, to);
    } else if redraw {
        tree_show(&mut g, &z, to);
    }
    let moved = go || redraw;
    drop(g);
    if moved {
        request_pv(hwnd);
        refresh(hwnd);
    }
}

/// 文件夹树：14px 图标走 accent 色，展折箭头用 ▾/▸，选中整行反白。
unsafe fn paint_tree(hdc: HDC, a: &mut App, z: &Zones, s: &Theme) {
    a.zone(theme::Z_TREE);
    let g = z.tree;
    if g.right - g.left < 8 || g.bottom - g.top < 8 {
        return;
    }
    clip(hdc, &g);
    fill(hdc, &g, if draft(s) { mix(s.skin.pane, s.skin.canvas, 0.22) } else { s.skin.pane });
    let head = RECT { left: g.left, top: g.top, right: g.right, bottom: g.top + s.pane_head_h };
    pane_head(hdc, &g, "文件夹(F)", M_PANE_TREE, a, s);
    let mut rows = Vec::new();
    flat_tree(&a.tree, 0, &mut rows);
    let rh = s.row.max(1);
    let top = head.bottom;
    let h = (g.bottom - top).max(0);
    let maxs = (rows.len() as i32 * rh - h).max(0);
    // 滚动位置只有他自己动才算：程序任何时候都不替用户滚，也不"定位到当前目录"。
    a.tscroll = a.tscroll.clamp(0, maxs);
    let first = (a.tscroll / rh).max(0) as usize;
    let last = ((a.tscroll + h) / rh + 1) as usize;
    let k = s.scale;
    let sb = tree_sb(g, head.bottom, rows.len(), rh, a.tscroll, k);
    let sbw = sb.as_ref().map(|(_, _, w)| *w).unwrap_or(0);
    // 滚动条那一竖条上不该再把行染成悬停色，不然看着像滑块会跟着高亮跑
    let sbx = sb.as_ref().map(|(t, _, _)| t.left).unwrap_or(i32::MAX);
    let isz = (14.0 * k).round() as i32;
    let tw = (11.0 * k).round() as i32;
    let indent = (20.0 * k).round() as i32;
    // 当前目录那一行可能压根没加载出来（上级被折叠、或还没展开）：
    // 这时点亮最近的一个看得见的上级，不然切进子目录后整棵树像什么都没选。
    let t = a.tsel.trim_end_matches('\\');
    let mut anchor = usize::MAX;
    let mut first_grp = usize::MAX;
    for (i, row) in rows.iter().enumerate() {
        let p = row.1.trim_end_matches('\\');
        if first_grp == usize::MAX {
            let q = &row.1;
            let kd = if q == "此电脑" || q == &a.home {
                Ik::Pc
            } else if q.len() == 3 && q.ends_with(":\\") {
                Ik::Drive
            } else {
                Ik::Folder
            };
            if kd != Ik::Folder {
                first_grp = i;
            }
        }
        if p.eq_ignore_ascii_case(t) {
            anchor = i;
            break;
        }
        if t.len() > p.len()
            && t.get(..p.len()).is_some_and(|pre| pre.eq_ignore_ascii_case(p))
            && t.get(p.len()..).is_some_and(|rest| rest.starts_with('\\'))
        {
            anchor = i;
        }
    }
    for i in first..last.min(rows.len()) {
        let (d, path, open, name) = &rows[i];
        let y = top + i as i32 * rh - a.tscroll;
        // 同一个目录可能在树里有两行（顶层「桌面」和 C 盘链下的 Desktop），
        // 只染先碰到的那一行，你点的那一行就只剩悬停的浅灰——是同一个目录就都染。
        let on = i == anchor
            || (!t.is_empty() && path.trim_end_matches('\\').eq_ignore_ascii_case(t));
        let full = RECT { left: g.left, top: y, right: g.right, bottom: y + rh };
        // 精密仪器的树行往里收 4px，行本身圆角；A/B 仍是通栏。
        let r = if fine(s) {
            RECT {
                left: g.left + (4.0 * k) as i32,
                top: y + 1,
                right: g.right - (4.0 * k) as i32,
                bottom: y + rh - 1,
            }
        } else {
            full
        };
        let hot = over(&full, a.pt.0, a.pt.1) && a.pt.0 < sbx;
        let hlrow = matches!(a.drop_hl, Some(Hl::Tree(j)) if j == i);
        let is_home = path == &a.home;
        let kind = if path == "此电脑" || is_home {
            Ik::Pc
        } else if path.len() == 3 && path.ends_with(":\\") {
            Ik::Drive
        } else {
            Ik::Folder
        };
        // 卡片档把“此电脑 / 盘符”这几行当表头：整块铺一档灰底，行与行之间留一道 1px。
        // 判据只认图标种类，不看展开状态 —— 折叠判据在展开的一瞬间会跳，他否过。
        let grp = kind != Ik::Folder;
        let lit = on || hot || hlrow;
        let keep = if fine(s) {
            Some(round_clip(hdc, &r, (5.0 * k).round() as i32))
        } else {
            None
        };
        let rowbg;
        if on || hlrow {
            // 选中整行铺纯色，左缘压一道主色导轨。
            fill(hdc, &r, s.skin.sel);
            rowbg = s.skin.sel;
            fill(
                hdc,
                &RECT {
                    left: r.left,
                    top: r.top + if fine(s) { 2 } else { 0 },
                    right: r.left + 2,
                    bottom: r.bottom - if fine(s) { 2 } else { 0 },
                },
                theme::darken(s.skin.acc, 0.30),
            );
        } else if hot {
            rowbg = if fine(s) {
                mix(s.skin.pane, s.skin.hov, 0.52)
            } else if draft(s) {
                mix(s.skin.pane, s.skin.hov, 0.55)
            } else if quiet(s) {
                mix(s.skin.pane, s.skin.hov, 0.42)
            } else {
                s.skin.hov
            };
            fill(hdc, &r, rowbg);
        } else if cards(s) && grp {
            rowbg = if fine(s) {
                mix(s.skin.pane, s.skin.chrome, 0.36)
            } else if draft(s) {
                mix(s.skin.pane, s.skin.chrome, 0.52)
            } else if quiet(s) {
                mix(s.skin.pane, s.skin.chrome, 0.34)
            } else {
                s.skin.hov
            };
            fill(hdc, &r, rowbg);
        } else {
            rowbg = s.skin.pane;
        }
        if (quiet(s) || draft(s)) && !lit {
            hline(
                hdc,
                r.left,
                r.right,
                r.bottom - 1,
                mix(rowbg, s.skin.line_soft, if draft(s) { 0.42 } else { 0.24 }),
            );
        }
        if draft(s) && grp {
            let d = (4.0 * k).round().max(3.0) as i32;
            fill(
                hdc,
                &RECT {
                    left: g.left + (3.0 * k) as i32,
                    top: y + (rh - d) / 2,
                    right: g.left + (3.0 * k) as i32 + d,
                    bottom: y + (rh - d) / 2 + d,
                },
                if on || hlrow { s.skin.sel_ink } else { s.skin.acc },
            );
        }
        if cards(s) && grp && !lit {
            let lc = mix(rowbg, s.skin.line_soft, if fine(s) { 0.30 } else { 0.45 });
            hline(hdc, r.left, r.right, r.bottom - 1, lc);
            if i == first_grp {
                hline(hdc, r.left, r.right, r.top, lc);
            }
        }
        if on || hlrow {
            if fine(s) {
                hline(hdc, r.left + 2, r.right, r.top, mix(rowbg, s.skin.line_soft, 0.28));
                hline(hdc, r.left + 2, r.right, r.bottom - 1, mix(rowbg, s.skin.line_soft, 0.40));
            } else if quiet(s) || draft(s) {
                hline(hdc, r.left + 2, r.right, r.top, mix(rowbg, s.skin.line_soft, 0.24));
                hline(hdc, r.left + 2, r.right, r.bottom - 1, mix(rowbg, s.skin.line_soft, 0.34));
                if draft(s) {
                    fill(
                        hdc,
                        &RECT { left: r.left + 2, top: r.bottom - 2, right: r.right, bottom: r.bottom },
                        mix(rowbg, theme::darken(s.skin.acc, 0.45), 0.56),
                    );
                }
            } else if cards(s) {
                frame(hdc, &r, mix(s.skin.sel, s.skin.focus, 0.55));
                fill(hdc, &RECT { left: r.right - 2, top: r.top, right: r.right, bottom: r.bottom }, s.skin.focus);
            } else if bands(s) {
                fill(hdc, &RECT { left: r.left, top: r.top, right: r.left + 2, bottom: r.bottom }, theme::darken(s.skin.acc, 0.30));
            }
        }
        let ink = if on || hlrow {
            s.skin.sel_ink
        } else if bands(s) {
            if hot { s.skin.ink } else { s.skin.ink_dim }
        } else {
            s.skin.ink
        };
        let x = g.left + (4.0 * k) as i32 + (*d as i32) * indent;
        let mk = RECT { left: x, top: y, right: x + tw, bottom: y + rh };
        if !is_home {
            let tc = if on || hlrow {
                s.skin.sel_ink
            } else if bands(s) {
                s.skin.ink_off
            } else {
                s.skin.ink_dim
            };
            let (mx, my) = ((mk.left + mk.right) / 2, (mk.top + mk.bottom) / 2);
            if bands(s) {
                tri(hdc, mx, my, (3.1 * k) as i32, tc, *open);
            } else {
                txt(
                    hdc,
                    &mk,
                    if *open { "▾" } else { "▸" },
                    tc,
                    a.font_sm,
                    DT_CENTER | DT_SINGLELINE | DT_VCENTER,
                );
            }
        }
        let ix = x + tw + (3.0 * k) as i32;
        // 图标比文字亮一档就会先被看到：收一点透明度，让它退回“行首的记号”
        let ic = if on || hlrow {
            if bands(s) { mix(rowbg, s.skin.sel_ink, 0.9) } else { s.skin.sel_ink }
        } else if bands(s) {
            mix(rowbg, s.skin.acc, if hot { 0.95 } else { 0.72 })
        } else {
            s.skin.acc
        };
        if fine(s) || quiet(s) || draft(s) {
            icon_line(hdc, kind, ix, y + (rh - isz) / 2, isz, ic, rowbg);
        } else {
            icon(hdc, kind, ix, y + (rh - isz) / 2, isz, ic, rowbg);
        }
        let nr = RECT {
            left: ix + isz + (3.0 * k) as i32,
            top: y,
            right: g.right - 2 - sbw,
            bottom: y + rh,
        };
        txt(
            hdc,
            &nr,
            name,
            ink,
            if (fine(s) || draft(s)) && (on || hlrow) { a.font_b } else { a.font },
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
        if let Some(k) = keep {
            unround(hdc, k);
        }
    }
    if let Some((track, thumb, _)) = &sb {
        let grab = over(track, a.pt.0, a.pt.1) || a.tree_drag.is_some();
        if fine(s) || quiet(s) {
            fill(hdc, track, mix(s.skin.pane, s.skin.canvas, 0.22));
            fill(
                hdc,
                thumb,
                if grab {
                    s.skin.acc
                } else {
                    mix(s.skin.chrome_hi, s.skin.acc, if quiet(s) { 0.18 } else { 0.26 })
                },
            );
        } else if draft(s) {
            fill(hdc, track, mix(s.skin.canvas, s.skin.pane, 0.18));
            fill(hdc, thumb, if grab { s.skin.acc } else { mix(s.skin.line, s.skin.acc, 0.18) });
        } else if bands(s) {
            fill(hdc, track, s.skin.canvas);
            fill(hdc, thumb, if grab { s.skin.focus } else { s.skin.line });
        } else {
            fill(hdc, track, s.skin.win);
            fill(hdc, thumb, if grab { s.skin.acc } else { s.skin.chrome_hi });
        }
    }
    unclip(hdc);
    hline(hdc, g.left, g.right, g.bottom - 1, if fine(s) { s.skin.line_soft } else { s.skin.line });
}

/// 虚线框：wait 态的缩略图盒。
unsafe fn dashed(hdc: HDC, r: &RECT, c: u32) {
    let pen = CreatePen(PS_DOT, 1, cref(c));
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let ob = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    let _ = Rectangle(hdc, r.left, r.top, r.right, r.bottom);
    SelectObject(hdc, ob);
    SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(pen.0));
}

/// 走光的相位：进程内一颗钟，所有格子共用，扫起来才是一条光带在走而不是一格各闪各的。
fn shimmer_t() -> f64 {
    static T0: OnceLock<Instant> = OnceLock::new();
    T0.get_or_init(Instant::now).elapsed().as_secs_f64()
}

/// 待生成格底缘那道走光：2px 一条，亮心从左扫到右，2.4 秒一圈。
/// 虚线破洞在深蓝上读成“坏了”，一条安静的光带才读成“在干活”。
unsafe fn wait_band(hdc: HDC, r: &RECT, s: &Theme, bg: u32) {
    let h = (2.0 * s.scale).round().max(2.0) as i32;
    let y = r.bottom - (3.0 * s.scale).round() as i32 - h;
    let x0 = r.left + (3.0 * s.scale).round() as i32;
    let w = (r.right - (3.0 * s.scale).round() as i32 - x0).max(4);
    let cx = -0.4 + (shimmer_t() / 2.4).fract() * 1.8;
    let seg = 20;
    for i in 0..seg {
        let f = i as f64 / seg as f64;
        let al = (1.0 - (f - cx).abs() / 0.4).clamp(0.0, 1.0);
        if al < 0.02 {
            continue;
        }
        let a0 = x0 + (w as f64 * f).round() as i32;
        let a1 = x0 + (w as f64 * (i + 1) as f64 / seg as f64).round() as i32;
        fill(hdc, &RECT { left: a0, top: y, right: a1.max(a0 + 1), bottom: y + h }, mix(bg, s.skin.chrome_hi, al));
    }
}

/// 2×2 格的棋盘底纹，预览盒用；图案刷对齐窗口原点，和 CSS 的 0 0 一致。
unsafe fn checker(hdc: HDC, r: &RECT, c1: u32, c2: u32, cell: i32) {
    let n = (cell * 2).max(2);
    let mut bits = vec![0u8; (n * n * 4) as usize];
    for y in 0..n {
        for x in 0..n {
            let c = if ((x / cell) + (y / cell)) % 2 == 0 { c1 } else { c2 };
            let o = ((y * n + x) * 4) as usize;
            bits[o] = (c & 0xff) as u8;
            bits[o + 1] = ((c >> 8) & 0xff) as u8;
            bits[o + 2] = ((c >> 16) & 0xff) as u8;
            bits[o + 3] = 0xff;
        }
    }
    let bmp = CreateBitmap(n, n, 1, 32, Some(bits.as_ptr() as *const _));
    if bmp.is_invalid() {
        fill(hdc, r, c1);
        return;
    }
    let br = CreatePatternBrush(bmp);
    let _ = FillRect(hdc, r, br);
    let _ = DeleteObject(HGDIOBJ(br.0));
    let _ = DeleteObject(HGDIOBJ(bmp.0));
}

/// 文件列表：三行一格（图盒 / 尺寸 / 名字条），选择只染名字条和图盒内框。
/// 格里的一行字：攒到网格循环末尾再按字体分堆画。
/// 一屏 144 行字，逐格画要在数字体和名字体之间来回切一百多次，
/// GDI 每切一次字体就把这个画布上攒好的字模丢掉、下一行重新现做。
/// 同一种字体连着画，一帧只切两三次，排字那笔钱才不用重付。
struct GridText {
    r: RECT,
    s: String,
    c: u32,
    /// 这一行字底下那块的纯色，用来把排好的字存成小位图反复贴；NO_BK 是底下不纯，照旧现排。
    bk: u32,
    /// 存位图时左右让开的列数（格子缝 + 卡片描边），现排时不用。
    pad: i32,
}

/// `band` 是这一帧新露出来的那一条：给了它的帧，整屏格子沿用后备位图里搬过来的那一份，
/// 这一帧只画压在这条带上的格子。没给就是照常整块列表从头画。
unsafe fn paint_grid(hdc: HDC, a: &mut App, z: &Zones, s: &Theme, band: Option<&RECT>) {
    // 只拆滚轮帧。地面钉在窗口上、逐格钉在内容上，滚动时这两层能不能一起搬进
    // 常驻位图，得先知道各吃掉多少。
    let sub_on = seg_tracing() && PAINT_SRC.load(Ordering::Relaxed) == 1;
    let mut sub_acc = [0u32; 10];
    let mut sub_t = Instant::now();
    macro_rules! submark {
        ($i:expr) => {{
            if sub_on {
                let now = Instant::now();
                sub_acc[$i] = sub_acc[$i].saturating_add(now.duration_since(sub_t).as_micros() as u32);
                sub_t = now;
            }
        }};
    }
    a.zone(theme::Z_GRID);
    let g = z.grid;
    clip(hdc, &g);
    // 条带帧成不成就看这一层地面：整片重画会把搬过来的格子一并盖掉。
    // 只补卡片缝成功才留 band；铺了整片地面就等于上一帧没了，所有格子都得重画。
    let mut band = band;
    if cards(s) {
        if band.is_some() && !ground_gaps(hdc, a, &g, s) {
            band = None;
        }
        if band.is_none() {
            ground_cached(hdc, &g, s);
        }
    } else {
        fill(hdc, &g, s.skin.canvas);
        band = None;
    }
    submark!(0);
    // 逐格这一层再按调用点拆：只在这个循环里开着，地面和尾巴不算进来。
    let deep0 = if sub_on {
        SUB_DEEP.store(true, Ordering::Relaxed);
        deep_snap()
    } else {
        [0; 7]
    };
    let (first, last) = a.range(g);
    a.vis = (first, last);
    let listy = matches!(a.mode, Mode::List | Mode::Detail);
    let cx = a.cols(g);
    let k = s.scale;
    let (bw, bh) = a.box_size();
    let dim_h = s.meta;
    let lab_h = s.label;
    let pad2 = (2.0 * k).round() as i32;
    let edge = (3.0 * k).round() as i32;
    // 尺寸行 / 名字行 / 选中加粗的名字行，三堆按字体分开攒
    let cap = (last - first).max(0) as usize;
    let mut tl_dim: Vec<GridText> = Vec::with_capacity(cap);
    let mut tl_nm: Vec<GridText> = Vec::with_capacity(cap);
    let mut tl_bd: Vec<GridText> = Vec::with_capacity(cap);
    let mut tl_band: Vec<Band> = Vec::with_capacity(cap);
    // 全表 filter 计数，一格一次就是 O(可见格 × 全目录)/帧；整帧算一次就够。
    let n_imgs = a.n_images();
    for i in first..last {
        let is_dir = a.entries[i].is_dir;
        let name = a.disp(i);
        // 文件的“亮着”只认 marked；sel 只是光标。Ctrl 点掉最后一项后不再画成选中。
        let sel = a.sel == i && a.sel_lit;
        let mark = a.marked.contains(&a.entries[i].path);
        let hot = a.hover == i;
        let hl = matches!(a.drop_hl, Some(Hl::Grid(k)) if k == i);
        // 进了选区就算选中：框选进来的和点选的那一格用同一套底色，
        // 不再分"光标停着那一个"和"其余"两种颜色。
        let picked = sel || mark || hl;
        let r = a.cell_rect(i, g);
        if r.bottom < g.top || r.top > g.bottom {
            continue;
        }
        // 条带帧：这一格没被新露出来那条压上，位图里搬过来的就是它，一笔都不用动
        if let Some(b) = band {
            if !cross(&r, b) {
                continue;
            }
        }
        if listy {
            if picked {
                fill(hdc, &r, s.skin.sel);
            } else if hot {
                fill(hdc, &r, s.skin.hov);
            }
            let ink = if picked { s.skin.sel_ink } else { s.skin.ink };
            let dim = if picked { ink } else { s.skin.ink_dim };
            let isz = s.icon.max(14);
            let ib = RECT {
                left: r.left + s.pad,
                top: r.top + 1,
                right: r.left + s.pad + isz + isz / 2,
                bottom: r.bottom - 1,
            };
            let iy = r.top + (r.bottom - r.top - isz) / 2;
            let ibg = if picked { s.skin.sel } else { s.skin.canvas };
            if is_dir {
                icon_cached(hdc, IK_FOLDER, ib.left, iy, isz, if picked { s.skin.sel_ink } else { s.skin.acc }, ibg);
            } else if a.entries[i].is_image {
                // 图片按扩展名给格式牌；表里没有的后缀退回原来的实心图片标。
                let nm = stem(&a.entries[i].path);
                let ext = nm.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
                match fmticon::index(ext) {
                    Some(fi) => {
                        let ink = fmticon::pick(fi, ibg);
                        // 列表档和详细档的行内图标用同一套：只放格式名，不带 X。
                        icon_cached(hdc, IK_FMT_LIST + fi as u8, ib.left, iy, isz, ink, ibg);
                    }
                    None => icon_cached(
                        hdc,
                        IK_PIC,
                        ib.left,
                        iy,
                        isz,
                        if picked { s.skin.sel_ink } else { s.skin.acc },
                        ibg,
                    ),
                }
            } else if !sysicon::draw(hdc, &a.entries[i].path, ib.left + 1, iy, isz, ibg) {
                icon_cached(
                    hdc,
                    IK_FILE,
                    ib.left + 1,
                    iy,
                    isz,
                    if picked { s.skin.sel_ink } else { s.skin.ink_dim },
                    ibg,
                );
            }
            // 列表视图：挂在类别里的图在图标右上角带一个配色标记色的小标
            if !is_dir && a.cat.as_ref().map(|c| c.any_cat(&a.entries[i].path)).unwrap_or(false) {
                let t = (9.0 * k).round().max(7.0) as i32;
                let ty = r.top + (r.bottom - r.top - isz) / 2;
                let tag = RECT { left: ib.right - t, top: ty, right: ib.right, bottom: ty + t };
                fill(hdc, &tag, s.skin.mark);
            }
            // 列表档一列只放图标 + 名字，名字从本单位格的图标右边起；
            // 详细资料还是整行排，名字按表头那条列线走。
            let (nl, nr) = if a.mode == Mode::List {
                (r.left + s.pad + isz + isz / 2 + s.gap, r.right - s.pad)
            } else {
                (cx.name, (cx.typ - 4).min(cx.right).max(cx.name))
            };
            let nb = RECT { left: nl, top: r.top, right: nr, bottom: r.bottom };
            if a.ren_idx != i {
                txt(hdc, &nb, &name, ink, a.font, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
            }
            if a.mode == Mode::Detail {
                let (sz, mt) = a.stat_cached(i);
                let typ_t = if is_dir {
                    "文件夹".to_string()
                } else {
                    type_label(
                        Path::new(&a.entries[i].path)
                            .extension()
                            .and_then(|x| x.to_str())
                            .unwrap_or(""),
                    )
                };
                let dim_t = if is_dir {
                    "文件夹".to_string()
                } else {
                    match a.dims.get(&i).copied() {
                        Some((w, h)) if w > 0 => format!("{w} × {h}"),
                        _ if !a.entries[i].is_image => Path::new(&a.entries[i].path)
                            .extension()
                            .and_then(|x| x.to_str())
                            .unwrap_or("")
                            .to_ascii_uppercase(),
                        _ => "…".to_string(),
                    }
                };
                // 列宽拖定以后总宽可能比可见区还宽：每格右缘都裁到列表右缘，
                // 不然文字会画进右边滚动条那一列。左缘已经越过右缘的格子留空。
                let cell = |x0: i32, x1: i32| RECT {
                    left: x0,
                    top: r.top,
                    right: x1.min(cx.right).max(x0),
                    bottom: r.bottom,
                };
                let y0 = cell(cx.typ, cx.dim - 4);
                let d = cell(cx.dim, cx.date - 4);
                let t = cell(cx.date, cx.size - 4);
                let z2 = cell(cx.size, cx.end);
                // 每列的文字只在自己那一格里裁（放不下就出省略号），
                // 窄窗口下压到邻列去就成了两列都读不出字。
                txt(hdc, &y0, &typ_t, dim, a.font_sm, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                txt(hdc, &d, &dim_t, dim, a.font_num, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                txt(hdc, &t, &mt, dim, a.font_num, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                txt(hdc, &z2, &fmt_size(sz), dim, a.font_num, DT_RIGHT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
            }
            continue;
        }
        let boxr = RECT { left: r.left, top: r.top, right: r.left + bw, bottom: r.top + bh };
        let inner = RECT {
            left: boxr.left + pad2,
            top: boxr.top + pad2,
            right: boxr.right - pad2,
            bottom: boxr.bottom - pad2,
        };
        let has_img = a.thumbs.contains_key(&i);
        let on = picked;
        let rich = rich(s);
        let fine = fine(s);
        // 选中/悬停/标记各有一层色纱：卡片、尺寸行、名称带都按同一个状态色走，
        // 这样不会出现“边线变了、底纹和字还是原样”的白开水状态。
        let tint = if on { s.skin.sel } else if mark { s.skin.acc_soft } else if hot { s.skin.hov } else { s.skin.pane };
        let tint_amt = if on {
            if fine {
                0.0
            } else if draft(s) {
                0.42
            } else if quiet(s) {
                0.34
            } else {
                0.58
            }
        } else if mark {
            if fine { 0.22 } else { 0.38 }
        } else if hot {
            if fine { 0.14 } else { 0.28 }
        } else {
            0.0
        };
        let tinted = |c: u32| {
            if rich || fine || quiet(s) || draft(s) {
                mix(c, tint, tint_amt)
            } else {
                c
            }
        };
        // 卡片档：整格（图 + 尺寸 + 名字）是一枚卡片，左右各让出 3px 当缝。
        // 环最后画，压在所有底色上面 —— 画早了会被盒的渐变切掉顶边，断成一截一截。
        let card = RECT { left: r.left + edge, top: r.top, right: r.right - edge, bottom: r.bottom };
        let cbox = RECT { left: card.left, top: card.top, right: card.right, bottom: card.top + bh };
        let cin = RECT {
            left: cbox.left + (6.0 * k).round() as i32,
            top: cbox.top + (6.0 * k).round() as i32,
            right: cbox.right - (6.0 * k).round() as i32,
            bottom: cbox.bottom - (6.0 * k).round() as i32,
        };
        // 图标档的格子就是按 48 像素图标留的，不能再吃卡片那 6px 内缩，
        // 不然同一个图标在档里会小一圈。
        let pic_in = if cards(s) && !is_dir && a.mode != Mode::Small { cin } else { inner };
        // 这一格铺不铺卡面：几档素卡上的文件夹格是不铺的，那时候字直接坐在柔光地面上。
        // 铺了才谈得上"字底下是一整块纯色"，尺寸行能不能存成小位图就看它。
        let plated =
            cards(s) && (!is_dir || on || hot || mark || rich || fine || quiet(s) || draft(s));
        if plated {
            // 卡片底和盒的渐变都要裁进圆角里：不裁的话方角的那几粒像素会顶在圆环外面。
            // 只有精密仪器用圆角；静谧玻璃和制图台是方角卡片。
            let keep = round_clip(hdc, &card, card_rad(s));
            // 文件夹格和文件格都铺一整块纯色，卡面不做渐变。
            let flat = is_dir;
            fill(hdc, &card, tinted(s.skin.chrome));
            if fine {
                // 上下各一道 1px 内沿：上沿提亮，下沿压暗，给卡面一点层次。
                let hi = mix(tinted(s.skin.pane), tinted(s.skin.chrome_hi), 0.58);
                let lo = mix(tinted(s.skin.pane), tinted(s.skin.canvas), 0.64);
                let inset = (8.0 * k).round() as i32;
                hline(hdc, card.left + inset, card.right - inset, card.top + 1, hi);
                hline(hdc, card.left + inset, card.right - inset, card.bottom - 2, lo);
            }
            if rich {
                // 尺寸行 + 名称带坐在同一块卡尾上；下半张不再露出地面。
                let foot = RECT { left: card.left, top: cbox.bottom, right: card.right, bottom: card.bottom };
                fill(hdc, &foot, if flat { tinted(s.skin.chrome) } else { tinted(s.skin.pane) });
                // 选中和标记给一道顶缘色条，和圆环一起把卡片从背景里提出来。
                if on || mark {
                    let rail = RECT {
                        left: card.left + (7.0 * k).round() as i32,
                        top: card.top,
                        right: card.right - (7.0 * k).round() as i32,
                        bottom: card.top + (2.0 * k).round().max(1.0) as i32,
                    };
                    fill(hdc, &rail, if mark && !on { s.skin.mark } else { s.skin.acc });
                }
            }
            unround(hdc, keep);
        }
        // 缩略图档和图标档共用同一套格子，文件夹/图片图标都铺满图位（两档一样大），
        // 区别只在图片那一格画的是缩略图还是一枚图标。
        let icon_sz = {
            let pw = pic_in.right - pic_in.left;
            let ph = pic_in.bottom - pic_in.top;
            // 图标档的图标按 SMALL_ICON 画，图位比它大也别撑开。
            if a.mode == Mode::Small {
                SMALL_ICON.min(pw).min(ph).max(1)
            } else {
                pw.min(ph).max(1)
            }
        };
        let icon_ix = pic_in.left + ((pic_in.right - pic_in.left) - icon_sz) / 2;
        let icon_iy = pic_in.top + ((pic_in.bottom - pic_in.top) - icon_sz) / 2;
        let icon_col = if quiet(s) && on {
            mix(s.skin.acc, s.skin.sel_ink, 0.22)
        } else if rich && on {
            mix(s.skin.acc, s.skin.sel_ink, 0.55)
        } else if fine && on {
            mix(s.skin.acc, s.skin.sel_ink, 0.30)
        } else {
            s.skin.acc
        };
        // 图标底色得贴着格子实际坐的那层：卡片档那层是盒的渐变，不是 canvas
        let icon_bg = if cards(s) { tinted(s.skin.pane) } else { s.skin.canvas };
        if is_dir {
            icon(hdc, Ik::Folder, icon_ix, icon_iy, icon_sz, icon_col, icon_bg);
        } else if a.mode == Mode::Small {
            // 图标档也列非图片：图片按扩展名给格式牌，表里没有的后缀退回实心图片标。
            if a.entries[i].is_image {
                let nm = stem(&a.entries[i].path);
                let ext = nm.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
                match fmticon::index(ext) {
                    // 格式牌每帧重采样一遍太贵，走和列表档同一份图标缓存。
                    Some(fi) => icon_cached(
                        hdc,
                        IK_FMT + fi as u8,
                        icon_ix,
                        icon_iy,
                        icon_sz,
                        fmticon::pick(fi, icon_bg),
                        icon_bg,
                    ),
                    None => icon_pic_solid(hdc, icon_ix, icon_iy, icon_sz, icon_col, icon_bg),
                }
            } else {
                let fc = if on { s.skin.sel_ink } else { s.skin.ink_dim };
                // 非图片文件用系统图标；取不到（没注册的扩展名）退回单色文件标。
                if !sysicon::draw(hdc, &a.entries[i].path, icon_ix, icon_iy, icon_sz, icon_bg) {
                    icon(hdc, Ik::File, icon_ix, icon_iy, icon_sz, fc, icon_bg);
                }
            }
        } else if let Some(im) = a.thumbs.get_mut(&i) {
            let d = box_in(im.w, im.h, pic_in);
            dib_cache(hdc, &d, im);
            // 设计稿：.cell.sel img{box-shadow:none} —— 选中时那圈外环已经够重，图边再描一道就糊了
            if !(cards(s) && on) {
                frame(hdc, &d, if bands(s) { s.skin.line_soft } else { s.skin.line });
            }
        } else if cards(s) {
            // 待生成：盒底铺平，进度收到底缘一条光带，不再打虚线破洞
            if !(rich || fine) {
                let keep = round_clip(hdc, &card, card_rad(s));
                fill(hdc, &cbox, s.skin.chrome);
                unround(hdc, keep);
            }
            wait_band(hdc, &cbox, s, s.skin.pane);
        } else if bands(s) {
            frame(hdc, &inner, s.skin.line_soft);
            wait_band(hdc, &inner, s, s.skin.canvas);
        } else {
            dashed(hdc, &inner, s.skin.line_soft);
        }
        if !is_dir {
            let ext = Path::new(&a.entries[i].path)
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_ascii_uppercase();
            if ext == "PSD" || ext == "PSB" {
                let br = RECT {
                    left: boxr.right - (30.0 * k) as i32,
                    top: boxr.bottom - (16.0 * k) as i32,
                    right: boxr.right - (4.0 * k) as i32,
                    bottom: boxr.bottom - (2.0 * k) as i32,
                };
                txt(hdc, &br, &ext, s.skin.mark, a.font_sm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
            }
        }
        if cards(s) {
            // 卡片档一律不画方形描边，状态全交给最后那圈圆环：方框顶在圆角外面就是"蓝框断开"。
            if mark && !is_dir && !rich && !fine && !quiet(s) && !draft(s) {
                let t = (13.0 * k).round().max(9.0) as i32;
                let bd = RECT { left: cin.left, top: cin.top, right: cin.left + t, bottom: cin.top + t };
                fill(hdc, &bd, s.skin.acc);
                tick(hdc, (bd.left + bd.right) / 2, (bd.top + bd.bottom) / 2, s.skin.ink_inv, k * 0.9);
            }
            if rich && (on || mark) && !is_dir {
                // 选中/标记角标：给“这一张被选中了”一个不依赖边框的标记。
                let t = (15.0 * k).round().max(11.0) as i32;
                let bd = RECT {
                    left: card.left + (7.0 * k).round() as i32,
                    top: card.top + (7.0 * k).round() as i32,
                    right: card.left + (7.0 * k).round() as i32 + t,
                    bottom: card.top + (7.0 * k).round() as i32 + t,
                };
                fill(hdc, &bd, if mark && !on { s.skin.mark } else { s.skin.acc });
                rring(hdc, &bd, (4.0 * k).round() as i32, mix(s.skin.acc, s.skin.ink_inv, 0.25), None);
                tick(hdc, (bd.left + bd.right) / 2, (bd.top + bd.bottom) / 2, s.skin.ink_inv, k);
            }
        } else if hl {
            let q = RECT { left: boxr.left + pad2, top: boxr.top + pad2, right: boxr.right - pad2, bottom: boxr.bottom - pad2 };
            frame(hdc, &q, s.skin.acc);
            let q2 = RECT { left: q.left + 1, top: q.top + 1, right: q.right - 1, bottom: q.bottom - 1 };
            frame(hdc, &q2, s.skin.acc);
        } else if mark {
            // 缩略图模式也要看得见选区：双边框 + 左上角勾选块。
            // 角标放在左上，免得和右上角的类别角标叠在一起。
            let q = RECT { left: boxr.left + pad2, top: boxr.top + pad2, right: boxr.right - pad2, bottom: boxr.bottom - pad2 };
            frame(hdc, &q, s.skin.acc);
            let q2 = RECT { left: q.left + 1, top: q.top + 1, right: q.right - 1, bottom: q.bottom - 1 };
            frame(hdc, &q2, s.skin.acc);
            // 素面档的文件夹不挂左上角那枚勾：这一档只有这一处角标，双边框已经把
            // "标过了"说清楚，勾在文件夹上没别的用途。
            if !is_dir {
                let t = (13.0 * k).round().max(9.0) as i32;
                let bd = RECT { left: q.left, top: q.top, right: q.left + t, bottom: q.top + t };
                fill(hdc, &bd, s.skin.acc);
                if bands(s) {
                    tick(hdc, (bd.left + bd.right) / 2, (bd.top + bd.bottom) / 2, s.skin.ink_inv, k * 0.9);
                } else {
                    txt(hdc, &bd, "✓", s.skin.ink_inv, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
                }
            }
        } else if bands(s) {
            // 平阶：选中一圈 accent + 里面一道更淡的同色环；悬停只给一道弱边
            if sel {
                frame(hdc, &boxr, s.skin.acc);
                let q = RECT { left: boxr.left + 1, top: boxr.top + 1, right: boxr.right - 1, bottom: boxr.bottom - 1 };
                frame(hdc, &q, mix(s.skin.canvas, s.skin.acc, 0.26));
            } else if hot {
                let q = RECT { left: boxr.left + 1, top: boxr.top + 1, right: boxr.right - 1, bottom: boxr.bottom - 1 };
                frame(hdc, &q, s.skin.line);
            }
        } else if sel {
            let q = RECT { left: boxr.left + pad2, top: boxr.top + pad2, right: boxr.right - pad2, bottom: boxr.bottom - pad2 };
            frame(hdc, &q, s.skin.acc);
        } else if hot {
            let q = RECT { left: boxr.left + pad2, top: boxr.top + pad2, right: boxr.right - pad2, bottom: boxr.bottom - pad2 };
            frame(hdc, &q, s.skin.acc_soft);
        }
        // 挂在类别里的图：角标钉在缩略图盒的右上角，取消勾选后这里就没了。
        // 以前跟着实际出图的右上角走，竖图横图各飘一处，一屏扫过去勾不成一排。
        if !is_dir && a.cat.as_ref().map(|c| c.any_cat(&a.entries[i].path)).unwrap_or(false) {
            let t = (13.0 * k).round().max(9.0) as i32;
            let tr = RECT { left: pic_in.right - t, top: pic_in.top, right: pic_in.right, bottom: pic_in.top + t };
            fill(hdc, &tr, s.skin.mark);
            txt(hdc, &tr, "✓", s.skin.ink_inv, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
        }
        // 名字带垫着的那一层色：绘制各处读同一个值，不在几处各算一遍。
        let wash = if mark { tinted(s.skin.acc_soft) } else { tinted(s.skin.hov) };
        let fine_foot = mix(tinted(s.skin.pane), wash, 0.18);
        let plate = if on {
            s.skin.sel
        } else if mark {
            s.skin.acc_soft
        } else if hot {
            s.skin.hov
        } else {
            mix(s.skin.pane, s.skin.canvas, 0.35)
        };
        // 尺寸行底下那块纯色：卡片档整格铺的是 pane，老版完整卡片那档下半张另铺一块卡尾色。
        // 没铺卡面的那几格（素卡上的文件夹、极简档）字直接坐在柔光地面上，底不是一色；
        // 列表和详细两档的行底是选中/悬停那套，也不是 pane —— 这几种都照旧现排。
        let dim_bk = if listy || !plated {
            NO_BK
        } else if rich {
            tinted(s.skin.pane)
        } else {
            s.skin.pane
        };
        // 尺寸行：只写真实信息，没出图就留空
        let dimr = RECT {
            left: r.left,
            top: boxr.bottom,
            right: r.right,
            bottom: boxr.bottom + dim_h,
        };
        let dim_t = if is_dir {
            String::new()
        } else if let Some((w, h)) = a.dims.get(&i).copied().filter(|(w, _)| *w > 0) {
            let ext = Path::new(&a.entries[i].path)
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_ascii_uppercase();
            match a.bpps.get(&i).copied().unwrap_or(0) {
                0 => format!("{w} × {h} · {ext}"),
                b => format!("{w} × {h} · {b}"),
            }
        } else if has_img || !a.wants_thumb() || a.got + a.err < n_imgs {
            // 还没出图的那一格什么都不写：进度已经在下面那条状态栏里报了，
            // 图上一格一个"生成中"只会把尺寸行刷成一片闪动的字。
            String::new()
        } else {
            "无预览".to_string()
        };
        tl_dim.push(GridText {
            r: dimr,
            s: dim_t,
            bk: dim_bk,
            // 左右让开格子缝和卡片描边那两列，位图只盖卡片内部。
            pad: edge + 2,
            c: if rich && on {
                s.skin.sel_ink
            } else if rich && hot {
                s.skin.ink_dim
            } else if bands(s) {
                if on { s.skin.ink_dim } else { s.skin.ink_off }
            } else {
                s.skin.ink_dim
            },
        });
        // 名字条：极简档还是那块带边框的牌；平阶往上把牌拆掉，字直接坐在格子里
        let nm = RECT {
            left: r.left + edge,
            top: boxr.bottom + dim_h,
            right: r.right - edge,
            bottom: boxr.bottom + dim_h + lab_h,
        };
        if bands(s) {
            let nc = if fine {
                if on {
                    // 名字带换成标题栏那档主色，字色按对比度跟着标题栏走。
                    let bar = s.skin.acc;
                    if theme::contrast(s.skin.ink, bar) >= theme::contrast(s.skin.ink_inv, bar) {
                        s.skin.ink
                    } else {
                        s.skin.ink_inv
                    }
                } else if mark || hot {
                    s.skin.ink
                } else {
                    s.skin.ink_dim
                }
            } else if rich {
                if on || mark {
                    s.skin.sel_ink
                } else if hot {
                    s.skin.ink
                } else {
                    s.skin.ink_dim
                }
            } else if on {
                if cards(s) { s.skin.ink } else { s.skin.sel_ink }
            } else if hot {
                s.skin.ink
            } else {
                s.skin.ink_dim
            };
            // 名字带底下那块纯色：选中那档精密仪器是主色渐变、素卡文件夹坐在柔光地面上，
            // 这两种底不是一色，照旧现排；其余都能整块存成小位图。
            let nm_bk = if !plated || (fine && on) {
                NO_BK
            } else if fine && (mark || hot) {
                fine_foot
            } else if rich {
                plate
            } else {
                s.skin.pane
            };
            let nm_line = if nm_bk == NO_BK {
                NO_BK
            } else if fine {
                if mark {
                    mix(tinted(s.skin.pane), s.skin.acc, 0.54)
                } else {
                    s.skin.line_soft
                }
            } else if rich {
                if on || mark { mix(plate, s.skin.acc, 0.62) } else { s.skin.line_soft }
            } else {
                NO_BK
            };
            let nm_uline = if nm_bk != NO_BK && quiet(s) && (on || mark) { s.skin.acc } else { NO_BK };
            let uw = (((nm.right - nm.left) as f64) * 0.30).round() as i32;
            let uh = (2.0 * k).round().max(1.0) as i32;
            let cached = nm_bk != NO_BK;
            if !cached {
                if fine {
                    let keep = round_clip(hdc, &card, card_rad(s));
                    if on {
                        // 名字带直接用标题栏那一档主色，铺纯色。
                        fill(hdc, &nm, s.skin.acc);
                    }
                    unround(hdc, keep);
                }
            }
            if !cached {
                if rich {
                    // 名称带不是一块透明字：它有自己的底、顶线和状态色。
                    let keep = round_clip(hdc, &card, card_rad(s));
                    fill(hdc, &nm, plate);
                    hline(
                        hdc,
                        nm.left,
                        nm.right,
                        nm.top,
                        if on || mark { mix(plate, s.skin.acc, 0.62) } else { s.skin.line_soft },
                    );
                    unround(hdc, keep);
                }
                if quiet(s) && (on || mark) {
                    // 这道下划线只归静谧玻璃；制图台靠四角刻度，不重复同一个记号。
                    let ux = (nm.left + nm.right) / 2;
                    fill(
                        hdc,
                        &RECT {
                            left: ux - uw / 2,
                            top: nm.bottom - uh - 1,
                            right: ux + uw / 2,
                            bottom: nm.bottom - 1,
                        },
                        s.skin.acc,
                    );
                }
            }
            // 稿子 .nm 有 padding:0 3px：不留这 3px，省略号末尾的字就压在卡片环上
            let q = (3.0 * k).round() as i32;
            let nmt = RECT { left: nm.left + q, top: nm.top, right: nm.right - q, bottom: nm.bottom };
            let bold = (rich || fine) && (on || mark);
            if a.ren_idx != i {
                if cached {
                    tl_band.push(Band {
                        r: nm,
                        s: name.clone(),
                        c: nc,
                        font: if bold { a.font_b } else { a.font },
                        bold,
                        bk: nm_bk,
                        line: nm_line,
                        uline: nm_uline,
                        uw,
                        uh,
                        q,
                        card,
                        rad: card_rad(s),
                    });
                } else {
                    let g = GridText { r: nmt, s: name.clone(), c: nc, bk: NO_BK, pad: 0 };
                    if bold {
                        tl_bd.push(g);
                    } else {
                        tl_nm.push(g);
                    }
                }
            }
        } else {
            fill(hdc, &nm, if on { s.skin.sel } else { s.skin.chrome_hi });
            frame(hdc, &nm, if on || hot { s.skin.acc } else { s.skin.line_soft });
            if a.ren_idx != i {
                let g = GridText {
                    r: nm,
                    s: name.clone(),
                    c: if on { s.skin.sel_ink } else { s.skin.ink },
                    // 极简档那块名字牌是带边框的，整块搬会把边框盖掉，先照旧现排。
                    bk: NO_BK,
                    pad: 0,
                };
                if on {
                    tl_bd.push(g);
                } else {
                    tl_nm.push(g);
                }
            }
        }
        // 文件夹格按稿子不做成卡片，平时一圈都不画；选中/悬停/标记才补一枚圆环当选框，
        // 内圈是给卡片那种 pane 底用的，格子底是柔光地面，套上去会脏，所以不给。
        if plated {
            let (ring, inner_ring) = if on || mark {
                (
                    if on {
                        if quiet(s) {
                            mix(s.skin.acc, s.skin.line_soft, 0.42)
                        } else {
                            s.skin.acc
                        }
                    } else {
                        s.skin.mark
                    },
                    mix(if on { s.skin.sel } else { s.skin.acc_soft }, s.skin.acc, 0.42),
                )
            } else if hot {
                if fine {
                    (mix(s.skin.line, s.skin.acc, 0.42), mix(s.skin.hov, s.skin.acc, 0.22))
                } else if draft(s) {
                    (mix(s.skin.line, s.skin.acc, 0.38), mix(s.skin.hov, s.skin.acc, 0.18))
                } else if quiet(s) {
                    (mix(s.skin.line_soft, s.skin.acc, 0.34), mix(s.skin.hov, s.skin.acc, 0.16))
                } else {
                    (s.skin.focus, mix(s.skin.hov, s.skin.acc, 0.32))
                }
            } else if rich || fine {
                (s.skin.line_soft, mix(s.skin.pane, s.skin.canvas, 0.55))
            } else if draft(s) {
                (s.skin.line, mix(s.skin.pane, s.skin.canvas, 0.62))
            } else {
                (s.skin.line_soft, mix(s.skin.pane, s.skin.canvas, 0.62))
            };
            let rad = card_rad(s);
            // 制图台的文件夹只剩四角刻度那一层：整圈框叠四角框读起来是两层框。
            if !(draft(s) && is_dir) {
                rring(
                    hdc,
                    &card,
                    rad,
                    ring,
                    if (is_dir && !rich && !fine) || (fine && on) || quiet(s) || draft(s) {
                        None
                    } else {
                        Some(inner_ring)
                    },
                );
            }
            if draft(s) && (on || hot || mark) {
                // 四角刻度、选框、名称下面那一条全走配色主色，版式不自带颜色。
                let t = (8.0 * k).round().max(5.0) as i32;
                let c = if on {
                    s.skin.acc
                } else if mark {
                    s.skin.mark
                } else {
                    ring
                };
                for (x, y, dx, dy) in [
                    (card.left + 2, card.top + 2, 1, 1),
                    (card.right - 2, card.top + 2, -1, 1),
                    (card.left + 2, card.bottom - 2, 1, -1),
                    (card.right - 2, card.bottom - 2, -1, -1),
                ] {
                    hline(hdc, x.min(x + dx * t), x.max(x + dx * t), y, c);
                    vline(hdc, x, y.min(y + dy * t), y.max(y + dy * t), c);
                }
            }
        }
    }
    // 格子画完才补字：同一种字体连着画到底，字体切换从一帧一百多次降到三次。
    // 字都坐在自己的带子里，跟圆环、角标互不重叠，挪到最后画看不出来。
    TXT_SLOT.store(5, Ordering::Relaxed);
    for t in &tl_dim {
        dim_text(hdc, &t.r, &t.s, t.c, a.font_num, t.bk, t.pad);
    }
    TXT_SLOT.store(3, Ordering::Relaxed);
    for b in &tl_band {
        band_paint(hdc, b);
    }
    for t in &tl_nm {
        txt(
            hdc,
            &t.r,
            &t.s,
            t.c,
            a.font,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
    }
    for t in &tl_bd {
        txt(
            hdc,
            &t.r,
            &t.s,
            t.c,
            a.font_b,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
    }
    submark!(1);
    if sub_on {
        let deep1 = deep_snap();
        SUB_DEEP.store(false, Ordering::Relaxed);
        for i in 0..7 {
            sub_acc[3 + i] = deep1[i].saturating_sub(deep0[i]);
        }
    }
    // 列头在 grid 之外（布局时从列表顶上扣出去的那一条），必须先解除列表的裁剪，
    // 不然整条表头落在裁剪区外，一个字也画不出来。
    unclip(hdc);
    if a.mode == Mode::Detail {
        let h = detail_head(z, s);
        fill(hdc, &h, s.skin.pane_head);
        // 列标题、点它排哪一档、箭头画在哪一列，全在这一张表里，别拆成两处。
        // 名称那一列的标题顶到最左边：数据前面让给了图标，标题前面不该再空一截。
        let nx0 = h.left + (6.0 * s.scale) as i32;
        let heads = [
            (nx0, cx.typ - 4, "名称", SortKey::Name),
            (cx.typ, cx.dim - 4, "类型", SortKey::Type),
            (cx.dim, cx.date - 4, "尺寸", SortKey::Image),
            (cx.date, cx.size - 4, "修改时间", SortKey::Date),
            (cx.size, cx.end, "大小", SortKey::Size),
        ];
        for (x0, x1, label, key) in heads {
            let right = key == SortKey::Size;
            // 定宽列加起来的宽可能超过可见区：文字框裁到表头右缘，别画进滚动条那列。
            let x1 = x1.min(h.right).max(x0);
            let r = RECT { left: x0, top: h.top, right: x1, bottom: h.bottom };
            let f = if right { DT_RIGHT } else { DT_LEFT };
            txt(hdc, &r, label, s.skin.ink, a.font_sm, f | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
            if a.sort == key {
                // 箭头贴着标题文字走：右对齐那一列放文字左边，其余放右边。
                // 两头都贴列的右缘就会叠在字上（"大小"那列就是这么糊成一团的）。
                let aw = (16.0 * s.scale) as i32;
                let tw = text_w(hdc, label, a.font_sm);
                let ax = if right {
                    (x1 - tw - 2 - aw).max(x0)
                } else {
                    (x0 + tw + 2).clamp(x0, (x1 - aw).max(x0))
                };
                let ar = RECT { left: ax, top: h.top, right: ax + aw, bottom: h.bottom };
                txt(hdc, &ar, if a.sort_desc { "▼" } else { "▲" }, s.skin.acc, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            }
        }
        // 列边界：按住往左右拖就改这条线左边那一列的宽，右边那几列只挪位置、宽度不动。
        // 指针停在线上、或正在拖这条时，线亮起来。五条线各管一列，最后一条（大小右缘）
        // 跟窗口右缘重合时也照样画、照样能拖——想把大小列收窄就按它。
        let hw = (3.0 * s.scale) as i32;
        for (i, ex) in [cx.typ, cx.dim, cx.date, cx.size, cx.end].iter().enumerate() {
            if *ex > cx.right {
                continue;
            }
            let strip = RECT { left: *ex - hw, top: h.top, right: *ex + hw, bottom: h.bottom };
            let hot = a.col_drag.map(|(d, _, _)| d == i).unwrap_or(false) || over(&strip, a.pt.0, a.pt.1);
            vline(hdc, *ex, h.top + 2, h.bottom - 3, if hot { s.skin.acc } else { s.skin.line });
        }
        hline(hdc, h.left, h.right, h.bottom - 1, s.skin.line);
    }
    // 竖滚动条：槽常驻，滑块只在内容超出时出现
    let sbg = z.gsb;
    let grab = over(&sbg, a.pt.0, a.pt.1) || a.grid_drag.is_some();
    if bands(s) {
        fill(hdc, &sbg, s.skin.canvas);
        if let Some((track, thumb, _)) = grid_sb(sbg, a.max_scroll(g), a.scroll, s.scale) {
            fill(hdc, &track, s.skin.canvas);
            fill(hdc, &thumb, if grab { s.skin.focus } else { s.skin.line });
        }
    } else {
        fill(hdc, &sbg, s.skin.win);
        vline(hdc, sbg.left, g.top, g.bottom, s.skin.line_soft);
        if let Some((track, thumb, _)) = grid_sb(sbg, a.max_scroll(g), a.scroll, s.scale) {
            fill(hdc, &track, s.skin.win);
            fill(hdc, &thumb, if grab { s.skin.acc } else { s.skin.chrome_hi });
        }
    }
    // 外部的文件拖到列表上：整圈描边，表示"松手就搬进当前文件夹"
    if matches!(a.drop_hl, Some(Hl::Here)) {
        frame(hdc, &g, s.skin.acc);
    }
    submark!(2);
    if sub_on {
        if let Ok(mut t) = LAST_SUB.lock() {
            *t = sub_acc;
        }
        sub_add(&sub_acc);
    }
}

/// 左栏下半的预览格：棋盘底、居中图片、底部两段说明（尺寸 / 类型大小）。
unsafe fn paint_pv(hdc: HDC, a: &mut App, z: &Zones, s: &Theme) {
    a.zone(theme::Z_PV);
    let g = z.pv;
    if g.right - g.left < 8 || g.bottom - g.top < 8 {
        return;
    }
    clip(hdc, &g);
    fill(hdc, &g, s.skin.canvas);
    let k = s.scale;
    pane_head(hdc, &g, "预览", M_PANE_PV, a, s);
    let head_h = s.pane_head_h;
    let cap_h = s.meta + (4.0 * k).round() as i32;
    let imgbox = RECT {
        left: g.left + (4.0 * k) as i32,
        top: g.top + head_h + (4.0 * k) as i32,
        right: g.right - (4.0 * k) as i32,
        bottom: g.bottom - cap_h - (4.0 * k) as i32,
    };
    if imgbox.bottom - imgbox.top < (24.0 * k) as i32 {
        unclip(hdc);
        return;
    }
    a.pv_w = imgbox.right - imgbox.left;
    if bands(s) {
        // 棋盘格在深蓝上读成“没加载完”：铺平，图自己带一圈边和一道落地影就立住了
        fill(hdc, &imgbox, s.skin.canvas);
    } else {
        checker(hdc, &imgbox, s.skin.canvas, s.skin.grid, (8.0 * k).round().max(2.0) as i32);
    }
    let hit = pv()
        .lock()
        .unwrap()
        .as_ref()
        .filter(|t| t.0 == a.stamp)
        .map(|t| (t.1, t.2, t.3.clone(), t.4, t.5, t.6));
    let cap_l;
    let mut cap_r = String::new();
    if let Some((w, h, px, nw, nh, bpp)) = hit {
        let img = Img { w, h, px, fit: None };
        let d = box_in(w, h, imgbox);
        if bands(s) {
            soft_shadow(hdc, &d, (4.0 * k).round() as i32);
        }
        dib(hdc, &d, &img);
        frame(hdc, &d, if fine(s) { s.skin.line_soft } else { s.skin.line });
        cap_l = format!("{nw} × {nh}");
        if let Some(e) = a.entries.get(a.sel) {
            let ext = Path::new(&e.path)
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_ascii_uppercase();
            cap_r = if bpp > 0 { format!("{bpp} 位") } else { ext };
        }
    } else if a.entries.get(a.sel).map(|e| e.is_dir).unwrap_or(false) {
        let isz = (72.0 * k).round() as i32;
        icon(
            hdc,
            Ik::Folder,
            imgbox.left + (imgbox.right - imgbox.left - isz) / 2,
            imgbox.top + (imgbox.bottom - imgbox.top - isz) / 2,
            isz,
            s.skin.acc,
            if bands(s) { s.skin.canvas } else { s.skin.grid },
        );
        cap_l = "文件夹".to_string();
    } else if a.entries.get(a.sel).map(|e| !e.is_image).unwrap_or(false) {
        let isz = (72.0 * k).round() as i32;
        icon(
            hdc,
            Ik::File,
            imgbox.left + (imgbox.right - imgbox.left - isz) / 2,
            imgbox.top + (imgbox.bottom - imgbox.top - isz) / 2,
            isz,
            s.skin.acc,
            if bands(s) { s.skin.canvas } else { s.skin.grid },
        );
        cap_l = "非图片文件".to_string();
        cap_r = a
            .entries
            .get(a.sel)
            .and_then(|e| Path::new(&e.path).extension().and_then(|x| x.to_str()))
            .unwrap_or("")
            .to_ascii_uppercase();
    } else if a.sel == usize::MAX {
        cap_l = "没有选中条目".to_string();
    } else {
        cap_l = "取图中…".to_string();
    }
    let cy = g.bottom - cap_h;
    hline(hdc, g.left, g.right, cy, s.skin.line_soft);
    let capc = if bands(s) { s.skin.ink_off } else { s.skin.ink_dim };
    let lr = RECT { left: g.left + (5.0 * k) as i32, top: cy, right: g.right / 2, bottom: g.bottom };
    txt(hdc, &lr, &cap_l, capc, a.font_num, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
    let rr = RECT { left: g.right / 2, top: cy, right: g.right - (5.0 * k) as i32, bottom: g.bottom };
    txt(hdc, &rr, &cap_r, capc, a.font_num, DT_RIGHT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
    unclip(hdc);
    hline(hdc, g.left, g.right, g.bottom - 1, if fine(s) { s.skin.line_soft } else { s.skin.line });
}

unsafe fn dot(hdc: HDC, cx: i32, cy: i32, r: i32, col: u32) {
    let (ob, op, br) = ibrush(hdc, col);
    let _ = Ellipse(hdc, cx - r, cy - r, cx + r, cy + r);
    irestore(hdc, ob, op, br);
}

/// 列表底下那条后台进度：出图速率、缓存命中。底色是 acc_soft，和设计稿一致。
unsafe fn paint_hint(hdc: HDC, a: &mut App, z: &Zones, s: &Theme) {
    a.zone(theme::Z_GRID);
    let g = z.hint;
    // 提示条比选中还亮会抢焦点：压到悬停色那一档，字退一档，圆点收成方点
    fill(hdc, &g, if bands(s) { s.skin.hov } else { s.skin.acc_soft });
    hline(hdc, g.left, g.right, g.top, s.skin.line_soft);
    a.page_btns.clear();
    let k = s.scale;
    let mut right = g.right - (6.0 * k) as i32;
    if a.src == Src::Search && !a.search_files.is_empty() {
        let pages = a.page_count();
        let pad = (10.0 * k) as i32;
        let label = format!("共 {} 张 · 第 {}/{} 页", a.search_files.len(), a.page + 1, pages);
        let lw = text_w(hdc, &label, a.font_sm);
        let lr = RECT { left: right - lw, top: g.top, right: right, bottom: g.bottom };
        txt(hdc, &lr, &label, if bands(s) { s.skin.ink_dim } else { s.skin.ink }, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
        right = lr.left - pad;
        let bh = (g.bottom - g.top - 4).max(12);
        let by = g.top + (g.bottom - g.top - bh) / 2;
        for (id, text, on) in [
            (M_PAGE_NEXT, "下一页 ›", a.page + 1 < pages),
            (M_PAGE_PREV, "‹ 上一页", a.page > 0),
        ] {
            let w = text_w(hdc, text, a.font_sm) + (12.0 * k) as i32;
            let r = RECT { left: right - w, top: by, right: right, bottom: by + bh };
            let hot = on && over(&r, a.pt.0, a.pt.1);
            fill(hdc, &r, if hot { s.skin.hov } else { s.skin.chrome_hi });
            frame(hdc, &r, if on { s.skin.line } else { s.skin.line_soft });
            txt(
                hdc,
                &r,
                text,
                if on { s.skin.ink } else { s.skin.ink_off },
                a.font_sm,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER,
            );
            if on {
                a.page_btns.push((id, r));
            }
            right = r.left - (4.0 * k) as i32;
        }
    }
    let n = a.pending_n();
    let done = a.got + a.err;
    // 张/秒跟着本池开跑时刻（改高度重排后要重新计），用时跟着进目录的时刻。
    let rate_sec = a.t0.elapsed().as_secs_f64().max(0.001);
    let cy = (g.top + g.bottom) / 2;
    let r = ((6.0 * s.scale) / 2.0).round().max(2.0) as i32;
    let all = a.ready_sec.is_some() || (n > 0 && done >= n);
    let used = a.ready_sec.unwrap_or_else(|| a.t_dir.elapsed().as_secs_f64());
    let dx = g.left + (6.0 * s.scale) as i32 + r;
    if bands(s) {
        fill(hdc, &RECT { left: dx - r + 1, top: cy - r + 1, right: dx + r, bottom: cy + r }, s.skin.acc);
    } else {
        dot(hdc, dx, cy, r, s.skin.acc);
    }
    let text = if !a.wants_thumb() {
        // 图标视图、列表视图只画图标、不开池子，报"已完成 0 / N"只会让人以为后台卡住了。
        let listed = a.entries.len() - usize::from(a.has_up);
        let name = if a.mode == Mode::List { "列表视图" } else { "图标视图" };
        format!("{name}不生成缩略图 · 本目录列了 {listed} 个文件")
    } else if a.mode == Mode::Detail {
        // 详细资料也只画图标，后台这一趟是为了"尺寸"那一列，别报成在生成缩略图。
        if all {
            format!("图片尺寸已读取 {n} 张 · 用时 {used:.1} 秒")
        } else {
            format!(
                "后台读取图片尺寸：已完成 {} / {n} · 约 {:.0} 张/秒",
                a.got,
                a.got as f64 / rate_sec
            )
        }
    } else if all {
        format!("缩略图就绪 {n} 张 · 用时 {used:.1} 秒 · 本目录命中 {} 张（零解码）", a.dir_hits())
    } else {
        format!(
            "后台生成缩略图：已完成 {} / {n} · 约 {:.0} 张/秒 · 本目录命中 {} 张（零解码）",
            a.got,
            a.got as f64 / rate_sec,
            a.dir_hits()
        )
    };
    let text = if a.msg.is_empty() { text } else { format!("{text} · {}", a.msg) };
    let tr = RECT {
        left: g.left + (6.0 * s.scale) as i32 + r * 2 + (6.0 * s.scale) as i32,
        top: g.top,
        right: (right - (4.0 * s.scale) as i32).max(g.left),
        bottom: g.bottom,
    };
    txt(hdc, &tr, &text, if bands(s) { s.skin.ink_dim } else { s.skin.ink }, a.font_sm, DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
}

fn kind_label(p: &Path) -> String {
    let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("").to_ascii_uppercase();
    match ext.as_str() {
        "JPG" | "JPE" => "JPEG".to_string(),
        "TIF" => "TIFF".to_string(),
        other => other.to_string(),
    }
}

/// 状态栏六段：总数 / 已选 / 修改时间 / 文件名 / 尺寸类型 / 缩略图进度。
/// 段与段之间是右边框，第四段吃掉剩余宽度。
unsafe fn paint_status(hdc: HDC, a: &mut App, z: &Zones, s: &Theme) {
    a.zone(theme::Z_STATUS);
    let g = z.status;
    if g.right - g.left < 8 || g.bottom - g.top < 8 {
        return;
    }
    if bands(s) {
        shell(hdc, &g, s, s.skin.chrome, false, true);
    } else {
        fill(hdc, &g, s.skin.chrome);
        hline(hdc, g.left, g.right, g.top, s.skin.line);
    }
    let k = s.scale;
    let imgs = a.n_images();
    // 图标/列表/详细资料连非图片一起列，总数得跟屏幕上的条目对上。
    let listed = a.entries.len() - usize::from(a.has_up);
    let count = if a.mode.all_files() { listed } else { imgs };
    let seg1 = match a.total_sz {
        Some(t) => format!("总共 {count} 个文件 ({})", fmt_size(t)),
        None => format!("总共 {count} 个文件"),
    };
    let sel_sz = a
        .entries
        .get(a.sel)
        .filter(|e| !e.is_dir)
        .map(|e| stat_of(&e.path).0);
    // 光标那一项如果已经在多选集合里就不能再数一遍，否则全选 1043 张会写成"已选 1044"。
    let n_sel = a.marked.len()
        + match a.entries.get(a.sel) {
            Some(e) if !a.marked.contains(&e.path) => 1,
            _ => 0,
        };
    let seg2 = match sel_sz {
        Some(t) if n_sel > 0 => format!("已选 {n_sel} 个 · {}", fmt_size(t)),
        _ if n_sel > 0 => format!("已选 {n_sel} 个"),
        _ => "未选择".to_string(),
    };
    let seg3 = a.entries.get(a.sel).map(|e| stat_of(&e.path).1).unwrap_or_default();
    let seg4 = a.entries.get(a.sel).map(|_| a.disp(a.sel)).unwrap_or_default();
    let seg5 = match a.entries.get(a.sel) {
        None => String::new(),
        Some(e) if e.is_dir => "文件夹".to_string(),
        Some(e) => {
            let kind = kind_label(Path::new(&e.path));
            let bpp = a.bpps.get(&a.sel).copied().unwrap_or(0);
            match a.dims.get(&a.sel).copied() {
                Some((w, h)) if w > 0 => {
                    if bpp > 0 {
                        format!("{w} × {h} · {bpp} 位 {kind}")
                    } else {
                        format!("{w} × {h} · {kind}")
                    }
                }
                _ => kind,
            }
        }
    };
    // 图标档、列表档没有"要生成的缩略图"，进度条按满格画，别停在 0% 让人以为卡住了。
    let pct = if !a.wants_thumb() || imgs == 0 {
        1.0
    } else {
        ((a.got + a.err) as f64 / imgs as f64).clamp(0.0, 1.0)
    };

    let pad = (7.0 * k).round() as i32;
    let gap = (4.0 * k) as i32;
    let bar_w = (120.0 * k).round() as i32;
    let seg = |text: &str, font: HFONT| -> i32 {
        if text.is_empty() { 0 } else { text_w(hdc, text, font) + pad * 2 }
    };
    // 从右往左：进度条 / 尺寸类型
    let x6 = g.right - bar_w;
    let w5 = seg(&seg5, a.font_num).max(0);
    let x5 = x6 - w5;
    let mut x = g.left;
    for (text, font, ink) in [
        (&seg1, a.font_num, s.skin.ink_dim),
        (&seg2, a.font_num, s.skin.ink_dim),
        (&seg3, a.font_num, s.skin.ink_dim),
    ] {
        let w = seg(text, font);
        if w == 0 {
            continue;
        }
        let r = RECT { left: x, top: g.top, right: (x + w).min(g.right), bottom: g.bottom };
        txt(hdc, &r, text, ink, font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
        stsep(hdc, r.right, &g, s);
        x = r.right;
    }
    // 第四段：文件名，吃满中间
    let fr = RECT { left: x + pad, top: g.top, right: (x5 - pad).max(x + pad + 10), bottom: g.bottom };
    txt(hdc, &fr, &seg4, s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_PATH_ELLIPSIS);
    stsep(hdc, fr.right + pad, &g, s);
    if !seg5.is_empty() {
        let r5 = RECT { left: x5, top: g.top, right: x6, bottom: g.bottom };
        txt(hdc, &r5, &seg5, s.skin.ink_dim, a.font_num, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
        stsep(hdc, x6, &g, s);
    }
    // 第六段：缩略图进度条
    let lab = text_w(hdc, "缩略图", a.font_sm);
    let r6 = RECT { left: x6 + (2.0 * k) as i32, top: g.top, right: g.right, bottom: g.bottom };
    txt(hdc, &r6, "缩略图", if bands(s) { s.skin.ink_off } else { s.skin.ink_dim }, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    let th = ((6.0 * k).round() as i32).max(4);
    let tx = x6 + (2.0 * k) as i32 + lab + gap;
    let tr = RECT {
        left: tx,
        top: ((g.top + g.bottom) / 2) - th / 2,
        right: (g.right - (4.0 * k) as i32).max(tx + 8),
        bottom: ((g.top + g.bottom) / 2) + th / 2,
    };
    if fine(s) {
        let sl = (4.0 * k).round().max(3.0) as i32;
        let sy = (g.top + g.bottom) / 2;
        let sr = RECT { left: tr.left, top: sy - sl / 2, right: tr.right, bottom: sy - sl / 2 + sl };
        let rad = sl / 2;
        let keep = round_clip(hdc, &sr, rad);
        fill(hdc, &sr, mix(s.skin.canvas, s.skin.line_soft, 0.25));
        unround(hdc, keep);
        rring(hdc, &sr, rad, s.skin.line_soft, None);
        let fw = ((sr.right - sr.left - 2) as f64 * pct).round() as i32;
        let fr = RECT { left: sr.left + 1, top: sr.top + 1, right: sr.left + 1 + fw, bottom: sr.bottom - 1 };
        if fw > 1 {
            let keep = round_clip(hdc, &fr, rad.saturating_sub(1).max(1));
            fill(hdc, &fr, s.skin.acc);
            unround(hdc, keep);
            hline(hdc, fr.left + 1, fr.right - 1, fr.top + 1, mix(s.skin.acc, s.skin.chrome_hi, 0.34));
        }
    } else if bands(s) {
        // 槽收成 3px：一圈边框在 6px 高的槽上只剩两条线，读不出“槽”
        let sl = (3.0 * k).round().max(2.0) as i32;
        let sy = (g.top + g.bottom) / 2;
        let sr = RECT { left: tr.left, top: sy - sl / 2, right: tr.right, bottom: sy - sl / 2 + sl };
        fill(hdc, &sr, s.skin.canvas);
        frame(hdc, &sr, s.skin.line_soft);
        let fw = ((sr.right - sr.left - 2) as f64 * pct).round() as i32;
        fill(hdc, &RECT { left: sr.left + 1, top: sr.top + 1, right: sr.left + 1 + fw, bottom: sr.bottom - 1 }, s.skin.acc);
    } else {
        fill(hdc, &tr, s.skin.canvas);
        frame(hdc, &tr, s.skin.line);
        let fw = ((tr.right - tr.left - 2) as f64 * pct).round() as i32;
        let fr6 = RECT { left: tr.left + 1, top: tr.top + 1, right: tr.left + 1 + fw, bottom: tr.bottom - 1 };
        fill(hdc, &fr6, s.skin.acc);
    }
}

/// 状态栏段与段之间那道分隔：整高的一条竖线在深蓝上排成一排栅栏，
/// 收到上下各让 4px 的一小段，才读成“相邻两段的缝”。
unsafe fn stsep(hdc: HDC, x: i32, g: &RECT, s: &Theme) {
    if bands(s) {
        let t = (4.0 * s.scale).round() as i32;
        vline(hdc, x, g.top + t, g.bottom - t, s.skin.line_soft);
    } else {
        vline(hdc, x, g.top, g.bottom, s.skin.line_soft);
    }
}

fn flat_tree(nodes: &[Node], depth: usize, out: &mut Vec<(usize, String, bool, String)>) {
    for n in nodes {
        out.push((depth, n.path.clone(), n.open, n.name.clone()));
        if n.open {
            flat_tree(&n.kids, depth + 1, out);
        }
    }
}

/// 树里某个文件夹的内容变了（搬进、搬出、改名）：把它那一层的子节点重扫一遍。
/// 节点没在树里（没展开过）就找不到，也就不用管。
fn tree_reload(nodes: &mut Vec<Node>, path: &str) {
    let want = path.trim_end_matches('\\');
    if want.is_empty() {
        return;
    }
    for n in nodes.iter_mut() {
        if n.path.trim_end_matches('\\').eq_ignore_ascii_case(want) {
            n.kids = sub_dirs(&n.path);
            n.loaded = true;
            return;
        }
        tree_reload(&mut n.kids, path);
    }
}

fn sub_dirs(path: &str) -> Vec<Node> {
    scan::list(path, false)
        .iter()
        .filter(|e| e.is_dir)
        .map(|e| Node {
            path: format!("{}\\", e.path),
            name: stem(&e.path).to_string(),
            open: false,
            loaded: false,
            kids: Vec::new(),
        })
        .collect()
}

/// 按扁平行号开合节点，计数顺序和 flat_tree 一致（先自己、再展开的子项）。
fn toggle_at(v: &mut Vec<Node>, want: usize, seq: &mut usize) -> bool {
    for n in v.iter_mut() {
        if *seq == want {
            if !n.loaded {
                n.kids = sub_dirs(&n.path);
                n.loaded = true;
            }
            n.open = !n.open;
            return true;
        }
        *seq += 1;
        if n.open && toggle_at(&mut n.kids, want, seq) {
            return true;
        }
    }
    false
}

fn vol_name(root: &str) -> String {
    let letter = root.chars().next().unwrap_or('C');
    let rootw: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut name = [0u16; 64];
    let mut fs = [0u16; 16];
    let (mut serial, mut maxlen, mut flags) = (0u32, 0u32, 0u32);
    let ok = unsafe {
        windows::Win32::Storage::FileSystem::GetVolumeInformationW(
            PCWSTR(rootw.as_ptr()),
            Some(&mut name),
            Some(&mut serial),
            Some(&mut maxlen),
            Some(&mut flags),
            Some(&mut fs),
        )
    };
    let n = name.iter().position(|&c| c == 0).unwrap_or(0);
    let label = if ok.is_ok() { String::from_utf16_lossy(&name[..n]) } else { String::new() };
    if label.is_empty() {
        format!("本地磁盘 ({letter}:)")
    } else {
        format!("{label} ({letter}:)")
    }
}

fn build_roots() -> Vec<Node> {
    let mut drives = Vec::new();
    // 一次拿盘符位掩码，别对 C..Z 逐个 Path::exists()：盘不在位掩码里就不碰它。
    // 逐个探的时候，一个断掉的映射盘/网络盘会让每次探测都等重定向器超时，
    // 二十个字母就是二十次等待，启动直接被拖成好几秒。
    let mask = unsafe { GetLogicalDrives() };
    for (i, c) in (b'A'..=b'Z').enumerate() {
        if mask & (1 << i) == 0 {
            continue;
        }
        let p = format!("{}:\\", c as char);
        drives.push(Node {
            name: vol_name(&p),
            path: p,
            open: false,
            loaded: false,
            kids: Vec::new(),
        });
    }
    // 桌面放在第一项并直接展开，桌面上的文件夹一眼就能点到
    let mut roots = Vec::new();
    if let Some(desktop) = desktop_dir() {
        roots.push(Node {
            name: "桌面".into(),
            path: format!("{desktop}\\"),
            open: true,
            loaded: true,
            kids: sub_dirs(&desktop),
        });
    }
    roots.push(Node { name: "此电脑".into(), path: "此电脑".into(), open: true, loaded: true, kids: drives });
    roots
}

unsafe fn client(hwnd: HWND) -> RECT {
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    rc
}

/// 菜单条是非客户区，GetClientRect 已经把它的 24 像素扣掉了。这里再算一遍
/// 就会把整个界面往下挪一屏，顶上露出一条从没画过的带子。
fn menu_h(_hwnd: HWND) -> i32 {
    0
}

/// 自绘文字不会替长路径折行，按固定字数断开，免得画到框外。
fn wrap_chars(s: &str, n: usize) -> String {
    let n = n.max(1);
    let mut out = String::with_capacity(s.len() + s.len() / n);
    for line in s.lines() {
        let mut cnt = 0;
        for ch in line.chars() {
            out.push(ch);
            cnt += 1;
            if cnt == n {
                out.push('\n');
                cnt = 0;
            }
        }
        if cnt > 0 || line.is_empty() {
            out.push('\n');
        }
    }
    out
}

/// 对话框尺寸；Ask 的大小要按正文量，由 open_ask 自己算，这里不给。
fn dlg_size(kind: DlgKind, k: f64) -> Option<(i32, i32)> {
    let (w, h) = match kind {
        DlgKind::Convert => (640.0, 540.0),
        DlgKind::FmtOpt => (560.0, 480.0),
        DlgKind::Search => (460.0, 250.0),
        DlgKind::BatchRename => (660.0, 500.0),
        DlgKind::NewCat => (420.0, 170.0),
        DlgKind::RenameCat => (420.0, 170.0),
        DlgKind::Skin => (780.0, 600.0),
        DlgKind::CatDb => (600.0, 330.0),
        DlgKind::ThumbDb => (600.0, 640.0),
        DlgKind::ScanRule => (560.0, 252.0),
        // 这一页装得下四块：库管理、检索条件、结果表、双预览，宽度按最长那批路径给的。
        DlgKind::SimSearch => (560.0, 752.0),
        // 关联页：顶部说明、三组五列格式勾选（56 个扩展名正好排满）、底下结果行和三个按钮。
        DlgKind::Assoc => (620.0, 620.0),
        DlgKind::Ask => return None,
    };
    Some(((w * k).round() as i32, (h * k).round() as i32))
}

/// 对话框都在客户区里居中偏上，所有窗共用这一套摆法。
fn dlg_rect(rc: &RECT, w: i32, h: i32) -> RECT {
    let left = rc.left + ((rc.right - rc.left) - w) / 2;
    let top = rc.top + ((rc.bottom - rc.top) - h) / 3;
    RECT { left, top, right: left + w, bottom: top + h }
}

fn dlg_title(kind: DlgKind, ask: &str) -> &str {
    match kind {
        DlgKind::Convert => "格式转换",
        DlgKind::FmtOpt => "格式设置",
        DlgKind::Search => "查找图像",
        DlgKind::BatchRename => "批量重命名",
        DlgKind::NewCat => "新建类别",
        DlgKind::RenameCat => "编辑类别",
        DlgKind::Skin => "皮肤设置",
        DlgKind::CatDb => "分类数据库",
        DlgKind::ThumbDb => "缩略图数据库",
        DlgKind::ScanRule => "扫描排除规则",
        DlgKind::SimSearch => "以图搜图",
        DlgKind::Assoc => "文件关联",
        DlgKind::Ask => ask,
    }
}

fn dlg_edit_rect(kind: DlgKind, r: RECT, k: f64) -> Option<RECT> {
    let (y, h, right_inset) = match kind {
        DlgKind::Convert => (332.0, 26.0, 162.0),
        DlgKind::BatchRename => (64.0, 28.0, 16.0),
        DlgKind::Search => (72.0, 28.0, 16.0),
        DlgKind::NewCat => (72.0, 28.0, 16.0),
        DlgKind::RenameCat => (72.0, 28.0, 16.0),
        // 皮肤设置窗只有"另存为"那一刻要输名字：底部空出来的那一整行就是它的位置
        DlgKind::Skin => (488.0, 26.0, 16.0),
        DlgKind::FmtOpt | DlgKind::CatDb | DlgKind::ThumbDb | DlgKind::ScanRule | DlgKind::SimSearch
        | DlgKind::Assoc | DlgKind::Ask => return None,
    };
    Some(RECT {
        left: r.left + (16.0 * k) as i32,
        top: r.top + (y * k) as i32,
        right: r.right - (right_inset * k) as i32,
        bottom: r.top + ((y + h) * k) as i32,
    })
}

unsafe fn dlg_button(
    hdc: HDC,
    a: &mut App,
    s: &Theme,
    r: RECT,
    id: usize,
    label: &str,
    primary: bool,
    enabled: bool,
) {
    let hot = enabled && over(&r, a.pt.0, a.pt.1);
    fill(hdc, &r, if !enabled { s.skin.chrome } else if primary { s.skin.acc } else if hot { s.skin.hov } else { s.skin.chrome_hi });
    frame(hdc, &r, if primary { s.skin.acc } else { s.skin.line });
    let ink = if !enabled {
        s.skin.ink_off
    } else if primary {
        s.skin.ink_inv
    } else {
        s.skin.ink
    };
    txt(hdc, &r, label, ink, a.font, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    if enabled {
        a.dlg_btns.push((id, r));
    }
}

unsafe fn dlg_check(hdc: HDC, a: &mut App, s: &Theme, r: RECT, id: usize, label: &str, on: bool) {
    let k = s.scale;
    let bx = (13.0 * k) as i32;
    let b = RECT {
        left: r.left,
        top: r.top + (r.bottom - r.top - bx) / 2,
        right: r.left + bx,
        bottom: r.top + (r.bottom - r.top + bx) / 2,
    };
    fill(hdc, &b, if on { s.skin.acc } else { s.skin.canvas });
    frame(hdc, &b, if over(&r, a.pt.0, a.pt.1) { s.skin.acc } else { s.skin.line });
    if on {
        txt(hdc, &b, "✓", s.skin.ink_inv, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    }
    let tr = RECT { left: b.right + (5.0 * k) as i32, top: r.top, right: r.right, bottom: r.bottom };
    txt(hdc, &tr, label, s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    a.dlg_btns.push((id, r));
}

/// 单选圈。三选一的缩略图设置用它，点圆圈或文字都算。
unsafe fn dlg_radio(hdc: HDC, a: &mut App, s: &Theme, r: RECT, id: usize, label: &str, on: bool) {
    let k = s.scale;
    let d = (13.0 * k) as i32;
    let b = RECT {
        left: r.left,
        top: r.top + (r.bottom - r.top - d) / 2,
        right: r.left + d,
        bottom: r.top + (r.bottom - r.top + d) / 2,
    };
    fill(hdc, &b, s.skin.canvas);
    frame(hdc, &b, if on { s.skin.acc } else { s.skin.line });
    if on {
        let p = (3.0 * k) as i32;
        fill(hdc, &RECT { left: b.left + p, top: b.top + p, right: b.right - p, bottom: b.bottom - p }, s.skin.acc);
    }
    let tr = RECT { left: b.right + (5.0 * k) as i32, top: r.top, right: r.right, bottom: r.bottom };
    txt(hdc, &tr, label, s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    a.dlg_btns.push((id, r));
}

/// 自绘下拉条。点一下开合，展开的行由调用方画出来并放进 dlg_rows。
unsafe fn dlg_combo(hdc: HDC, a: &mut App, s: &Theme, r: RECT, id: usize, text: &str) {
    let k = s.scale;
    let hot = over(&r, a.pt.0, a.pt.1);
    fill(hdc, &r, s.skin.canvas);
    frame(hdc, &r, if hot { s.skin.acc } else { s.skin.line });
    let tr = RECT { left: r.left + (7.0 * k) as i32, top: r.top, right: r.right - (22.0 * k) as i32, bottom: r.bottom };
    txt(hdc, &tr, text, s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    // 右侧的小三角
    let cx = r.right - (12.0 * k) as i32;
    let cy = (r.top + r.bottom) / 2;
    let w = (4.0 * k) as i32;
    for i in 0..w {
        let y = cy - w / 2 + i;
        hline(hdc, cx - w + i, cx + w - i, y, s.skin.ink_dim);
    }
    a.dlg_btns.push((id, r));
}

/// 一条自绘滑条。调用方负责画数值，命中的点击换算也由 mouse_down 按同一个矩形算。
unsafe fn dlg_slider(hdc: HDC, a: &mut App, s: &Theme, r: RECT, id: usize, v: i32, lo: i32, hi: i32) {
    let k = s.scale;
    let h = (6.0 * k) as i32;
    let mid = (r.top + r.bottom) / 2;
    let track = RECT { left: r.left, top: mid - h / 2, right: r.right, bottom: mid + h / 2 };
    fill(hdc, &track, s.skin.canvas);
    frame(hdc, &track, s.skin.line);
    let t = ((v - lo) as f64 / (hi - lo).max(1) as f64).clamp(0.0, 1.0);
    let fr = RECT { left: track.left + 1, top: track.top + 1, right: track.left + 1 + ((track.right - track.left - 2) as f64 * t) as i32, bottom: track.bottom - 1 };
    fill(hdc, &fr, s.skin.acc);
    let hd = RECT { left: fr.right - (3.0 * k) as i32, top: track.top - (5.0 * k) as i32, right: fr.right + (3.0 * k) as i32, bottom: track.bottom + (5.0 * k) as i32 };
    fill(hdc, &hd, s.skin.chrome_hi);
    frame(hdc, &hd, s.skin.acc);
    a.dlg_btns.push((id, r));
}

/// 目标格式行左边那个小方块图标：皮肤色打底，写扩展名首字母。
unsafe fn fmt_badge(hdc: HDC, a: &App, s: &Theme, r: RECT, fmt: usize, lit: bool) {
    fill(hdc, &r, if lit { s.skin.acc_soft } else { s.skin.chrome });
    frame(hdc, &r, if lit { s.skin.acc } else { s.skin.line });
    let letter = ["J", "P", "B", "T", "G"].get(fmt).copied().unwrap_or("?");
    txt(hdc, &r, letter, if lit { s.skin.acc } else { s.skin.ink_off }, a.font_b, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
}

/// 转换对话框右下的“将转换 N 个文件”，选中为空时提醒。
fn conv_scope_count(a: &App) -> usize {
    a.selected_files().len()
}

/// 模板里的 # 换成编号、* 换成原名，只出**主名**、不带扩展名。
/// 批量转换用它：目标扩展名由所选格式自己拼，这里带上源扩展名就成了 `照片.png.jpg`。
fn rn_stem(tpl: &str, start: i32, idx: usize, old: &str, case: u8) -> String {
    let p = Path::new(old);
    let old_stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let chars: Vec<char> = tpl.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '#' => {
                let mut j = i;
                while j < chars.len() && chars[j] == '#' {
                    j += 1;
                }
                let w = j - i;
                let n = (start + idx as i32).max(0) as u32;
                out.push_str(&format!("{:0w$}", n, w = w));
                i = j;
            }
            '*' => {
                out.push_str(old_stem);
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    let mut name = match case {
        2 => out.to_uppercase(),
        1 => out.to_lowercase(),
        _ => out,
    };
    if name.is_empty() {
        name = old_stem.to_string();
    }
    name
}

/// 模板展开成整名：主名 + 源文件自己的扩展名。批量重命名改的是整个文件名，用它。
fn rn_new_name(tpl: &str, start: i32, idx: usize, old: &str, case: u8) -> String {
    let stem = rn_stem(tpl, start, idx, old, case);
    match Path::new(old).extension().and_then(|s| s.to_str()) {
        Some(e) if !e.is_empty() => format!("{stem}.{e}"),
        _ => stem,
    }
}

/// 打开批量重命名时预填的模板：当前视图里第一个选中的文件名（不含扩展名）再接 ###。
fn rn_default_tpl(g: &App) -> String {
    let first = if g.marked.is_empty() {
        g.entries.get(g.sel).filter(|e| !e.is_dir)
    } else {
        g.entries.iter().find(|e| !e.is_dir && g.marked.contains(&e.path))
    };
    match first {
        Some(e) => {
            let name = Path::new(&e.path).file_stem().and_then(|s| s.to_str()).unwrap_or("");
            format!("{name}###")
        }
        None => g.rn_tpl.clone(),
    }
}

fn rn_preview(a: &App, live_tpl: &str) -> Vec<(String, String)> {
    let paths = a.selected_files();
    let raw = live_tpl.trim();
    let tpl = if raw.is_empty() { "*" } else { raw };
    paths
        .iter()
        .enumerate()
        .map(|(i, p)| (stem(p).to_string(), rn_new_name(tpl, a.rn_start, i, p, a.rn_case)))
        .collect()
}

/// 配色页上列出来的颜色。文字色（正文/次要/不可用/反白/选中项文字）不在这儿，
/// 字体页能按版块逐块改字色，比这里一刀切细。
const COLOR_SLOTS: [&str; 19] = [
    "base",
    "chrome",
    "chrome_hi",
    "menu",
    "popup",
    "canvas",
    "pane",
    "pane_head",
    "grid",
    "line",
    "line_soft",
    "hov",
    "prs",
    "sel",
    "accent",
    "acc_soft",
    "mark",
    "flash",
    "focus",
];

fn slot_label(k: &str) -> &str {
    match k {
        "base" => "窗口底色",
        "chrome" => "工具栏、图标底色",
        "chrome_hi" => "按钮、弹窗底色",
        "menu" => "菜单条底色",
        "popup" => "下拉菜单底色",
        "canvas" => "空白处、看大图底色",
        "pane" => "面板、文字底色",
        "pane_head" => "面板标题条",
        "grid" => "透明图片棋盘格",
        "line" => "边线",
        "line_soft" => "浅边线",
        "hov" => "鼠标悬停底",
        "prs" => "鼠标按下底",
        "sel" => "选中项底色",
        "accent" => "标题栏、文件夹图标",
        "acc_soft" => "浅主题色",
        "mark" => "标记和角标",
        "flash" => "复制剪切闪光",
        "focus" => "聚焦边框",
        _ => k,
    }
}

unsafe fn swatch(hdc: HDC, r: &RECT, c: u32, s: &Theme, hot: bool) {
    fill(hdc, r, c);
    frame(hdc, r, if hot { s.skin.acc } else { s.skin.line });
}

unsafe fn pick_color(hwnd: HWND, initial: u32) -> Option<u32> {
    let mut custom = [COLORREF(0); 16];
    let mut cc = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: hwnd,
        rgbResult: cref(initial),
        lpCustColors: custom.as_mut_ptr(),
        Flags: CC_RGBINIT | CC_FULLOPEN,
        ..Default::default()
    };
    if ChooseColorW(&mut cc).as_bool() {
        let v = cc.rgbResult.0;
        Some(((v & 0xff) << 16) | (v & 0xff00) | ((v >> 16) & 0xff))
    } else {
        None
    }
}

unsafe fn pick_font(hwnd: HWND, face: &str, px: i32) -> Option<String> {
    let mut lf = LOGFONTW::default();
    lf.lfHeight = -px.max(9);
    lf.lfCharSet = DEFAULT_CHARSET;
    for (i, c) in face.encode_utf16().take(lf.lfFaceName.len() - 1).enumerate() {
        lf.lfFaceName[i] = c;
    }
    let mut cf = CHOOSEFONTW {
        lStructSize: std::mem::size_of::<CHOOSEFONTW>() as u32,
        hwndOwner: hwnd,
        lpLogFont: &mut lf,
        iPointSize: (px * 75) / 10,
        Flags: CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT,
        ..Default::default()
    };
    if ChooseFontW(&mut cf).as_bool() {
        let n = lf.lfFaceName.iter().position(|c| *c == 0).unwrap_or(lf.lfFaceName.len());
        let s = String::from_utf16_lossy(&lf.lfFaceName[..n]);
        if s.is_empty() { None } else { Some(s) }
    } else {
        None
    }
}

/// 步进某一块的字号。皮肤设置窗开着时改的是草稿：界面上马上跟着变，点确定才写文件。
unsafe fn step_zone(hwnd: HWND, i: usize, delta: i32) {
    let mut guard = app().lock().unwrap();
    let a = &mut *guard;
    let open = a.dlg.is_some();
    let loose = a.density == "loose";
    let key = theme::ZONES[i].0;
    let look = match a.dlg.as_mut() {
        Some(d) => &mut d.look_draft,
        None => &mut a.look,
    };
    let def = theme::ZONE_DEF[i] + if loose { 1 } else { 0 };
    let cur = look.zone_px(key).unwrap_or(def);
    let v = (cur + delta).clamp(8, 40);
    look.set_zone_px(key, v);
    if open {
        a.apply_draft(hwnd);
    }
    drop(guard);
    refresh(hwnd);
}

/// 某一块的加粗开关，和字号走同一份草稿。
unsafe fn toggle_zone_bold(hwnd: HWND, i: usize) {
    let mut guard = app().lock().unwrap();
    let a = &mut *guard;
    let open = a.dlg.is_some();
    let key = theme::ZONES[i].0;
    match a.dlg.as_mut() {
        Some(d) => d.look_draft.flip_zone_bold(key),
        None => a.look.flip_zone_bold(key),
    }
    if open {
        a.apply_draft(hwnd);
    }
    drop(guard);
    refresh(hwnd);
}

unsafe fn size_row(
    hdc: HDC,
    a: &mut App,
    s: &Theme,
    r: RECT,
    label: &str,
    val: i32,
    dn: usize,
    up: usize,
) {
    let k = s.scale;
    txt(hdc, &r, label, s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    let bw = (28.0 * k) as i32;
    let x = r.left + (120.0 * k) as i32;
    let y = r.top - (2.0 * k) as i32;
    dlg_button(hdc, a, s, RECT { left: x, top: y, right: x + bw, bottom: y + (26.0 * k) as i32 }, dn, "−", false, true);
    let vr = RECT { left: x + bw + (6.0 * k) as i32, top: y, right: x + bw + (56.0 * k) as i32, bottom: y + (26.0 * k) as i32 };
    fill(hdc, &vr, s.skin.canvas);
    frame(hdc, &vr, s.skin.line_soft);
    txt(hdc, &vr, &val.to_string(), s.skin.ink, a.font_num, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    let x2 = vr.right + (6.0 * k) as i32;
    dlg_button(hdc, a, s, RECT { left: x2, top: y, right: x2 + bw, bottom: y + (26.0 * k) as i32 }, up, "+", false, true);
}

/// 模态对话框整块。命中区每次重画都重建，点的时候和看到的永远一致。
/// 有确认框压在设置窗上时画两层：底下那层只画不认命中，点哪一下都只算在浮上来这层。
unsafe fn paint_dlg(hdc: HDC, a: &mut App, s: &Theme, rc: &RECT) {
    if a.dlg_hold.is_some() {
        std::mem::swap(&mut a.dlg, &mut a.dlg_hold);
        paint_one_dlg(hdc, a, s, rc);
        std::mem::swap(&mut a.dlg, &mut a.dlg_hold);
    }
    paint_one_dlg(hdc, a, s, rc);
}

/// 弹层投影：一块与弹层同尺寸的灰纱，整体往右下让 6 像素，AlphaBlend 50%，
/// 底下压着的格子/图透得出来。灰而不是黑：黑压在主题底色上会读成"有黑边"。
/// 绘制和拖动时算脏区都读 shadow_off，改一处两边一起变。
const SHADOW_GRAY: u32 = 0x009E9E9E;
const SHADOW_ALPHA: u8 = 128;

fn shadow_off(k: f64) -> i32 {
    (4.0 * k) as i32 + 1
}

unsafe fn drop_shadow(hdc: HDC, r: &RECT, k: f64) {
    let off = shadow_off(k);
    let sh = RECT {
        left: r.left + off,
        top: r.top + off,
        right: r.right + off,
        bottom: r.bottom + off,
    };
    veil(hdc, &sh, SHADOW_GRAY, SHADOW_ALPHA);
}

unsafe fn paint_one_dlg(hdc: HDC, a: &mut App, s: &Theme, _rc: &RECT) {
    a.zone(theme::Z_DLG);
    a.dlg_btns.clear();
    a.dlg_rows.clear();
    let Some(d) = a.dlg.as_ref() else { return };
    let (kind, rect, fmt, opt, scope_db, combo_open, ask_title, ask_body, ask_yes) = (
        d.kind,
        d.rect,
        d.fmt,
        d.opt.clone(),
        d.scope_db,
        d.combo_open,
        d.ask_title.clone(),
        d.ask_body.clone(),
        d.ask_yes,
    );
    let k = s.scale;
    // 影子跟窗口同尺寸，整块往右下让开几像素：窗口盖住其余部分，右下露出一条
    // 连续的带，不会在窗口和影子之间留缝。走 AlphaBlend，底下压着的格子透得出来。
    drop_shadow(hdc, &rect, k);
    fill(hdc, &rect, s.skin.chrome_hi);
    frame(hdc, &rect, s.skin.line);
    // 浮层的标题条：底色仍是中性那两档，取色规则、✕ 那颗的尺寸和悬停那一层
    // 全部走 theme::Strip，和主窗口标题栏右上角那颗是同一套画法（以前这里是硬编码红）。
    let st = theme::Strip::float(s);
    let tbar = RECT { left: rect.left, top: rect.top, right: rect.right, bottom: rect.top + st.h };
    fill(hdc, &tbar, st.bar);
    hline(hdc, tbar.left, tbar.right, tbar.bottom - 1, st.edge);
    let tr = RECT { left: tbar.left + (12.0 * k) as i32, top: tbar.top, right: tbar.right - (st.bw + st.pad + (6.0 * k) as i32), bottom: tbar.bottom };
    txt(hdc, &tr, dlg_title(kind, &ask_title), st.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
    let xr = RECT {
        left: tbar.right - st.pad - st.bw,
        top: tbar.top + (tbar.bottom - tbar.top - st.bh) / 2,
        right: tbar.right - st.pad,
        bottom: tbar.top + (tbar.bottom - tbar.top - st.bh) / 2 + st.bh,
    };
    let xhot = over(&xr, a.pt.0, a.pt.1);
    if xhot {
        if fine(s) {
            let hr = RECT { left: xr.left + 2, top: xr.top + 2, right: xr.right - 2, bottom: xr.bottom - 2 };
            let rad = (4.0 * k).round() as i32;
            let keep = round_clip(hdc, &hr, rad);
            fill(hdc, &hr, st.hot);
            unround(hdc, keep);
            rring(hdc, &hr, rad, mix(st.bar, st.fg, 0.45), None);
        } else {
            fill(hdc, &xr, st.hot);
            if !bands(s) {
                frame(hdc, &xr, st.ring);
            }
        }
    }
    if bands(s) {
        xmark(hdc, (xr.left + xr.right) / 2, (xr.top + xr.bottom) / 2, (5.0 * k) as i32, if xhot { st.fg } else { st.dim });
    } else {
        txt(hdc, &xr, "✕", if xhot { st.fg } else { st.dim }, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
    }
    a.dlg_btns.push((D_CLOSE, xr));
    // 标题条登记在 ✕ 之后：命中表先扫到的算数，所以右上角那一下仍然是关闭，
    // 条上别的地方才是拖动。
    a.dlg_btns.push((D_TITLEBAR, tbar));

    let lx = rect.left + (16.0 * k) as i32;
    let lab = |y: f64| RECT {
        left: lx,
        top: rect.top + (y * k) as i32,
        right: rect.right - (16.0 * k) as i32,
        bottom: rect.top + ((y + 18.0) * k) as i32,
    };
    let ed = dlg_edit_rect(kind, rect, k);
    match kind {
        DlgKind::Convert => {
            txt(hdc, &lab(38.0), "目标格式(&F)", s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            let lb = RECT {
                left: lx,
                top: rect.top + (58.0 * k) as i32,
                right: lx + (250.0 * k) as i32,
                bottom: rect.top + (248.0 * k) as i32,
            };
            fill(hdc, &lb, s.skin.canvas);
            frame(hdc, &lb, s.skin.line);
            for (i, (name, ext, desc)) in encode::FORMATS.iter().enumerate() {
                let r = RECT {
                    left: lb.left + 1,
                    top: lb.top + 3 + i as i32 * (36.0 * k) as i32,
                    right: lb.right - 1,
                    bottom: lb.top + 3 + (i as i32 + 1) * (36.0 * k) as i32,
                };
                let on = i == fmt;
                if on {
                    fill(hdc, &r, s.skin.acc_soft);
                    frame(hdc, &r, s.skin.acc);
                } else if over(&r, a.pt.0, a.pt.1) {
                    fill(hdc, &r, s.skin.hov);
                }
                let bd = (24.0 * k) as i32;
                let branch = r.top + (r.bottom - r.top - bd) / 2;
                let br = RECT { left: r.left + (8.0 * k) as i32, top: branch, right: r.left + (8.0 * k) as i32 + bd, bottom: branch + bd };
                fmt_badge(hdc, a, s, br, i, on);
                let nr = RECT { left: br.right + (8.0 * k) as i32, top: r.top + (2.0 * k) as i32, right: r.right - (52.0 * k) as i32, bottom: r.top + (19.0 * k) as i32 };
                txt(hdc, &nr, name, s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                let dr = RECT { left: nr.left, top: r.top + (18.0 * k) as i32, right: r.right - (52.0 * k) as i32, bottom: r.bottom - (2.0 * k) as i32 };
                txt(hdc, &dr, desc, s.skin.ink_dim, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                let er = RECT { left: r.right - (52.0 * k) as i32, top: r.top, right: r.right - (6.0 * k) as i32, bottom: r.bottom };
                txt(hdc, &er, &format!(".{ext}"), s.skin.ink_dim, a.font_num, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
                a.dlg_rows.push((D_FMT + i, r));
            }
            dlg_button(hdc, a, s, RECT { left: lx, top: rect.top + (256.0 * k) as i32, right: lx + (150.0 * k) as i32, bottom: rect.top + (284.0 * k) as i32 }, D_FMTSET, "格式设置(&S)...", false, true);
            let n = conv_scope_count(a);
            let cr = RECT { left: lx + (160.0 * k) as i32, top: rect.top + (256.0 * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + (284.0 * k) as i32 };
            txt(hdc, &cr, &format!("将转换 {n} 个文件"), s.skin.ink_dim, a.font_sm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);

            // 输出组
            let gr = RECT { left: lx, top: rect.top + (300.0 * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + (470.0 * k) as i32 };
            frame(hdc, &gr, s.skin.line_soft);
            let gt = RECT { left: gr.left + (10.0 * k) as i32, top: gr.top - (9.0 * k) as i32, right: gr.left + (58.0 * k) as i32, bottom: gr.top + (9.0 * k) as i32 };
            fill(hdc, &gt, s.skin.chrome_hi);
            txt(hdc, &gt, "输出", s.skin.ink, a.font_b, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
            let fl = RECT { left: gr.left + (14.0 * k) as i32, top: rect.top + (312.0 * k) as i32, right: gr.right - (12.0 * k) as i32, bottom: rect.top + (330.0 * k) as i32 };
            txt(hdc, &fl, "目标文件夹(&T):", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            if let Some(e) = ed {
                fill(hdc, &e, s.skin.canvas);
                frame(hdc, &e, s.skin.line);
                dlg_button(hdc, a, s, RECT { left: e.right + (8.0 * k) as i32, top: e.top, right: gr.right - (12.0 * k) as i32, bottom: e.bottom }, D_BROWSE, "浏览...", false, true);
            }
            txt(hdc, &RECT { left: fl.left, top: rect.top + (364.0 * k) as i32, right: fl.right, bottom: rect.top + (382.0 * k) as i32 }, "覆盖现有文件(&O):", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            let ow = RECT { left: gr.left + (14.0 * k) as i32, top: rect.top + (382.0 * k) as i32, right: gr.left + (254.0 * k) as i32, bottom: rect.top + (408.0 * k) as i32 };
            let ow_name = ["取代", "跳过", "重命名", "询问"][a.conv_overwrite.min(3) as usize];
            dlg_combo(hdc, a, s, ow, D_OW, ow_name);
            dlg_check(hdc, a, s, RECT { left: gr.left + (14.0 * k) as i32, top: rect.top + (422.0 * k) as i32, right: gr.left + (280.0 * k) as i32, bottom: rect.top + (444.0 * k) as i32 }, D_DELSRC, "删除/替换原文件(&R)", a.conv_del);
            if combo_open {
                let ih = (22.0 * k) as i32;
                for i in 0..4 {
                    let r = RECT { left: ow.left, top: ow.bottom + i as i32 * ih, right: ow.right, bottom: ow.bottom + (i as i32 + 1) * ih };
                    let on = i == a.conv_overwrite as usize;
                    fill(hdc, &r, if on { s.skin.acc_soft } else if over(&r, a.pt.0, a.pt.1) { s.skin.hov } else { s.skin.chrome_hi });
                    frame(hdc, &r, s.skin.line_soft);
                    txt(hdc, &r, ["取代", "跳过", "重命名", "询问"][i], if on { s.skin.acc } else { s.skin.ink }, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
                    a.dlg_rows.push((D_OW_ITEM + i, r));
                }
            }
            let by = rect.bottom - (42.0 * k) as i32;
            let bw = (74.0 * k) as i32;
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw * 2 - (8.0 * k) as i32, top: by, right: rect.right - (16.0 * k) as i32 - bw - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_OK, "确定", true, n > 0);
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw, top: by, right: rect.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_CANCEL, "取消", false, true);
        }
        DlgKind::FmtOpt => {
            let (fname, _, _) = encode::FORMATS[fmt];
            txt(hdc, &lab(34.0), &format!("{fname} 格式设置"), s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            let lw = rect.right - rect.left - (32.0 * k) as i32;
            let sl = RECT { left: lx, top: rect.top + (84.0 * k) as i32, right: lx + lw - (124.0 * k) as i32, bottom: rect.top + (108.0 * k) as i32 };
            let vr = RECT { left: sl.right + (12.0 * k) as i32, top: rect.top + (80.0 * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + (112.0 * k) as i32 };
            let lbl = |y: f64| -> RECT { RECT { left: lx, top: rect.top + (y * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + ((y + 18.0) * k) as i32 } };
            let note = |y: f64| -> RECT { RECT { left: lx, top: rect.top + (y * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + ((y + 54.0) * k) as i32 } };
            let chk = |y: f64, wid: f64| -> RECT { RECT { left: lx, top: rect.top + (y * k) as i32, right: lx + (wid * k) as i32, bottom: rect.top + ((y + 22.0) * k) as i32 } };
            match fmt {
                0 => {
                    txt(hdc, &lbl(62.0), "图像质量(&Q):", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                    dlg_slider(hdc, a, s, sl, D_QUAL, opt.quality as i32, 1, 100);
                    fill(hdc, &vr, s.skin.canvas);
                    frame(hdc, &vr, s.skin.line);
                    txt(hdc, &vr, &format!("{}", opt.quality), s.skin.ink, a.font_num, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
                    dlg_check(hdc, a, s, chk(120.0, 180.0), D_OPT_PROG, "渐进式(&P)", opt.progressive);
                    dlg_check(hdc, a, s, chk(146.0, 260.0), D_OPT_HUFF, "优化的哈夫曼代码(&O)", opt.optimize_huffman);
                    hline(hdc, lx, rect.right - (16.0 * k) as i32, rect.top + (178.0 * k) as i32, s.skin.line_soft);
                    txt(hdc, &lbl(184.0), "2:1 色度取样", s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                    dlg_check(hdc, a, s, chk(206.0, 200.0), D_OPT_SH, "2:1 水平取样(&H)", opt.sample_h);
                    dlg_check(hdc, a, s, chk(232.0, 200.0), D_OPT_SV, "2:1 垂直取样(&V)", opt.sample_v);
                    hline(hdc, lx, rect.right - (16.0 * k) as i32, rect.top + (268.0 * k) as i32, s.skin.line_soft);
                    txt(hdc, &lbl(276.0), "已装入缩略图", s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                    dlg_radio(hdc, a, s, chk(298.0, 200.0), D_THUMB, "从来不更新", opt.thumb == 0);
                    dlg_radio(hdc, a, s, chk(322.0, 320.0), D_THUMB + 1, "仅在缩略图已经存在时更新", opt.thumb == 1);
                    dlg_radio(hdc, a, s, chk(346.0, 240.0), D_THUMB + 2, "总是增加更新", opt.thumb == 2);
                    dlg_check(hdc, a, s, chk(376.0, 300.0), D_DCF, "生成 DCF 兼容缩略图(&D)", opt.dcf);
                }
                1 => {
                    txt(hdc, &lbl(62.0), "压缩级别(&L):", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                    dlg_slider(hdc, a, s, sl, D_OPT_PNG, opt.png_level as i32, 0, 9);
                    fill(hdc, &vr, s.skin.canvas);
                    frame(hdc, &vr, s.skin.line);
                    txt(hdc, &vr, &format!("{}", opt.png_level), s.skin.ink, a.font_num, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
                    txt(hdc, &note(120.0), "0 = 不压缩，速度最快；9 = 压缩得最狠，速度最慢。\nPNG 是无损格式，压缩级别只影响文件大小和耗时，不影响画质。", s.skin.ink_dim, a.font_sm, DT_LEFT | DT_WORDBREAK);
                }
                3 => {
                    txt(hdc, &lbl(62.0), "压缩方式(&C):", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                    for (i, t) in ["不压缩", "LZW", "ZIP", "PackBits"].iter().enumerate() {
                        dlg_radio(hdc, a, s, chk(90.0 + i as f64 * 30.0, 200.0), D_OPT_TIFF + i, t, opt.tiff_comp as usize == i);
                    }
                    txt(hdc, &note(220.0), "LZW 兼容性最好；ZIP 对照片更小；PackBits 适合大面积纯色。\n无论选哪一种都不损失画质。", s.skin.ink_dim, a.font_sm, DT_LEFT | DT_WORDBREAK);
                }
                2 => {
                    txt(hdc, &note(62.0), "BMP 是未压缩位图，没有质量或压缩选项。\n转换后的文件通常比 JPEG / PNG 大很多。", s.skin.ink_dim, a.font_sm, DT_LEFT | DT_WORDBREAK);
                }
                _ => {
                    txt(hdc, &note(62.0), "GIF 使用固定的 256 色调色板和无损压缩，没有可调参数。\n超过 256 色或带 Alpha 渐变的图片转成 GIF 会出现色带。", s.skin.ink_dim, a.font_sm, DT_LEFT | DT_WORDBREAK);
                }
            }
            dlg_check(hdc, a, s, chk(408.0, 320.0), D_SAVEDEF, "把设置保存为默认值", scope_db);
            let by = rect.bottom - (42.0 * k) as i32;
            let bw = (74.0 * k) as i32;
            dlg_button(hdc, a, s, RECT { left: lx, top: by, right: lx + (96.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_FMT_RESET, "重置所有", false, true);
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw * 2 - (8.0 * k) as i32, top: by, right: rect.right - (16.0 * k) as i32 - bw - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_OK, "确定", true, true);
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw, top: by, right: rect.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_CANCEL, "取消", false, true);
        }
        DlgKind::BatchRename => {
            let live = match a.dlg_edit {
                Some(h) => edit_text(h),
                None => a.rn_tpl.clone(),
            };
            let pd = rn_preview(a, &live);
            txt(hdc, &lab(40.0), "文件名模板(&T):", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            if let Some(e) = ed {
                fill(hdc, &e, s.skin.canvas);
                frame(hdc, &e, s.skin.line);
            }
            let hint = RECT { left: lx + (410.0 * k) as i32, top: rect.top + (64.0 * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + (92.0 * k) as i32 };
            txt(hdc, &hint, "# = 编号　* = 原名", s.skin.ink_dim, a.font_sm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
            let note = RECT { left: lx, top: rect.top + (150.0 * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + (172.0 * k) as i32 };
            txt(hdc, &note, "说明：* 代表原文件名，# 代表顺序编号（## 补零，从“起始编号”递增）；重名或含非法字符的文件会自动跳过。", s.skin.ink_dim, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            txt(hdc, &RECT { left: lx, top: rect.top + (104.0 * k) as i32, right: lx + (100.0 * k) as i32, bottom: rect.top + (122.0 * k) as i32 }, "起始编号(&S):", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            dlg_button(hdc, a, s, RECT { left: lx, top: rect.top + (122.0 * k) as i32, right: lx + (28.0 * k) as i32, bottom: rect.top + (148.0 * k) as i32 }, D_RN_START_DN, "−", false, true);
            let nr = RECT { left: lx + (32.0 * k) as i32, top: rect.top + (122.0 * k) as i32, right: lx + (104.0 * k) as i32, bottom: rect.top + (148.0 * k) as i32 };
            fill(hdc, &nr, s.skin.canvas);
            frame(hdc, &nr, s.skin.line);
            txt(hdc, &nr, &format!("{}", a.rn_start), s.skin.ink, a.font_num, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
            dlg_button(hdc, a, s, RECT { left: lx + (108.0 * k) as i32, top: rect.top + (122.0 * k) as i32, right: lx + (136.0 * k) as i32, bottom: rect.top + (148.0 * k) as i32 }, D_RN_START_UP, "+", false, true);
            txt(hdc, &RECT { left: lx + (176.0 * k) as i32, top: rect.top + (104.0 * k) as i32, right: lx + (300.0 * k) as i32, bottom: rect.top + (122.0 * k) as i32 }, "文件名大小写:", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            for (i, t) in ["不变", "小写", "大写"].iter().enumerate() {
                dlg_button(hdc, a, s, RECT { left: lx + ((176 + i * 84) as f64 * k) as i32, top: rect.top + (122.0 * k) as i32, right: lx + ((250 + i * 84) as f64 * k) as i32, bottom: rect.top + (148.0 * k) as i32 }, D_RN_CASE + i, t, a.rn_case as usize == i, true);
            }
            // 预览表
            let tb = RECT { left: lx, top: rect.top + (176.0 * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + (438.0 * k) as i32 };
            fill(hdc, &tb, s.skin.canvas);
            frame(hdc, &tb, s.skin.line);
            let colw = (tb.right - tb.left) / 2;
            let hh = (24.0 * k) as i32;
            fill(hdc, &RECT { left: tb.left + 1, top: tb.top + 1, right: tb.right - 1, bottom: tb.top + 1 + hh }, s.skin.pane_head);
            txt(hdc, &RECT { left: tb.left + (8.0 * k) as i32, top: tb.top + 1, right: tb.left + colw, bottom: tb.top + 1 + hh }, "原名称", s.skin.ink_dim, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            txt(hdc, &RECT { left: tb.left + colw, top: tb.top + 1, right: tb.right - (8.0 * k) as i32, bottom: tb.top + 1 + hh }, "重命名后", s.skin.ink_dim, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            if combo_open {
                let h = RECT { left: tb.left + (12.0 * k) as i32, top: tb.top + hh + (8.0 * k) as i32, right: tb.right - (12.0 * k) as i32, bottom: tb.bottom - (8.0 * k) as i32 };
                txt(hdc, &h, "批量重命名用模板生成新文件名：\n\n\
                    · # 换成编号，连续写几个就补到几位：## 会得到 01、02、03。\n\
                    · * 换成原来的文件名（不含扩展名）。\n\
                    · 扩展名一律保留原文件的，模板里不用写。\n\
                    · 起始编号决定第一个文件的编号，往后依次加一。\n\
                    · 大小写只影响生成的名字，不影响扩展名。\n\n\
                    新名字和已有文件重名、或两个文件算出同一个新名字时，那一条会跳过，完成后在状态栏汇总。\n\n\
                    点“帮助”回到预览。", s.skin.ink, a.font_sm, DT_LEFT | DT_WORDBREAK);
            } else if pd.is_empty() {
                txt(hdc, &RECT { left: tb.left + 1, top: tb.top + hh + 1, right: tb.right - 1, bottom: tb.bottom - 1 }, "先选中要重命名的文件", s.skin.ink_dim, a.font, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
            } else {
                let rh = (20.0 * k) as i32;
                let maxrows = ((tb.bottom - tb.top - hh - 8) / rh).max(0) as usize;
                for (i, (old, new)) in pd.iter().take(maxrows).enumerate() {
                    let yr = tb.top + hh + 4 + i as i32 * rh;
                    if i % 2 == 1 {
                        fill(hdc, &RECT { left: tb.left + 1, top: yr, right: tb.right - 1, bottom: yr + rh }, s.skin.pane);
                    }
                    txt(hdc, &RECT { left: tb.left + (8.0 * k) as i32, top: yr, right: tb.left + colw - (4.0 * k) as i32, bottom: yr + rh }, old, s.skin.ink_dim, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                    txt(hdc, &RECT { left: tb.left + colw, top: yr, right: tb.right - (8.0 * k) as i32, bottom: yr + rh }, new, s.skin.ink, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                }
                if pd.len() > maxrows {
                    txt(hdc, &RECT { left: tb.left + (8.0 * k) as i32, top: tb.bottom - rh - 2, right: tb.right - (8.0 * k) as i32, bottom: tb.bottom - 2 }, &format!("共 {} 个，表里只列前 {} 个", pd.len(), maxrows), s.skin.ink_dim, a.font_sm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
                }
            }
            let by = rect.bottom - (42.0 * k) as i32;
            let bw = (74.0 * k) as i32;
            dlg_button(hdc, a, s, RECT { left: lx, top: by, right: lx + bw, bottom: by + (26.0 * k) as i32 }, D_RN_HELP, if combo_open { "预览" } else { "帮助" }, false, true);
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw * 2 - (8.0 * k) as i32, top: by, right: rect.right - (16.0 * k) as i32 - bw - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_OK, "确定", true, !pd.is_empty());
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw, top: by, right: rect.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_CANCEL, "取消", false, true);
        }
        DlgKind::Search => {
            txt(hdc, &lab(44.0), "名称包含(&N):", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            if let Some(e) = ed {
                fill(hdc, &e, s.skin.canvas);
                frame(hdc, &e, s.skin.line);
            }
            let sc = RECT { left: lx, top: rect.top + (112.0 * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + (132.0 * k) as i32 };
            let sd = RECT { left: lx, top: rect.top + (136.0 * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + (156.0 * k) as i32 };
            dlg_check(hdc, a, s, sc, D_SCOPE_CUR, "当前文件夹（含子文件夹）", !scope_db);
            dlg_check(hdc, a, s, sd, D_SCOPE_DB, "全数据库（存过缩略图的全部图）", scope_db);
            let hint = RECT { left: lx, top: rect.top + (166.0 * k) as i32, right: rect.right - (16.0 * k) as i32, bottom: rect.top + (184.0 * k) as i32 };
            txt(hdc, &hint, "搜索结果每页 120 张，用主窗口提示条翻页。", s.skin.ink_dim, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            let by = rect.bottom - (42.0 * k) as i32;
            let bw = (82.0 * k) as i32;
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw * 3 - (16.0 * k) as i32, top: by, right: rect.right - (16.0 * k) as i32 - bw * 2 - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_START, "开始查找", true, true);
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw * 2 - (8.0 * k) as i32, top: by, right: rect.right - (16.0 * k) as i32 - bw - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_CLEAR, "清除", false, true);
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw, top: by, right: rect.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_CLOSE, "关闭", false, true);
        }
        DlgKind::NewCat | DlgKind::RenameCat => {
            txt(hdc, &lab(44.0), "类别名称(&N):", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            if let Some(e) = ed {
                fill(hdc, &e, s.skin.canvas);
                frame(hdc, &e, s.skin.line);
            }
            let by = rect.bottom - (42.0 * k) as i32;
            let bw = (74.0 * k) as i32;
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw * 2 - (8.0 * k) as i32, top: by, right: rect.right - (16.0 * k) as i32 - bw - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_OK, "确定", true, true);
            dlg_button(hdc, a, s, RECT { left: rect.right - (16.0 * k) as i32 - bw, top: by, right: rect.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 }, D_CANCEL, "取消", false, true);
        }
        DlgKind::SimSearch => {
            // 这一页是干活用的窗：参数改了就改了，不必"确定"，点了"开始查找"才去扫指纹。
            // 所以状态全放在 App 上，读出来先 clone 成局部值，画完再登记命中区。
            // 档位用"目标档"：图标/列表这些小格档自己没有指纹，借库里账最多的一档。
            let (bw, bh) = sim_target_tier(a).unwrap_or_else(|| a.tbox());
            let qpath = a.sim_path.clone();
            let libmsg = a.sim_lib_msg.clone();
            let (pct, algo) = (a.sim_pct, a.sim_algo);
            let (rot, flip) = (a.sim_rot, a.sim_flip);
            let busy = a.sim_busy;
            let ms = a.sim_ms;
            let nhits = a.sim_hits.len();
            let sel = if nhits == 0 { usize::MAX } else { a.sim_sel.min(nhits - 1) };
            let (src_img, hit_img) = (a.sim_src_img.clone(), a.sim_hit_img.clone());
            let (fname, dims) = if qpath.is_empty() {
                ("还没选图片".to_string(), String::new())
            } else {
                let n = Path::new(&qpath)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| qpath.clone());
                // 原图尺寸只在换图时取一次：拖动浮层时一帧一取，光开文件头就够卡。
                if a.sim_qinfo_path != qpath {
                    a.sim_qinfo_path = qpath.clone();
                    a.sim_qinfo = match job::open_size(&qpath) {
                        Ok((w, h)) => format!("原图 {w}×{h}"),
                        Err(_) => "原图打不开".to_string(),
                    };
                }
                (n, a.sim_qinfo.clone())
            };
            let bits = sim::bits_of(algo);
            let allow = sim::dist_from_pct(pct, algo);
            let nvar = 1 + usize::from(rot) * 3 + usize::from(flip) * 2;
            let f = a.font;
            let fsm = a.font_sm;
            let rr = |x0: f64, y0: f64, x1: f64, y1: f64| RECT {
                left: rect.left + (x0 * k) as i32,
                top: rect.top + (y0 * k) as i32,
                right: rect.left + (x1 * k) as i32,
                bottom: rect.top + (y1 * k) as i32,
            };
            let pane = |x0: f64, y0: f64, x1: f64, y1: f64, head: &str| {
                let r = rr(x0, y0, x1, y1);
                fill(hdc, &r, s.skin.pane);
                frame(hdc, &r, s.skin.line_soft);
                let h = RECT { left: r.left + 1, top: r.top + 1, right: r.right - 1, bottom: r.top + (24.0 * k) as i32 };
                fill(hdc, &h, s.skin.pane_head);
                hline(hdc, h.left, h.right, h.bottom, s.skin.line);
                txt(
                    hdc,
                    &RECT { left: h.left + (8.0 * k) as i32, top: h.top, right: h.right - (8.0 * k) as i32, bottom: h.bottom },
                    head,
                    s.skin.ink,
                    f,
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                );
            };
            let say = |x0: f64, y0: f64, x1: f64, t: &str, c: u32, bold: bool| {
                let r = rr(x0, y0, x1, y0 + 18.0);
                txt(hdc, &r, t, c, if bold { f } else { fsm }, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            };
            // 算法那个下拉展开后浮在下面的勾选和「开始查找」上面。命中区得抢在它们
            // 之前登记，不然点开的是浮着的第三行、改的却是被压住的那个控件。
            if combo_open {
                for i in 0..3usize {
                    a.dlg_btns
                        .push((D_SIM_ALGO_ITEM + i, rr(92.0, 270.0 + i as f64 * 26.0, 302.0, 296.0 + i as f64 * 26.0)));
                }
            }

            // ── 检索条件 ────────────────────────────────────────────────
            pane(16.0, 40.0, 536.0, 344.0, "检索条件");
            say(24.0, 70.0, 300.0, "用于检索的图片", s.skin.ink_dim, false);
            let pr = rr(24.0, 90.0, 384.0, 114.0);
            dashed(hdc, &pr, if qpath.is_empty() { s.skin.line_soft } else { s.skin.line });
            txt(
                hdc,
                &RECT { left: pr.left + (6.0 * k) as i32, top: pr.top, right: pr.right - (6.0 * k) as i32, bottom: pr.bottom },
                if qpath.is_empty() { "（还没选图片）" } else { &qpath },
                if qpath.is_empty() { s.skin.ink_off } else { s.skin.ink },
                fsm,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_PATH_ELLIPSIS,
            );
            dlg_button(hdc, a, s, rr(392.0, 90.0, 536.0, 114.0), D_SIM_PICK, "选择图片", false, true);
            dlg_button(hdc, a, s, rr(392.0, 118.0, 536.0, 142.0), D_SIM_CLIP, "从剪贴板", false, true);
            let pv = rr(24.0, 118.0, 120.0, 186.0);
            fill(hdc, &pv, s.skin.canvas);
            frame(hdc, &pv, s.skin.line);
            if let Some(im) = &src_img {
                let box_ = box_in(im.w, im.h, RECT { left: pv.left + 2, top: pv.top + 2, right: pv.right - 2, bottom: pv.bottom - 2 });
                dib(hdc, &box_, im);
            }
            // 名字一长就顶到右边，没有省略号会被从中间截断；这里按尾部省略，
            // 整条路径上面那一行本来就写得全。
            let name_line =
                format!("{}{}", fname, if dims.is_empty() { String::new() } else { format!("  ·  {dims}") });
            txt(
                hdc,
                &rr(128.0, 120.0, 384.0, 138.0),
                &name_line,
                s.skin.ink,
                f,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            let fp_line = format!("指纹档 {bw}×{bh}  ·  比 {bits} 位");
            say(128.0, 140.0, 384.0, &fp_line, s.skin.ink_dim, false);
            say(
                128.0,
                160.0,
                384.0,
                &(if ms == 0 && nhits == 0 {
                    "还没查过".to_string()
                } else {
                    format!("上一次 {ms} 毫秒，命中 {nhits} 张")
                }),
                s.skin.ink_dim,
                false,
            );
            say(24.0, 192.0, 86.0, "相似度", s.skin.ink, true);
            dlg_slider(hdc, a, s, rr(92.0, 192.0, 380.0, 218.0), D_SIM_SLIDER, pct as i32, 50, 100);
            txt(
                hdc,
                &rr(386.0, 192.0, 460.0, 218.0),
                &format!("{pct}%"),
                s.skin.ink,
                f,
                DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
            );
            say(
                24.0,
                222.0,
                536.0,
                &format!(
                    "= 允许差 {allow} 位 / 共 {bits} 位；方向勾选要多算 {} 份指纹取最像的",
                    nvar - 1
                ),
                s.skin.ink_dim,
                false,
            );
            say(24.0, 246.0, 86.0, "算法", s.skin.ink, true);
            let algo_name = match algo & sim::ALGO_ALL {
                sim::ALGO_D => "只看骨架（dHash 256 位）",
                sim::ALGO_C => "只看内容（DCT 63 位）",
                _ => "全部（两种都算）",
            };
            let cr = rr(92.0, 246.0, 302.0, 270.0);
            dlg_combo(hdc, a, s, cr, D_SIM_ALGO, algo_name);
            dlg_check(hdc, a, s, rr(24.0, 274.0, 270.0, 296.0), D_SIM_ROT, "查找转过的（90°/180°/270°）", rot);
            dlg_check(hdc, a, s, rr(278.0, 274.0, 536.0, 296.0), D_SIM_FLIP, "查找镜像的（左右/上下）", flip);
            dlg_button(
                hdc,
                a,
                s,
                rr(24.0, 306.0, 536.0, 330.0),
                D_SIM_GO,
                if busy { "查找中…" } else { "开始查找" },
                true,
                !busy && !qpath.is_empty(),
            );
            if combo_open {
                let items = [
                    (sim::ALGO_ALL, "全部（两种都算）"),
                    (sim::ALGO_D, "只看骨架（dHash 256 位）"),
                    (sim::ALGO_C, "只看内容（DCT 63 位）"),
                ];
                for (i, (v, name)) in items.iter().enumerate() {
                    let r = rr(92.0, 270.0 + i as f64 * 26.0, 302.0, 270.0 + (i as f64 + 1.0) * 26.0);
                    fill(hdc, &r, if over(&r, a.pt.0, a.pt.1) { s.skin.hov } else { s.skin.canvas });
                    frame(hdc, &r, s.skin.line_soft);
                    txt(hdc, &RECT { left: r.left + (7.0 * k) as i32, top: r.top, right: r.right - (8.0 * k) as i32, bottom: r.bottom }, *name, if *v == algo & sim::ALGO_ALL { s.skin.acc } else { s.skin.ink }, fsm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                }
            }

            // ── 结果表 ──────────────────────────────────────────────────
            pane(
                16.0,
                352.0,
                536.0,
                584.0,
                &format!("搜索结果 · {nhits} 张{}", if busy { "（查找中…）" } else { "" }),
            );
            // 五列：名称 / 所在文件夹 / 匹配度 / 差几位 / 大小。窄页放不下改期，
            // 那列信息量也最低，砍掉；行结构里 mtime 留着不显示。
            let cols = [
                (24.0, 300.0, "名称", false),
                (300.0, 376.0, "所在文件夹", false),
                (376.0, 430.0, "匹配度", true),
                (430.0, 466.0, "差位", true),
                (466.0, 533.0, "大小", true),
            ];
            let hr = rr(17.0, 377.0, 533.0, 396.0);
            fill(hdc, &hr, s.skin.chrome);
            hline(hdc, hr.left, hr.right, hr.bottom - 1, s.skin.line);
            for (x0, x1, name, right) in cols {
                let base = if right { DT_RIGHT } else { DT_LEFT };
                txt(hdc, &rr(x0, 377.0, x1 - 4.0, 396.0), name, s.skin.ink_dim, fsm, base | DT_SINGLELINE | DT_VCENTER);
            }
            let body = rr(17.0, 396.0, 533.0, 572.0);
            a.sim_list = body;
            fill(hdc, &body, s.skin.canvas);
            let rh = (25.0 * k) as i32;
            let visible = (((body.bottom - body.top) / rh).max(1) as usize).min(SIM_ROWS);
            let scroll = a.sim_scroll.clamp(0, nhits.saturating_sub(visible) as i32) as usize;
            for i in 0..visible {
                let idx = scroll + i;
                if idx >= nhits {
                    break;
                }
                let top = body.top + i as i32 * rh;
                let row = RECT { left: body.left, top, right: body.right, bottom: top + rh };
                let (path, dist) = (a.sim_hits[idx].path.clone(), a.sim_hits[idx].dist);
                let (size, _mtime) = (a.sim_hits[idx].size, a.sim_hits[idx].mtime);
                let on = idx == sel;
                if on {
                    fill(hdc, &row, s.skin.sel);
                } else if over(&row, a.pt.0, a.pt.1) {
                    fill(hdc, &row, s.skin.hov);
                }
                let ink = if on { s.skin.sel_ink } else { s.skin.ink };
                let dim = if on { mix(s.skin.sel_ink, s.skin.canvas, 0.35) } else { s.skin.ink_dim };
                let cell = |ci: usize| RECT {
                    left: rect.left + (cols[ci].0 * k) as i32,
                    top,
                    right: rect.left + ((cols[ci].1 - 4.0) * k) as i32,
                    bottom: top + rh,
                };
                let name = Path::new(&path)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone());
                txt(hdc, &cell(0), &name, ink, fsm, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_PATH_ELLIPSIS);
                txt(hdc, &cell(1), &sim_parent(&path), dim, fsm, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_PATH_ELLIPSIS);
                let pctrow = sim::pct_from_dist(dist, algo);
                txt(hdc, &cell(2), &format!("{pctrow}%"), ink, fsm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
                txt(hdc, &cell(3), &format!("{dist}"), dim, fsm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
                txt(hdc, &cell(4), &fmt_size(size), dim, fsm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
                a.dlg_rows.push((D_SIM_ROW + i, row));
            }
            if nhits > visible {
                let track = RECT { left: body.right - (8.0 * k) as i32, top: body.top + 2, right: body.right - 2, bottom: body.bottom - 2 };
                fill(hdc, &track, s.skin.chrome);
                let span = (track.bottom - track.top).max(1) as f64;
                let th = (span * visible as f64 / nhits as f64).max(18.0) as i32;
                let maxs = (nhits - visible).max(1) as f64;
                let y0 = track.top + ((track.bottom - track.top - th) as f64 * (scroll as f64 / maxs)) as i32;
                fill(
                    hdc,
                    &RECT { left: track.left + 1, top: y0, right: track.right - 1, bottom: (y0 + th).min(track.bottom) },
                    s.skin.acc_soft,
                );
            }

            // ── 两张预览 ────────────────────────────────────────────────
            say(24.0, 590.0, 270.0, "源图片", s.skin.ink_dim, false);
            say(278.0, 590.0, 533.0, if sel == usize::MAX { "匹配图片" } else { "匹配图片（选中那一张）" }, s.skin.ink_dim, false);
            for (r, im) in [
                (rr(24.0, 610.0, 270.0, 698.0), &src_img),
                (rr(278.0, 610.0, 533.0, 698.0), &hit_img),
            ] {
                fill(hdc, &r, s.skin.canvas);
                frame(hdc, &r, s.skin.line);
                if let Some(im) = im {
                    let inner = RECT { left: r.left + 2, top: r.top + 2, right: r.right - 2, bottom: r.bottom - 2 };
                    let b = box_in(im.w, im.h, inner);
                    dib(hdc, &b, im);
                }
            }

            // ── 底部 ────────────────────────────────────────────────────
            txt(
                hdc,
                &rr(24.0, 704.0, 380.0, 728.0),
                if libmsg.is_empty() {
                    "格子右键也能直接开这一页，带着选中的那张图。"
                } else {
                    &libmsg
                },
                s.skin.ink_dim,
                fsm,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            dlg_button(hdc, a, s, rr(456.0, 704.0, 536.0, 728.0), D_CLOSE, "关闭", false, true);
        }
        DlgKind::CatDb => {
            // 分类数据库自己一页：类别/归类/入库时间的统计、库文件位置和检查。
            // 结果框只写检查（和位置更改）的账，缩略图那一套在另一页。
            let dlg_font = a.font;
            let pane = |top: f64, bottom: f64, head: &str| {
                let r = RECT {
                    left: lx,
                    top: rect.top + (top * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (bottom * k) as i32,
                };
                fill(hdc, &r, s.skin.pane);
                frame(hdc, &r, s.skin.line_soft);
                let hd = RECT {
                    left: r.left + 1,
                    top: r.top + 1,
                    right: r.right - 1,
                    bottom: r.top + (24.0 * k) as i32,
                };
                fill(hdc, &hd, s.skin.pane_head);
                hline(hdc, hd.left, hd.right, hd.bottom, s.skin.line);
                txt(
                    hdc,
                    &RECT {
                        left: hd.left + (8.0 * k) as i32,
                        top: hd.top,
                        right: hd.right - (8.0 * k) as i32,
                        bottom: hd.bottom,
                    },
                    head,
                    s.skin.ink,
                    dlg_font,
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                );
            };
            let cat_sz = std::fs::metadata(catalog_file()).map(|m| m.len()).unwrap_or(0);

            pane(42.0, 282.0, "分类数据库");
            let mut cat_line = "还在读取中…".to_string();
            if let Some(cat) = a.cat.as_ref() {
                cat_line = format!(
                    "类别：{} 个（顶层 {}） · 归类：{} 条 · 记了入库时间：{} 张",
                    cat.n_cats(),
                    cat.roots.len(),
                    cat.n_rows(),
                    cat.n_assets()
                );
            }
            txt(
                hdc,
                &RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (76.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (98.0 * k) as i32,
                },
                &cat_line,
                s.skin.ink,
                a.font,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            let mv = RECT {
                left: rect.right - (16.0 * k) as i32 - (88.0 * k) as i32,
                top: rect.top + (102.0 * k) as i32,
                right: rect.right - (16.0 * k) as i32,
                bottom: rect.top + (126.0 * k) as i32,
            };
            txt(
                hdc,
                &RECT {
                    left: lx + (14.0 * k) as i32,
                    top: mv.top,
                    right: mv.left - (10.0 * k) as i32,
                    bottom: mv.bottom,
                },
                &format!(
                    "库文件：{} · 位置：{}",
                    fmt_size(cat_sz),
                    data_dir().display()
                ),
                s.skin.ink,
                a.font,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            dlg_button(hdc, a, s, mv, D_DB_MOVE, "更改…", false, !a.db_busy);
            txt(
                hdc,
                &RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (132.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (164.0 * k) as i32,
                },
                "检查会核对分类库里的全部资产路径：硬盘上已经不存在的记录，\n先自动备份原库，再从库中剔除。",
                s.skin.ink_dim,
                a.font_sm,
                DT_LEFT | DT_WORDBREAK,
            );
            dlg_button(
                hdc,
                a,
                s,
                RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (166.0 * k) as i32,
                    right: lx + (118.0 * k) as i32,
                    bottom: rect.top + (192.0 * k) as i32,
                },
                D_DBCHECK,
                if a.db_busy { "请稍候…" } else { "检查数据库" },
                true,
                !a.db_busy && !a.scan_busy,
            );
            txt(
                hdc,
                &RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (200.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (220.0 * k) as i32,
                },
                "结果",
                s.skin.ink,
                a.font,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            let res = RECT {
                left: lx + (14.0 * k) as i32,
                top: rect.top + (222.0 * k) as i32,
                right: rect.right - (16.0 * k) as i32,
                bottom: rect.top + (270.0 * k) as i32,
            };
            fill(hdc, &res, s.skin.canvas);
            frame(hdc, &res, s.skin.line_soft);
            let none = a.db_maint.is_empty();
            txt(
                hdc,
                &RECT {
                    left: res.left + (8.0 * k) as i32,
                    top: res.top + (8.0 * k) as i32,
                    right: res.right - (8.0 * k) as i32,
                    bottom: res.bottom - (8.0 * k) as i32,
                },
                &wrap_chars(if none { "还没有结果。" } else { &a.db_maint }, 42),
                if none { s.skin.ink_dim } else { s.skin.ink },
                a.font_sm,
                DT_LEFT | DT_WORDBREAK | DT_TOP,
            );
            let by = rect.bottom - (42.0 * k) as i32;
            let bh = (26.0 * k) as i32;
            dlg_button(
                hdc,
                a,
                s,
                RECT {
                    left: rect.right - (16.0 * k) as i32 - (74.0 * k) as i32,
                    top: by,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: by + bh,
                },
                D_CLOSE,
                "关闭",
                false,
                true,
            );
        }
        DlgKind::ThumbDb => {
            // 缩略图数据库自己一页：缓存统计、重建、清指纹和扫描文件夹，
            // 以图搜图的指纹也在这一本账里。结果框只写这一页的账。
            let dlg_font = a.font;
            let pane = |top: f64, bottom: f64, head: &str| {
                let r = RECT {
                    left: lx,
                    top: rect.top + (top * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (bottom * k) as i32,
                };
                fill(hdc, &r, s.skin.pane);
                frame(hdc, &r, s.skin.line_soft);
                let hd = RECT {
                    left: r.left + 1,
                    top: r.top + 1,
                    right: r.right - 1,
                    bottom: r.top + (24.0 * k) as i32,
                };
                fill(hdc, &hd, s.skin.pane_head);
                hline(hdc, hd.left, hd.right, hd.bottom, s.skin.line);
                txt(
                    hdc,
                    &RECT {
                        left: hd.left + (8.0 * k) as i32,
                        top: hd.top,
                        right: hd.right - (8.0 * k) as i32,
                        bottom: hd.bottom,
                    },
                    head,
                    s.skin.ink,
                    dlg_font,
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                );
            };
            let (tn, tb) = cache::stats();
            pane(42.0, 592.0, "缩略图数据库（以图搜图的指纹也在这一本账里）");
            txt(
                hdc,
                &RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (76.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (98.0 * k) as i32,
                },
                &format!(
                    "缩略图缓存：{} 条 · {}（指纹跟着缩略图索引走，不另设库）",
                    tn,
                    fmt_size(tb)
                ),
                s.skin.ink,
                a.font,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            dlg_button(
                hdc,
                a,
                s,
                RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (102.0 * k) as i32,
                    right: lx + (144.0 * k) as i32,
                    bottom: rect.top + (128.0 * k) as i32,
                },
                D_THUMB_FIX,
                if a.fix_mode { "停止重建" } else { "重建当前目录" },
                false,
                !a.thumb_busy,
            );
            dlg_button(
                hdc,
                a,
                s,
                RECT {
                    left: lx + (152.0 * k) as i32,
                    top: rect.top + (102.0 * k) as i32,
                    right: lx + (244.0 * k) as i32,
                    bottom: rect.top + (128.0 * k) as i32,
                },
                D_THUMB_ALL,
                if a.thumb_busy { "停止重建" } else { "重建全部" },
                false,
                !a.fix_mode,
            );
            dlg_button(
                hdc,
                a,
                s,
                RECT {
                    left: lx + (252.0 * k) as i32,
                    top: rect.top + (102.0 * k) as i32,
                    right: lx + (372.0 * k) as i32,
                    bottom: rect.top + (128.0 * k) as i32,
                },
                D_DB_PRUNE,
                if a.db_busy { "请稍候…" } else { "移除无效索引" },
                false,
                !a.db_busy && !a.scan_busy && !a.thumb_busy && !a.fix_mode && !a.sim_busy,
            );
            // 以图搜图的自动补库开关从那一页搬过来：勾上后每次查找前先补指纹。
            dlg_check(
                hdc,
                a,
                s,
                RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (134.0 * k) as i32,
                    right: lx + (260.0 * k) as i32,
                    bottom: rect.top + (156.0 * k) as i32,
                },
                D_SIM_AUTO,
                "查找之前先补齐（以图搜图先补库再搜）",
                a.sim_auto,
            );
            txt(
                hdc,
                &RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (160.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (192.0 * k) as i32,
                },
                "重建：每张绕开缓存重解一遍，和缓存里那张对不上的当场换掉；当前目录只动这一列，全部照库里的名单走。\n\
                 移除无效索引：摘掉原图已不在盘上的指纹，缩略图保留，全部档位一次清完。",
                s.skin.ink_dim,
                a.font_sm,
                DT_LEFT | DT_WORDBREAK,
            );
            txt(
                hdc,
                &RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (198.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (218.0 * k) as i32,
                },
                "扫描文件夹（含子文件夹）",
                s.skin.ink,
                a.font_b,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            // ── 扫描文件夹：名单 + 增删/开扫/停止 ─────────────────────
            let scan_dirs = a.scan_dirs.clone();
            let scan_sel = a.scan_sel;
            let scan_scroll = a.scan_scroll;
            let scan_busy = a.scan_busy;
            // 库上只能有一件重活：重建/检查/找相似在跑就都别动扫描。
            let idle = !a.scan_busy
                && !a.db_busy
                && !a.thumb_busy
                && !a.fix_mode
                && !a.sim_busy;
            let list = RECT {
                left: lx,
                top: rect.top + (220.0 * k) as i32,
                right: rect.right - (16.0 * k) as i32,
                bottom: rect.top + (332.0 * k) as i32,
            };
            a.scan_list = list;
            fill(hdc, &list, s.skin.canvas);
            frame(hdc, &list, s.skin.line);
            let row_h = (22.0 * k) as i32;
            let maxs = scan_dirs.len().saturating_sub(SCAN_VIS);
            let top_row = scan_scroll.min(maxs);
            if scan_dirs.is_empty() {
                txt(
                    hdc,
                    &list,
                    "（名单是空的，先点「添加…」挑一个文件夹）",
                    s.skin.ink_off,
                    a.font_sm,
                    DT_CENTER | DT_SINGLELINE | DT_VCENTER,
                );
            }
            for vi in 0..SCAN_VIS {
                let idx = top_row + vi;
                if idx >= scan_dirs.len() {
                    break;
                }
                let root = &scan_dirs[idx];
                let (p, t) = (&root.path, root.last);
                let row = RECT {
                    left: list.left + 1,
                    top: list.top + 1 + vi as i32 * row_h,
                    right: list.right - 1,
                    bottom: (list.top + 1 + (vi as i32 + 1) * row_h).min(list.bottom - 1),
                };
                let sel = idx == scan_sel;
                fill(hdc, &row, if sel { s.skin.hov } else { s.skin.canvas });
                if sel {
                    frame(hdc, &row, s.skin.acc);
                }
                let when = if t == 0 {
                    "还没扫过".to_string()
                } else {
                    format!("上次 {}", local(t))
                };
                // 名单上看得见哪条带了规则：有的在时间前面挂一枚"有规则"的牌。
                if !root.dirs.is_empty() || !root.exts.is_empty() {
                    txt(
                        hdc,
                        &RECT {
                            left: row.right - (168.0 * k) as i32,
                            top: row.top,
                            right: row.right - (122.0 * k) as i32,
                            bottom: row.bottom,
                        },
                        "有规则",
                        s.skin.acc,
                        a.font_sm,
                        DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
                    );
                }
                txt(
                    hdc,
                    &RECT {
                        left: row.left + (6.0 * k) as i32,
                        top: row.top,
                        right: row.right - (172.0 * k) as i32,
                        bottom: row.bottom,
                    },
                    p,
                    s.skin.ink,
                    a.font_sm,
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_PATH_ELLIPSIS,
                );
                txt(
                    hdc,
                    &RECT {
                        left: row.right - (114.0 * k) as i32,
                        top: row.top,
                        right: row.right - (6.0 * k) as i32,
                        bottom: row.bottom,
                    },
                    &when,
                    s.skin.ink_off,
                    a.font_sm,
                    DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
                );
                a.dlg_btns.push((D_SCAN_ROW + vi, row));
            }
            let sb_y = rect.top + (338.0 * k) as i32;
            let sb_h = (26.0 * k) as i32;
            dlg_button(hdc, a, s, RECT { left: lx, top: sb_y, right: lx + (88.0 * k) as i32, bottom: sb_y + sb_h }, D_SCAN_ADD, "添加…", false, !scan_busy);
            dlg_button(hdc, a, s, RECT { left: lx + (96.0 * k) as i32, top: sb_y, right: lx + (166.0 * k) as i32, bottom: sb_y + sb_h }, D_SCAN_DEL, "移除", false, scan_sel < scan_dirs.len());
            dlg_button(hdc, a, s, RECT { left: lx + (174.0 * k) as i32, top: sb_y, right: lx + (280.0 * k) as i32, bottom: sb_y + sb_h }, D_SCAN_RUN, "扫描所选", false, idle && scan_sel < scan_dirs.len());
            dlg_button(hdc, a, s, RECT { left: lx + (288.0 * k) as i32, top: sb_y, right: lx + (394.0 * k) as i32, bottom: sb_y + sb_h }, D_SCAN_ALL, "全部扫描", false, idle && !scan_dirs.is_empty());
            dlg_button(hdc, a, s, RECT { left: lx + (402.0 * k) as i32, top: sb_y, right: lx + (468.0 * k) as i32, bottom: sb_y + sb_h }, D_SCAN_STOP, "停止", false, scan_busy);
            dlg_button(hdc, a, s, RECT { left: lx + (476.0 * k) as i32, top: sb_y, right: lx + (564.0 * k) as i32, bottom: sb_y + sb_h }, D_SCAN_RULES, "规则…", false, scan_sel < scan_dirs.len());
            txt(
                hdc,
                &RECT {
                    left: lx + (14.0 * k) as i32,
                    top: rect.top + (370.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (390.0 * k) as i32,
                },
                "每个文件夹可带排除规则（「规则…」），命中的贴图文件夹整棵剪掉、不入搜图库。",
                s.skin.ink_dim,
                a.font_sm,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            txt(
                hdc,
                &RECT {
                    left: lx,
                    top: rect.top + (398.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (418.0 * k) as i32,
                },
                "结果",
                s.skin.ink,
                a.font_b,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            let res = RECT {
                left: lx,
                top: rect.top + (420.0 * k) as i32,
                right: rect.right - (16.0 * k) as i32,
                bottom: rect.bottom - (58.0 * k) as i32,
            };
            fill(hdc, &res, s.skin.canvas);
            frame(hdc, &res, s.skin.line_soft);
            let mut text = String::new();
            if !a.db_prune.is_empty() {
                text.push_str(&a.db_prune);
            }
            // 全部重建跑在后台线程上，进度就从那两个计数现读，不等它推消息。
            let going = a.thumb_busy;
            if going {
                text.push_str(&format!(
                    "正在重做全部缩略图：{} / {}，已发现对不上 {} 张。",
                    a.thumb_done.load(Ordering::Relaxed),
                    a.thumb_total,
                    a.thumb_bad.load(Ordering::Relaxed)
                ));
            } else if !a.db_thumb.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&a.db_thumb);
            }
            // 扫描文件夹的进度同样现读；跑完落进 db_scan，跟检查/重建各记各的。
            if a.scan_busy {
                if !text.is_empty() {
                    text.push('\n');
                }
                if a.scan_total == 0 {
                    text.push_str("正在翻文件夹找图…");
                } else {
                    text.push_str(&format!(
                        "正在扫描：{} / {}，新增 {}，跳过 {}，失败 {}。",
                        a.scan_done.load(Ordering::Relaxed),
                        a.scan_total,
                        a.scan_added.load(Ordering::Relaxed),
                        a.scan_skip.load(Ordering::Relaxed),
                        a.scan_err.load(Ordering::Relaxed)
                    ));
                }
            } else if !a.db_scan.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&a.db_scan);
            }
            let none = text.is_empty();
            txt(
                hdc,
                &RECT {
                    left: res.left + (8.0 * k) as i32,
                    top: res.top + (8.0 * k) as i32,
                    right: res.right - (8.0 * k) as i32,
                    bottom: res.bottom - (8.0 * k) as i32,
                },
                &wrap_chars(if none { "还没有结果。" } else { &text }, 42),
                if none { s.skin.ink_dim } else { s.skin.ink },
                a.font_sm,
                DT_LEFT | DT_WORDBREAK | DT_TOP,
            );
            let by = rect.bottom - (42.0 * k) as i32;
            let bh = (26.0 * k) as i32;
            // 底下只留关闭：四颗库操作按钮已经各自归进上面的板块里。
            dlg_button(
                hdc,
                a,
                s,
                RECT {
                    left: rect.right - (16.0 * k) as i32 - (74.0 * k) as i32,
                    top: by,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: by + bh,
                },
                D_CLOSE,
                "关闭",
                false,
                true,
            );
        }
        DlgKind::ScanRule => {
            // 选中那条扫描根的排除规则。两个输入框是原生控件，字在框里自己长，
            // 这里只管说明、摆位和底下那排按钮。
            let (root, has_root) = match a.scan_dirs.get(a.scan_sel) {
                Some(r) => (r.path.clone(), true),
                None => (String::new(), false),
            };
            let edit1 = RECT {
                left: lx,
                top: rect.top + (94.0 * k) as i32,
                right: rect.right - (16.0 * k) as i32,
                bottom: rect.top + (122.0 * k) as i32,
            };
            let edit2 = RECT {
                left: lx,
                top: rect.top + (154.0 * k) as i32,
                right: rect.right - (16.0 * k) as i32,
                bottom: rect.top + (182.0 * k) as i32,
            };
            txt(
                hdc,
                &RECT {
                    left: lx,
                    top: rect.top + (44.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (66.0 * k) as i32,
                },
                &format!("作用对象：{root}"),
                s.skin.ink_dim,
                a.font_sm,
                DT_LEFT | DT_SINGLELINE | DT_PATH_ELLIPSIS,
            );
            txt(
                hdc,
                &RECT {
                    left: lx,
                    top: rect.top + (72.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (92.0 * k) as i32,
                },
                "排除文件夹关键字（逗号分隔；文件夹名包含即命中，不分大小写）：",
                s.skin.ink,
                a.font_sm,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            place_rule_edits(a, &[edit1, edit2]);
            txt(
                hdc,
                &RECT {
                    left: lx,
                    top: rect.top + (130.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (150.0 * k) as i32,
                },
                "排除文件格式（逗号分隔、不带点；后缀精确命中）：",
                s.skin.ink,
                a.font_sm,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            txt(
                hdc,
                &RECT {
                    left: lx,
                    top: rect.top + (190.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (214.0 * k) as i32,
                },
                "保存后自动把库里已入账、命中规则的指纹摘掉（缩略图保留，浏览贴图不受影响）。",
                s.skin.ink_dim,
                a.font_sm,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            let by = rect.bottom - (42.0 * k) as i32;
            let bh = (26.0 * k) as i32;
            let bw = (120.0 * k) as i32;
            let gap = (8.0 * k) as i32;
            let close = RECT {
                left: rect.right - (16.0 * k) as i32 - (74.0 * k) as i32,
                top: by,
                right: rect.right - (16.0 * k) as i32,
                bottom: by + bh,
            };
            let save = RECT {
                left: close.left - gap - bw,
                top: by,
                right: close.left - gap,
                bottom: by + bh,
            };
            dlg_button(hdc, a, s, save, D_RULE_SAVE, "保存并清理已入账", true, has_root);
            dlg_button(hdc, a, s, close, D_CLOSE, "关闭", false, true);
        }
        DlgKind::Assoc => {
            // 顶部这行是必须的：勾选改的不是"默认应用"，Win10/11 把那一层锁给了系统设置。
            // 说明摆第一行，底下才是能勾的东西，免得人以为点完「应用」双击就归 Xsee 了。
            txt(
                hdc,
                &RECT {
                    left: lx,
                    top: rect.top + (40.0 * k) as i32,
                    right: rect.right - (16.0 * k) as i32,
                    bottom: rect.top + (98.0 * k) as i32,
                },
                "Windows 10/11 不允许程序静默修改默认应用。这里的勾选只是把 Xsee 登记成这些格式的「打开方式」候选，\
                 双击文件时用的仍是系统当前的默认程序。\n\
                 要真正设成默认：点下面「打开默认应用设置」，在系统设置里按文件类型选 Xsee。",
                s.skin.ink_dim,
                a.font_sm,
                DT_LEFT | DT_WORDBREAK | DT_TOP,
            );
            // 按格式牌归组，和浏览窗口、资源管理器里看到的牌一一对应：
            // 0..8 是常用格式，9 是 RAW 家族，10 起是后来接的专业格式。
            let list = a.assoc.clone();
            let mut groups: [Vec<usize>; 3] = [Vec::new(), Vec::new(), Vec::new()];
            for (i, (ext, _)) in list.iter().enumerate() {
                let g = match fmticon::index(ext) {
                    Some(9) => 1,
                    Some(f) if f >= 10 => 2,
                    _ => 0,
                };
                groups[g].push(i);
            }
            let titles = ["常用图片格式", "相机 RAW 格式", "其他图像格式"];
            let cols = 6usize;
            let colw = (rect.right - (16.0 * k) as i32 - lx) / cols as i32;
            let rh = (26.0 * k) as i32;
            let hh = (21.0 * k) as i32;
            let isz = (22.0 * k) as i32;
            let cbx = (13.0 * k) as i32;
            let icon_bg = s.skin.chrome_hi;
            let mut y = rect.top + (102.0 * k) as i32;
            let list_right = rect.right - (16.0 * k) as i32;
            for (gi, idxs) in groups.iter().enumerate() {
                let hr = RECT { left: lx, top: y, right: list_right, bottom: y + hh };
                txt(hdc, &hr, titles[gi], s.skin.ink, a.font_b, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                txt(hdc, &hr, &format!("{} 个扩展名", idxs.len()), s.skin.ink_dim, a.font_sm, DT_RIGHT | DT_SINGLELINE | DT_VCENTER);
                y += hh;
                for (n, &ii) in idxs.iter().enumerate() {
                    let left = lx + (n % cols) as i32 * colw;
                    let top = y + (n / cols) as i32 * rh;
                    let r = RECT { left, top, right: left + colw - (4.0 * k) as i32, bottom: top + rh };
                    if over(&r, a.pt.0, a.pt.1) {
                        fill(hdc, &r, s.skin.hov);
                    }
                    let (ext, on) = &list[ii];
                    // 勾选框复用标准控件（命中区登记在整格上），右边补格式牌和扩展名。
                    dlg_check(hdc, a, s, r, D_ASSOC + ii, "", *on);
                    let ix = r.left + cbx + (5.0 * k) as i32;
                    let iy = top + (rh - isz) / 2;
                    if let Some(fi) = fmticon::index(ext) {
                        icon_cached(hdc, IK_FMT_LIST + fi as u8, ix, iy, isz, fmticon::pick(fi, icon_bg), icon_bg);
                    }
                    let tr = RECT {
                        left: ix + isz + (5.0 * k) as i32,
                        top,
                        right: r.right,
                        bottom: r.bottom,
                    };
                    txt(hdc, &tr, &format!(".{ext}"), s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                }
                y += (idxs.len().div_ceil(cols)) as i32 * rh;
            }
            let n_on = list.iter().filter(|(_, on)| *on).count();
            if let Ok(exe) = std::env::current_exe() {
                txt(
                    hdc,
                    &RECT {
                        left: lx,
                        top: rect.top + (470.0 * k) as i32,
                        right: list_right - (120.0 * k) as i32,
                        bottom: rect.top + (488.0 * k) as i32,
                    },
                    &format!("程序位置：{}", exe.display()),
                    s.skin.ink_dim,
                    a.font_sm,
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_PATH_ELLIPSIS,
                );
            }
            txt(
                hdc,
                &RECT {
                    left: list_right - (120.0 * k) as i32,
                    top: rect.top + (470.0 * k) as i32,
                    right: list_right,
                    bottom: rect.top + (488.0 * k) as i32,
                },
                &format!("已勾选 {n_on} / {} 项", list.len()),
                s.skin.ink_dim,
                a.font_sm,
                DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
            );
            dlg_check(
                hdc,
                a,
                s,
                RECT {
                    left: lx,
                    top: rect.top + (494.0 * k) as i32,
                    right: lx + (300.0 * k) as i32,
                    bottom: rect.top + (516.0 * k) as i32,
                },
                D_ASSOC_CHECK,
                "每次启动检查并补回这些关联",
                a.assoc_check,
            );
            let res = RECT {
                left: lx,
                top: rect.top + (522.0 * k) as i32,
                right: rect.right - (16.0 * k) as i32,
                bottom: rect.top + (570.0 * k) as i32,
            };
            fill(hdc, &res, s.skin.canvas);
            frame(hdc, &res, s.skin.line_soft);
            let hint = "勾选后点「应用」写入注册表；取消勾选只删 Xsee 自己那一条候选，别的程序不受影响。\n\
                        启动检查只管补回 Xsee 自己的登记，不碰系统默认。";
            let msg = if a.assoc_msg.is_empty() { hint } else { &a.assoc_msg };
            txt(
                hdc,
                &RECT {
                    left: res.left + (8.0 * k) as i32,
                    top: res.top + (8.0 * k) as i32,
                    right: res.right - (8.0 * k) as i32,
                    bottom: res.bottom - (8.0 * k) as i32,
                },
                msg,
                if a.assoc_msg.is_empty() { s.skin.ink_dim } else { s.skin.ink },
                a.font_sm,
                DT_LEFT | DT_WORDBREAK | DT_TOP,
            );
            let by = rect.bottom - (42.0 * k) as i32;
            let bh = (26.0 * k) as i32;
            let gap = (8.0 * k) as i32;
            let close = RECT { left: rect.right - (16.0 * k) as i32 - (74.0 * k) as i32, top: by, right: rect.right - (16.0 * k) as i32, bottom: by + bh };
            let def = RECT { left: close.left - gap - (150.0 * k) as i32, top: by, right: close.left - gap, bottom: by + bh };
            let apply = RECT { left: def.left - gap - (74.0 * k) as i32, top: by, right: def.left - gap, bottom: by + bh };
            dlg_button(hdc, a, s, apply, D_ASSOC_APPLY, "应用", true, true);
            dlg_button(hdc, a, s, def, D_ASSOC_DEFAULTS, "打开默认应用设置", false, true);
            dlg_button(hdc, a, s, close, D_CLOSE, "关闭", false, true);
        }
        DlgKind::Ask => {
            let body = RECT {
                left: lx,
                top: rect.top + (44.0 * k) as i32,
                right: rect.right - (16.0 * k) as i32,
                bottom: rect.bottom - (58.0 * k) as i32,
            };
            txt(hdc, &body, &ask_body, s.skin.ink, a.font, DT_LEFT | DT_WORDBREAK);
            let by = rect.bottom - (42.0 * k) as i32;
            let bw = (74.0 * k) as i32;
            let r1 = RECT { left: rect.right - (16.0 * k) as i32 - bw, top: by, right: rect.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 };
            if ask_yes {
                let r0 = RECT { left: r1.left - bw - (8.0 * k) as i32, top: by, right: r1.left - (8.0 * k) as i32, bottom: by + (26.0 * k) as i32 };
                dlg_button(hdc, a, s, r0, D_OK, "是", true, true);
                dlg_button(hdc, a, s, r1, D_CANCEL, "否", false, true);
            } else {
                dlg_button(hdc, a, s, r1, D_OK, "确定", true, true);
            }
        }
        DlgKind::Skin => {
            // 这个窗里改的一切都先进草稿，再马上叠到界面上：色块、字号、字体和主窗口一起跟着变，
            // 敢随便试。点确定弹一个窗问一句要不要写进皮肤文件，取消就退回打开那一刻。
            let (draft, save_as, arm_del) = a
                .dlg
                .as_ref()
                .map(|d| (d.look_draft.clone(), d.save_as, d.arm_del))
                .unwrap_or_default();
            // 底色取皮肤文件那一层，不是当前生效那张：草稿清空时色块要能回到文件默认色。
            let dk = theme::preview(&theme::skin_of(&a.skin), &draft);
            let tw = (rect.right - rect.left - (24.0 * k) as i32) / 3;
            for (i, name) in ["字体", "配色", "目录"].iter().enumerate() {
                let r = RECT {
                    left: rect.left + (12.0 * k) as i32 + i as i32 * tw,
                    top: rect.top + (40.0 * k) as i32,
                    right: rect.left + (12.0 * k) as i32 + (i as i32 + 1) * tw,
                    bottom: rect.top + (66.0 * k) as i32,
                };
                let on = i == fmt;
                fill(hdc, &r, if on { s.skin.chrome_hi } else { s.skin.chrome });
                frame(hdc, &r, if on { s.skin.acc } else { s.skin.line_soft });
                txt(hdc, &r, name, if on { s.skin.ink } else { s.skin.ink_dim }, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
                a.dlg_rows.push((D_SKIN_TAB + i, r));
            }
            let body = RECT {
                left: rect.left + (12.0 * k) as i32,
                top: rect.top + (70.0 * k) as i32,
                right: rect.right - (12.0 * k) as i32,
                bottom: rect.bottom - (118.0 * k) as i32,
            };
            fill(hdc, &body, s.skin.pane);
            frame(hdc, &body, s.skin.line_soft);
            let bx = body.left + (14.0 * k) as i32;
            let bw2 = body.right - (14.0 * k) as i32;
            let row = |y: f64| RECT {
                left: bx,
                top: body.top + (y * k) as i32,
                right: bw2,
                bottom: body.top + ((y + 22.0) * k) as i32,
            };
            match fmt {
                0 => {
                    txt(hdc, &row(14.0), "界面字体", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                    let fr = RECT {
                        left: bx + (110.0 * k) as i32,
                        top: body.top + (12.0 * k) as i32,
                        right: bw2 - (8.0 * k) as i32,
                        bottom: body.top + (40.0 * k) as i32,
                    };
                    fill(hdc, &fr, if over(&fr, a.pt.0, a.pt.1) { s.skin.hov } else { s.skin.canvas });
                    frame(hdc, &fr, s.skin.line);
                    txt(hdc, &fr, draft.face(), s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                    a.dlg_rows.push((D_FACE, fr));
                    txt(
                        hdc,
                        &RECT { left: bx, top: body.top + (46.0 * k) as i32, right: bw2, bottom: body.top + (64.0 * k) as i32 },
                        "点击字体名打开系统字体选择；数字与路径固定用 Consolas。",
                        s.skin.ink_dim,
                        a.font_sm,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                    );
                    // 十块逐块一档：左列五块、右列五块，每块一行字号加一个加粗勾。
                    txt(
                        hdc,
                        &row(72.0),
                        "版块字号（100% 缩放下的像素）",
                        s.skin.ink,
                        a.font,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                    );
                    let colw = (bw2 - bx - (16.0 * k) as i32) / 2;
                    let per_col = (theme::NZ + 1) / 2;
                    for i in 0..theme::NZ {
                        let (key, cn) = theme::ZONES[i];
                        let (col, ln) = (i / per_col, i % per_col);
                        let left = bx + col as i32 * (colw + (16.0 * k) as i32);
                        let top = body.top + ((96.0 + ln as f64 * 28.0) * k) as i32;
                        let r = RECT { left, top, right: left + colw, bottom: top + (22.0 * k) as i32 };
                        let def = theme::ZONE_DEF[i] + if a.density == "loose" { 1 } else { 0 };
                        let px = draft.zone_px(key).unwrap_or(def);
                        size_row(hdc, a, s, r, cn, px, D_ZONE + i * 3 + ZR_DN, D_ZONE + i * 3 + ZR_UP);
                        // 色块排在“+”右边（194 那档压在 + 上面，看不见加号），加粗再往后挪
                        let cr = RECT {
                            left: left + (248.0 * k) as i32,
                            top: top - (2.0 * k) as i32,
                            right: left + (282.0 * k) as i32,
                            bottom: top + (24.0 * k) as i32,
                        };
                        let chot = over(&cr, a.pt.0, a.pt.1);
                        let csw = RECT {
                            left: cr.left + (2.0 * k) as i32,
                            top: cr.top + (2.0 * k) as i32,
                            right: cr.right - (2.0 * k) as i32,
                            bottom: cr.bottom - (2.0 * k) as i32,
                        };
                        swatch(hdc, &csw, zone_now_ink(i, &dk, draft.zone_color(key)), s, chot);
                        a.dlg_rows.push((D_ZONEC + i, cr));
                        let br = RECT {
                            left: left + (296.0 * k) as i32,
                            top: top - (2.0 * k) as i32,
                            right: left + colw,
                            bottom: top + (24.0 * k) as i32,
                        };
                        dlg_check(hdc, a, s, br, D_ZONE + i * 3 + ZR_BOLD, "加粗", draft.zone_bold(key));
                    }
                    txt(
                        hdc,
                        &RECT {
                            left: bx,
                            top: body.top + (240.0 * k) as i32,
                            right: bw2,
                            bottom: body.top + (258.0 * k) as i32,
                        },
                        "改哪一块只有那一块变；右边的色块改这一块的字色，改动实时叠到界面上，点确定才写进皮肤文件。",
                        s.skin.ink_dim,
                        a.font_sm,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                    );
                }
                1 => {
                    let colw = (bw2 - bx - (16.0 * k) as i32) / 2;
                    let per_col = (COLOR_SLOTS.len() + 1) / 2;
                    for (i, slot) in COLOR_SLOTS.iter().enumerate() {
                        let cx = bx + (i as i32 / per_col as i32) * (colw + (16.0 * k) as i32);
                        let cy = body.top + (10.0 * k) as i32 + (i as i32 % per_col as i32) * (26.0 * k) as i32;
                        let r = RECT { left: cx, top: cy, right: cx + colw, bottom: cy + (22.0 * k) as i32 };
                        let hot = over(&r, a.pt.0, a.pt.1);
                        if hot {
                            fill(hdc, &r, s.skin.hov);
                        }
                        txt(hdc, &r, slot_label(slot), s.skin.ink, a.font_sm, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                        let sw = RECT {
                            left: r.right - (42.0 * k) as i32,
                            top: r.top + (2.0 * k) as i32,
                            right: r.right - (8.0 * k) as i32,
                            bottom: r.bottom - (2.0 * k) as i32,
                        };
                        swatch(hdc, &sw, dk.slot(slot), s, hot);
                        a.dlg_rows.push((D_COLOR + i, r));
                    }
                    txt(
                        hdc,
                        &RECT { left: bx, top: body.bottom - (38.0 * k) as i32, right: bw2, bottom: body.bottom - (10.0 * k) as i32 },
                        "点一行改这个颜色；色块马上跟着变，点确定才写进皮肤文件。文字颜色在「字体」页按版块单独调。",
                        s.skin.ink_dim,
                        a.font_sm,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                    );
                }
                _ => {
                    // 目录页：皮肤从哪里读。选完立刻重扫，不等"确定"——
                    // 它不是一个颜色草稿，换的就是下次从哪里开门。
                    let cur = theme::skin_dir();
                    txt(hdc, &row(14.0), "皮肤文件夹", s.skin.ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
                    let fr = RECT {
                        left: bx + (110.0 * k) as i32,
                        top: body.top + (12.0 * k) as i32,
                        right: bw2 - (8.0 * k) as i32,
                        bottom: body.top + (40.0 * k) as i32,
                    };
                    let fhot = over(&fr, a.pt.0, a.pt.1);
                    fill(hdc, &fr, if fhot { s.skin.hov } else { s.skin.canvas });
                    frame(hdc, &fr, s.skin.line);
                    txt(
                        hdc,
                        &fr,
                        &cur.display().to_string(),
                        s.skin.ink,
                        a.font,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                    );
                    a.dlg_rows.push((D_SKIN_DIR, fr));
                    let fy = body.top + (52.0 * k) as i32;
                    dlg_button(
                        hdc,
                        a,
                        s,
                        RECT { left: bx, top: fy, right: bx + (150.0 * k) as i32, bottom: fy + (26.0 * k) as i32 },
                        D_SKIN_DIR,
                        "选择文件夹…",
                        false,
                        true,
                    );
                    dlg_button(
                        hdc,
                        a,
                        s,
                        RECT { left: bx + (158.0 * k) as i32, top: fy, right: bx + (294.0 * k) as i32, bottom: fy + (26.0 * k) as i32 },
                        D_SKIN_DIR_DEF,
                        "用默认目录",
                        false,
                        !theme::skin_dir_is_default(),
                    );
                    let n = theme::skins().len();
                    txt(
                        hdc,
                        &row(92.0),
                        &format!("现在读到 {n} 套皮肤（含程序自带的那一套），换目录会立刻按新文件夹重扫。"),
                        s.skin.ink_dim,
                        a.font_sm,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                    );
                    txt(
                        hdc,
                        &row(118.0),
                        "这个选择会记住；程序不会主动去建文件夹，目录里没有 .skin 也能用。",
                        s.skin.ink_dim,
                        a.font_sm,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                    );
                    txt(
                        hdc,
                        &row(144.0),
                        if theme::skin_dir_is_default() {
                            "默认先看 exe 旁边的 skins，那里没有才用系统目录 %LOCALAPPDATA%\\xsee\\skins。"
                        } else {
                            "当前用单独指定的目录；点「用默认目录」退回。"
                        },
                        s.skin.ink_dim,
                        a.font_sm,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                    );
                }
            }
            // 底部从上到下三行：另存为的名字框（只在编辑时亮）、皮肤文件动作一排、确定/取消一排。
            // 覆盖与否的那一问不占这一排，是压在窗上的弹窗。
            let nb = RECT {
                left: rect.left + (16.0 * k) as i32,
                top: rect.bottom - (112.0 * k) as i32,
                right: rect.right - (16.0 * k) as i32,
                bottom: rect.bottom - (86.0 * k) as i32,
            };
            if save_as {
                fill(hdc, &nb, s.skin.canvas);
                frame(hdc, &nb, s.skin.acc);
            } else {
                // 另存和删除都只作用在这一套上，所以把名字摊在这里，别让人猜是哪套
                txt(
                    hdc,
                    &nb,
                    &format!("当前皮肤：{}", a.skin),
                    s.skin.ink_dim,
                    a.font_sm,
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                );
            }
            let by1 = rect.bottom - (78.0 * k) as i32;
            let bh1 = by1 + (26.0 * k) as i32;
            let mut px = bx;
            let mut slot = |w: f64| {
                let r = RECT { left: px, top: by1, right: px + (w * k) as i32, bottom: bh1 };
                px = r.right + (8.0 * k) as i32;
                r
            };
            dlg_button(hdc, a, s, slot(150.0), D_RESET_LOOK, "恢复本皮肤默认", false, true);
            dlg_button(
                hdc,
                a,
                s,
                slot(110.0),
                D_SKIN_ASAVE,
                if save_as { "存成新皮肤" } else { "另存为…" },
                save_as,
                true,
            );
            let only = theme::skins().len() <= 1;
            dlg_button(
                hdc,
                a,
                s,
                slot(130.0),
                D_SKIN_DEL,
                if arm_del { "确认删除皮肤" } else { "删除皮肤" },
                false,
                !only,
            );
            let by = rect.bottom - (40.0 * k) as i32;
            let cw = (74.0 * k) as i32;
            let gap = (8.0 * k) as i32;
            txt(
                hdc,
                &RECT { left: bx, top: by, right: rect.right - (200.0 * k) as i32, bottom: by + (26.0 * k) as i32 },
                if save_as {
                    "输入新皮肤的名字，Enter 或点「确定」存成一套新皮肤。"
                } else {
                    "改动已经实时叠到界面上；点确定会弹窗问要不要覆盖当前皮肤，取消就退回修改之前。"
                },
                s.skin.ink_dim,
                a.font_sm,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            dlg_button(
                hdc,
                a,
                s,
                RECT { left: rect.right - (16.0 * k) as i32 - cw, top: by, right: rect.right - (16.0 * k) as i32, bottom: by + (26.0 * k) as i32 },
                D_LOOK_OK,
                "确定",
                true,
                true,
            );
            dlg_button(
                hdc,
                a,
                s,
                RECT { left: rect.right - (16.0 * k) as i32 - cw * 2 - gap, top: by, right: rect.right - (16.0 * k) as i32 - cw - gap, bottom: by + (26.0 * k) as i32 },
                D_CLOSE,
                "取消",
                false,
                true,
            );
        }
    }
}

/// EDIT 子窗口只做一件事：Enter/Esc 交回主窗口，别让对话框里的输入框把键吃掉。
unsafe extern "system" fn edit_proc(
    h: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_KEYDOWN {
        let vk = wp.0 as u32;
        if vk == 0x0D || vk == 0x1B {
            if let Ok(p) = GetParent(h) {
                let _ = PostMessageW(Some(p), WM_KEYDOWN, wp, lp);
            }
            return LRESULT(0);
        }
    }
    DefSubclassProc(h, msg, wp, lp)
}

unsafe fn edit_text(ed: HWND) -> String {
    let n = GetWindowTextLengthW(ed);
    if n <= 0 {
        return String::new();
    }
    let mut b = vec![0u16; n as usize + 1];
    let got = GetWindowTextW(ed, &mut b);
    String::from_utf16_lossy(&b[..got.max(0) as usize])
}

unsafe fn set_edit_text(ed: HWND, s: &str) {
    let h = HSTRING::from(s);
    let _ = SetWindowTextW(ed, &h);
    let _ = SendMessageW(ed, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
}

unsafe fn place_edit(hwnd: HWND) {
    let _ = hwnd;
    let g = app().lock().unwrap();
    let (Some(d), Some(ed)) = (g.dlg.as_ref(), g.dlg_edit) else { return };
    let Some(r) = dlg_edit_rect(d.kind, d.rect, g.scale) else { return };
    let _ = SetWindowPos(
        ed,
        None,
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
}

unsafe fn create_dlg_edit(hwnd: HWND, kind: DlgKind) {
    let (r, text, font) = {
        let g = app().lock().unwrap();
        let Some(d) = g.dlg.as_ref() else { return };
        let Some(r) = dlg_edit_rect(kind, d.rect, g.scale) else { return };
        let text = match kind {
            DlgKind::Convert => g.conv_out.clone(),
            DlgKind::Search => g.search_kw.clone(),
            DlgKind::BatchRename => rn_default_tpl(&g),
            DlgKind::RenameCat => g.cat.as_ref().map(|c| c.name(g.org_sel)).unwrap_or_default(),
            DlgKind::Skin => format!("{} 副本", g.skin),
            _ => String::new(),
        };
        // 原生输入框挂在对话框里，字体就该跟对话框那一块一致，不能拿"上一次画的是哪块"。
        (r, text, g.zf[theme::Z_DLG].f)
    };
    let htxt = HSTRING::from(text);
    let ed = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        PCWSTR(pw("EDIT").as_ptr()),
        &htxt,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | WINDOW_STYLE(0x0080),
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        Some(hwnd),
        None,
        None,
        None,
    );
    let Ok(ed) = ed else { return };
    let _ = SendMessageW(ed, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
    let _ = SetWindowSubclass(ed, Some(edit_proc), 1, 0);
    let _ = SendMessageW(ed, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
    let _ = SetFocus(Some(ed));
    app().lock().unwrap().dlg_edit = Some(ed);
}

/// 原地改名开始：编辑框直接挂在名字那一格上。慢速双击名字和 F2 都走这一条，
/// 资源管理器改名从来没有对话框，这里也不给它对话框。
unsafe fn begin_rename(hwnd: HWND, idx: usize) {
    if app().lock().unwrap().ren_edit.is_some() {
        return;
    }
    let (z, _) = layout(hwnd);
    let (r, text, keep, center) = {
        let mut g = app().lock().unwrap();
        if !g.pickable(idx) {
            return;
        }
        let path = g.entries[idx].path.clone();
        let name = g.disp(idx);
        // 高亮只盖名字本体，扩展名原样留着，和资源管理器同一个手势。
        // 文件夹没有扩展名，整个名字选中。
        let stem_len = if g.entries[idx].is_dir {
            name.chars().count()
        } else {
            Path::new(&path)
                .file_stem()
                .map(|s| s.to_string_lossy().chars().count())
                .unwrap_or(name.chars().count())
        };
        let keep = stem_len.min(name.chars().count());
        // 格子档的名字本来就居中排，改名框跟着居中；两个列表档靠左，才靠左。
        let center = matches!(g.mode, Mode::Grid | Mode::Small);
        let r = g.name_rect(idx, z.grid);
        g.ren_idx = idx;
        g.ren_path = path;
        g.ren_tree = false;
        g.ren_box = (r.left, r.top, r.right, r.bottom);
        (r, name, keep, center)
    };
    let htxt = HSTRING::from(text);
    let font = app().lock().unwrap().font;
    // 0x0001 = 文字居中，0x0080 = 一行写满往两边滚；居中时不叠后者，不然居中会失效。
    let Ok(ed) = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        PCWSTR(pw("EDIT").as_ptr()),
        &htxt,
        WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(if center { 0x0001 } else { 0x0080 }),
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        Some(hwnd),
        None,
        None,
        None,
    ) else {
        return;
    };
    let _ = SendMessageW(ed, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
    let _ = SendMessageW(ed, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(keep as isize)));
    let _ = SetWindowSubclass(ed, Some(edit_proc), 1, 0);
    let _ = SetFocus(Some(ed));
    app().lock().unwrap().ren_edit = Some(ed);
    refresh(hwnd);
}

/// 树行原地改名：编辑框盖在那一行的名字上，靠左（树名本来就是左对齐）。
/// 文件夹名没有扩展名可言，文本整个选中。树名上慢半拍的两连击走这一条。
unsafe fn begin_rename_tree(hwnd: HWND, path: &str, r: RECT) {
    if app().lock().unwrap().ren_edit.is_some() {
        return;
    }
    let pt = path.trim_end_matches('\\').to_string();
    let Some(name) = Path::new(&pt).file_name().map(|s| s.to_string_lossy().to_string()) else {
        return;
    };
    let htxt = HSTRING::from(&name);
    let font = app().lock().unwrap().font;
    let Ok(ed) = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        PCWSTR(pw("EDIT").as_ptr()),
        &htxt,
        WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(0x0080),
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        Some(hwnd),
        None,
        None,
        None,
    ) else {
        return;
    };
    let _ = SendMessageW(ed, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
    let _ = SendMessageW(ed, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
    let _ = SetWindowSubclass(ed, Some(edit_proc), 1, 0);
    let _ = SetFocus(Some(ed));
    {
        let mut g = app().lock().unwrap();
        g.ren_edit = Some(ed);
        g.ren_path = pt;
        g.ren_tree = true;
        g.ren_idx = usize::MAX;
        g.ren_box = (r.left, r.top, r.right, r.bottom);
    }
    refresh(hwnd);
}

/// 收掉改名框。commit=false 是 Esc 作废，一个字都不改。
unsafe fn end_rename(hwnd: HWND, commit: bool) {
    let (ed, path, tree) = {
        let mut g = app().lock().unwrap();
        match g.ren_edit.take() {
            Some(e) => (
                e,
                std::mem::take(&mut g.ren_path),
                std::mem::take(&mut g.ren_tree),
            ),
            None => return,
        }
    };
    app().lock().unwrap().ren_idx = usize::MAX;
    let text = edit_text(ed);
    // 记录先摘干净再销毁：销毁会补一发 EN_KILLFOCUS，那时认不出来源就等于提交两遍。
    let _ = DestroyWindow(ed);
    let _ = SetFocus(Some(hwnd));
    if commit {
        if tree {
            apply_rename_tree(hwnd, &path, &text);
        } else {
            apply_rename(hwnd, &path, &text);
        }
    } else {
        refresh(hwnd);
    }
}

/// 改名落盘。名字没变或空着就是没改；改成了就把缓存、相似图指纹、类别里的旧路径一起换掉。
/// 回报这次有没有真的搬动，树上改名还要靠它决定跟不跟后续的树节点搬移。
unsafe fn apply_rename(hwnd: HWND, old: &str, raw: &str) -> bool {
    let newname = raw.trim().to_string();
    if newname.is_empty() || newname == stem(old) {
        refresh(hwnd);
        return false;
    }
    if newname.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
        app().lock().unwrap().msg = "文件名里有不能用的字符".to_string();
        refresh(hwnd);
        return false;
    }
    let dst = Path::new(old).with_file_name(&newname);
    if dst.exists() {
        app().lock().unwrap().msg = "同名文件已经存在".to_string();
        refresh(hwnd);
        return false;
    }
    let r = std::fs::rename(old, &dst);
    let ok = r.is_ok();
    let newp = dst.display().to_string();
    if ok {
        cache::move_path(old, &newp);
        sim::moved(old, &newp);
    }
    {
        let mut g = app().lock().unwrap();
        let mut m = match &r {
            Ok(()) => format!("已重命名为 {newname}"),
            Err(e) => format!("重命名失败：{e}"),
        };
        if ok {
            if let Some(e) = sync_cat_paths(&mut g, &[(old.to_string(), newp.clone())]) {
                m.push_str(&format!("；{e}"));
            }
        }
        g.msg = m;
    }
    app().lock().unwrap().refresh_src(hwnd);
    ok
}

/// 树上改名的落盘：校验和类别/缓存联动照用文件那一套，另外把树节点、
/// 选中行和当前目录一起搬到新路径上。
unsafe fn apply_rename_tree(hwnd: HWND, old: &str, raw: &str) {
    if !apply_rename(hwnd, old, raw) {
        return;
    }
    let newp = Path::new(old).with_file_name(raw.trim()).display().to_string();
    let mut g = app().lock().unwrap();
    rename_tree_node(&mut g.tree, old, &newp);
    if g.tsel.trim_end_matches('\\').eq_ignore_ascii_case(old) {
        g.tsel = format!("{newp}\\");
    }
    let cur = g.dir.display().to_string();
    let curt = cur.trim_end_matches('\\');
    // 当前目录是被改名的文件夹自己或它底下的：目录也得跟着挪，不然这扇窗停死在旧路径里。
    let inside = curt.eq_ignore_ascii_case(old)
        || (curt.len() > old.len()
            && curt.get(..old.len()).is_some_and(|p| p.eq_ignore_ascii_case(old))
            && curt.get(old.len()..).is_some_and(|r| r.starts_with('\\')));
    drop(g);
    if inside {
        let rest = curt.get(old.len()..).unwrap_or("").trim_start_matches('\\');
        let nd = if rest.is_empty() {
            newp.clone()
        } else {
            format!("{newp}\\{rest}")
        };
        app().lock().unwrap().goto(&nd, hwnd, true);
    }
    refresh(hwnd);
}

/// 树里找到旧路径那个节点换名换路。子项的路径是拿父路径拼出来的，跟着旧名字
/// 一起作废：清空让下次展开重新读盘，免得留着一条指到不存在目录的死路。
fn rename_tree_node(nodes: &mut [Node], old: &str, newp: &str) -> bool {
    for n in nodes.iter_mut() {
        if n.path.trim_end_matches('\\').eq_ignore_ascii_case(old) {
            n.path = format!("{newp}\\");
            if let Some(nm) = Path::new(newp).file_name() {
                n.name = nm.to_string_lossy().to_string();
            }
            n.kids = Vec::new();
            n.loaded = false;
            return true;
        }
        if rename_tree_node(&mut n.kids, old, newp) {
            return true;
        }
    }
    false
}

/// 树上某个路径那一行此刻的框。改名框跟着行走、右键改名也要它，几何只算一遍。
/// 行被折起来或滚出树了就返回 None，调用方按“这一行现在看不见”处理。
unsafe fn tree_row_rect(hwnd: HWND, want: &str) -> Option<RECT> {
    let (z, _) = layout(hwnd);
    let (rows, k, rh, head, tscroll) = {
        let g = app().lock().unwrap();
        let mut rows = Vec::new();
        flat_tree(&g.tree, 0, &mut rows);
        (rows, g.scale, g.th.row.max(1), z.tree.top + g.th.pane_head_h, g.tscroll)
    };
    let h = (z.tree.bottom - head).max(0);
    let maxs = (rows.len() as i32 * rh - h).max(0);
    let tscroll = tscroll.clamp(0, maxs);
    let sbw = tree_sb(z.tree, head, rows.len(), rh, tscroll, k)
        .map(|(_, _, w)| w)
        .unwrap_or(0);
    let isz = (14.0 * k).round() as i32;
    let tw = (11.0 * k).round() as i32;
    let indent = (20.0 * k).round() as i32;
    let want = want.trim_end_matches('\\');
    let (pos, (d, _, _, _)) = rows
        .iter()
        .enumerate()
        .find(|(_, (_, p, _, _))| p.trim_end_matches('\\').eq_ignore_ascii_case(want))?;
    let y = head + pos as i32 * rh - tscroll;
    Some(RECT {
        left: z.tree.left + (4.0 * k) as i32 + (*d as i32) * indent + tw + (6.0 * k) as i32 + isz,
        top: y,
        right: z.tree.right - 2 - sbw,
        bottom: y + rh,
    })
}

/// 格子会随滚动、缩放挪位置，挂着的编辑框就得跟着那一格走；树行随滚动、
/// 展折挪位置，同一个道理，按路径找回那一行把框挪过去。
unsafe fn place_rename(hwnd: HWND) {
    let (ed, idx, last, tree, path) = {
        let g = app().lock().unwrap();
        (g.ren_edit, g.ren_idx, g.ren_box, g.ren_tree, g.ren_path.clone())
    };
    let Some(ed) = ed else { return };
    if idx == usize::MAX && !tree {
        return;
    }
    let r = if tree {
        // 行被折起来滚出树了：框留在原地不动，反正一动就是盖在别的行上。
        let Some(r) = tree_row_rect(hwnd, &path) else { return };
        r
    } else {
        let (z, _) = layout(hwnd);
        app().lock().unwrap().name_rect(idx, z.grid)
    };
    let now = (r.left, r.top, r.right, r.bottom);
    if now == last {
        return;
    }
    app().lock().unwrap().ren_box = now;
    let _ = SetWindowPos(
        ed,
        None,
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
}

unsafe fn open_dlg(hwnd: HWND, kind: DlgKind) {
    let rc = client(hwnd);
    {
        let mut g = app().lock().unwrap();
        if g.dlg.is_some() {
            return;
        }
        g.open_menu = None;
        g.open_sub = None;
        let k = g.scale;
        let Some((w, h)) = dlg_size(kind, k) else { return };
        let rect = dlg_rect(&rc, w, h);
        let fmt = match kind {
            DlgKind::Convert | DlgKind::FmtOpt => g.conv_fmt,
            _ => 0,
        };
        // 输出文件夹默认跟着当前文件夹走：空着没填过，或者用户没自己定过路径，
        // 都按打开对话框时所在的文件夹算。定死的路径换文件夹后就成了暗坑。
        if kind == DlgKind::Convert && (g.conv_out.is_empty() || g.conv_out_auto) {
            g.conv_out = g.dir.display().to_string();
        }
        // 关联页打开时默认把所有支持的格式全部勾上：用户图的就是"一次全给 Xsee"。
        // 点过应用之后才回读注册表，没写进去的那项按实际结果显示。
        if kind == DlgKind::Assoc {
            g.assoc = assoc::supported_exts()
                .into_iter()
                .map(|e| (e.to_string(), true))
                .collect();
            g.assoc_msg.clear();
        }
        let opt = g.conv_opts.get(fmt).cloned().unwrap_or_default();
        let scope_db = if kind == DlgKind::FmtOpt { false } else { g.cat.is_some() };
        g.dlg = Some(Dlg {
            kind,
            rect,
            fmt,
            opt,
            scope_db,
            combo_open: false,
            ask_title: String::new(),
            ask_body: String::new(),
            ask_yes: false,
            ask_cmd: 0,
            look_draft: g.look.clone(),
            skin_before: g.look.clone(),
            look_keep: false,
            save_as: false,
            arm_del: false,
        });
    }
    if !matches!(kind, DlgKind::Skin | DlgKind::SimSearch) {
        create_dlg_edit(hwnd, kind);
    }
    refresh(hwnd);
}

/// 规则对话框：压在缩略图数据库页上（dlg_hold 那套，只叠一层），两个原生输入框
/// 预填选中那条扫描根的现行规则。开了这层期间维护页只画不认点击。
unsafe fn open_rule_dlg(hwnd: HWND) {
    {
        let mut g = app().lock().unwrap();
        if g.scan_sel >= g.scan_dirs.len() {
            drop(g);
            refresh(hwnd);
            return;
        }
        if g.dlg.is_some() {
            if g.dlg_hold.is_some() {
                return;
            }
            // 底下是维护页，没有原生输入框要摘；整层压进 hold 当背景。
            g.dlg_hold = g.dlg.take();
        }
        let k = g.scale;
        let Some((w, h)) = dlg_size(DlgKind::ScanRule, k) else { return };
        let rc = client(hwnd);
        let rect = dlg_rect(&rc, w, h);
        g.dlg = Some(Dlg {
            kind: DlgKind::ScanRule,
            rect,
            fmt: 0,
            opt: encode::Opt::default(),
            scope_db: false,
            combo_open: false,
            ask_title: String::new(),
            ask_body: String::new(),
            ask_yes: false,
            ask_cmd: 0,
            look_draft: g.look.clone(),
            skin_before: g.look.clone(),
            look_keep: false,
            save_as: false,
            arm_del: false,
        });
    }
    create_rule_edits(hwnd);
    refresh(hwnd);
}

/// 规则对话框的两个输入框，预填选中那条根的现行规则。摆位在 paint 里按帧对齐。
unsafe fn create_rule_edits(hwnd: HWND) {
    let (texts, font) = {
        let mut g = app().lock().unwrap();
        if g.rule_edits.iter().any(|h| !h.0.is_null()) {
            return;
        }
        let Some(r) = g.scan_dirs.get(g.scan_sel) else { return };
        (
            [r.dirs.join(","), r.exts.join(",")],
            g.zf[theme::Z_DLG].f,
        )
    };
    let mut made = [HWND::default(); 2];
    for (i, t) in texts.iter().enumerate() {
        let htxt = HSTRING::from(t.as_str());
        let ed = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(pw("EDIT").as_ptr()),
            &htxt,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | WINDOW_STYLE(0x0080),
            0,
            0,
            10,
            10,
            Some(hwnd),
            None,
            None,
            None,
        );
        let Ok(ed) = ed else { continue };
        let _ = SendMessageW(ed, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
        let _ = SetWindowSubclass(ed, Some(edit_proc), 1, 0);
        made[i] = ed;
    }
    app().lock().unwrap().rule_edits = made;
}

/// 规则对话框的两个输入框跟着画框走。
unsafe fn place_rule_edits(a: &App, rects: &[RECT; 2]) {
    for (ed, r) in a.rule_edits.iter().zip(rects) {
        if ed.0.is_null() {
            continue;
        }
        let _ = SetWindowPos(
            *ed,
            None,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

unsafe fn drop_rule_edits() {
    // 守卫先摘柄再销毁：框多半攥着焦点，DestroyWindow 同步触发 EN_KILLFOCUS
    // 回 WM_COMMAND，那里还要拿这把锁。
    let hs = std::mem::replace(
        &mut app().lock().unwrap().rule_edits,
        [HWND::default(); 2],
    );
    for ed in hs {
        if !ed.0.is_null() {
            let _ = DestroyWindow(ed);
        }
    }
}

/// 把关联页的勾选落到注册表。写坏的项汇总成一句话贴在页面上，不另弹框打断。
unsafe fn assoc_apply(hwnd: HWND) {
    let (items, chk, exe) = {
        let g = app().lock().unwrap();
        (g.assoc.clone(), g.assoc_check, std::env::current_exe().ok())
    };
    let Some(exe) = exe else {
        app().lock().unwrap().assoc_msg = "取不到本程序的位置，没法写关联".to_string();
        refresh(hwnd);
        return;
    };
    // 开关和清单一起在这一次落盘：勾了什么，下次启动就照什么补。
    assoc::set_check_on(chk);
    let n = items.iter().filter(|(_, on)| *on).count();
    let msg = match assoc::apply_all(&exe, &items) {
        Ok(()) if n == 0 => "已取消全部候选登记，本程序自己写的关联键都收掉了。".to_string(),
        Ok(()) => format!(
            "已把 {n} 种格式登记为候选。要从候选变成默认，还得在系统设置里选一次 Xsee。"
        ),
        Err(e) => format!("有项目没写进去：{e}"),
    };
    {
        let mut g = app().lock().unwrap();
        // 写完回读一遍：没写进去的那一项勾选状态按注册表的实际情况摆，界面不骗人。
        g.assoc = assoc::supported_exts()
            .into_iter()
            .map(|e| (e.to_string(), assoc::is_associated(e)))
            .collect();
        g.assoc_msg = msg;
    }
    // 告诉外壳打开方式列表变了。不通知的话资源管理器要等下一次刷新才认。
    unsafe {
        use windows::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
    refresh(hwnd);
}

/// 启动后跑一次的关联复查。清单为空就是从没在关联页点过「应用」，那时候一条注册表都不写。
unsafe fn assoc_recheck(hwnd: HWND) {
    app().lock().unwrap().assoc_probe = false;
    if !assoc::check_on() {
        return;
    }
    let Ok(exe) = std::env::current_exe() else { return };
    let rep = assoc::recheck(&exe);
    if rep.nothing() {
        return;
    }
    // 补完要通知外壳，不然资源管理器里的图标和「打开方式」得等下一次刷新才认。
    use windows::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
    SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    let line = format!(
        "启动复查文件关联：补回候选 {} 项，重建 ProgID {} 项，指向改回当前程序 {} 项。",
        rep.cands, rep.progids, rep.repaths
    );
    {
        let mut g = app().lock().unwrap();
        // 同一句话只组一次：状态栏和关联页底部那句是同一个结论的两个落点。
        g.msg = line.clone();
        g.assoc_msg = line;
        g.flash = Some(Instant::now());
    }
    refresh(hwnd);
}

/// 打开系统的「默认应用」设置页。Win10/11 只认这个入口，程序自己改不了默认。
unsafe fn assoc_open_defaults(hwnd: HWND) {
    use windows::Win32::UI::Shell::ShellExecuteW;
    let op = HSTRING::from("open");
    let uri = HSTRING::from("ms-settings:defaultapps");
    let r = ShellExecuteW(Some(hwnd), &op, &uri, None, None, SW_SHOWNORMAL);
    if r.0 as isize <= 32 {
        app().lock().unwrap().assoc_msg =
            "没能打开系统设置，请手动进「设置 → 应用 → 默认应用」。".to_string();
        refresh(hwnd);
    }
}

/// 从转换对话框点“格式设置”：先把目标文件夹编辑框里的字收回来，再换成设置窗。
unsafe fn open_fmtopt(hwnd: HWND) {
    let out = {
        let g = app().lock().unwrap();
        match g.dlg_edit {
            Some(e) => edit_text(e),
            None => g.conv_out.clone(),
        }
    };
    let rc = client(hwnd);
    let mut g = app().lock().unwrap();
    let k = g.scale;
    let fmt = g.conv_fmt;
    let opt = g.conv_opts.get(fmt).cloned().unwrap_or_default();
    let Some((w, h)) = dlg_size(DlgKind::FmtOpt, k) else { return };
    g.conv_out = out;
    g.dlg = Some(Dlg {
        kind: DlgKind::FmtOpt,
        rect: dlg_rect(&rc, w, h),
        fmt,
        opt,
        scope_db: false,
        combo_open: false,
        ask_title: String::new(),
        ask_body: String::new(),
        ask_yes: false,
        ask_cmd: 0,
        look_draft: Default::default(),
        skin_before: Default::default(),
        look_keep: false,
        save_as: false,
        arm_del: false,
    });
    // 先在锁内把句柄摘下来，放锁之后再销毁：这个框此刻多半还攥着焦点，
    // DestroyWindow 会同步触发 EN_KILLFOCUS 回到 WM_COMMAND，那里还要拿这把锁。
    let dlg_ed = g.dlg_edit.take();
    drop(g);
    if let Some(ed) = dlg_ed {
        let _ = DestroyWindow(ed);
    }
    refresh(hwnd);
}

/// 格式设置确定/取消之后回到转换对话框，格式和已填的输出目录都还在。
unsafe fn back_to_conv(hwnd: HWND) {
    let rc = client(hwnd);
    let mut g = app().lock().unwrap();
    let k = g.scale;
    let fmt = g.conv_fmt;
    let Some((w, h)) = dlg_size(DlgKind::Convert, k) else { return };
    g.dlg = Some(Dlg {
        kind: DlgKind::Convert,
        rect: dlg_rect(&rc, w, h),
        fmt,
        opt: encode::Opt::default(),
        scope_db: false,
        combo_open: false,
        ask_title: String::new(),
        ask_body: String::new(),
        ask_yes: false,
        ask_cmd: 0,
        look_draft: Default::default(),
        skin_before: Default::default(),
        look_keep: false,
        save_as: false,
        arm_del: false,
    });
    drop(g);
    create_dlg_edit(hwnd, DlgKind::Convert);
    refresh(hwnd);
}

/// 皮肤配色的确认框 / 提示框，取代系统 MessageBox 那种浅色窗。
/// `yes` 决定按钮是“是/否”还是单个“确定”；`cmd` 非 0 表示点确认后要重发这条命令。
/// 已经有窗开着（皮肤设置窗）时，那一层压到 dlg_hold 上当背景，答完再换回来——只叠一层。
unsafe fn open_ask(hwnd: HWND, title: &str, body: &str, yes: bool, cmd: usize) {
    // 名字输入框是原生 EDIT，跟着上一层窗走；压起来前先毁掉，不然它会浮在确认框上。
    // 句柄先摘下、放锁后销毁：毁掉带焦点的框会同步发 EN_KILLFOCUS，那里还要拿这把锁。
    let mut dlg_ed: Option<HWND> = None;
    {
        let mut g = app().lock().unwrap();
        if g.dlg.is_some() {
            if g.dlg_hold.is_some() {
                return;
            }
            g.dlg_hold = g.dlg.take();
            dlg_ed = g.dlg_edit.take();
        }
        g.open_menu = None;
        g.open_sub = None;
        let k = g.scale;
        // 确认框算对话框那一块：量正文高度和画正文用的是同一支笔。
        let font = g.zf[theme::Z_DLG].f;
        let rc = client(hwnd);
        let pad = (16.0 * k) as i32;
        let boxw = ((520.0 * k) as i32)
            .min(rc.right - rc.left - pad * 2 - (40.0 * k) as i32)
            .max((240.0 * k) as i32);
        // 先按目标宽度量出换行后的真实高度，正文才不会被裁；量的字体要和画正文的一致
        let hdc = GetDC(Some(hwnd));
        SelectObject(hdc, HGDIOBJ(font.0));
        let mut m = RECT { left: 0, top: 0, right: boxw, bottom: 4000 };
        let mut u: Vec<u16> = body.encode_utf16().collect();
        let _ = DrawTextW(
            hdc,
            &mut u,
            &mut m,
            DT_CALCRECT | DT_WORDBREAK | DT_EDITCONTROL | DT_LEFT | DT_NOPREFIX,
        );
        let _ = ReleaseDC(Some(hwnd), hdc);
        let w = (m.right + pad * 2).max((300.0 * k) as i32).min(boxw + pad * 2);
        let h = (44.0 * k) as i32 + m.bottom + (16.0 * k) as i32 + (42.0 * k) as i32 + (16.0 * k) as i32;
        let rect = dlg_rect(&rc, w, h);
        g.dlg = Some(Dlg {
            kind: DlgKind::Ask,
            rect,
            fmt: 0,
            opt: encode::Opt::default(),
            scope_db: false,
            combo_open: false,
            ask_title: title.to_string(),
            ask_body: body.to_string(),
            ask_yes: yes,
            ask_cmd: cmd,
            look_draft: Default::default(),
            skin_before: Default::default(),
            look_keep: false,
            save_as: false,
            arm_del: false,
        });
    }
    if let Some(ed) = dlg_ed {
        let _ = DestroyWindow(ed);
    }
    refresh(hwnd);
}

/// 头一回点「以图搜图」先讲一页：这一页怎么使、库和文件夹在哪维护、规则怎么写。
/// 弹出即算看过（Esc / ✕ 关掉也一样），下次直接进搜索页；点「确定」还会接着
/// 打开搜索页（ask_cmd = M_FINDSIM，走确认框那条回头路）。
unsafe fn sim_intro(hwnd: HWND) {
    {
        let mut g = app().lock().unwrap();
        if g.sim_intro {
            return;
        }
        g.sim_intro = true;
    }
    ui_save(&app().lock().unwrap(), hwnd);
    let body = "以图搜图不是搜整台电脑，它只在「已经入了库」的图里找。\n\
                看过的图会自动入库；没打开过的文件夹，要自己加进来扫一次。\n\
                \n\
                1. 先把文件夹加进来（做一次就够）\n\
                　 菜单「工具 → 缩略图数据库」→ 点「添加…」，选一个文件夹。\n\
                　 子文件夹会一起扫；选好点「开始扫描」，等进度跑完。\n\
                　 以后这个文件夹里看过的图都会自动入库，不用再扫。\n\
                \n\
                2. 开始搜\n\
                　 在图片上右键，选「以图搜图」。这一页默认拿你选中的那张当查询图。\n\
                　 点「开始查找」，下面按相似度从高到低列出结果。\n\
                \n\
                3. 调结果\n\
                　 嫌结果太杂，把「相似度」往高处拉；嫌漏了，往低处拉。\n\
                　 查询图是歪的，勾上「旋转」「镜像」再查一次。\n\
                \n\
                4. 维护和排除\n\
                　 缩略图数据库是搜图的地基。原图删了记录还在，点「移除无效索引」清理；\n\
                　 缩略图坏了，点「重建」重新生成。\n\
                　 不想要的文件夹，在名单里选中它点「规则…」：一行填文件夹名的关键字，\n\
                　 一行填不要的格式（如 psd,tif），用逗号隔开，扫的时候自动跳过。";
    open_ask(hwnd, "以图搜图 · 第一次使用", body, false, M_FINDSIM);
}

/// 找不回的那类删除先弹确认框（彻底删除、删类别）；正文按命令现取，点"是"后才重发原命令。
/// 进回收站的那条不问——回收站里能还原。
unsafe fn ask_delete(hwnd: HWND, id: usize) {    let (title, body) = if id == M_CAT_DEL {
        let (cid, name) = {
            let g = app().lock().unwrap();
            (
                g.org_sel,
                g.cat.as_ref().map(|c| c.name(g.org_sel)).unwrap_or_default(),
            )
        };
        if cid == 0 {
            app().lock().unwrap().msg = "先选一个类别".to_string();
            refresh(hwnd);
            return;
        }
        (
            "删除类别",
            format!("删除类别「{name}」？\n\n它的子类别和归类标记会一起删掉，图片文件本身不动。"),
        )
    } else {
        // 树落点按一行算，格子选区按选中个数算——两边同一个落点规则，
        // 不然树上 Shift+Del 报的是别处的数量。
        let (tree, tp) = tree_target();
        let n = if tree {
            match tp {
                Some(d) => {
                    let editable = {
                        let g = app().lock().unwrap();
                        g.tree_editable(&d)
                    };
                    if editable { 1 } else { 0 }
                }
                None => 0,
            }
        } else {
            app().lock().unwrap().selected_paths().len()
        };
        if n == 0 {
            let m = if tree { "这一项不能删除" } else { "没有选中文件" };
            app().lock().unwrap().msg = m.to_string();
            refresh(hwnd);
            return;
        }
        (
            "彻底删除",
            format!("把选中的 {n} 个文件彻底删掉？\n\n不进回收站，删掉就找不回了。"),
        )
    };
    open_ask(hwnd, title, &body, true, id);
}

/// 覆盖当前皮肤：把草稿里的颜色合并进它自己的 .skin 文件，然后收尾关窗。
/// 颜色既然进了文件，个人覆盖里那一层就当场清掉——同一组数不能留两个来源。
/// 字体还是留在这套皮肤的个人改动里（皮肤文件只装颜色）。
unsafe fn skin_commit(hwnd: HWND) {
    let (name, draft, base) = {
        let g = app().lock().unwrap();
        let Some(d) = g.dlg.as_ref() else { return };
        (g.skin.clone(), d.look_draft.clone(), theme::skin_of(&g.skin))
    };
    let Some((style, order, def)) = theme::meta_of(&name) else {
        app().lock().unwrap().msg = "找不到这套皮肤的文件".to_string();
        return;
    };
    let out = theme::preview(&base, &draft);
    // 内置皮肤盘上没有 .skin：改的颜色留在它的个人覆盖里就行，
    // 别为了覆盖一次就凭空长出一个 skin 文件夹。
    let on_disk = theme::skin_path(&name).is_some();
    if on_disk {
        if let Err(e) = theme::write_skin(&name, style, order, def, &out) {
            app().lock().unwrap().msg = format!("覆盖失败：{e}");
            return;
        }
        theme::rescan();
    }
    {
        let mut g = app().lock().unwrap();
        if on_disk {
            g.look.colors.clear();
        }
        g.look.save(&name);
        if let Some(d) = g.dlg.as_mut() {
            if on_disk {
                d.look_draft.colors.clear();
            }
            // 写盘已经办完，关窗时那一步退回不能再碰这份刚定下来的覆盖。
            d.look_keep = true;
        }
    }
    close_dlg(hwnd);
    app().lock().unwrap().msg = format!("已把这套颜色写进皮肤「{name}」");
}

/// 另存为一套新皮肤：新文件收下全部颜色，字体作为这套皮肤自己的个人覆盖带走。
unsafe fn skin_save_as(hwnd: HWND) {
    let (mut draft, raw) = {
        let g = app().lock().unwrap();
        let Some(d) = g.dlg.as_ref() else { return };
        let raw = match g.dlg_edit { Some(ed) => edit_text(ed), None => String::new() };
        (d.look_draft.clone(), raw)
    };
    let name = raw.trim().to_string();
    if name.is_empty() {
        app().lock().unwrap().msg = "还没填新皮肤的名字".to_string();
        return;
    }
    if theme::has_skin(&name) {
        app().lock().unwrap().msg = format!("已经有「{name}」这套皮肤，换个名字").to_string();
        return;
    }
    let (cur, base) = {
        let g = app().lock().unwrap();
        (g.skin.clone(), theme::skin_of(&g.skin))
    };
    let Some((style, order, _)) = theme::meta_of(&cur) else {
        app().lock().unwrap().msg = "读不到当前皮肤的风格和次序".to_string();
        return;
    };
    let out = theme::preview(&base, &draft);
    if let Err(e) = theme::write_skin(&name, style, order, false, &out) {
        app().lock().unwrap().msg = format!("另存失败：{e}");
        return;
    }
    theme::rescan();
    draft.colors.clear();
    let mut g = app().lock().unwrap();
    g.skin = name.clone();
    g.look = draft.clone();
    g.look.save(&name);
    ui_put_str("skin", &name);
    if let Some(d) = g.dlg.as_mut() {
        d.save_as = false;
        d.look_keep = true;
        d.look_draft = draft;
    }
    g.restyle(hwnd);
    drop(g);
    close_dlg(hwnd);
    app().lock().unwrap().msg = format!("已另存为皮肤「{name}」");
}

/// 换皮肤文件夹：记住选择、整表重扫；原来穿的那套不在新目录里就退回默认那套。
/// 设置窗开着也不关，目录页和主窗口当场换成新目录读到的样子。
unsafe fn skin_dir_switch(hwnd: HWND, dir: Option<PathBuf>) {
    if let Err(e) = theme::set_skin_dir(dir.as_deref()) {
        app().lock().unwrap().msg = e;
        return;
    }
    {
        let mut g = app().lock().unwrap();
        if !theme::has_skin(&g.skin) {
            g.skin = theme::default_name();
            g.look = theme::Look::load(&g.skin);
            ui_put_str("skin", &g.skin);
        }
        g.restyle(hwnd);
        g.msg = format!("皮肤目录已换，现在 {} 套", theme::skins().len());
    }
    refresh(hwnd);
}

/// 删掉当前这套皮肤，连它的 .skin 文件和个人改动一起。删的是身上这套就换到默认那套。
unsafe fn skin_delete(hwnd: HWND) {
    let name = app().lock().unwrap().skin.clone();
    if theme::skins().len() <= 1 {
        app().lock().unwrap().msg = "只剩这一套皮肤，删掉就没得穿了".to_string();
        return;
    }
    if let Err(e) = theme::delete_skin(&name) {
        app().lock().unwrap().msg = format!("删除失败：{e}");
        return;
    }
    let mut g = app().lock().unwrap();
    if g.skin == name {
        g.skin = theme::default_name();
        g.look = theme::Look::load(&g.skin);
        g.restyle(hwnd);
        ui_put_str("skin", &g.skin);
    }
    drop(g);
    close_dlg(hwnd);
    app().lock().unwrap().msg = format!("已删除皮肤「{name}」");
}

/// 收掉对话框上那个系统输入框。换页签时另存为的名字框不该继续浮在别的页上。
unsafe fn drop_dlg_edit() {
    // 守卫先放下再销毁：框多半正攥着焦点，DestroyWindow 会同步触发 EN_KILLFOCUS
    // 回到 WM_COMMAND，那里还要拿这把锁。
    let ed = app().lock().unwrap().dlg_edit.take();
    if let Some(ed) = ed {
        let _ = DestroyWindow(ed);
    }
}

unsafe fn close_dlg(hwnd: HWND) {
    drop_dlg_edit();
    drop_rule_edits();
    let revert = {
        let mut g = app().lock().unwrap();
        // 皮肤设置窗是边改边在界面上预览的，所以取消、Esc、"不覆盖"这些出口都得把
        // 界面收回打开那一刻；只有"覆盖当前皮肤"和"另存为"事先把 look_keep 置了真。
        let back = match g.dlg.as_ref() {
            Some(d) if d.kind == DlgKind::Skin && !d.look_keep => Some(d.skin_before.clone()),
            _ => None,
        };
        g.dlg = None;
        g.dlg_btns.clear();
        g.dlg_rows.clear();
        // 确认框关掉不算关窗：把压在底下那层设置窗换回来，界面维持预览的样子，人还在窗里。
        g.dlg = g.dlg_hold.take();
        // 窗都关了，拖动这件事跟着作废（不然那个短路会把刷新一直按下）。
        g.dlg_drag = None;
        DRAGGING.store(false, Ordering::Relaxed);
        back
    };
    if let Some(before) = revert {
        let mut g = app().lock().unwrap();
        g.look = before;
        g.restyle(hwnd);
    }
    // 压到确认框底下时那个原生名字输入框被毁了；换回来的那层还停在"另存为"就把框重新挂上。
    let redo = app().lock().unwrap().dlg.as_ref().map(|d| d.kind == DlgKind::Skin && d.save_as).unwrap_or(false);
    if redo {
        create_dlg_edit(hwnd, DlgKind::Skin);
    }
    let _ = SetFocus(Some(hwnd));
    refresh(hwnd);
}

fn walk_images(dir: &Path, key: &str, depth: usize, out: &mut Vec<String>) {
    if depth > 16 || out.len() >= 50_000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_images(&p, key, depth + 1, out);
        } else {
            let s = p.to_string_lossy().to_string();
            if scan::is_image(&s) && stem(&s).to_lowercase().contains(key) {
                out.push(s);
            }
        }
    }
}

unsafe fn start_search(hwnd: HWND, kw: String, scope_db: bool) {
    let hp = hwnd.0 as isize;
    let gid = SEARCH_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    // 全库那一路现在查自己的分类库：名单在锁里取出来，比对在锁外面做
    let all = app().lock().unwrap().cat.as_ref().map(|c| c.known_paths()).unwrap_or_default();
    let dir = app().lock().unwrap().dir.clone();
    app().lock().unwrap().msg = "正在查找…".to_string();
    refresh(hwnd);
    std::thread::spawn(move || {
        let mut files = Vec::new();
        if scope_db {
            let key = kw.trim().to_lowercase();
            files = if key.is_empty() {
                all
            } else {
                all
                    .into_iter()
                    .filter(|p| {
                        Path::new(p)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_lowercase().contains(&key))
                            .unwrap_or(false)
                    })
                    .collect()
            };
            files.sort_by_key(|p| p.to_ascii_lowercase());
        } else {
            let key = kw.trim().to_lowercase();
            walk_images(&dir, &key, 0, &mut files);
            files.sort_by_key(|p| p.to_ascii_lowercase());
        }
        let slot = SEARCH_RES.get_or_init(|| Mutex::new(None));
        *slot.lock().unwrap() = Some((gid, kw, files));
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_SEARCH_DONE, WPARAM(gid), LPARAM(0));
        }
    });
}

/// 这一页要用的那张缩略图：先拿这一档缓存里现成的，没有才解一次原图。
/// 建指纹那条线始终只碰缓存；只有查询图和预览允许开原图，而且一次就一张。
fn sim_rgba_at(path: &str, bw: u32, bh: u32) -> Option<Rgba> {
    if path.is_empty() {
        return None;
    }
    if let Some(px) = cache::peek(path, bw, bh) {
        return Some(px);
    }
    job::make(path, bw, bh).ok().map(|t| t.img)
}

/// 缓存里解出来的是 RGBA，Img 的约定是 GDI 直接能贴的 BGRA（1143 行）；
/// 不转的话相似页那两块预览红蓝颠倒，网格没事是因为它出图时转过了。
fn img_of(px: Rgba) -> Img {
    Img {
        w: px.w,
        h: px.h,
        px: rgba_to_bgra(&px.px),
        fit: None,
    }
}

/// 底下那两块预览：查询图 + 表里选中那一张。解码放后台，点一行不该卡一下界面。
unsafe fn sim_previews(hwnd: HWND) {
    let hp = hwnd.0 as isize;
    let gid = SIM_PV_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let (tier, q, hit) = {
        let g = app().lock().unwrap();
        // 预览跟搜索同一个目标档：图标/列表视图借库里最大的一档，不然预览
        // 对着小格档要么没图、要么现解一张塞进不该有账的档里。那一档得等库
        // 读进内存才知道，所以只在这里记下"当场能不能定"，定不了交给后台去问。
        let n = if g.sim_hits.is_empty() { usize::MAX } else { g.sim_sel.min(g.sim_hits.len() - 1) };
        (
            sim_target_tier(&g),
            g.sim_path.clone(),
            if n == usize::MAX { None } else { Some(g.sim_hits[n].path.clone()) },
        )
    };
    std::thread::spawn(move || {
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        job::init_com();
        let Some((bw, bh)) = sim::resolve_tier(tier) else { return };
        let src = sim_rgba_at(&q, bw, bh).map(img_of);
        let dst = hit.as_deref().and_then(|p| sim_rgba_at(p, bw, bh).map(img_of));
        {
            let mut g = app().lock().unwrap();
            if SIM_PV_GEN.load(Ordering::SeqCst) != gid {
                return;
            }
            g.sim_src_img = src;
            g.sim_hit_img = dst;
        }
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_SIM_PV, WPARAM(0), LPARAM(0));
        }
    });
}

/// 以图搜图用哪个档：缩略图视图用当前档；图标/列表/详细资料这些小格档自己
/// 不生成缩略图、没有指纹，借库里账最多的一档。相似库还没读进内存就是 None
/// （界面线程不该为了挑档把整库读进来），后台读完下一帧就有了。
fn sim_target_tier(g: &App) -> Option<(u32, u32)> {
    let (w, h) = g.tbox();
    if g.wants_thumb() && w.min(h) >= sim::MIN_SIDE {
        return Some((w, h));
    }
    sim::biggest_tier_loaded().map(|(bw, bh)| (bw as u32, bh as u32))
}

/// 把选中的扫描文件夹从名单里摘掉，「移除」按钮和 Del 键共用这一把刀。
/// 扫描在跑也允许：这一轮要扫哪些根在开扫时已经拍下快照，摘掉只影响
/// 收尾时给谁盖"上次扫描"的时间戳，名单落盘的就是摘完的样子。
unsafe fn remove_scan_sel(hwnd: HWND) {
    let mut g = app().lock().unwrap();
    if g.scan_sel >= g.scan_dirs.len() {
        return;
    }
    let sel = g.scan_sel;
    g.scan_dirs.remove(sel);
    g.scan_sel = sel.min(g.scan_dirs.len().saturating_sub(1));
    g.scan_scroll = g.scan_scroll.min(g.scan_dirs.len().saturating_sub(SCAN_VIS));
    save_scan_cfg(&g.scan_dirs);
    // 名单少了一条，规则关口跟着重排。
    sync_scan_rules(&g.scan_dirs);
    drop(g);
    refresh(hwnd);
}

/// 缩略图数据库页那颗「移除无效索引」：全部档位一次清完（从以图搜图页挪过来的，
/// 这里没有"这一档"的口径）。结果写自己的 db_prune 槽——和分类库的检查各走各的。
unsafe fn start_db_prune(hwnd: HWND) {
    let go = {
        let mut g = app().lock().unwrap();
        let on_page = g.dlg.as_ref().is_some_and(|d| d.kind == DlgKind::ThumbDb);
        if on_page && !g.db_busy && !g.scan_busy {
            g.db_prune = "正在移除无效索引…".to_string();
            g.db_busy = true;
            true
        } else {
            false
        }
    };
    if !go {
        return;
    }
    refresh(hwnd);
    let hp = hwnd.0 as isize;
    std::thread::spawn(move || {
        sim::ensure_loaded();
        let (gone, left) = sim::prune_all();
        let msg = format!(
            "移除无效索引完成：摘掉 {gone} 条原图已不在盘上的指纹（缩略图保留），全部档位还剩 {left} 条。"
        );
        {
            let mut g = app().lock().unwrap();
            g.db_prune = msg.clone();
            g.db_busy = false;
            g.msg = msg;
        }
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_PV, WPARAM(0), LPARAM(0));
        }
    });
}

/// 维护页的扫描文件夹：把名单里的文件夹（含子文件夹）里的图片直接生成缩略图入库，
/// 不用一张张翻。跳过的口径：这一档已经有和原图对得上的缩略图（cache::has_thumb，
/// 只查索引不解码）。先在后台把目录树走完拿总数，再四路分头照 job::make 那条线
/// 出图——没命中就解码入库、指纹跟着写，命中说明扫过，直接跳过。
/// 跑完把各根的扫描时间记回名单落盘，下次再扫同一份名单还是这个口径。
/// `only` 有值就只扫选中的那一个。
unsafe fn start_dir_scan(hwnd: HWND, only: Option<usize>) {
    let hp = hwnd.0 as isize;
    let (bw, bh, roots) = {
        let mut g = app().lock().unwrap();
        if g.scan_busy || g.db_busy || g.thumb_busy || g.fix_mode || g.sim_busy {
            g.msg = "还有别的库活在跑，等它结束再扫".to_string();
            drop(g);
            refresh(hwnd);
            return;
        }
        let (w, h) = g.tbox();
        // 图标、列表档不出缩略图；档太小就算扫进去了也建不了指纹，搜不到。
        if !g.wants_thumb() || w.min(h) < sim::MIN_SIDE {
            g.msg = if g.wants_thumb() {
                format!(
                    "这一档缩略图只有 {w}×{h}，扫进去也建不了指纹（短边要 ≥{}）。按 F8 换成缩略图视图再扫。",
                    sim::MIN_SIDE
                )
            } else {
                "这一档只画图标，不生成缩略图。按 F8 换成缩略图视图再扫。".to_string()
            };
            drop(g);
            refresh(hwnd);
            return;
        }
        let roots: Vec<ScanRoot> = g
            .scan_dirs
            .iter()
            .enumerate()
            .filter(|(i, _)| only.map_or(true, |o| *i == o))
            .map(|(_, r)| r.clone())
            .collect();
        if roots.is_empty() {
            g.msg = "名单是空的，先添加文件夹".to_string();
            drop(g);
            refresh(hwnd);
            return;
        }
        g.scan_busy = true;
        g.scan_total = 0;
        g.db_scan.clear();
        (w, h, roots)
    };
    let (done, added, skip, err, stop) = {
        let mut g = app().lock().unwrap();
        let (done, added, skip, err) = (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        let stop = Arc::new(AtomicBool::new(false));
        g.scan_done = done.clone();
        g.scan_added = added.clone();
        g.scan_skip = skip.clone();
        g.scan_err = err.clone();
        g.scan_stop = stop.clone();
        (done, added, skip, err, stop)
    };
    refresh(hwnd);
    std::thread::spawn(move || {
        job::init_com();
        // 先把目录树走完才知道总数；两个根互相套住时（a 和 a\b 都在名单里）去重。
        // 每个根带着自己的排除规则走：命中的文件夹整棵剪枝，贴图根本不枚举。
        let mut all: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut ruled = 0usize;
        for r in &roots {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let (files, skipped) =
                scan::walk_images(&r.path, &stop, &r.dirs, &r.exts);
            ruled += skipped;
            for p in files {
                if seen.insert(p.to_ascii_lowercase()) {
                    all.push(p);
                }
            }
        }
        all.sort_by_key(|p| p.to_ascii_lowercase());
        let total = all.len();
        app().lock().unwrap().scan_total = total;
        // 轮流发牌，四路各自的队列长度差不到一张，和全部重建同一个分法。
        let mut chunks: Vec<Vec<String>> = (0..4).map(|_| Vec::new()).collect();
        for (i, p) in all.into_iter().enumerate() {
            chunks[i % 4].push(p);
        }
        let live = Arc::new(AtomicUsize::new(chunks.len()));
        for chunk in chunks {
            let (live, done, added, skip, err, stop) = (
                live.clone(),
                done.clone(),
                added.clone(),
                skip.clone(),
                err.clone(),
                stop.clone(),
            );
            // 收尾要用名单对回时间戳，四个 worker 各带一份小克隆。
            let roots = roots.clone();
            std::thread::spawn(move || {
                let hwnd = HWND(hp as *mut core::ffi::c_void);
                job::init_com();
                for p in chunk {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    if cache::has_thumb(&p, bw, bh) {
                        skip.fetch_add(1, Ordering::Relaxed);
                        done.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    match job::make(&p, bw, bh) {
                        Ok(_) => added.fetch_add(1, Ordering::Relaxed),
                        Err(_) => err.fetch_add(1, Ordering::Relaxed),
                    };
                    done.fetch_add(1, Ordering::Relaxed);
                }
                if live.fetch_sub(1, Ordering::AcqRel) != 1 {
                    return;
                }
                // 最后一颗收尾：记账、报数、把各根的扫描时间写回名单。
                let (n_done, n_added) = (
                    done.load(Ordering::Relaxed),
                    added.load(Ordering::Relaxed),
                );
                let (n_skip, n_err) =
                    (skip.load(Ordering::Relaxed), err.load(Ordering::Relaxed));
                let stopped = stop.load(Ordering::Relaxed);
                // 命中排除规则的张数单独报，规则写没写对当场看得见。
                let rule_tail = if ruled > 0 {
                    format!("，按规则跳过 {ruled} 张")
                } else {
                    String::new()
                };
                let msg = if stopped {
                    format!(
                        "扫描已停止：走到 {n_done} / {total}，新增 {n_added}，跳过 {n_skip}，失败 {n_err}{rule_tail}。"
                    )
                } else {
                    format!(
                        "扫描完成：新增 {n_added} 张，跳过 {n_skip} 张（缩略图已有了），失败 {n_err} 张{rule_tail}。这一档 {bw}×{bh}。"
                    )
                };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                {
                    let mut g = app().lock().unwrap();
                    // 只给这一轮真扫过的根盖时间。扫描当中新添的文件夹没参与这轮，
                    // 时间留在 0，界面上照旧显示"还没扫过"。
                    for r in g.scan_dirs.iter_mut() {
                        if roots.iter().any(|x| x.path.eq_ignore_ascii_case(&r.path)) {
                            r.last = now;
                        }
                    }
                    save_scan_cfg(&g.scan_dirs);
                    g.db_scan = msg.clone();
                    g.scan_busy = false;
                    g.scan_total = 0;
                    g.msg = msg.lines().next().unwrap_or("扫描完成").to_string();
                }
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_PV, WPARAM(0), LPARAM(0));
                }
            });
        }
    });
}

/// 以图搜图那一页不模态：这一下落在浮层外面，就算底下那屏的。判据只写这一处，
/// 点击、右键、悬停、打字跳转都问它，免得四处口径不一、某一样忽然又吃住了鼠标。
fn sim_loose(a: &App, x: i32, y: i32) -> bool {
    match a.dlg.as_ref() {
        Some(d) => d.kind == DlgKind::SimSearch && !over(&d.rect, x, y),
        None => false,
    }
}

/// 带回这张图自己那个文件夹，回去还得把它选上、滚到看得见的那一行。
/// "转到文件"和以图搜图结果表的双击都走这一条，口径只有一份。
unsafe fn reveal_in_view(hwnd: HWND, p: &str) {
    let Some(dir) = Path::new(p).parent().map(|d| d.to_string_lossy().to_string()) else { return };
    if dir.is_empty() {
        return;
    }
    if app().lock().unwrap().dir.to_string_lossy() != dir {
        app().lock().unwrap().goto(&dir, hwnd, true);
    }
    let (z, _) = layout(hwnd);
    {
        let mut g = app().lock().unwrap();
        match g.entries.iter().position(|e| e.path == p) {
            Some(i) => {
                g.set_sel(i);
                g.keep_visible(z.grid);
            }
            // 过滤勾着的时候这张可能没被列出来。这里不动他的过滤开关，
            // 只说清楚为什么看不见，让他自己知道该去哪儿放开。
            None => g.msg = "这张被当前的过滤挡住了，先在过滤里放开再看".to_string(),
        }
    }
    request_pv(hwnd);
    refresh(hwnd);
}

/// 找相似，一整页那种走法。第一步是补库：目标档还没记过指纹的（或者原图改过的），
/// 从缩略图缓存里现成那张取图补上——全程不开原图，几千张也就几秒。
/// 第二步跨档搜：查询图在哪几档有过缩略图，就各算一份指纹、各搜各的账——
/// 指纹只有同档才可比，所以查询图的指纹也得按那一档算。一档都没见过就兜底：
/// 目标档解码原图入一档再搜，不然新图永远搜不到自己。
/// 图标/列表/详细资料这些小格档自己没有指纹，目标档自动借库里账最多的一档。
/// 跑在后台线程，界面照旧能动，进度由 60 毫秒心跳现读。
unsafe fn start_sim_search(hwnd: HWND) {
    let hp = hwnd.0 as isize;
    let prog = Arc::new(AtomicUsize::new(0));
    let (gid, tier, q, pct, algo, rot, flip, auto) = {
        let mut g = app().lock().unwrap();
        if g.sim_busy {
            g.sim_lib_msg = "上一次还没跑完，等它一下".to_string();
            drop(g);
            refresh(hwnd);
            return;
        }
        // 小图标/列表/详细资料这些档自己不建缩略图，目标档得借库里账最多的一份，
        // 那一份只有把库读进内存才知道。界面线程问到空不能就此退出——退出就再没人
        // 去读库了，那一页会永远说"指纹库还是空的"，连搜都搜不起来。交给后台线程问。
        let tier = sim_target_tier(&g);
        if g.sim_path.is_empty() || !Path::new(&g.sim_path).is_file() {
            g.sim_lib_msg = "先挑一张要用来找的图片".to_string();
            drop(g);
            refresh(hwnd);
            return;
        }
        // 代号只在真要起这一趟时才换：走到这儿又回去的那几条不换代，否则正在跑
        // 那趟交回来的收尾消息会被当成过期丢掉，"查找中"就再也摘不掉了。
        let gid = SIM_GEN.fetch_add(1, Ordering::SeqCst) + 1;
        g.sim_busy = true;
        g.sim_done = prog.clone();
        g.sim_total = 0;
        g.sim_stop.store(false, Ordering::SeqCst);
        (
            gid,
            tier,
            g.sim_path.clone(),
            g.sim_pct,
            g.sim_algo,
            g.sim_rot,
            g.sim_flip,
            g.sim_auto,
        )
    };
    let stop = app().lock().unwrap().sim_stop.clone();
    refresh(hwnd);
    std::thread::spawn(move || {
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        job::init_com();
        let t0 = Instant::now();
        // 收尾只留这一条路：落结果 + 让界面把"查找中"摘掉，别让人卡在灰按钮上。
        let finish = |hits: Vec<SimHit>, msg: Option<String>| {
            let ms = t0.elapsed().as_millis();
            let slot = SIM_RES.get_or_init(|| Mutex::new(None));
            *slot.lock().unwrap() = Some((gid, ms, hits, msg));
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_SIM_DONE, WPARAM(gid), LPARAM(0));
            }
        };
        let Some((bw, bh)) = sim::resolve_tier(tier) else {
            finish(
                Vec::new(),
                Some("指纹库还是空的：先在缩略图视图里翻几张图，或去缩略图数据库页扫描一批。".to_string()),
            );
            return;
        };
        if auto {
            // 同"补齐指纹"：待补名单直接来自缩略图库，不再先翻一遍路径名单。
            sim::ensure_loaded();
            let todo = cache::missing_fps(bw, bh, None);
            app().lock().unwrap().sim_total = todo.len();
            sim::fill(todo, bw, bh, &prog, &stop);
        }
        // 跨档搜：查询图在哪几档有过缩略图就搜哪几档；一档都没有（新图）就
        // 兜底在目标档解码原图入一档再搜——不然它永远搜不到自己。
        let mut cand = sim::search_all_tiers(
            &q,
            rot,
            flip,
            algo,
            sim::dist_from_pct(pct, algo),
            sim::MAX_HITS + 1024,
        );
        if cand.is_empty() {
            if let Some(img) = sim_rgba_at(&q, bw, bh) {
                let qs = sim::variants(&img, rot, flip);
                cand = sim::search(
                    bw,
                    bh,
                    &qs,
                    algo,
                    sim::dist_from_pct(pct, algo),
                    sim::MAX_HITS + 1024,
                );
            }
        }
        let hits = (!cand.is_empty()).then(|| {
            // 盘上已经不在的条目不进结果：0 字节 + 01-01 的日期，排最前还双击
            // 不动，纯搅局。逐条问盘四路并行（慢盘一次几毫秒，串行就是半秒起），
            // 各路带着原下标回来按原序拼回，榜还是按差异排的那个榜。
            // 库里的死账交给维护页「移除无效索引」清，结果表只认现存的文件。
            let mut v: Vec<(usize, SimHit)> = std::thread::scope(|s| {
                const W: usize = 4;
                (0..W)
                    .map(|w| {
                        let cand = &cand;
                        s.spawn(move || {
                            cand.iter()
                                .skip(w)
                                .step_by(W)
                                .enumerate()
                                .filter_map(|(i, (d, path))| {
                                    let m = std::fs::metadata(path).ok()?;
                                    let secs = m
                                        .modified()
                                        .ok()
                                        .and_then(|t| {
                                            t.duration_since(std::time::UNIX_EPOCH).ok()
                                        })
                                        .map(|d| d.as_secs())
                                        .unwrap_or(0);
                                    Some((
                                        i * W + w,
                                        SimHit {
                                            path: path.clone(),
                                            dist: *d,
                                            size: m.len(),
                                            mtime: secs.min(u32::MAX as u64) as u32,
                                        },
                                    ))
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .flat_map(|h| h.join().unwrap_or_default())
                    .collect()
            });
            v.sort_unstable_by_key(|(i, _)| *i);
            let mut v: Vec<SimHit> = v.into_iter().map(|(_, h)| h).collect();
            v.truncate(sim::MAX_HITS);
            v
        });
        finish(hits.unwrap_or_default(), None);
    });
}

/// 备份文件夹名里的时间戳：2026-10-21_014530。
unsafe fn stamp_now() -> String {
    let st = GetLocalTime();
    format!(
        "{:04}-{:02}-{:02}_{:02}{:02}{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
    )
}

const DB_TAR: &str = r"C:\Windows\System32\tar.exe";

/// 备份和导出打包的就是分类库那一份文件，连带上一版 `.bak` 一起，回退还有余地。
/// `.tmp` 也带上：写入是"临时文件→同步→原子替换"，崩在替换前时载入认的就是它，
/// 备份里少了这一份，恢复回来就少最后那一笔。
fn catalog_parts() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for name in ["catalog.tsv", "catalog.bak", "catalog.tmp"] {
        let p = data_dir().join(name);
        if p.is_file() {
            v.push(p);
        }
    }
    v
}

/// 打成 zip。用系统自带的 tar，别在进程里再引一套压缩库。
fn db_export(out: &Path) -> Result<usize, String> {
    let files = catalog_parts();
    if files.is_empty() {
        return Err("还没有分类库文件：先建一个类别，或者从别的库导入一份。".into());
    }
    let dir = catalog_file().parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let mut cmd = std::process::Command::new(DB_TAR);
    cmd.arg("--format").arg("zip").arg("-cf").arg(out).arg("-C").arg(&dir);
    for p in &files {
        if let Some(n) = p.file_name() {
            cmd.arg(n);
        }
    }
    let r = cmd.output().map_err(|e| format!("调用系统压缩工具失败：{e}"))?;
    if !r.status.success() {
        return Err(format!("打包失败：{}", String::from_utf8_lossy(&r.stderr).trim()));
    }
    if !out.is_file() {
        return Err("打包没有生成文件。".into());
    }
    Ok(files.len())
}

fn db_backup(dir: &Path) -> Result<String, String> {
    let stamp = unsafe { stamp_now() };
    let mut dst = dir.join(format!("XseeBK_{stamp}.zip"));
    let mut i = 1;
    while dst.exists() {
        dst = dir.join(format!("XseeBK_{stamp}_{i}.zip"));
        i += 1;
    }
    db_export(&dst)?;
    Ok(dst.display().to_string())
}

/// 能当库用的目录：认我们自己的那份自含分类库。
fn lib_dir_ok(p: &Path) -> bool {
    p.join("catalog.tsv").is_file()
}

fn find_db_root(dir: &Path) -> Option<PathBuf> {
    if lib_dir_ok(dir) {
        return Some(dir.to_path_buf());
    }
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() && lib_dir_ok(&p) {
            return Some(p);
        }
    }
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        for e2 in std::fs::read_dir(&p).ok()?.flatten() {
            let p2 = e2.path();
            if p2.is_dir() && lib_dir_ok(&p2) {
                return Some(p2);
            }
        }
    }
    None
}

/// 导入：先把压缩包解到 data 旁的临时目录，校验通过后再走覆盖导入。
fn db_import_pkg(pkg: &Path) -> Result<String, String> {
    let parent = catalog_file().parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let tmp = parent.join(format!(".import_pkg_{}", unsafe { stamp_now() }));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| format!("建临时解压目录失败：{e}"))?;
    let r = std::process::Command::new(DB_TAR)
        .arg("-xf")
        .arg(pkg)
        .arg("-C")
        .arg(&tmp)
        .output()
        .map_err(|e| format!("调用系统解压工具失败：{e}"))?;
    if !r.status.success() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(format!("解压失败：{}", String::from_utf8_lossy(&r.stderr).trim()));
    }
    let res = match find_db_root(&tmp) {
        Some(root) => db_import(&root),
        None => {
            let _ = std::fs::remove_dir_all(&tmp);
            Err("压缩包里找不到 catalog.tsv，不像是这个软件的数据库备份。".into())
        }
    };
    let _ = std::fs::remove_dir_all(&tmp);
    res
}

/// 覆盖式导入：校验通过才动，中途失败原库还在原处。换之前先把现库压成一份备份。
fn db_import(src: &Path) -> Result<String, String> {
    let cat = catalog_file();
    let dir = cat.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if !src.join("catalog.tsv").is_file() {
        return Err("选中的文件夹里没有 catalog.tsv，那不是这个软件认的数据库。".into());
    }
    if std::fs::canonicalize(src).ok() == std::fs::canonicalize(&dir).ok() {
        return Err("不能从当前数据库本身导入。".into());
    }
    // 先读干净再落盘：正文行数和头里记的数对不上的那份不收
    let text = std::fs::read_to_string(src.join("catalog.tsv"))
        .map_err(|e| format!("读 catalog.tsv 失败：{e}"))?;
    if !catdb::catalog_ok(&text) {
        return Err("这份 catalog.tsv 不自洽：正文行数和头里记的数对不上。".into());
    }
    let from = "分类库文件";
    let back = if cat.is_file() { db_backup(&dir)? } else { String::new() };
    catdb::write_catalog(&cat, &text).map_err(|e| format!("写入分类库失败：{e}"))?;
    let n = catdb::CatEdit::load_full(&cat).0;
    let msg = format!(
        "已从{from}导入：{} 个类别、{} 条归类、{} 张图记了入库时间。",
        n.n_cats(),
        n.n_rows(),
        n.n_assets()
    );
    Ok(if back.is_empty() { msg } else { format!("{msg}\n\n导入前的库备份在：\n{back}") })
}

/// 数据库检查/备份/导入都在后台跑。kind: 0 检查 1 备份 2 导入。
unsafe fn start_db_task(hwnd: HWND, kind: u8, src: Option<PathBuf>) {
    {
        let mut g = app().lock().unwrap();
        if g.db_busy {
            g.msg = "数据库任务还在跑，等它完成".to_string();
            drop(g);
            refresh(hwnd);
            return;
        }
        g.db_busy = true;
        g.msg = match kind {
            0 => "正在检查数据库（扫描全部资产路径）…",
            1 => "正在导出数据库…",
            _ => "正在导入数据库…",
        }
        .to_string();
    }
    refresh(hwnd);
    let hp = hwnd.0 as isize;
    std::thread::spawn(move || {
        let (msg, reload) = match kind {
            0 => {
                // 名单先在锁里取出来，逐张问硬盘在不在是四万次调用，攥着锁问会把界面冻住
                let all = app().lock().unwrap().cat.as_ref().map(|c| c.known_paths()).unwrap_or_default();
                let gone: Vec<String> = all.iter().filter(|p| !Path::new(p).exists()).cloned().collect();
                if all.is_empty() {
                    (String::from("分类库还没读进来，先等它读完。"), false)
                } else if gone.is_empty() {
                    (format!("数据库检查完成：查了 {} 张，文件都在硬盘上。", all.len()), false)
                } else {
                    let r = app().lock().unwrap().cat.as_mut().map(|cat| {
                        let n = cat.forget_paths(&gone);
                        cat.save_full(&catalog_file()).map(|()| n)
                    });
                    match r {
                        Some(Ok(n)) => (
                            format!(
                                "数据库检查完成：查了 {} 张，摘掉 {n} 张硬盘上已不存在的。\n\n上一版留在 catalog.bak。",
                                all.len()
                            ),
                            false,
                        ),
                        Some(Err(e)) => (format!("查出 {} 张已不存在，但写盘失败：{e}", gone.len()), false),
                        None => (String::from("分类库还没读进来，先等它读完。"), false),
                    }
                }
            }
            1 => match src {
                Some(p) => match db_export(&p) {
                    Ok(n) => (format!("分类库已导出（{n} 个文件）：\n{}", p.display()), false),
                    Err(e) => (format!("导出失败：{e}"), false),
                },
                None => ("没有选择导出文件。".to_string(), false),
            },
            _ => match src {
                Some(p) => match db_import_pkg(&p) {
                    Ok(b) => (format!("数据库已覆盖导入。\n\n{b}"), true),
                    Err(e) => (format!("导入失败：{e}"), false),
                },
                None => ("没有选择要导入的压缩包。".to_string(), false),
            },
        };
        *DB_RESULT.lock().unwrap() = (kind, msg, reload);
        app().lock().unwrap().db_busy = false;
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_DB_DONE, WPARAM(0), LPARAM(0));
        }
    });
}

/// 全部重建缩略图：名单来自缩略图自己记的那份（每一张存过的图都会留一行路径），
/// 不靠分类库——没分过类的图照样有缩略图。四个后台线程分头重解，界面照旧能动；
/// 进度由 60 毫秒心跳现读这两个计数，不额外发消息催。
unsafe fn start_thumb_all(hwnd: HWND) {
    let (paths, bw, bh) = {
        let g = app().lock().unwrap();
        if g.thumb_busy || g.fix_mode || g.db_busy {
            drop(g);
            app().lock().unwrap().msg = "还有别的活没干完，等它结束再重做全部".to_string();
            refresh(hwnd);
            return;
        }
        let (w, h) = g.tbox();
        (cache::known_paths(), w, h)
    };
    if paths.is_empty() {
        let mut g = app().lock().unwrap();
        g.db_thumb =
            "缩略图名单是空的。这份名单从本版才开始记录，之前存的那批索引里只有路径的哈希，认不出是谁。"
                .to_string();
        g.msg = g.db_thumb.clone();
        drop(g);
        refresh(hwnd);
        return;
    }
    let total = paths.len();
    let done = Arc::new(AtomicUsize::new(0));
    let bad = Arc::new(AtomicUsize::new(0));
    let gone = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    {
        let mut g = app().lock().unwrap();
        g.thumb_busy = true;
        g.thumb_total = total;
        g.thumb_done = done.clone();
        g.thumb_bad = bad.clone();
        g.thumb_stop = stop.clone();
        g.db_thumb.clear();
        g.msg = format!("正在重做全部缩略图：0 / {total}");
    }
    refresh(hwnd);
    // 轮流发牌，四路各自的队列长度差不到一张。
    let mut chunks: Vec<Vec<String>> = (0..4).map(|_| Vec::new()).collect();
    for (i, p) in paths.into_iter().enumerate() {
        chunks[i % 4].push(p);
    }
    let live = Arc::new(AtomicUsize::new(chunks.len()));
    let hp = hwnd.0 as isize;
    for chunk in chunks {
        let (live, done, bad, gone, stop) = (live.clone(), done.clone(), bad.clone(), gone.clone(), stop.clone());
        std::thread::spawn(move || {
            let hwnd = HWND(hp as *mut core::ffi::c_void);
            job::init_com();
            for p in chunk {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                match job::remake(&p, bw, bh) {
                    // 第二项是"缓存里原来那张对不对"。解不开的（文件没了、格式不认）
                    // 不算错位——那是另一回事，别混进对不上的账里。
                    Ok((_, same)) => {
                        done.fetch_add(1, Ordering::Relaxed);
                        if !same {
                            bad.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        done.fetch_add(1, Ordering::Relaxed);
                        if !Path::new(&p).exists() {
                            gone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
            if live.fetch_sub(1, Ordering::AcqRel) != 1 {
                return;
            }
            let n = done.load(Ordering::Relaxed);
            let m = bad.load(Ordering::Relaxed);
            let miss = gone.load(Ordering::Relaxed);
            let stopped = stop.load(Ordering::Relaxed);
            let mut g = app().lock().unwrap();
            g.db_thumb = if stopped {
                format!("重建已停止：这一轮重做了 {n} / {total} 张，其中 {m} 张对不上，已全部换成重解的。")
            } else if miss == 0 {
                format!("全部重建：重做了 {n} 张，其中 {m} 张和缓存里原来那张对不上，已全部换成重解的。")
            } else {
                format!(
                    "全部重建：重做了 {n} 张，其中 {m} 张和缓存里原来那张对不上，已全部换成重解的；\
                     名单上还有 {miss} 张在硬盘上已经找不到了。"
                )
            };
            g.thumb_busy = false;
            g.thumb_total = 0;
            g.msg = g.db_thumb.clone();
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_PV, WPARAM(0), LPARAM(0));
            }
        });
    }
}

/// 读盘读出了哪一档，就说哪一档的话。
fn overlay_note(st: catdb::OverlayLoad) -> String {
    match st {
        catdb::OverlayLoad::Backup => "分类数据库主文件损坏，已从备份恢复".to_string(),
        catdb::OverlayLoad::Temp => "分类数据库写完没来得及替换，已按临时文件接上".to_string(),
        catdb::OverlayLoad::Corrupt => {
            "分类数据库文件读不出来，原文件已改名留档，当前分类为空".to_string()
        }
        _ => String::new(),
    }
}

/// 后台读分类库，读完发 WM_DB 让右侧整理面板重画。
/// data 里已经是自含全量库（catalog.tsv）就只读它；还停在老格式（一份只记改动的
/// categories.tsv）就一次性并成一份全量写盘，老叠加层改名留档。
unsafe fn spawn_db_load(hwnd: HWND) {
    let hp = hwnd.0 as isize;
    std::thread::spawn(move || {
        let cat_path = catalog_file();
        let have_catalog = cat_path.exists()
            || cat_path.with_extension("bak").exists()
            || cat_path.with_extension("tmp").exists();
        let legacy = legacy_overlay_file();
        let (ce, note) = if have_catalog {
            let (ce, st) = catdb::CatEdit::load_full(&cat_path);
            (ce, overlay_note(st))
        } else if legacy.exists() {
            // 主库还没有、老叠加层在：它自己记的那些类别和归类就是全部内容，照原样并成一份
            let mut ce = catdb::CatEdit::empty();
            let mut n = overlay_note(ce.load_overlay(&legacy));
            if ce.save_full(&cat_path).is_err() && n.is_empty() {
                n = "分类库写盘失败".to_string();
            }
            retire_overlay(&legacy);
            (ce, n)
        } else {
            // 新机器上什么都没有：从空库开始，建第一个类别时才落盘
            (catdb::CatEdit::empty(), String::new())
        };
        {
            let mut g = app().lock().unwrap();
            g.cat = Some(ce);
            g.db_note = note;
        }
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        let _ = PostMessageW(Some(hwnd), WM_DB, WPARAM(0), LPARAM(0));
    });
}

/// 老叠加层不再被读：连着它的备份和临时文件一起改名留档，一个都不删。
fn retire_overlay(legacy: &Path) {
    for ext in ["", "bak", "tmp"] {
        let p = if ext.is_empty() { legacy.to_path_buf() } else { legacy.with_extension(ext) };
        if !p.exists() {
            continue;
        }
        let mut to = p.as_os_str().to_os_string();
        to.push(".imported");
        let _ = std::fs::rename(&p, std::path::PathBuf::from(to));
    }
}

/// 打开独立编辑器进程。保存后编辑器会发 WM_COPYDATA 回来，主窗口重读那一张缩略图。
unsafe fn launch_editor(hwnd: HWND, path: &str) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let ed_exe = dir.join("xedit.exe");
    if !ed_exe.is_file() {
        app().lock().unwrap().msg = "xedit.exe 不在程序旁边".to_string();
        refresh(hwnd);
        return;
    }
    let (skin, density) = {
        let g = app().lock().unwrap();
        (g.skin.clone(), g.density.clone())
    };
    let mut cmd = std::process::Command::new(&ed_exe);
    cmd.arg(path)
        .arg("--skin")
        .arg(skin)
        .arg("--density")
        .arg(density)
        .arg("--notify")
        .arg(format!("{}", hwnd.0 as isize));
    match cmd.spawn() {
        Ok(_) => app().lock().unwrap().msg = format!("已在编辑器中打开 {}", stem(path)),
        Err(e) => app().lock().unwrap().msg = format!("启动编辑器失败：{e}"),
    }
    refresh(hwnd);
}

/// 一批转换的结果：(成功, 跳过, 失败, 是不是中途停的)。开始下一批时重写。
static CONV_LAST: Mutex<(usize, usize, usize, bool)> = Mutex::new((0, 0, 0, false));
/// 失败样例，最多留五条，完成时一并说给用户。
static CONV_ERRS: Mutex<Vec<String>> = Mutex::new(Vec::new());

unsafe fn start_conv(
    hwnd: HWND,
    paths: Vec<String>,
    fmt: usize,
    opt: encode::Opt,
    out: String,
    policy: encode::Collision,
    del_src: bool,
) {
    if CONV_RUN.swap(true, Ordering::SeqCst) {
        let mut g = app().lock().unwrap();
        g.msg = "上一批还在转换，等它跑完".to_string();
        g.conv_msg = "上一批还在转换".to_string();
        refresh(hwnd);
        return;
    }
    CONV_STOP.store(false, Ordering::SeqCst);
    encode::begin_batch();
    if let Ok(mut v) = CONV_ERRS.lock() {
        v.clear();
    }
    app().lock().unwrap().conv_msg.clear();
    let hp = hwnd.0 as isize;
    let total = paths.len();
    let prog_done = {
        let g = app().lock().unwrap();
        g.conv_total.store(total, Ordering::SeqCst);
        g.conv_done.store(0, Ordering::SeqCst);
        g.conv_done.clone()
    };
    app().lock().unwrap().msg = format!("开始转换 {total} 个文件…（Esc 停止）");
    refresh(hwnd);
    let paths = Arc::new(paths);
    let out = Arc::new(out);
    let opt = Arc::new(opt);
    let batch = Arc::new(encode::batch_set(&paths));
    let next = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicUsize::new(0));
    let ok = Arc::new(AtomicUsize::new(0));
    let skip = Arc::new(AtomicUsize::new(0));
    let err = Arc::new(AtomicUsize::new(0));
    let workers = 4.min(total.max(1));
    let mut hs = Vec::new();
    for _ in 0..workers {
        let paths = paths.clone();
        let out = out.clone();
        let opt = opt.clone();
        let batch = batch.clone();
        let next = next.clone();
        let done = done.clone();
        let ok = ok.clone();
        let skip = skip.clone();
        let err = err.clone();
        let prog_done = prog_done.clone();
        hs.push(std::thread::spawn(move || loop {
            if CONV_STOP.load(Ordering::SeqCst) {
                break;
            }
            let i = next.fetch_add(1, Ordering::SeqCst);
            if i >= paths.len() {
                break;
            }
            let p = &paths[i];
            match encode::convert_file(p, &out, fmt, &opt, policy, &batch) {
                Ok(encode::Outcome::Done(outp, _)) => {
                    ok.fetch_add(1, Ordering::SeqCst);
                    // 同格式转到同一个文件夹时输出就是源路径：这一下删的不是
                    // "原文件"，是刚转出来的那份——删完这批等于白转。
                    if del_src && !encode::paths_eq(&outp, p) {
                        let _ = clip::delete_to_recycle(&[p.clone()]);
                    }
                }
                Ok(encode::Outcome::Skipped) => {
                    skip.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => {
                    err.fetch_add(1, Ordering::SeqCst);
                    if let Ok(mut v) = CONV_ERRS.lock() {
                        if v.len() < 5 {
                            v.push(format!("{}：{e}", stem(p)));
                        }
                    }
                }
            }
            let n = done.fetch_add(1, Ordering::SeqCst) + 1;
            prog_done.store(n, Ordering::SeqCst);
            if n % 4 == 0 || n == total {
                let pct = n * 100 / total.max(1);
                app().lock().unwrap().msg = format!("正在转换 {n}/{total}（{pct}%，Esc 停止）：{}", stem(p));
                let hwnd = HWND(hp as *mut core::ffi::c_void);
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_PV, WPARAM(0), LPARAM(0));
                }
            }
        }));
    }
    std::thread::spawn(move || {
        for h in hs {
            let _ = h.join();
        }
        let stopped = CONV_STOP.load(Ordering::SeqCst);
        let (o, s, e) = (ok.load(Ordering::SeqCst), skip.load(Ordering::SeqCst), err.load(Ordering::SeqCst));
        if let Ok(mut l) = CONV_LAST.lock() {
            *l = (o, s, e, stopped);
        }
        CONV_RUN.store(false, Ordering::SeqCst);
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CONV_DONE, WPARAM(0), LPARAM(0));
        }
    });
}

unsafe fn dlg_ok(hwnd: HWND) {
    let (kind, fmt, opt, scope_db, ed, ask_cmd) = {
        let g = app().lock().unwrap();
        let Some(d) = g.dlg.as_ref() else { return };
        (d.kind, d.fmt, d.opt.clone(), d.scope_db, g.dlg_edit, d.ask_cmd)
    };
    match kind {
        DlgKind::Ask => {
            close_dlg(hwnd);
            if ask_cmd == D_COVER_YES {
                // 这句是皮肤设置窗问的：底下压着的设置窗已经换回来，直接办覆盖。
                skin_commit(hwnd);
            } else if ask_cmd != 0 {
                // 告诉 command() 这次是确认过的，别再弹一遍框
                app().lock().unwrap().ask_go = ask_cmd;
                command(hwnd, ask_cmd);
            }
        }
        DlgKind::Convert => {
            let (paths, mut out, ow, del, bopt) = {
                let g = app().lock().unwrap();
                let o = g.conv_opts.get(fmt).cloned().unwrap_or_default();
                (g.selected_files(), g.conv_out.clone(), g.conv_overwrite, g.conv_del, o)
            };
            if let Some(ed) = ed {
                out = edit_text(ed).trim().to_string();
            }
            {
                let mut g = app().lock().unwrap();
                // 框里改过路径就落定，之后不再跟当前文件夹；没动过就保持跟手。
                if out != g.conv_out {
                    g.conv_out_auto = false;
                }
                g.conv_out = out.clone();
                g.conv_fmt = fmt;
            }
            if paths.is_empty() {
                app().lock().unwrap().msg = "没有选中要转换的图片".to_string();
                refresh(hwnd);
                return;
            }
            if out.is_empty() || !Path::new(&out).is_dir() {
                app().lock().unwrap().msg = "输出文件夹不存在".to_string();
                refresh(hwnd);
                return;
            }
            let policy = match ow {
                0 => encode::Collision::Overwrite,
                2 => encode::Collision::Rename,
                // 3 = 询问：批量里等同跳过，否则每张都弹框会把队列卡死
                _ => encode::Collision::Skip,
            };
            close_dlg(hwnd);
            start_conv(hwnd, paths, fmt, bopt, out, policy, del);
        }
        DlgKind::FmtOpt => {
            {
                let mut g = app().lock().unwrap();
                if fmt < g.conv_opts.len() {
                    g.conv_opts[fmt] = opt.clone();
                }
                if scope_db {
                    save_conv_cfg(&g.conv_opts);
                }
            }
            back_to_conv(hwnd);
        }
        DlgKind::BatchRename => {
            let tpl = if let Some(ed) = ed {
                edit_text(ed).trim().to_string()
            } else {
                app().lock().unwrap().rn_tpl.clone()
            };
            let (paths, start, case) = {
                let g = app().lock().unwrap();
                (g.selected_files(), g.rn_start, g.rn_case)
            };
            if paths.is_empty() {
                close_dlg(hwnd);
                app().lock().unwrap().msg = "没有选中要重命名的文件".to_string();
                refresh(hwnd);
                return;
            }
            if tpl.is_empty() {
                app().lock().unwrap().msg = "文件名模板不能为空".to_string();
                refresh(hwnd);
                return;
            }
            app().lock().unwrap().rn_tpl = tpl.clone();
            let mut pairs: Vec<(String, String)> = Vec::new();
            let mut used: HashSet<String> = HashSet::new();
            let mut bad = 0usize;
            for (i, p) in paths.iter().enumerate() {
                let name = rn_new_name(&tpl, start, i, p, case);
                if name.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
                    bad += 1;
                    continue;
                }
                let dst = Path::new(p).with_file_name(&name);
                let dsts = dst.display().to_string();
                if dst == Path::new(p) {
                    continue;
                }
                let key = dsts.to_lowercase();
                if used.contains(&key) || dst.exists() {
                    bad += 1;
                    continue;
                }
                used.insert(key);
                pairs.push((p.clone(), dsts));
            }
            let mut okn = 0usize;
            let mut moved: Vec<(String, String)> = Vec::new();
            for (old, new) in &pairs {
                if std::fs::rename(old, new).is_ok() {
                    okn += 1;
                    cache::move_path(old, new);
                    sim::moved(old, new);
                    moved.push((old.clone(), new.clone()));
                } else {
                    bad += 1;
                }
            }
            let mut m = format!("批量重命名完成：成功 {okn} 个");
            if bad > 0 {
                m.push_str(&format!("，跳过 {bad} 个（重名、非法字符或改名失败）"));
            }
            {
                let mut g = app().lock().unwrap();
                if let Some(e) = sync_cat_paths(&mut g, &moved) {
                    m.push_str(&format!("；{e}"));
                }
                g.msg = m;
                g.refresh_src(hwnd);
            }
            close_dlg(hwnd);
        }
        DlgKind::Search => {
            let kw = if let Some(ed) = ed { edit_text(ed) } else { String::new() };
            app().lock().unwrap().search_kw = kw.clone();
            close_dlg(hwnd);
            start_search(hwnd, kw, scope_db);
        }
        // 关联页回车等同按「应用」：勾完直接回车是最顺的一条路。
        DlgKind::Assoc => assoc_apply(hwnd),
        DlgKind::NewCat => {
            let name = if let Some(ed) = ed { edit_text(ed).trim().to_string() } else { String::new() };
            if name.is_empty() {
                app().lock().unwrap().msg = "类别名称不能为空".to_string();
                refresh(hwnd);
                return;
            }
            if name.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
                app().lock().unwrap().msg = "类别名称里有不能用的字符".to_string();
                refresh(hwnd);
                return;
            }
            {
                let mut g = app().lock().unwrap();
                let sel = g.org_sel;
                let mut parent = 0;
                let dup = if let Some(cat) = g.cat.as_ref() {
                    if sel != 0 && !cat.name(sel).is_empty() {
                        parent = sel;
                    }
                    let sib: Vec<i64> = if parent == 0 {
                        cat.roots.clone()
                    } else {
                        cat.child_ids(parent).to_vec()
                    };
                    sib.iter().any(|id| cat.name(*id).eq_ignore_ascii_case(&name))
                } else {
                    false
                };
                if dup {
                    g.msg = "同一级已经有这个类别".to_string();
                    drop(g);
                    refresh(hwnd);
                    return;
                }
                let mut new_id = 0;
                let done = {
                    if let Some(cat) = g.cat.as_mut() {
                        let id = cat.new_cat(parent, &name);
                        new_id = id;
                        match cat.save_full(&catalog_file()) {
                            Ok(()) => format!("已新建类别「{name}」"),
                            Err(e) => format!("类别已新建，但保存失败：{e}"),
                        }
                    } else {
                        "分类数据库还没载入".to_string()
                    }
                };
                if new_id != 0 {
                    g.org_sel = new_id;
                    if parent != 0 {
                        g.org_open_ids.insert(parent);
                    }
                }
                g.msg = done;
            }
            close_dlg(hwnd);
        }
        DlgKind::RenameCat => {
            let name = if let Some(ed) = ed { edit_text(ed).trim().to_string() } else { String::new() };
            let id = app().lock().unwrap().org_sel;
            if id == 0 {
                close_dlg(hwnd);
                return;
            }
            if name.is_empty() || name.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
                app().lock().unwrap().msg = "类别名称里有不能用的字符".to_string();
                refresh(hwnd);
                return;
            }
            let outcome = {
                let mut g = app().lock().unwrap();
                let mut out = "分类数据库还没载入".to_string();
                if let Some(cat) = g.cat.as_mut() {
                    let parent = cat.cats.iter().find(|c| c.id == id).map(|c| c.parent).unwrap_or(0);
                    let sib = if parent == 0 {
                        cat.roots.clone()
                    } else {
                        cat.child_ids(parent).to_vec()
                    };
                    if sib.iter().any(|x| *x != id && cat.name(*x).eq_ignore_ascii_case(&name)) {
                        out = "同一级已经有这个类别".to_string();
                    } else if cat.rename_cat(id, &name) {
                        out = match cat.save_full(&catalog_file()) {
                            Ok(()) => format!("类别已改名为「{name}」"),
                            Err(e) => format!("名字已改，但保存失败：{e}"),
                        };
                    }
                }
                out
            };
            app().lock().unwrap().msg = outcome;
            close_dlg(hwnd);
        }
        DlgKind::CatDb | DlgKind::ThumbDb => close_dlg(hwnd),
        // 规则对话框的回车不替人按「保存」：清理是个有后果的动作，走按钮。
        DlgKind::ScanRule => {}
        // 以图搜图那一页全是按钮点出来的（选图/补齐/查找），回车不该替人按下哪一个。
        DlgKind::SimSearch => {}
        // 改动已经实时叠到界面上了，确定这一步只问一句：这套颜色要不要写进皮肤文件。
        DlgKind::Skin => {
            let save_as = {
                let g = app().lock().unwrap();
                let Some(d) = g.dlg.as_ref() else { return };
                d.save_as
            };
            if save_as {
                skin_save_as(hwnd);
                return;
            }
            let name = app().lock().unwrap().skin.clone();
            // 底下这个设置窗先压住，确认框浮上来；答"否"或 Esc 会把它原样换回去。
            open_ask(
                hwnd,
                "覆盖当前皮肤",
                &format!(
                    "要把这次的改动写进皮肤「{name}」的文件吗？\n\n点「是」：这套颜色存进皮肤文件，换台机器也带着走。\n点「否」：回到设置页继续调，一个字也不写盘。"
                ),
                true,
                D_COVER_YES,
            );
        }
    }
}

unsafe fn browse_for_folder(hwnd: HWND, title: &str) -> Option<String> {
    let mut disp = [0u16; 260];
    let bi = BROWSEINFOW {
        hwndOwner: hwnd,
        pszDisplayName: PWSTR(disp.as_mut_ptr()),
        lpszTitle: PCWSTR(pw(title).as_ptr()),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
        ..Default::default()
    };
    let pidl = SHBrowseForFolderW(&bi);
    if pidl.is_null() {
        return None;
    }
    let mut buf = [0u16; 260];
    let ok = SHGetPathFromIDListW(pidl, &mut buf);
    CoTaskMemFree(Some(pidl as *const core::ffi::c_void));
    if !ok.as_bool() {
        return None;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

unsafe fn browse_for_file(
    hwnd: HWND,
    title: &str,
    save: bool,
    def_name: &str,
    init_dir: Option<&Path>,
) -> Option<PathBuf> {
    let mut buf = vec![0u16; 1024];
    for (i, c) in def_name.encode_utf16().take(buf.len() - 1).enumerate() {
        buf[i] = c;
    }
    let filter: Vec<u16> =
        "数据库备份压缩包 (*.zip)\0*.zip\0所有文件 (*.*)\0*.*\0\0".encode_utf16().collect();
    let dir: Vec<u16> = init_dir
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
        .encode_utf16()
        .chain([0])
        .collect();
    let title_w = pw(title);
    let ext_w = pw("zip");
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        nFilterIndex: 1,
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrInitialDir: if init_dir.is_some() { PCWSTR(dir.as_ptr()) } else { PCWSTR::null() },
        lpstrTitle: PCWSTR(title_w.as_ptr()),
        Flags: OFN_EXPLORER
            | OFN_PATHMUSTEXIST
            | OFN_HIDEREADONLY
            | if save { OFN_OVERWRITEPROMPT } else { OFN_FILEMUSTEXIST },
        lpstrDefExt: PCWSTR(ext_w.as_ptr()),
        ..Default::default()
    };
    let ok = if save { GetSaveFileNameW(&mut ofn) } else { GetOpenFileNameW(&mut ofn) };
    if !ok.as_bool() {
        return None;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let mut p = PathBuf::from(String::from_utf16_lossy(&buf[..end]));
    if save
        && p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref()
            != Some("zip")
    {
        p.set_extension("zip");
    }
    Some(p)
}

/// 挑一张用来找的图片。过滤器只列能读的图，起步落在当前这个文件夹。
unsafe fn pick_sim_image(hwnd: HWND) -> Option<String> {
    let mut buf = vec![0u16; 1024];
    let filter: Vec<u16> = "图像文件 (*.jpg *.png *.gif *.bmp *.tif *.webp)\0*.jpg;*.jpeg;*.png;*.bmp;*.gif;*.tif;*.tiff;*.webp\0所有文件 (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let dir: Vec<u16> = app()
        .lock()
        .unwrap()
        .dir
        .to_string_lossy()
        .to_string()
        .encode_utf16()
        .chain([0])
        .collect();
    let title_w = pw("挑一张用来找的图片");
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        nFilterIndex: 1,
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrInitialDir: PCWSTR(dir.as_ptr()),
        lpstrTitle: PCWSTR(title_w.as_ptr()),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_HIDEREADONLY,
        ..Default::default()
    };
    if !GetOpenFileNameW(&mut ofn).as_bool() {
        return None;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

fn reg_get(name: &str) -> String {
    unsafe {
        let mut hk = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(pw("Control Panel\\Desktop").as_ptr()),
            Some(0),
            KEY_READ,
            &mut hk,
        )
        .0
            != 0
        {
            return String::new();
        }
        let mut buf = vec![0u8; 2048];
        let mut sz = buf.len() as u32;
        let r = RegQueryValueExW(
            hk,
            PCWSTR(pw(name).as_ptr()),
            None,
            None,
            Some(buf.as_mut_ptr()),
            Some(&mut sz),
        );
        let _ = RegCloseKey(hk);
        if r.0 != 0 {
            return String::new();
        }
        let wide: Vec<u16> = buf[..sz as usize]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&c| c != 0)
            .collect();
        String::from_utf16_lossy(&wide)
    }
}

fn reg_set(name: &str, val: &str) {
    unsafe {
        let mut hk = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(pw("Control Panel\\Desktop").as_ptr()),
            Some(0),
            KEY_SET_VALUE,
            &mut hk,
        )
        .0
            != 0
        {
            return;
        }
        let wide: Vec<u16> = val.encode_utf16().chain([0]).collect();
        let bytes: Vec<u8> = wide.iter().flat_map(|w| w.to_le_bytes()).collect();
        let _ = RegSetValueExW(hk, PCWSTR(pw(name).as_ptr()), None, REG_SZ, Some(&bytes));
        let _ = RegCloseKey(hk);
    }
}

unsafe fn set_wallpaper(hwnd: HWND, mode: usize) {
    if mode == 2 {
        let old = INIT_WALL
            .get_or_init(|| Mutex::new(String::new()))
            .lock()
            .unwrap()
            .clone();
        if old.is_empty() {
            app().lock().unwrap().msg = "没有记录到原来的墙纸".to_string();
            refresh(hwnd);
            return;
        }
        reg_set("WallpaperStyle", "0");
        reg_set("TileWallpaper", "0");
        let w: Vec<u16> = old.encode_utf16().chain([0]).collect();
        let _ = SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(w.as_ptr() as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(SPIF_UPDATEINIFILE.0 | SPIF_SENDCHANGE.0),
        );
        app().lock().unwrap().msg = "墙纸已经还原".to_string();
        refresh(hwnd);
        return;
    }
    let p = app().lock().unwrap().selected_paths().into_iter().next();
    let Some(p) = p else {
        app().lock().unwrap().msg = "先选一张图片".to_string();
        refresh(hwnd);
        return;
    };
    if !scan::is_image(&p) {
        app().lock().unwrap().msg = "选中的不是图片".to_string();
        refresh(hwnd);
        return;
    }
    if mode == 1 {
        reg_set("WallpaperStyle", "0");
        reg_set("TileWallpaper", "1");
    } else {
        reg_set("WallpaperStyle", "0");
        reg_set("TileWallpaper", "0");
    }
    let w: Vec<u16> = p.encode_utf16().chain([0]).collect();
    let r = SystemParametersInfoW(
        SPI_SETDESKWALLPAPER,
        0,
        Some(w.as_ptr() as *mut core::ffi::c_void),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(SPIF_UPDATEINIFILE.0 | SPIF_SENDCHANGE.0),
    );
    app().lock().unwrap().msg = if r.is_ok() { "墙纸已经换好".to_string() } else { "换墙纸失败".to_string() };
    refresh(hwnd);
}

unsafe fn show_props(hwnd: HWND) {
    // 树落点优先（右键或键盘焦点在树上），看的是那个文件夹本体，不看格子里的选区。
    let (tree, tp) = tree_target();
    let paths = if tree {
        match tp {
            Some(d) => vec![d],
            None => {
                app().lock().unwrap().msg = "先在文件夹树上选一个文件夹".to_string();
                refresh(hwnd);
                return;
            }
        }
    } else {
        app().lock().unwrap().selected_paths()
    };
    let Some(first) = paths.first().cloned() else {
        app().lock().unwrap().msg = "没有选中文件".to_string();
        refresh(hwnd);
        return;
    };
    if paths.len() > 1 {
        // 多选：文件和文件夹分开数，大小只算文件——文件夹自己那条 metadata 长度是 0，
        // 混进合计里只会让人以为选中的东西很小。
        let (mut nf, mut nd, mut sz) = (0usize, 0usize, 0u64);
        for p in &paths {
            if let Ok(m) = std::fs::metadata(p) {
                if m.is_dir() {
                    nd += 1;
                } else {
                    nf += 1;
                    sz += m.len();
                }
            }
        }
        let mut body = format!("已选 {} 项\n文件：{nf} 个\n文件夹：{nd} 个", paths.len());
        if nf > 0 {
            body.push_str(&format!("\n合计大小：{}（{sz} 字节）", fmt_size(sz)));
        }
        open_ask(hwnd, "属性", &body, false, 0);
        return;
    }
    let meta = std::fs::metadata(&first).ok();
    let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
    // 文件名带扩展名，和资源管理器一致；盘根没有末级名字，整条路径顶上去。
    let name = Path::new(&first)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| first.clone());
    let dir = Path::new(&first)
        .parent()
        .map(|p| p.display().to_string())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "此电脑".to_string());
    let kind = if is_dir {
        "文件夹".to_string()
    } else if scan::is_image(&first) {
        format!("{} 图像", kind_label(Path::new(&first)))
    } else if Path::new(&first).extension().is_some() {
        format!("{} 文件", kind_label(Path::new(&first)))
    } else {
        "文件".to_string()
    };
    let mut body = format!("文件名：{name}\n类型：{kind}\n位置：{dir}\n");
    if let Some(m) = &meta {
        if !is_dir {
            body.push_str(&format!("大小：{}（{} 字节）\n", fmt_size(m.len()), m.len()));
        }
        let c = fmt_sys_time(m.created());
        let mo = fmt_sys_time(m.modified());
        let ac = fmt_sys_time(m.accessed());
        if !c.is_empty() {
            body.push_str(&format!("创建时间：{c}\n"));
        }
        if !mo.is_empty() {
            body.push_str(&format!("修改时间：{mo}\n"));
        }
        if !ac.is_empty() {
            body.push_str(&format!("访问时间：{ac}\n"));
        }
        // 只列常规页会亮出来的那三个；“存档”几乎每个文件都有，写上去只会占一行。
        let attrs = m.file_attributes();
        let mut at: Vec<&str> = Vec::new();
        if attrs & FILE_ATTRIBUTE_READONLY.0 != 0 {
            at.push("只读");
        }
        if attrs & FILE_ATTRIBUTE_HIDDEN.0 != 0 {
            at.push("隐藏");
        }
        if attrs & FILE_ATTRIBUTE_SYSTEM.0 != 0 {
            at.push("系统");
        }
        body.push_str(&format!(
            "属性：{}\n",
            if at.is_empty() { "无".to_string() } else { at.join("、") }
        ));
    } else {
        body.push_str("（读不到这个文件的详细信息）\n");
    }
    if !is_dir {
        if let Some((w, h, bpp)) = job::open_meta(&first) {
            let depth = if bpp > 0 { format!(" · {bpp} 位") } else { String::new() };
            body.push_str(&format!("尺寸：{w} × {h}{depth}\n"));
        }
    }
    open_ask(hwnd, "属性", &body, false, 0);
}

/// 整界面从头画一遍要先有块画布：现在是每帧新建一张和窗口一样大的位图，
/// 2560×1386 那张四百万字节，实测四毫秒，一个像素的内容都没多画。
/// 按窗口尺寸存一张位图复用，尺寸变了（拖窗口、换缩放）才丢旧的重建。
/// 画布的 DC 仍旧建 —— 那些画笔、画刷是一帧一换的，DC 留着会把上一帧的对象
/// 握过帧边界，不值得为省几十微秒冒这个险。只有画面线程碰这张位图。
struct Back {
    bmp: HBITMAP,
    w: i32,
    h: i32,
}

thread_local! {
    static BACK: std::cell::RefCell<Option<Back>> = const { std::cell::RefCell::new(None) };
}

/// 第二个返回值是真：这张是刚建出来的，里面一个像素都还没有，上一帧的内容指望不得。
unsafe fn back_bmp(hdc: HDC, w: i32, h: i32) -> (HBITMAP, bool) {
    let same = BACK.with(|c| match c.borrow().as_ref() {
        Some(x) if x.w == w && x.h == h => Some(x.bmp),
        _ => None,
    });
    if let Some(b) = same {
        return (b, false);
    }
    BACK.with(|c| {
        if let Some(b) = c.borrow_mut().take() {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(b.bmp.0));
            }
        }
    });
    let bmp = CreateCompatibleBitmap(hdc, w.max(1), h.max(1));
    BACK.with(|c| *c.borrow_mut() = Some(Back { bmp, w, h }));
    (bmp, true)
}

unsafe fn paint(hwnd: HWND) {
    // 工具栏图标用哪套开源库，画图标的那几层手上没有 Theme，一帧开头记一次就够。
    ICON_SET.set(app().lock().unwrap().th.art);
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let (ft, fgap) = frame_begin();
    let rc = client(hwnd);
    if xsee_host::viewer::is_open() {
        // 大图态整张脸都是查看器的。画在这块双缓冲上再一次性贴出去，
        // 后台出图时不会看到半帧擦除，也就不会闪。
        let mem = CreateCompatibleDC(Some(hdc));
        let bmp = CreateCompatibleBitmap(hdc, (rc.right - rc.left).max(1), (rc.bottom - rc.top).max(1));
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        {
            let mut g = app().lock().unwrap();
            let s = g.th.clone();
            hide_conv_edits(&g);
            fill(mem, &RECT { left: 0, top: 0, right: rc.right, bottom: rc.bottom }, s.skin.win);
            g.btns.clear();
        }
        flush_edit_ops(hwnd);
        xsee_host::viewer::paint(mem, rc);
        // 标题栏最后画：放大后的图比画布大，会画到顶上，得让它压在标题栏底下。
        {
            let mut g = app().lock().unwrap();
            let s = g.th.clone();
            if xsee_host::viewer::title_h() > 0 {
                paint_title(mem, &mut g, &s, &rc);
            }
        }
        let _ = BitBlt(hdc, 0, 0, rc.right, rc.bottom, Some(mem), 0, 0, SRCCOPY);
        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        frame_end(ft, fgap, hwnd);
        let _ = EndPaint(hwnd, &ps);
        return;
    }
    // 正拖着浮层：重绘区只有浮层走过的那条带，内容照样要画回来。
    let drag = DRAGGING.load(Ordering::Relaxed);
    render_into(hwnd, hdc, drag, &ps.rcPaint);
    flush_edit_ops(hwnd);
    frame_end(ft, fgap, hwnd);
    let _ = EndPaint(hwnd, &ps);
}

/// 整界面从头画一遍。`drag` 为真就是浮层正被人拖着走的那一帧：重绘区只是
/// 旧浮层与新浮层拼出来的那条带，带子底下的树、格子、预览必须照常画，
/// 只是这一帧把指针当成不在屏上，免得浮层滑过的按钮、树行跟着乱亮。
/// `upd` 是这一帧真正要补的脏区：它整块落在列表里时（滚轮、拖滚动条），树、预览、
/// 右栏、上栏一个字都不动，就不重画它们了。
unsafe fn render_into(hwnd: HWND, hdc: HDC, drag: bool, upd: &RECT) {
    let rc = client(hwnd);
    let seg_on = seg_tracing() && PAINT_SRC.load(Ordering::Relaxed) == 1;
    let mut seg_acc = [0u32; 8];
    let mut seg_t = Instant::now();
    macro_rules! segmark {
        ($i:expr) => {{
            let now = Instant::now();
            seg_acc[$i] = seg_acc[$i].saturating_add(now.duration_since(seg_t).as_micros() as u32);
            seg_t = now;
        }};
    }
    let mh = menu_h(hwnd);
    let mut g = app().lock().unwrap();
    let s = g.th.clone();
    let z = zones(&g, rc, mh);
    // 脏区整块落在列表里 = 这一帧只有列表在动。
    let gz = grid_box(&z, g.mode == Mode::Detail, &s);
    let only_grid = !drag && within(upd, &gz);
    let ur = RECT {
        left: upd.left.max(rc.left),
        top: upd.top.max(rc.top),
        right: upd.right.min(rc.right),
        bottom: upd.bottom.min(rc.bottom),
    };
    g.clamp_scroll(z.grid);
    g.bar = z.bar;
    g.path_bar = z.path;
    // 按钮命中表每帧重收一遍。清空放在这里而不是 paint_bar 里：工具栏收起来时
    // paint_bar 直接返回，那里清不掉，上一帧的矩形会留在原地，格子上的点击会
    // 被当成工具栏按钮。
    // 只画列表那一帧不能清：上栏、树、右栏这一帧没重画，它们的按钮就没人重新登记。
    if !only_grid {
        g.btns.clear();
    }
    let mem = CreateCompatibleDC(Some(hdc));
    let (bmp, fresh) = back_bmp(hdc, rc.right - rc.left, rc.bottom - rc.top);
    let old = SelectObject(mem, HGDIOBJ(bmp.0));
    // 条带：这一帧只有列表在动，就把位图里上一帧那一屏整个搬到新的滚动位置，
    // 只补新露出来的那一条。上一帧的内容能不能接着用，看三条：位图不是刚重建的、
    // 除了滚动位置别的都没变（新缩略图到齐那种算变了）、这一帧挪的距离不到一屏
    // —— 不到一屏才还有重叠的部分可搬。挪过一屏去的跳变不回退成整窗重绘：
    // 列表整块重画，树、预览、右栏、上栏照旧一个字不动。
    let dy = g.scroll - g.paint_scroll;
    let gh = (z.grid.bottom - z.grid.top).max(1);
    let mut band: Option<RECT> = None;
    if only_grid
        && !fresh
        && g.paint_ok
        && !g.grid_stale
        && dy != 0
        && dy.abs() < gh
        && matches!(g.mode, Mode::Grid | Mode::Small)
        && g.has_below(z.grid)
    {
        let _ = ScrollDC(mem, 0, -dy, Some(&z.grid), Some(&z.grid), None, None);
        // 搬完空出来的那一条：往下滚露在底下，往上滚露在顶上
        band = Some(if dy > 0 {
            RECT { left: z.grid.left, top: (z.grid.bottom - dy).max(z.grid.top), right: z.grid.right, bottom: z.grid.bottom }
        } else {
            RECT { left: z.grid.left, top: z.grid.top, right: z.grid.right, bottom: (z.grid.top - dy).min(z.grid.bottom) }
        });
    }
    match &band {
        // 搬过来的那一屏就是背景：脏区里除了这一条什么都不用铺，
        // 这一条由地面补缝、压到它的格子补齐，画到哪儿铺到哪儿。
        Some(_) => {}
        None if only_grid => fill(mem, &ur, s.skin.win),
        None => fill(mem, &RECT { left: 0, top: 0, right: rc.right, bottom: rc.bottom }, s.skin.win),
    }
    if only_grid {
        GRID_FRAMES.fetch_add(1, Ordering::Relaxed);
        if band.is_some() {
            BAND_FRAMES.fetch_add(1, Ordering::Relaxed);
        }
    }
    segmark!(0);
    // 右键弹层盖着的时候，指针只是从弹层上扫过：画底下那屏时把指针当成不在屏上，
    // 树行、格子、按钮的悬停才不会在菜单底下跟着跑。只有弹层自己那几笔用真实位置。
    let pop = g.ctx_at.is_some() || g.ctx_cat.is_some();
    let live_pt = g.pt;
    if pop {
        g.pt = (-1_000_000, -1_000_000);
    }
    // 每块可以单独换字色：换的是这一块拿到的 Theme，画法一行不用动。
    let st_bar = zone_theme(&s, &g.look, theme::Z_BAR);
    let st_menu = zone_theme(&s, &g.look, theme::Z_MENU);
    let st_path = zone_theme(&s, &g.look, theme::Z_PATH);
    let st_tree = sel_as_bar(&zone_theme(&s, &g.look, theme::Z_TREE));
    let st_grid = zone_theme(&s, &g.look, theme::Z_GRID);
    let st_pv = zone_theme(&s, &g.look, theme::Z_PV);
    let st_org = sel_as_bar(&zone_theme(&s, &g.look, theme::Z_ORG));
    let st_status = zone_theme(&s, &g.look, theme::Z_STATUS);
    let st_popup = zone_theme(&s, &g.look, theme::Z_POPUP);
    let st_dlg = zone_theme(&s, &g.look, theme::Z_DLG);
    if !only_grid {
        paint_bar(mem, &mut g, &st_bar);
        paint_title(mem, &mut g, &s, &rc);
        paint_menu_bar(mem, &mut g, &st_menu, &rc);
        paint_path(mem, &mut g, &st_path);
    }
    segmark!(1);
    // 浮层在不动的指针底下滑过去，一路上的按钮、树行、格子会跟着乱亮：拖动这一帧
    // 把指针当成不在屏上，松手那次全量重绘再恢复。
    if drag {
        g.pt = (-1_000_000, -1_000_000);
    }
    if !only_grid {
        paint_tree(mem, &mut g, &z, &st_tree);
    }
    segmark!(2);
    paint_grid(mem, &mut g, &z, &st_grid, band.as_ref());
    // 框选框：只画一道深灰边，中间不加任何色纱 —— 拖框途中盖一层颜色会像已经选中了整片。
    // 这里也不碰主色，主色是"已经选中"的说法。框按内容坐标记，所以这里减掉滚动量。
    if let Some((x0, y0, x1, y1)) = g.band {
        let r = RECT {
            left: x0.min(x1).clamp(z.grid.left, z.grid.right),
            top: (y0.min(y1) - g.scroll).clamp(z.grid.top, z.grid.bottom),
            right: x0.max(x1).clamp(z.grid.left, z.grid.right),
            bottom: (y0.max(y1) - g.scroll).clamp(z.grid.top, z.grid.bottom),
        };
        if r.right > r.left && r.bottom > r.top {
            frame(mem, &r, st_grid.skin.ink);
        }
    }
    segmark!(3);
    if !only_grid {
        paint_pv(mem, &mut g, &z, &st_pv);
        segmark!(4);
        // 右栏两页共用一块地方：当前是转换页就画转换，其余画整理
        if g.conv_open && (g.dock_tab == 1 || !g.org_open) {
            paint_conv(hwnd, mem, &mut g, &z, &st_org);
        } else {
            // 切回整理页时，转换页那五个原生输入框还停在原地，必须一起收掉，
            // 否则会浮在类别树上（它们不归自绘管，不会跟着页签消失）。
            hide_conv_edits(&g);
            paint_org(mem, &mut g, &z, &st_org);
        }
        paint_hint(mem, &mut g, &z, &st_grid);
        paint_status(mem, &mut g, &z, &st_status);
        // 分隔条：底色走窗底，中间一道握痕告诉人这里能拖。
        // 必须画在弹层和对话框之前：它是整高的一条，画晚了会横穿菜单和浮层，
        // 把菜单上的字拦腰截断。
        for sp in [z.spv, z.sph, z.spo] {
            if sp.right <= sp.left || sp.bottom <= sp.top {
                continue;
            }
            fill(mem, &sp, s.skin.win);
            let (cx, cy) = ((sp.left + sp.right) / 2, (sp.top + sp.bottom) / 2);
            if bands(&s) {
                // 三个圆点在深蓝上读成省略号；一道 17px 的握痕加两道浅影才读成“能拖”
                let hot = over(&sp, g.pt.0, g.pt.1);
                let (mc, sc) = if hot { (s.skin.acc, s.skin.acc_soft) } else { (s.skin.line, s.skin.line_soft) };
                let half = (8.5 * s.scale).round() as i32;
                if sp.right - sp.left >= sp.bottom - sp.top {
                    hline(mem, cx - half, cx + half, cy, mc);
                    hline(mem, cx - half, cx + half, cy - 2, sc);
                    hline(mem, cx - half, cx + half, cy + 2, sc);
                } else {
                    vline(mem, cx, cy - half, cy + half, mc);
                    vline(mem, cx - 2, cy - half, cy + half, sc);
                    vline(mem, cx + 2, cy - half, cy + half, sc);
                }
            } else {
                let r = (s.split / 3).clamp(1, 2);
                if sp.right - sp.left >= sp.bottom - sp.top {
                    for d in [-1, 0, 1] {
                        dot(mem, cx + d * (r * 3), cy, r, s.skin.line);
                    }
                } else {
                    for d in [-1, 0, 1] {
                        dot(mem, cx, cy + d * (r * 3), r, s.skin.line);
                    }
                }
            }
        }
    }
    segmark!(5);
    // 弹层最后画，压在所有格子之上
    if pop {
        g.pt = live_pt;
    }
    paint_open_menu(mem, &mut g, &st_popup, &rc);
    paint_ctx_menu(mem, &mut g, &st_popup, &rc);
    paint_bmenu(mem, &mut g, &st_popup, &rc);
    if pop {
        g.pt = (-1_000_000, -1_000_000);
    }
    // 对话框压在最上面
    if drag {
        // 浮层在不动的指针底下滑过去，一路上的按钮、色块会跟着乱亮：拖动这一帧把
        // 指针当成不在屏上，悬停等松手那次全量重绘再恢复。
        let keep = g.pt;
        g.pt = (-1_000_000, -1_000_000);
        paint_dlg(mem, &mut g, &st_dlg, &rc);
        g.pt = keep;
    } else {
        paint_dlg(mem, &mut g, &st_dlg, &rc);
    }
    // 剪切/复制之后整窗过一道纱：选框、格子、字全在纱底下透出来，
    // 220 毫秒自己收干净，一个像素都不动选区。
    if let Some(t0) = g.flash {
        let el = t0.elapsed().as_secs_f64();
        let dur = FLASH_MS.as_secs_f64();
        if el < dur {
            let alpha = (FLASH_ALPHA as f64 * (1.0 - el / dur)).round() as u8;
            veil(mem, &RECT { left: 0, top: 0, right: rc.right, bottom: rc.bottom }, s.skin.flash, alpha);
        }
    }
    // 窗口一圈实边：深色外框没有这道边，四条边会散进桌面背景里
    if bands(&s) {
        frame(mem, &RECT { left: 0, top: 0, right: rc.right, bottom: rc.bottom }, s.skin.line);
    }
    if pop || drag {
        g.pt = live_pt;
    }
    segmark!(6);
    // 只画列表那一帧，位图里列表以外那块和屏上本来就是同一份内容，拷上去也是白拷：
    // 上屏就限在列表那一块。
    if only_grid {
        let _ = BitBlt(hdc, gz.left, gz.top, gz.right - gz.left, gz.bottom - gz.top, Some(mem), gz.left, gz.top, SRCCOPY);
    } else {
        let _ = BitBlt(hdc, 0, 0, rc.right, rc.bottom, Some(mem), 0, 0, SRCCOPY);
    }
    SelectObject(mem, old);
    let _ = DeleteDC(mem);
    // 命中表按"这一帧真画上屏了"的那一份导出。以前只在画转换页时导出，
    // 面板被关掉之后文件里还是上一帧的旧坐标，照着它点就全点空。
    conv_dump_hits(&g, &z.org);
    segmark!(7);
    // 记下这一帧画进位图的是哪个滚动位置，下一帧只挪动就搬它。
    // 位图刚重建过又只补了列表那一块的那一帧，别处还是空的，不认。
    g.paint_scroll = g.scroll;
    g.paint_ok = !only_grid || !fresh;
    g.grid_stale = false;
    if seg_on {
        if let Ok(mut t) = LAST_SEG.lock() {
            *t = seg_acc;
        }
        seg_add(&seg_acc);
    }
    drop(g);
}

unsafe fn layout(hwnd: HWND) -> (Zones, RECT) {
    let rc = client(hwnd);
    let z = zones(&app().lock().unwrap(), rc, menu_h(hwnd));
    (z, rc)
}

/// 列表那一块的外接矩形：网格 + 右边那条滚动槽，详细资料档再加一条列头。
/// 滚动时动的只有它。
fn grid_box(z: &Zones, detail: bool, s: &Theme) -> RECT {
    let mut r = RECT {
        left: z.grid.left.min(z.gsb.left),
        top: z.grid.top.min(z.gsb.top),
        right: z.grid.right.max(z.gsb.right),
        bottom: z.grid.bottom.max(z.gsb.bottom),
    };
    if detail {
        let h = detail_head(z, s);
        r.left = r.left.min(h.left);
        r.top = r.top.min(h.top);
        r.right = r.right.max(h.right);
        r.bottom = r.bottom.max(h.bottom);
    }
    r
}

/// 只有列表在动（滚轮、拖滚动条）时用它：脏区就限在网格那一块。树、预览、右栏、上栏
/// 一个字都没变，跟着重画一遍实测要多花 25 毫秒，比网格本身还贵。
unsafe fn refresh_grid(hwnd: HWND) {
    if DRAGGING.load(Ordering::Relaxed) {
        return;
    }
    let r = {
        let g = app().lock().unwrap();
        // 有别的东西盖着就不玩这套：提示纱、菜单、浮层都画到网格外头去了。
        if g.flash.is_some()
            || g.ctx_at.is_some()
            || g.ctx_cat.is_some()
            || g.open_menu.is_some()
            || g.dlg.is_some()
            || g.band.is_some()
        {
            drop(g);
            return refresh(hwnd);
        }
        let rc = client(hwnd);
        let z = zones(&g, rc, menu_h(hwnd));
        grid_box(&z, g.mode == Mode::Detail, &g.th)
    };
    let _ = InvalidateRect(Some(hwnd), Some(&r), false);
}

unsafe fn refresh(hwnd: HWND) {
    // 拖着浮层的那一小会儿，别人要的整窗刷新先按下：WM_PAINT 只补浮层走出的脏带，
    // 这里再塞一整个窗口进去会把底下那一屏也重排一遍，跟着拖就会一顿一顿的。
    // 松手本来会补一次全量重绘，谁也不会少看什么。
    if DRAGGING.load(Ordering::Relaxed) {
        return;
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
}

/// 只脏一小块。空矩形（那一栏关着）直接跳过，别往失效区里塞垃圾。
unsafe fn invalidate(hwnd: HWND, r: &RECT) {
    if DRAGGING.load(Ordering::Relaxed) || r.right <= r.left || r.bottom <= r.top {
        return;
    }
    let _ = InvalidateRect(Some(hwnd), Some(r), false);
}

unsafe fn request_pv(hwnd: HWND) {
    let stamp = app().lock().unwrap().stamp;
    request_pv_gen(hwnd, stamp);
}

/// 右格预览另起一个线程要，冷读的巨型档不能卡住界面。
unsafe fn request_pv_gen(hwnd: HWND, stamp: usize) {
    let (path, box_, sel) = {
        let g = app().lock().unwrap();
        let Some(e) = g.entries.get(g.sel) else {
            *pv().lock().unwrap() = None;
            return;
        };
        if e.is_dir || !e.is_image {
            *pv().lock().unwrap() = None;
            return;
        }
        // 预览还没画过就没有盒宽，退回按左栏宽算，免得第一张要 64 像素的小图
        let bw = if g.pv_w > 0 { g.pv_w } else { g.left_w - g.th.gap * 2 };
        let w = bw.clamp(64, 640);
        (e.path.clone(), (w as u32, w as u32), g.sel)
    };
    if stamp != app().lock().unwrap().stamp {
        return;
    }
    let hp = hwnd.0 as isize;
    std::thread::spawn(move || {
        let hwnd = HWND(hp as *mut core::ffi::c_void);
        job::init_com();
        let Ok(t) = job::make(&path, box_.0, box_.1) else {
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_PV, WPARAM(0), LPARAM(0));
            }
            return;
        };
        let (nw, nh, bpp) = job::open_meta(&path).unwrap_or((t.img.w, t.img.h, 0));
        let px = rgba_to_bgra(&t.img.px);
        if app().lock().unwrap().stamp != stamp {
            return;
        }
        *pv().lock().unwrap() = Some((stamp, t.img.w, t.img.h, px, nw, nh, bpp));
        let mut g = app().lock().unwrap();
        if g.sel == sel {
            g.dims.insert(sel, (nw, nh));
            if bpp > 0 {
                g.bpps.insert(sel, bpp);
            }
        }
        drop(g);
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_PV, WPARAM(stamp), LPARAM(0));
        }
    });
}

unsafe fn wheel(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) {
    let d = ((wparam.0 >> 16) & 0xffff) as u16 as i16;
    let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
    // 只有滚列表那一档走部分重绘，别的滚轮目标照旧整窗刷
    let mut grid_only = false;
    // 滚轮的坐标是屏幕坐标，得先转客户端再判断落在哪个区
    let mut pt = POINT { x: (lparam.0 & 0xffff) as u16 as i16 as i32, y: ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32 };
    let _ = ScreenToClient(hwnd, &mut pt);
    let (z, _) = layout(hwnd);
    let mut g = app().lock().unwrap();
    if g.dlg.is_some() {
        // 以图搜图那一页是不模态的：滚轮落在它的结果表里就滚表，落在别处就当这页不存在。
        let free = g.dlg.as_ref().is_some_and(|d| d.kind == DlgKind::SimSearch);
        if free && over(&g.sim_list, pt.x, pt.y) {
            let n = g.sim_hits.len();
            let rh = (25.0 * g.scale) as i32;
            let visible = ((g.sim_list.bottom - g.sim_list.top) / rh.max(1)).max(1) as usize;
            let maxs = n.saturating_sub(visible) as i32;
            g.sim_scroll = (g.sim_scroll - (d as i32 / 120) * 3).clamp(0, maxs);
            drop(g);
            refresh(hwnd);
            return;
        }
        // 缩略图数据库页的扫描名单同一套办法：滚轮落在名单那块上就滚名单。
        if g.dlg.as_ref().is_some_and(|d| d.kind == DlgKind::ThumbDb)
            && g.scan_dirs.len() > SCAN_VIS
            && over(&g.scan_list, pt.x, pt.y)
        {
            let rh = (22.0 * g.scale) as i32;
            let visible = ((g.scan_list.bottom - g.scan_list.top) / rh.max(1)).max(1) as usize;
            let maxs = g.scan_dirs.len().saturating_sub(visible) as i32;
            g.scan_scroll = (g.scan_scroll as i32 - (d as i32 / 120) * 2).clamp(0, maxs) as usize;
            drop(g);
            refresh(hwnd);
            return;
        }
        if !free {
            return;
        }
    }
    if ctrl {
        // Ctrl+滚轮跟路径栏那对 −/+ 同一个步长：一格 10 像素，不许低于下限
        let step = if d > 0 { THUMB_H_STEP } else { -THUMB_H_STEP };
        g.thumb_h = (g.thumb_h + step).clamp(THUMB_H_MIN, THUMB_H_MAX);
        g.restart_pool();
        ui_save(&g, hwnd);
    } else if over(&z.org, pt.x, pt.y) {
        if g.conv_open && (g.dock_tab == 1 || !g.org_open) {
            g.conv_scroll = (g.conv_scroll - (d as i32 / 120) * g.th.row.max(1) * 3)
                .clamp(0, g.conv_max_scroll.max(0));
        } else {
            let top = z.org.top + org_head_h(&g.th);
            let h = (z.org.bottom - top).max(0);
            let n = org_rows(&g).len();
            let maxs = (n as i32 * g.th.row.max(1) - h).max(0);
            g.org_scroll = (g.org_scroll - (d as i32 / 120) * g.th.row.max(1) * 3).clamp(0, maxs);
        }
    } else if over(&z.tree, pt.x, pt.y) {
        g.tscroll = (g.tscroll - (d as i32 / 120) * g.th.row * 3).max(0);
    } else {
        // 滚轮只给冲量，真正的位移交给 16 毫秒的帧节拍去积分。
        // 连滚时冲量叠加成惯性，松手后再滑一段。
        let nd = g.max_scroll(z.grid);
        let dy = -((d as i32) / 120) * g.row_h() * 3;
        let mut v = g.scroll_vel + dy as f64 * 12.0;
        if (g.scroll <= 0 && v < 0.0) || (g.scroll >= nd && v > 0.0) {
            v = 0.0;
        }
        g.scroll_vel = v.clamp(-SCROLL_MAX_V, SCROLL_MAX_V);
        g.scroll_until = Instant::now() + Duration::from_millis(400);
        g.queue_visible(z.grid);
        grid_only = true;
    }
    drop(g);
    PAINT_SRC.store(1, Ordering::Relaxed);
    if grid_only {
        // 有速度才挂帧节拍；顶到边冲量作废就照旧补一帧
        if app().lock().unwrap().scroll_vel != 0.0 {
            start_anim(hwnd);
        } else {
            refresh_grid(hwnd);
        }
    } else {
        refresh(hwnd);
    }
}

static ANIM_ON: AtomicBool = AtomicBool::new(false);
/// 惯性刚停的宽限期里心跳攒下的整窗刷新：宽限一过，下一次心跳补上。
static DEFER_FULL: AtomicBool = AtomicBool::new(false);

#[link(name = "winmm")]
unsafe extern "system" {
    fn timeBeginPeriod(uperiod: u32) -> u32;
    fn timeEndPeriod(uperiod: u32) -> u32;
}

/// windows 那套绑定把 CreateWaitableTimerExW 挂在 Win32_Security 特性下，
/// 只为这一个调用去多开一整个特性不划算，这里直接接 kernel32。
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateWaitableTimerExW(
        lptimerattributes: *const core::ffi::c_void,
        lptimername: PCWSTR,
        dwflags: u32,
        dwdesiredaccess: u32,
    ) -> HANDLE;
}

/// 帧节拍用的等待定时器。建不出（老系统）就退回 SetTimer，帧距粗一点，功能不受影响。
static FRAME_TIMER: OnceLock<usize> = OnceLock::new();

fn frame_timer() -> HANDLE {
    let h = *FRAME_TIMER.get_or_init(|| unsafe {
        let h = CreateWaitableTimerExW(
            std::ptr::null(),
            PCWSTR::null(),
            CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
            TIMER_ALL_ACCESS.0,
        );
        h.0 as usize
    });
    HANDLE(h as *mut core::ffi::c_void)
}

/// 单调毫秒：帧的时刻只在进程内比较，不跟墙钟走。
static START: OnceLock<Instant> = OnceLock::new();
static NEXT_FRAME_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// 挂上（或保持）滚动帧节拍。重复 SetTimer 会重置节拍，所以只挂一次。
unsafe fn start_anim(hwnd: HWND) {
    if !ANIM_ON.swap(true, Ordering::Relaxed) {
        let _ = timeBeginPeriod(1);
        NEXT_FRAME_MS.store(0, Ordering::Relaxed);
        if frame_timer().0.is_null() {
            let _ = SetTimer(Some(hwnd), TMR_ANIM, FRAME_MS as u32, None);
        }
    }
}

unsafe fn stop_anim(hwnd: HWND) {
    if ANIM_ON.swap(false, Ordering::Relaxed) {
        let _ = KillTimer(Some(hwnd), TMR_ANIM);
        let _ = timeEndPeriod(1);
    }
}

/// 滚动帧节拍：速度积分 + 指数摩擦 + 边界夹紧，速度归零就摘节拍。
/// 动画期间被按下不表的整窗刷新（出图到齐、进度走光）在滚停的这一帧一次补齐。
unsafe fn anim_tick(hwnd: HWND) {
    let dt = FRAME_MS as f64 / 1000.0;
    let (z, _) = layout(hwnd);
    let mut g = app().lock().unwrap();
    let maxs = g.max_scroll(z.grid);
    let before = g.scroll;
    let mut v = g.scroll_vel;
    let mut ns = before as f64 + v * dt;
    let mut hit_edge = false;
    if ns <= 0.0 {
        ns = 0.0;
        hit_edge = v < 0.0;
    }
    if ns >= maxs as f64 {
        ns = maxs as f64;
        hit_edge = v > 0.0;
    }
    g.scroll = ns.round() as i32;
    v *= (-SCROLL_FRICTION * dt).exp();
    if hit_edge || v.abs() < SCROLL_STOP {
        v = 0.0;
    }
    g.scroll_vel = v;
    g.queue_visible(z.grid);
    let deferred = std::mem::take(&mut g.anim_defer);
    let done = v == 0.0;
    let moved = g.scroll != before;
    drop(g);
    PAINT_SRC.store(1, Ordering::Relaxed);
    if done {
        stop_anim(hwnd);
        if deferred {
            refresh(hwnd);
        } else if moved {
            refresh_grid(hwnd);
        }
    } else {
        refresh_grid(hwnd);
    }
}

fn over(r: &RECT, x: i32, y: i32) -> bool {    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

/// 两块矩形有没有压上。脏区只按这个判断。
fn cross(a: &RECT, b: &RECT) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

/// a 整块落在 b 里面（含贴边）。
fn within(a: &RECT, b: &RECT) -> bool {
    a.left >= b.left && a.top >= b.top && a.right <= b.right && a.bottom <= b.bottom
}

/// 地址栏文字里离 x 最近的那个字边界（UTF-16 偏移）。边界是量好的升序表，线性扫一遍就够。
fn addr_caret(xs: &[(usize, i32)], x: i32) -> usize {
    let mut best = 0usize;
    let mut bd = i32::MAX;
    for (o, bx) in xs {
        let d = (x - *bx).abs();
        if d < bd {
            bd = d;
            best = *o;
        }
    }
    best
}

/// 把一批文件真的拖出去（OLE 模态循环），松手后按落点结果刷新。
/// 格子和左树树行共用这一趟；2 = DROPEFFECT_MOVE：外面按移动接走了，
/// 源文件已经不在了，不重扫一遍的话旧缩略图还钉在原位，得手动刷新才消失。
unsafe fn run_file_drag(hwnd: HWND, paths: Vec<String>) {
    let _ = ReleaseCapture();
    SELF_DRAG.store(true, Ordering::SeqCst);
    let (_, eff) = xsee_host::drag::start(paths);
    SELF_DRAG.store(false, Ordering::SeqCst);
    {
        let mut g = app().lock().unwrap();
        g.drop_hl = None;
        if eff & 2 != 0 {
            if matches!(g.src, Src::Folder) {
                let d = g.dir.display().to_string();
                g.goto(&d, hwnd, false);
            } else {
                g.refresh_src(hwnd);
            }
        }
    }
    refresh(hwnd);
}

/// 按下点记下的起点，拖过系统那条拖动阈值才真的起拖。
fn drag_threshold(gone: (i32, i32)) -> bool {
    let tx = unsafe { GetSystemMetrics(SM_CXDRAG) }.max(1);
    let ty = unsafe { GetSystemMetrics(SM_CYDRAG) }.max(1);
    gone.0.abs() > tx || gone.1.abs() > ty
}

unsafe fn mouse_move(hwnd: HWND, lparam: LPARAM, keys: usize) {
    let x = (lparam.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    let (z, rc) = layout(hwnd);
    let mut g = app().lock().unwrap();
    g.pt = (x, y);
    // 工具栏 ▾ 菜单开着：这一趟只伺候菜单，底下的悬停全部钉住
    if g.bmenu.is_some() {
        drop(g);
        bmenu_move(hwnd, x, y);
        return;
    }
    if let Some((dx, dy)) = g.dlg_drag {
        // 按住浮层标题条拖：让浮层左上角跟住鼠标，扣掉按下时那点的偏移。
        // 只在客户区里活动，左右各留一点边，省得整页拖没了找不回来。
        let k = g.scale;
        let (edge, keep) = ((80.0 * k) as i32, (30.0 * k) as i32);
        let Some(d) = g.dlg.as_mut() else { return };
        let (nw, nh) = (d.rect.right - d.rect.left, d.rect.bottom - d.rect.top);
        // 脏区要一直盖到影子最外那一圈，不然拖过的地方留一道旧影子。
        let reach = shadow_off(k);
        let left = (x - dx).clamp(rc.left - nw + edge, rc.right - edge);
        let top = (y - dy).clamp(rc.top, rc.bottom - keep);
        // 顶到边上还在推：位置一动没动，这种 MOVE 一笔都不许画。
        if left == d.rect.left && top == d.rect.top {
            return;
        }
        // 要刷的只有"旧位置那一块 ∪ 新位置那一块"，整窗不碰：这一帧浮层走过的
        // 那条带之外，屏幕上的东西一个字都不动。
        let dirty = RECT {
            left: d.rect.left.min(left),
            top: d.rect.top.min(top),
            right: d.rect.right.max(left + nw) + reach,
            bottom: d.rect.bottom.max(top + nh) + reach,
        };
        d.rect = RECT { left, top, right: left + nw, bottom: top + nh };
        drop(g);
        place_edit(hwnd);
        let _ = InvalidateRect(Some(hwnd), Some(&dirty), false);
        return;
    }
    // 非模态那一页：指针在浮层外面就该照常伺候底下那屏（悬停高亮、光标形状）。
    let page_out = sim_loose(&g, x, y);
    if g.dlg.is_some() && !page_out {
        // 对话框开着也要自己钉指针：窗口类不挂光标时，系统会把指针设成 NULL，
        // 看着就是"输入的时候鼠标没了"。顺便只在按钮换人时重绘，别每一下移动都刷整屏。
        let hb = g
            .dlg_btns
            .iter()
            .find(|(_, r)| over(r, x, y))
            .map(|(i, _)| *i);
        let changed = hb != g.hover_btn;
        if changed {
            g.hover_btn = hb;
        }
        drop(g);
        if let Ok(c) = LoadCursorW(None, IDC_ARROW) {
            SetCursor(Some(c));
        }
        if changed {
            refresh(hwnd);
        }
        return;
    }
    // 框选：框按内容坐标记，滚动时跟住内容；指针顶到上下沿就自己滚。
    if let Some((x0, y0, _, _)) = g.band {
        // 按住左键的消息标志位自己会说明这一下还算不算拖动；捕获被抢走另有 WM_CAPTURECHANGED 兜底。
        if keys & MK_LBUTTON.0 as usize == 0 && GetCapture() != hwnd {
            g.band = None;
            drop(g);
            refresh(hwnd);
            return;
        }
        if y < z.grid.top {
            let v = (g.scroll - g.row_h() / 2).max(0);
            g.scroll_set(v);
            g.queue_visible(z.grid);
        } else if y >= z.grid.bottom {
            let maxs = g.max_scroll(z.grid);
            let v = (g.scroll + g.row_h() / 2).min(maxs);
            g.scroll_set(v);
            g.queue_visible(z.grid);
        }
        let x1 = x.clamp(z.grid.left, z.grid.right);
        let cur = (x1, y + g.scroll);
        g.band = Some((x0, y0, cur.0, cur.1));
        let hits = g.band_pick(z.grid, x0, y0 - g.scroll, cur.0, cur.1 - g.scroll);
        let mut sel = g.band_add.clone();
        for i in &hits {
            sel.insert(g.entries[*i].path.clone());
        }
        g.marked = sel;
        if let Some(&last) = hits.last() {
            g.sel = last;
            g.sel_lit = false;
        }
        g.stamp += 1;
        drop(g);
        refresh(hwnd);
        return;
    }
    if let Some((sx, sy, idx0)) = g.drag_seed {
        let down = GetKeyState(VK_LBUTTON.0 as i32) < 0;
        if !down {
            g.drag_seed = None;
        } else if drag_threshold((x - sx, y - sy)) {
            g.drag_seed = None;
            let paths: Vec<String> = match g.entries.get(idx0) {
                Some(e) if g.marked.contains(&e.path) => g.marked.iter().cloned().collect(),
                Some(e) => vec![e.path.clone()],
                None => Vec::new(),
            };
            drop(g);
            run_file_drag(hwnd, paths);
            return;
        }
    }
    // 左树那一行也可以拖走：拖到另一行就是把这个文件夹搬进去
    if let Some((sx, sy, path)) = g.tree_seed.clone() {
        let down = GetKeyState(VK_LBUTTON.0 as i32) < 0;
        if !down {
            g.tree_seed = None;
        } else if drag_threshold((x - sx, y - sy)) {
            g.tree_seed = None;
            drop(g);
            run_file_drag(hwnd, vec![path]);
            return;
        }
    }
    if let Some(an) = g.addr_anchor {
        let c = if g.addr_full {
            addr_caret(&g.addr_xs, x)
        } else {
            g.addr_n
        };
        let want = (an.min(c), an.max(c));
        if g.addr_sel != Some(want) {
            g.addr_sel = Some(want);
            drop(g);
            refresh(hwnd);
        }
        return;
    }
    let mut need = false;
    // 右键弹层：扫过带 ▸ 的行就展开子层，鼠标一动就重绘让高亮跟手
    if g.ctx_at.is_some() {
        need = true;
        // 子层画在主层之上，重叠区归子层：先判主层的话，指针跨到子层那一刻子层就被关掉
        let in_sub = g.ctx_sub.is_some() && over(&g.ctx_sub_rect, x, y);
        let row = if in_sub {
            None
        } else {
            g.ctx_rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i)
        };
        if let Some(row) = row {
            let items = if g.ctx_tree { TREE_CTX } else { CTX_MENU };
            let has = items.get(row).map(|it| it.sub.is_some()).unwrap_or(false);
            let want = if has { Some(row) } else { None };
            if g.ctx_sub != want {
                g.ctx_sub = want;
            }
        } else if g.ctx_sub.is_some() && !over(&g.ctx_sub_rect, x, y) && !over(&g.ctx_rect, x, y) {
            g.ctx_sub = None;
        }
    }
    // 类别右键菜单没有子层，跟着鼠标重画就行
    if g.ctx_cat.is_some() {
        need = true;
    }
    // 右键弹层开着：指针是在弹层上走，底下那屏的悬停一律钉住。
    // 不然树行和格子会拿同一个坐标继续算悬停，看着就是菜单底下的东西跟着亮。
    if g.ctx_at.is_some() || g.ctx_cat.is_some() {
        g.hover = usize::MAX;
        g.hover_btn = None;
        g.hover_menu = None;
        drop(g);
        if let Ok(c) = LoadCursorW(None, IDC_ARROW) {
            SetCursor(Some(c));
        }
        if need {
            refresh(hwnd);
        }
        return;
    }
    if let Some((grab, top, span, overflow)) = g.tree_drag {
        let v = (y - grab - top).max(0) as i64;
        g.tscroll = (v * overflow as i64 / span.max(1) as i64).min(overflow as i64) as i32;
        drop(g);
        refresh(hwnd);
        return;
    }
    if let Some((grab, top, span, overflow)) = g.org_drag {
        let v = (y - grab - top).max(0) as i64;
        g.org_scroll = (v * overflow as i64 / span.max(1) as i64).min(overflow as i64) as i32;
        drop(g);
        refresh(hwnd);
        return;
    }
    if let Some((grab, top, span, overflow)) = g.conv_drag {
        let v = (y - grab - top).max(0) as i64;
        g.conv_scroll = (v * overflow as i64 / span.max(1) as i64).min(overflow as i64) as i32;
        drop(g);
        refresh(hwnd);
        return;
    }
    if let Some((sid, l, rr)) = g.conv_sdrag {
        drop(g);
        // 拖动一路走的是 conv_command 里那同一条换算，值只可能从一处来
        conv_command(hwnd, sid, x, &RECT { left: l, top: 0, right: rr, bottom: 0 });
        return;
    }
    if let Some((_cid, x0, y0)) = g.org_cat_drag {
        let tx = GetSystemMetrics(SM_CXDRAG).max(1);
        let ty = GetSystemMetrics(SM_CYDRAG).max(1);
        let past = (x - x0).abs() > tx || (y - y0).abs() > ty;
        // 落点只认面板里的类别行：停在行缝或面板外时清掉高亮，抬起什么也不做
        let to = if past && over(&z.org, x, y) {
            g.org_hits
                .iter()
                .find(|(_, r)| over(r, x, y))
                .map(|(i, _)| *i)
                .unwrap_or(-1)
        } else {
            -1
        };
        if to != g.org_cat_to {
            g.org_cat_to = to;
            drop(g);
            refresh(hwnd);
        }
        return;
    }
    if let Some((grab, top, span, overflow)) = g.grid_drag {
        let v = (y - grab - top).max(0) as i64;
        g.scroll_set((v * overflow as i64 / span.max(1) as i64).min(overflow as i64) as i32);
        g.queue_visible(z.grid);
        drop(g);
        refresh_grid(hwnd);
        return;
    }
    if let Some((i, x0, w0)) = g.col_drag {
        // 指针走的是物理像素，col_w 记的是逻辑像素，得除掉缩放再存。
        let d = ((x - x0) as f64 / g.scale).round() as i32;
        // 五列从左往右排，每条线都是它左边那一列的右缘：线往右拖就是加宽，
        // 五条线同一个方向，右边那几列只跟着挪位置，宽度不动。
        g.col_w[i] = (w0 + d).clamp(48, 1400);
        drop(g);
        refresh(hwnd);
        return;
    }
    if let Some((which, s0, sw)) = g.drag {
        if which == 0 {
            g.left_w = (sw + (x - s0)).clamp((110.0 * g.scale) as i32, rc.right / 2);
        } else if which == 1 {
            g.pv_h = (sw - (y - s0)).clamp((100.0 * g.scale) as i32, rc.bottom / 2);
        } else {
            // 抓住整理面板的左边往左拖就是加宽，往右拖收窄
            g.org_w = (sw + (s0 - x)).clamp((170.0 * g.scale) as i32, rc.right / 2);
        }
        need = true;
    } else {
        // 菜单开着时：鼠标扫过别的顶层项就换菜单，扫过带 ▸ 的行就展开子层
        if let Some(open) = g.open_menu {
            let mut mn = false;
            if let Some(which) = g.menus.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
                if which != open {
                    g.open_menu = Some(which);
                    g.open_sub = None;
                    g.pop_hover = None;
                    g.sub_hover = None;
                    mn = true;
                }
            }
            let in_sub = g.open_sub.is_some() && over(&g.sub_rect, x, y);
            let row = if in_sub {
                None
            } else {
                g.pop_rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i)
            };
            // 换行必须重画一次：高亮是按指针位置现算的，但"指针换行了"没人告诉窗口，
            // 只在展开子层时才刷的话，压到另一行会一直亮着旧行（看着慢半拍）。
            let sub_row = if in_sub {
                g.sub_rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i)
            } else {
                None
            };
            if row != g.pop_hover || sub_row != g.sub_hover {
                g.pop_hover = row;
                g.sub_hover = sub_row;
                mn = true;
            }
            if let Some(row) = row {
                let has = MENUS
                    .get(open)
                    .and_then(|(_, items)| items.get(row))
                    .map(|it| it.sub.is_some())
                    .unwrap_or(false);
                let want = if has { Some(row) } else { None };
                if g.open_sub != want {
                    g.open_sub = want;
                    // 子层行表要等下一帧才更新，旧的子层行号先作废。
                    g.sub_hover = None;
                    mn = true;
                }
            } else if g.open_sub.is_some() && !over(&g.sub_rect, x, y) && !over(&g.pop_rect, x, y) {
                g.open_sub = None;
                g.sub_hover = None;
                mn = true;
            }
            if mn {
                need = true;
            }
        }
        // 顶层菜单条的悬停：不进 a.btns，单独比对一次，进出某一项才重画。
        let hm = g.menus.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i);
        if hm != g.hover_menu {
            g.hover_menu = hm;
            need = true;
        }
        let hit = g.hit(z.grid, x, y);
        if hit != g.hover {
            g.hover = hit;
            need = true;
        }
        let hb = g
            .btns
            .iter()
            .chain(g.dlg_btns.iter())
            .chain(g.page_btns.iter())
            .find(|(_, r)| over(r, x, y))
            .map(|(i, _)| *i);
        if hb != g.hover_btn {
            g.hover_btn = hb;
            need = true;
        }
        if over(&z.org, x, y) || over(&z.spo, x, y) {
            need = true;
        }
        // 列头那条边界线也当"可左右拖"的地方：光标不换成箭头，没人知道能拖。
        let on_col_edge = {
            if g.mode != Mode::Detail {
                false
            } else {
                let hw = (3.0 * g.scale) as i32;
                let hd = detail_head(&z, &g.th);
                let edges = [
                    g.col_edge(z.grid, 0),
                    g.col_edge(z.grid, 1),
                    g.col_edge(z.grid, 2),
                    g.col_edge(z.grid, 3),
                    g.col_edge(z.grid, 4),
                ];
                over(&hd, x, y) && edges.iter().any(|&e| (x - e).abs() <= hw)
            }
        };
        let cur = if on_col_edge || over(&z.spv, x, y) || over(&z.spo, x, y) {
            LoadCursorW(None, IDC_SIZEWE).unwrap_or_default()
        } else if over(&z.sph, x, y) {
            LoadCursorW(None, IDC_SIZENS).unwrap_or_default()
        } else if over(&g.addr_tr, x, y) {
            LoadCursorW(None, IDC_IBEAM).unwrap_or_default()
        } else {
            LoadCursorW(None, IDC_ARROW).unwrap_or_default()
        };
        SetCursor(Some(cur));
    }
    drop(g);
    if need {
        refresh(hwnd);
    }
}

unsafe fn mouse_down(hwnd: HWND, lparam: LPARAM, keys: usize, dbl: bool) {
    let x = (lparam.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    // 修饰键按消息自带的标志位读：这一位是系统随消息一起送来的，比 GetKeyState 准，
    // 也不会被输入法或别处的按键状态带偏。
    let ctrl = keys & MK_CONTROL.0 as usize != 0;
    let shift = keys & MK_SHIFT.0 as usize != 0;
    // 正在原地改名时，这一类“点到别处”的一下就是改名到此为止：先落盘，再按平常的点击处理。
    // 点进编辑框里的那一下由编辑框自己接走，不会到这里。
    if app().lock().unwrap().ren_edit.is_some() {
        end_rename(hwnd, true);
    }
    let (z, _) = layout(hwnd);
    // 工具栏 ▾ 菜单开着：这一下先看落点，归菜单就吃掉
    if app().lock().unwrap().bmenu.is_some() && bmenu_click(hwnd, x, y) {
        return;
    }
    {
        let mut g = app().lock().unwrap();
        g.drag_seed = None;
        g.tree_seed = None;
        // 点到地址栏外面就把选区摘掉，跟正文里点一下取消选中同一个道理
        let inaddr = over(&g.addr_tr, x, y);
        if !inaddr && g.addr_sel.take().is_some() {
            drop(g);
            refresh(hwnd);
        }
    }
    // 对话框默认是模态的：只认它自己的按钮，点别处全部吃掉。
    // 以图搜图那一页例外——它开着的时候浮层外面那一下要归底下那屏，
    // 这样人能对着结果直接操作文件。页里头（连空白处）照旧只认它自己的控件。
    let sim_pass = sim_loose(&app().lock().unwrap(), x, y);
    if sim_pass {
        // 点在页外顺手把展开的下拉收掉，别留一个浮着的列表在底下那屏上。
        if let Some(d) = app().lock().unwrap().dlg.as_mut() {
            d.combo_open = false;
        }
    }
    if app().lock().unwrap().dlg.is_some() && !sim_pass {
        let (hits, rows) = {
            let g = app().lock().unwrap();
            (g.dlg_btns.clone(), g.dlg_rows.clone())
        };
        let mut hit = hits.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i);
        if hit.is_none() {
            hit = rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i);
        }
        let Some(id) = hit else {
            // 守卫先放下再还焦点：万一焦点此刻在某个原生输入框上，
            // SetFocus 会同步触发 EN_KILLFOCUS 回 WM_COMMAND，那里还要拿锁。
            let ed = app().lock().unwrap().dlg_edit;
            if let Some(ed) = ed {
                let _ = SetFocus(Some(ed));
            }
            return;
        };
        // 双击只有结果表那一行有用（那一下是"转到这张图所在的文件夹"），其余照旧吞掉。
        if dbl && !(id >= D_SIM_ROW && id < D_SIM_ROW + SIM_ROWS) {
            return;
        }
        let pick = |id: usize, lo: i32, hi: i32| -> Option<i32> {
            hits.iter().find(|(i, _)| *i == id).map(|(_, r)| {
                let t = ((x - r.left) as f64 / (r.right - r.left).max(1) as f64).clamp(0.0, 1.0);
                lo + (t * (hi - lo) as f64).round() as i32
            })
        };
        match id {
            _ if id == D_CLOSE || id == D_CANCEL => {
                if app().lock().unwrap().dlg.as_ref().map(|d| d.kind) == Some(DlgKind::FmtOpt) {
                    back_to_conv(hwnd);
                } else {
                    close_dlg(hwnd);
                }
            }
            D_OK | D_START => dlg_ok(hwnd),
            D_DBCHECK => {
                let go = {
                    let mut g = app().lock().unwrap();
                    let is_maint = g.dlg.as_ref().is_some_and(|d| d.kind == DlgKind::CatDb);
                    if is_maint && !g.db_busy {
                        g.db_maint = "正在检查数据库…".to_string();
                        true
                    } else {
                        false
                    }
                };
                if go {
                    start_db_task(hwnd, 0, None);
                    refresh(hwnd);
                }
            }
            D_DB_MOVE => {
                let to = browse_for_folder(hwnd, "给分类数据库挑个文件夹");
                let msg = match to {
                    Some(p) => move_db_dir(Path::new(&p)),
                    None => "没有选文件夹，位置没动。".to_string(),
                };
                let mut g = app().lock().unwrap();
                g.db_maint = msg;
                drop(g);
                refresh(hwnd);
            }
            D_DB_PRUNE => unsafe { start_db_prune(hwnd) },
            D_SCAN_ADD => {
                if let Some(d) = browse_for_folder(hwnd, "挑一个要扫的文件夹") {
                    let mut g = app().lock().unwrap();
                    if !g.scan_dirs.iter().any(|o| o.path.eq_ignore_ascii_case(&d)) {
                        g.scan_dirs.push(ScanRoot {
                            path: d,
                            last: 0,
                            dirs: Vec::new(),
                            exts: Vec::new(),
                        });
                        g.scan_sel = g.scan_dirs.len() - 1;
                        // 新加的在名单末尾，把视图滚到能看见它。
                        g.scan_scroll = g.scan_dirs.len().saturating_sub(SCAN_VIS);
                        save_scan_cfg(&g.scan_dirs);
                    }
                    drop(g);
                    refresh(hwnd);
                }
            }
            D_SCAN_RULES => unsafe { open_rule_dlg(hwnd) },
            D_RULE_SAVE => {
                // 框里的字先读出来，保存进名单，规则同步给指纹关口，然后把库里
                // 已入账、命中规则的指纹摘掉（后台跑，完事报数）。
                let texts: Vec<String> = {
                    let g = app().lock().unwrap();
                    g.rule_edits.iter().map(|ed| edit_text(*ed)).collect()
                };
                {
                    let mut g = app().lock().unwrap();
                    let sel = g.scan_sel;
                    if let Some(r) = g.scan_dirs.get_mut(sel) {
                        r.dirs = rule_words(Some(&texts.first().cloned().unwrap_or_default()));
                        r.exts = rule_words(Some(&texts.get(1).cloned().unwrap_or_default()));
                    }
                    save_scan_cfg(&g.scan_dirs);
                    sync_scan_rules(&g.scan_dirs);
                }
                drop_rule_edits();
                close_dlg(hwnd);
                let hp = hwnd.0 as isize;
                std::thread::spawn(move || {
                    let n = sim::purge_excluded();
                    {
                        let mut g = app().lock().unwrap();
                        g.msg = if n > 0 {
                            format!("规则已保存：摘掉 {n} 条命中规则的已入账指纹")
                        } else {
                            "规则已保存：库里没有命中规则的已入账指纹".to_string()
                        };
                    }
                    unsafe {
                        let _ = PostMessageW(
                            Some(HWND(hp as *mut core::ffi::c_void)),
                            WM_PV,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                });
                refresh(hwnd);
            }
            D_SCAN_DEL => remove_scan_sel(hwnd),
            D_SCAN_RUN => {
                let sel = app().lock().unwrap().scan_sel;
                unsafe { start_dir_scan(hwnd, Some(sel)) };
            }
            D_SCAN_ALL => unsafe { start_dir_scan(hwnd, None) },
            D_SCAN_STOP => {
                let mut g = app().lock().unwrap();
                g.scan_stop.store(true, Ordering::Relaxed);
                g.msg = "正在收尾…".to_string();
                drop(g);
                refresh(hwnd);
            }
            i if i >= D_SCAN_ROW && i < D_SCAN_ROW + SCAN_VIS => {
                let mut g = app().lock().unwrap();
                let maxs = g.scan_dirs.len().saturating_sub(SCAN_VIS);
                g.scan_sel = g.scan_scroll.min(maxs) + (i - D_SCAN_ROW);
                drop(g);
                refresh(hwnd);
            }
            D_THUMB_FIX => {
                {
                    let mut g = app().lock().unwrap();
                    if g.fix_mode {
                        g.end_rebuild();
                        g.restart_pool();
                        g.db_thumb = "重建已停止，没重做完的那些照旧从缓存出图。".to_string();
                        g.msg = g.db_thumb.clone();
                    } else {
                        let n = g.rebuild_thumbs();
                        if n == 0 {
                            g.db_thumb = "这一列里没有图片，没有要重做的缩略图。".to_string();
                            g.msg = g.db_thumb.clone();
                        } else {
                            g.db_thumb.clear();
                            g.msg = format!("正在核对缩略图：{} 张", n);
                        }
                    }
                }
                refresh(hwnd);
            }
            D_THUMB_ALL => {
                if app().lock().unwrap().thumb_busy {
                    app().lock().unwrap().thumb_stop.store(true, Ordering::Relaxed);
                    app().lock().unwrap().msg = "正在收尾…".to_string();
                } else {
                    start_thumb_all(hwnd);
                }
                refresh(hwnd);
            }
            D_CLEAR => {
                if let Some(ed) = app().lock().unwrap().dlg_edit {
                    set_edit_text(ed, "");
                }
                app().lock().unwrap().search_kw.clear();
                refresh(hwnd);
            }
            // ── 文件关联页 ──────────────────────────────────────────────
            // 勾选只改草稿，点「应用」才落注册表；草稿一动，上一次的结果就不再作数。
            i if i >= D_ASSOC && i < D_ASSOC + ASSOC_MAX => {
                let mut g = app().lock().unwrap();
                if let Some(it) = g.assoc.get_mut(i - D_ASSOC) {
                    it.1 = !it.1;
                    g.assoc_msg.clear();
                }
                drop(g);
                refresh(hwnd);
            }
            D_ASSOC_CHECK => {
                let mut g = app().lock().unwrap();
                g.assoc_check = !g.assoc_check;
                g.assoc_msg.clear();
                drop(g);
                refresh(hwnd);
            }
            D_ASSOC_APPLY => assoc_apply(hwnd),
            D_ASSOC_DEFAULTS => assoc_open_defaults(hwnd),
            D_BROWSE => {
                if let Some(dir) = browse_for_folder(hwnd, "选择输出文件夹") {
                    app().lock().unwrap().conv_out = dir.clone();
                    if let Some(ed) = app().lock().unwrap().dlg_edit {
                        set_edit_text(ed, &dir);
                    }
                    refresh(hwnd);
                }
            }
            D_QUAL => {
                let v = pick(D_QUAL, 1, 100).unwrap_or(100);
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.opt.quality = v.clamp(1, 100) as u8;
                }
                refresh(hwnd);
            }
            D_OPT_PNG => {
                let v = pick(D_OPT_PNG, 0, 9).unwrap_or(9);
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.opt.png_level = v.clamp(0, 9) as u8;
                }
                refresh(hwnd);
            }
            D_FMTSET => open_fmtopt(hwnd),
            D_OW => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.combo_open = !d.combo_open;
                }
                refresh(hwnd);
            }
            i if i >= D_OW_ITEM && i < D_OW_ITEM + 4 => {
                let mut g = app().lock().unwrap();
                g.conv_overwrite = (i - D_OW_ITEM) as u8;
                if let Some(d) = g.dlg.as_mut() {
                    d.combo_open = false;
                }
                drop(g);
                refresh(hwnd);
            }
            D_OPT_PROG | D_OPT_HUFF | D_OPT_SH | D_OPT_SV | D_DCF => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    match id {
                        D_OPT_PROG => d.opt.progressive = !d.opt.progressive,
                        D_OPT_HUFF => d.opt.optimize_huffman = !d.opt.optimize_huffman,
                        D_OPT_SH => d.opt.sample_h = !d.opt.sample_h,
                        D_OPT_SV => d.opt.sample_v = !d.opt.sample_v,
                        _ => d.opt.dcf = !d.opt.dcf,
                    }
                }
                refresh(hwnd);
            }
            i if i >= D_THUMB && i < D_THUMB + 3 => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.opt.thumb = (i - D_THUMB) as u8;
                }
                refresh(hwnd);
            }
            i if i >= D_OPT_TIFF && i < D_OPT_TIFF + 4 => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.opt.tiff_comp = (i - D_OPT_TIFF) as u8;
                }
                refresh(hwnd);
            }
            D_SAVEDEF => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.scope_db = !d.scope_db;
                }
                refresh(hwnd);
            }
            D_FMT_RESET => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.opt = encode::Opt::default();
                }
                refresh(hwnd);
            }
            D_RN_START_DN | D_RN_START_UP => {
                let mut g = app().lock().unwrap();
                let step = if id == D_RN_START_UP { 1 } else { -1 };
                g.rn_start = (g.rn_start + step).max(0);
                drop(g);
                refresh(hwnd);
            }
            i if i >= D_RN_CASE && i < D_RN_CASE + 3 => {
                app().lock().unwrap().rn_case = (i - D_RN_CASE) as u8;
                refresh(hwnd);
            }
            D_RN_HELP => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.combo_open = !d.combo_open;
                }
                refresh(hwnd);
            }
            D_DELSRC => {
                let mut g = app().lock().unwrap();
                g.conv_del = !g.conv_del;
                drop(g);
                refresh(hwnd);
            }
            D_SCOPE_CUR => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.scope_db = false;
                }
                refresh(hwnd);
            }
            D_SCOPE_DB => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.scope_db = true;
                }
                refresh(hwnd);
            }
            D_SKIN_DIR => {
                if let Some(dir) = browse_for_folder(hwnd, "选择皮肤文件夹") {
                    skin_dir_switch(hwnd, Some(PathBuf::from(dir)));
                }
            }
            D_SKIN_DIR_DEF => skin_dir_switch(hwnd, None),
            i if i >= D_SKIN_TAB && i < D_SKIN_TAB + 3 => {
                let mut had_edit = false;
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.fmt = i - D_SKIN_TAB;
                    had_edit = d.save_as;
                    d.save_as = false;
                    d.arm_del = false;
                }
                if had_edit {
                    drop_dlg_edit();
                }
                refresh(hwnd);
            }
            // ── 以图搜图那一页 ──────────────────────────────────────────
            // 这页的参数是"当场生效"的：改一项就动，点开始查找才扫，
            // 不套设置窗那种"点确定才落"的草稿语义。
            D_TITLEBAR => {
                // 记下按下这点离浮层左上角差多少，剩下的交给 mouse_move 平移。
                let off = app().lock().unwrap().dlg.as_ref().map(|d| (x - d.rect.left, y - d.rect.top));
                if let Some(off) = off {
                    app().lock().unwrap().dlg_drag = Some(off);
                    DRAGGING.store(true, Ordering::Relaxed);
                    let _ = SetCapture(hwnd);
                }
            }
            D_SIM_AUTO => {
                let mut g = app().lock().unwrap();
                g.sim_auto = !g.sim_auto;
                drop(g);
                refresh(hwnd);
            }
            D_SIM_PICK => {
                if let Some(p) = unsafe { pick_sim_image(hwnd) } {
                    let mut g = app().lock().unwrap();
                    g.sim_path = p;
                    g.sim_hits.clear();
                    g.sim_sel = 0;
                    g.sim_scroll = 0;
                    g.sim_ms = 0;
                    drop(g);
                    unsafe { sim_previews(hwnd) };
                }
            }
            D_SIM_CLIP => {
                // 剪贴板里那些图片文件的路径。直接贴在剪贴板上的位图（截图那种）
                // 这一版还没接：手上没有能落盘的临时目录口径，接了就是假的。
                let got = clip::clipboard_paths()
                    .0
                    .into_iter()
                    .find(|p| scan::is_image(p) && Path::new(p).is_file());
                match got {
                    Some(p) => {
                        let mut g = app().lock().unwrap();
                        g.sim_path = p;
                        g.sim_hits.clear();
                        g.sim_sel = 0;
                        g.sim_scroll = 0;
                        g.sim_ms = 0;
                        drop(g);
                        unsafe { sim_previews(hwnd) };
                    }
                    None => {
                        app().lock().unwrap().sim_lib_msg = "剪贴板里没有图片文件的路径".to_string();
                        refresh(hwnd);
                    }
                }
            }
            D_SIM_SLIDER => {
                if let Some(v) = pick(D_SIM_SLIDER, 50, 100) {
                    app().lock().unwrap().sim_pct = v.clamp(50, 100) as u32;
                    refresh(hwnd);
                }
            }
            D_SIM_ALGO => {
                if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                    d.combo_open = !d.combo_open;
                }
                refresh(hwnd);
            }
            i if i >= D_SIM_ALGO_ITEM && i < D_SIM_ALGO_ITEM + 3 => {
                let mut g = app().lock().unwrap();
                g.sim_algo = match i - D_SIM_ALGO_ITEM {
                    1 => sim::ALGO_D,
                    2 => sim::ALGO_C,
                    _ => sim::ALGO_ALL,
                };
                if let Some(d) = g.dlg.as_mut() {
                    d.combo_open = false;
                }
                drop(g);
                refresh(hwnd);
            }
            D_SIM_ROT | D_SIM_FLIP => {
                let mut g = app().lock().unwrap();
                if id == D_SIM_ROT {
                    g.sim_rot = !g.sim_rot;
                } else {
                    g.sim_flip = !g.sim_flip;
                }
                drop(g);
                refresh(hwnd);
            }
            D_SIM_GO => unsafe { start_sim_search(hwnd) },
            i if i >= D_SIM_ROW && i < D_SIM_ROW + SIM_ROWS => {
                let (n, rev) = {
                    let mut g = app().lock().unwrap();
                    let row = g.sim_scroll.max(0) as usize + (i - D_SIM_ROW);
                    let keep = row < g.sim_hits.len();
                    let rev = if keep && dbl { Some(g.sim_hits[row].path.clone()) } else { None };
                    if keep {
                        g.sim_sel = row;
                    }
                    (keep, rev)
                };
                if n {
                    if let Some(p) = rev {
                        // 双击这一行就是"带我到那张图的文件夹里去"，这一页不收。
                        reveal_in_view(hwnd, &p);
                    } else {
                        unsafe { sim_previews(hwnd) };
                    }
                }
            }
            D_FACE => {
                let (face, px) = {
                    let g = app().lock().unwrap();
                    (
                        g.dlg.as_ref().map(|d| d.look_draft.face().to_string()).unwrap_or_else(|| g.look.face().to_string()),
                        g.th.font,
                    )
                };
                if let Some(f) = pick_font(hwnd, &face, px) {
                    let mut g = app().lock().unwrap();
                    if let Some(d) = g.dlg.as_mut() {
                        d.look_draft.face = Some(f);
                    }
                    g.apply_draft(hwnd);
                }
                refresh(hwnd);
            }
            i if i >= D_ZONE && i < D_ZONE + theme::NZ * 3 => {
                let z = (i - D_ZONE) / 3;
                match (i - D_ZONE) % 3 {
                    ZR_DN => step_zone(hwnd, z, -1),
                    ZR_UP => step_zone(hwnd, z, 1),
                    _ => toggle_zone_bold(hwnd, z),
                }
            }
            i if i >= D_ZONEC && i < D_ZONEC + theme::NZ => {
                let zi = i - D_ZONEC;
                let key = theme::ZONES[zi].0.to_string();
                let cur = {
                    let g = app().lock().unwrap();
                    let base = theme::skin_of(&g.skin);
                    match g.dlg.as_ref() {
                        Some(d) => {
                            let sk = theme::preview(&base, &d.look_draft);
                            zone_now_ink(zi, &sk, d.look_draft.zone_color(&key))
                        }
                        None => base.slot(theme::ZONE_INK[zi]),
                    }
                };
                if let Some(c) = pick_color(hwnd, cur) {
                    let mut g = app().lock().unwrap();
                    if let Some(d) = g.dlg.as_mut() {
                        d.look_draft.zone_c.insert(key, c);
                    }
                    g.apply_draft(hwnd);
                }
                refresh(hwnd);
            }
            i if i >= D_COLOR && i < D_COLOR + COLOR_SLOTS.len() => {
                let slot = COLOR_SLOTS[i - D_COLOR].to_string();
                let cur = {
                    let g = app().lock().unwrap();
                    let base = theme::skin_of(&g.skin);
                    match g.dlg.as_ref() {
                        Some(d) => theme::preview(&base, &d.look_draft).slot(&slot),
                        None => base.slot(&slot),
                    }
                };
                if let Some(c) = pick_color(hwnd, cur) {
                    let mut g = app().lock().unwrap();
                    if let Some(d) = g.dlg.as_mut() {
                        d.look_draft.colors.insert(slot, c);
                    }
                    g.apply_draft(hwnd);
                }
                refresh(hwnd);
            }
            D_RESET_LOOK => {
                // 草稿倒回"本皮肤默认"，界面立刻跟着退回去；盘上那层等最后那一句再决定。
                let mut g = app().lock().unwrap();
                match g.dlg.as_mut() {
                    Some(d) => d.look_draft = theme::Look::default(),
                    None => {
                        g.look.reset();
                        g.look.save(&g.skin);
                        g.restyle(hwnd);
                    }
                }
                g.apply_draft(hwnd);
                drop(g);
                refresh(hwnd);
            }
            D_LOOK_OK => dlg_ok(hwnd),
            D_SKIN_ASAVE => {
                let editing = app().lock().unwrap().dlg.as_ref().map(|d| d.save_as).unwrap_or(false);
                if editing {
                    skin_save_as(hwnd);
                } else {
                    if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                        d.save_as = true;
                    }
                    create_dlg_edit(hwnd, DlgKind::Skin);
                }
                refresh(hwnd);
            }
            D_SKIN_DEL => {
                // 删除这种不可逆的，按第一次只武装、按钮变成"确认删除皮肤"，再按才真删。
                let armed = app().lock().unwrap().dlg.as_ref().map(|d| d.arm_del).unwrap_or(false);
                if armed {
                    if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                        d.arm_del = false;
                    }
                    skin_delete(hwnd);
                } else {
                    if let Some(d) = app().lock().unwrap().dlg.as_mut() {
                        d.arm_del = true;
                    }
                }
                refresh(hwnd);
            }
            i if i >= D_FMT && i < D_FMT + 16 => {
                let f = i - D_FMT;
                if f < encode::FORMATS.len() {
                    let mut g = app().lock().unwrap();
                    if let Some(d) = g.dlg.as_mut() {
                        d.fmt = f;
                    }
                    g.conv_fmt = f;
                }
                refresh(hwnd);
            }
            _ => {}
        }
        return;
    }
    // 类别右键菜单：命中就执行，点在外面只关菜单、不让这次点击落到底下
    if app().lock().unwrap().ctx_cat.is_some() {
        let rows = app().lock().unwrap().ctx_cat_rows.clone();
        let mut picked: Option<usize> = None;
        if let Some(pi) = rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
            if let Some(it) = CAT_CTX.get(pi) {
                let off = it.off || (it.id == M_CAT_PASTE && cat_clip_empty());
                if !off {
                    picked = Some(it.id);
                }
            }
        }
        app().lock().unwrap().ctx_cat = None;
        if let Some(id) = picked {
            if !dbl {
                command(hwnd, id);
            }
        }
        refresh(hwnd);
        return;
    }
    // 右键弹层开着时先处理它：点菜单外只关菜单，不把这次点击传给下面的格子
    if app().lock().unwrap().ctx_at.is_some() {
        let (rows, sub_rows, sub, items) = {
            let g = app().lock().unwrap();
            (
                g.ctx_rows.clone(),
                g.ctx_sub_rows.clone(),
                g.ctx_sub,
                if g.ctx_tree { TREE_CTX } else { CTX_MENU },
            )
        };
        let mut picked: Option<usize> = None;
        if let Some(si) = sub_rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
            picked = sub
                .and_then(|p| items.get(p))
                .and_then(|it| it.sub)
                .and_then(|sub_items| sub_items.get(si))
                .map(|it| it.id);
        } else if let Some(pi) = rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
            if let Some(it) = items.get(pi) {
                if it.sub.is_some() {
                    app().lock().unwrap().ctx_sub = Some(pi);
                    refresh(hwnd);
                    return;
                }
                if !it.off {
                    let off = {
                        let g = app().lock().unwrap();
                        tree_ctx_off(&g, it.id) || grid_ctx_off(&g, it.id)
                    };
                    if !off {
                        picked = Some(it.id);
                    }
                }
            }
        }
        {
            let mut g = app().lock().unwrap();
            g.ctx_at = None;
            g.ctx_sub = None;
        }
        if let Some(id) = picked {
            if id != 0 && !dbl {
                // 树右键的落点得活到命令执行完：剪切/复制/删除/属性/转到文件都要读它。
                command(hwnd, id);
            }
        }
        // 命令已经拿到落点，这里再清，免得随后的键盘命令又撞上刚才那次树右键。
        app().lock().unwrap().ctx_dir = None;
        app().lock().unwrap().ctx_img = None;
        app().lock().unwrap().ctx_blank = true;
        refresh(hwnd);
        return;
    }
    // 菜单条和弹层压在工具条之上：先判它们，命中就吃掉这次点击
    {
        let (menus, pop_rows, sub_rows, open_menu, open_sub) = {
            let g = app().lock().unwrap();
            (g.menus.clone(), g.pop_rows.clone(), g.sub_rows.clone(), g.open_menu, g.open_sub)
        };
        if let Some(open) = open_menu {
            let items = MENUS.get(open).map(|(_, it)| *it).unwrap_or(&[]);
            if let Some(si) = sub_rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
                let id = open_sub
                    .and_then(|p| items.get(p))
                    .and_then(|it| it.sub)
                    .and_then(|sub| sub.get(si))
                    .map(|it| it.id)
                    .unwrap_or(0);
                {
                    let mut g = app().lock().unwrap();
                    g.open_menu = None;
                    g.open_sub = None;
                }
                if id != 0 && !dbl {
                    command(hwnd, id);
                }
                refresh(hwnd);
                return;
            }
            if let Some(pi) = pop_rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
                if let Some(it) = items.get(pi) {
                    if it.sub.is_some() {
                        // 子层行表下一帧才画出来，旧的 sub_hover 先清掉，
                        // 不然指针移进新子层第一行也会被当成没换行。
                        let mut g = app().lock().unwrap();
                        g.open_sub = Some(pi);
                        g.sub_hover = None;
                        drop(g);
                        refresh(hwnd);
                        return;
                    }
                    let (id, off) = (it.id, it.off);
                    {
                        let mut g = app().lock().unwrap();
                        g.open_menu = None;
                        g.open_sub = None;
                    }
                    if id != 0 && !off && !dbl {
                        command(hwnd, id);
                    }
                    refresh(hwnd);
                    return;
                }
            }
        }
        if let Some(which) = menus.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
            let mut g = app().lock().unwrap();
            g.open_menu = if g.open_menu == Some(which) { None } else { Some(which) };
            g.open_sub = None;
            // 新菜单的行表还没画出来，上次留下的行号不能用来判"换行"。
            g.pop_hover = None;
            g.sub_hover = None;
            drop(g);
            refresh(hwnd);
            return;
        }
        if open_menu.is_some() {
            let mut g = app().lock().unwrap();
            g.open_menu = None;
            g.open_sub = None;
            drop(g);
            refresh(hwnd);
            return;
        }
    }
    let pbs: Vec<(usize, RECT)> = app().lock().unwrap().page_btns.clone();
    for (id, r) in pbs {
        if over(&r, x, y) {
            if !dbl {
                command(hwnd, id);
            }
            return;
        }
    }
    let clicks: Vec<(usize, RECT)> = app().lock().unwrap().btns.clone();
    for (id, r) in clicks {
        if over(&r, x, y) {
            // 右栏批量转换页上的控件先走自己那一套（页签、滑条、开始/停止…）
            if conv_command(hwnd, id, x, &r) {
                return;
            }
            // 第二下会连着发一个 DBLCLK，不挡掉就是同一个命令跑两遍（看整夹会开出两个窗口）。
            if !dbl {
                command(hwnd, id);
            }
            return;
        }
    }
    {
        let (tr, n, full, xs) = {
            let g = app().lock().unwrap();
            (g.addr_tr, g.addr_n, g.addr_full, g.addr_xs.clone())
        };
        if n > 0 && over(&tr, x, y) {
            let mut g = app().lock().unwrap();
            if dbl || !full {
                // 省略号态中间画的是别的内容，逐字偏移没有意义；双击也一样，整条给他
                g.addr_sel = Some((0, n));
                g.addr_anchor = None;
            } else {
                let c = addr_caret(&xs, x);
                g.addr_anchor = Some(c);
                g.addr_sel = Some((c, c));
            }
            drop(g);
            SetCapture(hwnd);
            refresh(hwnd);
            return;
        }
    }
    {
        let mut g = app().lock().unwrap();
        if over(&z.spv, x, y) {
            g.drag = Some((0, x, g.left_w));
            SetCapture(hwnd);
            return;
        }
        if over(&z.spo, x, y) {
            g.drag = Some((2, x, g.org_w));
            // 人手拖过栏宽之后，开关转换页就不再自动改宽度了。
            g.conv_w_auto = false;
            SetCapture(hwnd);
            return;
        }
        if over(&z.sph, x, y) {
            if g.pv_h == 0 {
                g.pv_h = z.pv.bottom - z.pv.top;
            }
            g.drag = Some((1, y, g.pv_h));
            SetCapture(hwnd);
            return;
        }
    }
    if over(&z.org, x, y) {
        // 转换页和整理页共用这一块：当前画的是转换页，就只认它自己的滚动条，
        // 其余点击（页签/滑条/按钮）已经在通用按钮表里消化掉了。
        let conv_page = {
            let g = app().lock().unwrap();
            g.conv_open && (g.dock_tab == 1 || !g.org_open)
        };
        if conv_page {
            let mut g = app().lock().unwrap();
            let k = g.scale;
            let head_bottom = z.org.top + org_title_h(&g.th);
            let foot_h = (72.0 * k) as i32;
            let body = RECT {
                left: z.org.left,
                top: head_bottom,
                right: z.org.right,
                bottom: (z.org.bottom - foot_h).max(head_bottom),
            };
            let h = (body.bottom - body.top).max(1);
            let cap = g.conv_max_scroll.max(0);
            let content = (h + cap).max(1);
            if let Some((track, thumb, _)) =
                tree_sb(body, body.top, content as usize, 1, g.conv_scroll, k)
            {
                if x >= track.left {
                    let span = (track.bottom - track.top - (thumb.bottom - thumb.top)).max(1);
                    if y >= thumb.top && y < thumb.bottom {
                        g.conv_drag = Some((y - thumb.top, track.top, span, cap));
                        drop(g);
                        SetCapture(hwnd);
                        return;
                    }
                    let want = ((y - track.top - (thumb.bottom - thumb.top) / 2).max(0) as i64
                        * cap as i64
                        / span as i64) as i32;
                    g.conv_scroll = want.clamp(0, cap);
                    drop(g);
                    refresh(hwnd);
                    return;
                }
            }
            drop(g);
            return;
        }
        let rows = {
            let g = app().lock().unwrap();
            org_rows(&g)
        };
        let (top, rh, k, scroll) = {
            let g = app().lock().unwrap();
            (
                z.org.top + org_head_h(&g.th),
                g.th.row.max(1),
                g.scale,
                g.org_scroll,
            )
        };
        let h = (z.org.bottom - top).max(0);
        let maxs = (rows.len() as i32 * rh - h).max(0);
        let sb = tree_sb(
            RECT { left: z.org.left, top, right: z.org.right, bottom: z.org.bottom },
            top,
            rows.len(),
            rh,
            scroll,
            k,
        );
        if let Some((track, thumb, _)) = &sb {
            if x >= track.left {
                let hh = track.bottom - track.top;
                let tb = thumb.bottom - thumb.top;
                let span = (hh - tb).max(1);
                let cap = maxs.max(1);
                let mut g = app().lock().unwrap();
                if y >= thumb.top && y < thumb.bottom {
                    g.org_drag = Some((y - thumb.top, track.top, span, cap));
                    SetCapture(hwnd);
                    return;
                }
                let want = ((y - track.top - tb / 2).max(0) as i64 * cap as i64 / span as i64) as i32;
                g.org_scroll = want.clamp(0, cap);
                drop(g);
                refresh(hwnd);
                return;
            }
        }
        // 组头的“上移 / 下移”（新建类别在右键菜单里）
        let (upr, dnr, sel) = {
            let g = app().lock().unwrap();
            (g.org_up, g.org_dn, g.org_sel)
        };
        if over(&upr, x, y) || over(&dnr, x, y) {
            if !dbl {
                let delta = if over(&upr, x, y) { -1 } else { 1 };
                if sel == 0 {
                    app().lock().unwrap().msg = "先选一个类别再调顺序".to_string();
                } else {
                    let outcome = {
                        let mut g = app().lock().unwrap();
                        let moved = match g.cat.as_mut() {
                            Some(cat) => cat.move_cat(sel, delta),
                            None => false,
                        };
                        if moved {
                            g.cat.as_ref().map(|cat| cat.save_full(&catalog_file()))
                        } else {
                            None
                        }
                    };
                    let mut g = app().lock().unwrap();
                    g.msg = match outcome {
                        Some(Ok(())) => String::new(),
                        Some(Err(e)) => format!("顺序已改，但保存失败：{e}"),
                        None => "已经到头了".to_string(),
                    };
                }
                refresh(hwnd);
            }
            return;
        }
        let hits = app().lock().unwrap().org_hits.clone();
        if let Some(cid) = hits.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
            let Some(row) = rows.iter().find(|r| r.id == cid).cloned() else { return };
            // 勾选框：加入/移出当前选中的图片，不切换当前类别
            let chk = app()
                .lock()
                .unwrap()
                .org_checks
                .iter()
                .find(|(ri, _)| *ri == cid)
                .map(|(_, r)| *r);
            if let Some(cr) = chk {
                if over(&cr, x, y) {
                    if !dbl {
                        toggle_cat_for_selection(hwnd, row.id);
                    }
                    return;
                }
            }
            let ind = (13.0 * k).round() as i32;
            let arrow_w = (12.0 * k).round() as i32;
            let ax = z.org.left + (6.0 * k) as i32 + row.depth * ind;
            let on_arrow = row.kids && x < ax + arrow_w + (2.0 * k) as i32;
            if on_arrow {
                // 双击的第二次消息不再折一次，否则箭头双击等于没动
                if !dbl {
                    let mut g = app().lock().unwrap();
                    if !g.org_open_ids.remove(&row.id) {
                        g.org_open_ids.insert(row.id);
                    }
                }
                refresh(hwnd);
                return;
            }
            if dbl {
                // 双击类别行：展开/收起子类别
                if row.kids {
                    let mut g = app().lock().unwrap();
                    if !g.org_open_ids.remove(&row.id) {
                        g.org_open_ids.insert(row.id);
                    }
                }
                refresh(hwnd);
                return;
            }
            {
                let mut g = app().lock().unwrap();
                if row.id != 0 {
                    // 按下先记下起拖点，拖过系统阈值才算改父级；单击仍然只是切视图
                    g.org_cat_drag = Some((row.id, x, y));
                    g.org_cat_to = -1;
                }
                g.load_category(row.id, hwnd);
            }
            if row.id != 0 {
                SetCapture(hwnd);
            }
            refresh(hwnd);
        }
        return;
    }
    if over(&z.tree, x, y) {
        let mut rows = Vec::new();
        let (rh, head, tscroll, left, col, tw, sb, sc) = {
            let g = app().lock().unwrap();
            flat_tree(&g.tree, 0, &mut rows);
            let k = g.scale;
            let hr = g.th.row.max(1);
            let hb = z.tree.top + g.th.pane_head_h;
            (
                hr,
                hb,
                g.tscroll,
                z.tree.left + (4.0 * k) as i32,
                (20.0 * k).round() as i32,
                (11.0 * k).round() as i32,
                tree_sb(z.tree, hb, rows.len(), hr, g.tscroll, k),
                k,
            )
        };
        // 滚动条先接住：按住滑块就是拖动，点在槽的空白处翻一页
        if let Some((track, thumb, _)) = &sb {
            if x >= track.left {
                let h = track.bottom - track.top;
                let tb = thumb.bottom - thumb.top;
                let span = (h - tb).max(1);
                let cap = (rows.len() as i32 * rh - h).max(1);
                if y >= thumb.top && y < thumb.bottom {
                    app().lock().unwrap().tree_drag = Some((y - thumb.top, track.top, span, cap));
                    SetCapture(hwnd);
                    return;
                }
                let mut g = app().lock().unwrap();
                let want = ((y - track.top - tb / 2).max(0) as i64 * cap as i64 / span as i64) as i32;
                g.tscroll = want.clamp(0, cap);
                drop(g);
                refresh(hwnd);
                return;
            }
        }
        let k = ((y - head + tscroll) / rh).max(0) as usize;
        let Some((d, path, _, _)) = rows.get(k).cloned() else {
            return;
        };
        let home = app().lock().unwrap().home.clone();
        let mark_x = left + (d as i32) * col;
        let arrow = x >= mark_x && x < mark_x + tw + 4 && path != home;
        if arrow {
            // 箭头单击开关；双击的第二次消息不再折一次，否则等于没动
            if !dbl {
                app().lock().unwrap().tree_kb = true;
                tree_toggle_keep(hwnd, k, k as i32 * rh - tscroll);
            }
            return;
        }
        // 名字区从行首图标之后起算，右缘让开滚动条；和 paint_tree 的排字同一套几何。
        // 改名只对普通文件夹开放：此电脑、盘符、用户主目录不在其列。
        let pt = path.trim_end_matches('\\');
        let ren_ok = app().lock().unwrap().tree_editable(&path);
        let isz = (14.0 * sc).round() as i32;
        let sbw = sb.as_ref().map(|(_, _, w)| *w).unwrap_or(0);
        let row_y = head + (k as i32) * rh - tscroll;
        let nr = RECT {
            left: left + (d as i32) * col + tw + (3.0 * sc) as i32 + isz + (3.0 * sc) as i32,
            top: row_y,
            right: z.tree.right - 2 - sbw,
            bottom: row_y + rh,
        };
        {
            let mut g = app().lock().unwrap();
            g.tree_kb = true;
            if !dbl {
                // 记下起点：拖过阈值才把这一行当文件拖出去，普通单击不受影响
                g.tree_seed = Some((x, y, path.clone()));
            }
            if path == "此电脑" {
                if dbl {
                    drop(g);
                    tree_toggle_keep(hwnd, k, k as i32 * rh - tscroll);
                } else {
                    g.tsel = path.clone();
                    drop(g);
                    refresh(hwnd);
                }
                return;
            }
            // 树名的两连击按固定节拍分档，不顺系统双击阈值走：500 毫秒以内
            // 是双击（开合节点），500~1000 毫秒是改名，再慢就只当重新选中。
            // 系统把阈值调宽时，500~1000 的那两下也会被送成双击消息，所以
            // 收到双击也要按时刻重判一次。
            const OPEN_MS: u128 = 500;
            const RENAME_MS: u128 = 1000;
            if ren_ok && over(&nr, x, y) && !ctrl && !shift {
                let now = Instant::now();
                if let Some((p, t)) = g.ren_arm_t.take() {
                    let ms = now.duration_since(t).as_millis();
                    if p.eq_ignore_ascii_case(pt) && ms >= OPEN_MS && ms < RENAME_MS {
                        drop(g);
                        begin_rename_tree(hwnd, pt, nr);
                        return;
                    }
                    // 不是改名：普通单击就重新备记；双击交给下面开合。
                    if !dbl {
                        g.ren_arm_t = Some((pt.to_string(), now));
                    }
                } else if !dbl {
                    g.ren_arm_t = Some((pt.to_string(), now));
                }
            }
            if dbl {
                drop(g);
                tree_toggle_keep(hwnd, k, k as i32 * rh - tscroll);
                return;
            }
            // 单击只切当前目录，不展开树节点（sync_tree 看到 tree_noscroll 会跳过展开）
            g.tree_noscroll = true;
            g.goto(&path, hwnd, true);
        }
        request_pv(hwnd);
        refresh(hwnd);
        return;
    }
    if over(&z.gsb, x, y) {
        let mut g = app().lock().unwrap();
        let maxs = g.max_scroll(z.grid);
        let k = g.scale;
        if let Some((track, thumb, _)) = grid_sb(z.gsb, maxs, g.scroll, k) {
            let h = track.bottom - track.top;
            let tb = thumb.bottom - thumb.top;
            let span = (h - tb).max(1);
            if y >= thumb.top && y < thumb.bottom {
                g.grid_drag = Some((y - thumb.top, track.top, span, maxs));
                SetCapture(hwnd);
                return;
            }
            // 点槽空白：把滑块中心移到这里
            let want = ((y - track.top - tb / 2).max(0) as i64 * maxs as i64 / span as i64) as i32;
            g.scroll_set(want.clamp(0, maxs));
            g.queue_visible(z.grid);
        }
        drop(g);
        refresh_grid(hwnd);
        return;
    }
    // 详细资料档的列头。这一条不在 z.grid 里（布局时从列表顶上扣出去了），所以判在
    // grid 前面。两种点法：压着列边界按下去是拖列宽，点在列里是按那一列排序
    // （再点同一列换正序/倒序）。
    {
        let mut g = app().lock().unwrap();
        if g.mode == Mode::Detail {
            let h = detail_head(&z, &g.th);
            if over(&h, x, y) {
                let hw = (3.0 * g.scale) as i32;
                let c0 = g.cols(z.grid);
                let edges = [c0.typ, c0.dim, c0.date, c0.size, c0.end];
                if let Some(i) = edges.iter().position(|&e| (x - e).abs() <= hw) {
                    // 名称、大小两列没调过时记的是 0（意思是"铺满剩下的"）。按下那一刻先把
                    // 眼前这两格定格成具体宽，手一拖只动目标列，别列的宽度一个像素都不变。
                    let k = g.scale;
                    let lg = move |px: i32| ((px as f64 / k).round() as i32).clamp(48, 1400);
                    if g.col_w[0] == 0 {
                        g.col_w[0] = lg(c0.typ - c0.name);
                    }
                    if g.col_w[4] == 0 {
                        g.col_w[4] = lg(c0.right - c0.size);
                    }
                    g.col_drag = Some((i, x, g.col_w[i]));
                    SetCapture(hwnd);
                    drop(g);
                    refresh(hwnd);
                    return;
                }
                let cx = g.cols(z.grid);
                // 五列拖定后总宽可能比可见区窄，大小列右边会空出一截：落在空白里不排序。
                if x >= cx.end {
                    drop(g);
                    return;
                }
                let key = if x < cx.typ {
                    SortKey::Name
                } else if x < cx.dim {
                    SortKey::Type
                } else if x < cx.date {
                    SortKey::Image
                } else if x < cx.size {
                    SortKey::Date
                } else {
                    SortKey::Size
                };
                if g.sort == key {
                    g.sort_desc = !g.sort_desc;
                } else {
                    g.sort = key;
                    g.sort_desc = false;
                }
                g.apply_sort();
                // 点列头排序不走 command()，这一趟得自己记一笔
                ui_save(&g, hwnd);
                drop(g);
                done_view(hwnd);
                request_pv(hwnd);
                return;
            }
        }
    }
    if over(&z.grid, x, y) {
        let mut g = app().lock().unwrap();
        // 点进格子，键盘焦点就从树上收回来：方向键开始走文件。
        g.tree_kb = false;
        let hit = g.hit(z.grid, x, y);
        if hit == usize::MAX {
            // 空白处按下：清掉旧选区（按住 Ctrl 就先留着，抬起时并进框选），
            // 然后拖出选择框。没拖动就抬起，等于在空白处点了一下。
            if ctrl {
                g.band_add = g.marked.clone();
            } else {
                g.marked.clear();
                g.sel = usize::MAX;
                g.anchor = usize::MAX;
                g.sel_lit = false;
                g.band_add.clear();
                g.stamp += 1;
            }
            g.band = Some((x, y + g.scroll, x, y + g.scroll));
            g.ren_arm = None;
            drop(g);
            SetCapture(hwnd);
            refresh(hwnd);
            return;
        }
        let is_dir = g.entries[hit].is_dir;
        let path = g.entries[hit].path.clone();
        let on_name = over(&g.name_rect(hit, z.grid), x, y);
        let armed = g.ren_arm.take();
        // 改名只认这一个手势：两下都落在文件名那一格上，中间隔开。
        // 隔没隔开不用自己数：按得快，系统会把这一下送成双击消息，上面先走"打开"了。
        let slow = !ctrl && !shift && on_name && armed.as_ref().is_some_and(|p| p == &path);
        if on_name && !ctrl && !shift && !dbl {
            g.ren_arm = Some(path.clone());
        }
        if dbl && is_dir {
            drop(g);
            app().lock().unwrap().goto(&path, hwnd, true);
            request_pv(hwnd);
            refresh(hwnd);
            return;
        }
        if dbl {
            let p = PathBuf::from(&path);
            drop(g);
            spawn_viewer(&p, hwnd);
            return;
        }
        // 文件夹的名字格同样这一下也改名，资源管理器里文件和文件夹是一个手势。
        if slow {
            g.ren_arm = None;
            drop(g);
            begin_rename(hwnd, hit);
            return;
        }
        let pickable = g.pickable(hit);
        if shift {
            // Shift 点选：从锚点量到这一项，两头都算，中间连文件夹一起；Ctrl+Shift 是再并一段。
            let a = if g.anchor == usize::MAX { g.sel } else { g.anchor };
            if a != usize::MAX {
                let (a, b) = (a.min(hit), a.max(hit));
                // b 夹到表尾：锚点理论上已随视图重置，这里再兜一道底。
                let add: Vec<String> = (a..=b.min(g.entries.len() - 1))
                    .filter(|&j| g.pickable(j))
                    .map(|j| g.entries[j].path.clone())
                    .collect();
                if ctrl {
                    g.marked.extend(add);
                } else {
                    g.marked = add.into_iter().collect();
                }
            } else if !ctrl {
                g.marked.clear();
                if pickable {
                    g.marked.insert(path.clone());
                }
            }
            g.set_sel(hit);
        } else if ctrl {
            // Ctrl 点选：只切这一项，别处不动。点掉最后一项后光标还在，但不画成选中。
            if pickable {
                if !g.marked.remove(&path) {
                    g.marked.insert(path.clone());
                }
                g.sel_lit = g.marked.contains(&path);
            }
            g.sel = hit;
            g.anchor = hit;
            g.stamp += 1;
        } else if pickable && g.marked.contains(&path) {
            // 资源管理器规矩：按在已经选中的一项上，整组选区保留，只把光标挪过来——
            // 不然"拖一组文件进文件夹"这一下就被压成一项，看着就像只能拖走一个。
            g.sel = hit;
            g.sel_lit = true;
            g.anchor = hit;
        } else {
            // 不带修饰键的单击 = 从头只选这一个。虚拟的 `..` 只挪光标，进不了选区。
            g.marked.clear();
            if pickable {
                g.marked.insert(path.clone());
            }
            g.anchor = hit;
            g.set_sel(hit);
        }
        g.keep_visible(z.grid);
        g.drag_seed = Some((x, y, hit));
        g.trace(&format!("pick{}{}", if ctrl { 'c' } else { '-' }, if shift { 's' } else { '-' }));
        // 版式实测行：把命中项和格子的算法参数一起落一行，外面要核对"这点落在第几格"就够了。
        let (cw, ch) = g.cell();
        let nr = g.name_rect(hit, z.grid);
        g.trace(&format!(
            "geo{}|{},{},{},{}|{}x{}|{}|{}|{}|{}|{}|{},{}|d{}",
            hit,
            z.grid.left,
            z.grid.top,
            z.grid.right,
            z.grid.bottom,
            cw,
            ch,
            g.cols_n(z.grid.right - z.grid.left),
            g.row_h(),
            g.grid_pad(),
            g.grid_x0(z.grid),
            g.scroll,
            nr.top,
            nr.bottom,
            is_dir as u8
        ));
        drop(g);
        request_pv(hwnd);
        refresh(hwnd);
    }
}

impl App {
    /// 把当前目录所在的盘和整条路径链在树里展开、选中，并加载当前目录的子文件夹。
    fn sync_tree(&mut self) {
        let mut d = self.dir.display().to_string();
        if d.len() > 3 && d.ends_with('\\') {
            d.pop();
        }
        let want = format!("{d}\\");
        self.tsel = want;
        // 树里点出来的那一下：不替用户展开节点。滚动位置任何时候都不由程序动。
        let keep = self.tree_noscroll;
        self.tree_noscroll = false;
        if keep || d.len() < 2 {
            return;
        }
        // 桌面在树顶单独占一行。从桌面或它下面的子目录进来时只走这一支，不再从
        // 「此电脑」把 C:\ → Users → lz → Desktop 整条链展开，树里凭空多出一长串上级。
        if let Some(i) = self.tree.iter().position(|n| {
            let dp = n.path.trim_end_matches('\\');
            d.eq_ignore_ascii_case(dp)
                || (d.len() > dp.len()
                    && d.get(..dp.len()).is_some_and(|pre| pre.eq_ignore_ascii_case(dp))
                    && d.get(dp.len()..).is_some_and(|rest| rest.starts_with('\\')))
        }) {
            let desk = &mut self.tree[i];
            let base = desk.path.clone();
            let dp = base.trim_end_matches('\\').len();
            desk.open = true;
            let mut cur = base;
            let mut nodes = &mut desk.kids;
            let rest = d.get(dp..).unwrap_or("").trim_start_matches('\\');
            for part in rest.split('\\').filter(|p| !p.is_empty()) {
                cur = format!("{cur}{part}\\");
                let Some(n) = nodes.iter_mut().find(|n| n.path.eq_ignore_ascii_case(&cur)) else {
                    break;
                };
                if !n.loaded {
                    n.kids = sub_dirs(&n.path);
                    n.loaded = true;
                }
                n.open = true;
                nodes = &mut n.kids;
            }
            return;
        }
        let drive = format!("{}\\", &d[..2]);
        let Some(pc) = self.tree.iter_mut().find(|n| n.name == "此电脑") else {
            return;
        };
        let Some(disk) = pc.kids.iter_mut().find(|n| n.path.eq_ignore_ascii_case(&drive)) else {
            return;
        };
        if !disk.loaded {
            disk.kids = sub_dirs(&disk.path);
            disk.loaded = true;
        }
        disk.open = true;
        let mut cur = disk.path.clone();
        let mut nodes = &mut disk.kids;
        let rest = if d.len() > 3 { &d[3..] } else { "" };
        for part in rest.split('\\').filter(|p| !p.is_empty()) {
            cur = format!("{cur}{part}\\");
            let Some(n) = nodes.iter_mut().find(|n| n.path.eq_ignore_ascii_case(&cur)) else {
                break;
            };
            if !n.loaded {
                n.kids = sub_dirs(&n.path);
                n.loaded = true;
            }
            n.open = true;
            nodes = &mut n.kids;
        }
    }
}

/// Ctrl 组合键交给加速键表，不放在 key() 里：中文输入法激活时 WM_KEYDOWN 会被输入法吃掉，
/// 而加速键在消息循环里判断，早于窗口过程，所以照常生效。
/// 带 Shift 的必须排在前面——加速键按顺序匹配，没写出的修饰键算"不关心"。
unsafe fn accelerators() -> HACCEL {
    let c = FVIRTKEY | FCONTROL;
    let cs = FVIRTKEY | FCONTROL | FSHIFT;
    let table = [
        ACCEL { fVirt: cs, key: 0x41, cmd: M_NONE as u16 },
        ACCEL { fVirt: cs, key: 0x43, cmd: M_COPYIMG as u16 },
        ACCEL { fVirt: cs, key: 0x4F, cmd: M_ORG as u16 },
        ACCEL { fVirt: cs, key: 0x45, cmd: M_PANE_PV as u16 },
        ACCEL { fVirt: cs, key: 0x46, cmd: M_PANE_TREE as u16 },
        ACCEL { fVirt: cs, key: 0x4E, cmd: M_NEWDIR as u16 },
        ACCEL { fVirt: cs, key: 0x56, cmd: M_VIEWER as u16 },
        ACCEL { fVirt: c, key: 0x41, cmd: M_ALL as u16 },
        ACCEL { fVirt: c, key: 0x42, cmd: M_CONV_PANEL as u16 },
        ACCEL { fVirt: c, key: 0x43, cmd: M_COPYF as u16 },
        ACCEL { fVirt: c, key: 0x58, cmd: M_CUT as u16 },
        ACCEL { fVirt: c, key: 0x56, cmd: M_PASTE as u16 },
        ACCEL { fVirt: c, key: 0x45, cmd: M_CONV as u16 },
        ACCEL { fVirt: c, key: 0x46, cmd: M_FIND as u16 },
        ACCEL { fVirt: c, key: 0x4E, cmd: M_NEWWIN as u16 },
        ACCEL { fVirt: c, key: 0x57, cmd: M_CLOSE as u16 },
    ];
    CreateAcceleratorTableW(&table).unwrap_or(HACCEL(std::ptr::null_mut()))
}

unsafe fn key(hwnd: HWND, vk: u32) {
    let shift = GetKeyState(VK_SHIFT.0 as i32) < 0;
    let alt = GetKeyState(VK_MENU.0 as i32) < 0;
    let (z, _) = layout(hwnd);
    // 正在原地改名：只有 Enter/Esc 归窗口，别的按键都留在编辑框里改字。
    if !alt && matches!(vk, 0x0D | 0x1B) && app().lock().unwrap().ren_edit.is_some() {
        end_rename(hwnd, vk == 0x0D);
        return;
    }
    // 对话框开着时，键先归它：Enter 确定、Esc 关闭，其余交给里面的输入框。
    // 维护页多认一个 Del：删掉选中的扫描文件夹，跟「移除」按钮同一把刀。
    // Alt 组合不归对话框（Alt+Enter 不能当成"确定"再开一层属性框）。
    // 以图搜图那一页不模态：只有 Esc 是收页，别的键跟鼠标一样算底下那屏的。
    // 这一档先取出来再放锁：close_dlg/dlg_ok 自己还要拿这把锁，临时 guard 活到
    // 整个 if let 结束的话，进去就是死锁。
    let dlg_kind = {
        let g = app().lock().unwrap();
        g.dlg.as_ref().map(|d| d.kind)
    };
    if let Some(kind) = dlg_kind {
        if kind == DlgKind::SimSearch {
            if vk == 0x1B && !alt {
                close_dlg(hwnd);
                return;
            }
        } else {
            if alt {
                return;
            }
            match vk {
                0x1B => {
                    if kind == DlgKind::FmtOpt {
                        back_to_conv(hwnd);
                    } else {
                        close_dlg(hwnd);
                    }
                }
                0x0D => dlg_ok(hwnd),
                0x2E => {
                    if kind == DlgKind::ThumbDb {
                        remove_scan_sel(hwnd);
                    }
                }
                _ => {}
            }
            return;
        }
    }
    // Shift+F10 和菜单键：跟右键同一个入口，落点在当前焦点那一项上。
    if vk == 0x5D || (vk == 0x79 && shift) {
        ctx_menu_key(hwnd);
        return;
    }
    // Tab / F6 在两个窗格之间换键盘焦点，和资源管理器一样。
    if vk == 0x09 || vk == 0x75 {
        let mut g = app().lock().unwrap();
        if g.pane_tree {
            g.tree_kb = !g.tree_kb;
            drop(g);
            refresh(hwnd);
            return;
        }
    }
    // Alt 组合先认下来：树上按 Alt+Enter 是属性，不是"回车进目录"；
    // Alt+←/→ 也始终是前进后退，不在树上折节点。
    if alt {
        let want = match vk {
            0x0D => Some(M_PROP),
            0x43 => Some(M_COPY),
            0x4D => Some(M_MOVE),
            0x58 => Some(M_QUIT),
            0x25 => Some(M_BACK),
            0x26 => Some(M_UP),
            0x27 => Some(M_FWD),
            _ => None,
        };
        if let Some(id) = want {
            command(hwnd, id);
            return;
        }
    }
    // 键盘焦点在树上时，方向键/回车/翻页都归树。Del/F2 只在树上有文件落点
    // （不是“此电脑”）时放行，交给底下那套命令按树行执行。
    if app().lock().unwrap().tree_kb {
        match vk {
            0x26 | 0x28 | 0x25 | 0x27 | 0x24 | 0x23 | 0x21 | 0x22 | 0x0D => {
                tree_key(hwnd, vk);
                return;
            }
            0x2E | 0x71 => {
                if app().lock().unwrap().tree_focus_dir().is_none() {
                    return;
                }
            }
            _ => {}
        }
    }
    // Ctrl 组合键不在这里处理：它们走加速键表，中文输入法激活时也拦得住
    let ctrl_down = GetKeyState(VK_CONTROL.0 as i32) < 0;
    // 小键盘排序：菜单上写的就是 NumPad 0–6。紧跟着的 WM_CHAR 要吞掉，
    // 不然那个数字会被“打字跳条目”接走。
    if (0x60..=0x66).contains(&vk) && !ctrl_down {
        CHAR_SWALLOW.store(true, Ordering::SeqCst);
    }
    let quick = match vk {
        0x70 => Some(M_HELP),
        0x71 => Some(M_RENAME),
        0x74 => Some(M_RESCAN),
        0x2E => Some(if shift { M_DEL_PERM } else { M_DEL }),
        0x60 => Some(M_SORT_DESC),
        0x61 => Some(M_SORT_NAME),
        0x62 => Some(M_SORT_EXT),
        0x63 => Some(M_SORT_SIZE),
        0x64 => Some(M_SORT_TYPE),
        0x65 => Some(M_SORT_DATE),
        0x66 => Some(M_SORT_IMAGE),
        _ => None,
    };
    if let Some(id) = quick {
        command(hwnd, id);
        return;
    }
    let mut g = app().lock().unwrap();
    if vk == 0x1B
        && (g.open_menu.is_some() || g.ctx_at.is_some() || g.ctx_cat.is_some() || g.bmenu.is_some())
    {
        let had_b = g.bmenu.take().is_some();
        g.open_menu = None;
        g.open_sub = None;
        g.ctx_at = None;
        g.ctx_sub = None;
        g.ctx_cat = None;
        g.ctx_dir = None;
        g.ctx_img = None;
        g.ctx_blank = true;
        drop(g);
        // 工具栏菜单可能是带着悬停预览开着的，收掉时把那套外观还回去
        if had_b {
            menu_restore(hwnd);
        }
        refresh(hwnd);
        return;
    }
    let mut changed = false;
    let mut want: Option<Mode> = None;
    let kctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
    let step = |g: &mut App, d: i32| g.move_sel(d, z.grid, shift, kctrl);
    match vk {
        0x28 => {
            step(&mut g, 1);
            changed = true;
        }
        0x26 => {
            step(&mut g, -1);
            changed = true;
        }
        0x27 => {
            step(&mut g, 1);
            changed = true;
        }
        0x25 => {
            step(&mut g, -1);
            changed = true;
        }
        0x24 => {
            step(&mut g, -99);
            changed = true;
        }
        0x23 => {
            step(&mut g, 99);
            changed = true;
        }
        0x21 => {
            step(&mut g, -98);
            changed = true;
        }
        0x22 => {
            step(&mut g, 98);
            changed = true;
        }
        0x20 if kctrl => {
            // Ctrl+空格：切当前这一项，别处不动（和 Ctrl 点选同一个规矩）。
            let p = if g.pickable(g.sel) { g.entries.get(g.sel).map(|e| e.path.clone()) } else { None };
            if let Some(p) = p {
                if !g.marked.remove(&p) {
                    g.marked.insert(p.clone());
                }
                g.sel_lit = g.marked.contains(&p);
                g.anchor = g.sel;
                g.trace("spc");
                changed = true;
            }
        }
        0x0D => {
            let p = g.open_sel();
            let d = g.entries.get(g.sel).filter(|e| e.is_dir).map(|e| e.path.clone());
            drop(g);
            if let Some(p) = p {
                spawn_viewer(&p, hwnd);
            } else if let Some(d) = d {
                app().lock().unwrap().goto(&d, hwnd, true);
                request_pv(hwnd);
                refresh(hwnd);
            }
            return;
        }
        0x08 => {
            let up = g.parent();
            drop(g);
            if let Some(up) = up {
                app().lock().unwrap().goto(&up, hwnd, true);
                request_pv(hwnd);
                refresh(hwnd);
            }
            return;
        }
        // F8–F11，跟菜单上写的那几个键号一一对齐
        0x77 => want = Some(Mode::Grid),
        0x78 => want = Some(Mode::Small),
        0x79 => want = Some(Mode::List),
        0x7A => want = Some(Mode::Detail),
        0x1B => {
            // 转换正在跑时，Esc 先用来停转换，别顺手把标记也清了
            if CONV_RUN.load(Ordering::SeqCst) {
                CONV_STOP.store(true, Ordering::SeqCst);
                // 这里 g 还攥在手里，再 app().lock() 一次就是自己等自己：界面当场不动，
                // worker 取进度时也要这把锁，跟着一起停。
                g.msg = "正在停止转换，等手上这几张收尾…".to_string();
                drop(g);
                refresh(hwnd);
                return;
            }
            g.marked.clear();
            g.sel_lit = false;
            g.anchor = usize::MAX;
            // Esc 同时把键盘焦点从目录树收回文件区，和资源管理器一个规矩。
            g.tree_kb = false;
            g.trace("esc");
            changed = true;
        }
        _ => {}
    }
    if let Some(m) = want {
        drop(g);
        set_mode(hwnd, m);
        return;
    }
    if matches!(vk, 0x25..=0x28) {
        g.queue_visible(z.grid);
    } else if matches!(vk, 0x77..=0x7A) {
        g.feed(hwnd);
    }
    drop(g);
    if changed {
        check_menu(hwnd);
        request_pv(hwnd);
        refresh(hwnd);
    }
}

/// 打字直接跳到同名开头的条目，浏览大目录时比滚动快。
unsafe fn type_ahead(hwnd: HWND, ch: u16) {
    {
        let g = app().lock().unwrap();
        if g.dlg.is_some() && !sim_loose(&g, g.pt.0, g.pt.1) {
            return;
        }
        if g.tree_kb {
            drop(g);
            tree_type_ahead(hwnd, ch);
            return;
        }
    }
    let c = match char::from_u32(ch as u32) {
        Some(c) if c.is_alphanumeric() => c,
        _ => return,
    };
    let (z, _) = layout(hwnd);
    let mut g = app().lock().unwrap();
    let n = g.entries.len();
    if n == 0 {
        return;
    }
    // 没有光标时从末尾起步，第一圈的第一项就是第 0 项，不会把列表头上那个漏掉。
    let start = if g.sel == usize::MAX { n - 1 } else { g.sel };
    for k in 1..=n {
        let i = (start + k) % n;
        let nm = g.disp(i);
        if nm.chars().next().map(|x| x.to_lowercase().next()).flatten() == c.to_lowercase().next() {
            g.set_sel(i);
            // 跳到哪一项就等于点了哪一项：选区跟着落在这一项上。
            let cur = g.entries.get(i).filter(|e| !e.is_dir).map(|e| e.path.clone());
            g.marked.clear();
            if let Some(p) = cur {
                g.marked.insert(p);
            }
            g.anchor = i;
            g.keep_visible(z.grid);
            drop(g);
            request_pv(hwnd);
            refresh(hwnd);
            return;
        }
    }
}

/// 树里的打字跳转：从当前行往下绕一圈找名字开头的那个文件夹。
unsafe fn tree_type_ahead(hwnd: HWND, ch: u16) {
    let c = match char::from_u32(ch as u32) {
        Some(c) if c.is_alphanumeric() => c,
        _ => return,
    };
    let z = layout(hwnd).0;
    let mut g = app().lock().unwrap();
    let mut rows = Vec::new();
    flat_tree(&g.tree, 0, &mut rows);
    if rows.is_empty() {
        return;
    }
    let cur = {
        let r = tree_row(&rows, &g.tsel);
        // 找不到现行光标时从末尾起步：第一圈第一项是第 0 行。
        if r == usize::MAX { rows.len() - 1 } else { r }
    };
    let n = rows.len();
    for k in 1..=n {
        let i = (cur + k) % n;
        if rows[i].3.chars().next().map(|x| x.to_lowercase().next()).flatten()
            == c.to_lowercase().next()
        {
            let path = rows[i].1.clone();
            g.tsel = path.clone();
            if path != "此电脑" {
                g.tree_noscroll = true;
                g.goto(&path, hwnd, true);
            }
            tree_show(&mut g, &z, i);
            drop(g);
            request_pv(hwnd);
            refresh(hwnd);
            return;
        }
    }
}

impl App {
    /// 第一格是虚拟的"上一级"条目，显示名不能按真实路径取。
    fn disp(&self, i: usize) -> String {
        if self.has_up && i == 0 {
            "..".to_string()
        } else {
            self.entries.get(i).map(|e| stem(&e.path).to_string()).unwrap_or_default()
        }
    }

    /// 详细资料档每一行的大小和修改时间。`stat_of` 是一次磁盘 metadata，
    /// 一屏四十行、每帧一轮，滚动时一秒几千次；文件没重扫说明没变，答案也就没变。
    /// 按路径记下来，换目录或刷新（`restart_pool`）时整表清掉。
    fn stat_cached(&mut self, i: usize) -> (u64, String) {
        let p = &self.entries[i].path;
        if let Some(v) = self.stats.get(p) {
            return v.clone();
        }
        let v = stat_of(p);
        if self.stats.len() < 400_000 {
            self.stats.insert(p.clone(), v.clone());
        }
        v
    }
}

/// 菜单打开那一刻的皮肤与版式。这期间只是路过看看的，关菜单时要还回去。
static MENU_SNAP: Mutex<Option<(String, theme::Look, Option<String>)>> = Mutex::new(None);
static MENU_DEPTH: AtomicUsize = AtomicUsize::new(0);
/// 这次菜单里到底穿上去看过没有。没看过就一个字都不动，免得开个"文件"菜单也重排一遍。
static MENU_PREV: AtomicBool = AtomicBool::new(false);

/// 鼠标停在皮肤/版式菜单项上：当场换上给他看，只动内存，不写注册表也不写文件。
unsafe fn menu_preview(hwnd: HWND, id: usize) {
    if id >= M_SKIN && id - M_SKIN < theme::skins().len() {
        let names = theme::skins();
        let Some(n) = names.get(id - M_SKIN).cloned() else { return };
        let mut g = app().lock().unwrap();
        if g.skin != n {
            g.skin = n;
            g.look = theme::Look::load(&g.skin);
            MENU_PREV.store(true, Ordering::SeqCst);
            g.restyle_preview(hwnd);
        }
    } else if id > M_LAYOUT && id < M_LAYOUT + 1 + glyphs::all().len() {
        let key = glyphs::all()[id - M_LAYOUT - 1].key;
        let mut g = app().lock().unwrap();
        if g.layout.as_deref() != Some(key) {
            g.layout = Some(key.to_string());
            MENU_PREV.store(true, Ordering::SeqCst);
            g.restyle_preview(hwnd);
        }
    }
}

/// 菜单关了：这次没点中，就把打开菜单前那一套还回去。
unsafe fn menu_restore(hwnd: HWND) {
    if !MENU_PREV.swap(false, Ordering::SeqCst) {
        *MENU_SNAP.lock().unwrap() = None;
        return;
    }
    let Some((skin, look, layout)) = MENU_SNAP.lock().unwrap().take() else { return };
    {
        let mut g = app().lock().unwrap();
        g.skin = skin;
        g.look = look;
        g.layout = layout;
        // 预览这趟没动过缩略图池，回来也只换外观：池子跟着重开一次
        // 会把刚解好的缩略图全清掉，菜单一关就看着缩略图整屏重刷。
        g.restyle_preview(hwnd);
    }
    check_menu(hwnd);
}

/// 工具栏 ▾ 菜单此刻该有哪些项。皮肤名、历史路径都是现取的；图标库按原生那版平铺。
fn bar_items(a: &App, btn: usize) -> Vec<BItem> {
    match btn {
        M_EDIT => vec![
            BItem::new("在编辑器中打开(&O)", M_EDITOR),
            BItem::new("以外部程序打开(&P)...", M_EXTAPP),
        ],
        M_ADDR => {
            let mut v = Vec::new();
            if a.hist.is_empty() {
                v.push(BItem::off("还没有浏览记录"));
            }
            for (i, h) in a.hist.iter().enumerate().rev() {
                v.push(BItem::new(h.clone(), 3000 + i));
            }
            v
        }
        M_LOOK => {
            let names = theme::skins();
            let mut groups: Vec<String> = Vec::new();
            for n in &names {
                let g = theme::skin_group(n);
                if !g.is_empty() && !groups.contains(&g) {
                    groups.push(g);
                }
            }
            groups.sort();
            // 皮肤按组平铺，组名当一行灰标题。原生那版是"皮肤→组"两层子菜单，
            // 这里收成一层，分组照旧看得见，点起来少一层。
            let mut sk: Vec<BItem> = Vec::new();
            for g in &groups {
                if !sk.is_empty() {
                    sk.push(BItem::sep());
                }
                sk.push(BItem::off(g.clone()));
                for (i, n) in names.iter().enumerate() {
                    if theme::skin_group(n) == *g {
                        sk.push(BItem::checked(n.clone(), M_SKIN + i, n.as_str() == a.skin));
                    }
                }
            }
            let loose: Vec<BItem> = names
                .iter()
                .enumerate()
                .filter(|(_, n)| theme::skin_group(n).is_empty())
                .map(|(i, n)| BItem::checked(n.clone(), M_SKIN + i, n.as_str() == a.skin))
                .collect();
            if !loose.is_empty() {
                if !sk.is_empty() {
                    sk.push(BItem::sep());
                }
                sk.extend(loose);
            }
            let ly: Vec<BItem> = glyphs::all()
                .iter()
                .enumerate()
                .map(|(i, st)| {
                    BItem::checked(st.label, M_LAYOUT + 1 + i, a.layout.as_deref() == Some(st.key))
                })
                .collect();
            vec![
                BItem::submenu("皮肤(&K)", sk),
                BItem::submenu("图标(&L)", ly),
                BItem::submenu(
                    "密度(&Y)",
                    vec![BItem::new("标准(&N)", M_DENS + 1), BItem::new("宽松(&W)", M_DENS + 2)],
                ),
                BItem::sep(),
                BItem::new("皮肤设置(&S)...", M_SKIN_SET),
            ]
        }
        _ => Vec::new(),
    }
}

/// 这一项是不是"路过就换上给他看"的外观项
fn preview_id(id: usize) -> bool {
    (id >= M_SKIN && id - M_SKIN < theme::skins().len())
        || (id > M_LAYOUT && id < M_LAYOUT + 1 + glyphs::all().len())
}

/// 打开工具栏 ▾ 菜单。条目在这一刻现排，顺手把外观快照记下来，
/// 悬停预览过、最后没点中时关菜单要还回去。
unsafe fn open_bmenu(hwnd: HWND, btn: usize) {
    let r = app()
        .lock()
        .unwrap()
        .btns
        .iter()
        .find(|(i, _)| *i == btn)
        .map(|(_, r)| *r);
    let Some(r) = r else { return };
    // 上一张还开着：按"没点中"收掉，别把它的预览留在界面上
    if app().lock().unwrap().bmenu.take().is_some() {
        menu_restore(hwnd);
    }
    let mut g = app().lock().unwrap();
    let items = bar_items(&g, btn);
    *MENU_SNAP.lock().unwrap() = Some((g.skin.clone(), g.look.clone(), g.layout.clone()));
    MENU_PREV.store(false, Ordering::SeqCst);
    g.bmenu = Some(BMenu {
        anchor: r,
        items,
        rect: RECT::default(),
        rows: Vec::new(),
        sub_open: None,
        sub_rect: RECT::default(),
        sub_rows: Vec::new(),
        hover: -1,
        sub_hover: -1,
    });
    drop(g);
    refresh(hwnd);
}

/// 收起工具栏菜单。`restore` = 把悬停预览换掉的那套外观还回去；
/// 点中皮肤/版式项时传 false，让 command 那边自己落盘。
unsafe fn close_bmenu(hwnd: HWND, restore: bool) {
    if app().lock().unwrap().bmenu.take().is_none() {
        return;
    }
    if restore {
        menu_restore(hwnd);
    }
    // 点中皮肤/版式项时预览标记与快照都留着：command 那边靠它们判断
    // "这套是悬停换上的"，才会把选择真正落盘。
    refresh(hwnd);
}

/// 鼠标在工具栏菜单上移动：换行重画；停在皮肤/版式项上当场换给他看。
unsafe fn bmenu_move(hwnd: HWND, x: i32, y: i32) {
    let mut changed = false;
    let mut id = 0usize;
    {
        let mut g = app().lock().unwrap();
        let Some(m) = g.bmenu.as_mut() else { return };
        let sub = if m.sub_open.is_some() && over(&m.sub_rect, x, y) {
            m.sub_rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i as i32).unwrap_or(-1)
        } else {
            -1
        };
        let row = if sub >= 0 {
            -1
        } else {
            m.rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i as i32).unwrap_or(-1)
        };
        if m.hover != row || m.sub_hover != sub {
            m.hover = row;
            m.sub_hover = sub;
            changed = true;
        }
        if sub >= 0 {
            if let Some(it) =
                m.sub_open.and_then(|p| m.items.get(p)).and_then(|it| it.sub.get(sub as usize))
            {
                id = it.id;
            }
        } else if row >= 0 {
            if let Some(it) = m.items.get(row as usize) {
                id = it.id;
                // 扫到带子层的那一行就展开；进了子层或移到面板外都不收
                let want = if it.sub.is_empty() { None } else { Some(row as usize) };
                if m.sub_open != want {
                    m.sub_open = want;
                    m.sub_hover = -1;
                    changed = true;
                }
            }
        }
    }
    if changed {
        refresh(hwnd);
    }
    if id != 0 {
        menu_preview(hwnd, id);
    }
}

/// 工具栏菜单里按下左键。还回这一下归不归菜单吃。
unsafe fn bmenu_click(hwnd: HWND, x: i32, y: i32) -> bool {
    let mut act: Option<(usize, bool)> = None;
    let mut close_only = false;
    let mut opened = false;
    {
        let mut g = app().lock().unwrap();
        let Some(m) = g.bmenu.as_mut() else { return false };
        if m.sub_open.is_some() && over(&m.sub_rect, x, y) {
            if let Some(si) = m.sub_rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
                if let Some(it) =
                    m.sub_open.and_then(|p| m.items.get(p)).and_then(|it| it.sub.get(si))
                {
                    if !it.off && it.id != 0 {
                        act = Some((it.id, preview_id(it.id)));
                    }
                }
            }
        } else if let Some(pi) = m.rows.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i) {
            if let Some(it) = m.items.get(pi) {
                if !it.sub.is_empty() {
                    m.sub_open = Some(pi);
                    m.sub_hover = -1;
                    opened = true;
                } else if !it.off && it.id != 0 {
                    act = Some((it.id, preview_id(it.id)));
                }
            }
        } else {
            close_only = true;
        }
    }
    if opened {
        refresh(hwnd);
        return true;
    }
    if let Some((id, keep)) = act {
        if keep {
            menu_preview(hwnd, id);
        }
        close_bmenu(hwnd, !keep);
        command(hwnd, id);
        return true;
    }
    if close_only {
        close_bmenu(hwnd, true);
        // 点在工具栏按钮上就让这一下继续走：再点"外观"是收起/重开，
        // 点别的按钮直接换菜单。落在别处第一下只负责关菜单。
        let on_btn = app().lock().unwrap().btns.iter().any(|(_, r)| over(r, x, y));
        return !on_btn;
    }
    true
}

/// 画一层菜单面板（主层或子层共用）：底色、1px 框、图标/勾选列、悬停高亮。
unsafe fn paint_bitems(
    hdc: HDC,
    a: &App,
    s: &Theme,
    items: &[BItem],
    rect: &RECT,
    hover: i32,
) -> Vec<(usize, RECT)> {
    let k = s.scale;
    fill(hdc, rect, s.skin.popup);
    if fine(s) {
        hline(
            hdc,
            rect.left + 1,
            rect.right - 1,
            rect.top + 1,
            mix(s.skin.popup, s.skin.line_soft, 0.55),
        );
    }
    frame(hdc, rect, s.skin.acc);
    let pad = (theme::MENU_SEP_PAD * k) as i32;
    let tst = theme::Strip::top(s);
    let mut rows = Vec::with_capacity(items.len());
    let mut cy = rect.top + (theme::MENU_INSET * k) as i32;
    for (i, it) in items.iter().enumerate() {
        let rh = if it.sep {
            (theme::MENU_SEP_H * k) as i32
        } else {
            theme::popup_row(s)
        };
        let r = RECT { left: rect.left, top: cy, right: rect.right, bottom: cy + rh };
        cy += rh;
        if it.sep {
            hline(hdc, r.left + pad, r.right - pad, (r.top + r.bottom) / 2, s.skin.line_soft);
            rows.push((i, r));
            continue;
        }
        let hot = hover == i as i32;
        if hot {
            fill(hdc, &r, tst.bar);
        }
        let ink = if it.off {
            if hot { tst.dim } else { s.skin.ink_off }
        } else if hot {
            tst.fg
        } else {
            s.skin.ink
        };
        let ck = RECT {
            left: r.left + (MENU_IC_X * k) as i32,
            top: r.top,
            right: r.left + ((MENU_IC_X + MENU_IC_W) * k) as i32,
            bottom: r.bottom,
        };
        if it.check {
            let cc = if hot {
                tst.fg
            } else if fine(s) || !bands(s) {
                s.skin.acc
            } else {
                ink
            };
            if bands(s) {
                tick(hdc, (ck.left + ck.right) / 2, (ck.top + ck.bottom) / 2, cc, k);
            } else {
                txt(hdc, &ck, "✓", cc, a.font_sm, DT_CENTER | DT_SINGLELINE | DT_VCENTER);
            }
        } else if it.id != 0 {
            if let Some(g) = menuicons::find(it.id) {
                let isz = (MENU_IC_W * k).round() as i32;
                let ibg = if hot { tst.bar } else { s.skin.popup };
                draw_glyph(
                    hdc,
                    g,
                    r.left + (MENU_IC_X * k) as i32,
                    (r.top + r.bottom - isz) / 2,
                    isz,
                    ink,
                    ibg,
                );
            }
        }
        let lr = RECT {
            left: r.left + theme::menu_text_x(k),
            top: r.top,
            right: r.right - (18.0 * k) as i32,
            bottom: r.bottom,
        };
        txt(hdc, &lr, &it.label, ink, a.font, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
        if !it.sub.is_empty() {
            let ar = RECT {
                left: r.right - (14.0 * k) as i32,
                top: r.top,
                right: r.right - (2.0 * k) as i32,
                bottom: r.bottom,
            };
            if fine(s) || bands(s) {
                tri(
                    hdc,
                    (ar.left + ar.right) / 2,
                    (ar.top + ar.bottom) / 2,
                    (3.2 * k) as i32,
                    if hot { tst.fg } else { s.skin.ink_off },
                    false,
                );
            } else {
                txt(
                    hdc,
                    &ar,
                    "▸",
                    if hot { tst.fg } else { s.skin.ink_dim },
                    a.font_sm,
                    DT_CENTER | DT_SINGLELINE | DT_VCENTER,
                );
            }
        }
        rows.push((i, r));
    }
    rows
}

unsafe fn paint_bmenu(hdc: HDC, a: &mut App, s: &Theme, rc: &RECT) {
    a.zone(theme::Z_POPUP);
    let Some((items, ax, ay, hover, sub_hover, sub_open)) = a.bmenu.as_ref().map(|m| {
        (m.items.clone(), m.anchor.left, m.anchor.bottom, m.hover, m.sub_hover, m.sub_open)
    }) else {
        return;
    };
    let k = s.scale;
    let mut lw = 0;
    let mut h = (theme::MENU_INSET_Y * k) as i32;
    for it in &items {
        if it.sep {
            h += (theme::MENU_SEP_H * k) as i32;
            continue;
        }
        h += theme::popup_row(s);
        lw = lw.max(text_w(hdc, &it.label, a.font));
    }
    let w = (theme::menu_text_x(k) + lw + (26.0 * k) as i32).max((150.0 * k) as i32);
    let mut x = ax;
    let mut y = ay;
    if x + w > rc.right - 2 {
        x = (rc.right - w - 2).max(rc.left);
    }
    if y + h > rc.bottom - 2 {
        y = (ay - h).max(rc.top);
    }
    let rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
    let rows = paint_bitems(hdc, a, s, &items, &rect, hover);
    let mut srect = RECT::default();
    let mut srows = Vec::new();
    if let Some(pi) = sub_open {
        if let Some(sub) = items.get(pi).map(|it| &it.sub).filter(|v| !v.is_empty()) {
            let mut lw = 0;
            let mut h = (theme::MENU_INSET_Y * k) as i32;
            for it in sub {
                if it.sep {
                    h += (theme::MENU_SEP_H * k) as i32;
                    continue;
                }
                h += theme::popup_row(s);
                lw = lw.max(text_w(hdc, &it.label, a.font));
            }
            let w2 = (theme::menu_text_x(k) + lw + (26.0 * k) as i32).max((150.0 * k) as i32);
            let prow = rows.iter().find(|(i, _)| *i == pi).map(|(_, r)| *r).unwrap_or(rect);
            let mut sx = prow.right - (2.0 * k) as i32;
            let mut sy = prow.top - (3.0 * k) as i32;
            if sx + w2 > rc.right - 2 {
                sx = (rect.left - w2).max(rc.left);
            }
            if sy + h > rc.bottom - 2 {
                sy = (rc.bottom - h - 2).max(rc.top);
            }
            srect = RECT { left: sx, top: sy, right: sx + w2, bottom: sy + h };
            srows = paint_bitems(hdc, a, s, sub, &srect, sub_hover);
        }
    }
    if let Some(m) = a.bmenu.as_mut() {
        m.rect = rect;
        m.rows = rows;
        m.sub_rect = srect;
        m.sub_rows = srows;
    }
}

unsafe fn command(hwnd: HWND, id: usize) {
    // 删除进回收站不用问：回收站里能还原。找不回的这两类（彻底删除、删类别）
    // 先过确认框，点过"是"的那次带 ask_go 标记，才真的往下执行
    if matches!(id, M_CAT_DEL | M_DEL_PERM) {
        let go = app().lock().unwrap().ask_go;
        if go != id {
            ask_delete(hwnd, id);
            return;
        }
        app().lock().unwrap().ask_go = 0;
    }
    // 以图搜图头一回点先讲一页；说明框的「确定」带 ask_go 回来，这一趟直接进搜索页。
    if id == M_FINDSIM {
        let (seen, go) = {
            let g = app().lock().unwrap();
            (g.sim_intro, g.ask_go)
        };
        if !seen && go != id {
            sim_intro(hwnd);
            return;
        }
        app().lock().unwrap().ask_go = 0;
    }
    if id >= 3000 {
        let p = app().lock().unwrap().hist.get(id - 3000).cloned();
        if let Some(p) = p {
            let mut g = app().lock().unwrap();
            g.goto(&p, hwnd, false);
            g.hpos = id - 3000;
            drop(g);
            request_pv(hwnd);
            refresh(hwnd);
        }
        return;
    }
    match id {
        D_OK | D_START => dlg_ok(hwnd),
        D_CANCEL => close_dlg(hwnd),
        D_CLOSE => {
            if app().lock().unwrap().dlg.as_ref().map(|d| d.kind) == Some(DlgKind::FmtOpt) {
                back_to_conv(hwnd);
            } else {
                close_dlg(hwnd);
            }
        }
        M_MIN => {
            let _ = ShowWindow(hwnd, SW_MINIMIZE);
        }
        M_MAX => {
            let _ = ShowWindow(hwnd, if IsZoomed(hwnd).as_bool() { SW_RESTORE } else { SW_MAXIMIZE });
        }
        M_CLOSE => {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        M_OPEN => {
            let (p, d) = {
                let g = app().lock().unwrap();
                (
                    g.open_sel(),
                    g.entries.get(g.sel).filter(|e| e.is_dir).map(|e| e.path.clone()),
                )
            };
            if let Some(p) = p {
                spawn_viewer(&p, hwnd);
            } else if let Some(d) = d {
                app().lock().unwrap().goto(&d, hwnd, true);
                request_pv(hwnd);
                refresh(hwnd);
            }
        }
        M_EXPL => {
            // 转到文件：类别视图和查找结果里列的图各来自不同文件夹，这一条是
            // 带回它自己那个文件夹的。右键落点或被键盘选中的树行就是文件夹本体。
            let p = {
                let g = app().lock().unwrap();
                if let Some(d) = g.ctx_dir.clone() {
                    Some(d)
                } else if g.tree_kb {
                    g.tree_focus_dir()
                } else {
                    g.entries.get(g.sel).map(|e| e.path.clone())
                }
            };
            if let Some(p) = p {
                reveal_in_view(hwnd, &p);
            }
        }
        M_RESCAN => {
            let d = app().lock().unwrap().dir.to_string_lossy().to_string();
            app().lock().unwrap().goto(&d, hwnd, false);
            request_pv(hwnd);
            refresh(hwnd);
        }
        M_TREE_REFRESH => {
            // 只重扫右键那一行的子节点；它正好是当前目录时，格子也跟着重扫一遍
            let d = app().lock().unwrap().ctx_dir.clone();
            if let Some(d) = d {
                let nm = Path::new(&d)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| d.clone());
                let mut g = app().lock().unwrap();
                tree_reload(&mut g.tree, &d);
                let hit = g
                    .dir
                    .to_string_lossy()
                    .trim_end_matches('\\')
                    .eq_ignore_ascii_case(d.trim_end_matches('\\'));
                if hit {
                    g.goto(&d, hwnd, false);
                }
                g.msg = format!("已刷新 {nm} 这一层");
                drop(g);
                request_pv(hwnd);
                refresh(hwnd);
            }
        }
        M_FILT_ALL => {
            {
                let mut g = app().lock().unwrap();
                g.filt_dirs = false;
                g.filt_nonimg = false;
                g.filt_cat = false;
                g.msg = "过滤已关掉，全部显示".to_string();
            }
            refilter(hwnd);
        }
        M_FILT_DIRS | M_FILT_NONIMG | M_FILT_CAT => {
            {
                let mut g = app().lock().unwrap();
                let (on, name) = match id {
                    M_FILT_DIRS => {
                        g.filt_dirs = !g.filt_dirs;
                        (g.filt_dirs, "文件夹")
                    }
                    M_FILT_NONIMG => {
                        g.filt_nonimg = !g.filt_nonimg;
                        (g.filt_nonimg, "非图片文件")
                    }
                    _ => {
                        g.filt_cat = !g.filt_cat;
                        (g.filt_cat, "已分类图片")
                    }
                };
                g.msg = if matches!(g.src, Src::Folder) {
                    format!("{}{}", if on { "已隐藏" } else { "已显示" }, name)
                } else {
                    format!("开关记住了，但过滤只在浏览文件夹时用")
                };
            }
            refilter(hwnd);
        }
        M_QUIT => {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        M_ALL => {
            // 全选 = 这个目录在视图里列出来的每一个文件，跟屏幕装得下几张无关。
            // 文件夹一律不选；缩略图档只列图片，图标/列表/详细资料连非图片一起列，
            // 全选到的就是当前视图里列出来的这些文件。选区按路径记，
            // 切到别的视图时这套选中原样还在。
            let mut g = app().lock().unwrap();
            g.marked = g
                .entries
                .iter()
                .filter(|e| !e.is_dir)
                .map(|e| e.path.clone())
                .collect();
            // 光标这时若停在文件夹上，它会画成亮着的，看着像文件夹也被选进来了；
            // 全选不含文件夹，把光标挪到第一个文件上。
            if g.entries.get(g.sel).map_or(true, |e| e.is_dir) {
                if let Some(i) = g.entries.iter().position(|e| !e.is_dir) {
                    g.sel = i;
                    g.anchor = i;
                    g.sel_lit = true;
                }
            }
            // 选上了要在状态栏说一句。光有格子那圈选框不够：一屏之外的没画，
            // 人看不出这条命令到底跑没跑。
            let n = g.marked.len();
            g.msg = format!("已选取 {n} 个文件");
            g.trace("all");
            drop(g);
            refresh(hwnd);
        }
        M_NONE => {
            let mut g = app().lock().unwrap();
            g.marked.clear();
            // 取消全选之后光标那一项也不该再亮着，不然看着像还剩一张没取消。
            g.sel_lit = false;
            g.anchor = usize::MAX;
            g.stamp += 1;
            g.trace("none");
            drop(g);
            refresh(hwnd);
        }
        M_VIEWER => {
            let d = app().lock().unwrap().dir.clone();
            spawn_viewer(&d, hwnd);
        }
        M_BACK => {
            let mut g = app().lock().unwrap();
            if g.hpos > 0 {
                g.hpos -= 1;
                let p = g.hist[g.hpos].clone();
                drop(g);
                app().lock().unwrap().goto(&p, hwnd, false);
                request_pv(hwnd);
                refresh(hwnd);
            }
        }
        M_FWD => {
            let mut g = app().lock().unwrap();
            if g.hpos + 1 < g.hist.len() {
                g.hpos += 1;
                let p = g.hist[g.hpos].clone();
                drop(g);
                app().lock().unwrap().goto(&p, hwnd, false);
                request_pv(hwnd);
                refresh(hwnd);
            }
        }
        M_UP => {
            let up = app().lock().unwrap().parent();
            match up {
                Some(up) => {
                    app().lock().unwrap().goto(&up, hwnd, true);
                    request_pv(hwnd);
                    refresh(hwnd);
                }
                None => {
                    app().lock().unwrap().msg = "已经在盘根".to_string();
                    refresh(hwnd);
                }
            }
        }
        M_VGRID => set_mode(hwnd, Mode::Grid),
        M_VSMALL => set_mode(hwnd, Mode::Small),
        M_VLIST => set_mode(hwnd, Mode::List),
        M_VDETAIL => set_mode(hwnd, Mode::Detail),
        M_ZOOM_DN | M_ZOOM_UP => nudge_thumb(hwnd, id == M_ZOOM_UP),
        M_SORT_NAME | M_SORT_EXT | M_SORT_SIZE | M_SORT_TYPE | M_SORT_DATE | M_SORT_IMAGE
        | M_SORT_ADDED | M_SORT_DESC | M_SORT_FWD | M_SORT_REV => {
            {
                let mut g = app().lock().unwrap();
                match id {
                    M_SORT_NAME => g.sort = SortKey::Name,
                    M_SORT_EXT => g.sort = SortKey::Ext,
                    M_SORT_SIZE => g.sort = SortKey::Size,
                    M_SORT_TYPE => g.sort = SortKey::Type,
                    M_SORT_DATE => g.sort = SortKey::Date,
                    M_SORT_IMAGE => g.sort = SortKey::Image,
                    M_SORT_ADDED => g.sort = SortKey::Added,
                    M_SORT_DESC => g.sort = SortKey::Desc,
                    M_SORT_FWD => g.sort_desc = false,
                    _ => g.sort_desc = true,
                }
                g.apply_sort();
                // 右键菜单/查看菜单改排列也当场记一笔，不等到关窗：崩了或没关干净就不丢
                ui_save(&g, hwnd);
            }
            done_view(hwnd);
            request_pv(hwnd);
        }
        M_NEWDIR => {
            // 类别视图底下没有落地目录，和粘贴同一个口径，别往上一处文件夹里乱建。
            if matches!(app().lock().unwrap().src, Src::Category(_)) {
                app().lock().unwrap().msg = "类别视图没有落地目录，请先打开一个文件夹再新建".to_string();
                refresh(hwnd);
                return;
            }
            let dir = app().lock().unwrap().dir.clone();
            let mut name = "新建文件夹".to_string();
            let mut n = 2;
            let mut p = dir.join(&name);
            while p.exists() {
                name = format!("新建文件夹 ({n})");
                p = dir.join(&name);
                n += 1;
            }
            match std::fs::create_dir(&p) {
                Ok(()) => {
                    let d = dir.display().to_string();
                    let (z, _) = layout(hwnd);
                    let idx = {
                        let mut g = app().lock().unwrap();
                        g.goto(&d, hwnd, false);
                        // 左树里这个目录的子节点列表也要多出这一格
                        tree_reload(&mut g.tree, &d);
                        let target = p.display().to_string();
                        let i = g.entries.iter().position(|e| e.path.eq_ignore_ascii_case(&target));
                        if let Some(i) = i {
                            g.set_sel(i);
                            g.keep_visible(z.grid);
                        }
                        i
                    };
                    request_pv(hwnd);
                    refresh(hwnd);
                    // 和资源管理器一样：新建完直接进改名态，名字整段选中，敲字就替换。
                    if let Some(i) = idx {
                        begin_rename(hwnd, i);
                    }
                }
                Err(e) => {
                    app().lock().unwrap().msg = format!("新建文件夹失败：{e}");
                    refresh(hwnd);
                }
            }
        }
        M_CUT | M_COPYF => {
            // 地址栏有选区时 Ctrl+C 先给文字：刚拖完路径的人要的是那串字，不是文件。
            // 剪切不抢——这里没有"剪走路径"这回事，照旧走文件剪切。
            if id == M_COPYF {
                let sel = app().lock().unwrap().addr_sel_text();
                if let Some(t) = sel {
                    let msg = match clip::copy_text(&t) {
                        Ok(()) => "已把选中文字复制到剪贴板".to_string(),
                        Err(e) => e,
                    };
                    app().lock().unwrap().msg = msg;
                    refresh(hwnd);
                    return;
                }
            }
            // 树上右键/键盘焦点在树上时：复制或剪切的是树上的文件夹本体，
            // 不是格子选区。只许 lock 一次：match 的临时 guard 活到整个表达式结束，
            // 以前 None 分支里又 lock 了一遍，std 的 Mutex 不可重入，按 Ctrl+C 整窗直接挂死。
            let (tree, tp) = tree_target();
            let paths = match (tree, tp) {
                (true, Some(d)) => {
                    let editable = {
                        let g = app().lock().unwrap();
                        g.tree_editable(&d)
                    };
                    if id == M_CUT && !editable {
                        app().lock().unwrap().msg = "这一项不能剪切".to_string();
                        refresh(hwnd);
                        return;
                    }
                    vec![d]
                }
                (true, None) => {
                    app().lock().unwrap().msg = "先在文件夹树上选一个文件夹".to_string();
                    refresh(hwnd);
                    return;
                }
                (false, _) => app().lock().unwrap().selected_paths(),
            };
            let cut = id == M_CUT;
            let r = clip::copy_paths(&paths, cut);
            {
                let mut g = app().lock().unwrap();
                let ok = r.is_ok();
                g.msg = match r {
                    Ok(()) => format!(
                        "已{} {} 个文件到剪贴板",
                        if cut { "剪切" } else { "复制" },
                        paths.len()
                    ),
                    Err(e) => e,
                };
                // 没进剪贴板就别闪：这一下整窗提示是"办成了"的信号，
                // 失败也闪的话，后面粘贴不出文件就找不到是复制那一步没成。
                if ok {
                    g.flash = Some(Instant::now());
                }
            }
            refresh(hwnd);
        }
        M_COPYIMG => {
            let p = app().lock().unwrap().selected_paths().into_iter().next();
            let msg = match p {
                Some(p) => match clip::copy_image(&p) {
                    Ok(()) => "图像已复制到剪贴板".to_string(),
                    Err(e) => e,
                },
                None => "没有选中图片".to_string(),
            };
            app().lock().unwrap().msg = msg;
            refresh(hwnd);
        }
        M_COPYPATH => {
            // 路径按纯文本进剪贴板：树落点优先，其次格子里的选区。多选用换行分开，
            // 和资源管理器“复制路径”的口径一致。
            let (tree, tp) = tree_target();
            let paths = match (tree, tp) {
                (true, Some(d)) => vec![d],
                (true, None) => {
                    app().lock().unwrap().msg = "先在文件夹树上选一个文件夹".to_string();
                    refresh(hwnd);
                    return;
                }
                (false, _) => app().lock().unwrap().selected_paths(),
            };
            let msg = if paths.is_empty() {
                "没有选中文件".to_string()
            } else {
                match clip::copy_text(&paths.join("\r\n")) {
                    Ok(()) => format!("已复制 {} 个路径", paths.len()),
                    Err(e) => e,
                }
            };
            app().lock().unwrap().msg = msg;
            refresh(hwnd);
        }
        M_PASTE => {
            if matches!(app().lock().unwrap().src, Src::Category(_)) {
                app().lock().unwrap().msg = "类别视图没有落地目录，请先打开一个文件夹再粘贴".to_string();
                refresh(hwnd);
                return;
            }
            let (paths, cut) = clip::clipboard_paths();
            if paths.is_empty() {
                app().lock().unwrap().msg = "剪贴板里没有文件".to_string();
                refresh(hwnd);
                return;
            }
            let dir = app().lock().unwrap().dir.display().to_string();
            let (pairs, skip, err) = clip::paste_into(&paths, &dir, cut);
            let mut m = if pairs.is_empty() && !err.is_empty() {
                format!("粘贴失败：{err}")
            } else {
                let mut s = format!("粘贴 {} 个文件", pairs.len());
                if skip > 0 {
                    s.push_str(&format!("，{skip} 个没过去"));
                }
                if !err.is_empty() {
                    s.push_str(&format!("：{err}"));
                }
                s
            };
            {
                let mut g = app().lock().unwrap();
                if cut && !pairs.is_empty() {
                    for (old, new) in &pairs {
                        cache::move_path(old, new);
                        sim::moved(old, new);
                    }
                    if let Some(e) = sync_cat_paths(&mut g, &pairs) {
                        m.push_str(&format!("；{e}"));
                    } else {
                        m.push_str("，类别位置已同步");
                    }
                }
                g.msg = m;
                // 树里那两层的缓存也要跟着改：搬走的不再钉在老位置，搬进来的要出现
                for (old, new) in &pairs {
                    for p in [old, new] {
                        if let Some(par) = Path::new(p).parent() {
                            tree_reload(&mut g.tree, &par.display().to_string());
                        }
                    }
                }
                if !pairs.is_empty() {
                    g.flash = Some(Instant::now());
                }
                g.refresh_src(hwnd);
            }
            request_pv(hwnd);
        }
        M_DEL | M_DEL_PERM => {
            // 确认（只有彻底删除要）在 command() 开头做过，这里只管执行
            // 树落点：删的是右键中或键盘选中的那个文件夹本体。
            let (tree, tp) = tree_target();
            let folder = if tree { tp.clone() } else { None };
            let paths = match (tree, tp) {
                (true, Some(d)) => {
                    let editable = {
                        let g = app().lock().unwrap();
                        g.tree_editable(&d)
                    };
                    if !editable {
                        app().lock().unwrap().msg = "这一项不能删除".to_string();
                        refresh(hwnd);
                        return;
                    }
                    vec![d]
                }
                (true, None) => {
                    app().lock().unwrap().msg = "先在文件夹树上选一个文件夹".to_string();
                    refresh(hwnd);
                    return;
                }
                (false, _) => app().lock().unwrap().selected_paths(),
            };
            if paths.is_empty() {
                app().lock().unwrap().msg = "没有选中文件".to_string();
                refresh(hwnd);
                return;
            }
            let perm = id == M_DEL_PERM;
            let res = if perm { clip::delete_permanent(&paths) } else { clip::delete_to_recycle(&paths) };
            let ok = res.is_ok();
            {
                let mut g = app().lock().unwrap();
                let mut m = match res {
                    Ok(n) if perm => format!("已彻底删除 {n} 个文件"),
                    Ok(n) => format!("已把 {n} 个文件放进回收站"),
                    Err(e) => e,
                };
                if ok {
                    if let Some(cat) = g.cat.as_mut() {
                        for p in &paths {
                            cat.forget_path(p);
                        }
                        if let Err(e) = cat.save_full(&catalog_file()) {
                            m.push_str(&format!("；类别里的死路径已清，但保存失败：{e}"));
                        }
                    }
                    // 删的是树上的文件夹本体，而它多半就是当前目录：退回上一级，
                    // 不然这扇窗停在一个已经不存在的路径里。
                    if let Some(d) = &folder {
                        let cur = g.dir.display().to_string();
                        let inside = cur.eq_ignore_ascii_case(d) || cur.starts_with(&format!("{d}\\"));
                        if inside {
                            let up = std::path::Path::new(d)
                                .parent()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| d.clone());
                            g.goto(&up, hwnd, false);
                        }
                    }
                }
                g.msg = m;
                g.refresh_src(hwnd);
            }
            request_pv(hwnd);
        }
        M_RENAME => {
            // 改名只在名字那一格上原地进行，没有对话框。树落点优先（右键或键盘焦点），
            // 没有树落点才改格子里光标那一项。
            let (tree, tp) = tree_target();
            if tree {
                let Some(d) = tp else {
                    app().lock().unwrap().msg = "先在文件夹树上选一个文件夹".to_string();
                    refresh(hwnd);
                    return;
                };
                let editable = {
                    let g = app().lock().unwrap();
                    g.tree_editable(&d)
                };
                if !editable {
                    app().lock().unwrap().msg = "这一项不能改名".to_string();
                    refresh(hwnd);
                } else if let Some(r) = tree_row_rect(hwnd, &d) {
                    begin_rename_tree(hwnd, &d, r);
                } else {
                    app().lock().unwrap().msg = "树上的这一行已经找不到了".to_string();
                    refresh(hwnd);
                }
            } else {
                let idx = app().lock().unwrap().sel;
                if idx != usize::MAX {
                    begin_rename(hwnd, idx);
                }
            }
        }
        M_COPY | M_MOVE => {
            let copy = id == M_COPY;
            let paths = app().lock().unwrap().selected_paths();
            if paths.is_empty() {
                app().lock().unwrap().msg = "没有选中文件".to_string();
                refresh(hwnd);
                return;
            }
            let title = if copy { "复制到文件夹" } else { "移到文件夹" };
            if let Some(dir) = browse_for_folder(hwnd, title) {
                transfer_to_folder(hwnd, paths, dir, copy);
            }
        }
        M_CONV => open_dlg(hwnd, DlgKind::Convert),
        M_FIND => open_dlg(hwnd, DlgKind::Search),
        M_ASSOC => open_dlg(hwnd, DlgKind::Assoc),
        M_PROP => show_props(hwnd),
        M_SKIN_SET => open_dlg(hwnd, DlgKind::Skin),
        M_CATDB => open_dlg(hwnd, DlgKind::CatDb),
        M_THUMBDB => open_dlg(hwnd, DlgKind::ThumbDb),
        M_HELP => open_ask(
            hwnd,
            "帮助",
            "星雨图片查看器\n\
             \n\
             一款 Windows 看图软件，界面照 ACDSee 3.0 那一套布局重排了一遍。\n\
             在看图之外加了三样：图片管理（按类别把图归起来）、以图搜图（找长得像的图）、参考图画布（把图浮在别的软件上当对照）。",
            false,
            0,
        ),
        M_NEWWIN => {
            if let Ok(exe) = std::env::current_exe() {
                let d = app().lock().unwrap().dir.clone();
                let _ = std::process::Command::new(exe).arg(d).spawn();
            }
        }
        M_REF => {
            // 多选也只摆一张：这条要的是"手上这张"，不是把选中的全摊进画布。
            let p = app().lock().unwrap().selected_images().into_iter().next();
            match p {
                Some(p) => refwin::open(&p, hwnd),
                None => {
                    app().lock().unwrap().msg = "没有选中图片".to_string();
                    refresh(hwnd);
                }
            }
        }
        M_FINDSIM => {
            // 多选时拿第一张：一次只能有一张当查询图，那一页上写明用的是哪张。
            // 右键进来的直接认右键那一格的图（ctx_img），不走全局选区。
            let q = {
                let mut g = app().lock().unwrap();
                g.ctx_img
                    .take()
                    .map(|p| vec![p])
                    .unwrap_or_else(|| g.selected_images())
                    .into_iter()
                    .next()
            };
            unsafe { open_dlg(hwnd, DlgKind::SimSearch) };
            match q {
                Some(p) => unsafe {
                    {
                        let mut g = app().lock().unwrap();
                        g.sim_path = p;
                        g.sim_hits.clear();
                        g.sim_sel = 0;
                        g.sim_scroll = 0;
                        g.sim_ms = 0;
                    }
                    sim_previews(hwnd);
                    start_sim_search(hwnd);
                },
                None => {
                    // 这一页会记住上次选的那张图。文件被删、被移走以后，再打开就只剩
                    // 一句“原图打不开”和空预览框，看着像坏了；这里当场把死路径清掉。
                    let mut g = app().lock().unwrap();
                    let stale = !g.sim_path.is_empty() && !Path::new(&g.sim_path).is_file();
                    if stale {
                        g.sim_path.clear();
                        g.sim_qinfo_path.clear();
                        g.sim_qinfo.clear();
                        g.sim_src_img = None;
                        g.sim_hit_img = None;
                        g.sim_hits.clear();
                        g.sim_sel = 0;
                        g.sim_scroll = 0;
                        g.sim_ms = 0;
                        g.sim_lib_msg = "上次那张图已经不在了，请重新选择".to_string();
                    } else {
                        g.sim_lib_msg = "先在格子里点一张图片".to_string();
                    }
                    drop(g);
                    if !stale {
                        sim_previews(hwnd);
                    }
                    refresh(hwnd);
                }
            }
        }
        M_EDITOR => {
            let p = app().lock().unwrap().selected_paths().into_iter().next();
            if let Some(p) = p {
                launch_editor(hwnd, &p);
            } else {
                app().lock().unwrap().msg = "没有选中文件".to_string();
                refresh(hwnd);
            }
        }
        M_BATCH => open_dlg(hwnd, DlgKind::BatchRename),
        M_DBBACKUP => {
            let stamp = unsafe { stamp_now() };
            let def = format!("XseeBK_{stamp}.zip");
            let d = db_dir();
            let init = Some(d.as_path());
            if let Some(p) = browse_for_file(hwnd, "导出数据库", true, &def, init) {
                start_db_task(hwnd, 1, Some(p));
            }
        }
        M_DBIMPORT => {
            // 先选目录、再确认；确认框点"是"会重发这条命令，这次才真导入。
            let go = {
                let mut g = app().lock().unwrap();
                let c = g.ask_go == M_DBIMPORT;
                if c {
                    g.ask_go = 0;
                }
                c
            };
            if go {
                let src = app().lock().unwrap().db_import_src.clone();
                match src {
                    Some(p) => start_db_task(hwnd, 2, Some(p)),
                    None => {
                        app().lock().unwrap().msg = "没有选择要导入的数据库".to_string();
                        refresh(hwnd);
                    }
                }
            } else {
                // 从库所在的那个目录起手：备份包多半就压在那儿旁边。
                let d = db_dir();
                let Some(pkg) =
                    browse_for_file(hwnd, "选择要导入的数据库压缩包", false, "", Some(d.as_path()))
                else {
                    return;
                };
                app().lock().unwrap().db_import_src = Some(pkg.clone());
                let body = format!(
                    "导入会用\n{}\n这个压缩包里的数据库覆盖当前数据库，不是合并。\n\n\
                     导入前会自动把当前数据库完整备份成压缩包。\n\n要继续吗？",
                    pkg.display()
                );
                open_ask(hwnd, "导入数据库", &body, true, M_DBIMPORT);
            }
        }
        M_EXTAPP => {
            let p = app().lock().unwrap().selected_paths().into_iter().next();
            if let Some(p) = p {
                let _ = std::process::Command::new("rundll32.exe")
                    .arg("shell32.dll,OpenAs_RunDLL")
                    .arg(&p)
                    .spawn();
            } else {
                app().lock().unwrap().msg = "没有选中文件".to_string();
                refresh(hwnd);
            }
        }
        M_WALL => set_wallpaper(hwnd, 0),
        M_CAT_NEW => open_dlg(hwnd, DlgKind::NewCat),
        M_CAT_COPY => {
            let snap = {
                let g = app().lock().unwrap();
                g.cat.as_ref().and_then(|c| c.snapshot(g.org_sel))
            };
            match snap {
                Some(s) => {
                    let name = s.name.clone();
                    *CAT_CLIP.lock().unwrap() = Some(s);
                    app().lock().unwrap().msg = format!("已复制类别「{name}」");
                }
                None => app().lock().unwrap().msg = "先在类别行上点右键".to_string(),
            }
            refresh(hwnd);
        }
        M_CAT_PASTE => {
            let Some(snap) = CAT_CLIP.lock().unwrap().clone() else {
                app().lock().unwrap().msg = "先复制一个类别".to_string();
                refresh(hwnd);
                return;
            };
            let (msg, new_id, parent) = {
                let mut g = app().lock().unwrap();
                let parent = g.org_sel;
                match g.cat.as_mut() {
                    Some(cat) => {
                        let id = cat.paste_cat(parent, &snap);
                        let name = cat.name(id);
                        let msg = match cat.save_full(&catalog_file()) {
                            Ok(()) => format!("已粘贴类别「{name}」"),
                            Err(e) => format!("类别已粘贴，但保存失败：{e}"),
                        };
                        (msg, id, parent)
                    }
                    None => ("分类数据库还没载入".to_string(), 0, 0),
                }
            };
            {
                let mut g = app().lock().unwrap();
                g.msg = msg;
                if new_id != 0 {
                    g.org_sel = new_id;
                    if parent != 0 {
                        g.org_open_ids.insert(parent);
                    }
                }
            }
            refresh(hwnd);
        }
        M_CAT_DEL => {
            // 选中和确认都在 command() 开头做过，这里只管执行
            let id = app().lock().unwrap().org_sel;
            let (msg, gone, cur) = {
                let mut g = app().lock().unwrap();
                let cur = g.src;
                match g.cat.as_mut() {
                    Some(cat) => {
                        let name = cat.name(id);
                        let ids = cat.del_cat(id);
                        let msg = if ids.is_empty() {
                            "类别不存在".to_string()
                        } else {
                            match cat.save_full(&catalog_file()) {
                                Ok(()) => format!("已删除类别「{name}」（图片文件没动）"),
                                Err(e) => format!("类别已删除，但保存失败：{e}"),
                            }
                        };
                        (msg, ids, cur)
                    }
                    None => ("分类数据库还没载入".to_string(), Vec::new(), cur),
                }
            };
            {
                let mut g = app().lock().unwrap();
                g.msg = msg;
                if !gone.is_empty() {
                    g.org_open_ids.retain(|x| !gone.contains(x));
                    g.org_sel = 0;
                    if let Src::Category(cid) = cur {
                        if gone.contains(&cid) {
                            g.load_category(0, hwnd);
                        }
                    }
                }
            }
            refresh(hwnd);
        }
        M_CAT_RENAME => {
            let id = app().lock().unwrap().org_sel;
            if id == 0 {
                app().lock().unwrap().msg = "先选一个类别".to_string();
                refresh(hwnd);
            } else {
                open_dlg(hwnd, DlgKind::RenameCat);
            }
        }
        M_ORG | M_ORG_CLOSE => {
            let mut g = app().lock().unwrap();
            if id == M_ORG {
                if !g.org_open {
                    g.org_open = true;
                    g.dock_tab = 0;
                } else if g.conv_open && g.dock_tab == 1 {
                    // 两页都开着时，这一项先把人送回整理页，再点一次才关。
                    g.dock_tab = 0;
                } else {
                    g.org_open = false;
                    if g.conv_open {
                        g.dock_tab = 1;
                    }
                }
            } else {
                g.org_open = false;
                if g.conv_open {
                    g.dock_tab = 1;
                }
            }
            // 两页都关掉才把自动加宽还回去。
            if !g.org_open && !g.conv_open && g.conv_w_auto {
                g.org_w = g.conv_w_prev;
                g.conv_w_auto = false;
            }
            g.feed(hwnd);
            drop(g);
            refresh(hwnd);
        }
        M_CONV_PANEL | M_CONV_PANEL_CLOSE => {
            let open = app().lock().unwrap().conv_open;
            if id == M_CONV_PANEL && !open {
                open_conv_panel(hwnd);
            } else {
                close_conv_panel(hwnd);
            }
        }
        // 窗格开关：窗格头那个 ✕ 和查看菜单里那一项是同一个 id，一处口径
        M_PANE_TREE | M_PANE_PV => {
            let mut g = app().lock().unwrap();
            if id == M_PANE_TREE {
                g.pane_tree = !g.pane_tree;
                if !g.pane_tree {
                    // 树收起来时键盘焦点不能再在树上，不然方向键按了没反应。
                    g.tree_kb = false;
                }
            } else {
                g.pane_pv = !g.pane_pv;
            }
            g.feed(hwnd);
            drop(g);
            refresh(hwnd);
        }
        M_TB_TEXT | M_TB_NOTEXT | M_TB_RIGHT | M_TB_BELOW => {
            let mut g = app().lock().unwrap();
            g.tb_text = id != M_TB_NOTEXT;
            match id {
                M_TB_RIGHT => g.tb_below = false,
                M_TB_BELOW => g.tb_below = true,
                _ => {}
            }
            drop(g);
            refresh(hwnd);
        }
        M_TB_HIDE | M_BAR_PATH | M_BAR_STATUS => {
            let mut g = app().lock().unwrap();
            match id {
                M_TB_HIDE => g.bar_main = !g.bar_main,
                M_BAR_PATH => g.bar_path = !g.bar_path,
                _ => g.bar_status = !g.bar_status,
            }
            g.feed(hwnd);
            drop(g);
            refresh(hwnd);
        }
        M_PAGE_PREV | M_PAGE_NEXT => {
            let mut g = app().lock().unwrap();
            let pages = g.page_count();
            let want = if id == M_PAGE_NEXT { g.page + 1 } else { g.page.saturating_sub(1) };
            if want < pages && want != g.page {
                g.page = want;
                g.show_page(hwnd);
            }
            drop(g);
            request_pv(hwnd);
        }
        M_LOOK | M_EDIT | M_ADDR => open_bmenu(hwnd, id),
        i if i == M_WALL + 1 => set_wallpaper(hwnd, 1),
        i if i == M_WALL + 2 => set_wallpaper(hwnd, 2),
        M_ABOUT => {
            let n = theme::skins().len();
            let ver = env!("CARGO_PKG_VERSION");
            open_ask(
                hwnd,
                "关于 Xsee",
                &format!(
                    "星雨图片查看器（Xsee）· 版本 {ver}\n\
                     作者：星雨文成\n\
                     用 Rust 写的本地图片浏览与整理工具，窗口全部自绘。\n\
                     \n\
                     · 浏览：文件夹树 + 缩略图四档视图，双击看大图\n\
                     · 整理：分类库、以图搜图、批量重命名、格式转换、文件关联\n\
                     · 外观：皮肤系统（当前 {n} 套），配色和图标都能换\n\
                     · 格式：JPG PNG BMP GIF TIFF ICO WebP HEIC PSD 与常见 RAW\n\
                     \n\
                     致谢\n\
                     开发中借鉴了三个开源项目，在此一并致谢：\n\
                     · 西西图 SEESEE（作者 WUHAO）——浏览器形态、解码与缩略图缓存的做法\n\
                     · ImageSearch（作者 ldqk，MIT 许可）——以图搜图的感知哈希 + 汉明距离思路\n\
                     · nomacs 3.22.4（github.com/nomacs/nomacs，GPL-3.0）——图像编辑的像素运算与参数口径"
                ),
                false,
                0,
            );
        }
        i if i >= M_SKIN && i - M_SKIN < theme::skins().len() => {
            let names = theme::skins();
            // 悬停预览可能已经把这套换上了，g.skin 看着没变；"点中"仍要落盘，
            // 也仍要补一次完整重排（预览那趟故意没重启缩略图池）。
            let previewed = MENU_PREV.swap(false, Ordering::SeqCst);
            *MENU_SNAP.lock().unwrap() = None;
            if let Some(n) = names.get(i - M_SKIN).cloned() {
                let mut g = app().lock().unwrap();
                if g.skin != n || previewed {
                    g.skin = n;
                    g.look = theme::Look::load(&g.skin);
                    g.restyle(hwnd);
                }
                ui_put_str("skin", &g.skin);
            }
        }
        i if i > M_LAYOUT && i < M_LAYOUT + 1 + glyphs::all().len() => {
            let key = glyphs::all()[i - M_LAYOUT - 1].key;
            let previewed = MENU_PREV.swap(false, Ordering::SeqCst);
            *MENU_SNAP.lock().unwrap() = None;
            {
                let mut g = app().lock().unwrap();
                if g.layout.as_deref() != Some(key) || previewed {
                    g.layout = Some(key.to_string());
                    g.restyle(hwnd);
                }
            }
            theme::save_layout(Some(key));
        }
        i if i >= M_DENS && i < M_DENS + 3 => {
            let d = DENS[i - M_DENS];
            let mut g = app().lock().unwrap();
            if g.density != d {
                g.density = d.to_string();
                g.restyle(hwnd);
            }
        }
        _ => {}
    }
    check_menu(hwnd);
    // 菜单和工具栏上能点的东西全走这一趟：末尾把可记的数整体存一次，
    // 攒到关窗才写的话，崩一次或断电一次就全丢。
    ui_save(&app().lock().unwrap(), hwnd);
}

/// 换视图：尺寸档、解码框、菜单勾选三处得一起动，漏一处就是菜单和画面互相对不上。
unsafe fn set_mode(hwnd: HWND, m: Mode) {
    let mut g = app().lock().unwrap();
    if g.mode != m {
        let rescope = g.mode.all_files() != m.all_files();
        g.mode = m;
        if rescope {
            // 缩略图档只列图片，其余三档连非图片一起列：文件范围变了要重列目录
            let d = g.dir.display().to_string();
            let keep = g.entries.get(g.sel).map(|e| e.path.clone());
            // goto 会清选区，但换视图不是换目录，原来框住的那几张切过去还得是框住的。
            let keep_marked = g.marked.clone();
            g.goto(&d, hwnd, false);
            g.marked = keep_marked;
            if let Some(p) = keep {
                if let Some(i) = g.entries.iter().position(|e| e.path == p) {
                    g.set_sel(i);
                }
            }
        } else {
            g.restart_pool();
        }
    }
    drop(g);
    done_view(hwnd);
    request_pv(hwnd);
}

unsafe fn done_view(hwnd: HWND) {
    app().lock().unwrap().feed(hwnd);
    check_menu(hwnd);
    refresh(hwnd);
}

/// 缩略图高度加减一档：路径栏那对 −/+ 和设置页里那对走的是同一条。
/// 到下限就停住——那条线（100 像素）是为了不跌破以图搜图认得出的短边。
unsafe fn nudge_thumb(hwnd: HWND, up: bool) {
    let step = if up { THUMB_H_STEP } else { -THUMB_H_STEP };
    {
        let mut g = app().lock().unwrap();
        g.thumb_h = (g.thumb_h + step).clamp(THUMB_H_MIN, THUMB_H_MAX);
        g.restart_pool();
    }
    done_view(hwnd);
}

/// Shift+F10 / 菜单键：在键盘焦点那一项上弹同一个右键菜单。
/// 焦点在树上就落焦点行，在格子里就落光标那一格；都没有时贴格子中间。
unsafe fn ctx_menu_key(hwnd: HWND) {
    let (z, _) = layout(hwnd);
    let (tree_kb, tree_path, cell, img) = {
        let g = app().lock().unwrap();
        if g.tree_kb {
            (true, g.tree_focus_dir(), None, None)
        } else if g.sel != usize::MAX {
            let r = g.name_rect(g.sel, z.grid);
            let e = g.entries.get(g.sel);
            (
                false,
                None,
                Some(r),
                e.filter(|e| !e.is_dir)
                    .map(|e| e.path.clone())
                    .filter(|p| scan::is_image(p)),
            )
        } else {
            (false, None, None, None)
        }
    };
    // “此电脑”这类行没有文件落点，右键本来也不弹，键盘这边保持一致。
    if tree_kb && tree_path.is_none() {
        return;
    }
    let blank = !tree_kb && cell.is_none();
    let (x, y, ctx_dir) = if let Some(p) = tree_path {
        let Some(r) = tree_row_rect(hwnd, &p) else { return };
        (r.left + 2, (r.top + r.bottom) / 2, Some(p))
    } else if let Some(r) = cell.filter(|r| {
        r.top >= z.grid.top && r.bottom <= z.grid.bottom && r.left >= z.grid.left && r.right <= z.grid.right
    }) {
        ((r.left + r.right) / 2, (r.top + r.bottom) / 2, None)
    } else {
        (
            (z.grid.left + z.grid.right) / 2,
            (z.grid.top + z.grid.bottom) / 2,
            None,
        )
    };
    let mut g = app().lock().unwrap();
    g.open_menu = None;
    g.open_sub = None;
    g.ctx_at = Some((x, y));
    g.ctx_sub = None;
    g.ctx_tree = ctx_dir.is_some();
    g.ctx_dir = ctx_dir;
    g.ctx_img = img;
    g.ctx_blank = blank;
    g.ctx_rows.clear();
    g.ctx_sub_rows.clear();
    g.hover = usize::MAX;
    g.hover_btn = None;
    g.hover_menu = None;
    drop(g);
    refresh(hwnd);
}

unsafe fn ctx_menu(hwnd: HWND, lparam: LPARAM) {
    let x = (lparam.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    let (z, _) = layout(hwnd);
    // 右键也要跟着那一条非模态的页放行，否则这页开着就没法对着结果操作文件。
    {
        let g = app().lock().unwrap();
        if g.dlg.is_some() && !sim_loose(&g, x, y) {
            return;
        }
    }
    // 整理面板这一整块都归类别菜单：点在哪个类别行就把当前类别挪到哪，
    // 点在下边的空白处不动当前类别、照样弹菜单（转换页占着这一块时不算，那是另一页）。
    if over(&z.org, x, y) {
        let (hit, conv_page) = {
            let g = app().lock().unwrap();
            (
                g.org_hits.iter().find(|(_, r)| over(r, x, y)).map(|(i, _)| *i),
                g.conv_open && (g.dock_tab == 1 || !g.org_open),
            )
        };
        if conv_page {
            return;
        }
        let mut g = app().lock().unwrap();
        if let Some(id) = hit {
            g.org_sel = id;
        }
        g.open_menu = None;
        g.open_sub = None;
        g.ctx_at = None;
        g.ctx_sub = None;
        g.ctx_tree = false;
        g.ctx_cat = Some((x, y));
        g.ctx_dir = None;
        g.ctx_img = None;
        g.ctx_blank = true;
        g.hover = usize::MAX;
        g.hover_btn = None;
        g.hover_menu = None;
        drop(g);
        refresh(hwnd);
        return;
    }
    {
        let mut g = app().lock().unwrap();
        // 右键落在文件区，键盘焦点也收到这边来：不然树上还亮着，接下来的
        // 复制/删除会按“树落点”认走树上那一行，而不是刚右键的这张图。
        if over(&z.grid, x, y) {
            g.tree_kb = false;
        }
        let h = g.hit(z.grid, x, y);
        if h != usize::MAX {
            g.set_sel(h);
            // 右键落在哪个格子就把那张图记下来：以图搜图从右键进来时直接认它，
            // 不经过全局选区，菜单开关之间选区被谁动了都不碍事。
            g.ctx_img = g
                .entries
                .get(h)
                .filter(|e| !e.is_dir)
                .map(|e| e.path.clone())
                .filter(|p| scan::is_image(p));
            g.ctx_blank = false;
            drop(g);
            request_pv(hwnd);
        } else {
            g.ctx_img = None;
            g.ctx_blank = true;
        }
    };
    // 树上右键：那一行按单击同一条路先选中再切过去，粘贴/新建文件夹才有
    // 落点，复制/删除/改名按文件夹本体算（命令那头看 ctx_dir 认）。
    // 标题条、滚动条、“此电脑”和行下面的空白都不弹这张菜单：它们没有文件层面的
    // 落点，硬弹出来只会拿格子里的选区去下手。
    let mut ctx_dir = None;
    if over(&z.tree, x, y) {
        let mut rows = Vec::new();
        let (rh, head, tscroll, sb) = {
            let g = app().lock().unwrap();
            flat_tree(&g.tree, 0, &mut rows);
            let hr = g.th.row.max(1);
            let hb = z.tree.top + g.th.pane_head_h;
            (hr, hb, g.tscroll, tree_sb(z.tree, hb, rows.len(), hr, g.tscroll, g.scale))
        };
        let on_sb = sb.is_some_and(|(track, _, _)| x >= track.left);
        let ki = ((y - head + tscroll) / rh).max(0) as usize;
        let row = if y >= head && !on_sb { rows.get(ki).cloned() } else { None };
        let Some(path) = row.map(|(_, p, _, _)| p).filter(|p| p != "此电脑") else {
            let mut g = app().lock().unwrap();
            g.ctx_at = None;
            g.ctx_sub = None;
            g.ctx_dir = None;
            g.ctx_img = None;
            g.ctx_blank = true;
            drop(g);
            refresh(hwnd);
            return;
        };
        {
            let mut g = app().lock().unwrap();
            g.tree_kb = true;
            g.tsel = path.clone();
            g.tree_noscroll = true;
            g.ctx_img = None;
            g.ctx_blank = true;
            g.goto(&path, hwnd, true);
        }
        // 树行的路径带着结尾反斜杠；存落点前摘掉（盘根的 "D:\" 原样留着），
        // 否则取上级、属性里的位置、剪贴板条目都会偏一层。
        ctx_dir = Some(if path.len() > 3 {
            path.trim_end_matches('\\').to_string()
        } else {
            path
        });
    }
    // 条目照逆向出来的文件列表右键菜单（#20111）排，画法跟顶部菜单的弹层一致
    {
        let mut g = app().lock().unwrap();
        g.open_menu = None;
        g.open_sub = None;
        g.ctx_at = Some((x, y));
        g.ctx_sub = None;
        g.ctx_tree = ctx_dir.is_some();
        g.ctx_dir = ctx_dir;
        g.ctx_rows.clear();
        g.ctx_sub_rows.clear();
        g.hover = usize::MAX;
        g.hover_btn = None;
        g.hover_menu = None;
    }
    refresh(hwnd);
}

unsafe fn check_menu(hwnd: HWND) {
    let hm = GetMenu(hwnd);
    if hm.is_invalid() {
        return;
    }
    let v = GetSubMenu(hm, 2);
    let (mode, skin, dens) = {
        let g = app().lock().unwrap();
        (g.mode, g.skin.clone(), g.density.clone())
    };
    for (i, m) in [Mode::Grid, Mode::Small, Mode::List, Mode::Detail].iter().enumerate() {
        let on = if *m == mode { MF_CHECKED } else { MF_UNCHECKED };
        let _ = CheckMenuItem(v, (M_VGRID + i) as u32, (on | MF_BYCOMMAND).0);
    }
    let sk = GetSubMenu(v, 6);
    // 按子目录分过组的皮肤挂在二级菜单里，勾选状态要在它自己那层设。
    let mut menus = vec![sk];
    for k in 0..GetMenuItemCount(Some(sk)).max(0) {
        menus.push(GetSubMenu(sk, k));
    }
    for (i, n) in theme::skins().iter().enumerate() {
        let on = if *n == skin { MF_CHECKED } else { MF_UNCHECKED };
        for m in &menus {
            let _ = CheckMenuItem(*m, (M_SKIN + i) as u32, (on | MF_BYCOMMAND).0);
        }
    }
    let dn = GetSubMenu(v, 7);
    for (i, n) in ["compact", "normal", "loose"].iter().enumerate() {
        let on = if *n == dens { MF_CHECKED } else { MF_UNCHECKED };
        let _ = CheckMenuItem(dn, (M_DENS + i) as u32, (on | MF_BYCOMMAND).0);
    }
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

unsafe fn relayout(hwnd: HWND) {
    let dpi = GetDpiForWindow(hwnd);
    let scale = dpi as f64 / 96.0;
    let mut g = app().lock().unwrap();
    g.scale = scale;
    g.th = Theme::new_with_layout(&g.skin, &g.density, scale, &g.look, g.layout.as_deref());
    // 查看器画自己那套界面，用的必须是同一份皮肤和同一套尺寸
    xsee_host::viewer::set_theme(&g.th);
    // 栏宽和预览高：0 是"还没人定过"（第一次运行，或预览块从没拖过），那才给按缩放
    // 算的默认值。记回来的和当场拖出来的都不在换缩放时改动，否则调一次 DPI 摆位就没了。
    let min_l = (110.0 * scale) as i32;
    let min_o = (170.0 * scale) as i32;
    g.left_w = if g.left_w == 0 { (300.0 * scale).round() as i32 } else { g.left_w.max(min_l) };
    g.org_w = if g.org_w == 0 {
        (if g.conv_open { 300.0 } else { 250.0 } * scale).round() as i32
    } else {
        g.org_w.max(min_o)
    };
    g.pv_w = 0;
    g.refresh_visual();
    g.restart_pool();
}

/// 指针底下那个能接文件的文件夹：列表条目或左边目录树里的真实目录。
/// 指针底下是不是一个能收文件的文件夹，落在哪一格。
unsafe fn drop_hit(hwnd: HWND, x: i32, y: i32) -> Option<Hl> {
    let (z, _) = layout(hwnd);
    let g = app().lock().unwrap();
    if over(&z.tree, x, y) {
        let mut rows = Vec::new();
        flat_tree(&g.tree, 0, &mut rows);
        let rh = g.th.row.max(1);
        let head = z.tree.top + g.th.pane_head_h;
        let k = ((y - head + g.tscroll) / rh).max(0) as usize;
        if let Some((_, path, _, _)) = rows.get(k) {
            if path != "此电脑" && Path::new(path).is_dir() {
                return Some(Hl::Tree(k));
            }
        }
        return None;
    }
    if over(&z.grid, x, y) {
        let hit = g.hit(z.grid, x, y);
        if hit != usize::MAX && g.entries.get(hit).map(|e| e.is_dir).unwrap_or(false) {
            return Some(Hl::Grid(hit));
        }
        // 文件夹视图里，列表区不管压在格子上还是缝里，落点都是"当前这个文件夹"
        if matches!(g.src, Src::Folder) {
            return Some(Hl::Here);
        }
    }
    None
}

unsafe fn drop_target_dir(hwnd: HWND, x: i32, y: i32) -> Option<String> {
    let hl = drop_hit(hwnd, x, y)?;
    let g = app().lock().unwrap();
    match hl {
        Hl::Grid(i) => g.entries.get(i).map(|e| e.path.clone()),
        Hl::Here => Some(g.dir.display().to_string()),
        Hl::Tree(k) => {
            let mut rows = Vec::new();
            flat_tree(&g.tree, 0, &mut rows);
            rows.get(k).map(|(_, p, _, _)| p.clone())
        }
    }
}

fn drop_is_folder(hwnd: HWND, x: i32, y: i32) -> bool {
    unsafe { drop_hit(hwnd, x, y).is_some() }
}

/// 往外拖的过程中，Ole 每次指针移动都会问一遍：把落点画出来或擦掉。
fn drop_hover(hwnd: HWND, at: Option<(i32, i32)>) {
    unsafe {
        let hl = at.and_then(|(x, y)| drop_hit(hwnd, x, y));
        {
            let mut g = app().lock().unwrap();
            if g.drop_hl == hl {
                return;
            }
            g.drop_hl = hl;
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}
/// 拖进来的东西：落在文件夹上就搬进去，落在空白处直接打开。
fn drop_received(hwnd: HWND, paths: Vec<String>, x: i32, y: i32, copy: bool) {
    unsafe {
        let mine = SELF_DRAG.load(Ordering::SeqCst);
        if let Some(dir) = drop_target_dir(hwnd, x, y) {
            // 自己拖出来又放回它原来那个文件夹：等于原地没动，不记成搬运
            let back = paths.iter().all(|p| {
                Path::new(p)
                    .parent()
                    .map(|d| d.display().to_string())
                    .unwrap_or_default()
                    == dir
            });
            if !back {
                transfer_to_folder(hwnd, paths, dir, copy);
            }
            return;
        }
        // 从自己窗口拖起来、随手松手的，什么都不做
        if mine {
            return;
        }
        if paths.len() == 1 && Path::new(&paths[0]).is_dir() {
            app().lock().unwrap().goto(&paths[0], hwnd, true);
            request_pv(hwnd);
            refresh(hwnd);
            return;
        }
        let imgs: Vec<String> = paths
            .iter()
            .filter(|p| scan::is_image(p))
            .cloned()
            .collect();
        if imgs.is_empty() {
            app().lock().unwrap().msg = "拖进来的东西里没有能显示的图片".to_string();
            refresh(hwnd);
        } else {
            let first = imgs[0].clone();
            xsee_host::viewer::switch_to(&first, Some(imgs));
        }
    }
}

/// 真正的搬运交给 shell，跨盘、文件夹、撤销都按系统规则来；界面线程不等。
fn transfer_to_folder(hwnd: HWND, paths: Vec<String>, dir: String, copy: bool) {
    let hp = hwnd.0 as isize;
    std::thread::spawn(move || {
        use windows::Win32::Foundation::HWND as H;
        let hwnd = H(hp as *mut core::ffi::c_void);
        unsafe {
            use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let mut pairs: Vec<(String, String)> = Vec::new();
        for p in &paths {
            let src = Path::new(p);
            let Some(name) = src.file_name().map(|n| n.to_string_lossy().to_string()) else {
                continue;
            };
            // 只有"复制到自己待着的那个文件夹"才直接落副本；其余撞名交给系统弹框
            let dst_name = if copy && clip::same_folder(p, &dir) {
                clip::copy_name(Path::new(&dir), &name)
            } else {
                name.clone()
            };
            let dst = Path::new(&dir).join(dst_name);
            pairs.push((p.clone(), dst.display().to_string()));
        }
        // 一个一个落：pTo 给完整目标路径才落得出我们算好的名字（见 clip::shell_move）
        let mut done = 0usize;
        let mut fail = 0usize;
        let mut err = String::new();
        for (src, dst) in &pairs {
            match clip::shell_move(src, dst, !copy) {
                Ok(()) => done += 1,
                Err(e) => {
                    fail += 1;
                    if err.is_empty() {
                        err = e;
                    }
                }
            }
        }
        *DROP_ERR.lock().unwrap() = err;
        // 动过的目录：目标那层 + 每个源的上层。树里这些节点缓存着子项，
        // 不重扫的话搬走的文件夹还钉在老位置、搬进来的那个也不出现。
        {
            let mut v = DROP_DIRS.lock().unwrap();
            v.push(dir.clone());
            for p in &paths {
                if let Some(par) = Path::new(p).parent() {
                    v.push(par.display().to_string());
                }
            }
        }
        // 真的搬过去的：源已经不在、目标已经在了。缩略图按旧图直接挂到新路径，
        // 比到新目录重解码快得多；分类库的路径由界面线程改（那份数据在主线程）。
        let moved: Vec<(String, String)> = pairs
            .into_iter()
            .filter(|(old, new)| !Path::new(old).exists() && Path::new(new).exists())
            .collect();
        for (old, new) in &moved {
            cache::move_path(old, new);
            sim::moved(old, new);
        }
        if !copy && !moved.is_empty() {
            MOVED_IMGS.lock().unwrap().extend(moved);
        }
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_DROP_DONE, WPARAM(done), LPARAM(fail as isize));
            windows::Win32::System::Com::CoUninitialize();
        }
    });
}

/// 看图态就画在本窗口里：换一张脸，不再开第二个窗口。
static FULL: AtomicBool = AtomicBool::new(false);
static SAVE_RECT: Mutex<RECT> = Mutex::new(RECT {
    left: 0,
    top: 0,
    right: 0,
    bottom: 0,
});
static SAVE_STYLE: AtomicIsize = AtomicIsize::new(0);

/// 退出大图态时把标题换回浏览那一条。
unsafe fn browse_title(hwnd: HWND) {
    let d = app().lock().unwrap().dir.display().to_string();
    let _ = SetWindowTextW(hwnd, &HSTRING::from(format!("Xsee · 浏览 — {d}")));
}

/// 中键：在"铺满显示器"和"原来那个窗口"之间来回。只有大图态用得到。
unsafe fn toggle_full(hwnd: HWND) {
    if !FULL.swap(true, Ordering::SeqCst) {
        let mut r = RECT::default();
        let _ = GetWindowRect(hwnd, &mut r);
        *SAVE_RECT.lock().unwrap() = r;
        SAVE_STYLE.store(GetWindowLongPtrW(hwnd, GWL_STYLE), Ordering::SeqCst);
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(mon, &mut mi).as_bool() {
            let _ = SetWindowLongPtrW(
                hwnd,
                GWL_STYLE,
                (WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN).0 as isize,
            );
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOP),
                mi.rcMonitor.left,
                mi.rcMonitor.top,
                mi.rcMonitor.right - mi.rcMonitor.left,
                mi.rcMonitor.bottom - mi.rcMonitor.top,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            );
            xsee_host::viewer::set_titled(false);
            xsee_host::viewer::enter_full();
        } else {
            FULL.store(false, Ordering::SeqCst);
        }
    } else {
        let style = SAVE_STYLE.load(Ordering::SeqCst);
        // 落位这几步全程冻住重绘。以前是先写回样式、再让系统按新边框重算一次、
        // 最后 ShowWindow 还原+最大化两跳；每一跳系统都把窗口摆到新位置去画，
        // 而我们那一帧还没画出来，窗口刚挪走的那块露出来的就是底下的桌面。
        let _ = SendMessageW(hwnd, WM_SETREDRAW, Some(WPARAM(0)), Some(LPARAM(0)));
        if style != 0 {
            let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, style);
        }
        // 边框和位置在同一次调用里落定：样式里带着 WS_MAXIMIZE 一起写回，
        // 系统认的还是"最大化"，摆回进全屏前量到的那个矩形就是原来那一屏，
        // 不需要再靠还原+最大化两跳去凑（那一跳会把窗口甩到别处再跳回来）。
        let r = *SAVE_RECT.lock().unwrap();
        let _ = SetWindowPos(
            hwnd,
            None,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        FULL.store(false, Ordering::SeqCst);
        xsee_host::viewer::set_titled(true);
        xsee_host::viewer::exit_full();
        let _ = SendMessageW(hwnd, WM_SETREDRAW, Some(WPARAM(1)), Some(LPARAM(0)));
        // 解冻后必须自己催一次同步重绘：这段期间攒下的无效区系统不会补发，不催就空在那儿。
        // 催之前先把视图状态还回去，这样第一帧画出来的就是最终那一屏，不会先看见一帧全屏排布。
        let _ = RedrawWindow(
            Some(hwnd),
            None,
            None,
            RDW_INVALIDATE | RDW_FRAME | RDW_UPDATENOW | RDW_ALLCHILDREN,
        );
    }
    xsee_host::viewer::resize(client(hwnd));
    refresh(hwnd);
}

/// 查看器按钮要办的事，交给浏览窗口原有的命令去做；第二个值是要不要先退回浏览态。
fn cmd_of(c: xsee_host::viewer::Cmd) -> (usize, bool) {
    use xsee_host::viewer::Cmd::*;
    match c {
        Open => (M_OPEN, false),
        Explore => (M_EXPL, true),
        Edit => (M_EDIT, false),
        MoveTo => (M_MOVE, false),
        CopyTo => (M_COPY, false),
        Delete => (M_DEL, true),
        Prop => (M_PROP, false),
        Up => (M_UP, true),
        Look => (M_LOOK, true),
        List => (M_VLIST, true),
        Detail => (M_VDETAIL, true),
        Browse => (0, true),
        Slide => (0, false),
    }
}

/// 退出大图：全屏态先还原窗口，再回到浏览界面。
unsafe fn leave_viewer(hwnd: HWND) {
    xsee_host::viewer::close();
    browse_title(hwnd);
    if FULL.load(Ordering::SeqCst) {
        toggle_full(hwnd);
    } else {
        refresh(hwnd);
    }
}

unsafe fn view_act(hwnd: HWND, a: xsee_host::viewer::Act) {
    use xsee_host::viewer::Act as A;
    match a {
        A::Nothing => {}
        A::Leave => {
            leave_viewer(hwnd);
        }
        A::Full => toggle_full(hwnd),
        A::Do(c) => {
            let (id, leave) = cmd_of(c);
            if leave {
                leave_viewer(hwnd);
            }
            if id != 0 {
                command(hwnd, id);
            } else if c == xsee_host::viewer::Cmd::Slide {
                app().lock().unwrap().msg = "幻灯显示还没做".to_string();
            }
            refresh(hwnd);
        }
    }
}

/// 大图态下，输入先归查看器；返回 Some 表示这条消息已经处理完。
unsafe fn view_input(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<LRESULT> {
    use xsee_host::viewer as v;
    if msg == v::WM_VCLEAR {
        let i = wparam.0 as usize;
        if v::clear_msg(i) {
            refresh(hwnd);
        }
        return Some(LRESULT(0));
    }
    if msg == v::WM_VQUICK {
        let i = wparam.0 as usize;
        if v::quick_msg(i) {
            refresh(hwnd);
        }
        return Some(LRESULT(0));
    }
    if !v::is_open() {
        return None;
    }
    if msg == WM_MOUSEMOVE {
        // 要等鼠标真的出窗口：全屏的箭头靠这条消息开始两秒倒计时
        let mut t = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        let _ = TrackMouseEvent(&mut t);
    }
    if msg == WM_MOUSELEAVE {
        v::mouse_leave();
        return None;
    }
    let x = (lparam.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    match msg {
        WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_MBUTTONDOWN
        | WM_RBUTTONUP | WM_MOUSEWHEEL => {
            if y < v::title_h() {
                // 顶上标题栏不归查看器：按钮点击和悬停高亮要走宿主原来那条路。
                return None;
            }
            let a = v::mouse(msg, wparam, x, y);
            view_act(hwnd, a);
            Some(LRESULT(0))
        }
        WM_KEYDOWN => {
            view_act(hwnd, v::key(wparam.0 as u32));
            Some(LRESULT(0))
        }
        WM_RBUTTONDOWN | WM_CHAR => Some(LRESULT(0)),
        _ => None,
    }
}

/// 看图就在本窗口里换一张脸。文件表一起递过去，查看器不用再扫一遍目录。
unsafe fn spawn_viewer(p: &Path, hwnd: HWND) {
    let ps = p.to_string_lossy().to_string();
    let is_dir = p.is_dir();
    if !is_dir && !scan::is_image(&ps) {
        use windows::Win32::UI::Shell::ShellExecuteW;
        let op = HSTRING::from("open");
        let file = HSTRING::from(ps.as_str());
        let r = ShellExecuteW(Some(hwnd), &op, &file, None, None, SW_SHOWNORMAL);
        if r.0 as isize <= 32 {
            app().lock().unwrap().msg = format!("系统没有能打开 {} 的程序", p.display());
            refresh(hwnd);
        }
        return;
    }
    let files: Vec<String> = {
        let g = app().lock().unwrap();
        g.entries
            .iter()
            .filter(|e| !e.is_dir && e.is_image)
            .map(|e| e.path.clone())
            .collect()
    };
    xsee_host::viewer::open(&ps, Some(files).filter(|f| !f.is_empty()));
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if let Some(r) = view_input(hwnd, msg, wparam, lparam) {
        return r;
    }
    match msg {
        // 菜单开着的时候，鼠标停在皮肤/版式项上就当场换给他看；关菜单没点中再还回去。
        WM_ENTERMENULOOP => {
            if MENU_DEPTH.fetch_add(1, Ordering::SeqCst) == 0 {
                let g = app().lock().unwrap();
                *MENU_SNAP.lock().unwrap() =
                    Some((g.skin.clone(), g.look.clone(), g.layout.clone()));
                MENU_PREV.store(false, Ordering::SeqCst);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_EXITMENULOOP => {
            if MENU_DEPTH.load(Ordering::SeqCst) > 0
                && MENU_DEPTH.fetch_sub(1, Ordering::SeqCst) == 1
            {
                menu_restore(hwnd);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MENUSELECT => {
            let id = (wparam.0 & 0xffff) as usize;
            let flags = ((wparam.0 >> 16) & 0xffff) as u32;
            if id != 0xffff && flags & MF_POPUP.0 == 0 {
                menu_preview(hwnd, id);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_PAINT => {
            paint(hwnd);
            // 改名框是挂在格子上的一件真控件：这一帧画完就把它挪到名字现在待的位置。
            place_rename(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        // 带 WS_THICKFRAME 的窗口，系统还会默认描一圈非客户区边框：激活时左上有
        // 白高光、右下深灰阴影，失焦时整圈变浅。客户区已经铺满窗口，这圈是多余的。
        WM_NCPAINT => LRESULT(0),
        WM_NCACTIVATE => LRESULT(1),
        // 切到别的程序时把右键弹层收起来
        WM_ACTIVATE => {
            if (wparam.0 & 0xffff) == 0 {
                let mut g = app().lock().unwrap();
                g.ctx_at = None;
                g.ctx_sub = None;
                g.ctx_cat = None;
                g.ctx_dir = None;
                g.ctx_img = None;
                g.ctx_blank = true;
                drop(g);
                refresh(hwnd);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        // 最大化按工作区算，别把任务栏和屏幕外的边一起盖掉
        WM_GETMINMAXINFO => {
            let mmi = lparam.0 as *mut MINMAXINFO;
            let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(mon, &mut mi).as_bool() {
                let (wr, mr) = (mi.rcWork, mi.rcMonitor);
                (*mmi).ptMaxPosition.x = wr.left - mr.left;
                (*mmi).ptMaxPosition.y = wr.top - mr.top;
                (*mmi).ptMaxSize.x = wr.right - wr.left;
                (*mmi).ptMaxSize.y = wr.bottom - wr.top;
            }
            LRESULT(0)
        }
        // 无边框窗口：客户区盖满整个窗口。最大化不能再让边——让出来的那几像素
        // 归系统画，它会涂成浅灰白边，标题栏上面就多出一条白条。
        // 但系统给最大化窗口往外多推了一圈边框，那一圈正压在任务栏上；这里把
        // 客户区收进工作区，多出来的边落在任务栏底下，既看不见也不挡事。
        WM_NCCALCSIZE => {
            if wparam.0 != 0 && IsZoomed(hwnd).as_bool() {
                let p = lparam.0 as *mut NCCALCSIZE_PARAMS;
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
        WM_NCHITTEST => {
            let sx = (lparam.0 & 0xffff) as u16 as i16 as i32;
            let sy = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
            if !IsZoomed(hwnd).as_bool() {
                let mut wr = RECT::default();
                let _ = GetWindowRect(hwnd, &mut wr);
                let b = (6.0 * app().lock().unwrap().scale).round() as i32;
                let (l, r) = (sx - wr.left < b, wr.right - sx < b);
                let (t, bo) = (sy - wr.top < b, wr.bottom - sy < b);
                let edge = match (l, r, t, bo) {
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
                if let Some(hit) = edge {
                    return LRESULT(hit as isize);
                }
            }
            let mut pt = POINT { x: sx, y: sy };
            let _ = ScreenToClient(hwnd, &mut pt);
            let (title, btns, on_dlg) = {
                let g = app().lock().unwrap();
                let t = if xsee_host::viewer::is_open() {
                    xsee_host::viewer::title_h()
                } else {
                    g.th.title
                };
                // 浮层能被拖到标题栏那一条上，它画在上层：那一片的点击归它。
                // 否则 HTCAPTION 先被系统拿去拖主窗口，浮层连鼠标消息都收不到。
                let on = g.dlg.as_ref().is_some_and(|d| over(&d.rect, pt.x, pt.y));
                (t, g.btns.clone(), on)
            };
            if on_dlg {
                return LRESULT(HTCLIENT as isize);
            }
            if pt.y < title {
                if btns.iter().any(|(_, r)| over(r, pt.x, pt.y)) {
                    return LRESULT(HTCLIENT as isize);
                }
                return LRESULT(HTCAPTION as isize);
            }
            LRESULT(HTCLIENT as isize)
        }
        WM_SIZE => {
            xsee_host::viewer::resize(client(hwnd));
            let (z, _) = layout(hwnd);
            let mut g = app().lock().unwrap();
            let v = g.scroll.clamp(0, g.max_scroll(z.grid));
            g.scroll_set(v);
            g.queue_visible(z.grid);
            drop(g);
            place_edit(hwnd);
            refresh(hwnd);
            // 最大化/还原换了档才记一笔：双击标题条那条路走系统默认处理，
            // 不经过 command()，只能在这儿认。
            let zoomed = u32::from(IsZoomed(hwnd).as_bool());
            if UI_ZOOM.swap(zoomed, Ordering::Relaxed) != zoomed {
                ui_save(&app().lock().unwrap(), hwnd);
            }
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            // 拖边框这一下结束了，把新的宽高记下来
            ui_save(&app().lock().unwrap(), hwnd);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let rc = lparam.0 as *mut RECT;
            let r = &*rc;
            let _ = SetWindowPos(hwnd, None, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
            relayout(hwnd);
            place_edit(hwnd);
            refresh(hwnd);
            LRESULT(0)
        }
        WM_TIMER => {
            // 滚动帧节拍走单独一条：只动列表，不碰这颗心跳里缩略图/进度的活
            if wparam.0 == TMR_ANIM {
                anim_tick(hwnd);
                return LRESULT(0);
            }
            frame_tick();
            // 启动复查挂在这第一颗心跳上：要读几十次注册表，塞在 main 里就挡在首帧前面了。
            if app().lock().unwrap().assoc_probe {
                assoc_recheck(hwnd);
            }
            let (z, _) = layout(hwnd);
            let mut upd = app().lock().unwrap().pump();
            // 这一轮是不是后台有新缩略图到齐。滚停后的宽限期里只认这一笔。
            let pumped = upd;
            // 这一帧要动到网格外头吗。提示纱盖整窗、维护页的重建进度、找相似的提示条
            // 都在网格外，照旧整窗；新缩略图上屏和格底缘那道走光只落在列表里，
            // 整窗重画一帧要四五十毫秒，滚动中撞上就是一次明显的一顿。
            let mut full = false;
            {
                // 纱还在播就借这一颗现成的 60 毫秒心跳续帧，到点抹掉再画一帧干净的。
                let mut g = app().lock().unwrap();
                if let Some(t0) = g.flash {
                    upd = true;
                    full = true;
                    if t0.elapsed() >= FLASH_MS {
                        g.flash = None;
                    }
                }
                // 待生成格底缘那道走光也要续帧：还有图没出完就继续跳，出完自己停
                if g.wants_thumb() && g.got + g.err < g.n_images() {
                    upd = true;
                }
                // 全部重建的进度写在维护页里，靠这颗心跳把它顶上去
                if g.thumb_busy {
                    upd = true;
                    full = true;
                }
                // 找相似的进度写在提示条和那一页上，借同一颗心跳，不另发消息
                if g.sim_busy {
                    upd = true;
                    full = true;
                    let d = g.sim_done.load(Ordering::Relaxed);
                    let t = g.sim_total;
                    g.msg = format!("正在找相似：{d} / {t}");
                }
                // 维护页的两件后台活（移除无效索引、扫描文件夹）进度写在页里，
                // 借同一颗心跳顶上去；扫描没扫完前状态栏也跟着报数。
                if g.db_busy || g.scan_busy {
                    upd = true;
                    full = true;
                }
                if g.scan_busy {
                    let d = g.scan_done.load(Ordering::Relaxed);
                    g.msg = if g.scan_total == 0 {
                        "正在扫描文件夹：翻目录找图…".to_string()
                    } else {
                        format!("正在扫描文件夹：{d} / {}，新增 {}", g.scan_total, g.scan_added.load(Ordering::Relaxed))
                    };
                }
            }
            // 惯性还在跑就不插任何重画：出图到齐、进度走光先记一笔，滚停那一帧或
            // 宽限过后的心跳一次补齐。节拍在跑时别标 grid_stale——滚动期间旧格子
            // 照搬是条带路径的前提。惯性一停就只等缩略图：宽限期内收到的新图立刻
            // 补列表，不再空等 400 毫秒；闪纱、重建进度这些要整窗的仍旧押后。
            let (moving, scrolling) = {
                let g = app().lock().unwrap();
                (g.scroll_vel != 0.0 || g.grid_drag.is_some(), g.scrolling())
            };
            if moving {
                if upd {
                    if ANIM_ON.load(Ordering::Relaxed) {
                        app().lock().unwrap().anim_defer = true;
                    } else {
                        DEFER_FULL.store(true, Ordering::Relaxed);
                    }
                }
                upd = false;
            } else if DEFER_FULL.swap(false, Ordering::Relaxed) {
                upd = true;
                full = true;
            } else if scrolling {
                // 滚停后的 400 毫秒宽限期：缩略图到齐只补列表，别的整窗刷新照旧等。
                upd = pumped;
                full = false;
            }
            // 后台把这一层的字节数点完了：只补状态栏那一格。滚动中先不去取，
            // 等停下后的下一次心跳再取，免得为一行数字打断条带重画。
            if !scrolling {
                let tag = SZ_DONE.swap(0, Ordering::Acquire);
                if tag != 0 {
                    let mut g = app().lock().unwrap();
                    if tag == g.sz_seq {
                        g.total_sz = Some(SZ_TOTAL.load(Ordering::Relaxed));
                        drop(g);
                        invalidate(hwnd, &z.status);
                    }
                }
            }
            if upd {
                PAINT_SRC.store(2, Ordering::Relaxed);
                {
                    let mut g = app().lock().unwrap();
                    g.queue_visible(z.grid);
                    // 出图是一批一批到齐的：位图里那一屏有格子已经过时了，
                    // 下一帧别搬旧的，整块列表重画。
                    g.grid_stale = true;
                }
                if full {
                    refresh(hwnd);
                } else {
                    // 状态条上的张数和用时也归 pump 管，跟着列表一起补，别整窗。
                    refresh_grid(hwnd);
                    invalidate(hwnd, &z.status);
                }
            }
            LRESULT(0)
        }
        WM_PV => {
            // 带 stamp 的那条是预览图生成好了：变的只有预览格，和选中那一格的尺寸行
            // （宽高是解出来才知道的）。其余几条是批量转换/全部重建的进度，改的是提示条
            // 和维护页，低频，照旧整窗。以前一律整窗，滚动中撞上一帧就是四十多毫秒。
            let (z, _) = layout(hwnd);
            if wparam.0 != 0 {
                let cell = {
                    let g = app().lock().unwrap();
                    g.cell_rect(g.sel, z.grid)
                };
                invalidate(hwnd, &z.pv);
                invalidate(hwnd, &cell);
            } else {
                refresh(hwnd);
            }
            LRESULT(0)
        }
        WM_DROP_DONE => {
            let moved: Vec<(String, String)> = MOVED_IMGS.lock().unwrap().drain(..).collect();
            let src = app().lock().unwrap().src;
            {
                let mut g = app().lock().unwrap();
                let dirs: Vec<String> = DROP_DIRS.lock().unwrap().drain(..).collect();
                for d in &dirs {
                    tree_reload(&mut g.tree, d);
                }
                let why = DROP_ERR.lock().unwrap().clone();
                let mut m = if wparam.0 != 0 && lparam.0 == 0 {
                    "文件已经放进文件夹".to_string()
                } else if wparam.0 != 0 {
                    format!("放进去 {} 个，另有 {} 个没过去：{why}", wparam.0, lparam.0)
                } else if !why.is_empty() {
                    format!("没搬过去：{why}")
                } else {
                    "没有文件搬过去".to_string()
                };
                if !moved.is_empty() {
                    match sync_cat_paths(&mut g, &moved) {
                        Some(e) => m.push_str(&format!("；{e}")),
                        None => m.push_str(&format!("，{} 张图的类别位置已同步", moved.len())),
                    }
                }
                if src == Src::Folder {
                    let d = g.dir.display().to_string();
                    g.goto(&d, hwnd, false);
                } else {
                    g.refresh_src(hwnd);
                }
                g.msg = m;
            }
            request_pv(hwnd);
            refresh(hwnd);
            LRESULT(0)
        }
        WM_DB => {
            {
                let mut g = app().lock().unwrap();
                if g.cat.is_some() {
                    g.msg = if g.db_note.is_empty() {
                        "分类数据库已载入".to_string()
                    } else {
                        std::mem::take(&mut g.db_note)
                    };
                }
            }
            refresh(hwnd);
            LRESULT(0)
        }
        WM_DB_DONE => {
            let (kind, msg, reload) = DB_RESULT.lock().map(|r| r.clone()).unwrap_or_default();
            if reload {
                spawn_db_load(hwnd);
            }
            let to_page = {
                let mut g = app().lock().unwrap();
                let open = g.dlg.as_ref().is_some_and(|d| d.kind == DlgKind::CatDb);
                if open && kind == 0 {
                    g.db_maint = msg.clone();
                }
                g.msg = msg.lines().next().unwrap_or("数据库任务完成").to_string();
                open && kind == 0
            };
            // 检查从维护页发起就回到页面里显示；页面已关或导出/导入仍用提示框。
            if !to_page {
                open_ask(hwnd, "数据库", &msg, false, 0);
            }
            refresh(hwnd);
            LRESULT(0)
        }
        // 独立编辑器保存完发回来的路径：重扫当前视图，让那张缩略图重新解码。
        WM_COPYDATA => {
            let cds = lparam.0 as *const COPYDATASTRUCT;
            if cds.is_null() || (*cds).dwData != (WM_APP + 9) as usize {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            // 编辑器只回传一个路径；外来进程灌个超大 cbData 也别照单分配。
            if (*cds).cbData > 4096 {
                return LRESULT(0);
            }
            let p = if (*cds).cbData > 0 && !(*cds).lpData.is_null() {
                String::from_utf8_lossy(std::slice::from_raw_parts(
                    (*cds).lpData as *const u8,
                    (*cds).cbData as usize,
                ))
                .to_string()
            } else {
                String::new()
            };
            let keep = {
                let g = app().lock().unwrap();
                if p.is_empty() {
                    g.entries.get(g.sel).map(|e| e.path.clone())
                } else {
                    Some(p.clone())
                }
            };
            {
                let mut g = app().lock().unwrap();
                g.msg = if p.is_empty() {
                    "编辑已保存".to_string()
                } else {
                    format!("编辑已保存：{}", stem(&p))
                };
                g.refresh_src(hwnd);
                if let Some(k) = &keep {
                    if let Some(i) = g.entries.iter().position(|e| e.path.eq_ignore_ascii_case(k)) {
                        g.set_sel(i);
                    }
                }
            }
            request_pv(hwnd);
            refresh(hwnd);
            LRESULT(1)
        }
        WM_SEARCH_DONE => {
            let gid = wparam.0 as usize;
            if SEARCH_GEN.load(Ordering::SeqCst) != gid {
                return LRESULT(0);
            }
            let slot = SEARCH_RES.get_or_init(|| Mutex::new(None));
            let got = slot.lock().unwrap().take();
            if let Some((g, kw, files)) = got {
                if g == gid {
                    let n = files.len();
                    {
                        let mut a = app().lock().unwrap();
                        a.search_kw = kw;
                        a.search_files = files;
                        a.page = 0;
                        a.show_page(hwnd);
                        a.msg = format!("找到 {n} 张");
                    }
                    request_pv(hwnd);
                }
            }
            LRESULT(0)
        }
        WM_SIM_DONE => {
            let gid = wparam.0 as usize;
            if SIM_GEN.load(Ordering::SeqCst) != gid {
                return LRESULT(0);
            }
            let slot = SIM_RES.get_or_init(|| Mutex::new(None));
            let got = slot.lock().unwrap().take();
            {
                let mut a = app().lock().unwrap();
                a.sim_busy = false;
                a.sim_total = 0;
                match got {
                    Some((g, ms, hits, msg)) if g == gid => {
                        let n = hits.len();
                        a.sim_ms = ms;
                        a.sim_hits = hits;
                        a.sim_sel = 0;
                        a.sim_scroll = 0;
                        a.sim_lib_msg = match msg {
                            Some(m) => m,
                            None if n == 0 => {
                                "一张都没对上。把相似度调低些，或者勾上\u{201c}查找之前先补齐\u{201d}，下次会先补库再找。".to_string()
                            }
                            None => format!("找到 {n} 张，用了 {ms} 毫秒"),
                        };
                    }
                    // 后台那条没交回结果也要把"忙"摘掉，不然下次点只会回"还没跑完"。
                    _ => a.sim_lib_msg = "查找没跑成，再点一次试试".to_string(),
                }
            }
            sim_previews(hwnd);
            request_pv(hwnd);
            refresh(hwnd);
            LRESULT(0)
        }
        WM_SIM_PV => {
            refresh(hwnd);
            LRESULT(0)
        }
        WM_CONV_DONE => {
            let (ok, skip, err, stopped) = CONV_LAST.lock().map(|l| *l).unwrap_or((0, 0, 0, false));
            let errs = CONV_ERRS.lock().map(|v| v.clone()).unwrap_or_default();
            {
                let mut g = app().lock().unwrap();
                let mut m = if stopped {
                    format!("转换已停止：成功 {ok} 个")
                } else {
                    format!("转换完成：成功 {ok} 个")
                };
                if skip > 0 {
                    m.push_str(&format!("，跳过 {skip} 个同名"));
                }
                if err > 0 {
                    m.push_str(&format!("，失败 {err} 个"));
                }
                if !errs.is_empty() {
                    m.push_str(&format!("（{}）", errs.join("；")));
                }
                // 面板那条提示行同写一份：对话框发起的转换也要能在面板上看到收尾，
                // 不然面板上挂着的还是上一批的旧话。
                g.conv_msg = m.clone();
                g.msg = m;
                g.refresh_src(hwnd);
            }
            request_pv(hwnd);
            refresh(hwnd);
            LRESULT(0)
        }
        WM_CTLCOLOREDIT => {
            let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
            let g = app().lock().unwrap();
            SetTextColor(hdc, cref(g.th.skin.ink));
            SetBkColor(hdc, cref(g.th.skin.canvas));
            LRESULT(g.brush_edit.0 as isize)
        }
        WM_MOUSEWHEEL => {
            // 光标压在画布那扇上：这一下归画布，别滚自己的列表（焦点在这扇时系统不会
            // 把滚轮送到那块分层窗板上，得在这儿转交）
            if xsee_host::refwin::wheel_here(wparam, lparam) {
                return LRESULT(0);
            }
            wheel(hwnd, wparam, lparam);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            mouse_move(hwnd, lparam, wparam.0 as usize);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            mouse_down(hwnd, lparam, wparam.0 as usize, false);
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            mouse_down(hwnd, lparam, wparam.0 as usize, true);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (cat_drag, to, was_drag, band_done) = {
                let mut g = app().lock().unwrap();
                // 松手的这一下是不是刚拖完分隔条或列边界：是的话摆位要记一次
                let pane_drag = g.drag.is_some() || g.col_drag.is_some();
                g.drag = None;
                g.tree_drag = None;
                g.grid_drag = None;
                g.col_drag = None;
                g.org_drag = None;
                g.conv_drag = None;
                g.conv_sdrag = None;
                g.addr_anchor = None;
                if pane_drag {
                    ui_save(&g, hwnd);
                }
                // 框选收尾：把最后这一帧碰到的文件并进选区，光标落在最后一个命中的项上。
                let mut band_done = false;
                if let Some((x0, y0, x1, y1)) = g.band.take() {
                    band_done = true;
                    let z = zones(&g, client(hwnd), menu_h(hwnd));
                    let hits = g.band_pick(z.grid, x0, y0 - g.scroll, x1, y1 - g.scroll);
                    let mut sel = g.band_add.clone();
                    for i in &hits {
                        sel.insert(g.entries[*i].path.clone());
                    }
                    g.marked = sel;
                    if let Some(&last) = hits.last() {
                        g.sel = last;
                        g.anchor = last;
                        g.sel_lit = false;
                    }
                    g.band_add.clear();
                    g.stamp += 1;
                    g.trace(&format!("band{}", hits.len()));
                }
                // 拖完这一下：交回真重绘一次，把贴出来的那张图换成当下该画的界面
                // （进度、悬停、选中的色块都在这一步对齐）。
                let dragging = g.dlg_drag.take().is_some() || band_done;
                DRAGGING.store(false, Ordering::Relaxed);
                // 按下没拖动就是光标落位，不构成选区
                if let Some((a0, a1)) = g.addr_sel {
                    if a0 == a1 {
                        g.addr_sel = None;
                    }
                }
                (g.org_cat_drag.take(), std::mem::replace(&mut g.org_cat_to, -1), dragging, band_done)
            };
            let _ = ReleaseCapture();
            if was_drag {
                place_edit(hwnd);
                refresh(hwnd);
            }
            if band_done {
                request_pv(hwnd);
            }
            if let Some((cid, _, _)) = cat_drag {
                cat_drop(hwnd, cid, to);
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            // 捕获被别人抢走（系统菜单、别的事务）就算拖动到此为止：状态清干净，
            // 立刻补一帧全量重绘。少这一手，那个短路会把界面一直按在半路不动。
            let mut g = app().lock().unwrap();
            let band = g.band.take().is_some();
            if band {
                g.band_add.clear();
            }
            let drag = DRAGGING.swap(false, Ordering::Relaxed);
            if drag || band {
                g.dlg_drag = None;
                g.conv_drag = None;
                drop(g);
                refresh(hwnd);
            }
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            ctx_menu(hwnd, lparam);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            key(hwnd, wparam.0 as u32);
            LRESULT(0)
        }
        // Alt 组合从系统键进来。只把菜单上写出来的那几个转给 key()，
        // 别的（Alt+F4、Alt+空格、菜单助记符）照旧交给系统。
        // F10 不管带不带 Shift 都是系统键：不转进来，菜单上写的 F10=列表、
        // Shift+F10=右键 就都落空（按下只会去碰系统菜单）。
        WM_SYSKEYDOWN => {
            let vk = wparam.0 as u32;
            let alt_down = GetKeyState(VK_MENU.0 as i32) < 0;
            if matches!(
                vk,
                0x0D | 0x25 | 0x26 | 0x27 | 0x43 | 0x4D | 0x58
            ) || (vk == 0x79 && !alt_down)
            {
                key(hwnd, vk);
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        // 上面那几个 Alt 组合已经处理过，TranslateMessage 补出来的 WM_SYSCHAR
        // 不能再落给系统，否则会响一声或去碰菜单助记符。大小写都算上。
        WM_SYSCHAR => {
            if matches!(
                wparam.0 as u32,
                0x0D | 0x43 | 0x63 | 0x4D | 0x6D | 0x58 | 0x78
            ) {
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        // Shift+F10 / 菜单键经系统翻成这条消息。鼠标右键走的是 WM_RBUTTONUP，
        // 这条只处理键盘来的（lparam = -1）。
        WM_CONTEXTMENU => {
            if lparam.0 == -1 {
                ctx_menu_key(hwnd);
            }
            LRESULT(0)
        }
        WM_CHAR => {
            // 小键盘排序键后面跟着的那个数字不算打字，不然刚排完又跳走。
            if CHAR_SWALLOW.swap(false, Ordering::SeqCst) {
                return LRESULT(0);
            }
            type_ahead(hwnd, wparam.0 as u16);
            LRESULT(0)
        }
        WM_COMMAND => {
            let code = (wparam.0 >> 16) & 0xffff;
            if code == 0x0300 {
                // 编辑框内容变了：批量重命名和搜索的提示要跟着重画
                let id = (wparam.0 & 0xffff) as usize;
                if (E_TW..E_TW + E_N).contains(&id) {
                    // 面板上的格子只回写字段；它们的值不参与自绘，每敲一个字重画整屏没必要。
                    let text = edit_text(HWND(lparam.0 as *mut core::ffi::c_void));
                    let mut g = app().lock().unwrap();
                    match id {
                        E_TW => g.conv_tw = text,
                        E_TH => g.conv_th = text,
                        E_OUT => {
                            g.conv_out = text;
                            // 手敲了路径就算定死了，不再跟着当前文件夹走
                            g.conv_out_auto = false;
                        }
                        E_WM => g.conv_wm_text = text,
                        E_RN => g.conv_rn_tpl = text,
                        _ => {}
                    }
                } else {
                    refresh(hwnd);
                }
            } else if code == 8 {
                // 8 = EN_KILLFOCUS：改名框丢了焦点就当场提交，和点别处那一下是同一件事。
                let src = HWND(lparam.0 as *mut core::ffi::c_void);
                if app().lock().unwrap().ren_edit == Some(src) {
                    end_rename(hwnd, true);
                }
            } else if code == 0 || code == 1 {
                // 0 = 按钮/菜单，1 = 加速键。只认 0 的话，Ctrl+A 这类全在这一步被丢掉。
                command(hwnd, wparam.0 & 0xffff);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // 关窗再兜一次全量：中途哪个调整点漏写了，这一笔补上
            ui_save(&app().lock().unwrap(), hwnd);
            let _ = KillTimer(Some(hwnd), TMR);
            destroy_conv_edits();
            xsee_host::drop::unregister(hwnd);
            if let Some(p) = app().lock().unwrap().pool.take() {
                p.shutdown();
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn main() {
    // 启动探针：设了 XSEE_PROBE=<日志路径> 就不显示窗口，只把各阶段耗时写进去再退出。
    // 测试启动慢用，正常双击不受影响。
    let probe_path = std::env::var_os("XSEE_PROBE");
    let t_boot = std::time::Instant::now();
    let probe = |tag: &str| {
        let Some(p) = probe_path.as_ref() else { return };
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{} {:.1}", tag, t_boot.elapsed().as_secs_f64() * 1000.0);
        }
    };
    probe("main");
    // 自绘窗的 WndProc 是 extern "system"，里面一 panic 就是 0xC0000409 直接崩，
    // 事件日志里只留一个偏移，什么都查不出来。先把 panic 的那句话落一份盘。
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let p = std::env::temp_dir().join("xsee-panic.log");
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "[{}] {info}", std::process::id());
        }
    }));
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // DoDragDrop 要求发起线程是单线程套间；主线程本来就有消息循环
        let _ = windows::Win32::System::Ole::OleInitialize(None);
    }

    // JSON 命令口：把 --cmd 那一份作业摘出去，剩下的才是目录/文件参数。
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.windows(2).any(|w| {
        (w[0] == "--cmd" || w[0] == "-cmd") && matches!(w[1].as_str(), "help" | "?" | "--help")
    }) {
        println!("{}", drive::USAGE);
        return;
    }
    let (job, args) = drive::take(&raw);
    // 命令口默认把这扇摆到屏幕外：真画、真消息，但不挡住他手上的活
    let off = job.as_ref().is_some_and(|j| !j.show);
    let start = args.first().cloned().unwrap_or_else(startup_dir);
    let file_mode = !Path::new(&start).is_dir();
    let dir = if file_mode {
        Path::new(&start).parent().map(|p| p.display().to_string()).unwrap_or_else(|| "C:\\".into())
    } else {
        start.clone()
    };

    unsafe {
        let hinst = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        // 类上必须挂箭头光标：全自绘窗口不发消息里 SetCursor 的地方（比如对话框遮罩）
        // 会被系统设成 NULL，指针直接消失。
        let arrow = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        // 任务栏和 Alt+Tab 认的是窗口类上这两个句柄，不认 exe 资源段里那份；
        // 只嵌不挂，任务栏还是系统那张白纸。
        let (hicon, hicon_sm) = appicon::load();
        let cls = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            hIcon: hicon,
            hIconSm: hicon_sm,
            hCursor: arrow,
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            lpszClassName: PCWSTR(pw(CLASS).as_ptr()),
            ..Default::default()
        };
        if RegisterClassExW(&cls) == 0 {
            std::process::exit(1);
        }
        // 命令口/探针用工具窗样式：不进任务栏、不抢激活。摆哪儿是另一回事，
        // 下面单独判 —— 拍图要 DWM 那份重定向位图，摆到屏幕外主窗口就拍不着了。
        let exstyle = if probe_path.is_some() || job.is_some() {
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0)
        } else {
            WINDOW_EX_STYLE(0)
        };
        let hwnd = CreateWindowExW(
            exstyle,
            PCWSTR(pw(CLASS).as_ptr()),
            PCWSTR(pw(TITLE).as_ptr()),
            // 标题栏和菜单条都是自绘的，窗口不留系统边框；WS_THICKFRAME 保留拉伸和投影。
            // WS_CLIPCHILDREN：对话框的输入框是真子窗口，整屏 BitBlt 不剪掉它的话
            // 每帧都会把人家刚画的字抹了再等它自绘，看着就是输入框里的字在闪。
            WS_POPUP | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU
                | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1240,
            800,
            None,
            None,
            Some(hinst),
            None,
        );
        let Ok(hwnd) = hwnd else {
            std::process::exit(1);
        };
        probe("window");
        // 系统主题是浅色时，DWM 会给无标题栏窗口描一圈浅色边框；深色模式让这圈跟着皮肤走。
        let dark = windows::core::BOOL::from(true);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<windows::core::BOOL>() as u32,
        );
        // 无边框窗口仍会被系统描一圈 8px 的经典窗口边框（切回焦点时最明显），
        // 关掉 DWM 的非客户区渲染，边框和阴影都不画。
        let nc = DWMNCRP_DISABLED;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            &nc as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<DWMNCRENDERINGPOLICY>() as u32,
        );
        relayout(hwnd);
        probe("relayout");
        // 分类库在后台读，界面先出来；读完发一声让右侧面板自己刷新
        spawn_db_load(hwnd);
        probe("db_spawn");
        // 换壁纸前先记下原来的墙纸路径
        INIT_WALL.get_or_init(|| Mutex::new(reg_get("Wallpaper")));
        // 拖放接收：资源管理器/别的窗口拖进来的文件可以落进文件夹或直接打开
        xsee_host::viewer::set_host(hwnd);
        if !xsee_host::drop::register(hwnd, drop_received, drop_is_folder, drop_hover) {
            // 登记不上就是这台机器彻底收不到拖放，界面不会有任何动静，这句是唯一线索。
            app().lock().unwrap().msg = "这个窗口登记拖放接收失败，拖文件进来不会有反应".to_string();
        }
        app().lock().unwrap().tree = build_roots();
        probe("roots");
        // 不走 ShowWindow(SW_SHOWMAXIMIZED)：那条路上系统要先放一遍最大化过渡，
        // 窗口要过一秒多才第一次出现。直接把窗口摆到所在屏的工作区，一次 SetWindowPos 就上屏。
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let wr = if GetMonitorInfoW(mon, &mut mi).as_bool() {
            mi.rcWork
        } else {
            RECT { left: 0, top: 0, right: 1280, bottom: 800 }
        };
        // 记过大小就摆那么大（位置故意不记，从工作区左上起算）；记的是最大化、
        // 或者压根没记过，就照旧铺满整屏。上限收在工作区内，别让记忆把窗口顶出屏。
        let (mut ww, mut wh) = (wr.right - wr.left, wr.bottom - wr.top);
        let (rw, rh) = (ui_num("win_w", 0) as i32, ui_num("win_h", 0) as i32);
        if !ui_flag("win_max", true) && rw > 0 && rh > 0 {
            ww = rw.min(ww);
            wh = rh.min(wh);
        }
        let flags = SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_NOZORDER | SWP_NOACTIVATE;
        // 探针把窗口摆到屏幕外：真显示、真画，但用户看不见，也不激活。
        let (px, py) = if probe_path.is_some() || off { (-32000, -32000) } else { (wr.left, wr.top) };
        let _ = SetWindowPos(hwnd, None, px, py, ww, wh, flags);
        // NOACTIVATE 只跳过激活那一跳，窗口还得拿到前台和键盘焦点。
        // 命令口跑测试不抢这两下：他手上的活不该被一次测试打断。
        if probe_path.is_none() && job.is_none() {
            let _ = SetForegroundWindow(hwnd);
            let _ = SetFocus(Some(hwnd));
        }
        probe("shown");
        app().lock().unwrap().goto(&dir, hwnd, true);
        probe("goto");
        if file_mode {
            let mut g = app().lock().unwrap();
            if let Some(i) = g.entries.iter().position(|e| e.path == start) {
                g.set_sel(i);
            }
            drop(g);
            // 从资源管理器双击进来就是要看这一张，别再让他对着格子点第二下。
            // 只认得的后缀才这么办：不认得的留在格子里，走 shell 那条分支会
            // 又把 Xsee 叫起来（它就是这些格式自己），双击一下起一串进程。
            if scan::is_image(&start) {
                spawn_viewer(Path::new(&start), hwnd);
            }
        }
        request_pv(hwnd);
        check_menu(hwnd);
        let (z, _) = layout(hwnd);
        // 上次退出时转换页开着，这一启动面板就已经在屏幕上，不会走 open_conv_panel；
        // 输入框只在开面板/切页时建，缺这一下五个框全是空的（目录、宽高、水印、改名都没法填）。
        if app().lock().unwrap().conv_open {
            create_conv_edits(hwnd);
        }
        app().lock().unwrap().queue_visible(z.grid);
        probe("queued");
        if probe_path.is_some() {
            // 窗口在屏幕外也照样走真正的 WM_PAINT，量到的就是首帧成本。
            let _ = UpdateWindow(hwnd);
            probe("paint");
            // 再跑几秒消息循环，把首屏缩略图就绪的时间也记下来。
            let tid = SetTimer(Some(hwnd), TMR, 60, None);
            probe(&format!("timer {}", tid));
            // 直接在主线程解一张，分清是解码卡还是池子没取活。
            {
                let path = {
                    let a = app().lock().unwrap();
                    a.entries.iter().find(|e| e.is_image).map(|e| e.path.clone())
                };
                if let Some(p) = path {
                    let t = Instant::now();
                    let ok = job::make(&p, 181, 125).is_ok();
                    probe(&format!("direct {:.0} ok={}", t.elapsed().as_secs_f64() * 1000.0, ok));
                }
            }
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut pm = MSG::default();
            let mut beat = Instant::now();
            let mut tick = Instant::now();
            while Instant::now() < deadline {
                if PeekMessageW(&mut pm, None, 0, 0, PM_REMOVE).as_bool() {
                    if pm.message == WM_QUIT {
                        break;
                    }
                    let _ = TranslateMessage(&pm);
                    let _ = DispatchMessageW(&pm);
                } else {
                    std::thread::sleep(Duration::from_millis(4));
                }
                // 探针窗口在屏幕外，心跳可能不派发；自己泵，保证量到的是取图速度。
                if tick.elapsed() >= Duration::from_millis(8) {
                    tick = Instant::now();
                    let _ = app().lock().unwrap().pump();
                }
                if beat.elapsed() >= Duration::from_millis(500) {
                    beat = Instant::now();
                    let a = app().lock().unwrap();
                    probe(&format!(
                        "beat entries={} imgs={} q={} got={} err={} pool={} depth={} dir={}",
                        a.entries.len(),
                        a.n_images(),
                        a.queued.len(),
                        a.got,
                        a.err,
                        a.pool.is_some(),
                        a.pool.as_ref().map(|p| p.depth()).unwrap_or(9999),
                        a.dir.display()
                    ));
                }
                if let Some(r) = app().lock().unwrap().ready_sec {
                    probe(&format!("ready_ms {:.0}", r * 1000.0));
                    break;
                }
            }
            probe("probe_end");
            std::process::exit(0);
        }
        if let Some(j) = job.as_ref() {
            // JSON 命令口：把这一串跑完就退，结果那段 JSON 打到标准输出。
            // 窗口在屏幕外也照样走真绘制，PrintWindow 拍得到。
            println!("{}", drive::run(hwnd, j));
            std::process::exit(0);
        }
        let _ = SetTimer(Some(hwnd), TMR, 60, None);

        let haccel = accelerators();
        let mut m = MSG::default();
        loop {
            // 滚动动画期间不能死等消息：帧得按 16 毫秒的节拍自己走。先等到
            // "下一帧的时刻"或"来了一条消息"，两者谁先到都行。
            let mut got = false;
            if ANIM_ON.load(Ordering::Relaxed) {
                let now = now_ms();
                let next = NEXT_FRAME_MS.load(Ordering::Relaxed);
                if next == 0 || now >= next {
                    NEXT_FRAME_MS.store(now + FRAME_MS, Ordering::Relaxed);
                    anim_tick(hwnd);
                    continue;
                }
                let h = frame_timer();
                if h.0.is_null() {
                    std::thread::sleep(Duration::from_millis((next - now).min(50)));
                    continue;
                }
                let due = -((next - now) as i64 * 10_000);
                let _ = SetWaitableTimer(h, &due, 0, None, None, false);
                let r = MsgWaitForMultipleObjects(Some(&[h]), false, INFINITE, QS_ALLINPUT);
                // 计时器先到就回循环补帧；等来的若是消息（WAIT_OBJECT_0+1）才落到下面去取
                if r.0 != 1 {
                    continue;
                }
            } else if !GetMessageW(&mut m, None, 0, 0).as_bool() {
                break;
            } else {
                got = true;
            }
            if !got && !PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() {
                continue;
            }
            if m.message == WM_QUIT {
                break;
            }
            // 只认主窗口自己的按键。加速键表是拿整条消息去配的，参考图那类
            // 独立窗口拿到键盘焦点时也会被配中——在他点开的参考图上按 Del
            // 就会把主窗口选中的文件删了。
            let mine = m.hwnd == hwnd;
            // 对话框开着时不抢键：里面的输入框要吃 Enter 和字母。
            // 以图搜图那一页不模态：Ctrl+C/V 这类快捷键照旧归底下那屏。
            let dlg_open = app()
                .lock()
                .unwrap()
                .dlg
                .as_ref()
                .is_some_and(|d| d.kind != DlgKind::SimSearch);
            // 只喂按下事件：松手也交给加速键表会让同一条命令跑两遍
            let down = matches!(m.message, WM_KEYDOWN | WM_SYSKEYDOWN);
            let acc = mine
                && down
                && !dlg_open
                && !haccel.0.is_null()
                && TranslateAcceleratorW(hwnd, haccel, &m) != 0;
            if acc {
                continue;
            }
            let _ = TranslateMessage(&m);
            DispatchMessageW(&m);
        }
        let _ = DestroyAcceleratorTable(haccel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nd(path: &str, open: bool, kids: Vec<Node>) -> Node {
        Node { path: path.into(), name: path.into(), open, loaded: true, kids }
    }

    fn rows_of(tree: &[Node]) -> Vec<String> {
        let mut r = Vec::new();
        flat_tree(tree, 0, &mut r);
        r.into_iter().map(|(_, p, _, _)| p).collect()
    }

    /// 顶层「桌面」和 C 盘链下的桌面路径一模一样：开合只能碰被点的那一行。
    #[test]
    fn toggle_at_leaves_the_other_same_path_node_alone() {
        let desk = r"C:\Users\me\Desktop\";
        let sub = r"C:\Users\me\Desktop\proj\";
        let mut tree = vec![nd(desk, true, vec![nd(sub, false, vec![])]), nd(desk, false, vec![])];
        assert_eq!(rows_of(&tree), vec![desk, sub, desk]);

        let mut seq = 0;
        assert!(toggle_at(&mut tree, 2, &mut seq));
        assert!(tree[0].open, "第一个同路径节点不该被牵连");
        assert!(tree[1].open, "被点的第二行该从折叠变展开");
        assert_eq!(rows_of(&tree).len(), 3);

        let mut seq = 0;
        assert!(toggle_at(&mut tree, 0, &mut seq));
        assert!(!tree[0].open, "第 0 行该折叠");
        assert!(tree[1].open, "第二行保持原样");
        assert_eq!(rows_of(&tree), vec![desk, desk]);
    }
}

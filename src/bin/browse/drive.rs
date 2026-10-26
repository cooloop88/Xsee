//! xsee 自己的 JSON 命令口：`xsee --cmd '{"steps":[...]}'`
//!
//! 跑的是真窗口：真建窗、真消息、真绘制。默认摆在工作区里，但不抢前台、不抢焦点、
//! 不往注册表写偏好 —— 他手上的活不会被打断。给 `"show":false` 才摆到屏幕外：
//! 画布那扇是分层窗口，屏幕外也拍得到；主窗口摆出去 DWM 就不留位图，拍出来是黑的。
//!
//! 步骤按数组顺序走，每步之间自己泵消息（缩略图该回来的会回来）。
//! 结果一段 JSON 打到标准输出；出错也照样打，带 `ok:false`、第几步、错在哪。
use std::time::{Duration, Instant};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use xsee_host::{refwin, shot};

use super::{app, refresh, TMR};

// 鼠标按键状态位（windows 那套绑定把它们放在 SystemServices 下，就三个数，
// 不为这点东西多开一条 import 路径）。
const MK_LBUTTON: usize = 0x0001;
const MK_RBUTTON: usize = 0x0002;
const MK_SHIFT: usize = 0x0004;
const MK_CONTROL: usize = 0x0008;
const MK_MBUTTON: usize = 0x0010;

// ------------------------------------------------------------------ JSON 一小份
// 只为这一个口，不引 serde 那一串依赖。够用就行：对象、数组、字符串（带 \u 转义，
// 含代理对）、数字、true/false/null。

#[derive(Clone, Debug)]
pub enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    fn get(&self, k: &str) -> Option<&J> {
        match self {
            J::Obj(kv) => kv.iter().find(|(n, _)| n == k).map(|(_, v)| v),
            _ => None,
        }
    }

    /// 取字符串；没这个键、或不是字符串，就回默认值。
    fn s(&self, k: &str, d: &str) -> String {
        match self.get(k) {
            Some(J::Str(v)) => v.clone(),
            _ => d.to_string(),
        }
    }

    fn n(&self, k: &str, d: i64) -> i64 {
        match self.get(k) {
            Some(J::Num(v)) => *v as i64,
            Some(J::Bool(true)) => 1,
            Some(J::Bool(false)) => 0,
            _ => d,
        }
    }

    /// 给了数字才回 Some —— 用来分「没写这个键」和「写了 0」。
    fn num(&self, k: &str) -> Option<i64> {
        match self.get(k) {
            Some(J::Num(v)) => Some(*v as i64),
            _ => None,
        }
    }

    fn b(&self, k: &str, d: bool) -> bool {
        match self.get(k) {
            Some(J::Bool(v)) => *v,
            Some(J::Num(v)) => *v != 0.0,
            _ => d,
        }
    }

    fn list(&self, k: &str) -> Vec<J> {
        match self.get(k) {
            Some(J::Arr(v)) => v.clone(),
            _ => Vec::new(),
        }
    }

    /// 四个数的数组（裁剪框）。给不齐就回 None。
    fn rect4(&self, k: &str) -> Option<(i32, i32, i32, i32)> {
        match self.get(k) {
            Some(J::Arr(v)) if v.len() == 4 => {
                let n = |i: usize| match &v[i] {
                    J::Num(x) => *x as i32,
                    _ => 0,
                };
                Some((n(0), n(1), n(2), n(3)))
            }
            _ => None,
        }
    }
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn val(&mut self) -> Result<J, String> {
        self.ws();
        match self.peek().ok_or("JSON 到头发完了")? {
            b'{' => self.obj(),
            b'[' => self.arr(),
            b'"' => Ok(J::Str(self.string()?)),
            b't' => self.lit("true", J::Bool(true)),
            b'f' => self.lit("false", J::Bool(false)),
            b'n' => self.lit("null", J::Null),
            c if c == b'-' || c.is_ascii_digit() => self.num(),
            c => Err(format!("这里不该有 '{}'", c as char)),
        }
    }

    fn obj(&mut self) -> Result<J, String> {
        self.i += 1;
        let mut kv = Vec::new();
        loop {
            self.ws();
            match self.peek() {
                Some(b'}') => {
                    self.i += 1;
                    return Ok(J::Obj(kv));
                }
                Some(b'"') => {
                    let k = self.string()?;
                    self.ws();
                    if self.peek() != Some(b':') {
                        return Err(format!("键 {k} 后面缺冒号"));
                    }
                    self.i += 1;
                    kv.push((k, self.val()?));
                    self.ws();
                    if self.peek() == Some(b',') {
                        self.i += 1;
                    }
                }
                other => {
                    return Err(format!("对象里看到了不该有的 {:?}", other.map(|c| c as char)))
                }
            }
        }
    }

    fn arr(&mut self) -> Result<J, String> {
        self.i += 1;
        let mut v = Vec::new();
        loop {
            self.ws();
            if self.peek() == Some(b']') {
                self.i += 1;
                return Ok(J::Arr(v));
            }
            v.push(self.val()?);
            self.ws();
            if self.peek() == Some(b',') {
                self.i += 1;
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut o = String::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            if c == b'"' {
                return Ok(o);
            }
            if c != b'\\' {
                // UTF-8 原样搬：按首字节判断这个字符占几字节
                let start = self.i - 1;
                let n = self.b[start] as usize;
                let len = if n < 0x80 {
                    1
                } else if n >> 5 == 0b110 {
                    2
                } else if n >> 4 == 0b1110 {
                    3
                } else {
                    4
                };
                if start + len > self.b.len() {
                    return Err("字符串到一半断了".to_string());
                }
                let s = std::str::from_utf8(&self.b[start..start + len]).map_err(|e| e.to_string())?;
                o.push_str(s);
                self.i = start + len;
                continue;
            }
            let e = self.peek().ok_or("转义到一半断了")?;
            self.i += 1;
            match e {
                b'n' => o.push('\n'),
                b'r' => o.push('\r'),
                b't' => o.push('\t'),
                b'\\' => o.push('\\'),
                b'/' => o.push('/'),
                b'"' => o.push('"'),
                b'u' => o.push(self.hex4()?),
                x => return Err(format!("不认的转义 \\{}", x as char)),
            }
        }
        Err("字符串没闭合".to_string())
    }

    fn hex4(&mut self) -> Result<char, String> {
        let hi = self.hex()?;
        if !(0xD800..0xDC00).contains(&hi) {
            return char::from_u32(hi).ok_or("码点不是字符".to_string());
        }
        // 代理对：后一个码元紧跟着 \u
        if self.peek() == Some(b'\\') && self.b.get(self.i + 1) == Some(&b'u') {
            self.i += 2;
            let lo = self.hex()?;
            let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
            return char::from_u32(cp).ok_or("代理对算出来不是字符".to_string());
        }
        Err("代理对缺后半".to_string())
    }

    fn hex(&mut self) -> Result<u32, String> {
        if self.i + 4 > self.b.len() {
            return Err("\\u 后面不满四位".to_string());
        }
        let s = std::str::from_utf8(&self.b[self.i..self.i + 4]).map_err(|e| e.to_string())?;
        let v = u32::from_str_radix(s, 16).map_err(|e| e.to_string())?;
        self.i += 4;
        Ok(v)
    }

    fn num(&mut self) -> Result<J, String> {
        let start = self.i;
        while let Some(c) = self.peek() {
            if !(c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E')) {
                break;
            }
            self.i += 1;
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).map_err(|e| e.to_string())?;
        s.parse::<f64>().map(J::Num).map_err(|e| format!("数字「{s}」读不出来 {e}"))
    }

    fn lit(&mut self, w: &str, v: J) -> Result<J, String> {
        if self.b.len() < self.i + w.len() || &self.b[self.i..self.i + w.len()] != w.as_bytes() {
            return Err(format!("这里应该是 {w}"));
        }
        self.i += w.len();
        Ok(v)
    }
}

pub fn parse(src: &str) -> Result<J, String> {
    let mut p = P { b: src.as_bytes(), i: 0 };
    let v = p.val()?;
    p.ws();
    if p.i != p.b.len() {
        return Err("JSON 后面还有多余的东西".to_string());
    }
    Ok(v)
}

// ------------------------------------------------------------------ 一份作业
pub struct Job {
    /// 主窗口摆哪儿：true（默认）= 摆在工作区里，但不抢前台不抢焦点；
    /// false = 摆到屏幕外 —— 主窗口是普通窗口，摆出去 DWM 就不给它留位图了，
    /// 拍出来是一张黑的。画布那扇是分层窗口，驱动模式下一直在屏幕外，照样拍得到。
    pub show: bool,
    /// 要不要把偏好写回注册表：默认不写
    pub save: bool,
    steps: Vec<J>,
    err: Option<String>,
}

impl Job {
    fn fail(e: String) -> Job {
        Job { show: false, save: false, steps: Vec::new(), err: Some(e) }
    }
}

/// 从命令行里把 `--cmd <json>` / `--cmd=@json` / `--cmd @文件` / `--cmd -`（读标准输入）
/// 摘出来。回（这一份作业，剩下的位置参数）。没给 --cmd 就是 (None, 原参数)。
pub fn take(args: &[String]) -> (Option<Job>, Vec<String>) {
    let mut rest: Vec<String> = Vec::new();
    let mut src: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--cmd" || a == "-cmd" {
            src = it.next().cloned();
        } else if let Some(v) = a.strip_prefix("--cmd=") {
            src = Some(v.to_string());
        } else {
            rest.push(a.clone());
        }
    }
    let Some(src) = src else { return (None, rest) };
    let text = if src == "-" {
        use std::io::Read;
        let mut b = String::new();
        match std::io::stdin().read_to_string(&mut b) {
            Ok(_) => b,
            Err(e) => return (Some(Job::fail(format!("读标准输入失败 {e}"))), rest),
        }
    } else if let Some(p) = src.strip_prefix('@') {
        match std::fs::read_to_string(p) {
            Ok(t) => t,
            Err(e) => return (Some(Job::fail(format!("读 {p} 失败 {e}"))), rest),
        }
    } else {
        src
    };
    let v = match parse(&text) {
        Ok(v) => v,
        Err(e) => return (Some(Job::fail(format!("JSON 读不出来：{e}"))), rest),
    };
    let steps = v.list("steps");
    if steps.is_empty() {
        return (Some(Job::fail("steps 是空的（或没给）".to_string())), rest);
    }
    (
        Some(Job { show: v.b("show", true), save: v.b("save", false), steps, err: None }),
        rest,
    )
}

pub const USAGE: &str = r#"用法: xsee --cmd '{"steps":[...]}'   或  --cmd @作业.json   或  --cmd -（读标准输入）
跑的是真窗口。默认：主窗口摆工作区但不抢前台不抢焦点、画布摆屏幕外（分层窗口照样拍得到）、不写注册表偏好。结果 JSON 打到标准输出。

win 取 "main"（默认）/ "canvas" / "class:窗口类名"。每步都可选 "ms"（这步之后泵多久）。
  {"op":"sleep","ms":300}                泵消息等一会儿
  {"op":"dir","path":"C:\\图"}           主窗口进这个目录
  {"op":"select","name":"a.jpg"}         选中那一格（给右键、给命令用）；也可 "index":3
  {"op":"cmd","id":1234}                 走一条命令号（同 WM_COMMAND）
  {"op":"canvas","path":"C:\\a.jpg"}     开画布并把这张摆进去
  {"op":"click","x":40,"y":70,"btn":"right|left|dbl|move|mid","n":2,"ctrl":true}
  {"op":"click","x":40,"y":70,"wheel":"down","ctrl":true}
  {"op":"key","vk":"C|DELETE|ESC|RETURN|DOWN","ctrl":true,"shift":false}
  {"op":"shot","out":"C:\\s.png","crop":[x,y,w,h],"zoom":2}    拍这扇窗，回报宽高
  {"op":"state"}                         读此刻的数字：主窗口目录/条数/选中/提示；画布卡片和弹层每行矩形
  {"op":"wait","win":"canvas","gone":true,"ms":3000}           等窗口出现（gone:true 是等它没了）
  {"op":"park"}                          把这扇窗挪到屏幕外
  {"op":"quit"}                          剩下的不跑
顶层还有 "show":false（窗口摆到屏幕外 —— 画布照样拍得到，主窗口会拍成一张黑的）、
"save":true（允许把偏好写回注册表）。
例子：xsee --cmd "{\"steps\":[{\"op\":\"canvas\",\"path\":\"C:\\a.jpg\"},{\"op\":\"click\",\"win\":\"canvas\",\"x\":40,\"y\":70,\"btn\":\"right\"},{\"op\":\"state\",\"win\":\"canvas\"},{\"op\":\"shot\",\"win\":\"canvas\",\"out\":\"C:\\m.png\",\"crop\":[30,60,300,140]}]}"
"#;

// ------------------------------------------------------------------ 执行
static mut QUIET: bool = false;

/// 命令口跑起来时置真：主窗口那几处 ui_save 据此跳过，不把他那套偏好覆盖掉。
pub fn quiet() -> bool {
    unsafe { QUIET }
}

/// 在真窗口建好之后、正常消息循环之前跑一次。回结果 JSON 文本，调用方打出去。
pub unsafe fn run(main: HWND, job: &Job) -> String {
    QUIET = !job.save;
    // 画布那扇在驱动模式下永远摆屏幕外：它是分层窗口，屏幕外 PrintWindow 照样
    // 拍得到，而 open() 那一下不该把前台从他手上抢走。
    refwin::set_headless(true);
    if let Some(e) = job.err.as_ref() {
        return format!("{{\"ok\":false,\"err\":\"{}\"}}", esc(e));
    }
    let _ = SetTimer(Some(main), TMR, 60, None);
    let mut out = Vec::with_capacity(job.steps.len());
    for (i, st) in job.steps.iter().enumerate() {
        let op = st.s("op", "");
        let body = match one(main, st) {
            Ok(v) => format!("{{\"i\":{i},\"op\":\"{}\",\"r\":{v}}}", esc(&op)),
            Err(e) => {
                if e == "__quit__" {
                    out.push(format!("{{\"i\":{i},\"op\":\"quit\"}}"));
                    break;
                }
                out.push(format!("{{\"i\":{i},\"op\":\"{}\",\"err\":\"{}\"}}", esc(&op), esc(&e)));
                return format!(
                    "{{\"ok\":false,\"stopped\":{i},\"steps\":[{}]}}",
                    out.join(",")
                );
            }
        };
        out.push(body);
    }
    format!("{{\"ok\":true,\"steps\":[{}]}}", out.join(","))
}

/// 泵这么多毫秒的消息：窗口该画的画、缩略图池该取的取。
unsafe fn pump(ms: i64) {
    let end = Instant::now() + Duration::from_millis(ms.clamp(0, 60_000) as u64);
    let mut pm = MSG::default();
    loop {
        while PeekMessageW(&mut pm, None, 0, 0, PM_REMOVE).as_bool() {
            if pm.message == WM_QUIT {
                std::process::exit(0);
            }
            let _ = TranslateMessage(&pm);
            DispatchMessageW(&pm);
        }
        if let Ok(mut g) = app().lock() {
            let _ = g.pump();
        }
        if Instant::now() >= end {
            return;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
}

/// 找那一扇。
fn target(main: HWND, name: &str) -> Result<HWND, String> {
    let n = if name.is_empty() { "main" } else { name };
    let h = match n {
        "main" => main,
        "canvas" => refwin::window().ok_or("画布还没开（先跑一条 canvas）".to_string())?,
        _ => {
            let cls = n.strip_prefix("class:").unwrap_or(n);
            let w: Vec<u16> = cls.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                FindWindowW(PCWSTR(w.as_ptr()), PCWSTR::null()).unwrap_or_default()
            }
        }
    };
    if h.0.is_null() || !unsafe { IsWindow(Some(h)) }.as_bool() {
        return Err(format!("找不到窗口「{n}」"));
    }
    Ok(h)
}

fn lp(x: i64, y: i64) -> LPARAM {
    LPARAM((((y as i32 as u32) as u64) << 16 | ((x as i32 as u32) as u64 & 0xffff)) as isize)
}

/// 把 Ctrl/Shift/Alt 在**本线程**的按键状态里临时按下去再放开。GetKeyState 读的
/// 就是本线程那份，所以不碰真键盘、不抢他的输入，而程序里"按住 Ctrl 点"那类
/// 分支照样走得通。
unsafe fn with_mods<T>(ctrl: bool, shift: bool, alt: bool, f: impl FnOnce() -> T) -> T {
    let mut old = [0u8; 256];
    let _ = GetKeyboardState(&mut old);
    let mut st = old;
    for (vk, on) in
        [(VK_CONTROL.0 as usize, ctrl), (VK_SHIFT.0 as usize, shift), (VK_MENU.0 as usize, alt)]
    {
        if on {
            st[vk] |= 0x80;
        } else {
            st[vk] &= 0x7f;
        }
    }
    let _ = SetKeyboardState(&st);
    let r = f();
    let _ = SetKeyboardState(&old);
    r
}

/// "C" / "DELETE" / "F5" 这一类名字换成虚拟键码。
fn vk_of(name: &str) -> u32 {
    let up = name.trim().to_ascii_uppercase();
    if up.len() == 1 {
        let c = up.as_bytes()[0];
        return match c {
            b' ' => VK_SPACE.0 as u32,
            b'0'..=b'9' | b'A'..=b'Z' => c as u32,
            _ => 0,
        };
    }
    if let Some(f) = up.strip_prefix('F') {
        if let Ok(n) = f.parse::<u32>() {
            if (1..=24).contains(&n) {
                return VK_F1.0 as u32 + n - 1;
            }
        }
    }
    match up.as_str() {
        "DELETE" | "DEL" => VK_DELETE.0 as u32,
        "BACK" | "BACKSPACE" => VK_BACK.0 as u32,
        "ENTER" | "RETURN" => VK_RETURN.0 as u32,
        "ESC" | "ESCAPE" => VK_ESCAPE.0 as u32,
        "TAB" => VK_TAB.0 as u32,
        "SPACE" => VK_SPACE.0 as u32,
        "LEFT" => VK_LEFT.0 as u32,
        "UP" => VK_UP.0 as u32,
        "RIGHT" => VK_RIGHT.0 as u32,
        "DOWN" => VK_DOWN.0 as u32,
        "HOME" => VK_HOME.0 as u32,
        "END" => VK_END.0 as u32,
        "PGUP" | "PRIOR" => VK_PRIOR.0 as u32,
        "PGDN" | "NEXT" => VK_NEXT.0 as u32,
        "INSERT" => VK_INSERT.0 as u32,
        _ => 0,
    }
}

unsafe fn one(main: HWND, st: &J) -> Result<String, String> {
    let op = st.s("op", "");
    let win = target(main, &st.s("win", "main"))?;
    match op.as_str() {
        "" => Err("这一步没写 op".to_string()),
        "sleep" => {
            let ms = st.n("ms", 250);
            pump(ms);
            Ok(format!("{{\"ms\":{ms}}}"))
        }
        "dir" => {
            let p = st.s("path", "");
            if p.is_empty() {
                return Err("dir 没给 path".to_string());
            }
            {
                let mut g = app().lock().map_err(|e| e.to_string())?;
                g.goto(&p, main, true);
            }
            pump(st.n("ms", 500));
            Ok(format!("{{\"path\":\"{}\"}}", esc(&p)))
        }
        "select" => {
            let name = st.s("name", "");
            let mut g = app().lock().map_err(|e| e.to_string())?;
            let hit = match st.num("index") {
                Some(i) => {
                    let i = (i.max(0) as usize).min(g.entries.len().saturating_sub(1));
                    Some(i)
                }
                None => g.entries.iter().position(|e| {
                    e.path.rsplit(['\\', '/']).next().unwrap_or("").eq_ignore_ascii_case(&name)
                }),
            };
            let Some(i) = hit else {
                return Err(format!("格子里没有「{name}」"));
            };
            g.set_sel(i);
            let p = g.entries[i].path.clone();
            drop(g);
            refresh(main);
            pump(st.n("ms", 150));
            Ok(format!("{{\"index\":{i},\"path\":\"{}\"}}", esc(&p)))
        }
        "cmd" => {
            let id = st.n("id", 0);
            if id <= 0 {
                return Err("cmd 没给 id".to_string());
            }
            // 投消息，不直接调 command()：那里面有一类会开模态框，直接调会把这条
            // 线卡死在那一扇里。
            let _ = PostMessageW(Some(main), WM_COMMAND, WPARAM(id as usize), LPARAM(0));
            pump(st.n("ms", 300));
            Ok(format!("{{\"id\":{id}}}"))
        }
        "canvas" => {
            let p = st.s("path", "");
            refwin::open(&p, main);
            pump(st.n("ms", 800));
            let h = target(main, "canvas")?;
            let (_, _, w, hh) = shot::rect(h);
            Ok(format!("{{\"w\":{w},\"h\":{hh}}}"))
        }
        "click" => {
            let (x, y) = (st.n("x", 0), st.n("y", 0));
            let btn = st.s("btn", "left");
            let wheel = st.s("wheel", "");
            let (ctrl, shift, alt) = (st.b("ctrl", false), st.b("shift", false), st.b("alt", false));
            if !wheel.is_empty() {
                let mut pt = POINT { x: x as i32, y: y as i32 };
                let _ = ClientToScreen(win, &mut pt);
                let delta: u16 = if wheel == "down" { (-120i16) as u16 } else { 120 };
                let wp = WPARAM(
                    delta as usize | if ctrl { MK_CONTROL } else { 0 } | if shift { MK_SHIFT } else { 0 },
                );
                let lp = LPARAM((((pt.y as u32) as u64) << 16 | (pt.x as u32) as u64) as isize);
                with_mods(ctrl, shift, alt, || {
                    let _ = SendMessageW(win, WM_MOUSEWHEEL, Some(wp), Some(lp));
                });
                pump(st.n("ms", 240));
                return Ok(format!("{{\"wheel\":\"{wheel}\"}}"));
            }
            let n = st.n("n", 1).max(1);
            for i in 0..n {
                with_mods(ctrl, shift, alt, || match btn.as_str() {
                    "right" => {
                        let _ = SendMessageW(win, WM_RBUTTONDOWN, Some(WPARAM(MK_RBUTTON)), Some(lp(x, y)));
                        let _ = SendMessageW(win, WM_RBUTTONUP, None, Some(lp(x, y)));
                    }
                    "mid" => {
                        let _ = SendMessageW(win, WM_MBUTTONDOWN, Some(WPARAM(MK_MBUTTON)), Some(lp(x, y)));
                        let _ = SendMessageW(win, WM_MBUTTONUP, None, Some(lp(x, y)));
                    }
                    "move" => {
                        let _ = SendMessageW(win, WM_MOUSEMOVE, None, Some(lp(x, y)));
                    }
                    _ => {
                        // 第二下起算双击：走系统那套 DBLCLK，程序里的快慢双击判据才跟真手一致
                        let msg = if i >= 1 { WM_LBUTTONDBLCLK } else { WM_LBUTTONDOWN };
                        let _ = SendMessageW(win, msg, Some(WPARAM(MK_LBUTTON)), Some(lp(x, y)));
                        let _ = SendMessageW(win, WM_LBUTTONUP, None, Some(lp(x, y)));
                    }
                });
                pump(st.n("ms", if btn == "right" { 140 } else { 90 }));
            }
            Ok(format!("{{\"at\":[{x},{y}],\"btn\":\"{}\",\"n\":{n}}}", esc(&btn)))
        }
        "key" => {
            let vk = vk_of(&st.s("vk", ""));
            if vk == 0 {
                return Err("key 没给 vk（或这个名字不认）".to_string());
            }
            let (ctrl, shift, alt) = (st.b("ctrl", false), st.b("shift", false), st.b("alt", false));
            with_mods(ctrl, shift, alt, || {
                let _ = SendMessageW(win, WM_KEYDOWN, Some(WPARAM(vk as usize)), Some(LPARAM(0)));
                let _ = SendMessageW(win, WM_KEYUP, Some(WPARAM(vk as usize)), Some(LPARAM(0)));
            });
            pump(st.n("ms", 200));
            Ok(format!("{{\"vk\":{vk},\"ctrl\":{ctrl},\"shift\":{shift},\"alt\":{alt}}}"))
        }
        "shot" => {
            let out = st.s("out", "");
            if out.is_empty() {
                return Err("shot 没给 out".to_string());
            }
            if let Some(p) = out.rsplit_once(['\\', '/']).map(|(a, _)| a) {
                if !p.is_empty() {
                    let _ = std::fs::create_dir_all(p);
                }
            }
            pump(st.n("ms", 280));
            let (w, h) = shot::window(win, &out, st.rect4("crop"), st.n("zoom", 1) as i32)?;
            Ok(format!("{{\"out\":\"{}\",\"w\":{w},\"h\":{h}}}", esc(&out)))
        }
        "state" => {
            pump(st.n("ms", 80));
            Ok(if win == main { main_json() } else { refwin::probe_json() })
        }
        "wait" => {
            let name = st.s("win", "main");
            let want_gone = st.b("gone", false);
            let ms = st.n("ms", 3000).max(200);
            let t0 = Instant::now();
            let end = t0 + Duration::from_millis(ms as u64);
            loop {
                let open = target(main, &name).is_ok();
                if open != want_gone {
                    let took = t0.elapsed().as_millis();
                    return Ok(format!("{{\"found\":{open},\"ms\":{took}}}"));
                }
                if Instant::now() >= end {
                    return Err(format!("等 {name} 等到超时（{ms} 毫秒）"));
                }
                pump(60);
            }
        }
        "park" => {
            let _ = SetWindowPos(
                win,
                None,
                -32000,
                -32000,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            pump(120);
            Ok("{\"parked\":true}".to_string())
        }
        "quit" => Err("__quit__".to_string()),
        other => Err(format!("不认的 op「{other}」")),
    }
}

/// 主窗口此刻的数字。状态从程序自己嘴里报，不靠截图像素去猜。
unsafe fn main_json() -> String {
    let Ok(g) = app().lock() else { return "{\"err\":\"锁没拿到\"}".to_string() };
    let sel_path = g.entries.get(g.sel).map(|e| e.path.clone()).unwrap_or_default();
    format!(
        "{{\"open\":true,\"dir\":\"{}\",\"entries\":{},\"imgs\":{},\"sel\":{},\"sel_path\":\"{}\",\"msg\":\"{}\",\"got\":{},\"err\":{},\"conv_open\":{},\"org_open\":{}}}",
        esc(&g.dir.display().to_string()),
        g.entries.len(),
        g.n_images(),
        g.sel,
        esc(&sel_path),
        esc(&g.msg),
        g.got,
        g.err,
        g.conv_open,
        g.org_open
    )
}

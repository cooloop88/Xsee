//! 皮肤系统：一套写死的规则，皮肤只是规则喂出来的数据。
//!
//! 皮肤一套一个文件，摆在 skins/*.skin 或 skins/<色系>/*.skin 下。皮肤目录有两处默认：
//! exe 旁的 skins/（便携包自带那一套）优先，那里没有才用系统目录 %LOCALAPPDATA%\xsee\skins。
//! exe 里另编一套兜底配色（builtin.skin）：两处都没有也能完整开窗，
//! 盘上有同名的一套时以盘上的为准；程序自己不会去建 skins/ 目录。
//!
//! 规则（皮肤作者只给 8 个基色，其余 14 个槽位由派生规则算出来）：
//!   必填 base / surface / canvas / ink / dim / accent / border / mark
//!   派生 window=mix(base,ink,4%) chrome=surface chrome_hi=lighten(surface,4%)
//!        pane=canvas pane_head=mix(surface,base,50%) grid=mix(canvas,ink,7%)
//!        line=border line_soft=mix(border,base,45%) ink_off=fade(ink,canvas,72%@对比2)
//!        dim=ensure(dim,canvas,3) hov=tint_surface(accent,14%) prs=tint_surface(accent,26%)
//!        sel=tint_surface(accent,52%) sel_ink=contrast_pick(sel, ink)
//!        acc_soft=tint_base(accent,22%) flash=tint_base(accent,22%) focus=lighten(accent,8%)
//!   校验：ink/canvas≥4.5，ink/chrome≥4.5，dim/canvas≥3，sel_ink/sel≥4.5，
//!        ink/base≥4.5；不达标先换文字极性，仍不行就报错（不静默出难看皮肤）。
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use windows::core::PCWSTR;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW,
};

pub const SLOTS: [&str; 24] = [
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
    "ink",
    "dim",
    "off",
    "inv",
    "hov",
    "prs",
    "sel",
    "sel_ink",
    "accent",
    "acc_soft",
    "mark",
    "flash",
    "focus",
];

type Rgb = (f64, f64, f64);

fn split(c: u32) -> Rgb {
    (
        ((c >> 16) & 0xff) as f64 / 255.0,
        ((c >> 8) & 0xff) as f64 / 255.0,
        (c & 0xff) as f64 / 255.0,
    )
}

fn join(c: Rgb) -> u32 {
    let q = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    (q(c.0) << 16) | (q(c.1) << 8) | q(c.2)
}

/// 感知空间混合（在 sqrt 域插值再平方），纯线性混合在亮色端会把颜色压得太狠。
pub fn mix(a: u32, b: u32, t: f64) -> u32 {
    let (x, y) = (split(a), split(b));
    let m = |p: f64, q: f64| (p.sqrt() * (1.0 - t) + q.sqrt() * t).powi(2);
    join((m(x.0, y.0), m(x.1, y.1), m(x.2, y.2)))
}

pub fn lighten(c: u32, t: f64) -> u32 {
    mix(c, 0xffffff, t)
}

pub fn darken(c: u32, t: f64) -> u32 {
    mix(c, 0x000000, t)
}

pub fn lum(c: u32) -> f64 {
    let f = |v: f64| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = split(c);
    0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
}

pub fn contrast(a: u32, b: u32) -> f64 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// 文字极性：同一个字色压到两个底上，挑对比度高的那个。
fn pick(bg: u32, ink: u32, inv: u32) -> u32 {
    if contrast(ink, bg) >= contrast(inv, bg) {
        ink
    } else {
        inv
    }
}

/// 淡出到看得不见为止：往背景色方向淡，但对比度掉到下限就停住。
/// 深色皮肤上"淡 72%"会直接淡没，所以淡多少由规则算，不由皮肤作者猜。
fn fade(ink: u32, bg: u32, min: f64, cap: f64) -> u32 {
    let mut best = ink;
    let mut t = 0.0;
    while t <= cap + 1e-6 {
        let c = mix(ink, bg, t);
        if contrast(c, bg) >= min {
            best = c;
        } else {
            break;
        }
        t += 1.0 / 40.0;
    }
    best
}

/// 对比度不达标就往背景的反方向顶，顶到够为止。作者给的值能保留就保留。
fn ensure(fg: u32, bg: u32, min: f64) -> u32 {
    if contrast(fg, bg) >= min {
        return fg;
    }
    let target = if lum(bg) > 0.5 { 0x0a0a0a } else { 0xffffff };
    let mut t = 1.0 / 40.0;
    while t <= 1.0 {
        let c = mix(fg, target, t);
        if contrast(c, bg) >= min {
            return c;
        }
        t += 1.0 / 40.0;
    }
    fg
}

#[derive(Clone, Copy)]
pub struct Skin {
    pub win: u32,
    pub chrome: u32,
    pub chrome_hi: u32,
    /// 顶部菜单条自己的底色。原来拿 chrome 和 canvas 现算一档，现在独立成槽。
    pub menu: u32,
    /// 下拉弹出菜单的底色。原来跟 chrome_hi 共用，现在单独一槽，菜单条和弹层能各配各的。
    pub popup: u32,
    pub canvas: u32,
    pub pane: u32,
    pub pane_head: u32,
    pub grid: u32,
    pub line: u32,
    pub line_soft: u32,
    pub ink: u32,
    pub ink_dim: u32,
    pub ink_off: u32,
    pub ink_inv: u32,
    pub hov: u32,
    pub prs: u32,
    pub sel: u32,
    pub sel_ink: u32,
    pub acc: u32,
    pub acc_soft: u32,
    pub mark: u32,
    /// 整窗提示纱那一层（剪切/复制按下时过一道），跟 accent 同色系但压到暗档。
    pub flash: u32,
    /// 比 accent 亮一档：深色皮肤上 accent 本身立不住的地方（悬停环、聚焦滑块）用它。
    pub focus: u32,
}

/// 界面上分得出来的十块。字号按块给：改哪一块，只有那一块的字跟着变。
/// 键是写进个人覆盖文件的那一个（`z.<键>=字号`、`zb.<键>=1`），中文名只给设置页看。
pub const ZONES: [(&str, &str); 10] = [
    ("menu", "菜单条"),
    ("bar", "工具栏"),
    ("path", "地址栏"),
    ("tree", "文件夹树"),
    ("pv", "预览详情"),
    ("grid", "缩略图标题"),
    ("org", "图片分类"),
    ("status", "状态栏"),
    ("popup", "弹出菜单"),
    ("dlg", "对话框"),
];
pub const NZ: usize = ZONES.len();
pub const Z_MENU: usize = 0;
pub const Z_BAR: usize = 1;
pub const Z_PATH: usize = 2;
pub const Z_TREE: usize = 3;
pub const Z_PV: usize = 4;
pub const Z_GRID: usize = 5;
pub const Z_ORG: usize = 6;
pub const Z_STATUS: usize = 7;
pub const Z_POPUP: usize = 8;
pub const Z_DLG: usize = 9;

/// 每块的默认字号，100% 缩放下的逻辑值。这就是原来三档摊到十块上的样子：
/// 正文 13 的那些块照旧 13；工具栏、状态栏本来靠小字画，整块给 13，它的小字仍是 12；地址栏等宽 16。
pub const ZONE_DEF: [i32; NZ] = [13, 13, 16, 13, 13, 13, 13, 13, 13, 13];

/// 每一块的正文平时用的是哪一支笔。状态栏、预览详情靠次文画，其余靠正文。
/// 字体页那一枚色块要报的是“这一块现在到底是什么颜色”，所以照这张表取，
/// 不能十块全拿同一支笔糊过去。
pub const ZONE_INK: [&str; NZ] = ["ink", "ink", "ink", "ink", "dim", "ink", "ink", "dim", "ink", "ink"];
/// 一套“在皮肤文件之上的个人改动”。按皮肤名分文件存，换皮肤就换回那套改动；
/// 颜色是槽位覆盖，字体是按版块的字档（逻辑值，100% 缩放）。
#[derive(Clone, Default)]
pub struct Look {
    pub face: Option<String>,
    /// 每块的字号与加粗。没记进表的那块走 ZONE_DEF 的默认。
    pub zone: HashMap<String, i32>,
    pub zone_b: HashMap<String, bool>,
    /// 每块自己的文字主色。没记的那块跟全局“正文”走。
    pub zone_c: HashMap<String, u32>,
    pub colors: HashMap<String, u32>,
}

impl Look {
    pub fn face(&self) -> &str {
        self.face.as_deref().filter(|s| !s.trim().is_empty()).unwrap_or("Microsoft YaHei UI")
    }
    pub fn zone_px(&self, key: &str) -> Option<i32> {
        self.zone.get(key).copied()
    }
    pub fn zone_bold(&self, key: &str) -> bool {
        *self.zone_b.get(key).unwrap_or(&false)
    }
    pub fn zone_color(&self, key: &str) -> Option<u32> {
        self.zone_c.get(key).copied()
    }
    pub fn set_zone_px(&mut self, key: &str, v: i32) {
        self.zone.insert(key.to_string(), v);
    }
    /// 加粗关掉时直接不写这一行，读回来就是不勾。
    pub fn flip_zone_bold(&mut self, key: &str) {
        if self.zone_bold(key) {
            self.zone_b.remove(key);
        } else {
            self.zone_b.insert(key.to_string(), true);
        }
    }
    pub fn reset(&mut self) {
        *self = Look::default();
    }
    pub fn load(skin: &str) -> Look {
        let text = match std::fs::read_to_string(look_file(skin)) {
            Ok(t) => t,
            Err(_) if skin == builtin_name() => BUILTIN_LOOK.to_string(),
            Err(_) => return Look::default(),
        };
        let mut l = Look::default();
        // 旧文件只有正文/小字/路径三档。先收着，读完后如果那份改动还没摊到块上，就替它摊一次，
        // 免得换个设定口径就把他已经调好的字号和加粗静默丢了。
        let mut old: [Option<i32>; 3] = [None, None, None];
        let mut old_b = [false; 3];
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let num = || v.parse::<i32>().ok();
            let on = matches!(v, "1" | "true" | "yes");
            match k {
                "face" => l.face = Some(v.to_string()),
                "font" => old[0] = num(),
                "font_sm" => old[1] = num(),
                "font_path" => old[2] = num(),
                "bold" => old_b[0] = on,
                "bold_sm" => old_b[1] = on,
                "bold_path" => old_b[2] = on,
                _ if k.starts_with("zb.") => {
                    let key = &k[3..];
                    if ZONES.iter().any(|z| z.0 == key) && on {
                        l.zone_b.insert(key.to_string(), true);
                    }
                }
                _ if k.starts_with("z.") => {
                    let key = &k[2..];
                    if ZONES.iter().any(|z| z.0 == key) {
                        if let Some(n) = num() {
                            l.zone.insert(key.to_string(), n);
                        }
                    }
                }
                _ if k.starts_with("zc.") => {
                    let key = &k[3..];
                    if ZONES.iter().any(|z| z.0 == key) {
                        if let Ok(c) = parse_hex(v) {
                            l.zone_c.insert(key.to_string(), c);
                        }
                    }
                }
                _ if k.starts_with("c.") => {
                    if let Ok(c) = parse_hex(v) {
                        l.colors.insert(k[2..].to_string(), c);
                    }
                }
                _ => {}
            }
        }
        // 那份老的三档改动按块摊开：正文管七块，小字管工具栏和状态栏，路径管地址栏。
        // 老文件的默认值正好等于新的每块默认，没调过的人摊完还是原样。
        if l.zone.is_empty() {
            let body = ["menu", "tree", "pv", "grid", "org", "popup", "dlg"];
            let small = ["bar", "status"];
            let path = [ZONES[Z_PATH].0];
            for (keys, tier) in [(&body[..], 0usize), (&small[..], 1), (&path[..], 2)] {
                if let Some(px) = old[tier] {
                    for key in keys {
                        l.zone.insert((*key).to_string(), px);
                    }
                }
                if old_b[tier] {
                    for key in keys {
                        l.zone_b.insert((*key).to_string(), true);
                    }
                }
            }
        }
        l
    }
    pub fn save(&self, skin: &str) {
        let p = look_file(skin);
        let Some(parent) = p.parent() else { return };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let mut out = String::new();
        out.push_str(&format!("# skin={skin}\n"));
        if let Some(face) = &self.face {
            out.push_str(&format!("face={face}\n"));
        }
        for (key, _cn) in ZONES.iter() {
            if let Some(v) = self.zone_px(key) {
                out.push_str(&format!("z.{key}={v}\n"));
            }
            if self.zone_bold(key) {
                out.push_str(&format!("zb.{key}=1\n"));
            }
            if let Some(c) = self.zone_color(key) {
                out.push_str(&format!("zc.{key}=0x{c:06x}\n"));
            }
        }
        let mut colors: Vec<_> = self.colors.iter().collect();
        colors.sort_by(|a, b| a.0.cmp(b.0));
        for (k, c) in colors {
            out.push_str(&format!("c.{k}=0x{c:06x}\n"));
        }
        let _ = std::fs::write(p, out);
    }
}

fn parse_hex(v: &str) -> Result<u32, std::num::ParseIntError> {
    let s = v
        .strip_prefix("0x")
        .or_else(|| v.strip_prefix("0X"))
        .or_else(|| v.strip_prefix('#'))
        .unwrap_or(v);
    u32::from_str_radix(s, 16)
}

fn look_file(skin: &str) -> PathBuf {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    skin.hash(&mut h);
    crate::cache::dir()
        .join("skin_settings")
        .join(format!("{:016x}.tsv", h.finish()))
}

fn layout_file() -> PathBuf {
    crate::cache::dir().join("appearance.tsv")
}

/// 全局版式偏好：None = 内置的圆角仪表（工具栏图标也是内置那一套）；
/// Some = 某套开源图标库的 key（见 glyphs::all()）。
/// 这里故意不放进 Look：Look 是按皮肤保存的个人覆盖，版式要跨配色保留。
pub fn saved_layout() -> Option<String> {
    let Ok(text) = std::fs::read_to_string(layout_file()) else {
        return None;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        if k.trim() != "layout" {
            continue;
        }
        // 老存盘里的 plain/quiet/draft/cards* 一律回到圆角仪表：那些结构档已经删了，
        // 认不出来的一律当内置档，别让界面落回一个画不出来的键。
        return crate::glyphs::find(v.trim()).map(|s| s.key.to_string());
    }
    None
}

/// 启动时穿哪套图标：没写过偏好就用集成进来的那一套，写过就按文件；
/// 文件在但认不出来（老键）仍回内置的圆角仪表，别让界面落回画不出来的键。
pub fn startup_layout() -> Option<String> {
    if layout_file().exists() {
        saved_layout()
    } else {
        Some(DEFAULT_LAYOUT.to_string())
    }
}

/// 写全局版式偏好。None 写内置的圆角仪表。
pub fn save_layout(layout: Option<&str>) {
    let p = layout_file();
    let Some(parent) = p.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let value = layout.unwrap_or("precision");
    let _ = std::fs::write(p, format!("# 版式与配色分开保存\nlayout={value}\n"));
}

/// 校验报告：一条问题 = (槽位对, 要求, 实测)。
pub struct Report(pub Vec<(String, f64, f64)>);

impl Skin {
    /// 读一个槽位，给设置窗画色块用。
    pub fn slot(&self, slot: &str) -> u32 {
        match slot {
            "base" => self.win,
            "chrome" => self.chrome,
            "chrome_hi" => self.chrome_hi,
            "menu" => self.menu,
            "popup" => self.popup,
            "canvas" => self.canvas,
            "pane" => self.pane,
            "pane_head" => self.pane_head,
            "grid" => self.grid,
            "line" => self.line,
            "line_soft" => self.line_soft,
            "ink" => self.ink,
            "dim" => self.ink_dim,
            "off" => self.ink_off,
            "inv" => self.ink_inv,
            "hov" => self.hov,
            "prs" => self.prs,
            "sel" => self.sel,
            "sel_ink" => self.sel_ink,
            "accent" => self.acc,
            "acc_soft" => self.acc_soft,
            "mark" => self.mark,
            "flash" => self.flash,
            "focus" => self.focus,
            _ => 0,
        }
    }

    /// 设置一个槽位。只认 SLOTS 里的名字，颜色改完由 Theme 重建时生效。
    pub fn set_slot(&mut self, slot: &str, c: u32) {
        match slot {
            "base" => self.win = c,
            "chrome" => self.chrome = c,
            "chrome_hi" => self.chrome_hi = c,
            "menu" => self.menu = c,
            "popup" => self.popup = c,
            "canvas" => self.canvas = c,
            "pane" => self.pane = c,
            "pane_head" => self.pane_head = c,
            "grid" => self.grid = c,
            "line" => self.line = c,
            "line_soft" => self.line_soft = c,
            "ink" => self.ink = c,
            "dim" => self.ink_dim = c,
            "off" => self.ink_off = c,
            "inv" => self.ink_inv = c,
            "hov" => self.hov = c,
            "prs" => self.prs = c,
            "sel" => self.sel = c,
            "sel_ink" => self.sel_ink = c,
            "accent" => self.acc = c,
            "acc_soft" => self.acc_soft = c,
            "mark" => self.mark = c,
            "flash" => self.flash = c,
            "focus" => self.focus = c,
            _ => {}
        }
    }

    /// 8 个基色 + 派生。overrides 里的键优先，覆盖本身也是规则的一部分。
    pub fn derive(kv: &HashMap<String, u32>) -> Skin {
        let need = |k: &str| kv.get(k).copied().unwrap_or(0x000000);
        let get = |k: &str, d: u32| kv.get(k).copied().unwrap_or(d);
        let (base, surface, canvas, ink, dim, accent, border, mark) = (
            need("base"),
            need("surface"),
            need("canvas"),
            need("ink"),
            need("dim"),
            need("accent"),
            need("border"),
            need("mark"),
        );
        let inv = if lum(base) > 0.5 { 0xffffff } else { 0x0a0a0a };
        let tint = |over: f64| mix(surface, accent, over);
        let acc = get("accent", accent);
        let chrome_hi = get("chrome_hi", lighten(surface, 0.04));
        Skin {
            win: get("base", base),
            chrome: get("chrome", surface),
            chrome_hi,
            menu: get("menu", mix(surface, canvas, 0.26)),
            popup: get("popup", chrome_hi),
            canvas: get("canvas", canvas),
            pane: get("pane", canvas),
            pane_head: get("pane_head", mix(surface, base, 0.5)),
            grid: get("grid", mix(canvas, ink, 0.07)),
            line: get("line", border),
            line_soft: get("line_soft", mix(border, base, 0.45)),
            ink: get("ink", ink),
            ink_dim: get("dim", ensure(dim, canvas, 3.0)),
            ink_off: get("off", fade(ink, canvas, 2.0, 0.72)),
            ink_inv: get("inv", inv),
            hov: get("hov", tint(0.14)),
            prs: get("prs", tint(0.26)),
            sel: get("sel", tint(0.52)),
            sel_ink: get("sel_ink", pick(tint(0.52), ink, inv)),
            acc,
            acc_soft: get("acc_soft", mix(base, acc, 0.22)),
            mark: get("mark", mark),
            flash: get("flash", mix(base, acc, 0.22)),
            focus: get("focus", lighten(acc, 0.08)),
        }
    }

    /// 按规则跑一遍对比度，不合格就地修文字色，修完还不合格的记进报告。
    pub fn checked(mut self) -> (Skin, Report) {
        let mut bad = Vec::new();
        let mut want = |fg: &mut u32, bg: u32, min: f64, name: &str| {
            if contrast(*fg, bg) >= min {
                return;
            }
            let alt = if lum(bg) > 0.5 { 0x0a0a0a } else { 0xffffff };
            if contrast(alt, bg) >= min {
                *fg = alt;
            } else {
                bad.push((name.to_string(), min, contrast(*fg, bg)));
            }
        };
        want(&mut self.ink, self.canvas, 4.5, "ink/canvas");
        want(&mut self.ink, self.chrome, 4.5, "ink/chrome");
        want(&mut self.ink, self.menu, 4.5, "ink/menu");
        want(&mut self.ink, self.popup, 4.5, "ink/popup");
        want(&mut self.ink_dim, self.canvas, 3.0, "dim/canvas");
        want(&mut self.sel_ink, self.sel, 4.5, "sel_ink/sel");
        want(&mut self.ink, self.win, 4.5, "ink/base");
        let off = ensure(self.ink_off, self.canvas, 2.0);
        if off != self.ink_off && contrast(off, self.canvas) >= 2.0 {
            self.ink_off = off;
        }
        (self, Report(bad))
    }
}

/// 结构档：同一套配色可以按几种工艺画。配色文件里的 `style=` 只是推荐值，
/// 用户可以在外观菜单里换成任意一档；绘制代码只认 Theme.style。
///   Plain = 原样平铺（工具栏文字在图标下方关掉，其余不动）
///   Bands = 明度带 + 单像素蚀刻线，格子保持方角
///   Cards = Bands 之上再套圆角卡片：内容区一块纯色，卡片本体纯色 + 双层边
///   CardsRich = Cards 的完整状态版：文件夹也有卡片，选中/悬停/标记会给底纹、
///               名称带、图标座和角标分层，而不是只描一圈边
///   Quiet = 静谧玻璃：低对比、薄雾、发丝线，整块界面像一块磨砂玻璃
///   Draft = 制图台：结构线、分组框，像一张钉在灯箱上的制图板；颜色全跟配色走
///   Precision = 精密仪器：胶囊按钮、圆环、圆角选框，仪表盘式的细分层
/// 2026-10-23 起外观菜单只剩圆角仪表这一套结构，其余档只留给老 .skin 解析；
/// 菜单里的“版式”改成选工具栏那十枚图标来自哪套开源图标库。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Plain,
    Quiet,
    Draft,
    Precision,
    Bands,
    Cards,
    CardsRich,
    CardsFine,
}

impl Style {
    /// 只要用到卡片/铺底那套结构的，都算卡片档。
    pub fn is_cards(self) -> bool {
        !matches!(self, Style::Plain)
    }

    /// C 精密仪器：胶囊按钮、圆环、圆角选框。
    pub fn is_fine(self) -> bool {
        matches!(self, Style::Precision | Style::CardsFine)
    }

    /// A 静谧玻璃：低对比、薄雾、发丝线。
    pub fn is_quiet(self) -> bool {
        self == Style::Quiet
    }

    /// B 制图台：结构线、分组框。
    pub fn is_draft(self) -> bool {
        self == Style::Draft
    }

    /// 菜单里显示的名字。和皮肤名分开，避免把“配色”与“版式”混成一个东西。
    pub fn label(self) -> &'static str {
        match self {
            Style::Plain => "素面无框",
            Style::Quiet => "薄雾细线",
            Style::Draft => "制图格线",
            Style::Precision => "圆角仪表",
            Style::Bands => "平阶·明度带",
            Style::Cards => "圆角卡片",
            Style::CardsRich => "圆角卡片·完整",
            Style::CardsFine => "圆角卡片·精修",
        }
    }

    /// 存盘用的稳定键名，和 `.skin` 里的 style= 取值一致。
    pub fn key(self) -> &'static str {
        match self {
            Style::Plain => "plain",
            Style::Quiet => "quiet",
            Style::Draft => "draft",
            Style::Precision => "precision",
            Style::Bands => "bands",
            Style::Cards => "cards",
            Style::CardsRich => "cards_rich",
            Style::CardsFine => "cards_fine",
        }
    }

    /// 解析 `.skin` 的 style=。旧档位映射到新四档，
    /// 这样老皮肤文件不用改，选“跟随配色”时也不会再冒出旧版式。
    pub fn from_key(v: &str) -> Option<Style> {
        match v.trim().to_ascii_lowercase().as_str() {
            "plain" | "flat" => Some(Style::Plain),
            "quiet" | "glass" | "a" => Some(Style::Quiet),
            "draft" | "drafting" | "b" => Some(Style::Draft),
            "precision" | "instrument" | "c" => Some(Style::Precision),
            // 旧名字：平阶并入静谧玻璃，所有圆角卡片并入精密仪器。
            "bands" => Some(Style::Quiet),
            "cards" | "cards_rich" | "cards+" | "rich" | "cards_fine" | "fine" | "refined" => {
                Some(Style::Precision)
            }
            _ => None,
        }
    }
}

/// 悬停那一层怎么画，和主窗口 `paint_title` 那四个分支一一对应。画布那一扇是自己
/// 逐像素铺面的，借不到 GDI 的圆角裁剪，所以把这一档一起喂过去让它照着画。
pub const TAG_PLAIN: u32 = 0;
pub const TAG_FINE: u32 = 1;
pub const TAG_QUIET: u32 = 2;
pub const TAG_BANDS: u32 = 3;

/// 标题条这一套口径：底色怎么压、字取哪一枚、按钮多大一颗、条多高，全收在这一处。
/// 主窗口、编辑器、画布那三扇顶层窗和所有浮层都从这里拿，别再各定一套 ——
/// 以前编辑窗的命中带写死 32、浮层写死 30、画布干脆是三个常量，四扇窗四种长相。
#[derive(Clone, Copy, PartialEq)]
pub struct Strip {
    /// 这条的底色
    pub bar: u32,
    /// 底缘那一条 1px
    pub edge: u32,
    /// 压在底色上最读得清的那枚文字色
    pub fg: u32,
    /// 没悬停时的图标色
    pub dim: u32,
    /// 标题那行字的颜色
    pub ink: u32,
    /// 关闭键悬停时铺的那一层（比另外两颗重一档）
    pub hot: u32,
    /// 最小/最大那颗悬停时铺的那一层
    pub hov: u32,
    /// 极简档悬停还要描的那圈边
    pub ring: u32,
    /// 悬停那一层的画法（TAG_* 之一）
    pub tag: u32,
    /// 制图台档：左下压一道短尺
    pub ruler: bool,
    /// 条高
    pub h: i32,
    /// 一颗按钮多宽多高、离右缘多远、两颗之间留多少
    pub bw: i32,
    pub bh: i32,
    pub pad: i32,
    pub gap: i32,
}

impl Strip {
    /// 压在 `bar` 上最读得清的那枚字色。标题栏底色是主题主色，皮肤有深有浅，
    /// 写死 ink 会在某一套皮肤上糊成一片。
    pub fn ink_on(skin: &Skin, bar: u32) -> u32 {
        if contrast(skin.ink, bar) >= contrast(skin.ink_inv, bar) {
            skin.ink
        } else {
            skin.ink_inv
        }
    }

    fn geom(k: f64, h: i32) -> Strip {
        Strip {
            bar: 0,
            edge: 0,
            fg: 0,
            dim: 0,
            ink: 0,
            hot: 0,
            hov: 0,
            ring: 0,
            tag: TAG_PLAIN,
            ruler: false,
            h,
            bw: (34.0 * k).round() as i32,
            bh: (18.0 * k).round() as i32,
            pad: (4.0 * k).round() as i32,
            gap: (2.0 * k).round() as i32,
        }
    }

    /// 顶层窗口的标题条：底色走本主题的主色，也就是文件夹图标那枚深色。
    /// 整条铺纯色，不做渐变。
    pub fn top(th: &Theme) -> Strip {
        let s = th.skin;
        let bar = s.acc;
        let fg = Self::ink_on(&s, bar);
        let mut g = Self::geom(th.scale, th.title);
        g.bar = bar;
        g.fg = fg;
        g.dim = mix(fg, bar, 0.34);
        g.ink = if th.style.is_cards() { fg } else { mix(fg, bar, 0.15) };
        g.hot = mix(bar, fg, 0.34);
        g.hov = mix(bar, fg, 0.18);
        g.ring = mix(bar, fg, 0.28);
        g.ruler = th.style.is_draft();
        g.tag = if th.style.is_fine() {
            TAG_FINE
        } else if th.style.is_quiet() {
            TAG_QUIET
        } else if th.style == Style::Plain {
            TAG_PLAIN
        } else {
            TAG_BANDS
        };
        g.edge = s.line_soft;
        if th.style.is_quiet() {
            (g.hot, g.hov) = (mix(bar, fg, 0.10), mix(bar, fg, 0.10));
        }
        g
    }

    /// 浮在窗口上的那些对话框的标题条。底色、四套版式的分支、按钮几何、悬停那一层
    /// 全都就是顶层那一条（以前它走中性那两档、高度另写一个 30，跟主窗口对不上）。
    /// 只两处差别：条上那行字是粗体，字色取满，不再像顶层那样往底色压半档；
    /// 制图台那把短尺只压在顶层窗上，浮层不压。
    pub fn float(th: &Theme) -> Strip {
        let mut g = Self::top(th);
        g.ink = g.fg;
        g.ruler = false;
        g
    }
}

/// 尺寸令牌（density 只改这一层，和皮肤正交）。
#[derive(Clone)]
pub struct Theme {
    pub skin: Skin,
    /// 当前实际使用的结构档。2026-10-23 起恒为圆角仪表，其余档已从菜单删除。
    pub style: Style,
    /// 工具栏那十枚图标来自哪套开源图标库；None = 内置那一套。
    pub art: Option<&'static crate::glyphs::Set>,
    pub title: i32,
    pub menu: i32,
    pub bar: i32,
    pub path: i32,
    pub status: i32,
    pub row: i32,
    pub pane_head_h: i32,
    pub cell_w: i32,
    pub cell_h: i32,
    pub label: i32,
    pub meta: i32,
    pub icon: i32,
    pub gap: i32,
    pub pad: i32,
    pub split: i32,
    pub font: i32,
    pub font_sm: i32,
    /// 地址栏路径字号：比小字大一点，但不再由代码写死 +4。
    pub font_path: i32,
    /// 十块各自的字号（已按 DPI 放大）与加粗，下标同 ZONES。
    /// 上面那三个是给排版算高度用的代表值：正文取对话框那一块，路径取地址栏那一块。
    pub zf: [(i32, bool); NZ],
    /// 界面字体名。数字/路径仍用等宽字体，避免目录名对不齐。
    pub face: String,
    pub name: String,
    /// 这套尺寸是按哪个 DPI 倍率算的，画界面时要按它换算列宽
    pub scale: f64,
}

/// 默认皮肤名：文件里 `default=1` 指认的那套，没指认就是 order 最靠前的那张。
pub fn default_name() -> String {
    registry().lock().unwrap().default.clone()
}

/// 一张 .skin 读出来的东西：显示名、装饰工艺、菜单次序、是否默认、以及所有颜色键。
struct File {
    name: String,
    style: Style,
    order: i32,
    is_default: bool,
    kv: HashMap<String, u32>,
}

/// 皮肤文件 = 一份 `键=值`，颜色值写 #rrggbb 或 8 位 0x，# 开头整行是注释。
/// name / style / order / default 这四个键不是颜色：style 取 plain|bands|cards|cards_rich|cards_fine，
/// order 决定菜单次序（没写的排最后），default=1 指认启动默认那套。
fn parse(text: &str) -> File {
    let mut f = File {
        name: String::new(),
        style: Style::Plain,
        order: i32::MAX,
        is_default: false,
        kv: HashMap::new(),
    };
    // 记事本存出来的 UTF-8 可能带 BOM，带了就顶在第一条键前面
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "name" => f.name = v.trim_matches('"').to_string(),
            "style" => f.style = Style::from_key(v).unwrap_or(Style::Plain),
            "order" => f.order = v.parse().unwrap_or(i32::MAX),
            "default" => f.is_default = matches!(v, "1" | "true" | "yes"),
            _ => {
                // 只剥一次前缀再解析：早先按字符集 trim 会把 "#000000" 这类纯黑
                // 整串吞成空串，解析失败后这个槽位就静默回落到派生值了。
                let s = v
                    .strip_prefix("0x")
                    .or_else(|| v.strip_prefix("0X"))
                    .or_else(|| v.strip_prefix('#'))
                    .unwrap_or(v);
                if let Ok(c) = u32::from_str_radix(s, 16) {
                    f.kv.insert(k.to_string(), c);
                }
            }
        }
    }
    f
}

/// 读一次就缓存进内存；启动路径上没有文件 IO，只有查这张表。
/// 设置窗能另存为/删除皮肤，所以改完盘要能整张重扫一遍（rescan）。
pub struct Registry {
    pub names: Vec<String>,
    skins: HashMap<String, Skin>,
    styles: HashMap<String, Style>,
    /// 显示名 → 它自己那个 .skin 文件。显示名和文件名可以不一样，改删都得按这张表找盘。
    files: HashMap<String, PathBuf>,
    /// 显示名 → 所在子目录名（"浅色"/"深色"/"莫兰迪"）。菜单按这个分子菜单，
    /// 直接摆在 skins/ 顶层的是空串，列在根菜单上。
    groups: HashMap<String, String>,
    orders: HashMap<String, i32>,
    default: String,
}

static REG: std::sync::OnceLock<std::sync::Mutex<Registry>> = std::sync::OnceLock::new();

/// 出厂自带的一套默认配色。分发包可以只有一个 exe：两处皮肤目录都没有时，
/// 界面也有一整套完整的颜色可穿。盘上要是也有同名的一套，以盘上的为准。
const BUILTIN_SKIN: &str = include_str!("builtin.skin");

/// 内置那套自带的个人改动（字体与每块字号）。盘上还没按皮肤存过那份的时候
/// 用它，出厂看到的就是集成进来的这张脸。
const BUILTIN_LOOK: &str = include_str!("builtin_look.tsv");

/// 内置皮肤叫什么，从它自己的 name= 读，不在别处再抄一份。
fn builtin_name() -> &'static str {
    static N: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    N.get_or_init(|| parse(BUILTIN_SKIN).name)
}

/// 出厂默认的工具栏图标库（见 glyphs::all()）。没有 appearance.tsv 时用它，
/// 和内置配色凑成一整套外观；盘上有偏好文件就照文件来。
pub const DEFAULT_LAYOUT: &str = "lucide";

fn registry() -> &'static std::sync::Mutex<Registry> {
    REG.get_or_init(|| std::sync::Mutex::new(scan()))
}

/// 重扫 skins 目录，把内存里那张表整个换掉。另存为、删除之后必须叫一次，
/// 不然菜单和皮肤查询还认旧的那几套。
pub fn rescan() {
    *registry().lock().unwrap() = scan();
}

fn scan() -> Registry {
    // 皮肤来源：设置窗选定的目录（没选过是 skin_dir() 按默认两级算出的那处）下的 *.skin，
    // 正式的住处是它的「皮肤」子文件夹，分类子目录一层在「皮肤」里面（浅色/深色…），
    // 子目录名就是菜单里的分类名；散在 skins 根上的 .skin 也认，归"未分类"。
    // 显示名仍以文件里的 name= 为准。图标库（图标\）不在这里扫。
    let dir = skin_dir();
    let inner = dir.join("皮肤");
    let mut files: Vec<(PathBuf, File, String)> = Vec::new();
    let mut buckets = vec![dir.clone()];
    if inner.is_dir() {
        buckets.push(inner.clone());
        if let Ok(rd) = std::fs::read_dir(&inner) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    buckets.push(p);
                }
            }
        }
    }
    for bucket in buckets {
        let group = if bucket == dir {
            String::new()
        } else {
            bucket
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        };
        let Ok(rd) = std::fs::read_dir(&bucket) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() || p.extension().and_then(|x| x.to_str()) != Some("skin") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let mut f = parse(&text);
            if f.name.is_empty() {
                f.name = p
                    .file_stem()
                    .and_then(|x| x.to_str())
                    .unwrap_or("?")
                    .to_string();
            }
            files.push((p, f, group.clone()));
        }
    }
    // 内置那一套也混进同一份清单：排序、重名、默认三件事跟盘上的皮肤走同一条
    // 规则，只是它没有盘上的路径——覆盖/删除要按"内置"处理。
    let builtin = parse(BUILTIN_SKIN);
    if !builtin.name.is_empty() && !files.iter().any(|(_, f, _)| f.name == builtin.name) {
        files.push((PathBuf::new(), builtin, String::new()));
    }
    files.sort_by(|a, b| {
        a.1.order
            .cmp(&b.1.order)
            .then_with(|| a.1.name.cmp(&b.1.name))
    });
    let mut names = Vec::new();
    let mut skins: HashMap<String, Skin> = HashMap::new();
    let mut styles: HashMap<String, Style> = HashMap::new();
    let mut paths: HashMap<String, PathBuf> = HashMap::new();
    let mut groups: HashMap<String, String> = HashMap::new();
    let mut orders: HashMap<String, i32> = HashMap::new();
    let mut default = String::new();
    for (path, f, group) in files {
        if default.is_empty() && f.is_default {
            default = f.name.clone();
        }
        if skins.contains_key(&f.name) {
            continue;
        }
        names.push(f.name.clone());
        styles.insert(f.name.clone(), f.style);
        orders.insert(f.name.clone(), f.order);
        groups.insert(f.name.clone(), group);
        if !path.as_os_str().is_empty() {
            paths.insert(f.name.clone(), path);
        }
        let skin = Skin::derive(&f.kv).checked().0;
        skins.insert(f.name, skin);
    }
    if default.is_empty() {
        default = names.first().cloned().unwrap_or_default();
    }
    Registry {
        names,
        skins,
        styles,
        files: paths,
        groups,
        orders,
        default,
    }
}

pub fn skins() -> Vec<String> {
    registry().lock().unwrap().names.clone()
}

/// 这套皮肤归在哪个子目录下；直接摆在 skins/ 顶层的返回空串。
pub fn skin_group(name: &str) -> String {
    registry()
        .lock()
        .unwrap()
        .groups
        .get(name)
        .cloned()
        .unwrap_or_default()
}

pub fn skin_of(name: &str) -> Skin {
    let g = registry().lock().unwrap();
    // 名字对不上（文件被改名或删了）就穿默认那张，别把界面留成一片黑
    g.skins
        .get(name)
        .copied()
        .or_else(|| g.skins.get(g.default.as_str()).copied())
        .unwrap_or_else(|| Skin::derive(&HashMap::new()).checked().0)
}

/// 皮肤目录：设置窗里单独指过就按它；没指过时先看 exe 旁的 skins/（便携包自带皮肤），
/// 那里没有才用系统目录。只认盘上已经存在的目录——程序不会替使用者去建这个文件夹。
pub fn skin_dir() -> PathBuf {
    if let Some(p) = pref_str("skins_dir") {
        return PathBuf::from(p);
    }
    if let Some(exe) = std::env::current_exe().ok() {
        if let Some(dir) = exe.parent() {
            let side = dir.join("skins");
            if dir_has_skins(&side) {
                return side;
            }
        }
    }
    system_skin_dir()
}

/// 系统目录里的那份皮肤：%LOCALAPPDATA%\xsee\skins，拿不到 LOCALAPPDATA 时从 USERPROFILE 推。
/// 取不到任何一个时退回相对路径 skins，与旧行为一致。
fn system_skin_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(|u| PathBuf::from(u).join("AppData").join("Local"))
        });
    match base {
        Some(b) => b.join("xsee").join("skins"),
        None => PathBuf::from("skins"),
    }
}

/// 这个目录里到底有没有皮肤：顶层和一层色系子目录都算。
/// 空壳目录不能算有，否则 exe 旁留了个空的 skins 会把系统目录挡在门外。
fn dir_has_skins(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut subdirs = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            subdirs.push(p);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("skin")) {
            return true;
        }
    }
    subdirs.iter().any(|d| has_skin_two_levels(d))
}

/// 分类子目录现在住在「皮肤」里面（skins\皮肤\浅色\*.skin），往里再多看一层。
fn has_skin_two_levels(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if has_skin_two_levels(&p) {
                return true;
            }
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("skin")) {
            return true;
        }
    }
    false
}

/// 现在的皮肤目录是不是没在设置窗里单独指定过。
pub fn skin_dir_is_default() -> bool {
    pref_str("skins_dir").is_none()
}

/// 改皮肤目录：记住选择再整表重扫。传 None 就退回默认那两级。
pub fn set_skin_dir(dir: Option<&std::path::Path>) -> Result<(), String> {
    match dir {
        Some(d) => {
            if !d.is_dir() {
                return Err("那个文件夹不存在".to_string());
            }
            pref_put("skins_dir", Some(&d.display().to_string()));
        }
        None => pref_put("skins_dir", None),
    }
    rescan();
    Ok(())
}

const UI_KEY: &str = "Software\\Xsee\\UI";

/// 界面设置的对外读写口。键名只有 `UI_KEY` 这一处，别的模块要存偏好就走这里，
/// 不要在各自文件里再抄一遍路径。
pub fn ui_get(name: &str) -> Option<String> {
    pref_str(name)
}

pub fn ui_put(name: &str, val: Option<&str>) {
    pref_put(name, val)
}

fn wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// 读一条 REG_SZ 设置。没这条键、值为空都算"没记过"。
fn pref_str(name: &str) -> Option<String> {
    unsafe {
        let key = wide_z(UI_KEY);
        let mut hk = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), None, KEY_READ, &mut hk).0 != 0 {
            return None;
        }
        let wn = wide_z(name);
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
        let mut wide = Vec::new();
        for c in buf[..sz as usize].chunks_exact(2) {
            let w = u16::from_le_bytes([c[0], c[1]]);
            if w == 0 {
                break;
            }
            wide.push(w);
        }
        let s = String::from_utf16_lossy(&wide);
        (!s.trim().is_empty()).then_some(s)
    }
}

/// 写一条 REG_SZ 设置；None 表示删掉这条，回到缺省。
fn pref_put(name: &str, val: Option<&str>) {
    unsafe {
        let key = wide_z(UI_KEY);
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
        .0 != 0
        {
            return;
        }
        let wn = wide_z(name);
        match val {
            Some(v) => {
                let bytes: Vec<u8> = wide_z(v).iter().flat_map(|w| w.to_le_bytes()).collect();
                let _ = RegSetValueExW(hk, PCWSTR(wn.as_ptr()), None, REG_SZ, Some(&bytes));
            }
            None => {
                let _ = RegDeleteValueW(hk, PCWSTR(wn.as_ptr()));
            }
        }
        let _ = RegCloseKey(hk);
    }
}

/// 这套皮肤在盘上的文件。显示名和文件名可以不一样，删除必须按这张表找。
pub fn skin_path(name: &str) -> Option<PathBuf> {
    registry().lock().unwrap().files.get(name).cloned()
}

/// 皮肤自带的装饰工艺、菜单次序、是不是默认那一套。覆盖和另存为要照抄这三项。
pub fn meta_of(name: &str) -> Option<(Style, i32, bool)> {
    let g = registry().lock().unwrap();
    let style = *g.styles.get(name)?;
    let order = *g.orders.get(name).unwrap_or(&i32::MAX);
    Some((style, order, g.default == name))
}

pub fn has_skin(name: &str) -> bool {
    registry().lock().unwrap().skins.contains_key(name)
}

/// 草稿预览：已生效那张皮肤，叠上还没点确定的槽位改动。设置窗里的色块读它，
/// 不然改了颜色要等确定才看得见，点色块那一下就没有反馈。
pub fn preview(base: &Skin, draft: &Look) -> Skin {
    let mut s = *base;
    for (k, v) in &draft.colors {
        s.set_slot(k, *v);
    }
    s
}

/// 把一套皮肤整个写成文件。24 个槽位全写死：读回来和屏幕上看到的完全一致，
/// 不给派生留余地，否则覆盖一次皮肤颜色就会自己漂掉。
pub fn write_skin(name: &str, style: Style, order: i32, def: bool, skin: &Skin) -> Result<PathBuf, String> {
    let dir = skin_dir().join("皮肤");
    if std::fs::create_dir_all(&dir).is_err() {
        return Err("皮肤目录打不开".to_string());
    }
    let mut stem: String = name
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
        .collect();
    while stem.ends_with('.') || stem.ends_with(' ') {
        stem.pop();
    }
    if stem.is_empty() {
        return Err("这套皮肤的名字只剩符号，做不出文件名".to_string());
    }
    // 覆盖已有皮肤时写回它原来那个文件（可能在色系子目录里）；
    // 只有另存成新名字才落到 skins\皮肤\ 顶层。
    let p = skin_path(name).unwrap_or_else(|| dir.join(format!("{stem}.skin")));
    let mut out = String::new();
    out.push_str("# 皮肤设置窗写出：24 个槽位全写死，读回来就是当时看到的那张脸。\n");
    out.push_str(&format!(
        "name={name}\nstyle={}\norder={order}\ndefault={}\n",
        style.key(),
        u8::from(def)
    ));
    for k in SLOTS {
        out.push_str(&format!("{k}=0x{:06x}\n", skin.slot(k)));
    }
    // surface / border 只对派生有意义，这里全显式给了，写它们只是让人看懂底子来自哪。
    out.push_str(&format!("surface=0x{:06x}\nborder=0x{:06x}\n", skin.chrome, skin.line));
    std::fs::write(&p, out).map_err(|e| format!("写文件失败：{e}"))?;
    Ok(p)
}

/// 删掉一套皮肤：连它自己的 .skin 和按皮肤名存的个人改动一起删。
pub fn delete_skin(name: &str) -> Result<(), String> {
    let p = skin_path(name).ok_or_else(|| {
        if has_skin(name) {
            "这是内置皮肤，删不掉".to_string()
        } else {
            "盘上没有这套皮肤".to_string()
        }
    })?;
    if std::fs::remove_file(&p).is_err() {
        return Err("删不掉，文件可能正被别的程序占用".to_string());
    }
    delete_look(name);
    rescan();
    Ok(())
}

pub fn delete_look(skin: &str) {
    let _ = std::fs::remove_file(look_file(skin));
}

/// 校验一套皮肤文件，给 skin.exe 当把关台。
pub fn audit(name: &str) -> (Skin, Vec<String>) {
    let mut out = Vec::new();
    let s = skin_of(name);
    for (fg, bg, min, what) in [
        (s.ink, s.canvas, 4.5, "正文/画布"),
        (s.ink, s.chrome, 4.5, "正文/工具条"),
        (s.ink, s.menu, 4.5, "正文/菜单条"),
        (s.ink, s.popup, 4.5, "正文/弹出菜单"),
        (s.ink_dim, s.canvas, 3.0, "次文/画布"),
        (s.sel_ink, s.sel, 4.5, "选中文字/选中底"),
        (s.ink, s.win, 4.5, "正文/窗底"),
        (s.ink_off, s.canvas, 2.0, "灰显/画布"),
    ] {
        let c = contrast(fg, bg);
        if c < min {
            out.push(format!("{what} 对比度 {c:.2} 不够 {min}"));
        }
    }
    if s.canvas == s.chrome && s.chrome == s.win {
        out.push("窗底/工具条/画布三层同色，界面会糊成一片".to_string());
    }
    (s, out)
}

impl Theme {
    pub fn new(skin: &str, density: &str, scale: f64) -> Theme {
        Theme::new_with(skin, density, scale, &Look::default())
    }

    /// 皮肤文件给底色，Look 给个人覆盖。先算尺寸，再套字号，最后按 DPI 放大。
    pub fn new_with(skin: &str, density: &str, scale: f64, look: &Look) -> Theme {
        Theme::new_with_layout(skin, density, scale, look, None)
    }

    /// 和 new_with 相同，但允许把工具栏图标换成某套开源图标库。
    /// `layout` 是 glyphs::all() 里的 key；None 或认不出来的键都用内置图标。
    pub fn new_with_layout(
        skin: &str,
        density: &str,
        scale: f64,
        look: &Look,
        layout: Option<&str>,
    ) -> Theme {
        // 结构一律圆角仪表；版式现在只剩“工具栏图标用哪套开源库”这一层差别。
        let art = layout.and_then(crate::glyphs::find);
        let mut t = Theme {
            skin: skin_of(skin),
            style: Style::Precision,
            art,
            title: 22,
            menu: 20,
            // 52：给悬停那圈 2 像素的框留出空隙，框线不再贴着图标和字。
            bar: 52,
            path: 25,
            status: 20,
            row: 20,
            pane_head_h: 19,
            cell_w: 173,
            cell_h: 119,
            label: 18,
            meta: 14,
            icon: 26,
            gap: 6,
            pad: 3,
            split: 3,
            font: 13,
            font_sm: 12,
            font_path: 16,
            zf: [(13, false); NZ],
            face: look.face().to_string(),
            name: skin.to_string(),
            scale: 1.0,
        };
        match density {
            "loose" => {
                t.menu = 24;
                t.bar = 62;
                t.path = 30;
                t.status = 24;
                t.row = 24;
                t.cell_w = 216;
                t.cell_h = 150;
                t.gap = 9;
                t.label = 20;
                t.meta = 16;
                t.font = 14;
                t.font_sm = 13;
            }
            _ => {}
        }
        for (k, v) in &look.colors {
            t.skin.set_slot(k, *v);
        }
        for v in [
            &mut t.title,
            &mut t.menu,
            &mut t.bar,
            &mut t.path,
            &mut t.status,
            &mut t.row,
            &mut t.pane_head_h,
            &mut t.cell_w,
            &mut t.cell_h,
            &mut t.label,
            &mut t.meta,
            &mut t.icon,
            &mut t.gap,
            &mut t.pad,
            &mut t.split,
        ] {
            *v = (*v as f64 * scale).round() as i32;
        }
        // 十块逐块一档：先按 100% 的逻辑值定每块的字号，再统一乘 DPI。
        // 松版式每块 +1，就是原来"正文 14 / 小字 13"那个数。
        let bump = if density == "loose" { 1 } else { 0 };
        let mut zf = [(13, false); NZ];
        for (i, (key, _cn)) in ZONES.iter().enumerate() {
            let px = look.zone_px(key).unwrap_or(ZONE_DEF[i] + bump).clamp(8, 40);
            zf[i] = ((px as f64 * scale).round() as i32, look.zone_bold(key));
        }
        t.zf = zf;
        // 上面那三个只拿来算排版高度：正文取对话框那一块，小字永远比正文小 1，路径取地址栏那一块。
        t.font = zf[Z_DLG].0;
        t.font_sm = (zf[Z_DLG].0 - 1).max(8);
        t.font_path = zf[Z_PATH].0;
        // 只撑动使用者真改过的那一块；一块没动时，布局和今天一个像素都不差。
        let k = scale;
        let pad = (8.0 * k) as i32;
        let touched = |i: usize| look.zone_px(ZONES[i].0).is_some();
        if touched(Z_TREE) || touched(Z_POPUP) {
            t.row = t.row.max(zf[Z_TREE].0.max(zf[Z_POPUP].0) + pad);
        }
        if touched(Z_MENU) {
            t.menu = t.menu.max(zf[Z_MENU].0 + pad);
        }
        if touched(Z_STATUS) {
            t.status = t.status.max(zf[Z_STATUS].0 + pad);
        }
        if touched(Z_ORG) || touched(Z_PV) {
            t.pane_head_h = t.pane_head_h.max(zf[Z_ORG].0.max(zf[Z_PV].0) + pad);
        }
        if touched(Z_PATH) {
            t.path = t.path.max(zf[Z_PATH].0 + pad);
        }
        if touched(Z_BAR) {
            t.bar = t.bar.max(t.icon + zf[Z_BAR].0 + (5.0 * k) as i32);
        }
        if touched(Z_GRID) {
            t.label = t.label.max(zf[Z_GRID].0 + pad);
            t.meta = t.meta.max(zf[Z_GRID].0);
        }
        t.scale = scale;
        t
    }
}

pub fn cref(c: u32) -> COLORREF {
    COLORREF(((c & 0xff) << 16) | (c & 0xff00) | (c >> 16))
}

/// 拿皮肤画一个迷你浏览窗口 + 一条色带。新皮肤好不好先看这张，不用启动整个程序。
pub fn mock(s: &Skin) -> image::DynamicImage {
    use image::{Rgb, RgbImage};
    let (w, h) = (560u32, 320u32);
    let mut img = RgbImage::from_pixel(w, h, Rgb([0, 0, 0]));
    let put = |img: &mut RgbImage, x: u32, y: u32, ww: u32, hh: u32, c: u32| {
        let (r, g, b) = (((c >> 16) & 255) as u8, ((c >> 8) & 255) as u8, (c & 255) as u8);
        for yy in y.min(h)..(y + hh).min(h) {
            for xx in x.min(w)..(x + ww).min(w) {
                img.put_pixel(xx, yy, Rgb([r, g, b]));
            }
        }
    };
    put(&mut img, 0, 0, w, h, s.win);
    put(&mut img, 8, 8, w - 16, 30, s.chrome); // 工具条
    put(&mut img, 8, 42, w - 16, 20, s.chrome_hi); // 路径条
    put(&mut img, 14, 12, 70, 22, s.acc_soft);
    put(&mut img, 90, 12, 70, 22, s.hov);
    put(&mut img, 12, 68, 120, h - 108, s.pane); // 文件夹格
    for i in 0..6 {
        let y = 74 + i * 22;
        put(&mut img, 16, y, 112, 20, if i == 1 { s.sel } else { s.pane });
        put(
            &mut img,
            20,
            y + 4,
            12,
            12,
            if i == 1 { s.sel_ink } else { s.mark },
        );
    }
    put(&mut img, 136, 68, w - 264, h - 108, s.canvas); // 格子区
    for r in 0..3 {
        for c in 0..4 {
            let x = 146 + c * 96;
            let y = 78 + r * 74;
            let on = r == 0 && c == 1;
            put(&mut img, x, y, 90, 70, if on { s.sel } else { s.grid });
            put(
                &mut img,
                x + 8,
                y + 6,
                74,
                40,
                [s.acc, s.mark, s.line_soft, s.hov][(r * 4 + c) as usize % 4],
            );
            put(&mut img, x + 14, y + 50, 62, 6, s.ink_dim);
            put(&mut img, x + 22, y + 60, 46, 5, if on { s.sel_ink } else { s.ink });
        }
    }
    put(&mut img, w - 124, 68, 116, h - 108, s.pane); // 预览/信息格
    put(&mut img, w - 118, 78, 104, 90, s.canvas);
    put(&mut img, w - 110, 178, 88, 6, s.ink);
    put(&mut img, w - 110, 190, 70, 6, s.ink_dim);
    put(&mut img, w - 110, 202, 78, 6, s.ink_off);
    put(&mut img, 8, h - 36, w - 16, 20, s.chrome); // 状态条
    put(&mut img, w - 70, h - 33, 56, 14, s.acc);
    let cw = (w - 16) / SLOTS.len() as u32;
    let vals = [
        s.win,
        s.chrome,
        s.chrome_hi,
        s.menu,
        s.popup,
        s.canvas,
        s.pane,
        s.pane_head,
        s.grid,
        s.line,
        s.line_soft,
        s.ink,
        s.ink_dim,
        s.ink_off,
        s.ink_inv,
        s.hov,
        s.prs,
        s.sel,
        s.sel_ink,
        s.acc,
        s.acc_soft,
        s.mark,
        s.flash,
        s.focus,
    ];
    assert_eq!(vals.len(), SLOTS.len(), "色带必须和 SLOTS 一一对应");
    for (i, c) in vals.iter().enumerate() {
        put(&mut img, 8 + i as u32 * cw, h - 12, cw, 8, *c);
    }
    image::DynamicImage::ImageRgb8(img)
}

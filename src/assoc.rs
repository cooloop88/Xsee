//! 文件关联：全部只写 HKCU\Software\Classes，不碰 HKLM，所以不需要管理员权限。
//! Win10/11 不允许程序静默改默认应用，这里能做的只是"登记成候选"：
//! 每个扩展名的 OpenWithProgids 里挂一条本格式的 ProgID，图片类型再挂一条右键动词。
//! 取消勾选只删本程序写的那几条值，别的程序的候选原样留着。
//!
//! 一个格式一个 ProgID（Xsee.jpg / Xsee.png …），各自带自己的 DefaultIcon：
//! 资源号是格式表下标加 101，和 `fmticon` 画的格式牌一一对应。
//!
//! 关联会被抢走：别的看图软件安装时把「打开方式」候选清掉，卸载和清理工具把
//! `Xsee.*` 整棵 ProgID 树删了，exe 挪过位置之后 command 也就指向空头。所以点「应用」
//! 时顺手记下勾中的那批扩展名（注册表 `assoc_exts`），启动时按这份清单复查一次
//! （开关 `assoc_check`，默认开）。清单为空就是从没关联过，那时一条都不写。

use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, KEY_WRITE, REG_NONE, REG_OPTION_NON_VOLATILE,
    REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegDeleteValueW, RegOpenKeyExW,
    RegQueryValueExW, RegSetValueExW,
};

use crate::fmticon::FMTS;
use crate::theme;

/// 每个格式的 ProgID 前缀，后面接小写格式名（`Xsee.jpg`、`Xsee.raw`）。
const PROGID_PREFIX: &str = "Xsee.";
/// 老版本所有扩展名共用的 ProgID。新版不再写它，只负责迁移和清理；
/// 树本身先留着，万一某个扩展名的默认应用还指着它，文件照样能打开。
const LEGACY_PROGID: &str = "Xsee.Image";
/// 格式牌在 exe 里的资源号起点，顺序同 `FMTS`。
const ICON_BASE: u32 = 101;
/// `Applications` 下面挂的是带扩展名的 exe 名，和 bin 名一致。
const APP_KEY: &str = "Applications\\xsee.exe";
/// 右键动词：对所有 image 类型生效，比逐个扩展名挂 verb 省事。
const SFA_KEY: &str = "SystemFileAssociations\\image\\shell\\Xsee";
const VERB_NAME: &str = "用 Xsee 打开";

/// 本程序能解码的扩展名。清单口径和 `scan::is_image` 完全一致：那边是"这一格出不出缩略图"，
/// 这里要"双击能不能看"，多一个少一个都会对不上。所以下面只列候选，最后统一拿
/// `scan::is_image` 过滤一遍，真正以它为准。
const EXTS: &[&str] = &[
    "jpg", "jpeg", "jpe", "png", "bmp", "gif", "tif", "tiff", "ico", "jfif", "webp", "heic",
    "psd", "psb", "nef", "cr2", "cr3", "arw", "rw2", "dng", "orf", "raf", "pef", "srw", "x3f",
    "3fr", "fff", "iiq",
    // image 库默认解码器段的格式，is_image 认了这边跟着列。
    "avif", "tga", "dds", "qoi", "exr", "hdr", "ff", "pbm", "pgm", "ppm", "pnm", "pam",
    // JPEG XR（WIC 内置）+ 2026-10-26 对照 ACDSee 补齐的 RAW 后缀（走 is_raw）。
    "wdp", "hdp", "jxr", "crw", "nrw", "rwl", "mrw", "kdc", "k25", "dcr", "erf", "mos", "srf",
    "sr2", "raw",
];

pub fn supported_exts() -> Vec<&'static str> {
    EXTS.iter()
        .copied()
        .filter(|e| crate::scan::is_image(&format!("x.{e}")))
        .collect()
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// Win32 错误码翻成人能看的一句话。系统语言是中文时就是中文。
fn err_msg(code: u32) -> String {
    if code == 0 {
        return "未知错误".to_string();
    }
    std::io::Error::from_raw_os_error(code as i32).to_string()
}

fn sub_path(sub: &str) -> String {
    format!("Software\\Classes\\{sub}")
}

fn create(sub: &str) -> Result<HKEY, String> {
    let p = sub_path(sub);
    let w = wide(&p);
    let mut hk = HKEY::default();
    // 只申请 KEY_WRITE。老的 RegCreateKeyW 要的是 KEY_ALL_ACCESS，文件名关联这类键上
    // 会被系统直接拒（拒绝访问），换成 Ex 版把权限收窄才写得进去。
    let e = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(w.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hk,
            None,
        )
    };
    if e.0 == 0 { Ok(hk) } else { Err(err_msg(e.0)) }
}

/// 只读打开；键不存在不算错，调用方按"没有"处理。
fn open_read(sub: &str) -> Option<HKEY> {
    let p = sub_path(sub);
    let w = wide(&p);
    let mut hk = HKEY::default();
    let e = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(w.as_ptr()), None, KEY_READ, &mut hk) };
    if e.0 == 0 { Some(hk) } else { None }
}

fn close(hk: HKEY) {
    unsafe {
        let _ = RegCloseKey(hk);
    }
}

fn set_sz(hk: HKEY, name: Option<&str>, val: &str) -> Result<(), String> {
    let bytes: Vec<u8> = val.encode_utf16().chain([0]).flat_map(|w| w.to_le_bytes()).collect();
    let wname = name.map(wide);
    let np = wname.as_ref().map_or(PCWSTR::null(), |w| PCWSTR(w.as_ptr()));
    let e = unsafe { RegSetValueExW(hk, np, None, REG_SZ, Some(&bytes)) };
    if e.0 == 0 { Ok(()) } else { Err(err_msg(e.0)) }
}

/// 空数据、REG_NONE 的值。OpenWithProgids 和 SupportedTypes 里认的就是这种"只有名字"的值。
fn set_empty(hk: HKEY, name: &str) -> Result<(), String> {
    let w = wide(name);
    let e = unsafe { RegSetValueExW(hk, PCWSTR(w.as_ptr()), None, REG_NONE, None) };
    if e.0 == 0 { Ok(()) } else { Err(err_msg(e.0)) }
}

fn del_value(hk: HKEY, name: &str) -> Result<(), String> {
    let w = wide(name);
    let e = unsafe { RegDeleteValueW(hk, PCWSTR(w.as_ptr())) };
    // 本来就没有这一条也算删干净了：取消勾选是幂等的。
    if e.0 == 0 || e.0 == ERROR_FILE_NOT_FOUND.0 { Ok(()) } else { Err(err_msg(e.0)) }
}

fn has_value(hk: HKEY, name: &str) -> bool {
    let w = wide(name);
    let mut sz = 0u32;
    let r = unsafe { RegQueryValueExW(hk, PCWSTR(w.as_ptr()), None, None, None, Some(&mut sz)) };
    r.0 == 0
}

/// 读一个键的默认值。键不在、值不在、或是那种只有名字的空调用值，都算"没有"。
/// 复查要靠它分辨"command 指向的还是不是当前这个 exe"，光看键在不在不够。
fn get_sz(sub: &str) -> Option<String> {
    let hk = open_read(sub)?;
    let mut buf = [0u8; 8192];
    let mut sz = buf.len() as u32;
    let r =
        unsafe { RegQueryValueExW(hk, PCWSTR::null(), None, None, Some(buf.as_mut_ptr()), Some(&mut sz)) };
    close(hk);
    if r.0 != 0 || sz < 2 {
        return None;
    }
    let wide: Vec<u16> = buf[..sz as usize]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|w| *w != 0)
        .collect();
    let s = String::from_utf16_lossy(&wide);
    (!s.is_empty()).then_some(s)
}

/// 删掉本程序自己建的那一整棵子键。只有全都不勾或某格式不再勾时才用，别的地方不调。
fn del_tree(sub: &str) {
    let p = sub_path(sub);
    let w = wide(&p);
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(w.as_ptr()));
    }
}

/// `"<exe>" "%1"` 这一句，ProgID、Applications、右键动词三处共用。
fn command_line(exe: &Path) -> String {
    format!("\"{}\" \"%1\"", exe.display())
}

fn write_command(sub: &str, exe: &Path) -> Result<(), String> {
    let hk = create(sub)?;
    let r = set_sz(hk, None, &command_line(exe));
    close(hk);
    r
}

fn norm(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn progid_for(idx: usize) -> String {
    format!("{PROGID_PREFIX}{}", FMTS[idx].label.to_ascii_lowercase())
}

/// 一个扩展名的候选：勾上就建 `OpenWithProgids` 并挂本格式的 ProgID，
/// 取消就只删本程序的几条值。老版本留下的 `Xsee.Image` 值顺手删掉，
/// 右键"打开方式"里不会出现两个 Xsee。
fn set_ext(ext: &str, on: bool) -> Result<(), String> {
    let e = norm(ext);
    if e.is_empty() {
        return Ok(());
    }
    let Some(idx) = crate::fmticon::index(&e) else { return Ok(()) };
    let pid = progid_for(idx);
    if on {
        let hk = create(&format!(".{e}\\OpenWithProgids"))?;
        let a = set_empty(hk, &pid);
        let b = del_value(hk, LEGACY_PROGID);
        close(hk);
        a.and(b)
    } else {
        // 删值要 KEY_SET_VALUE，只读句柄删不动。
        let Some(hk) = open_write(&format!(".{e}\\OpenWithProgids")) else { return Ok(()) };
        let a = del_value(hk, &pid);
        let b = del_value(hk, LEGACY_PROGID);
        close(hk);
        a.and(b)
    }
}

/// 打开已有的键，只要"写值/删值"这一项权限；键不在就当没有，不用建。
fn open_write(sub: &str) -> Option<HKEY> {
    let p = sub_path(sub);
    let w = wide(&p);
    let mut hk = HKEY::default();
    let e = unsafe {
        RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(w.as_ptr()), None, KEY_SET_VALUE, &mut hk)
    };
    if e.0 == 0 { Some(hk) } else { None }
}

/// 这个扩展名现在是不是已经登记给 Xsee 了。`<ext>\OpenWithProgids` 里有本格式的
/// ProgID 或者老版本的 `Xsee.Image`，都算。
pub fn is_associated(ext: &str) -> bool {
    let e = norm(ext);
    if e.is_empty() {
        return false;
    }
    let Some(hk) = open_read(&format!(".{e}\\OpenWithProgids")) else { return false };
    let r = match crate::fmticon::index(&e) {
        Some(idx) => has_value(hk, &progid_for(idx)) || has_value(hk, LEGACY_PROGID),
        None => has_value(hk, LEGACY_PROGID),
    };
    close(hk);
    r
}

fn write_fmt_progid(exe: &Path, idx: usize) -> Result<(), String> {
    let pid = progid_for(idx);
    let name = format!("Xsee {} 图片", FMTS[idx].label);
    let hk = create(&pid)?;
    let r = set_sz(hk, None, &name).and_then(|_| set_sz(hk, Some("FriendlyTypeName"), &name));
    close(hk);
    r?;
    let hk = create(&format!("{pid}\\DefaultIcon"))?;
    let r = set_sz(hk, None, &format!("\"{}\",-{}", exe.display(), ICON_BASE + idx as u32));
    close(hk);
    r?;
    write_command(&format!("{pid}\\shell\\open\\command"), exe)
}

fn write_app(exe: &Path, on_exts: &[String]) -> Result<(), String> {
    write_command(&format!("{APP_KEY}\\shell\\open\\command"), exe)?;
    let hk = create(&format!("{APP_KEY}\\SupportedTypes"))?;
    let mut first: Option<String> = None;
    for ext in on_exts {
        let e = norm(ext);
        if e.is_empty() {
            continue;
        }
        if let Err(m) = set_empty(hk, &format!(".{e}")) {
            first.get_or_insert(m);
        }
    }
    close(hk);
    match first {
        Some(m) => Err(m),
        None => Ok(()),
    }
}

fn write_sfa(exe: &Path) -> Result<(), String> {
    let hk = create(SFA_KEY)?;
    let r = set_sz(hk, None, VERB_NAME);
    close(hk);
    r?;
    write_command(&format!("{SFA_KEY}\\command"), exe)
}

/// 把勾选整批落到注册表。返回的第一条错误就是给界面看的那句话，没失败项就是 Ok。
/// `items` 里是 (扩展名, 要不要关联)，扩展名带不带点都行。
pub fn apply_all(exe: &Path, items: &[(String, bool)]) -> Result<(), String> {
    let mut errs: Vec<String> = Vec::new();
    let mut on_exts: Vec<String> = Vec::new();
    let mut on_fmts: Vec<usize> = Vec::new();
    for (ext, on) in items {
        let e = norm(ext);
        if e.is_empty() {
            continue;
        }
        if *on {
            on_exts.push(e.clone());
            if let Some(i) = crate::fmticon::index(&e) {
                if !on_fmts.contains(&i) {
                    on_fmts.push(i);
                }
            }
        }
        if let Err(m) = set_ext(&e, *on) {
            errs.push(format!(".{e}：{m}"));
        }
    }
    if on_exts.is_empty() {
        // 一个都不勾：把本程序自己建的树全收掉，别人的值一条不碰。
        for i in 0..FMTS.len() {
            del_tree(&progid_for(i));
        }
        for k in [LEGACY_PROGID, APP_KEY, SFA_KEY] {
            del_tree(k);
        }
    } else {
        for &i in &on_fmts {
            if let Err(m) = write_fmt_progid(exe, i) {
                errs.push(format!("{}：{m}", progid_for(i)));
            }
        }
        // 这一批没勾到的格式，上一版留下的 ProgID 树一起收掉。
        for i in 0..FMTS.len() {
            if !on_fmts.contains(&i) {
                del_tree(&progid_for(i));
            }
        }
        if let Err(m) = write_app(exe, &on_exts) {
            errs.push(format!("Applications：{m}"));
        }
        if let Err(m) = write_sfa(exe) {
            errs.push(format!("右键动词：{m}"));
        }
    }
    // 记下这一批勾中的扩展名，启动复查照这份补。全不勾时清单跟着清空：
    // 从没关联过的人不该被程序擅自关联上。
    set_saved_exts(items);
    if errs.is_empty() { Ok(()) } else { Err(errs.join("；")) }
}

/// 上一次点「应用」时勾中的那批扩展名，分号分隔存在界面设置那一处。
pub fn saved_exts() -> Vec<String> {
    theme::ui_get("assoc_exts")
        .map(|s| s.split(';').map(norm).filter(|e| !e.is_empty()).collect())
        .unwrap_or_default()
}

fn set_saved_exts(items: &[(String, bool)]) {
    let on: Vec<String> = items.iter().filter(|(_, o)| *o).map(|(e, _)| norm(e)).collect();
    let s = on.join(";");
    theme::ui_put("assoc_exts", if s.is_empty() { None } else { Some(&s) });
}

/// 启动复查的开关。没记过就算开——默认人人都查一次，关掉得在关联页明确点掉。
pub fn check_on() -> bool {
    theme::ui_get("assoc_check").map(|v| v != "0").unwrap_or(true)
}

pub fn set_check_on(on: bool) {
    theme::ui_put("assoc_check", Some(if on { "1" } else { "0" }));
}

/// 一次复查补了哪些东西，给界面那句话用。
#[derive(Default)]
pub struct Repair {
    /// 补回的「打开方式」候选条数
    pub cands: usize,
    /// 整棵被删、重新建起来的 ProgID 条数
    pub progids: usize,
    /// command 或图标指向不对，改回当前 exe 的次数
    pub repaths: usize,
}

impl Repair {
    pub fn nothing(&self) -> bool {
        self.cands + self.progids + self.repaths == 0
    }
}

/// 老状态没有清单（这版之前点过「应用」）：从盘上还在的 `Xsee.*` ProgID 树反推当初勾了哪些。
/// 一棵都没有就是从没关联过，返回空——启动时一条都不写，不给没关联的人擅自挂上。
fn exts_from_progids() -> Vec<String> {
    let all = supported_exts();
    let mut out: Vec<String> = Vec::new();
    for i in 0..FMTS.len() {
        if get_sz(&progid_for(i)).is_none() {
            continue;
        }
        // 一个格式管着好几个扩展名（jpg / jpeg / jpe 都算 JPG 那一格），整组一起收。
        for e in &all {
            if crate::fmticon::index(e) == Some(i) {
                let s = e.to_string();
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
    }
    out
}

/// 启动时复查一次：上一次勾过的那些格式，候选还在不在、ProgID 树还在不在、
/// command 和格式牌图标指的还是不是现在这个 exe。缺什么补什么，一样都不缺就一行注册表都不写。
pub fn recheck(exe: &Path) -> Repair {
    let mut rep = Repair::default();
    let want = {
        let saved = saved_exts();
        if saved.is_empty() {
            exts_from_progids()
        } else {
            saved
        }
    };
    if want.is_empty() {
        return rep;
    }
    let cmd_want = command_line(exe);
    let mut fmts: Vec<usize> = Vec::new();
    for e in &want {
        let Some(idx) = crate::fmticon::index(e) else { continue };
        if !fmts.contains(&idx) {
            fmts.push(idx);
        }
        // 候选没了就挂回去。只挂着老 `Xsee.Image` 也算没到位，set_ext 会顺手把它删掉。
        let pid = progid_for(idx);
        let missing = match open_read(&format!(".{e}\\OpenWithProgids")) {
            Some(hk) => {
                let has = has_value(hk, &pid);
                close(hk);
                !has
            }
            None => true,
        };
        if missing && set_ext(e, true).is_ok() {
            rep.cands += 1;
        }
    }
    for i in fmts {
        let pid = progid_for(i);
        let icon_want = format!("\"{}\",-{}", exe.display(), ICON_BASE + i as u32);
        // 根键上那个值是显示名，没有就是整棵树被人删了；建起来顺手把三项一起写全。
        let gone = get_sz(&pid).is_none();
        let stale = get_sz(&format!("{pid}\\shell\\open\\command")).as_deref() != Some(cmd_want.as_str())
            || get_sz(&format!("{pid}\\DefaultIcon")).as_deref() != Some(icon_want.as_str());
        if (gone || stale) && write_fmt_progid(exe, i).is_ok() {
            if gone {
                rep.progids += 1;
            } else {
                rep.repaths += 1;
            }
        }
    }
    if get_sz(&format!("{APP_KEY}\\shell\\open\\command")).as_deref() != Some(cmd_want.as_str())
        && write_app(exe, &want).is_ok()
    {
        rep.repaths += 1;
    }
    if get_sz(&format!("{SFA_KEY}\\command")).as_deref() != Some(cmd_want.as_str())
        && write_sfa(exe).is_ok()
    {
        rep.repaths += 1;
    }
    rep
}

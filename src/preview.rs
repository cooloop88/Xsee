//! 档内预览与文件头信息：巨型文件不解全图，直接用它自带的缩略图；
//! 尺寸和方向只读文件头，不为拿两个数字去开一次解码器。

use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::Path;

pub const HEAD: usize = 256 * 1024;

/// 嗅宽高要开头 64KB：SOF 排在所有 APP 段之后，而档内那张 EXIF 缩略图能有几十 KB，
/// 窗口给小了就只能把这类文件的尺寸白送给解码器去报。
const SNIFF: usize = 64 * 1024;
/// 方向标签在 IFD0 里，紧跟 TIFF 头，条目再多也就几百字节——4KB 是几十倍余量。
/// 摆正这条每次出图都要走一遍，所以它单独用这个小窗口，不跟着尺寸读 64KB。
const SNIFF_IFD: usize = 4 * 1024;
/// 补读预览段的长度上限，比这还长的预览不如老实解码。
const MAX_PREVIEW: usize = 64 << 20;
/// RAW 里预览 JPEG 的位置各家差得远：尼康佳能在前部，索尼那条全尺寸的占大半档。
/// 先按 8MB 扫一遍，找不到完整的才放开窗口重扫。
const RAW_WIN: usize = 8 << 20;
const RAW_MAX: usize = 256 << 20;

fn ext(path: &str) -> Option<String> {
    Some(Path::new(path).extension()?.to_str()?.to_ascii_lowercase())
}

/// 相机 RAW 后缀。WIC 一个都不认，档里那条内嵌 JPEG 是唯一能看的东西。
/// 这个判断只这一处，扫描器列不列它们也走它。
/// 2026-10-26 对照 ACDSee 的 RAW 清单补齐 12 个：crw/nrw/rwl/mrw/kdc/k25/dcr/erf/mos/srf/sr2/raw。
pub fn is_raw(path: &str) -> bool {
    matches!(
        ext(path).as_deref(),
        Some("nef")
            | Some("cr2")
            | Some("cr3")
            | Some("arw")
            | Some("rw2")
            | Some("dng")
            | Some("orf")
            | Some("raf")
            | Some("pef")
            | Some("srw")
            | Some("x3f")
            | Some("3fr")
            | Some("fff")
            | Some("iiq")
            | Some("crw")
            | Some("nrw")
            | Some("rwl")
            | Some("mrw")
            | Some("kdc")
            | Some("k25")
            | Some("dcr")
            | Some("erf")
            | Some("mos")
            | Some("srf")
            | Some("sr2")
            | Some("raw")
    )
}

/// Photoshop 档。WIC 不认，全尺寸只能自己解合成图（psd.rs）。
pub fn is_psd(path: &str) -> bool {
    matches!(ext(path).as_deref(), Some("psd") | Some("psb"))
}

/// JPEG 系 + PSD：都带可靠的自带预览段。
pub fn has_preview(path: &str) -> bool {
    matches!(
        ext(path).as_deref(),
        Some("jpg") | Some("jpeg") | Some("jpe") | Some("jfif")
    ) || is_psd(path)
}

pub fn read_head(path: &str, n: usize) -> Option<Vec<u8>> {
    let mut f = File::open(path).ok()?;
    let mut v = vec![0u8; n];
    let mut got = 0usize;
    while got < n {
        match f.read(&mut v[got..]) {
            Ok(0) => break,
            Ok(k) => got += k,
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    v.truncate(got);
    if v.len() >= 16 { Some(v) } else { None }
}

/// 从 off 起读 len 字节。预览被窗口截断时按头里报出的长度补齐。
fn read_at(path: &str, off: usize, len: usize) -> Option<Vec<u8>> {
    if len == 0 || len > MAX_PREVIEW {
        return None;
    }
    let mut f = File::open(path).ok()?;
    f.seek(SeekFrom::Start(off as u64)).ok()?;
    let mut v = vec![0u8; len];
    f.read_exact(&mut v).ok()?;
    Some(v)
}

fn be16(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]) as usize)
}

fn be32(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_be_bytes([*b.get(i)?, *b.get(i + 1)?, *b.get(i + 2)?, *b.get(i + 3)?]) as usize)
}

fn le16(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes([*b.get(i)?, *b.get(i + 1)?]) as usize)
}

fn le32(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes([*b.get(i)?, *b.get(i + 1)?, *b.get(i + 2)?, *b.get(i + 3)?]) as usize)
}

/// TIFF 块的端序，true = 'II' 小端。
#[derive(Clone, Copy)]
struct Be(bool);

impl Be {
    fn u16(self, b: &[u8], i: usize) -> Option<usize> {
        if self.0 { le16(b, i) } else { be16(b, i) }
    }

    fn u32(self, b: &[u8], i: usize) -> Option<usize> {
        if self.0 { le32(b, i) } else { be32(b, i) }
    }
}

/// 找到 TIFF 块起点：整个文件就是 TIFF/RAW 时在 0，JPEG 时在 APP1 的 Exif 载荷。
fn tiff_at(buf: &[u8]) -> Option<(Be, usize)> {
    if buf.starts_with(b"II") || buf.starts_with(b"MM") {
        let e = Be(buf[0] == b'I');
        // 版本 43 是 BigTIFF，条目宽度不一样，这里不当它 TIFF 使。
        if e.u16(buf, 2)? != 42 {
            return None;
        }
        return Some((e, 0));
    }
    if !buf.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut p = 2usize;
    while p + 4 <= buf.len() {
        if buf[p] != 0xFF {
            return None;
        }
        let m = buf[p + 1];
        if m == 0xDA || m == 0xD9 {
            return None;
        }
        if m == 0x01 || (0xD0..=0xD7).contains(&m) {
            p += 2;
            continue;
        }
        let seg = be16(buf, p + 2)?;
        let body = p + 4;
        if m == 0xE1 && buf.get(body..body + 6) == Some(&[b'E', b'x', b'i', b'f', 0, 0]) {
            let tiff = body + 6;
            let e = Be(buf.get(tiff)? == &b'I');
            if e.u16(buf, tiff + 2)? != 42 {
                return None;
            }
            return Some((e, tiff));
        }
        p = body + seg.saturating_sub(2);
    }
    None
}

/// IFD0 起点（TIFF 里所有偏移都相对块起点）。
fn ifd_at(buf: &[u8], e: Be, base: usize) -> Option<usize> {
    let ifd = base + e.u32(buf, base + 4)?;
    if ifd + 2 > buf.len() {
        return None;
    }
    Some(ifd)
}

/// IFD 里查一个 tag 的数值。SHORT 和 LONG 都内联在条目第 9 字节起。
fn ifd_val(buf: &[u8], e: Be, ifd: usize, tag: u16) -> Option<usize> {
    let n = e.u16(buf, ifd)?;
    for k in 0..n {
        let en = ifd + 2 + k as usize * 12;
        if e.u16(buf, en)? != tag as usize {
            continue;
        }
        return match e.u16(buf, en + 2)? {
            3 => e.u16(buf, en + 8),
            4 => e.u32(buf, en + 8),
            _ => None,
        };
    }
    None
}

/// EXIF 方向档 1..=8，读不到当 1（不转）。JPEG 的 APP1、TIFF/RAW 的 IFD0 都走这里。
pub fn orientation(path: &str) -> u16 {
    orient_of(&read_head(path, SNIFF_IFD).unwrap_or_default())
}

fn orient_of(d: &[u8]) -> u16 {
    let Some((e, base)) = tiff_at(d) else {
        return 1;
    };
    let Some(ifd) = ifd_at(d, e, base) else {
        return 1;
    };
    match ifd_val(d, e, ifd, 0x0112) {
        Some(v @ 1..=8) => v as u16,
        _ => 1,
    }
}

/// 一次开文件把宽高和方向都拿走。先按 4KB 读：绝大多数格式的尺寸就在最前面几百字节
/// 里。嗅不到再放开到 64KB 重读一次，只有 EXIF 缩略图很大那批 JPEG 走这一趟。
pub fn sniff_orient(path: &str) -> (Option<(u32, u32)>, u16) {
    let small = read_head(path, SNIFF_IFD).unwrap_or_default();
    if let Some(d) = dims_of(&small) {
        return (Some(d), orient_of(&small));
    }
    let Some(d) = read_head(path, SNIFF) else {
        return (None, orient_of(&small));
    };
    (dims_of(&d), orient_of(&d))
}

/// 只认文件头拿宽高：jpg 的 SOF、png 的 IHDR、gif 的 LSD、bmp 的 DIB、
/// tiff/RAW 的 IFD0、psd 的 26 字节头。认不出返回 None 让调用方回落解码器。
/// webp/heic/ico 不在列：它们的尺寸要么埋在压缩块里，要么和 WIC 挑的那张图不一致，
/// 读错一个数字比多开一次解码器贵。
fn dims_of(d: &[u8]) -> Option<(u32, u32)> {
    // 三个读头函数拿到的都是 32 位以内的数，as u32 不会截掉真值，只挡荒谬数。
    let ok = |w: u32, h: u32| -> Option<(u32, u32)> {
        if w > 0 && h > 0 && w < 1_000_000 && h < 1_000_000 {
            Some((w, h))
        } else {
            None
        }
    };
    if d.starts_with(&[0xFF, 0xD8]) {
        let mut p = 2usize;
        while p + 9 < d.len() {
            if d[p] != 0xFF {
                p += 1;
                continue;
            }
            let m = d[p + 1];
            if m == 0xD8 || (0xD0..=0xD9).contains(&m) {
                p += 2;
                continue;
            }
            let Some(len) = be16(d, p + 2) else { break };
            // SOF0..SOF15 才带帧尺寸，但 DHT(C4)、内码(C8)、DAC(CC) 也在这个号段里。
            if (0xC0..=0xCF).contains(&m) && !matches!(m, 0xC4 | 0xC8 | 0xCC) {
                return ok(be16(d, p + 7)? as u32, be16(d, p + 5)? as u32);
            }
            if len < 2 {
                break;
            }
            p += 2 + len;
        }
        return None;
    }
    if d.starts_with(b"\x89PNG") {
        return ok(be32(d, 16)? as u32, be32(d, 20)? as u32);
    }
    if d.starts_with(b"GIF8") {
        return ok(le16(d, 6)? as u32, le16(d, 8)? as u32);
    }
    if d.starts_with(b"BM") {
        // 高度为负是 top-down 位图，取绝对值。
        return ok(
            (le32(d, 18)? as i32).unsigned_abs(),
            (le32(d, 22)? as i32).unsigned_abs(),
        );
    }
    if d.starts_with(b"8BPS") {
        return ok(be32(d, 18)? as u32, be32(d, 14)? as u32);
    }
    if let Some((e, base)) = tiff_at(d) {
        let ifd = ifd_at(d, e, base)?;
        return ok(
            ifd_val(d, e, ifd, 0x0100)? as u32,
            ifd_val(d, e, ifd, 0x0101)? as u32,
        );
    }
    None
}

/// 返回内嵌 JPEG 在 buf 里的 (起点, 长度)。
pub fn embedded(buf: &[u8]) -> Option<(usize, usize)> {
    if buf.starts_with(b"8BPS") {
        return psd(buf);
    }
    if buf.starts_with(&[0xFF, 0xD8]) {
        return jpeg(buf);
    }
    None
}

/// 把档内预览 JPEG 整段 own 一份出来。窗口截断的按头里报出的长度补读——
/// 半条 JPEG 只会让 WIC 报错或解出坏图，不如多读一次盘。
pub fn embedded_bytes(path: &str) -> Option<Vec<u8>> {
    let head = read_head(path, HEAD)?;
    let (start, len) = embedded(&head)?;
    if start + len <= head.len() {
        return Some(head[start..start + len].to_vec());
    }
    read_at(path, start, len)
}

/// RAW 档里最大的那段连续 JPEG。各家预览存放位置不一样，按 SOI…EOI 扫
/// 比按各家 IFD 猜结构稳；小段（图标级预览）当噪音跳过。
pub fn raw_preview(path: &str) -> Option<Vec<u8>> {
    let len = std::fs::metadata(path).ok()?.len() as usize;
    if len < 1024 || len > RAW_MAX {
        return None;
    }
    if let Some(v) = blob_of(&read_head(path, RAW_WIN.min(len))?) {
        return Some(v);
    }
    if len <= RAW_WIN {
        return None;
    }
    blob_of(&read_head(path, len.min(MAX_PREVIEW))?)
}

fn blob_of(d: &[u8]) -> Option<Vec<u8>> {
    let (s, l) = largest_jpeg(d)?;
    Some(d[s..s + l].to_vec())
}

fn largest_jpeg(d: &[u8]) -> Option<(usize, usize)> {
    let n = d.len();
    let (mut best_s, mut best_e) = (0usize, 0usize);
    let mut i = 0usize;
    while i + 4 <= n {
        if d[i] == 0xFF && d[i + 1] == 0xD8 && d[i + 2] == 0xFF {
            let Some(j) = d[i + 3..].windows(2).position(|w| w == [0xFF, 0xD9]) else {
                break;
            };
            let e = i + 3 + j + 2;
            if e - i > best_e - best_s {
                best_s = i;
                best_e = e;
            }
            i = e;
            continue;
        }
        i += 1;
    }
    if best_e - best_s >= 1024 {
        Some((best_s, best_e - best_s))
    } else {
        None
    }
}

fn psd(buf: &[u8]) -> Option<(usize, usize)> {
    let cmdlen = be32(buf, 26)?;
    let reslen = be32(buf, 30)?;
    let mut p = 34 + cmdlen;
    let end = (p + reslen).min(buf.len());
    while p + 10 <= end {
        if buf[p..p + 4] != *b"8BIM" {
            break;
        }
        let id = be16(buf, p + 4)?;
        let nm = *buf.get(p + 6)? as usize;
        let mut q = p + 7 + nm;
        if q % 2 == 1 {
            q += 1;
        }
        let size = be32(buf, q)?;
        let data = q + 4;
        // 预览块比 256KB 窗口长是常事（全尺寸预览好几 MB），这里只要报出起点和长度，
        // 补齐那段由 embedded_bytes 去做，别因为窗口短就当没有。
        if (id == 0x040C || id == 0x040D) && size > 32 && data < buf.len() {
            let blk = &buf[data..(data + size).min(buf.len())];
            if let Some(o) = blk.windows(3).position(|w| w == [0xFF, 0xD8, 0xFF]) {
                return Some((data + o, size - o));
            }
        }
        p = data + size;
        if p % 2 == 1 {
            p += 1;
        }
    }
    None
}

fn jpeg(buf: &[u8]) -> Option<(usize, usize)> {
    let (e, tiff) = tiff_at(buf)?;
    exif(buf, e, tiff)
}

/// IFD0 的下一个 IFD 就是缩略图 IFD，0x0201/0x0202 给它的相对位置和长度。
fn exif(buf: &[u8], e: Be, tiff: usize) -> Option<(usize, usize)> {
    let ifd0 = ifd_at(buf, e, tiff)?;
    let n = e.u16(buf, ifd0)?;
    let next = ifd0 + 2 + n * 12;
    let ifd1 = tiff + e.u32(buf, next)?;
    if ifd1 <= tiff || ifd1 >= buf.len() {
        return None;
    }
    let n1 = e.u16(buf, ifd1)?;
    let mut off = None;
    let mut len = None;
    for k in 0..n1 {
        let en = ifd1 + 2 + k * 12;
        match e.u16(buf, en) {
            Some(0x0201) => off = e.u32(buf, en + 8),
            Some(0x0202) => len = e.u32(buf, en + 8),
            _ => {}
        }
    }
    let (o, l) = (off?, len?);
    let s = tiff + o;
    // 只要求起点落在窗口里；长度超出窗口的由 embedded_bytes 补读。
    if l > 0 && s >= tiff && s < buf.len() {
        Some((s, l))
    } else {
        None
    }
}

//! PSD/PSB 的合成图（拼合之后那张）自己解。WIC 压根没有 PSD 解码器，
//! 这是这套软件唯一的整格式空洞。
//!
//! 只读图像数据段：头部之后那三节（颜色模式 / 图像资源 / 图层与蒙版）按长度跳过，
//! 不整档进内存——一张 1.1 GB 的档先要 1.1 GB 的堆是白给的，实际要的像素只有 84 MB。

use std::fs::{File, metadata};
use std::io::{BufReader, Read, Seek, SeekFrom};

/// 合成图像素上限。再往下就是几百 MB 的 BGRA，不如退回档内预览那条 1ms 的路。
const MAX_PIXELS: u64 = 120_000_000;

type R = BufReader<File>;

fn u16b(b: &[u8], i: usize) -> usize {
    ((b[i] as usize) << 8) | b[i + 1] as usize
}

fn u32b(b: &[u8], i: usize) -> usize {
    (b[i] as usize) << 24 | (b[i + 1] as usize) << 16 | (b[i + 2] as usize) << 8 | b[i + 3] as usize
}

fn u16(f: &mut R) -> Option<usize> {
    let mut b = [0u8; 2];
    f.read_exact(&mut b).ok()?;
    Some(u16b(&b, 0))
}

fn u32(f: &mut R) -> Option<usize> {
    let mut b = [0u8; 4];
    f.read_exact(&mut b).ok()?;
    Some(u32b(&b, 0))
}

/// 解出 (宽, 高, BGRA)。格式不对、位深/色彩模式不在支持表里、或档撒谎说不下
/// 这么多像素，一律 None，让调用方回落到档内预览。
pub fn decode(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let len = metadata(path).ok()?.len() as usize;
    let mut f = BufReader::with_capacity(1 << 20, File::open(path).ok()?);
    let mut head = [0u8; 26];
    f.read_exact(&mut head).ok()?;
    if head[..4] != *b"8BPS" {
        return None;
    }
    let ver = u16b(&head, 4);
    // 1 = PSD，2 = PSB（大文档）。行字节表的宽度不一样，下面按这个分。
    if ver != 1 && ver != 2 {
        return None;
    }
    let channels = u16b(&head, 12);
    let height = u32b(&head, 14);
    let width = u32b(&head, 18);
    let depth = u16b(&head, 22);
    let mode = u16b(&head, 24);
    // 只吃 8/16 位的 RGB(3) 与 CMYK(4)。灰度、索引、双半调、32 位浮点这些
    // 要么没有预览可回落的代价更大，要么位深换算不划算。
    if !((3..=5).contains(&channels) && (depth == 8 || depth == 16) && (mode == 3 || mode == 4)) {
        return None;
    }
    if mode == 4 && channels < 4 {
        return None;
    }
    if width == 0 || height == 0 || width > 30000 || height > 30000 {
        return None;
    }
    if width as u64 * height as u64 > MAX_PIXELS {
        return None;
    }

    let mut pos = 26usize;
    for _ in 0..3 {
        let n = u32(&mut f)?;
        f.seek(SeekFrom::Current(n as i64)).ok()?;
        pos += 4 + n;
    }
    let comp = u16(&mut f)?;
    if comp > 1 {
        return None;
    }
    pos += 3;

    let sample = depth / 8;
    let row = width * sample;
    let plane = row * height;
    let need = plane.checked_mul(channels)?;
    let mut planes = vec![vec![0u8; plane]; channels];

    if comp == 0 {
        if pos + need > len {
            return None;
        }
        for p in planes.iter_mut() {
            f.read_exact(p).ok()?;
        }
    } else {
        let tw = channels * height * if ver == 2 { 4 } else { 2 };
        let mut table = vec![0u8; tw];
        f.read_exact(&mut table).ok()?;
        let mut packed = Vec::new();
        for c in 0..channels {
            for y in 0..height {
                let n = if ver == 2 {
                    u32b(&table, (c * height + y) * 4)
                } else {
                    u16b(&table, (c * height + y) * 2)
                };
                packed.resize(n, 0);
                f.read_exact(&mut packed).ok()?;
                let dst = &mut planes[c][y * row..][..row];
                if !unpack_bits(&packed, dst) {
                    return None;
                }
            }
        }
    }

    let mut px = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            // 16 位档取大端首字节，就是高 8 位——和 8 位档同一句下标。
            let o = y * row + x * sample;
            let d = (y * width + x) * 4;
            if mode == 3 {
                px[d + 2] = planes[0][o];
                px[d + 1] = planes[1][o];
                px[d] = planes[2][o];
                px[d + 3] = if channels >= 4 { planes[3][o] } else { 255 };
            } else {
                let k = planes[3][o] as u16;
                px[d + 2] = 255 - (planes[0][o] as u16 + k).min(255) as u8;
                px[d + 1] = 255 - (planes[1][o] as u16 + k).min(255) as u8;
                px[d] = 255 - (planes[2][o] as u16 + k).min(255) as u8;
                px[d + 3] = if channels >= 5 { planes[4][o] } else { 255 };
            }
        }
    }
    Some((width as u32, height as u32, px))
}

/// PackBits 逐行解压：>=0 是照抄 n+1 个字节，-127..-1 是把下一个字节重复 1-n 次，
/// -128 是空操作。凑不满一行就是坏档。
fn unpack_bits(src: &[u8], dst: &mut [u8]) -> bool {
    let (mut si, mut di) = (0usize, 0usize);
    while si < src.len() && di < dst.len() {
        let n = src[si] as i8;
        si += 1;
        if n >= 0 {
            let cnt = n as usize + 1;
            if si + cnt > src.len() || di + cnt > dst.len() {
                return false;
            }
            dst[di..di + cnt].copy_from_slice(&src[si..si + cnt]);
            si += cnt;
            di += cnt;
        } else if n != -128 {
            let cnt = 1 - n as usize;
            if si >= src.len() || di + cnt > dst.len() {
                return false;
            }
            for b in &mut dst[di..di + cnt] {
                *b = src[si];
            }
            si += 1;
            di += cnt;
        }
    }
    di == dst.len()
}

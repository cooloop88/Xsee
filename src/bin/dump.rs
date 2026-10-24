/// 把一张图按看图的解码路径出成 PNG，给人眼看解码对不对（PSD 这条是自己新解的，
/// 通道序、平面步长这些只有看图才看得出来）。
/// 用法: dump <输出目录> <宽x高|原尺寸|预览宽x高> <图片>...
/// `预览WxH` 走档内自带的那张拼合预览，拿它当"标准答案"对拍自解出来的颜色。
use std::path::Path;

use image::RgbaImage;

use xsee_host::job;

fn main() {
    let mut it = std::env::args().skip(1);
    let out_dir = it.next().expect("用法: dump <输出目录> <宽x高|原尺寸|预览宽x高> <图片>...");
    let box_arg = it.next().unwrap_or_else(|| "原尺寸".to_string());
    let want_preview = box_arg.starts_with("预览");
    let size = box_arg.trim_start_matches("预览");
    let (bw, bh) = match size.split_once('x') {
        Some((w, h)) => (w.parse().unwrap_or(0), h.parse().unwrap_or(0)),
        None => (0, 0),
    };
    std::fs::create_dir_all(&out_dir).ok();
    job::init_com();
    xsee_host::cache::set_enabled(false);

    for path in it {
        // 预览档：Photoshop 自己写进档里那张拼合预览，拿它当"标准答案"对拍。
        let v = if want_preview {
            match job::embedded_view(&path, bw, bh) {
                Some(v) => Ok(v),
                None => Err(windows::core::Error::new(
                    windows::core::HRESULT(0x80004005_u32 as i32),
                    "档内没预览",
                )),
            }
        } else {
            job::decode_view(&path, bw, bh)
        };
        match v {
            Ok(v) => {
                let mut px = v.px;
                for p in px.chunks_exact_mut(4) {
                    p.swap(0, 2);
                    p[3] = 0xFF;
                }
                let stem = Path::new(&path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("out");
                let tag = if want_preview { "预览" } else { "自解" };
                let out = format!("{out_dir}\\{stem}_{tag}_{}x{}.png", v.w, v.h);
                match RgbaImage::from_raw(v.w, v.h, px) {
                    Some(img) => match img.save(&out) {
                        Ok(_) => println!("{stem} -> {out}"),
                        Err(e) => println!("{stem} 存不下：{e}"),
                    },
                    None => println!("{stem} 像素缓冲对不上 {}x{}", v.w, v.h),
                }
            }
            Err(e) => println!("{path} 解不开：{e}"),
        }
    }
}

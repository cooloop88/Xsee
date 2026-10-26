// 临时探针：并发复现"4 个同名文件 + 编号档只出 1 个"。4 worker 并行 + 同主名源图，
// 和面板 start_conv_panel 的调用方式一致。
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use xsee_host::encode;

fn main() {
    let dir = std::env::temp_dir().join("xsee-conv-race");
    let _ = std::fs::remove_dir_all(&dir);
    let din = dir.join("in");
    let dout = dir.join("out");
    let _ = std::fs::create_dir_all(&din);
    let _ = std::fs::create_dir_all(&dout);

    // 四个同名（同主名不同扩展名）+ 四个不同名，共 8 张
    let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(160, 120, |x, y| {
        image::Rgba([(x % 256) as u8, (y % 200) as u8, 77, 255])
    }));
    let mut srcs = Vec::new();
    for ext in ["png", "jpg", "bmp", "gif"] {
        let p = din.join(format!("同名.{ext}"));
        let f = match ext {
            "png" => image::ImageFormat::Png,
            "jpg" => image::ImageFormat::Jpeg,
            "bmp" => image::ImageFormat::Bmp,
            _ => image::ImageFormat::Gif,
        };
        img.save_with_format(&p, f).unwrap();
        srcs.push(p.display().to_string());
    }
    for i in 0..4 {
        let p = din.join(format!("pic{i}.png"));
        img.save_with_format(&p, image::ImageFormat::Png).unwrap();
        srcs.push(p.display().to_string());
    }
    let mut paths = srcs.clone();
    paths.sort();
    encode::begin_batch();
    let batch: Arc<HashSet<String>> = Arc::new(encode::batch_set(&paths));
    let next = Arc::new(AtomicUsize::new(0));
    let mut hs = Vec::new();
    for _ in 0..4 {
        let paths = paths.clone();
        let batch = batch.clone();
        let next = next.clone();
        let dout = dout.display().to_string();
        hs.push(std::thread::spawn(move || loop {
            let i = next.fetch_add(1, Ordering::SeqCst);
            if i >= paths.len() {
                break;
            }
            let p = &paths[i];
            match encode::convert_file_x(
                p,
                &dout,
                None,
                0,
                &encode::Opt::for_fmt(0),
                encode::Collision::Rename, // 面板上的「编号」档
                None,
                None,
                &batch,
            ) {
                Ok(encode::Outcome::Done(o, _)) => {
                    println!("done: {}", Path::new(&o).file_name().unwrap().to_string_lossy())
                }
                Ok(encode::Outcome::Skipped) => println!("skip: {}", Path::new(p).file_name().unwrap().to_string_lossy()),
                Err(e) => println!("err: {} {e}", Path::new(p).file_name().unwrap().to_string_lossy()),
            }
        }));
    }
    for h in hs {
        let _ = h.join();
    }
    let mut names: Vec<String> = std::fs::read_dir(&dout)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    println!("outputs={} {names:?}", names.len());
    println!("expect 8 distinct (同名.jpg + 同名 (2..5).jpg + pic0-3.jpg)");
}

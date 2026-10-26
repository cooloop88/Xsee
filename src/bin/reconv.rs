// 临时探针：五种格式的往返（自己转出来的图再转一次），照批量转换面板那条调用路径跑。
// 只读写 %TEMP%\xsee-rt，不碰用户目录。
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use xsee_host::encode;

fn inputs(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path().display().to_string())
                .filter(|p| {
            let p = p.to_ascii_lowercase();
                    p.ends_with(".jpg") || p.ends_with(".png") || p.ends_with(".bmp")
                        || p.ends_with(".tif") || p.ends_with(".gif")
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// 一批：像面板那样多线程，返回每张的结果行。
fn batch(label: &str, srcs: &[String], out_dir: &Path, fmt: usize, opt: &encode::Opt, policy: encode::Collision) {
    let _ = std::fs::create_dir_all(out_dir);
    encode::begin_batch();
    let batch_set = Arc::new(encode::batch_set(srcs));
    let srcs = Arc::new(srcs.to_vec());
    let out = Arc::new(out_dir.display().to_string());
    let opt = Arc::new(opt.clone());
    let next = Arc::new(AtomicUsize::new(0));
    let lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let threads = 4.min(srcs.len().max(1));
    let mut hs = Vec::new();
    for _ in 0..threads {
        let srcs = srcs.clone();
        let out = out.clone();
        let opt = opt.clone();
        let next = next.clone();
        let lines = lines.clone();
        let batch_set = batch_set.clone();
        let fmt = fmt;
        hs.push(std::thread::spawn(move || {
            loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= srcs.len() {
                    break;
                }
                let s = &srcs[i];
                let r = encode::convert_file_x(
                    s,
                    &out,
                    None,
                    fmt,
                    &opt,
                    policy,
                    Some(&encode::Xform::default()),
                    None,
                    &HashSet::<String>::from_iter(batch_set.iter().cloned()),
                );
                let name = Path::new(s).file_name().unwrap().to_string_lossy().to_string();
                let line = match r {
                    Ok(encode::Outcome::Done(p, n)) => {
                        format!("      OK   {name} -> {} ({n} B)", Path::new(&p).file_name().unwrap().to_string_lossy())
                    }
                    Ok(encode::Outcome::Skipped) => format!("      SKIP {name}"),
                    Err(e) => format!("      ERR  {name}：{e}"),
                };
                lines.lock().unwrap().push(line);
            }
        }));
    }
    for h in hs {
        let _ = h.join();
    }
    let mut l = lines.lock().unwrap();
    l.sort();
    println!("── {label}");
    for x in l.iter() {
        println!("{x}");
    }
}

fn main() {
    let root = std::env::temp_dir().join("xsee-rt");
    let _ = std::fs::remove_dir_all(&root);
    let din = root.join("in");
    std::fs::create_dir_all(&din).unwrap();
    // 输入：用户那张 a.jpg 的副本 + 两张 PNG 截图
    let _ = std::fs::copy(std::env::temp_dir().join("xsee-reconv-his/a.jpg"), din.join("a.jpg"));
    let _ = std::fs::copy(std::env::temp_dir().join("xsee-reconv-his/his.png"), din.join("his.png"));
    for f in ["shot5.png", "shot5_seam.png"] {
        let _ = std::fs::copy(
            std::env::temp_dir().join("xsee-gui").join(f),
            din.join(f),
        );
    }
    let srcs = inputs(&din);
    println!("输入 {} 张：{:?}", srcs.len(), srcs.iter().map(|p| Path::new(p).file_name().unwrap().to_string_lossy().to_string()).collect::<Vec<_>>());

    let names = ["JPEG", "PNG", "BMP", "TIFF", "GIF"];
    for fmt in 0..5 {
        let o1 = root.join(format!("r1_{fmt}"));
        batch(&format!("第一轮 -> {}", names[fmt]), &srcs, &o1, fmt, &encode::Opt::for_fmt(fmt), encode::Collision::Overwrite);
        let mid = inputs(&o1);
        let o2 = root.join(format!("r2_{fmt}"));
        batch(&format!("第二轮（转出来的再转） -> {}", names[fmt]), &mid, &o2, fmt, &encode::Opt::for_fmt(fmt), encode::Collision::Overwrite);
        // 第三轮：把上一种格式的产物转成 JPEG
        let o3 = root.join(format!("r3_{fmt}"));
        batch(&format!("第三轮 {name} 产物 -> JPEG", name = names[fmt]), &inputs(&o2), &o3, 0, &encode::Opt::for_fmt(0), encode::Collision::Overwrite);
    }

    // 面板上能勾的两个特殊档
    let mut prog = encode::Opt::for_fmt(0);
    prog.progressive = true;
    let p1 = root.join("prog1");
    batch("JPEG 渐进式 第一轮", &srcs, &p1, 0, &prog, encode::Collision::Overwrite);
    let p2 = root.join("prog2");
    batch("JPEG 渐进式 第二轮（自己的产物）", &inputs(&p1), &p2, 0, &prog, encode::Collision::Overwrite);

    let mut dcf = encode::Opt::for_fmt(0);
    dcf.dcf = true;
    dcf.thumb = 2;
    let d1 = root.join("dcf1");
    batch("JPEG 带 DCF 缩略图 第一轮", &srcs, &d1, 0, &dcf, encode::Collision::Overwrite);
    let d2 = root.join("dcf2");
    batch("JPEG 带 DCF 缩略图 第二轮（自己的产物）", &inputs(&d1), &d2, 0, &dcf, encode::Collision::Overwrite);

    for comp in 0..4u8 {
        let mut t = encode::Opt::for_fmt(3);
        t.tiff_comp = comp;
        let name = ["无压缩", "LZW", "ZIP", "PackBits"][comp as usize];
        let c1 = root.join(format!("tif{comp}a"));
        batch(&format!("TIFF {name} 第一轮"), &srcs, &c1, 3, &t, encode::Collision::Overwrite);
        let c2 = root.join(format!("tif{comp}b"));
        batch(&format!("TIFF {name} 第二轮（自己的产物）"), &inputs(&c1), &c2, 3, &t, encode::Collision::Overwrite);
    }

    // 原地重编（输出目录=源目录），四种覆盖策略各跑一遍
    let inplace = inputs(&din);
    for (label, pol) in [
        ("取代", encode::Collision::Overwrite),
        ("跳过", encode::Collision::Skip),
        ("编号", encode::Collision::Rename),
    ] {
        let cp = root.join(format!("inplace_{}", label));
        std::fs::create_dir_all(&cp).unwrap();
        for p in inputs(&din) {
            let _ = std::fs::copy(&p, cp.join(Path::new(&p).file_name().unwrap()));
        }
        let v = inputs(&cp);
        batch(&format!("原地重编 · {label}"), &v, &cp, 0, &encode::Opt::for_fmt(0), pol);
    }
    let _ = inplace;

    // 两个不同文件夹里的同名源文件，转到第三个文件夹：批次查重该让号，不能并成一张。
    for (label, pol) in [
        ("取代", encode::Collision::Overwrite),
        ("编号", encode::Collision::Rename),
    ] {
        let x = root.join("two_x");
        let y = root.join("two_y");
        let o = root.join(format!("two_out_{}", label));
        for d in [&x, &y, &o] {
            let _ = std::fs::remove_dir_all(d);
            std::fs::create_dir_all(d).unwrap();
        }
        let a = image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(40, 30, |_, _| image::Rgba([200, 10, 10, 255])));
        let b = image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(40, 30, |_, _| image::Rgba([10, 10, 200, 255])));
        a.save_with_format(x.join("same.jpg"), image::ImageFormat::Jpeg).unwrap();
        b.save_with_format(y.join("same.jpg"), image::ImageFormat::Jpeg).unwrap();
        let pair = vec![x.join("same.jpg").display().to_string(), y.join("same.jpg").display().to_string()];
        batch(
            &format!("两张不同目录的同名图转进同一个文件夹 · {label}"),
            &pair,
            &o,
            1,
            &encode::Opt::for_fmt(1),
            pol,
        );
        let got = std::fs::read_dir(&o).unwrap().flatten().count();
        println!("   → 盘上 {got} 个文件（两张源应为 2 个）");
    }

    // 同一批、同一对源，但一张转完才轮到下一张（4 线程错开就是这样）：
    // 让号登记表已经松手，盘上又已有同名文件，此时批次查重只认"目标在源清单里"就露馅了。
    {
        let x = root.join("two_x");
        let y = root.join("two_y");
        let o = root.join("two_seq");
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        let pair = vec![x.join("same.jpg").display().to_string(), y.join("same.jpg").display().to_string()];
        let set = encode::batch_set(&pair);
        println!("── 同一批但顺序转 · 取代（批内查重该让号）");
        for s in &pair {
            match encode::convert_file_x(
                s,
                o.display().to_string().as_str(),
                None,
                1,
                &encode::Opt::for_fmt(1),
                encode::Collision::Overwrite,
                None,
                None,
                &set,
            ) {
                Ok(encode::Outcome::Done(p, n)) => println!("   OK   {} -> {} ({n} B)", Path::new(s).file_name().unwrap().to_string_lossy(), Path::new(&p).file_name().unwrap().to_string_lossy()),
                Ok(encode::Outcome::Skipped) => println!("   SKIP {}", Path::new(s).file_name().unwrap().to_string_lossy()),
                Err(e) => println!("   ERR  {}：{e}", Path::new(s).file_name().unwrap().to_string_lossy()),
            }
        }
        let got = std::fs::read_dir(&o).unwrap().flatten().count();
        println!("   → 盘上 {got} 个文件（两张源应为 2 个）");
    }
}

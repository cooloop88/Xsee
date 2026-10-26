// 临时探针：按批量转换面板的调用路径跑 convert_file_x，验证链路。
use std::path::Path;
use xsee_host::encode;

fn main() {
    collision_skip_case();
    encode::begin_batch();
    let dir = std::env::temp_dir().join("xsee-conv-test");
    let din = dir.join("in");
    let dout = dir.join("out");
    let _ = std::fs::create_dir_all(&din);
    let _ = std::fs::create_dir_all(&dout);

    // 造三张不同扩展名的源图：png / bmp / jpg
    let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(320, 200, |x, y| {
        image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
    }));
    let mut srcs = Vec::new();
    for (name, fmt) in [
        ("a.png", image::ImageFormat::Png),
        ("b.bmp", image::ImageFormat::Bmp),
        ("c.jpg", image::ImageFormat::Jpeg),
    ] {
        let p = din.join(name);
        img.save_with_format(&p, fmt).unwrap();
        srcs.push(p.display().to_string());
    }

    // 面板默认口径：JPEG、质量 90、不缩放、覆盖
    for (fmt, label) in [(0usize, "JPEG"), (1, "PNG"), (2, "BMP"), (3, "TIFF"), (4, "GIF")] {
        // 清空输出目录
        for e in std::fs::read_dir(&dout).unwrap().flatten() {
            let _ = std::fs::remove_file(e.path());
        }
        let opt = encode::Opt::for_fmt(fmt);
        let xf = encode::Xform::default();
        for s in &srcs {
            match encode::convert_file_x(
                s,
                dout.display().to_string().as_str(),
                None,
                fmt,
                &opt,
                encode::Collision::Overwrite,
                Some(&xf),
                None,
                &std::collections::HashSet::new(),
            ) {
                Ok(encode::Outcome::Done(p, n)) => {
                    println!("{label} <- {} OK {n} bytes  {}", Path::new(s).file_name().unwrap().to_string_lossy(), Path::new(&p).file_name().unwrap().to_string_lossy());
                }
                Ok(encode::Outcome::Skipped) => println!("{label} <- {} SKIPPED", Path::new(s).file_name().unwrap().to_string_lossy()),
                Err(e) => println!("{label} <- {} ERR: {e}", Path::new(s).file_name().unwrap().to_string_lossy()),
            }
        }
    }
    // 缩放路径也验一遍（面板上最常见的用途）
    for e in std::fs::read_dir(&dout).unwrap().flatten() {
        let _ = std::fs::remove_file(e.path());
    }
    let xf = encode::Xform {
        fit: encode::FitMode::Contain,
        w: 100,
        h: 100,
        ..encode::Xform::default()
    };
    for s in &srcs {
        match encode::convert_file_x(
            s,
            dout.display().to_string().as_str(),
            None,
            1,
            &encode::Opt::for_fmt(1),
            encode::Collision::Overwrite,
            Some(&xf),
            None,
            &std::collections::HashSet::new(),
        ) {
            Ok(encode::Outcome::Done(p, n)) => {
                let d = image::io::Reader::open(&p).unwrap().with_guessed_format().unwrap().decode().unwrap();
                println!("PNG-resize <- {} OK {n} bytes, out {}x{}", Path::new(s).file_name().unwrap().to_string_lossy(), d.width(), d.height());
            }
            Ok(encode::Outcome::Skipped) => println!("PNG-resize <- {} SKIPPED", Path::new(s).file_name().unwrap().to_string_lossy()),
            Err(e) => println!("PNG-resize <- {} ERR: {e}", Path::new(s).file_name().unwrap().to_string_lossy()),
        }
    }

    // 同批同主名：a.jpg + a.png 一起转 JPEG，取代档下也必须出两份，不许并成一份。
    for e in std::fs::read_dir(&dout).unwrap().flatten() {
        let _ = std::fs::remove_file(e.path());
    }
    let pair = vec![srcs[2].clone(), srcs[0].clone()]; // c.jpg + a.png
    for s in &pair {
        match encode::convert_file_x(
            s,
            dout.display().to_string().as_str(),
            None,
            0,
            &encode::Opt::for_fmt(0),
            encode::Collision::Overwrite,
            None,
            None,
            &encode::batch_set(&pair),
        ) {
            Ok(encode::Outcome::Done(p, n)) => println!("same-stem <- {} OK {n} bytes -> {}", Path::new(s).file_name().unwrap().to_string_lossy(), Path::new(&p).file_name().unwrap().to_string_lossy()),
            Ok(encode::Outcome::Skipped) => println!("same-stem <- {} SKIPPED", Path::new(s).file_name().unwrap().to_string_lossy()),
            Err(e) => println!("same-stem <- {} ERR: {e}", Path::new(s).file_name().unwrap().to_string_lossy()),
        }
    }
    let n_out = std::fs::read_dir(&dout).unwrap().flatten().count();
    println!("same-stem outputs: {n_out} (expect 2)");

    // 同格式原地转：c.jpg -> c.jpg，输出路径必须认成"就是自己"。
    match encode::convert_file_x(
        &srcs[2],
        din.display().to_string().as_str(),
        None,
        0,
        &encode::Opt::for_fmt(0),
        encode::Collision::Overwrite,
        None,
        None,
        &std::collections::HashSet::new(),
    ) {
        Ok(encode::Outcome::Done(p, n)) => println!("in-place OK {n} bytes -> {}", p),
        Ok(encode::Outcome::Skipped) => println!("in-place SKIPPED"),
        Err(e) => println!("in-place ERR: {e}"),
    }
    println!("done");
}

// 跳过档 + 同批同主名（a.jpg、a.png 都在盘上）：a.png 不得被"同名"跳掉，要让到空号。
fn collision_skip_case() {
    let dir = std::env::temp_dir().join("xsee-conv-collide");
    let _ = std::fs::remove_dir_all(&dir);
    let din = dir.join("in");
    let _ = std::fs::create_dir_all(&din);
    let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(64, 48, |x, y| {
        image::Rgba([(x * 4) as u8, (y * 5) as u8, 90, 255])
    }));
    let pa = din.join("a.jpg");
    let pb = din.join("a.png");
    img.save_with_format(&pa, image::ImageFormat::Jpeg).unwrap();
    img.save_with_format(&pb, image::ImageFormat::Png).unwrap();
    let batch = vec![pa.display().to_string(), pb.display().to_string()];
    for s in &batch {
        match encode::convert_file_x(
            s,
            din.display().to_string().as_str(),
            None,
            0,
            &encode::Opt::for_fmt(0),
            encode::Collision::Skip,
            None,
            None,
            &encode::batch_set(&batch),
        ) {
            Ok(encode::Outcome::Done(p, n)) => println!("skip-collide <- {} OK -> {}", Path::new(s).file_name().unwrap().to_string_lossy(), Path::new(&p).file_name().unwrap().to_string_lossy()),
            Ok(encode::Outcome::Skipped) => println!("skip-collide <- {} SKIPPED", Path::new(s).file_name().unwrap().to_string_lossy()),
            Err(e) => println!("skip-collide <- {} ERR: {e}", Path::new(s).file_name().unwrap().to_string_lossy()),
        }
    }
    let names: Vec<String> = std::fs::read_dir(&din)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    println!("skip-collide files: {:?} (expect a.jpg + a (2).jpg, no a.png)", names);
}

use image::{DynamicImage, Rgba, RgbaImage};

/// 逐张对拍两个目录里的同名缩略图：报平均差 / 最大差，并拼出一张 3 倍放大的对照图。
fn main() {
    let a = std::env::args().nth(1).expect("用法: cmp <目录A> <目录B> <对照图.png>");
    let b = std::env::args().nth(2).expect("缺目录B");
    let out = std::env::args().nth(3).expect("缺输出文件");
    let mut names: Vec<String> = std::fs::read_dir(&a)
        .expect("目录A 读不了")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".png"))
        .collect();
    names.sort();

    let mut canvas: Option<RgbaImage> = None;
    let mut y = 0u32;
    let gap = 6u32;
    let scale = 3u32;
    for name in &names {
        let pb = format!("{b}\\{name}");
        if !std::path::Path::new(&pb).exists() {
            println!("  {name} 只在 A 有，跳过");
            continue;
        }
        let ia = image::open(format!("{a}\\{name}"))
            .expect("A 打不开")
            .to_rgba8();
        let ib = image::open(&pb).expect("B 打不开").to_rgba8();
        if ia.dimensions() != ib.dimensions() {
            println!("  {name} 尺寸不一样 A={:?} B={:?}", ia.dimensions(), ib.dimensions());
            continue;
        }
        let (w, h) = ia.dimensions();
        let mut sum = 0u64;
        let mut max = 0u8;
        for (p, q) in ia.pixels().zip(ib.pixels()) {
            for k in 0..3 {
                let d = p.0[k].abs_diff(q.0[k]);
                sum += d as u64;
                max = max.max(d);
            }
        }
        let n = (w as u64 * h as u64 * 3).max(1);
        println!(
            "  {:<44} {:>4}x{:<4} 平均差 {:.2} /255   最大差 {}",
            name.trim_end_matches(".png"), w, h, sum as f64 / n as f64, max
        );

        let row = h * scale + gap;
        let cw = (w * 2 * scale + gap * 3) as usize;
        let canvas = canvas.get_or_insert_with(|| RgbaImage::new(cw as u32, 0));
        if canvas.width() < cw as u32 {
            let mut big = RgbaImage::new(cw as u32, canvas.height());
            image::imageops::overlay(&mut big, canvas, 0, 0);
            *canvas = big;
        }
        if (y + row) as usize > canvas.height() as usize {
            let mut tall = RgbaImage::new(canvas.width(), y + row);
            image::imageops::overlay(&mut tall, canvas, 0, 0);
            *canvas = tall;
        }
        for sy in 0..h {
            for sx in 0..w {
                let pa = ia.get_pixel(sx, sy);
                let pbv = ib.get_pixel(sx, sy);
                for dy in 0..scale {
                    for dx in 0..scale {
                        let x = gap + sx * scale + dx;
                        let yy = y + sy * scale + dy;
                        canvas.put_pixel(x, yy, *pa);
                        canvas.put_pixel(gap * 2 + w * scale + x, yy, *pbv);
                    }
                }
            }
        }
        for c in 0..canvas.width() {
            canvas.put_pixel(c, y + h * scale, Rgba([255, 0, 0, 255]));
        }
        y += row;
    }
    if let Some(c) = canvas {
        let p = DynamicImage::ImageRgba8(c);
        p.save(&out).expect("对照图存不下");
        println!("对照图（每一对：左=直接解码，右=走缓存，3 倍放大）→ {}", out);
    }
}

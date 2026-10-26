// 批量转换面板的测试台：造固定测试图 + 把落盘文件的真实内容打出来给驱动脚本核对。
// reconv.exe 那套是纯 encode 层的矩阵回归；这个管"面板点下去之后盘上到底有什么"。
use std::path::Path;

use image::{Rgba, RgbaImage};
use xsee_host::{edit, encode};

/// 简单稳定的 64 位散列（只用来比对"像素变没变"，不当身份凭证）
fn fnv(px: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in px {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// 一块矩形内的像素散列：水印只该动右下角，别处必须一位不差
fn block(img: &RgbaImage, x0: u32, y0: u32, w: u32, h: u32) -> u64 {
    let mut v = Vec::new();
    for y in y0..(y0 + h).min(img.height()) {
        for x in x0..(x0 + w).min(img.width()) {
            v.extend_from_slice(&img.get_pixel(x, y).0);
        }
    }
    fnv(&v)
}

/// 逐行相邻像素差的平均值——锐化该让它变大，模糊该让它变小。
fn edge(img: &RgbaImage) -> f64 {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w < 2 || h == 0 {
        return 0.0;
    }
    let mut sum = 0u64;
    let mut n = 0u64;
    for y in 0..h {
        for x in 1..w {
            let a = img.get_pixel((x - 1) as u32, y as u32).0[0] as i64;
            let b = img.get_pixel(x as u32, y as u32).0[0] as i64;
            sum += (a - b).abs().unsigned_abs();
            n += 1;
        }
    }
    sum as f64 / n.max(1) as f64
}

/// 一张图的全部可核对量。面板产物和这里对拍，差就落在差的那一格上。
fn report(tag: &str, name: &str, bytes: u64, fmt: &str, img: &RgbaImage) {
    let (w, h) = (img.width().max(1), img.height().max(1));
    let pts = [(2u32, 2u32), (w - 2, 2), (2, h - 2), (w - 2, h - 2), (w / 2, h / 2)];
    let cs: Vec<String> = pts
        .iter()
        .map(|(x, y)| {
            let p = img.get_pixel(*x, *y).0;
            format!("{},{},{}", p[0], p[1], p[2])
        })
        .collect();
    let al: Vec<String> = (0..4)
        .map(|i| {
            let x = ((w / 4) * i + 1).min(w - 1);
            img.get_pixel(x, h / 2).0[3].to_string()
        })
        .collect();
    let bw = (w / 4).max(1);
    let bh = (h / 4).max(1);
    println!(
        "{tag} file={name} bytes={bytes} fmt={fmt} w={w} h={h} px={} alpha={} edge={:.3} hash={:016x} tl={:016x} br={:016x}",
        cs.join("|"),
        al.join(","),
        edge(img),
        fnv(img.as_raw()),
        block(img, 0, 0, bw, bh),
        block(img, w - bw, h - bh, bw, bh),
    );
}

fn make(dir: &Path) {
    // asym：四块颜色各不相同 + 只在顶边一条白杠，旋转/翻转后一眼可辨
    let a = RgbaImage::from_fn(240, 160, |x, y| {
        let v = match (x < 120, y < 80) {
            (true, true) => [250u8, 10, 10, 255],
            (false, true) => [10, 250, 10, 255],
            (true, false) => [10, 10, 250, 255],
            (false, false) => [200, 200, 50, 255],
        };
        Rgba(if y < 3 { [255, 255, 255, 255] } else { v })
    });
    a.save(dir.join("asym.png")).unwrap();

    // edge：左暗右亮一条竖阶，锐化前后差值得量化
    let e = RgbaImage::from_fn(200, 200, |x, _| Rgba(if x < 100 { [40, 40, 40, 255] } else { [200, 200, 200, 255] }));
    e.save(dir.join("edge.png")).unwrap();

    // 同主名两张：pair.png + pair.jpg，验同一批里两个同名会不会并成一张
    let p = RgbaImage::from_fn(64, 48, |x, y| Rgba([(x * 3) as u8, (y * 5) as u8, 90, 255]));
    p.save(dir.join("pair.png")).unwrap();
    image::DynamicImage::ImageRgba8(p)
        .save_with_format(dir.join("pair.jpg"), image::ImageFormat::Jpeg)
        .unwrap();

    // alpha：左半透明右半不透明，验透明区落到各格式变成什么
    let al = RgbaImage::from_fn(100, 100, |x, _| Rgba(if x < 50 { [255, 0, 0, 0] } else { [0, 0, 255, 255] }));
    al.save(dir.join("alpha.png")).unwrap();

    // small 与 wide：一个比目标框小（验放大档），一个比例悬殊（验适应/填充/拉伸）
    RgbaImage::from_fn(80, 60, |_, _| Rgba([128, 128, 128, 255]))
        .save(dir.join("small.png"))
        .unwrap();
    let w = RgbaImage::from_fn(640, 200, |x, y| Rgba(if x < 4 || y < 4 { [255, 255, 0, 255] } else { [30, 30, 30, 255] }));
    w.save(dir.join("wide.png")).unwrap();

    println!("make ok：asym/edge/pair(png+jpg)/alpha/small/wide 共 7 个文件进 {}", dir.display());
}

fn inspect(dir: &Path) {
    let mut names: Vec<String> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path().display().to_string())
            .filter(|p| {
                let l = p.to_ascii_lowercase();
                [".jpg", ".jpeg", ".png", ".bmp", ".tif", ".tiff", ".gif"].iter().any(|e| l.ends_with(e))
            })
            .collect(),
        Err(e) => {
            println!("inspect 目录打不开：{e}");
            return;
        }
    };
    names.sort();
    for p in names {
        let pb = Path::new(&p);
        let bytes = std::fs::metadata(pb).map(|m| m.len()).unwrap_or(0);
        let fmt = pb.extension().and_then(|s| s.to_str()).unwrap_or("?").to_ascii_lowercase();
        let name = pb.file_name().unwrap().to_string_lossy().to_string();
        match encode::decode(&p, false) {
            Ok(img) => report("盘上", &name, bytes, &fmt, &img),
            Err(e) => println!("盘上 file={name} bytes={bytes} fmt={fmt} 解码失败：{e}"),
        }
    }
}

/// 直接拿编辑窗口那份实现算一遍，和面板产物逐位对拍：同一语义只能有一个来源
fn sharpen(file: &str, r: f32, a: f32) {
    let mut img = match encode::decode(file, false) {
        Ok(v) => v,
        Err(e) => {
            println!("sharpen 解码失败：{e}");
            return;
        }
    };
    let bytes = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0);
    let name = Path::new(file).file_name().unwrap().to_string_lossy().to_string();
    let fmt = Path::new(file).extension().and_then(|s| s.to_str()).unwrap_or("?").to_ascii_lowercase();
    report("原图", &name, bytes, &fmt, &img);
    edit::sharpen(&mut img, r, a);
    println!("（下面这行只算不写盘：半径{r} 强度{a}，走的是 edit::sharpen 那同一份代码）");
    report("算出来", &name, 0, &fmt, &img);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match (args.first().map(|s| s.as_str()), args.get(1)) {
        (Some("make"), Some(dir)) => make(Path::new(dir)),
        (Some("inspect"), Some(dir)) => inspect(Path::new(dir)),
        (Some("sharpen"), Some(file)) => {
            let r = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.5);
            let a = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.6);
            sharpen(file, r, a);
        }
        _ => eprintln!("用法：convrig make <目录> | convrig inspect <目录> | convrig sharpen <图> <半径> <强度>"),
    }
}

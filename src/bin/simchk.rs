//! 指纹自证台。两个用途：
//!   simchk mk <源图> <目录>          —— 造一组"同一张图的变形 + 一张不相干的对照"
//!   simchk probe <目录> [宽 高]      —— 同档出指纹入库，再逐张找相似，打印名次和位数
//! 用 XSEE_CACHE_DIR 指到隔离目录，别碰真缓存。
use std::time::Instant;

use xsee_host::{job, scan, sim, Rgba};

const DEF_BOX: (u32, u32) = (208, 141);

fn name(p: &str) -> String {
    let s = std::path::Path::new(p)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string());
    let mut out = String::new();
    for c in s.chars() {
        if out.chars().count() >= 28 {
            out.push('~');
            break;
        }
        out.push(c);
    }
    out
}

fn images_in(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for e in std::fs::read_dir(dir).expect("read_dir") {
        let p = match e {
            Ok(e) => e.path(),
            Err(_) => continue,
        };
        let s = p.to_string_lossy().into_owned();
        if p.is_file() && scan::is_image(&s) {
            v.push(s);
        }
    }
    v.sort();
    v
}

/// 变形是"同一张图被人怎么改"：缩放、调亮、裁边，再加镜像、上下翻、转 180。
/// 翻转一律自己逐像素搬——`image::imageops::flip_horizontal/vertical` 在这里静默没生效过
/// （造出来的 e/f 和原图字节数一模一样），别指望库函数，改完还自检一下。
fn flip(img: &image::RgbaImage, lr: bool, ud: bool) -> image::RgbaImage {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut out = image::RgbaImage::new(img.width(), img.height());
    for y in 0..h {
        for x in 0..w {
            let sx = if lr { w - 1 - x } else { x };
            let sy = if ud { h - 1 - y } else { y };
            out.put_pixel(x as u32, y as u32, *img.get_pixel(sx as u32, sy as u32));
        }
    }
    out
}

fn mk(src: &str, outdir: &str) {
    std::fs::create_dir_all(outdir).expect("mkdir");
    let base = image::open(src).expect("读不了源图").into_rgba8();
    let (w, h) = (base.width(), base.height());
    let put = |img: image::RgbaImage, tag: &str| {
        let p = std::path::Path::new(outdir).join(format!("{tag}.jpg"));
        // JPEG 编码器只收 RGB（RGBA 直接回 Unsupported），和 cache.rs 里同一件事。
        image::DynamicImage::ImageRgba8(img)
            .to_rgb8()
            .save(&p)
            .expect("存不了");
        println!("  wrote {}", p.to_string_lossy());
    };
    put(
        image::imageops::resize(&base, w * 85 / 100, h * 85 / 100, image::imageops::FilterType::Lanczos3),
        "a_scale85",
    );
    let mut bright = base.clone();
    for px in bright.pixels_mut() {
        px[0] = (px[0] as u32 * 112 / 100).min(255) as u8;
        px[1] = (px[1] as u32 * 112 / 100).min(255) as u8;
        px[2] = (px[2] as u32 * 112 / 100).min(255) as u8;
    }
    put(bright, "b_bright112");
    put(
        image::imageops::crop_imm(&base, w / 12, h / 12, w * 5 / 6, h * 5 / 6).to_image(),
        "c_crop17",
    );
    let mir = flip(&base, true, false);
    let fvr = flip(&base, false, true);
    let rot = flip(&base, true, true);
    // 自检：真翻过的图，左上角那个像素不可能还和原图一样。
    let chk = |t: &str, img: &image::RgbaImage| {
        let a = base.get_pixel(0, 0);
        let b = img.get_pixel(0, 0);
        let same = a == b;
        println!("  check {t}: corner {} ({a:?} vs {b:?})", if same { "UNCHANGED - flip broken" } else { "moved ok" });
    };
    chk("e_mirror_lr", &mir);
    chk("f_mirror_ud", &fvr);
    chk("g_rot180", &rot);
    put(mir, "e_mirror_lr");
    put(fvr, "f_mirror_ud");
    put(rot, "g_rot180");
    put(base, "d_original");
}

/// 按给定映射重排一张位图（只挪像素，不动尺寸）。
fn remap(img: &Rgba, f: impl Fn(usize, usize) -> (usize, usize)) -> Rgba {
    let (w, h) = (img.w as usize, img.h as usize);
    let mut px = vec![0u8; img.px.len()];
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = f(x, y);
            let (di, si) = ((y * w + x) * 4, (sy * w + sx) * 4);
            if si + 4 > img.px.len() || di + 4 > px.len() {
                continue;
            }
            px[di..di + 4].copy_from_slice(&img.px[si..si + 4]);
        }
    }
    Rgba {
        w: img.w,
        h: img.h,
        px,
    }
}

/// 一张图的四个方向：正、左右反、上下反、转 180。
/// 库只存"正"那一份，查询时这四份都算、取差异最小的——反着的图再反一次就是正的。
fn orientations(img: &Rgba) -> Vec<sim::Fp> {
    let (w, h) = (img.w as usize, img.h as usize);
    let maps: [Box<dyn Fn(usize, usize) -> (usize, usize)>; 4] = [
        Box::new(move |x, y| (x, y)),
        Box::new(move |x, y| (w - 1 - x, y)),
        Box::new(move |x, y| (x, h - 1 - y)),
        Box::new(move |x, y| (w - 1 - x, h - 1 - y)),
    ];
    maps.iter()
        .map(|m| sim::fingerprint(&remap(img, |x, y| m(x, y))))
        .collect()
}

fn probe(dir: &str, bw: u32, bh: u32) {
    job::init_com();
    let paths = images_in(dir);
    println!("listed {} pics, box {}x{}", paths.len(), bw, bh);
    let mut fps: Vec<(String, Vec<sim::Fp>)> = Vec::new();
    let t0 = Instant::now();
    for p in &paths {
        match job::make(p, bw, bh) {
            Ok(t) => {
                let v = orientations(&t.img);
                sim::note(p, bw, bh, &t.img);
                fps.push((p.clone(), v));
            }
            Err(e) => println!("  skip {}: {}", name(p), e),
        }
    }
    println!(
        "decoded+filed {} pics in {:.1}s",
        fps.len(),
        t0.elapsed().as_secs_f64()
    );

    println!(
        "\n[nearest 6, each pair shown as 正向差位/四方向取最小差位; ! = over threshold {}]",
        sim::MAX_DIST
    );
    for (i, (p, mine)) in fps.iter().enumerate() {
        let mut all: Vec<(u32, u32, String)> = fps
            .iter()
            .map(|(q, theirs)| {
                let d0 = sim::distance(&mine[0], &theirs[0]);
                let dm = mine
                    .iter()
                    .flat_map(|a| theirs.iter().map(|b| sim::distance(a, b)))
                    .min()
                    .unwrap_or(0);
                (d0, dm, name(q))
            })
            .collect();
        all.sort();
        let line: Vec<String> = all
            .iter()
            .take(6)
            .map(|(d0, dm, q)| {
                let mark = if *dm <= sim::MAX_DIST { "" } else { "!" };
                format!("{d0}/{dm}{mark} {q}")
            })
            .collect();
        println!("  {:30} [{}]  #{}", name(p), line.join(" | "), i);
    }

    // 补库这条线：从缩略图缓存取图，不开原图。
    let prog = std::sync::atomic::AtomicUsize::new(0);
    sim::ensure_loaded();
    let todo = xsee_host::cache::missing_fps(bw, bh, None);
    let t2 = Instant::now();
    let made = sim::catch_up(None, bw, bh, &prog, &std::sync::atomic::AtomicBool::new(false));
    println!(
        "\ncatch_up: {} cached thumbs missing a print, +{} fingerprints in {:.2}s",
        todo.len(),
        made,
        t2.elapsed().as_secs_f64()
    );
}

/// 全库两两扫一遍：这一档的库里到底有多少对算"相似"。
/// 阈值是不是定偏了，只有在他真看过的那几千张上量才知道。
fn sweep(bw: u32, bh: u32) {
    job::init_com();
    let prog = std::sync::atomic::AtomicUsize::new(0);
    sim::ensure_loaded();
    let todo = xsee_host::cache::missing_fps(bw, bh, None);
    let t0 = Instant::now();
    let made = sim::catch_up(None, bw, bh, &prog, &std::sync::atomic::AtomicBool::new(false));
    println!(
        "{} cached thumbs missing a print, +{} fingerprints in {:.2}s",
        todo.len(),
        made,
        t0.elapsed().as_secs_f64()
    );
    let e = sim::entries(bw, bh);
    let n = e.len();
    println!("filed on this box: {n}");
    let t1 = Instant::now();
    let (mut le40, mut le80, mut le120) = (0usize, 0usize, 0usize);
    let mut pairs: Vec<(u32, String, String)> = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            let d = sim::distance(&e[i].1, &e[j].1);
            if d <= sim::MAX_DIST {
                le40 += 1;
            }
            if d <= 80 {
                le80 += 1;
            }
            if d <= 120 {
                le120 += 1;
            }
            if d <= 80 {
                pairs.push((d, name(&e[i].0), name(&e[j].0)));
            }
        }
    }
    let ms = t1.elapsed().as_millis();
    println!("pairs={n}  <=40:{le40}  <=80:{le80}  <=120:{le120}  swept in {ms}ms");
    pairs.sort();
    for (d, a, b) in pairs.iter().take(20) {
        println!("  {d:4}  {a}  <->  {b}");
    }
}

/// 配图用：把目录里每张缩成小 JPG 存出去，说明页能把图和位数一一对上。
fn tiles(dir: &str, out: &str, bw: u32, bh: u32) {
    std::fs::create_dir_all(out).expect("mkdir");
    for p in images_in(dir) {
        let img = image::open(&p).expect("open").into_rgba8();
        let t = image::imageops::resize(&img, bw, bh, image::imageops::FilterType::Lanczos3);
        let stem = std::path::Path::new(&p)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "x".to_string());
        let dst = std::path::Path::new(out).join(format!("{stem}.jpg"));
        image::DynamicImage::ImageRgba8(t)
            .to_rgb8()
            .save(&dst)
            .expect("存不了");
        println!("  tile {}", dst.to_string_lossy());
    }
}

fn main() {
    let mut a = std::env::args();
    let _me = a.next();
    let box_of = |a: &mut std::env::Args| match (a.next(), a.next()) {
        (Some(w), Some(h)) => (w.parse().expect("宽"), h.parse().expect("高")),
        _ => DEF_BOX,
    };
    match a.next().as_deref() {
        Some("mk") => mk(&a.next().expect("src"), &a.next().expect("outdir")),
        Some("probe") => {
            let dir = a.next().expect("dir");
            let (bw, bh) = box_of(&mut a);
            probe(&dir, bw, bh);
        }
        Some("tiles") => {
            let dir = a.next().expect("dir");
            let out = a.next().expect("out");
            let (bw, bh) = box_of(&mut a);
            tiles(&dir, &out, bw, bh);
        }
        Some("sweep") => {
            let (bw, bh) = box_of(&mut a);
            sweep(bw, bh);
        }
        _ => println!("usage: simchk mk <src> <outdir> | probe <dir> [bw bh] | tiles <dir> <out> [bw bh] | sweep [bw bh]"),
    }
}

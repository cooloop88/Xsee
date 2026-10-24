use std::error::Error;
use std::fs::File;
use std::time::Instant;

use jpeg_decoder::Decoder;

fn save(buf: &[u8], w: u32, h: u32, bpp: usize, out: &str) -> Result<(), Box<dyn Error>> {
    let im = match bpp {
        3 => image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(w, h, buf.to_vec())
            .map(image::DynamicImage::ImageRgb8),
        1 => image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_raw(w, h, buf.to_vec())
            .map(image::DynamicImage::ImageLuma8),
        4 => image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(w, h, buf.to_vec())
            .map(image::DynamicImage::ImageRgba8),
        _ => None,
    };
    match im {
        Some(im) => {
            im.save(out)?;
            Ok(())
        }
        None => Err(format!("bad buffer {} bytes for {}x{} bpp {}", buf.len(), w, h, bpp).into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    // 用法：probe <图片> [输出目录]。不给图片就不动，输出目录默认落在系统临时目录。
    let Some(path) = args.get(1).cloned() else {
        eprintln!("用法：probe <图片> [输出目录]");
        return Ok(());
    };
    let outdir = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().join("xsee-scale").display().to_string());
    std::fs::create_dir_all(&outdir)?;

    let f = File::open(&path)?;
    let mut dec = Decoder::new(f);
    dec.read_info()?;
    let info = dec.info().ok_or("no info")?;
    let (iw, ih) = (info.width as u32, info.height as u32);
    let bpp = info.pixel_format.pixel_bytes();
    println!(
        "probe: {}\n  native {}x{}  {:?} ({} bytes/px)",
        path.rsplit('\\').next().unwrap_or(&path),
        iw,
        ih,
        info.pixel_format,
        bpp
    );

    let full_mb = (iw as usize * ih as usize * bpp) as f64 / 1048576.0;
    let t = Instant::now();
    let buf = dec.decode()?;
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    let out = format!("{}\\full.png", outdir);
    save(&buf, iw, ih, bpp, &out)?;
    println!(
        "  full decode      {:>5}x{:<5} {:>7.1} MB  {:>8.1} ms",
        iw,
        ih,
        buf.len() as f64 / 1048576.0,
        ms
    );

    for target in [769u16, 173u16] {
        let f = File::open(&path)?;
        let mut dec = Decoder::new(f);
        let (ow, oh) = dec.scale(target, target)?;
        let t = Instant::now();
        let buf = dec.decode()?;
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let ow = ow as u32;
        let oh = oh as u32;
        let out = format!("{}\\t{}.png", outdir, target);
        save(&buf, ow, oh, bpp, &out)?;
        println!(
            "  scale({:>4}) ask  {:>5}x{:<5} got {:>5}x{:<5} {:>7.1} MB  {:>8.1} ms  factor 1/{}",
            target,
            iw,
            ih,
            ow,
            oh,
            buf.len() as f64 / 1048576.0,
            ms,
            (iw as f64 / ow as f64).round() as u32
        );
    }
    println!("  (full buffer would be {:.1} MB)", full_mb);
    Ok(())
}

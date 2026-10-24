/// 缩放插值模式对比：同一张图、同一个目标框，五种模式各量两遍取快的。
use std::time::Instant;

use xsee_host::job;
use windows::Win32::Graphics::Imaging::*;

const MODES: [(&str, WICBitmapInterpolationMode); 5] = [
    ("最近邻", WICBitmapInterpolationModeNearestNeighbor),
    ("线性", WICBitmapInterpolationModeLinear),
    ("三次", WICBitmapInterpolationModeCubic),
    ("Fant", WICBitmapInterpolationModeFant),
    ("高质三次", WICBitmapInterpolationModeHighQualityCubic),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = args.first().cloned().unwrap_or("work\\big".to_string());
    let (bw, bh) = match args.get(1) {
        Some(s) => {
            let (a, b) = s.split_once('x').unwrap_or(("2560", "1357"));
            (a.parse().unwrap_or(2560), b.parse().unwrap_or(1357))
        }
        None => (2560, 1357),
    };
    job::init_com();
    let mut files: Vec<String> = xsee_host::scan::list(&dir, false)
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.path.clone())
        .collect();
    files.retain(|p| {
        let l = p.to_ascii_lowercase();
        l.ends_with(".jpg") || l.ends_with(".jpeg") || l.ends_with(".png") || l.ends_with(".tif")
    });
    println!("目标框 {}x{}，每种模式两遍取快的", bw, bh);
    println!("{:<8} {:<10}{}", "文件", "原图", "五种插值模式的耗时");
    for path in &files {
        let name = path.rsplit('\\').next().unwrap_or(path).split_once("__").map(|(a, _)| a.to_string()).unwrap_or_default();
        let Ok((nw, nh)) = job::open_size(path) else { continue };
        let mut row = Vec::new();
        for (label, mode) in MODES {
            let mut best = f64::MAX;
            for _ in 0..2 {
                let t = Instant::now();
                match job::decode_view_mode(path, bw, bh, Some(mode)) {
                    Ok(_) => best = best.min(t.elapsed().as_secs_f64() * 1000.0),
                    Err(_) => {
                        best = f64::NAN;
                        break;
                    }
                }
            }
            row.push(format!("{} {:.0}ms", label, best));
        }
        println!("{:<8} {:>6}x{:<5} {}", name, nw, nh, row.join("  "));
    }
}

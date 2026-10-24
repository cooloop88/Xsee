/// 各格式最坏情况测速：打开 / 缩略图 / 适配屏幕 / 原尺寸全解。
use std::time::Instant;

use xsee_host::{job, peak_mb, scan};

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = args
        .first()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| "work\\big".to_string());
    let (fw, fh) = match opt(&args, "--fit").as_deref() {
        Some(s) => {
            let (a, b) = s.split_once('x').unwrap_or(("2560", "1440"));
            (a.parse().unwrap_or(2560), b.parse().unwrap_or(1440))
        }
        None => (2560, 1440),
    };
    let cap: f64 = opt(&args, "--cap")
        .and_then(|s| s.parse().ok())
        .unwrap_or(300.0);
    let rounds: usize = opt(&args, "--rounds").and_then(|s| s.parse().ok()).unwrap_or(2);

    job::init_com();
    // 这张表量的是纯解码成本，命中缓存会变成 1ms，所以这里必须把缓存关掉。
    xsee_host::cache::set_enabled(false);
    let mut files: Vec<String> = scan::list(&dir, false)
        .iter()
        .filter(|e| !e.is_dir && scan::is_image(&e.path))
        .map(|e| e.path.clone())
        .collect();
    files.sort_by_key(|p| {
        p.rsplit('\\')
            .next()
            .unwrap_or("")
            .split_once("__")
            .map(|(a, _)| a.to_ascii_lowercase())
            .unwrap_or_default()
    });
    println!(
        "{} 个文件；适配框 {}x{}；原尺寸上限 {:.0} 百万像素；每个量 {} 次取最后一次",
        files.len(),
        fw,
        fh,
        cap,
        rounds
    );
    println!("格式   原图像素      文件大小   打开    缩略图  适配屏幕  原尺寸   峰值内存");
    for path in &files {
        let name = path.rsplit('\\').next().unwrap_or(path);
        let fmt = name.split_once("__").map(|(a, _)| a.to_string()).unwrap_or_else(|| {
            name.rsplit('.').next().unwrap_or("?").to_ascii_lowercase()
        });
        let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let (mut nw, mut nh) = (0u32, 0u32);
        let mut t_open = f64::NAN;
        let mut fail = false;
        for r in 0..rounds {
            let t = Instant::now();
            match job::open_size(path) {
                Ok((a, b)) => {
                    nw = a;
                    nh = b;
                }
                Err(e) => {
                    println!(
                        "{:<6} {:>18}  {:>8.1} MB   打不开: {}",
                        "",
                        "",
                        bytes as f64 / 1048576.0,
                        e
                    );
                    fail = true;
                    break;
                }
            }
            let el = ms(t);
            if r == 0 {
                t_open = el;
            }
        }
        if fail {
            continue;
        }
        let mp = nw as f64 * nh as f64 / 1e6;

        let hi = |name: &'static str,
                  run: &dyn Fn() -> windows::core::Result<()>|
         -> (f64, f64, String) {
            let mut first = f64::NAN;
            let mut best = f64::NAN;
            let mut err = String::new();
            for r in 0..rounds {
                let t = Instant::now();
                match run() {
                    Ok(()) => {
                        let el = ms(t);
                        if r == 0 {
                            first = el;
                        }
                        best = if best.is_nan() { el } else { best.min(el) };
                    }
                    Err(e) => err = format!("{name} {e}"),
                }
            }
            (first, best, err)
        };

        let (th1, thi, e1) = hi("缩略图", &|| job::make(path, 173, 119).map(|_| ()));
        let (f1, fi, e2) = hi("适配", &|| job::decode_view(path, fw, fh).map(|_| ()));
        let (u1, ui, e3) = if mp <= cap {
            hi("原尺寸", &|| job::decode_view(path, 0, 0).map(|_| ()))
        } else {
            (f64::NAN, f64::NAN, String::new())
        };
        let err = if !e1.is_empty() {
            e1
        } else if !e2.is_empty() {
            e2
        } else {
            e3
        };
        let f = |a: f64, b: f64| -> String {
            if a.is_nan() {
                if b.is_nan() && mp > cap {
                    "跳过".to_string()
                } else if b.is_nan() {
                    "失败".to_string()
                } else {
                    format!("{b:.0}ms")
                }
            } else if (a - b).abs() < 1.0 {
                format!("{a:.0}ms")
            } else {
                format!("{a:.0}->{b:.0}ms")
            }
        };
        println!(
            "{:<6} {:>6}x{:<5} {:>8.1} MB {:>9} {:>10} {:>10}  {:>8}   {:>6.1} MB  {:>5.1}MP {:>7.1}亿像素/秒",
            fmt,
            nw,
            nh,
            bytes as f64 / 1048576.0,
            format!("{t_open:.0}ms"),
            f(th1, thi),
            f(f1, fi),
            f(u1, ui),
            peak_mb(),
            mp,
            if fi.is_nan() {
                0.0
            } else {
                mp * 10.0 / fi
            }
        );
        if !err.is_empty() {
            println!("       └ 失败原因：{}", err);
        }
    }
}

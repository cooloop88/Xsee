use std::time::{Duration, Instant};

use xsee_host::{cache, pool::Pool, scan};
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::GetCurrentProcess;

fn peak_mb() -> f64 {
    unsafe {
        let mut c = PROCESS_MEMORY_COUNTERS::default();
        c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let _ = K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb);
        c.PeakWorkingSetSize as f64 / 1048576.0
    }
}

fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = args.first().filter(|a| !a.starts_with('-')).cloned().unwrap_or_else(|| {
        eprintln!("用法: bench <文件夹> [--box 173x119] [--workers 8] [--seconds 5] [--limit N] [--repeat N] [--no-cache]");
        std::process::exit(2);
    });
    let (bw, bh) = match opt(&args, "--box").as_deref() {
        Some(s) => {
            let (a, b) = s.split_once('x').unwrap_or(("173", "119"));
            (a.parse().unwrap_or(173), b.parse().unwrap_or(119))
        }
        None => (173, 119),
    };
    let workers: usize = opt(&args, "--workers").and_then(|s| s.parse().ok()).unwrap_or(8);
    let seconds: f64 = opt(&args, "--seconds").and_then(|s| s.parse().ok()).unwrap_or(5.0);
    let limit: usize = opt(&args, "--limit").and_then(|s| s.parse().ok()).unwrap_or(0);
    let repeat: usize = opt(&args, "--repeat").and_then(|s| s.parse().ok()).unwrap_or(1);
    let save: Option<String> = opt(&args, "--save");
    if args.iter().any(|a| a == "--no-cache") {
        cache::set_enabled(false);
    }
    let (idx_n, idx_bytes) = cache::stats();
    println!(
        "缓存：{}（索引 {} 条 / {:.1} MB），目录 {}",
        if args.iter().any(|a| a == "--no-cache") {
            "关"
        } else {
            "开"
        },
        idx_n,
        idx_bytes as f64 / 1048576.0,
        cache::dir().display()
    );
    let mut saved = 0usize;

    let t = Instant::now();
    let entries = scan::list(&dir, false);
    let scan_ms = t.elapsed().as_secs_f64() * 1000.0;
    let mut files: Vec<String> = entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.path.clone())
        .collect();
    if limit > 0 {
        files.truncate(limit);
    }
    let n = files.len();
    if n == 0 {
        println!("这个文件夹里没有可显示的图");
        return;
    }
    let paths: Vec<String> = (0..n * repeat).map(|i| files[i % n].clone()).collect();
    println!(
        "扫描 {} 个文件（含目录）用时 {:.0} ms，取其中 {} 张图，并发 {} 线程，目标框 {}x{}",
        entries.len(),
        scan_ms,
        paths.len(),
        workers,
        bw,
        bh
    );

    // 第六个参数是重建档：测量台量的是正常出图这条线，不绕缓存。
    let pool = Pool::start(paths, bw, bh, workers, false, false, false);
    let all: Vec<usize> = (0..n * repeat).collect();
    let t0 = Instant::now();
    pool.submit(&all);
    let deadline = Duration::from_secs_f64(seconds);
    let mut done = 0usize;
    let mut ok = 0usize;
    let mut prev = 0usize;
    let mut times: Vec<f64> = Vec::new();
    let mut errs: Vec<String> = Vec::new();
    let mut bytes = 0usize;
    let mut first100 = 0f64;
    while t0.elapsed() < deadline {
        if let Some(r) = pool.recv(Duration::from_millis(200)) {
            done += 1;
            times.push(r.ms);
            if r.from_preview {
                prev += 1;
            }
            match r.res {
                Ok(t) => {
                    ok += 1;
                    if ok == 100 {
                        first100 = t0.elapsed().as_secs_f64();
                    }
                    bytes = bytes.max(t.img.px.len());
                    if let Some(dir) = &save {
                        if saved < 6 {
                            saved += 1;
                            let name = files[r.idx % n].rsplit('\\').next().unwrap_or("x");
                            let out = format!(
                                "{}\\{}_{:02}x{:02}.png",
                                dir,
                                name.trim_end_matches(".jpg"),
                                t.img.w,
                                t.img.h
                            );
                            let _ = image::RgbaImage::from_raw(t.img.w, t.img.h, t.img.px.clone())
                                .map(|i| image::DynamicImage::ImageRgba8(i).save(&out));
                            println!("  存图 {}", out);
                        }
                    }
                }
                Err(e) => {
                    if errs.len() < 3 {
                        let name = files[r.idx].rsplit('\\').next().unwrap_or("?");
                        errs.push(format!("{}: {} | {}", r.idx, name, e));
                    }
                }
            }
        } else if done == 0 && t0.elapsed().as_secs_f64() > 2.0 {
            break;
        }
        if done >= n * repeat {
            break;
        }
    }
    let el = t0.elapsed().as_secs_f64();
    pool.shutdown();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg = times.iter().sum::<f64>() / times.len().max(1) as f64;
    println!(
        "{:.2} s 完成 {} 张，成功 {} 张 -> {:.1} 张/秒（单张平均 {:.0} ms，最快 {:.0}，最慢 {:.0}）",
        el,
        done,
        ok,
        ok as f64 / el,
        avg,
        times.first().copied().unwrap_or(0.0),
        times.last().copied().unwrap_or(0.0)
    );
    if first100 > 0.0 {
        println!(
            "第 100 张到齐用时 {:.2} s -> 开局 {:.1} 张/秒",
            first100,
            100.0 / first100
        );
    }
    println!(
        "走内嵌预览（一张全图都没解）{} 张；单张缩略图字节 {:.1} KB；进程峰值内存 {:.1} MB",
        prev,
        bytes as f64 / 1024.0,
        peak_mb()
    );
    let (n2, b2) = cache::stats();
    println!(
        "缓存命中 {} 张；跑完索引 {} 条 / 数据段 {:.1} MB，平均每张存 {:.1} KB",
        cache::hits(),
        n2,
        b2 as f64 / 1048576.0,
        if n2 == 0 {
            0.0
        } else {
            b2 as f64 / n2 as f64 / 1024.0
        }
    );
    for e in errs {
        println!("  失败 {}", e);
    }
}

/// PNG 后端对比的证据：WIC 缩放解码 vs image 只解压（不含缩放）。
/// 差值就是"换后端最多能赚多少"，实测结论见 job::make 的注释。
use std::time::Instant;

use xsee_host::{job, peak_mb, scan};

fn best(run: &mut dyn FnMut() -> bool) -> f64 {
    let mut b = f64::MAX;
    for _ in 0..2 {
        let t = Instant::now();
        if !run() {
            return f64::NAN;
        }
        b = b.min(t.elapsed().as_secs_f64() * 1000.0);
    }
    b
}

/// 只读文件头 29 字节的 IHDR，不整读文件。
fn ihdr(path: &str) -> Option<(u32, u32, u32, u32)> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut b = [0u8; 29];
    std::io::Read::read_exact(&mut f, &mut b).ok()?;
    Some((
        u32::from_be_bytes(b[16..20].try_into().ok()?),
        u32::from_be_bytes(b[20..24].try_into().ok()?),
        b[24] as u32,
        b[25] as u32,
    ))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = args.first().cloned().unwrap_or("work\\big".to_string());
    let box_arg = args.get(1).cloned().unwrap_or("173x119".into());
    let (bw, bh): (u32, u32) = box_arg
        .split_once('x')
        .map(|(a, b)| (a.parse().unwrap_or(173), b.parse().unwrap_or(119)))
        .unwrap_or((173, 119));
    let n: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
    job::init_com();
    let mut rows: Vec<(String, u32, u32, u32, u32)> = scan::list(&dir, false)
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.path.clone())
        .filter(|p| p.to_ascii_lowercase().ends_with(".png"))
        .filter_map(|p| ihdr(&p).map(|(w, h, d, c)| (p, w, h, d, c)))
        .collect();
    // 从大到小取，差距在巨型图上最明显。
    rows.sort_by_key(|r| std::cmp::Reverse(r.1 as u64 * r.2 as u64));
    println!("目标框 {bw}x{bh}，每个量两遍取快的，共 {} 张 PNG", rows.len());
    println!(
        "{:<26} {:>13} {:>7} {:>7} {:>7} {:>7} {:>8}",
        "文件", "原图", "位深", "WIC", "只解压", "省", "峰值MB"
    );
    for (path, w, h, depth, ctype) in rows.iter().take(n) {
        let path = path.as_str();
        let wic = best(&mut || job::make_wic(path, bw, bh).is_ok());
        let dec = best(&mut || image::open(path).is_ok());
        let f = |v: f64| if v.is_finite() { v } else { 0.0 };
        let kind = match ctype {
            0 => "灰",
            2 => "色",
            3 => "板",
            4 => "灰A",
            6 => "A",
            _ => "?",
        };
        println!(
            "{:<26} {:>5}x{:<7} {:>7} {:>7.0} {:>7.0} {:>6.1}x {:>8.0}",
            path.rsplit('\\')
                .next()
                .unwrap_or(path)
                .chars()
                .take(24)
                .collect::<String>(),
            w,
            h,
            format!("{depth}{kind}"),
            f(wic),
            f(dec),
            if f(dec) > 0.0 {
                f(wic) / f(dec)
            } else {
                0.0
            },
            peak_mb()
        );
    }
}

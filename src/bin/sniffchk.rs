/// 头嗅探对不对：把目录里每张图的嗅探尺寸和 WIC 报的尺寸逐张对拍，
/// 顺便把方向不是 1 的文件列出来（摆正这条要有真素材才验得了）。
/// 用法：sniffchk <图片目录>...
use std::time::Instant;

use xsee_host::{job, preview, scan};

fn main() {
    let dirs: Vec<String> = std::env::args().skip(1).collect();
    if dirs.is_empty() {
        eprintln!("用法：sniffchk <图片目录>...");
        return;
    }
    job::init_com();

    let mut files: Vec<String> = Vec::new();
    for d in &dirs {
        files.extend(
            scan::list(d, true)
                .iter()
                .filter(|e| !e.is_dir && scan::is_image(&e.path))
                .map(|e| e.path.clone()),
        );
    }
    println!("对拍 {} 张图", files.len());

    let (mut agree, mut disagree, mut no_sniff) = (0usize, 0usize, 0usize);
    let mut rotated: Vec<(String, u16)> = Vec::new();
    let (mut t_sniff, mut t_wic) = (0f64, 0f64);

    for path in &files {
        let t = Instant::now();
        let (sn, o) = preview::sniff_orient(path);
        t_sniff += t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        let w = job::wic_size(path);
        t_wic += t.elapsed().as_secs_f64() * 1000.0;

        if o != 1 {
            rotated.push((path.clone(), o));
        }
        let Ok((ww, wh)) = w else { continue };
        match sn {
            None => no_sniff += 1,
            Some((sw, sh)) => {
                if (sw, sh) == (ww, wh) {
                    agree += 1;
                } else {
                    disagree += 1;
                    if disagree <= 20 {
                        println!("不符 {} 嗅 {:>6}x{:<6} WIC {:>6}x{:<6}", name(path), sw, sh, ww, wh);
                    }
                }
            }
        }
    }

    println!(
        "一致 {} 张，不符 {} 张，嗅不出（回落 WIC）{} 张",
        agree, disagree, no_sniff
    );
    println!(
        "嗅探总用时 {:.0} ms（{:.3} ms/张），WIC 开档总用时 {:.0} ms（{:.2} ms/张）",
        t_sniff,
        t_sniff / files.len() as f64,
        t_wic,
        t_wic / files.len() as f64
    );
    println!("方向非 1 的 {} 张：", rotated.len());
    for (p, o) in rotated.iter().take(30) {
        println!("  档 {} -> {}", o, name(p));
    }
}

fn name(p: &str) -> String {
    let s = p.rsplit(['\\', '/']).next().unwrap_or(p);
    if s.len() <= 44 {
        return s.to_string();
    }
    format!("…{}", &s[s.len() - 43..])
}

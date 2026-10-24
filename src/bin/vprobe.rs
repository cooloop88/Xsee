#![allow(unsafe_op_in_unsafe_fn)]
/// 一次性量具：把"双击图片到看见图"这条路拆成一段一段计时，不动 view.exe 本身。
/// 用法: vprobe <图片路径> [次数]
use std::time::Instant;

use xsee_host::{job, scan};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = match args.first() {
        Some(p) => p.clone(),
        None => {
            eprintln!("用法: vprobe <图片路径> [次数]");
            std::process::exit(2);
        }
    };
    let reps: usize = args.get(1).map(|s| s.parse().unwrap_or(1)).unwrap_or(1);
    let dir = path.rsplit_once('\\').map(|(a, _)| a.to_string()).unwrap_or(".".into());

    let t = Instant::now();
    job::init_com();
    let com = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let n = scan::list(&dir, false).len();
    let list = t.elapsed().as_secs_f64() * 1000.0;

    for r in 0..reps {
        let t = Instant::now();
        let size = job::open_size(&path);
        let size_ms = t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        let quick = job::embedded_view(&path, 1180, 700);
        let quick_ms = t.elapsed().as_secs_f64() * 1000.0;

        let t = Instant::now();
        let full = job::decode_view(&path, 1180, 700);
        let full_ms = t.elapsed().as_secs_f64() * 1000.0;

        println!(
            "[{}] 目录 {} 项: 扫夹 {:.0} ms · COM {:.0} ms | 读尺寸 {:.0} ms ({:?}) | 档内预览 {:.0} ms ({}) | 缩放解码 {:.0} ms ({})",
            r,
            n,
            list,
            com,
            size_ms,
            size,
            quick_ms,
            if quick.is_some() { "有" } else { "无" },
            full_ms,
            full.as_ref().map(|v| format!("{}x{}", v.w, v.h)).unwrap_or_else(|e| format!("失败 {e}")),
        );
    }
}

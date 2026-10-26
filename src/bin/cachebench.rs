/// 量缩略图库这几条线的读入耗时：索引首开、名字名单、哈希对照、相似库内存表，
/// 以及全库逐条 exists 的统计（相似页"库统计"那一步的真身）。
/// 用法：cargo run --release --bin cachebench [缓存目录]
/// 目录走参数不走环境变量：argv 是唯一保证能到得了子进程的通道。
use std::time::Instant;

use xsee_host::{cache, sim};

fn mb(n: u64) -> String {
    if n >= 1 << 20 {
        format!("{:.1} MB", n as f64 / (1 << 20) as f64)
    } else {
        format!("{n} B")
    }
}

fn main() {
    if let Some(dir) = std::env::args().nth(1) {
        // edition 2024 起改环境变量是 unsafe：这里进程刚起、单线程，安全。
        unsafe { std::env::set_var("XSEE_CACHE_DIR", &dir) };
    }
    println!("缓存目录 {}", cache::dir().display());
    // 索引首开：第一条缓存命令触发 Idx::open（读整份索引 + 逐条解析进内存表）。
    let t = Instant::now();
    let (n, bytes) = cache::stats();
    println!(
        "索引首开：{} ms · {} 条缩略图 · 数据段 {}",
        t.elapsed().as_millis(),
        n,
        mb(bytes)
    );
    // 名字名单：整份读进来按行去重（"全部重建"和相似库都要它）。
    let t = Instant::now();
    let paths = cache::known_paths();
    println!("名字名单：{} ms · {} 行", t.elapsed().as_millis(), paths.len());
    // 哈希→路径对照：相似库把索引里的指纹配回路径靠它。
    let t = Instant::now();
    let m = cache::path_map();
    println!("哈希对照表：{} ms · {} 条", t.elapsed().as_millis(), m.len());
    // 相似库内存表：全库指纹按档位归堆。
    let t = Instant::now();
    sim::ensure_loaded();
    let (nbox, nall) = sim::stats_all();
    println!(
        "相似库内存表：{} ms · {nbox} 档共 {nall} 条",
        t.elapsed().as_millis()
    );
    // 全库逐条 exists：相似页"库统计"和它算"多少条已不在盘上"的真身，
    // 天生的磁盘口径——这一步才是"统计中…"要等多久的决定项。
    let t = Instant::now();
    let mut gone = 0usize;
    for p in &paths {
        if !std::path::Path::new(p).exists() {
            gone += 1;
        }
    }
    println!(
        "全库逐条 exists：{} ms · {} 张里 {} 张已不在盘上",
        t.elapsed().as_millis(),
        paths.len(),
        gone
    );
}

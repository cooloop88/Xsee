//! 量"打开软件第一次搜图"和"第二次搜图"各要多久。
//! 同一进程里按界面那条线的顺序跑两遍：载入相似库 → 查/补缺指纹 → 跨档搜 →
//! 查死条目。第一遍带着首次载入，第二遍是热的，正好对上界面上的两个数。
//! 默认用真实缓存；`simtime dry` 只报规模，不补库也不搜。
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::time::{Duration, Instant};

use xsee_host::{cache, job, sim, Rgba};

fn ms(d: Duration) -> String {
    format!("{:.1}", d.as_secs_f64() * 1000.0)
}

struct Segs {
    load: Duration,
    missing: Duration,
    fill: Duration,
    search: Duration,
    fallback: Duration,
    meta: Duration,
    total: Duration,
}

/// 和界面一样的死条目过滤：四路并行问盘，只数还在的。
fn alive_count(cand: &[(u32, String)]) -> usize {
    const W: usize = 4;
    std::thread::scope(|s| {
        let hs: Vec<_> = (0..W)
            .map(|w| {
                s.spawn(move || {
                    cand.iter()
                        .skip(w)
                        .step_by(W)
                        .filter(|(_, p)| std::fs::metadata(p).is_ok())
                        .count()
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap_or(0)).sum()
    })
}

fn run_once(
    bw: u32,
    bh: u32,
    q: &str,
    algo: u8,
    max_dist: u32,
    noauto: bool,
    fill_cap: Option<usize>,
) -> (Segs, usize, usize, usize, usize) {
    let prog = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let t_all = Instant::now();

    let t = Instant::now();
    sim::ensure_loaded();
    let load = t.elapsed();

    let t = Instant::now();
    let mut todo = if noauto { Vec::new() } else { cache::missing_fps(bw, bh, None) };
    let missing = t.elapsed();
    let n_todo = todo.len();
    if let Some(n) = fill_cap {
        todo.truncate(n);
    }

    let t = Instant::now();
    let made = sim::fill(todo, bw, bh, &prog, &stop);
    let fill = t.elapsed();

    let t = Instant::now();
    let mut cand = sim::search_all_tiers(
        q,
        false,
        false,
        algo,
        max_dist,
        sim::MAX_HITS + 1024,
    );
    let search = t.elapsed();

    let t = Instant::now();
    if cand.is_empty() {
        // 兜底线和界面同一条：现成的缩略图优先，没有再解一次原图。
        let img: Option<Rgba> = cache::peek(q, bw, bh).or_else(|| job::make(q, bw, bh).ok().map(|t| t.img));
        if let Some(img) = img {
            let qs = sim::variants(&img, false, false);
            cand = sim::search(bw, bh, &qs, algo, max_dist, sim::MAX_HITS + 1024);
        }
    }
    let fallback = t.elapsed();

    let t = Instant::now();
    let alive = alive_count(&cand);
    let meta = t.elapsed();

    let segs = Segs {
        load,
        missing,
        fill,
        search,
        fallback,
        meta,
        total: t_all.elapsed(),
    };
    (segs, cand.len(), alive, made, n_todo)
}

/// 独立读一遍名字名单挑查询图——不走 cache::path_map()，
/// 免得把"第一次搜图"要付的那笔载入提前付掉。
fn pick_query(rows: &[(u64, u16, u16, u32, u64, sim::Fp)], bw: u16, bh: u16) -> Option<String> {
    let mut want: HashMap<u64, ()> = HashMap::new();
    for (h, w, hh, _, _, _) in rows {
        if *w == bw && *hh == bh {
            want.insert(*h, ());
        }
    }
    if want.is_empty() {
        return None;
    }
    let all = std::fs::read(cache::dir().join("cache2.name")).ok()?;
    let mut cands: Vec<String> = Vec::new();
    for line in all.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let p = String::from_utf8_lossy(line).into_owned();
        // 测试台自己的样图不算“库里的图”，挑一张用户真看过的。
        let low = p.to_ascii_lowercase();
        if low.contains("xsee-convtest") || low.contains("\\temp\\") || low.contains("/temp/") {
            continue;
        }
        if want.contains_key(&cache::hash_of(&p)) && std::path::Path::new(&p).is_file() {
            cands.push(p);
        }
    }
    cands.sort();
    cands.into_iter().find(|p| cache::peek(p, bw as u32, bh as u32).is_some())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dry = args.iter().any(|a| a == "dry");
    let noauto = args.iter().any(|a| a == "noauto");
    let fill_cap: Option<usize> = args
        .iter()
        .find_map(|a| a.strip_prefix("fill=").and_then(|s| s.parse().ok()));
    job::init_com();
    println!("cache={}", cache::dir().display());
    println!("cache_on={}", cache::on());

    // 界面启动浏览目录时索引就已经读进内存了，这里先付掉这笔，
    // 让下面两遍都只算搜图线程里那一段。
    let t = Instant::now();
    let rows = cache::fps();
    println!("[{} ms] 索引已在内存：{} 条带指纹的记录", ms(t.elapsed()), rows.len());

    let mut tiers: HashMap<(u16, u16), usize> = HashMap::new();
    for (_, bw, bh, _, _, _) in &rows {
        *tiers.entry((*bw, *bh)).or_default() += 1;
    }
    let mut tv: Vec<((u16, u16), usize)> = tiers.into_iter().collect();
    tv.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("档位(有指纹)：{:?}", tv);

    let Some(((bw, bh), _)) = tv.first().copied() else {
        println!("这一档指纹库是空的，没法测");
        return;
    };
    let (bwu, bhu) = (bw as u32, bh as u32);
    println!("用账最多的一档 {}x{} 来测", bwu, bhu);

    let Some(q) = pick_query(&rows, bw, bh) else {
        println!("没在缓存里找到能用的查询图");
        return;
    };
    println!("查询图：{}", q);

    let algo = sim::ALGO_ALL;
    let max_dist = sim::dist_from_pct(87, algo);

    if dry {
        let t = Instant::now();
        sim::ensure_loaded();
        let load = t.elapsed();
        let t = Instant::now();
        let todo = cache::missing_fps(bwu, bhu, None);
        let miss = t.elapsed();
        let (tiers, total) = sim::stats_all();
        println!(
            "dry: 载入 {} ms，查缺 {} ms，库 {} 档 {} 条，该档待补 {} 条（不补不搜）",
            ms(load),
            ms(miss),
            tiers,
            total,
            todo.len()
        );
        return;
    }

    for i in 1..=2 {
        let (s, hits, alive, made, n_todo) =
            run_once(bwu, bhu, &q, algo, max_dist, noauto, fill_cap);
        println!(
            "第 {} 次：总 {} ms | 载入 {} | 查缺 {} | 补库 {} | 跨档搜 {} | 兜底 {} | 查盘 {} | 命中 {} 张，盘上还在 {} 张 | 待补 {} 条，补了 {} 条",
            i,
            ms(s.total),
            ms(s.load),
            ms(s.missing),
            ms(s.fill),
            ms(s.search),
            ms(s.fallback),
            ms(s.meta),
            hits,
            alive,
            n_todo,
            made
        );
        if i == 1 {
            let (tiers, total) = sim::stats_all();
            println!("  相似库现在 {} 档 {} 条", tiers, total);
        }
    }
}

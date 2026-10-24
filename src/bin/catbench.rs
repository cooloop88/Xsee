/// 量分类库读盘/取一个类别/整份写盘的耗时。只读真库，写盘写到临时目录。
/// 用法：cargo run --release --bin catbench [catalog.tsv 路径]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use xsee_host::catdb;

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| {
        // 不写死任何机器上的路径：默认就是 exe 旁边那份真库
        std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|d| d.join("data").join("catalog.tsv")))
            .unwrap_or_else(|| PathBuf::from("data").join("catalog.tsv"))
            .display()
            .to_string()
    });
    let p = PathBuf::from(&arg);
    println!("读 {}", p.display());

    let t = Instant::now();
    let (cat, st) = catdb::CatEdit::load_full(&p);
    let ms = t.elapsed().as_millis();
    let st = match st {
        catdb::OverlayLoad::Main => "主文件",
        catdb::OverlayLoad::Temp => "临时文件",
        catdb::OverlayLoad::Backup => "上一版",
        catdb::OverlayLoad::Corrupt => "已损坏改名",
        catdb::OverlayLoad::Empty => "空",
    };
    println!(
        "load_full：{ms} ms · 读自 {st} · 类别 {} · 归类 {} · 时间 {}",
        cat.n_cats(),
        cat.n_rows(),
        cat.n_assets()
    );

    // 找张数最多的那个类别（看名字里的根节点）
    let mut roots: Vec<i64> = cat.roots.clone();
    roots.sort_by_key(|i| std::cmp::Reverse(cat.count(*i)));
    for (k, id) in roots.iter().take(3).enumerate() {
        let top = *id;
        let mut best = (0usize, top);
        // 整棵树里挑最大的节点（含它自己的子类）
        fn walk(c: &catdb::CatEdit, id: i64, best: &mut (usize, i64)) {
            let n = c.count(id);
            if n > best.0 {
                *best = (n, id);
            }
            for k in c.child_ids(id) {
                walk(c, *k, best);
            }
        }
        walk(&cat, top, &mut best);
        let id = if k == 0 { best.1 } else { top };
        let t = Instant::now();
        let v1 = cat.files(id);
        let a = t.elapsed().as_micros();
        let t = Instant::now();
        let v2 = cat.files(id);
        let b = t.elapsed().as_micros();
        let t = Instant::now();
        let mut keys: HashMap<String, usize> = HashMap::new();
        for pth in &v2 {
            *keys.entry(pth.to_ascii_lowercase()).or_default() += 1;
        }
        let c = t.elapsed().as_micros();
        println!(
            "类别 {id}（{} 张）：files 第一次 {a} µs · 第二次 {b} µs · 排一遍小写键 {c} µs",
            v1.len()
        );
    }

    let t = Instant::now();
    let text = cat.dump();
    let d = t.elapsed().as_micros();
    let tmp = std::env::temp_dir().join("catbench_out.tsv");
    let t = Instant::now();
    let _ = catdb::write_catalog(Path::new(&tmp), &text);
    let w = t.elapsed().as_micros();
    println!(
        "dump {} 字节 {d} µs · 整份写盘+同步+替换 {w} µs（临时文件 {}）",
        text.len(),
        tmp.display()
    );
    let _ = std::fs::remove_file(&tmp);

    // 每一下分类操作都要跑的部分：改一笔 → 全树张数重算 → 整份文本重排 → 落盘
    // 点开类别这三步：取名单 → 逐张问大小（2000 张以内程序会算总大小）→ 建列表
    for (label, id) in [("综合类别 13842", 59i64), ("吊灯 1554", 133i64), ("小类", 21i64)] {
        let t = Instant::now();
        let v = cat.files(id);
        let f = t.elapsed().as_micros();
        let t = Instant::now();
        let mut sum = 0u64;
        if v.len() <= 2000 {
            for p in &v {
                if let Ok(m) = std::fs::metadata(p) {
                    sum += m.len();
                }
            }
        }
        let st = t.elapsed().as_micros();
        let t = Instant::now();
        let owned: Vec<(String, bool, bool)> = v.iter().map(|p| (p.clone(), false, true)).collect();
        let ent = t.elapsed().as_micros();
        println!(
            "{label}（{} 张）：取名单 {f} µs · 逐张问大小 {st} µs（合计 {sum} 字节）· 建列表 {ent} µs",
            v.len()
        );
        let _ = owned;
    }

    let mut cat = cat;
    let one = cat.files(roots[0]).into_iter().next().unwrap_or_default();
    for k in 0..3 {
        let t = Instant::now();
        cat.toggle_paths(roots[0], &[one.clone()], true);
        let add = t.elapsed().as_micros();
        let t = Instant::now();
        cat.toggle_paths(roots[0], &[one.clone()], false);
        let del = t.elapsed().as_micros();
        let t = Instant::now();
        let s = cat.dump();
        let dm = t.elapsed().as_micros();
        println!("第 {k} 轮：挂一张 {add} µs · 摘一张 {del} µs · 重排文本 {dm} µs（{} 字节）", s.len());
    }
    let t = Instant::now();
    cat.rename_cat(roots[0], "改名测一下");
    let rn = t.elapsed().as_micros();
    println!("类别改名（含全树张数重算）：{rn} µs");
}

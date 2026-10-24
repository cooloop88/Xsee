//! 缩略图重建这条线的自证：分几个进程跑，看"缓存里那张被换成了别的图 → 重建能不能查出来并纠正"。
//! 用法: thfix <make|poison|check|again> 图片 [bw bh] [r g b]
use xsee_host::{Rgba, cache, job};

fn mean(img: &Rgba) -> [u32; 3] {
    let mut s = [0u64; 3];
    let mut n = 0u64;
    for p in img.px.chunks_exact(4) {
        for c in 0..3 {
            s[c] += p[c] as u64;
        }
        n += 1;
    }
    if n == 0 {
        return [0; 3];
    }
    [(s[0] / n) as u32, (s[1] / n) as u32, (s[2] / n) as u32]
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let Some(step) = a.first() else {
        eprintln!("用法: thfix <make|poison|check|again> 图片 [bw bh] [r g b]");
        return;
    };
    let path = a[1].clone();
    let bw: u32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(160);
    let bh: u32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(160);
    job::init_com();
    match step.as_str() {
        "make" => {
            let t = job::make(&path, bw, bh).expect("解图失败");
            println!(
                "make：{}×{} 内嵌={} 平均色={:?}",
                t.img.w,
                t.img.h,
                t.from_preview,
                mean(&t.img)
            );
        }
        "poison" => {
            let rgb = [
                a.get(4).and_then(|s| s.parse().ok()).unwrap_or(255) as u8,
                a.get(5).and_then(|s| s.parse().ok()).unwrap_or(0) as u8,
                a.get(6).and_then(|s| s.parse().ok()).unwrap_or(0) as u8,
            ];
            let old = cache::peek(&path, bw, bh).expect("缓存里没这张，先跑 make");
            let fake = Rgba {
                w: old.w,
                h: old.h,
                px: vec![rgb[0], rgb[1], rgb[2], 255].repeat((old.w * old.h) as usize),
            };
            cache::replace(&path, bw, bh, &fake);
            println!(
                "poison：原来 {:?} → 塞进去 {:?}（{}×{}）",
                mean(&old),
                rgb,
                old.w,
                old.h
            );
        }
        "check" => {
            let before = cache::peek(&path, bw, bh);
            println!(
                "重开进程后缓存读到：{}",
                match &before {
                    Some(i) => format!("{:?} {}×{}", mean(i), i.w, i.h),
                    None => "没有".to_string(),
                }
            );
            let (t, ok) = job::remake(&path, bw, bh).expect("重解失败");
            println!(
                "remake：和缓存里那张对不对={} 重解出来 {:?} {}×{}",
                ok,
                mean(&t.img),
                t.img.w,
                t.img.h
            );
            let after = cache::peek(&path, bw, bh).expect("重建后缓存反而没了");
            println!("重建后立刻读到：{:?}", mean(&after));
        }
        "again" => match cache::peek(&path, bw, bh) {
            Some(i) => println!("再重开一次进程读到：{:?} {}×{}", mean(&i), i.w, i.h),
            None => println!("再重开一次进程读到：没有"),
        },
        _ => eprintln!("不认识的一步：{step}"),
    }
}

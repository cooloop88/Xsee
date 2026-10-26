/// 皮肤系统的把关台和设计器。
/// 用法：
///   skin.exe                     列出所有皮肤，逐个跑规则校验，画一张迷你界面预览
///   skin.exe --derive 名字=#rrggbb   只给一个强调色，按规则推出一整套皮肤并写成 .skin 文件
use std::collections::HashMap;
use std::time::Instant;

use xsee_host::theme::{self, Skin, contrast};

fn hex(c: u32) -> String {
    format!("#{c:06x}")
}

/// 只给一个强调色时，其余 7 个基色按这条规则推：底是这个色去饱和压到近黑、
/// 带一点色相；文字是这个强调色洗白；边框是底色提亮带字色。
fn from_accent(name: &str, acc: u32) -> HashMap<String, u32> {
    let dark = lum_of(acc) > 0.45;
    let base = if dark {
        theme::mix(acc, 0x0e1114, 0.88)
    } else {
        theme::mix(acc, 0xf7f5ef, 0.9)
    };
    let ink = if dark {
        theme::mix(acc, 0xffffff, 0.72)
    } else {
        theme::mix(acc, 0x12140f, 0.82)
    };
    let mut kv: HashMap<String, u32> = HashMap::new();
    kv.insert("base".into(), base);
    kv.insert("surface".into(), theme::lighten(base, 0.05));
    kv.insert("canvas".into(), theme::darken(base, 0.35));
    kv.insert("ink".into(), ink);
    kv.insert("dim".into(), theme::mix(ink, base, 0.45));
    kv.insert("accent".into(), acc);
    kv.insert("border".into(), theme::mix(theme::lighten(base, 0.05), ink, 0.18));
    kv.insert("mark".into(), theme::mix(acc, 0xd9b56b, 0.35));
    let _ = name;
    kv
}

fn lum_of(c: u32) -> f64 {
    let r = ((c >> 16) & 255) as f64 / 255.0;
    let g = ((c >> 8) & 255) as f64 / 255.0;
    let b = (c & 255) as f64 / 255.0;
    (0.2126 * r + 0.7152 * g + 0.0722 * b).sqrt()
}

fn dump(s: &Skin) {
    let slots = [
        ("base", s.win),
        ("chrome", s.chrome),
        ("chrome_hi", s.chrome_hi),
        ("menu", s.menu),
        ("popup", s.popup),
        ("canvas", s.canvas),
        ("pane", s.pane),
        ("pane_head", s.pane_head),
        ("grid", s.grid),
        ("line", s.line),
        ("line_soft", s.line_soft),
        ("ink", s.ink),
        ("dim", s.ink_dim),
        ("off", s.ink_off),
        ("inv", s.ink_inv),
        ("hov", s.hov),
        ("prs", s.prs),
        ("sel", s.sel),
        ("sel_ink", s.sel_ink),
        ("accent", s.acc),
        ("acc_soft", s.acc_soft),
        ("mark", s.mark),
        ("flash", s.flash),
    ];
    for c in slots.chunks(5) {
        println!(
            "   {}",
            c.iter()
                .map(|(k, v)| format!("{k}={}", hex(*v)))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    for (a, b, what, min) in [
        (s.ink, s.canvas, "正文/画布", 4.5),
        (s.ink, s.chrome, "正文/工具条", 4.5),
        (s.ink_dim, s.canvas, "次文/画布", 3.0),
        (s.sel_ink, s.sel, "选中字/选中底", 4.5),
        (s.ink_off, s.canvas, "灰显/画布", 2.0),
    ] {
        let c = contrast(a, b);
        println!(
            "   {:<12} {:>5.2}  要求 ≥{min}  {}",
            what,
            c,
            if c >= min { "过" } else { "不过" }
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("skins")))
        .unwrap_or_else(|| "work\\skins".into());
    let _ = std::fs::create_dir_all(&out);

    if let Some(i) = args.iter().position(|a| a == "--derive") {
        let spec = args.get(i + 1).cloned().unwrap_or_default();
        let Some((name, hx)) = spec.split_once('=') else {
            eprintln!("--derive 名字=#rrggbb");
            return;
        };
        let acc = u32::from_str_radix(hx.trim_start_matches(['#', '0', 'x', 'X']), 16).expect("颜色要写 #rrggbb");
        let kv = from_accent(name, acc);
        let (skin, rep) = Skin::derive(&kv).checked();
        println!("按规则从 {} 推出皮肤「{name}」：", hex(acc));
        dump(&skin);
        if !rep.0.is_empty() {
            for (w, min, got) in &rep.0 {
                println!("   规则不合格：{w} 实测 {got:.2} 要求 ≥{min}");
            }
        }
        let p = out.join(format!("{name}.skin"));
        let mut text = format!(
            "# 只写 8 个基色，其余 13 个槽位由 theme.rs 的派生规则算出来\n\
             # style=plain|bands|cards|cards_rich|cards_fine 定装饰工艺，order=数字 定菜单次序，default=1 设为启动默认\n\
             name={name}\n"
        );
        for k in ["base", "surface", "canvas", "ink", "dim", "accent", "border", "mark"] {
            text.push_str(&format!("{k}={}\n", hex(kv[k])));
        }
        if let Err(e) = std::fs::write(&p, text) {
            println!("   写不下去 {p:?}：{e}");
        } else {
            println!("   皮肤文件 → {}", p.display());
            let _ = theme::mock(&skin).save(out.join(format!("{name}_预览.png")));
            println!("   迷你界面预览 → {}", out.join(format!("{name}_预览.png")).display());
        }
        return;
    }

    let t = Instant::now();
    let names = theme::skins();
    let reg_ms = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "一共 {} 套皮肤，全部来自 {}，注册表建完用 {:.3} ms",
        names.len(),
        out.display(),
        reg_ms
    );
    for n in &names {
        let (s, bad) = theme::audit(n);
        println!("\n== {n} =={} 项不合格", bad.len());
        dump(&s);
        for b in &bad {
            println!("   规则不合格：{b}");
        }
        let f = out.join(format!("{n}.png"));
        match theme::mock(&s).save(&f) {
            Ok(()) => println!("   迷你界面预览 → {}", f.display()),
            Err(e) => println!("   预览存不下：{e}"),
        }
    }
}

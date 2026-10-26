//! 把 `glyphs::all()` 里的图标库写成 `skins\图标\<序号>-<key>.icons`，
//! 每套一个文件。exe 里只编译了出厂默认那一套，其余全靠这个文件夹。
//! 用法：`cargo run --release --bin mkglyphs -- <skins目录>`（不给就写 exe 旁边的 skins\图标）。
use std::path::PathBuf;

fn main() {
    let out = match std::env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => match std::env::current_exe() {
            Ok(e) => e.parent().unwrap().join("skins").join("图标"),
            Err(_) => PathBuf::from("图标"),
        },
    };
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("建目录失败 {}: {e}", out.display());
        std::process::exit(1);
    }
    let mut n = 0;
    for (i, s) in xsee_host::glyphs::all().iter().enumerate() {
        let mut text = String::new();
        text.push_str("# Xsee 图标库（版式）。name= 菜单里显示的名字，key= appearance.tsv 里 layout= 存的键。\n");
        text.push_str("# [0]..[9] 十枚固定顺序（上一/下一/上一层/查找/编辑/移到/复制到/删除/属性/外观），\n");
        text.push_str("# g= 这枚的网格边长，p= 一笔：路径|fill|even|alpha|笔宽|dx|dy；整枚缺块就用内置图形顶上。\n");
        text.push_str(&format!("name={}\n", s.label));
        text.push_str(&format!("key={}\n", s.key));
        for (gi, g) in s.glyphs.iter().enumerate() {
            let Some(g) = g else { continue };
            text.push_str(&format!("\n[{gi}]\ng={}\n", g.grid));
            for p in g.parts {
                text.push_str(&format!(
                    "p={}|{}|{}|{}|{}|{}|{}\n",
                    p.d,
                    p.fill as u8,
                    p.even as u8,
                    p.alpha,
                    p.w,
                    p.dx,
                    p.dy
                ));
            }
        }
        text.push('\n');
        let file = out.join(format!("{:02}-{}.icons", i, s.key));
        if let Err(e) = std::fs::write(&file, text) {
            eprintln!("写 {} 失败: {e}", file.display());
            std::process::exit(1);
        }
        println!("{:02}-{}.icons", i, s.key);
        n += 1;
    }
    println!("共 {n} 套 → {}", out.display());
}

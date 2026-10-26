// 每档每枚用的是上游哪个图标，名字记在 LICENSES/glyph-names.json；
// 许可与版权行见 THIRD-PARTY.md。
//
// 每个 Set 是一套开源图标库（MIT / ISC / Apache 2.0 / Unlicense，个别 CC BY 4.0）。
// 图形原样取自各库，路径已经摊平成运行时那套解析器认识的命令
// （M/L/H/V/C/S/A/Z），圆弧仍在，只是 Q/T 换成了三次贝塞尔。
//
// 这里只编译出厂默认那一套（Lucide）；整套的正式住处在 skins\图标\ 下，
// 一套一个 *.icons 文件，`cargo run --bin mkglyphs` 可以重新导出。
// 盘上的文件优先于编译进来的：同名 key 以文件为准。

/// 一段路径。fill 为真走实心填充，否则按 w（图标自己的网格单位）描边；
/// alpha 是库里的 opacity（双色档的第二层就是这么画的）。
pub struct Part {
    pub d: &'static str,
    pub fill: bool,
    pub even: bool,
    pub alpha: f32,
    pub w: f32,
    pub dx: f32,
    pub dy: f32,
}

pub struct Glyph {
    /// 这套图标的网格边长（24 / 16 / 20 / 256 …），画的时候按它换算笔宽和坐标。
    pub grid: f32,
    pub parts: &'static [Part],
}

#[derive(Clone)]
pub struct Set {
    pub key: &'static str,
    pub label: &'static str,
    /// 顺序固定：上一文件夹 / 下一文件夹 / 上一层 / 查找 / 编辑 / 移到 / 复制到 /
    /// 删除 / 属性 / 外观。
    pub glyphs: [Option<&'static Glyph>; 10],
}

static G_LUCIDE_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17.97 4.29 A 2 2 0 0 1 21 6 V 18 A 2 2 0 0 1 17.97 19.71 L 7.97 13.72 A 2 2 0 0 1 7.97 10.28 z M 3 20 V 4", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LUCIDE_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 4 V 20 M 6.03 4.29 A 2 2 0 0 0 3 6 V 18 A 2 2 0 0 0 6.03 19.71 L 16.03 13.72 A 2 2 0 0 0 16.03 10.28 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LUCIDE_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 12 L 12 5 L 19 12 M 12 19 V 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LUCIDE_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 21 L 16.66 16.66", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 3 11 A 8 8 0 1 0 19 11 A 8 8 0 1 0 3 11", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LUCIDE_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 3 H 5 A 2 2 0 0 0 3 5 V 19 A 2 2 0 0 0 5 21 H 19 A 2 2 0 0 0 21 19 V 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 18.38 2.62 A 1 1 0 0 1 21.38 5.62 L 12.36 14.64 A 2 2 0 0 1 11.51 15.14 L 8.64 15.98 A 0.5 0.5 0 0 1 8.02 15.36 L 8.86 12.49 A 2 2 0 0 1 9.36 11.64 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LUCIDE_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 9 V 5 A 2 2 0 0 1 4 3 H 7.9 A 2 2 0 0 1 9.59 3.9 L 10.4 5.1 A 2 2 0 0 0 12.07 6 H 20 A 2 2 0 0 1 22 8 V 18 A 2 2 0 0 1 20 20 H 4 A 2 2 0 0 1 2 18 V 17 M 2 13 H 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 9 16 L 12 13 L 9 10", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LUCIDE_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 8 H 20 A 2 2 0 0 1 22 10 V 20 A 2 2 0 0 1 20 22 H 10 A 2 2 0 0 1 8 20 V 10 A 2 2 0 0 1 10 8 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4 16 C 2.9 16 2 15.1 2 14 V 4 C 2 2.9 2.9 2 4 2 H 14 C 15.1 2 16 2.9 16 4", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LUCIDE_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 11 V 17 M 14 11 V 17 M 19 6 V 20 A 2 2 0 0 1 17 22 H 7 A 2 2 0 0 1 5 20 V 6 M 3 6 H 21 M 8 6 V 4 A 2 2 0 0 1 10 2 H 14 A 2 2 0 0 1 16 4 V 6", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LUCIDE_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 12 A 10 10 0 1 0 22 12 A 10 10 0 1 0 2 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 16 V 12 M 12 8 H 12.01", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LUCIDE_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 22 A 1 1 0 0 1 12 2 A 10 9 0 0 1 22 11 A 5 5 0 0 1 17 16 H 14.75 A 1.75 1.75 0 0 0 13.35 18.8 L 13.65 19.2 A 1.75 1.75 0 0 1 12.25 22 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 13 6.5 A 0.5 0.5 0 1 0 14 6.5 A 0.5 0.5 0 1 0 13 6.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 13 6.5 A 0.5 0.5 0 1 0 14 6.5 A 0.5 0.5 0 1 0 13 6.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 17 10.5 A 0.5 0.5 0 1 0 18 10.5 A 0.5 0.5 0 1 0 17 10.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 17 10.5 A 0.5 0.5 0 1 0 18 10.5 A 0.5 0.5 0 1 0 17 10.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 6 12.5 A 0.5 0.5 0 1 0 7 12.5 A 0.5 0.5 0 1 0 6 12.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 6 12.5 A 0.5 0.5 0 1 0 7 12.5 A 0.5 0.5 0 1 0 6 12.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8 7.5 A 0.5 0.5 0 1 0 9 7.5 A 0.5 0.5 0 1 0 8 7.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8 7.5 A 0.5 0.5 0 1 0 9 7.5 A 0.5 0.5 0 1 0 8 7.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
/// 编译进 exe 的出厂默认那一套（theme::DEFAULT_LAYOUT 指向的 key）。
/// 单独拷一个 exe、skins\图标\ 不在或认不出来时，靠它顶住，版式菜单不至于空。
static BUILTIN: &[Set] = &[
    Set { key: "lucide", label: "Lucide · 线", glyphs: [Some(&G_LUCIDE_0), Some(&G_LUCIDE_1), Some(&G_LUCIDE_2), Some(&G_LUCIDE_3), Some(&G_LUCIDE_4), Some(&G_LUCIDE_5), Some(&G_LUCIDE_6), Some(&G_LUCIDE_7), Some(&G_LUCIDE_8), Some(&G_LUCIDE_9)] },
];

/// 全部可用的图标库：skins\图标\ 下的 *.icons 一套一个文件，按文件名开头的序号排；
/// 文件里没有出厂默认那套时把它插回去。进程内只装一次。
pub fn all() -> &'static [Set] {
    static ALL: std::sync::OnceLock<Vec<Set>> = std::sync::OnceLock::new();
    ALL.get_or_init(load_all)
}

fn load_all() -> Vec<Set> {
    let mut loaded: Vec<(u32, Set)> = Vec::new();
    let dir = crate::theme::skin_dir().join("图标");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().map(|x| x == "icons").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    for p in files {
        let ord = p
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.split('-').next())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(999);
        if let Some(s) = parse_set_file(&p) {
            if loaded.iter().any(|(_, e)| e.key == s.key) {
                continue;
            }
            loaded.push((ord, s));
        }
    }
    if !loaded.iter().any(|(_, e)| e.key == crate::theme::DEFAULT_LAYOUT) {
        loaded.push((2, BUILTIN[0].clone()));
    }
    loaded.sort_by_key(|(o, _)| *o);
    loaded.into_iter().map(|(_, s)| s).collect()
}

/// 一套一个文本文件：name= 菜单名，key= 偏好键，[0]..[9] 每枚一块，
/// 块里 g= 网格、p= 一笔（路径|fill|even|alpha|笔宽|dx|dy）。解析不动就整份跳过。
fn parse_set_file(p: &std::path::Path) -> Option<Set> {
    let text = std::fs::read_to_string(p).ok()?;
    let mut label = String::new();
    let mut key = String::new();
    let mut glyphs: [Option<&'static Glyph>; 10] = [None; 10];
    let mut cur: Option<usize> = None;
    let mut grid = 24.0f32;
    let mut parts: Vec<Part> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Ok(n) = line[1..line.len() - 1].trim().parse::<usize>() {
                if n < 10 {
                    if let Some(i) = cur.take() {
                        glyphs[i] = take_glyph(grid, std::mem::take(&mut parts));
                    }
                    cur = Some(n);
                    grid = 24.0;
                    continue;
                }
            }
        }
        if let Some(v) = line.strip_prefix("name=") {
            label = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("key=") {
            key = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("g=") {
            grid = v.trim().parse().unwrap_or(24.0);
        } else if let Some(v) = line.strip_prefix("p=") {
            let f: Vec<&str> = v.split('|').collect();
            if f.len() >= 7 {
                parts.push(Part {
                    d: Box::leak(f[0].trim().to_string().into_boxed_str()),
                    fill: f[1].trim() == "1",
                    even: f[2].trim() == "1",
                    alpha: f[3].trim().parse().unwrap_or(1.0),
                    w: f[4].trim().parse().unwrap_or(2.0),
                    dx: f[5].trim().parse().unwrap_or(0.0),
                    dy: f[6].trim().parse().unwrap_or(0.0),
                });
            }
        }
    }
    if let Some(i) = cur.take() {
        glyphs[i] = take_glyph(grid, std::mem::take(&mut parts));
    }
    if key.is_empty() {
        return None;
    }
    if label.is_empty() {
        label = key.clone();
    }
    Some(Set {
        key: Box::leak(key.into_boxed_str()),
        label: Box::leak(label.into_boxed_str()),
        glyphs,
    })
}

fn take_glyph(grid: f32, parts: Vec<Part>) -> Option<&'static Glyph> {
    if grid <= 0.0 || parts.is_empty() {
        return None;
    }
    Some(Box::leak(Box::new(Glyph {
        grid,
        parts: Box::leak(parts.into_boxed_slice()),
    })))
}

pub fn find(key: &str) -> Option<&'static Set> {
    all().iter().find(|s| s.key == key)
}

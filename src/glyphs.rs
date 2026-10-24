// 每档每枚用的是上游哪个图标，名字记在 LICENSES/glyph-names.json；
// 许可与版权行见 THIRD-PARTY.md。
//
// 每个 Set 是一套开源图标库（MIT / ISC / Apache 2.0 / Unlicense，个别 CC BY 4.0）。
// 图形原样取自各库，路径已经在这里摊平成运行时那套
// 解析器认识的命令（M/L/H/V/C/S/A/Z），圆弧仍在，只是 Q/T 换成了三次贝塞尔。

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

pub struct Set {
    pub key: &'static str,
    pub label: &'static str,
    /// 顺序固定：上一文件夹 / 下一文件夹 / 上一层 / 查找 / 编辑 / 移到 / 复制到 /
    /// 删除 / 属性 / 外观。
    pub glyphs: [Option<&'static Glyph>; 10],
}

static G_TABLER_LINE_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 12 H 20 M 10 12 L 14 16 M 10 12 L 14 8 M 4 4 V 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_LINE_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14 12 H 4 M 14 12 L 10 16 M 14 12 L 10 8 M 20 4 V 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_LINE_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 4 V 14 M 12 4 L 16 8 M 12 4 L 8 8 M 4 20 H 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_LINE_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 10 A 7 7 0 1 0 17 10 A 7 7 0 1 0 3 10 M 21 21 L 15 15", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_LINE_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 20 H 8 L 18.5 9.5 A 2.83 2.83 0 1 0 14.5 5.5 L 4 16 z M 13.5 6.5 L 17.5 10.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_LINE_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13 19 H 5 A 2 2 0 0 1 3 17 V 6 A 2 2 0 0 1 5 4 H 9 L 12 7 H 19 A 2 2 0 0 1 21 9 V 13 M 16 22 L 21 17 M 21 21.5 V 17 H 16.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_LINE_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 9.67 A 2.67 2.67 0 0 1 9.67 7 H 18.33 A 2.67 2.67 0 0 1 21 9.67 V 18.33 A 2.67 2.67 0 0 1 18.33 21 H 9.67 A 2.67 2.67 0 0 1 7 18.33 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4.01 16.74 A 2 2 0 0 1 3 15 V 5 C 3 3.9 3.9 3 5 3 H 15 C 15.75 3 16.16 3.38 16.5 4", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_LINE_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 7 H 20 M 10 11 V 17 M 14 11 V 17 M 5 7 L 6 19 A 2 2 0 0 0 8 21 H 16 A 2 2 0 0 0 18 19 L 19 7 M 9 7 V 4 A 1 1 0 0 1 10 3 H 14 A 1 1 0 0 1 15 4 V 7", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_LINE_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 12 A 9 9 0 1 0 21 12 A 9 9 0 0 0 3 12 M 12 9 H 12.01", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 11 12 H 12 V 16 H 13", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_LINE_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 21 A 9 9 0 0 1 12 3 C 16.97 3 21 6.58 21 11 C 21 12.06 20.53 13.08 19.68 13.83 S 17.69 15 16.5 15 H 14 A 2 2 0 0 0 13 18.75 A 1.3 1.3 0 0 1 12 21", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 7.5 10.5 A 1 1 0 1 0 9.5 10.5 A 1 1 0 1 0 7.5 10.5 M 11.5 7.5 A 1 1 0 1 0 13.5 7.5 A 1 1 0 1 0 11.5 7.5 M 15.5 10.5 A 1 1 0 1 0 17.5 10.5 A 1 1 0 1 0 15.5 10.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6.71 8.29 A 1 1 0 0 1 6.71 9.71 L 5.42 11 H 16.17 A 3 3 0 1 1 16.17 13 H 5.42 L 6.71 14.29 A 1 1 0 0 1 6.79 15.61 L 6.71 15.71 A 1 1 0 0 1 5.29 15.71 L 2.29 12.71 A 1 1 0 0 1 2.2 12.59 L 2.12 12.48 L 2.07 12.37 L 2.04 12.27 L 2.01 12.15 L 2 12.09 L 2 12 L 2 11.93 L 2.02 11.8 L 2.05 11.69 L 2.09 11.58 L 2.15 11.48 L 2.21 11.38 L 2.29 11.29 L 5.29 8.29 A 1 1 0 0 1 6.71 8.29", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 15 A 3 3 0 0 1 2 12 L 2 11.82 A 3 3 0 0 1 7.83 11 H 18.59 L 17.29 9.71 A 1 1 0 0 1 17.21 8.39 L 17.29 8.29 A 1 1 0 0 1 18.71 8.29 L 21.71 11.29 L 21.8 11.41 L 21.88 11.52 L 21.91 11.58 L 21.94 11.66 L 21.96 11.73 L 22 11.88 L 22 12 L 22 12.09 L 21.98 12.2 L 21.95 12.31 L 21.91 12.42 L 21.85 12.52 L 21.78 12.63 L 21.71 12.71 L 18.71 15.71 A 1 1 0 0 1 17.29 14.29 L 18.59 13 H 7.83 A 3 3 0 0 1 5 15", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.85 2.01 L 11.91 2 L 12 2 L 12.07 2 L 12.2 2.02 L 12.31 2.05 L 12.42 2.09 L 12.52 2.15 L 12.62 2.22 L 12.71 2.29 L 15.71 5.29 A 1 1 0 1 1 14.29 6.71 L 13 5.42 V 16.17 A 3 3 0 1 1 11 16.17 V 5.42 L 9.71 6.71 A 1 1 0 0 1 8.39 6.79 L 8.29 6.71 A 1 1 0 0 1 8.29 5.29 L 11.29 2.29 C 11.33 2.26 11.37 2.22 11.41 2.2 L 11.52 2.12 L 11.63 2.07 L 11.73 2.04 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14 3.07 A 8 8 0 0 1 16.32 14.91 L 21.71 20.29 A 1 1 0 0 1 20.29 21.71 L 14.9 16.32 A 8 8 0 1 1 14 3.07", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.09 6.5 L 17.5 11.91 L 8.71 20.71 A 1 1 0 0 1 8 21 H 4 A 1 1 0 0 1 3 20 V 16 A 1 1 0 0 1 3.29 15.29 z M 17.49 3.8 A 3.83 3.83 0 0 1 19.21 10.21 L 18.91 10.5 L 13.5 5.08 L 13.79 4.79 A 3.83 3.83 0 0 1 17.49 3.8", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18.71 8.29 L 21.71 11.29 C 21.74 11.33 21.78 11.37 21.8 11.4 L 21.88 11.51 L 21.93 11.63 L 21.96 11.73 L 21.99 11.88 L 22 12 L 22 12.07 L 21.98 12.2 L 21.95 12.31 L 21.91 12.42 L 21.85 12.52 L 21.78 12.62 L 21.71 12.71 L 18.71 15.71 A 1 1 0 0 1 17.29 14.29 L 18.59 13 H 8 V 14 A 1 1 0 0 1 7 15 H 3 A 1 1 0 0 1 2 14 V 10 A 1 1 0 0 1 3 9 H 7 A 1 1 0 0 1 8 10 V 11 H 18.59 L 17.29 9.71 A 1 1 0 0 1 17.21 8.39 L 17.29 8.29 A 1 1 0 0 1 18.71 8.29", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20.93 7.07 A 3.67 3.67 0 0 1 22 9.67 V 18.33 A 3.67 3.67 0 0 1 18.33 22 H 9.67 A 3.67 3.67 0 0 1 6 18.33 V 9.67 C 6 9.63 6 9.6 6 9.56 A 3.66 3.66 0 0 1 9.67 6 H 18.33 C 19.31 6 20.24 6.39 20.93 7.07", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 17.37 3.51 A 1 1 0 1 1 15.63 4.49 C 15.4 4.09 15.28 4 15 4 H 5 C 4.45 4 4 4.45 4 5 V 15 C 4 15.36 4.19 15.69 4.51 15.87 A 1 1 0 1 1 3.52 17.61 A 3 3 0 0 1 2 15 V 5 C 2 3.35 3.35 2 5 2 H 15 C 16.09 2 16.83 2.53 17.37 3.51", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 6 A 1 1 0 0 1 20.12 7.99 L 20 8 H 19.92 L 19 19 A 3 3 0 0 1 16.18 22 L 16 22 H 8 C 6.4 22 5.1 20.75 5.01 19.25 L 5 19.08 L 4.08 8 H 4 A 1 1 0 0 1 3.88 6.01 L 4 6 z M 10 10 A 1 1 0 0 0 9 11 V 17 A 1 1 0 0 0 11 17 V 11 A 1 1 0 0 0 10 10 M 14 10 A 1 1 0 0 0 13 11 V 17 A 1 1 0 0 0 15 17 V 11 A 1 1 0 0 0 14 10 M 14 2 A 2 2 0 0 1 16 4 A 1 1 0 0 1 14.01 4.12 L 14 4 H 10 L 9.99 4.12 A 1 1 0 0 1 8 4 A 2 2 0 0 1 9.85 2 L 10 2 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 2 C 17.52 2 22 6.48 22 12 A 10 10 0 0 1 2 12.32 L 2 12 L 2 11.72 C 2.15 6.33 6.57 2 12 2 M 12 11 H 11 L 10.88 11.01 A 1 1 0 0 0 10.88 12.99 L 11 13 V 16 L 11.01 16.12 A 1 1 0 0 0 11.88 16.99 L 12 17 H 13 L 13.12 16.99 A 1 1 0 0 0 13.99 16.12 L 14 16 L 13.99 15.88 A 1 1 0 0 0 13.23 15.03 L 13.12 15.01 L 13 15 V 12 L 12.99 11.88 A 1 1 0 0 0 12.12 11.01 z M 12.01 8 L 11.88 8.01 A 1 1 0 0 0 11.88 9.99 L 12 10 L 12.13 9.99 A 1 1 0 0 0 12.13 8.01 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TABLER_FILLED_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 2 C 17.5 2 22 6 22 11 C 22 12.35 21.4 13.64 20.35 14.58 C 19.32 15.49 17.93 16 16.5 16 H 13.98 A 1 1 0 0 0 13.48 17.88 A 1 1 0 0 1 13.68 18.02 A 2.3 2.3 0 0 1 12.08 22.01 L 11.93 22 L 11.99 22 L 11.72 22 C 6.42 21.85 2.15 17.58 2 12.28 L 2 12 C 2 6.48 6.48 2 12 2 M 8.5 8.5 A 2 2 0 0 0 6.5 10.35 L 6.5 10.5 A 2 2 0 1 0 8.5 8.5 M 16.5 8.5 A 2 2 0 0 0 14.5 10.35 L 14.5 10.5 A 2 2 0 1 0 16.5 8.5 M 12.5 5.5 A 2 2 0 0 0 10.5 7.35 L 10.5 7.5 A 2 2 0 1 0 12.5 5.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
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
static G_BI_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8.35 1.65 A 0.5 0.5 0 0 1 8.35 2.35 L 2.71 8 L 8.35 13.65 A 0.5 0.5 0 0 1 7.65 14.35 L 1.65 8.35 A 0.5 0.5 0 0 1 1.65 7.65 L 7.65 1.65 A 0.5 0.5 0 0 1 8.35 1.65", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12.35 1.65 A 0.5 0.5 0 0 1 12.35 2.35 L 6.71 8 L 12.35 13.65 A 0.5 0.5 0 0 1 11.65 14.35 L 5.65 8.35 A 0.5 0.5 0 0 1 5.65 7.65 L 11.65 1.65 A 0.5 0.5 0 0 1 12.35 1.65", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_BI_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3.65 1.65 A 0.5 0.5 0 0 1 4.35 1.65 L 10.35 7.65 A 0.5 0.5 0 0 1 10.35 8.35 L 4.35 14.35 A 0.5 0.5 0 0 1 3.65 13.65 L 9.29 8 L 3.65 2.35 A 0.5 0.5 0 0 1 3.65 1.65", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 7.65 1.65 A 0.5 0.5 0 0 1 8.35 1.65 L 14.35 7.65 A 0.5 0.5 0 0 1 14.35 8.35 L 8.35 14.35 A 0.5 0.5 0 0 1 7.65 13.65 L 13.29 8 L 7.65 2.35 A 0.5 0.5 0 0 1 7.65 1.65", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_BI_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 10 A 0.5 0.5 0 0 0 8.5 9.5 V 3.71 L 10.65 5.85 A 0.5 0.5 0 0 0 11.35 5.15 L 8.35 2.15 A 0.5 0.5 0 0 0 7.65 2.15 L 4.65 5.15 A 0.5 0.5 0 1 0 5.35 5.85 L 7.5 3.71 V 9.5 A 0.5 0.5 0 0 0 8 10 M 1 12.5 A 0.5 0.5 0 0 1 1.5 12 H 14.5 A 0.5 0.5 0 0 1 14.5 13 H 1.5 A 0.5 0.5 0 0 1 1 12.5", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_BI_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.74 10.34 A 6.5 6.5 0 1 0 10.35 11.74 H 10.34 C 10.37 11.78 10.41 11.82 10.44 11.86 L 14.29 15.71 A 1 1 0 0 0 15.71 14.29 L 11.86 10.44 A 1 1 0 0 0 11.74 10.34 z M 12 6.5 A 5.5 5.5 0 1 1 1 6.5 A 5.5 5.5 0 0 1 12 6.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_BI_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.15 0.15 A 0.5 0.5 0 0 1 12.85 0.15 L 15.85 3.15 A 0.5 0.5 0 0 1 15.85 3.85 L 5.85 13.85 A 0.5 0.5 0 0 1 5.69 13.96 L 0.69 15.96 A 0.5 0.5 0 0 1 0.04 15.31 L 2.04 10.31 A 0.5 0.5 0 0 1 2.15 10.15 z M 11.21 2.5 L 13.5 4.79 L 14.79 3.5 L 12.5 1.21 z M 12.79 5.5 L 10.5 3.21 L 4 9.71 V 10 H 4.5 A 0.5 0.5 0 0 1 5 10.5 V 11 H 5.5 A 0.5 0.5 0 0 1 6 11.5 V 12 H 6.29 z M 3.03 10.68 L 2.93 10.78 L 1.4 14.6 L 5.22 13.07 L 5.33 12.97 A 0.5 0.5 0 0 1 5 12.5 V 12 H 4.5 A 0.5 0.5 0 0 1 4 11.5 V 11 H 3.5 A 0.5 0.5 0 0 1 3.03 10.68", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_BI_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15 2 A 1 1 0 0 0 14 1 H 2 A 1 1 0 0 0 1 2 V 14 A 1 1 0 0 0 2 15 H 14 A 1 1 0 0 0 15 14 z M 0 2 A 2 2 0 0 1 2 0 H 14 A 2 2 0 0 1 16 2 V 14 A 2 2 0 0 1 14 16 H 2 A 2 2 0 0 1 0 14 z M 4.5 7.5 A 0.5 0.5 0 0 0 4.5 8.5 H 10.29 L 8.15 10.65 A 0.5 0.5 0 0 0 8.85 11.35 L 11.85 8.35 A 0.5 0.5 0 0 0 11.85 7.65 L 8.85 4.65 A 0.5 0.5 0 1 0 8.15 5.35 L 10.29 7.5 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_BI_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 2 A 2 2 0 0 1 6 0 H 14 A 2 2 0 0 1 16 2 V 10 A 2 2 0 0 1 14 12 H 6 A 2 2 0 0 1 4 10 z M 6 1 A 1 1 0 0 0 5 2 V 10 A 1 1 0 0 0 6 11 H 14 A 1 1 0 0 0 15 10 V 2 A 1 1 0 0 0 14 1 z M 2 5 A 1 1 0 0 0 1 6 V 14 A 1 1 0 0 0 2 15 H 10 A 1 1 0 0 0 11 14 V 13 H 12 V 14 A 2 2 0 0 1 10 16 H 2 A 2 2 0 0 1 0 14 V 6 A 2 2 0 0 1 2 4 H 3 V 5 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_BI_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.5 5.5 A 0.5 0.5 0 0 1 6 6 V 12 A 0.5 0.5 0 0 1 5 12 V 6 A 0.5 0.5 0 0 1 5.5 5.5 M 8 5.5 A 0.5 0.5 0 0 1 8.5 6 V 12 A 0.5 0.5 0 0 1 7.5 12 V 6 A 0.5 0.5 0 0 1 8 5.5 M 11 6 A 0.5 0.5 0 0 0 10 6 V 12 A 0.5 0.5 0 0 0 11 12 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 14.5 3 A 1 1 0 0 1 13.5 4 H 13 V 13 A 2 2 0 0 1 11 15 H 5 A 2 2 0 0 1 3 13 V 4 H 2.5 A 1 1 0 0 1 1.5 3 V 2 A 1 1 0 0 1 2.5 1 H 6 A 1 1 0 0 1 7 0 H 9 A 1 1 0 0 1 10 1 H 13.5 A 1 1 0 0 1 14.5 2 z M 4.12 4 L 4 4.06 V 13 A 1 1 0 0 0 5 14 H 11 A 1 1 0 0 0 12 13 V 4.06 L 11.88 4 z M 2.5 3 H 13.5 V 2 H 2.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_BI_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 15 A 7 7 0 1 1 8 1 A 7 7 0 0 1 8 15 M 8 16 A 8 8 0 1 0 8 0 A 8 8 0 0 0 8 16", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8.93 6.59 L 6.64 6.88 L 6.56 7.25 L 7.01 7.34 C 7.3 7.41 7.36 7.51 7.3 7.81 L 6.56 11.28 C 6.36 12.17 6.66 12.59 7.37 12.59 C 7.91 12.59 8.54 12.34 8.83 12 L 8.92 11.58 C 8.72 11.76 8.43 11.83 8.23 11.83 C 7.96 11.83 7.86 11.63 7.93 11.29 z M 9 4.5 A 1 1 0 1 1 7 4.5 A 1 1 0 0 1 9 4.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_BI_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 5 A 1.5 1.5 0 1 0 8 2 A 1.5 1.5 0 0 0 8 5 M 12 8 A 1.5 1.5 0 1 0 12 5 A 1.5 1.5 0 0 0 12 8 M 5.5 7 A 1.5 1.5 0 1 1 2.5 7 A 1.5 1.5 0 0 1 5.5 7 M 6 13 A 1.5 1.5 0 1 0 6 10 A 1.5 1.5 0 0 0 6 13", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 16 8 C 16 11.15 14.13 10.59 12.43 10.07 C 11.42 9.76 10.46 9.47 10 10 C 9.4 10.68 9.53 11.82 9.65 12.92 C 9.83 14.49 10 16 8 16 A 8 8 0 1 1 16 8 M 8 15 C 8.61 15 8.65 14.83 8.65 14.82 C 8.73 14.68 8.78 14.36 8.72 13.71 C 8.71 13.54 8.69 13.34 8.66 13.11 C 8.61 12.65 8.55 12.11 8.55 11.65 C 8.54 10.95 8.63 10.04 9.25 9.34 C 9.62 8.92 10.1 8.76 10.52 8.72 C 10.93 8.68 11.33 8.75 11.68 8.82 C 12.03 8.9 12.38 9.01 12.71 9.11 L 12.74 9.12 C 13.08 9.22 13.39 9.31 13.69 9.38 C 14.34 9.53 14.59 9.46 14.68 9.41 C 14.72 9.38 15 9.16 15 8 A 7 7 0 1 0 8 15", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_0: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 232 128 A 8 8 0 0 1 224 136 H 91.31 L 149.66 194.34 A 8 8 0 0 1 138.34 205.66 L 66.34 133.66 A 8 8 0 0 1 66.34 122.34 L 138.34 50.34 A 8 8 0 0 1 149.66 61.66 L 91.31 120 H 224 A 8 8 0 0 1 232 128 M 40 32 A 8 8 0 0 0 32 40 V 216 A 8 8 0 0 0 48 216 V 40 A 8 8 0 0 0 40 32", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_1: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 189.66 122.34 A 8 8 0 0 1 189.66 133.66 L 117.66 205.66 A 8 8 0 0 1 106.34 194.34 L 164.69 136 H 32 A 8 8 0 0 1 32 120 H 164.69 L 106.34 61.66 A 8 8 0 0 1 117.66 50.34 Z M 216 32 A 8 8 0 0 0 208 40 V 216 A 8 8 0 0 0 224 216 V 40 A 8 8 0 0 0 216 32", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_2: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 205.66 138.34 A 8 8 0 0 1 194.34 149.66 L 136 91.31 V 224 A 8 8 0 0 1 120 224 V 91.31 L 61.66 149.66 A 8 8 0 0 1 50.34 138.34 L 122.34 66.34 A 8 8 0 0 1 133.66 66.34 Z M 216 32 H 40 A 8 8 0 0 0 40 48 H 216 A 8 8 0 0 0 216 32", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_3: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 229.66 218.34 L 179.59 168.28 A 88.11 88.11 0 1 0 168.28 179.59 L 218.34 229.66 A 8 8 0 0 0 229.66 218.34 M 40 112 A 72 72 0 1 1 112 184 A 72.08 72.08 0 0 1 40 112", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_4: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 227.31 73.37 L 182.63 28.68 A 16 16 0 0 0 160 28.68 L 36.69 152 A 15.86 15.86 0 0 0 32 163.31 V 208 A 16 16 0 0 0 48 224 H 92.69 A 15.86 15.86 0 0 0 104 219.31 L 227.31 96 A 16 16 0 0 0 227.31 73.37 M 51.31 160 L 136 75.31 L 152.69 92 L 68 176.68 Z M 48 179.31 L 76.69 208 H 48 Z M 96 204.69 L 79.31 188 L 164 103.31 L 180.69 120 Z M 192 108.69 L 147.31 64 L 171.31 40 L 216 84.68 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_5: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 112 V 208 A 16 16 0 0 1 200 224 H 56 A 16 16 0 0 1 40 208 V 112 A 16 16 0 0 1 56 96 H 80 A 8 8 0 0 1 80 112 H 56 V 208 H 200 V 112 H 176 A 8 8 0 0 1 176 96 H 200 A 16 16 0 0 1 216 112 M 93.66 69.66 L 120 43.31 V 136 A 8 8 0 0 0 136 136 V 43.31 L 162.34 69.66 A 8 8 0 0 0 173.66 58.34 L 133.66 18.34 A 8 8 0 0 0 122.34 18.34 L 82.34 58.34 A 8 8 0 0 0 93.66 69.66", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_6: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 32 H 88 A 8 8 0 0 0 80 40 V 80 H 40 A 8 8 0 0 0 32 88 V 216 A 8 8 0 0 0 40 224 H 168 A 8 8 0 0 0 176 216 V 176 H 216 A 8 8 0 0 0 224 168 V 40 A 8 8 0 0 0 216 32 M 160 208 H 48 V 96 H 160 Z M 208 160 H 176 V 88 A 8 8 0 0 0 168 80 H 96 V 48 H 208 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_7: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 48 H 176 V 40 A 24 24 0 0 0 152 16 H 104 A 24 24 0 0 0 80 40 V 48 H 40 A 8 8 0 0 0 40 64 H 48 V 208 A 16 16 0 0 0 64 224 H 192 A 16 16 0 0 0 208 208 V 64 H 216 A 8 8 0 0 0 216 48 M 96 40 A 8 8 0 0 1 104 32 H 152 A 8 8 0 0 1 160 40 V 48 H 96 Z M 192 208 H 64 V 64 H 192 Z M 112 104 V 168 A 8 8 0 0 1 96 168 V 104 A 8 8 0 0 1 112 104 M 160 104 V 168 A 8 8 0 0 1 144 168 V 104 A 8 8 0 0 1 160 104", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_8: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 128 24 A 104 104 0 1 0 232 128 A 104.11 104.11 0 0 0 128 24 M 128 216 A 88 88 0 1 1 216 128 A 88.1 88.1 0 0 1 128 216 M 144 176 A 8 8 0 0 1 136 184 A 16 16 0 0 1 120 168 V 128 A 8 8 0 0 1 120 112 A 16 16 0 0 1 136 128 V 168 A 8 8 0 0 1 144 176 M 112 84 A 12 12 0 1 1 124 96 A 12 12 0 0 1 112 84", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_REGULAR_9: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 200.77 53.89 A 103.27 103.27 0 0 0 128 24 H 126.93 A 104 104 0 0 0 24 128 C 24 171 50.58 207.06 93.36 222.17 A 32 32 0 0 0 136 192 A 16 16 0 0 1 152 176 H 198.21 A 31.81 31.81 0 0 0 229.41 151.12 A 104.4 104.4 0 0 0 232 127.12 A 103.28 103.28 0 0 0 200.77 53.89 M 213.77 147.6 A 15.89 15.89 0 0 1 198.21 160 H 152 A 32 32 0 0 0 120 192 A 16 16 0 0 1 98.69 207.07 C 62.49 194.3 40 164 40 128 A 88 88 0 0 1 127.09 40 H 127.99 A 88.35 88.35 0 0 1 215.99 127.25 A 89 89 0 0 1 213.81 147.6 Z M 140 76 A 12 12 0 1 1 128 64 A 12 12 0 0 1 140 76 M 96 100 A 12 12 0 1 1 84 88 A 12 12 0 0 1 96 100 M 96 156 A 12 12 0 1 1 84 144 A 12 12 0 0 1 96 156 M 184 100 A 12 12 0 1 1 172 88 A 12 12 0 0 1 184 100", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_0: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 236 128 A 12 12 0 0 1 224 140 H 109 L 160.52 191.51 A 12 12 0 0 1 143.52 208.51 L 71.52 136.51 A 12 12 0 0 1 71.52 119.51 L 143.52 47.51 A 12 12 0 0 1 160.52 64.51 L 109 116 H 224 A 12 12 0 0 1 236 128 M 40 28 A 12 12 0 0 0 28 40 V 216 A 12 12 0 0 0 52 216 V 40 A 12 12 0 0 0 40 28", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_1: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 184.49 119.51 A 12 12 0 0 1 184.49 136.51 L 112.49 208.51 A 12 12 0 0 1 95.49 191.51 L 147 140 H 32 A 12 12 0 0 1 32 116 H 147 L 95.51 64.49 A 12 12 0 0 1 112.51 47.49 Z M 216 28 A 12 12 0 0 0 204 40 V 216 A 12 12 0 0 0 228 216 V 40 A 12 12 0 0 0 216 28", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_2: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 208.49 143.51 A 12 12 0 0 1 191.49 160.51 L 140 109 V 224 A 12 12 0 0 1 116 224 V 109 L 64.49 160.49 A 12 12 0 0 1 47.49 143.49 L 119.49 71.49 A 12 12 0 0 1 136.49 71.49 Z M 216 28 H 40 A 12 12 0 0 0 40 52 H 216 A 12 12 0 0 0 216 28", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_3: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 232.49 215.51 L 185 168 A 92.12 92.12 0 1 0 168 185 L 215.53 232.54 A 12 12 0 0 0 232.53 215.54 Z M 44 112 A 68 68 0 1 1 112 180 A 68.07 68.07 0 0 1 44 112", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_4: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 230.14 70.54 L 185.46 25.85 A 20 20 0 0 0 157.17 25.85 L 33.86 149.17 A 19.85 19.85 0 0 0 28 163.31 V 208 A 20 20 0 0 0 48 228 H 92.69 A 19.86 19.86 0 0 0 106.83 222.14 L 230.14 98.82 A 20 20 0 0 0 230.14 70.54 M 93 180 L 164 109 L 175 120 L 104 191 Z M 76 163 L 65 152 L 136 81 L 147 92 Z M 52 173 L 67.51 188.51 L 83 204 H 52 Z M 192 103 L 153 64 L 171.34 45.66 L 210.34 84.66 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_5: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 220 112 V 208 A 20 20 0 0 1 200 228 H 56 A 20 20 0 0 1 36 208 V 112 A 20 20 0 0 1 56 92 H 76 A 12 12 0 0 1 76 116 H 60 V 204 H 196 V 116 H 180 A 12 12 0 0 1 180 92 H 200 A 20 20 0 0 1 220 112 M 96.49 72.49 L 116 53 V 136 A 12 12 0 0 0 140 136 V 53 L 159.51 72.52 A 12 12 0 1 0 176.51 55.52 L 136.51 15.52 A 12 12 0 0 0 119.51 15.52 L 79.51 55.52 A 12 12 0 1 0 96.51 72.52 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_6: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 28 H 88 A 12 12 0 0 0 76 40 V 76 H 40 A 12 12 0 0 0 28 88 V 216 A 12 12 0 0 0 40 228 H 168 A 12 12 0 0 0 180 216 V 180 H 216 A 12 12 0 0 0 228 168 V 40 A 12 12 0 0 0 216 28 M 156 204 H 52 V 100 H 156 Z M 204 156 H 180 V 88 A 12 12 0 0 0 168 76 H 100 V 52 H 204 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_7: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 48 H 180 V 36 A 28 28 0 0 0 152 8 H 104 A 28 28 0 0 0 76 36 V 48 H 40 A 12 12 0 0 0 40 72 H 44 V 208 A 20 20 0 0 0 64 228 H 192 A 20 20 0 0 0 212 208 V 72 H 216 A 12 12 0 0 0 216 48 M 100 36 A 4 4 0 0 1 104 32 H 152 A 4 4 0 0 1 156 36 V 48 H 100 Z M 188 204 H 68 V 72 H 188 Z M 116 104 V 168 A 12 12 0 0 1 92 168 V 104 A 12 12 0 0 1 116 104 M 164 104 V 168 A 12 12 0 0 1 140 168 V 104 A 12 12 0 0 1 164 104", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_8: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 108 84 A 16 16 0 1 1 124 100 A 16 16 0 0 1 108 84 M 236 128 A 108 108 0 1 1 128 20 A 108.12 108.12 0 0 1 236 128 M 212 128 A 84 84 0 1 0 128 212 A 84.09 84.09 0 0 0 212 128 M 140 164.68 V 132 A 20 20 0 0 0 120 112 A 12 12 0 0 0 116 135.32 V 168 A 20 20 0 0 0 136 188 A 12 12 0 0 0 140 164.68", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_BOLD_9: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 203.57 51 A 107.9 107.9 0 0 0 20 128 C 20 172.72 47.6 210.25 92 225.94 A 36 36 0 0 0 140 192 A 12 12 0 0 1 152 180 H 198.21 A 35.79 35.79 0 0 0 233.31 152 A 108.6 108.6 0 0 0 236 127.09 A 107.23 107.23 0 0 0 203.57 51 M 209.91 146.67 A 11.91 11.91 0 0 1 198.21 155.97 H 152 A 36 36 0 0 0 116 191.97 A 12 12 0 0 1 100 203.27 C 83.35 197.39 69.35 187.51 59.52 174.71 A 76 76 0 0 1 44 128 A 84 84 0 0 1 127.13 44 H 128 A 84.35 84.35 0 0 1 212 127.29 A 84.7 84.7 0 0 1 209.91 146.71 Z M 144 76 A 16 16 0 1 1 128 60 A 16 16 0 0 1 144 76 M 100 100 A 16 16 0 1 1 84 84 A 16 16 0 0 1 100 100 M 100 156 A 16 16 0 1 1 84 140 A 16 16 0 0 1 100 156 M 188 100 A 16 16 0 1 1 172 84 A 16 16 0 0 1 188 100", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_0: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 48 40 V 216 A 8 8 0 0 1 32 216 V 40 A 8 8 0 0 1 48 40 M 224 120 H 152 V 56 A 8 8 0 0 0 138.34 50.34 L 66.34 122.34 A 8 8 0 0 0 66.34 133.66 L 138.34 205.66 A 8 8 0 0 0 152 200 V 136 H 224 A 8 8 0 0 0 224 120", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_1: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 224 40 V 216 A 8 8 0 0 1 208 216 V 40 A 8 8 0 0 1 224 40 M 117.66 50.34 A 8 8 0 0 0 104 56 V 120 H 32 A 8 8 0 0 0 32 136 H 104 V 200 A 8 8 0 0 0 117.66 205.66 L 189.66 133.66 A 8 8 0 0 0 189.66 122.34 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_2: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 205.66 138.34 A 8 8 0 0 1 200 152 H 136 V 224 A 8 8 0 0 1 120 224 V 152 H 56 A 8 8 0 0 1 50.34 138.34 L 122.34 66.34 A 8 8 0 0 1 133.66 66.34 Z M 216 32 H 40 A 8 8 0 0 0 40 48 H 216 A 8 8 0 0 0 216 32", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_3: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 168 112 A 56 56 0 1 1 112 56 A 56 56 0 0 1 168 112 M 229.66 229.66 A 8 8 0 0 1 218.34 229.66 L 168.28 179.59 A 88 88 0 1 1 179.6 168.28 L 229.66 218.34 A 8 8 0 0 1 229.66 229.66 M 112 184 A 72 72 0 1 0 40 112 A 72.08 72.08 0 0 0 112 184", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_4: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 227.31 73.37 L 182.63 28.68 A 16 16 0 0 0 160 28.68 L 36.69 152 A 15.86 15.86 0 0 0 32 163.31 V 208 A 16 16 0 0 0 48 224 H 92.69 A 15.86 15.86 0 0 0 104 219.31 L 227.31 96 A 16 16 0 0 0 227.31 73.37 M 51.31 160 L 141.66 69.65 L 158.34 86.34 L 68 176.68 Z M 48 179.31 L 76.69 208 H 48 Z M 96 204.69 L 79.31 188 L 169.66 97.65 L 186.34 114.34 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_5: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 112 V 208 A 16 16 0 0 1 200 224 H 56 A 16 16 0 0 1 40 208 V 112 A 16 16 0 0 1 56 96 H 120 V 144 A 8 8 0 0 0 136 144 V 96 H 200 A 16 16 0 0 1 216 112 M 136 43.31 L 162.34 69.66 A 8 8 0 0 0 173.66 58.34 L 133.66 18.34 A 8 8 0 0 0 122.34 18.34 L 82.34 58.34 A 8 8 0 0 0 93.66 69.66 L 120 43.31 V 96 H 136 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_6: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 32 H 88 A 8 8 0 0 0 80 40 V 80 H 40 A 8 8 0 0 0 32 88 V 216 A 8 8 0 0 0 40 224 H 168 A 8 8 0 0 0 176 216 V 176 H 216 A 8 8 0 0 0 224 168 V 40 A 8 8 0 0 0 216 32 M 208 160 H 176 V 88 A 8 8 0 0 0 168 80 H 96 V 48 H 208 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_7: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 48 H 176 V 40 A 24 24 0 0 0 152 16 H 104 A 24 24 0 0 0 80 40 V 48 H 40 A 8 8 0 0 0 40 64 H 48 V 208 A 16 16 0 0 0 64 224 H 192 A 16 16 0 0 0 208 208 V 64 H 216 A 8 8 0 0 0 216 48 M 112 168 A 8 8 0 0 1 96 168 V 104 A 8 8 0 0 1 112 104 Z M 160 168 A 8 8 0 0 1 144 168 V 104 A 8 8 0 0 1 160 104 Z M 160 48 H 96 V 40 A 8 8 0 0 1 104 32 H 152 A 8 8 0 0 1 160 40 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_8: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 128 24 A 104 104 0 1 0 232 128 A 104.11 104.11 0 0 0 128 24 M 124 72 A 12 12 0 1 1 112 84 A 12 12 0 0 1 124 72 M 136 184 A 16 16 0 0 1 120 168 V 128 A 8 8 0 0 1 120 112 A 16 16 0 0 1 136 128 V 168 A 8 8 0 0 1 136 184", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_FILL_9: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 200.77 53.89 A 103.27 103.27 0 0 0 128 24 H 126.93 A 104 104 0 0 0 24 128 C 24 171 50.58 207.06 93.36 222.17 A 32 32 0 0 0 136 192 A 16 16 0 0 1 152 176 H 198.21 A 31.81 31.81 0 0 0 229.41 151.12 A 104.4 104.4 0 0 0 232 127.12 A 103.28 103.28 0 0 0 200.77 53.89 M 84 168 A 12 12 0 1 1 96 156 A 12 12 0 0 1 84 168 M 84 112 A 12 12 0 1 1 96 100 A 12 12 0 0 1 84 112 M 128 88 A 12 12 0 1 1 140 76 A 12 12 0 0 1 128 88 M 172 112 A 12 12 0 1 1 184 100 A 12 12 0 0 1 172 112", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_0: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 144 56 V 200 L 72 128 Z", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 224 120 H 152 V 56 A 8 8 0 0 0 138.34 50.34 L 66.34 122.34 A 8 8 0 0 0 66.34 133.66 L 138.34 205.66 A 8 8 0 0 0 152 200 V 136 H 224 A 8 8 0 0 0 224 120 M 136 180.69 L 83.31 128 L 136 75.31 Z M 48 40 V 216 A 8 8 0 0 1 32 216 V 40 A 8 8 0 0 1 48 40", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_1: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 184 128 L 112 200 V 56 Z", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 117.66 50.34 A 8 8 0 0 0 104 56 V 120 H 32 A 8 8 0 0 0 32 136 H 104 V 200 A 8 8 0 0 0 117.66 205.66 L 189.66 133.66 A 8 8 0 0 0 189.66 122.34 Z M 120 180.69 V 75.31 L 172.69 128 Z M 224 40 V 216 A 8 8 0 0 1 208 216 V 40 A 8 8 0 0 1 224 40", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_2: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 200 144 H 56 L 128 72 Z", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 133.66 66.34 A 8 8 0 0 0 122.34 66.34 L 50.34 138.34 A 8 8 0 0 0 56 152 H 120 V 224 A 8 8 0 0 0 136 224 V 152 H 200 A 8 8 0 0 0 205.66 138.34 Z M 75.31 136 L 128 83.31 L 180.69 136 Z M 224 40 A 8 8 0 0 1 216 48 H 40 A 8 8 0 0 1 40 32 H 216 A 8 8 0 0 1 224 40", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_3: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 192 112 A 80 80 0 1 1 112 32 A 80 80 0 0 1 192 112", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 229.66 218.34 L 179.6 168.28 A 88.21 88.21 0 1 0 168.28 179.59 L 218.34 229.66 A 8 8 0 0 0 229.66 218.34 M 40 112 A 72 72 0 1 1 112 184 A 72.08 72.08 0 0 1 40 112", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_4: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 221.66 90.34 L 192 120 L 136 64 L 165.66 34.34 A 8 8 0 0 1 176.97 34.34 L 221.66 79 A 8 8 0 0 1 221.66 90.34", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 227.31 73.37 L 182.63 28.68 A 16 16 0 0 0 160 28.68 L 36.69 152 A 15.86 15.86 0 0 0 32 163.31 V 208 A 16 16 0 0 0 48 224 H 92.69 A 15.86 15.86 0 0 0 104 219.31 L 227.31 96 A 16 16 0 0 0 227.31 73.37 M 51.31 160 L 136 75.31 L 152.69 92 L 68 176.68 Z M 48 179.31 L 76.69 208 H 48 Z M 96 204.69 L 79.31 188 L 164 103.31 L 180.69 120 Z M 192 108.69 L 147.31 64 L 171.31 40 L 216 84.68 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_5: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 208 104 V 216 H 48 V 104 Z", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 216 112 V 208 A 16 16 0 0 1 200 224 H 56 A 16 16 0 0 1 40 208 V 112 A 16 16 0 0 1 56 96 H 80 A 8 8 0 0 1 80 112 H 56 V 208 H 200 V 112 H 176 A 8 8 0 0 1 176 96 H 200 A 16 16 0 0 1 216 112 M 93.66 69.66 L 120 43.31 V 136 A 8 8 0 0 0 136 136 V 43.31 L 162.34 69.66 A 8 8 0 0 0 173.66 58.34 L 133.66 18.34 A 8 8 0 0 0 122.34 18.34 L 82.34 58.34 A 8 8 0 0 0 93.66 69.66", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_6: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 40 V 168 H 168 V 88 H 88 V 40 Z", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 216 32 H 88 A 8 8 0 0 0 80 40 V 80 H 40 A 8 8 0 0 0 32 88 V 216 A 8 8 0 0 0 40 224 H 168 A 8 8 0 0 0 176 216 V 176 H 216 A 8 8 0 0 0 224 168 V 40 A 8 8 0 0 0 216 32 M 160 208 H 48 V 96 H 160 Z M 208 160 H 176 V 88 A 8 8 0 0 0 168 80 H 96 V 48 H 208 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_7: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 200 56 V 208 A 8 8 0 0 1 192 216 H 64 A 8 8 0 0 1 56 208 V 56 Z", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 216 48 H 176 V 40 A 24 24 0 0 0 152 16 H 104 A 24 24 0 0 0 80 40 V 48 H 40 A 8 8 0 0 0 40 64 H 48 V 208 A 16 16 0 0 0 64 224 H 192 A 16 16 0 0 0 208 208 V 64 H 216 A 8 8 0 0 0 216 48 M 96 40 A 8 8 0 0 1 104 32 H 152 A 8 8 0 0 1 160 40 V 48 H 96 Z M 192 208 H 64 V 64 H 192 Z M 112 104 V 168 A 8 8 0 0 1 96 168 V 104 A 8 8 0 0 1 112 104 M 160 104 V 168 A 8 8 0 0 1 144 168 V 104 A 8 8 0 0 1 160 104", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_8: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 224 128 A 96 96 0 1 1 128 32 A 96 96 0 0 1 224 128", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 144 176 A 8 8 0 0 1 136 184 A 16 16 0 0 1 120 168 V 128 A 8 8 0 0 1 120 112 A 16 16 0 0 1 136 128 V 168 A 8 8 0 0 1 144 176 M 232 128 A 104 104 0 1 1 128 24 A 104.11 104.11 0 0 1 232 128 M 216 128 A 88 88 0 1 0 128 216 A 88.1 88.1 0 0 0 216 128 M 124 96 A 12 12 0 1 0 112 84 A 12 12 0 0 0 124 96", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_DUOTONE_9: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 224 127.17 A 96.5 96.5 0 0 1 221.61 149.35 A 24 24 0 0 1 198.21 168 H 152 A 24 24 0 0 0 128 192 A 24 24 0 0 1 96 214.61 C 58.73 201.44 32 169.81 32 128 A 96 96 0 0 1 127 32 C 179.84 31.47 223.55 74.35 224 127.17", fill: true, even: false, alpha: 0.2, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 200.77 53.89 A 103.27 103.27 0 0 0 128 24 H 126.93 A 104 104 0 0 0 24 128 C 24 171 50.58 207.06 93.36 222.17 A 32 32 0 0 0 136 192 A 16 16 0 0 1 152 176 H 198.21 A 31.81 31.81 0 0 0 229.41 151.12 A 104.4 104.4 0 0 0 232 127.12 A 103.28 103.28 0 0 0 200.77 53.89 M 213.77 147.6 A 15.89 15.89 0 0 1 198.21 160 H 152 A 32 32 0 0 0 120 192 A 16 16 0 0 1 98.69 207.07 C 62.49 194.3 40 164 40 128 A 88 88 0 0 1 127.09 40 H 127.99 A 88.35 88.35 0 0 1 215.99 127.25 A 89 89 0 0 1 213.81 147.6 Z M 140 76 A 12 12 0 1 1 128 64 A 12 12 0 0 1 140 76 M 96 100 A 12 12 0 1 1 84 88 A 12 12 0 0 1 96 100 M 96 156 A 12 12 0 1 1 84 144 A 12 12 0 0 1 96 156 M 184 100 A 12 12 0 1 1 172 88 A 12 12 0 0 1 184 100", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_0: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 230 128 A 6 6 0 0 1 224 134 H 86.49 L 148.24 195.76 A 6 6 0 1 1 139.76 204.24 L 67.76 132.24 A 6 6 0 0 1 67.76 123.76 L 139.76 51.76 A 6 6 0 0 1 148.24 60.24 L 86.49 122 H 224 A 6 6 0 0 1 230 128 M 40 34 A 6 6 0 0 0 34 40 V 216 A 6 6 0 0 0 46 216 V 40 A 6 6 0 0 0 40 34", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_1: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 188.24 123.76 A 6 6 0 0 1 188.24 132.24 L 116.24 204.24 A 6 6 0 0 1 107.76 195.76 L 169.51 134 H 32 A 6 6 0 0 1 32 122 H 169.51 L 107.76 60.24 A 6 6 0 0 1 116.24 51.76 Z M 216 34 A 6 6 0 0 0 210 40 V 216 A 6 6 0 0 0 222 216 V 40 A 6 6 0 0 0 216 34", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_2: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 204.24 139.76 A 6 6 0 1 1 195.76 148.24 L 134 86.49 V 224 A 6 6 0 0 1 122 224 V 86.49 L 60.24 148.24 A 6 6 0 0 1 51.76 139.76 L 123.76 67.76 A 6 6 0 0 1 132.24 67.76 Z M 216 34 H 40 A 6 6 0 0 0 40 46 H 216 A 6 6 0 0 0 216 34", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_3: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 228.24 219.76 L 176.86 168.38 A 86.15 86.15 0 1 0 168.38 176.86 L 219.76 228.24 A 6 6 0 0 0 228.24 219.76 M 38 112 A 74 74 0 1 1 112 186 A 74.09 74.09 0 0 1 38 112", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_4: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 225.9 74.78 L 181.21 30.09 A 14 14 0 0 0 161.41 30.09 L 38.1 153.41 A 13.94 13.94 0 0 0 34 163.31 V 208 A 14 14 0 0 0 48 222 H 92.69 A 13.94 13.94 0 0 0 102.59 217.9 L 225.9 94.58 A 14 14 0 0 0 225.9 74.78 M 48.49 160 L 136 72.48 L 155.51 92 L 68 179.51 Z M 46 208 V 174.48 L 81.51 210 H 48 A 2 2 0 0 1 46 208 M 96 207.51 L 76.49 188 L 164 100.48 L 183.51 120 Z M 217.41 86.1 L 192 111.51 L 144.49 64 L 169.9 38.58 A 2 2 0 0 1 172.73 38.58 L 217.41 83.27 A 2 2 0 0 1 217.41 86.1", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_5: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 214 112 V 208 A 14 14 0 0 1 200 222 H 56 A 14 14 0 0 1 42 208 V 112 A 14 14 0 0 1 56 98 H 80 A 6 6 0 0 1 80 110 H 56 A 2 2 0 0 0 54 112 V 208 A 2 2 0 0 0 56 210 H 200 A 2 2 0 0 0 202 208 V 112 A 2 2 0 0 0 200 110 H 176 A 6 6 0 0 1 176 98 H 200 A 14 14 0 0 1 214 112 M 92.24 68.24 L 122 38.49 V 136 A 6 6 0 0 0 134 136 V 38.49 L 163.76 68.24 A 6 6 0 1 0 172.24 59.76 L 132.24 19.76 A 6 6 0 0 0 123.76 19.76 L 83.76 59.76 A 6 6 0 1 0 92.24 68.24", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_6: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 34 H 88 A 6 6 0 0 0 82 40 V 82 H 40 A 6 6 0 0 0 34 88 V 216 A 6 6 0 0 0 40 222 H 168 A 6 6 0 0 0 174 216 V 174 H 216 A 6 6 0 0 0 222 168 V 40 A 6 6 0 0 0 216 34 M 162 210 H 46 V 94 H 162 Z M 210 162 H 174 V 88 A 6 6 0 0 0 168 82 H 94 V 46 H 210 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_7: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 216 50 H 174 V 40 A 22 22 0 0 0 152 18 H 104 A 22 22 0 0 0 82 40 V 50 H 40 A 6 6 0 0 0 40 62 H 50 V 208 A 14 14 0 0 0 64 222 H 192 A 14 14 0 0 0 206 208 V 62 H 216 A 6 6 0 0 0 216 50 M 94 40 A 10 10 0 0 1 104 30 H 152 A 10 10 0 0 1 162 40 V 50 H 94 Z M 194 208 A 2 2 0 0 1 192 210 H 64 A 2 2 0 0 1 62 208 V 62 H 194 Z M 110 104 V 168 A 6 6 0 0 1 98 168 V 104 A 6 6 0 0 1 110 104 M 158 104 V 168 A 6 6 0 0 1 146 168 V 104 A 6 6 0 0 1 158 104", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_8: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 142 176 A 6 6 0 0 1 136 182 A 14 14 0 0 1 122 168 V 128 A 2 2 0 0 0 120 126 A 6 6 0 0 1 120 114 A 14 14 0 0 1 134 128 V 168 A 2 2 0 0 0 136 170 A 6 6 0 0 1 142 176 M 124 94 A 10 10 0 1 0 114 84 A 10 10 0 0 0 124 94 M 230 128 A 102 102 0 1 1 128 26 A 102.12 102.12 0 0 1 230 128 M 218 128 A 90 90 0 1 0 128 218 A 90.1 90.1 0 0 0 218 128", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PH_LIGHT_9: Glyph = Glyph {
    grid: 256.0,
    parts: &[
        Part { d: "M 199.37 55.31 A 101.32 101.32 0 0 0 128 26 H 127 A 102 102 0 0 0 26 128 C 26 170.09 52.07 205.44 94 220.26 A 30.2 30.2 0 0 0 104.11 222 A 30.06 30.06 0 0 0 134 192 A 18 18 0 0 1 152 174 H 198.21 A 29.82 29.82 0 0 0 227.46 150.69 A 102.7 102.7 0 0 0 230 127.11 A 101.25 101.25 0 0 0 199.37 55.31 M 215.76 148 A 17.89 17.89 0 0 1 198.21 162 H 152 A 30 30 0 0 0 122 192 A 18 18 0 0 1 98 209 C 61 195.86 38 164.85 38 128 A 90 90 0 0 1 127.07 38 H 128 A 90.34 90.34 0 0 1 218 127.22 A 90.5 90.5 0 0 1 215.76 148 M 138 76 A 10 10 0 1 1 128 66 A 10 10 0 0 1 138 76 M 94 100 A 10 10 0 1 1 84 90 A 10 10 0 0 1 94 100 M 94 156 A 10 10 0 1 1 84 146 A 10 10 0 0 1 94 156 M 182 100 A 10 10 0 1 1 172 90 A 10 10 0 0 1 182 100", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 12 H 3 M 3 12 L 11.5 3.5 M 3 12 L 11.5 20.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 12 H 21 M 21 12 L 12.5 3.5 M 21 12 L 12.5 20.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 21 V 3 M 12 3 L 20.5 11.5 M 12 3 L 3.5 11.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17 17 L 21 21 M 3 11 A 8 8 0 1 0 19 11 A 8 8 0 0 0 3 11", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 21 H 21 M 12.22 5.83 L 15.05 3 L 20 7.95 L 17.17 10.78 M 12.22 5.83 L 6.62 11.44 A 1 1 0 0 0 6.32 12.14 V 16.68 H 10.86 A 1 1 0 0 0 11.57 16.39 L 17.17 10.78 M 12.22 5.83 L 17.17 10.78", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6 19 L 19 6 M 19 6 V 18.48 M 19 6 H 6.52", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19.4 20 H 9.6 A 0.6 0.6 0 0 1 9 19.4 V 9.6 A 0.6 0.6 0 0 1 9.6 9 H 19.4 A 0.6 0.6 0 0 1 20 9.6 V 19.4 A 0.6 0.6 0 0 1 19.4 20", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 15 9 V 4.6 A 0.6 0.6 0 0 0 14.4 4 H 4.6 A 0.6 0.6 0 0 0 4 4.6 V 14.4 A 0.6 0.6 0 0 0 4.6 15 H 9", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 9 L 18 20.35 A 2 2 0 0 1 16.04 22 H 7.96 A 2 2 0 0 1 6 20.35 L 4 9 M 21 6 H 15.38 M 3 6 H 8.62 M 8.62 6 V 4 A 2 2 0 0 1 10.62 2 H 13.38 A 2 2 0 0 1 15.38 4 V 6 M 8.62 6 H 15.38", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 11.5 V 16.5 M 12 7.51 L 12.01 7.5 M 12 22 C 17.52 22 22 17.52 22 12 S 17.52 2 12 2 S 2 6.48 2 12 S 6.48 22 12 22", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONOIR_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20.51 9.54 A 1.9 1.9 0 0 1 19.51 10.63 A 7 7 0 0 0 15.37 17 C 15.37 17.47 15.42 17.94 15.51 18.4 A 2.16 2.16 0 0 1 15.2 20.05 A 1.8 1.8 0 0 1 13.99 20.85 C 13.45 20.95 12.91 21 12.37 21 A 9 9 0 0 1 3.37 11.72 A 9.05 9.05 0 0 1 11.85 3 H 12.36 A 9 9 0 0 1 20.42 8 A 2 2 0 0 1 20.51 9.52 z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 8 16.01 L 8.01 16 M 6 12.01 L 6.01 12 M 8 8.01 L 8.01 8 M 12 6.01 L 12.01 6 M 16 8.01 L 16.01 8", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18.75 4.5 L 11.25 12 L 18.75 19.5 M 12.75 4.5 L 5.25 12 L 12.75 19.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.25 4.5 L 12.75 12 L 5.25 19.5 M 11.25 4.5 L 18.75 12 L 11.25 19.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4.5 10.5 L 12 3 M 12 3 L 19.5 10.5 M 12 3 V 21", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 21 L 15.8 15.8 M 15.8 15.8 A 7.5 7.5 0 1 0 5.2 5.2 A 7.5 7.5 0 0 0 15.8 15.8", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16.86 4.49 L 18.55 2.8 A 1.88 1.88 0 1 1 21.2 5.45 L 6.83 19.82 A 4.5 4.5 0 0 1 4.93 20.95 L 2.25 21.75 L 3.05 19.07 A 4.5 4.5 0 0 1 4.18 17.17 z M 16.86 4.49 L 19.5 7.12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9 13.5 L 12 16.5 M 12 16.5 L 15 13.5 M 12 16.5 V 10.5 M 13.06 6.31 L 10.94 4.19 A 1.5 1.5 0 0 0 9.88 3.75 H 4.5 A 2.25 2.25 0 0 0 2.25 6 V 18 A 2.25 2.25 0 0 0 4.5 20.25 H 19.5 A 2.25 2.25 0 0 0 21.75 18 V 9 A 2.25 2.25 0 0 0 19.5 6.75 H 14.12 A 1.5 1.5 0 0 1 13.06 6.31", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.67 3.89 A 2.25 2.25 0 0 0 13.5 2.25 H 10.5 C 9.47 2.25 8.6 2.94 8.33 3.89 M 15.67 3.89 C 15.72 4.08 15.75 4.29 15.75 4.5 V 4.5 A 0.75 0.75 0 0 1 15 5.25 H 9 A 0.75 0.75 0 0 1 8.25 4.5 V 4.5 C 8.25 4.29 8.28 4.08 8.33 3.89 M 15.67 3.89 C 16.31 3.94 16.95 4 17.59 4.07 C 18.69 4.2 19.5 5.15 19.5 6.26 V 19.5 A 2.25 2.25 0 0 1 17.25 21.75 H 6.75 A 2.25 2.25 0 0 1 4.5 19.5 V 6.26 C 4.5 5.15 5.31 4.2 6.41 4.07 A 48 48 0 0 1 8.33 3.89", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14.74 9 L 14.39 18 M 9.61 18 L 9.26 9 M 19.23 5.79 C 19.57 5.84 19.91 5.9 20.25 5.96 M 19.23 5.79 L 18.16 19.67 A 2.25 2.25 0 0 1 15.92 21.75 H 8.08 A 2.25 2.25 0 0 1 5.84 19.67 L 4.77 5.79 M 19.23 5.79 A 48 48 0 0 0 15.75 5.39 M 3.75 5.96 C 4.09 5.9 4.43 5.84 4.77 5.79 M 4.77 5.79 A 48 48 0 0 1 8.25 5.39 M 15.75 5.39 V 4.48 C 15.75 3.3 14.84 2.31 13.66 2.28 A 52 52 0 0 0 10.34 2.28 C 9.16 2.31 8.25 3.3 8.25 4.48 V 5.39 M 15.75 5.39 A 49 49 0 0 0 8.25 5.39", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.25 11.25 L 11.29 11.23 A 0.75 0.75 0 0 1 12.35 12.08 L 11.65 14.92 A 0.75 0.75 0 0 0 12.71 15.77 L 12.75 15.75 M 21 12 A 9 9 0 1 1 3 12 A 9 9 0 0 1 21 12 M 12 8.25 H 12.01 V 8.26 H 12 z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_OUTLINE_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9.53 16.12 A 3 3 0 0 0 3.75 17.25 A 2.25 2.25 0 0 1 1.35 19.5 A 4.5 4.5 0 0 0 9.75 17.25 C 9.75 16.85 9.67 16.47 9.53 16.12 M 9.53 16.12 A 16 16 0 0 0 12.92 14.5 M 7.87 14.48 A 16 16 0 0 1 9.5 11.08 M 12.92 14.5 A 16 16 0 0 0 17.68 9.85 L 21.56 4.04 A 1.15 1.15 0 0 0 19.96 2.44 L 14.15 6.32 A 16 16 0 0 0 9.5 11.08 M 12.92 14.5 A 6.78 6.78 0 0 0 9.5 11.08", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10.72 11.47 A 0.75 0.75 0 0 0 10.72 12.53 L 18.22 20.03 A 0.75 0.75 0 1 0 19.28 18.97 L 12.31 12 L 19.28 5.03 A 0.75 0.75 0 0 0 18.22 3.97 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4.72 11.47 A 0.75 0.75 0 0 0 4.72 12.53 L 12.22 20.03 A 0.75 0.75 0 1 0 13.28 18.97 L 6.31 12 L 13.28 5.03 A 0.75 0.75 0 0 0 12.22 3.97 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13.28 11.47 A 0.75 0.75 0 0 1 13.28 12.53 L 5.78 20.03 A 0.75 0.75 0 0 1 4.72 18.97 L 11.69 12 L 4.72 5.03 A 0.75 0.75 0 0 1 5.78 3.97 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 19.28 11.47 A 0.75 0.75 0 0 1 19.28 12.53 L 11.78 20.03 A 0.75 0.75 0 1 1 10.72 18.97 L 17.69 12 L 10.72 5.03 A 0.75 0.75 0 0 1 11.78 3.97 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.47 2.47 A 0.75 0.75 0 0 1 12.53 2.47 L 20.03 9.97 A 0.75 0.75 0 1 1 18.97 11.03 L 12.75 4.81 V 21 A 0.75 0.75 0 0 1 11.25 21 V 4.81 L 5.03 11.03 A 0.75 0.75 0 1 1 3.97 9.97 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10.5 3.75 A 6.75 6.75 0 1 0 10.5 17.25 A 6.75 6.75 0 0 0 10.5 3.75 M 2.25 10.5 A 8.25 8.25 0 1 1 16.84 15.78 L 21.53 20.47 A 0.75 0.75 0 1 1 20.47 21.53 L 15.78 16.84 A 8.25 8.25 0 0 1 2.25 10.5", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21.73 2.27 A 2.62 2.62 0 0 0 18.02 2.27 L 16.86 3.43 L 20.57 7.14 L 21.73 5.98 A 2.62 2.62 0 0 0 21.73 2.27 M 19.51 8.2 L 15.8 4.49 L 3.65 16.64 A 5.25 5.25 0 0 0 2.33 18.85 L 1.53 21.54 A 0.75 0.75 0 0 0 2.46 22.47 L 5.15 21.67 A 5.25 5.25 0 0 0 7.36 20.35 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19.5 21 A 3 3 0 0 0 22.5 18 V 9 A 3 3 0 0 0 19.5 6 H 14.12 A 0.75 0.75 0 0 1 13.59 5.78 L 11.47 3.66 A 2.25 2.25 0 0 0 9.88 3 H 4.5 A 3 3 0 0 0 1.5 6 V 18 A 3 3 0 0 0 4.5 21 z M 12.75 10.5 A 0.75 0.75 0 0 0 11.25 10.5 V 14.69 L 9.53 12.97 A 0.75 0.75 0 0 0 8.47 14.03 L 11.47 17.03 A 0.75 0.75 0 0 0 12.53 17.03 L 15.53 14.03 A 0.75 0.75 0 1 0 14.47 12.97 L 12.75 14.69 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10.5 3 A 1.5 1.5 0 0 0 9 4.5 H 15 C 15 4.36 14.98 4.22 14.94 4.09 A 1.5 1.5 0 0 0 13.5 3 z M 7.81 3.18 A 3 3 0 0 1 10.5 1.5 H 13.5 A 3 3 0 0 1 16.19 3.18 C 16.69 3.22 17.19 3.27 17.68 3.33 C 19.18 3.5 20.25 4.79 20.25 6.26 V 19.5 A 3 3 0 0 1 17.25 22.5 H 6.75 A 3 3 0 0 1 3.75 19.5 V 6.26 C 3.75 4.79 4.82 3.5 6.32 3.33 C 6.81 3.27 7.31 3.22 7.81 3.18", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16.5 4.48 V 4.71 A 49 49 0 0 1 20.38 5.22 A 0.75 0.75 0 1 1 20.12 6.7 L 19.91 6.66 L 18.91 19.73 A 3 3 0 0 1 15.92 22.5 H 8.08 A 3 3 0 0 1 5.09 19.73 L 4.09 6.66 L 3.88 6.7 A 0.75 0.75 0 0 1 3.62 5.22 A 49 49 0 0 1 7.5 4.71 V 4.48 C 7.5 2.91 8.71 1.58 10.32 1.53 A 53 53 0 0 1 13.68 1.53 C 15.29 1.58 16.5 2.91 16.5 4.48 M 10.36 3.03 A 51 51 0 0 1 13.64 3.03 C 14.39 3.05 15 3.68 15 4.48 V 4.59 A 50 50 0 0 0 9 4.59 V 4.48 C 9 3.68 9.61 3.05 10.36 3.03 M 10.01 8.97 A 0.75 0.75 0 1 0 8.51 9.03 L 8.86 18.03 A 0.75 0.75 0 1 0 10.36 17.97 z M 15.49 9.03 A 0.75 0.75 0 1 0 13.99 8.97 L 13.64 17.97 A 0.75 0.75 0 0 0 15.14 18.03 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2.25 12 C 2.25 6.62 6.62 2.25 12 2.25 S 21.75 6.62 21.75 12 S 17.38 21.75 12 21.75 S 2.25 17.39 2.25 12 M 10.96 10.56 C 12.1 9.98 13.39 11.02 13.08 12.26 L 12.37 15.1 L 12.41 15.08 A 0.75 0.75 0 0 1 13.08 16.42 L 13.04 16.44 C 11.9 17.02 10.61 15.98 10.92 14.74 L 11.63 11.9 L 11.59 11.92 A 0.75 0.75 0 1 1 10.92 10.58 z M 12 9 A 0.75 0.75 0 1 0 12 7.5 A 0.75 0.75 0 0 0 12 9", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HERO_SOLID_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20.6 1.5 C 20.22 1.5 19.86 1.61 19.54 1.82 L 14.46 5.21 A 18.8 18.8 0 0 0 10.99 8.19 A 10.04 10.04 0 0 1 15.81 13.01 A 18.8 18.8 0 0 0 18.8 9.54 L 22.18 4.46 A 1.9 1.9 0 0 0 20.6 1.5 M 12.3 15.53 A 19 19 0 0 0 14.2 14.32 A 8.03 8.03 0 0 0 9.68 9.8 A 19 19 0 0 0 8.47 11.7 L 8.2 12.2 A 5.26 5.26 0 0 1 11.8 15.8 z M 6.75 13.5 A 3.75 3.75 0 0 0 3 17.25 A 1.5 1.5 0 0 1 1.4 18.75 A 0.75 0.75 0 0 0 0.7 19.87 A 5.25 5.25 0 0 0 10.5 17.25 A 3.75 3.75 0 0 0 6.75 13.5", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10.73 19.79 A 0.75 0.75 0 0 0 11.77 18.7 L 5.52 12.75 H 20.25 A 0.75 0.75 0 0 0 20.25 11.25 H 5.52 L 11.77 5.29 A 0.75 0.75 0 0 0 10.73 4.21 L 3.31 11.28 A 1 1 0 0 0 3.01 11.86 A 0.8 0.8 0 0 0 3.01 12.14 A 1 1 0 0 0 3.31 12.72 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13.27 4.21 A 0.75 0.75 0 0 0 12.23 5.29 L 18.48 11.25 H 3.75 A 0.75 0.75 0 0 0 3.75 12.75 H 18.48 L 12.23 18.7 A 0.75 0.75 0 0 0 13.27 19.79 L 20.69 12.72 A 1 1 0 0 0 20.99 12.14 A 0.8 0.8 0 0 0 20.99 11.85 A 1 1 0 0 0 20.69 11.28 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4.21 10.73 A 0.75 0.75 0 0 0 5.29 11.77 L 11.25 5.52 V 20.25 A 0.75 0.75 0 0 0 12.75 20.25 V 5.52 L 18.7 11.77 A 0.75 0.75 0 0 0 19.79 10.73 L 12.72 3.31 A 1 1 0 0 0 12.14 3.01 A 0.8 0.8 0 0 0 11.85 3.01 A 1 1 0 0 0 11.28 3.31 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16.1 17.16 A 8 8 0 1 1 17.16 16.1 L 21.78 20.72 A 0.75 0.75 0 1 1 20.72 21.78 z M 17.5 11 A 6.5 6.5 0 1 0 4.5 11 A 6.5 6.5 0 0 0 17.5 11", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20.95 3.05 A 3.58 3.58 0 0 0 15.89 3.05 L 3.94 15 A 3.1 3.1 0 0 0 3.12 16.48 L 2.02 21.08 A 0.75 0.75 0 0 0 2.92 21.98 L 7.53 20.88 A 3.1 3.1 0 0 0 9 20.06 L 20.95 8.11 A 3.58 3.58 0 0 0 20.95 3.05 M 16.95 4.11 A 2.08 2.08 0 1 1 19.89 7.05 L 19 7.94 L 16.06 5 z M 15 6.06 L 17.94 9 L 7.94 19 C 7.73 19.21 7.47 19.36 7.18 19.43 L 3.76 20.24 L 4.57 16.82 C 4.64 16.53 4.79 16.27 5 16.06 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17.5 11 A 5.5 5.5 0 1 1 17.5 22 A 5.5 5.5 0 0 1 17.5 11 z M 8.21 4 C 8.67 4 9.12 4.14 9.49 4.4 L 9.65 4.52 L 12.02 6.5 H 19.75 A 2.25 2.25 0 0 1 21.98 8.44 L 21.99 8.6 L 22 8.75 V 11.81 A 6.52 6.52 0 0 0 20.5 10.73 L 20.5 8.75 A 0.75 0.75 0 0 0 19.85 8.01 L 19.75 8 H 12.02 L 9.65 9.98 A 2.25 2.25 0 0 1 8.4 10.49 L 8.21 10.5 L 3.5 10.5 V 17.75 C 3.5 18.13 3.78 18.44 4.15 18.49 L 4.25 18.5 H 11.31 A 6.47 6.47 0 0 0 12.02 20 L 4.25 20 A 2.25 2.25 0 0 1 2 17.9 L 2 17.75 V 6.25 A 2.25 2.25 0 0 1 4.1 4 L 4.25 4 H 8.21 z M 17.79 13.54 L 17.72 13.59 L 17.65 13.65 L 17.59 13.72 A 0.5 0.5 0 0 0 17.59 14.28 L 17.65 14.35 L 19.29 16 H 14 L 13.91 16.01 A 0.5 0.5 0 0 0 13.51 16.41 L 13.5 16.5 L 13.51 16.59 A 0.5 0.5 0 0 0 13.91 16.99 L 14 17 H 19.29 L 17.65 18.65 L 17.59 18.72 A 0.5 0.5 0 0 0 18.28 19.41 L 18.35 19.35 L 20.88 16.82 L 20.93 16.76 L 20.96 16.69 L 20.98 16.63 L 21 16.55 L 21 16.5 L 21 16.45 L 20.98 16.37 L 20.95 16.29 L 20.91 16.21 L 20.87 16.16 L 18.35 13.65 L 18.28 13.59 A 0.5 0.5 0 0 0 17.79 13.54 z M 8.21 5.5 H 4.25 A 0.75 0.75 0 0 0 3.51 6.15 L 3.5 6.25 V 9 L 8.21 9 A 0.75 0.75 0 0 0 8.6 8.89 L 8.69 8.83 L 10.58 7.25 L 8.69 5.67 A 0.75 0.75 0 0 0 8.31 5.51 L 8.21 5.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8.5 8.5 H 6.25 A 1.75 1.75 0 0 0 4.5 10.25 V 17.75 C 4.5 18.72 5.28 19.5 6.25 19.5 H 11.25 C 12.13 19.5 12.86 18.85 12.98 18 H 14.49 A 3.25 3.25 0 0 1 11.25 21 H 6.25 A 3.25 3.25 0 0 1 3 17.75 V 10.25 A 3.25 3.25 0 0 1 6.25 7 H 8.5 z M 17.75 3 A 3.25 3.25 0 0 1 21 6.25 V 13.75 A 3.25 3.25 0 0 1 17.75 17 H 12.75 A 3.25 3.25 0 0 1 9.5 13.75 V 6.25 A 3.25 3.25 0 0 1 12.75 3 z M 12.75 4.5 A 1.75 1.75 0 0 0 11 6.25 V 13.75 C 11 14.72 11.78 15.5 12.75 15.5 H 17.75 A 1.75 1.75 0 0 0 19.5 13.75 V 6.25 A 1.75 1.75 0 0 0 17.75 4.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 5 H 14 A 2 2 0 1 0 10 5 M 8.5 5 A 3.5 3.5 0 1 1 15.5 5 H 21.25 A 0.75 0.75 0 0 1 21.25 6.5 H 19.93 L 18.76 18.61 A 3.75 3.75 0 0 1 15.03 22 H 8.97 A 3.75 3.75 0 0 1 5.24 18.61 L 4.07 6.5 H 2.75 A 0.75 0.75 0 0 1 2.75 5 z M 10.5 9.75 A 0.75 0.75 0 0 0 9 9.75 V 17.25 A 0.75 0.75 0 0 0 10.5 17.25 z M 14.25 9 A 0.75 0.75 0 0 1 15 9.75 V 17.25 A 0.75 0.75 0 0 1 13.5 17.25 V 9.75 A 0.75 0.75 0 0 1 14.25 9 M 6.73 18.47 A 2.25 2.25 0 0 0 8.97 20.5 H 15.03 A 2.25 2.25 0 0 0 17.27 18.47 L 18.42 6.5 H 5.58 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 2 C 17.52 2 22 6.48 22 12 C 22 17.52 17.52 22 12 22 C 6.48 22 2 17.52 2 12 C 2 6.48 6.48 2 12 2 M 12 3.5 A 8.5 8.5 0 1 0 12 20.5 A 8.5 8.5 0 0 0 12 3.5 M 12 10.5 A 0.75 0.75 0 0 1 12.75 11.25 V 16.25 A 0.75 0.75 0 0 1 11.25 16.25 V 11.25 A 0.75 0.75 0 0 1 12 10.5 M 12 9 A 1 1 0 1 0 12 7 A 1 1 0 0 0 12 9", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_REGULAR_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.75 2 A 0.75 0.75 0 0 0 5 2.75 V 14.25 A 2.25 2.25 0 0 0 7.25 16.5 H 9.5 V 19.5 A 2.5 2.5 0 1 0 14.5 19.5 V 16.5 H 16.75 A 2.25 2.25 0 0 0 19 14.25 V 2.75 A 0.75 0.75 0 0 0 18.25 2 z M 6.5 11 V 3.5 H 12.5 V 5.25 A 0.75 0.75 0 1 0 14 5.25 V 3.5 H 15 V 6.25 A 0.75 0.75 0 1 0 16.5 6.25 V 3.5 H 17.5 V 11 z M 6.5 14.25 V 12.5 H 17.5 V 14.25 A 0.75 0.75 0 0 1 16.75 15 H 13.75 A 0.75 0.75 0 0 0 13 15.75 V 19.5 A 1 1 0 0 1 11 19.5 V 15.75 A 0.75 0.75 0 0 0 10.25 15 H 7.25 A 0.75 0.75 0 0 1 6.5 14.25", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10.29 19.71 A 1 1 0 0 0 11.7 18.29 L 6.33 13 H 20 A 1 1 0 0 0 20 11 H 6.34 L 11.7 5.71 A 1 1 0 0 0 10.3 4.29 L 3.37 11.11 A 1.25 1.25 0 0 0 3.37 12.89 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13.71 4.28 A 1 1 0 1 0 12.3 5.71 L 17.67 11 H 4 A 1 1 0 1 0 4 13 H 17.66 L 12.3 18.29 A 1 1 0 0 0 13.7 19.71 L 20.63 12.89 A 1.25 1.25 0 0 0 20.63 11.11 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4.28 10.29 A 1 1 0 0 0 5.71 11.7 L 11 6.33 V 20 A 1 1 0 1 0 13 20 V 6.33 L 18.29 11.7 A 1 1 0 0 0 19.71 10.29 L 12.89 3.37 A 1.25 1.25 0 0 0 11.11 3.37 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.84 17.37 A 8 8 0 1 1 17.27 15.97 L 21.7 20.28 A 1 1 0 1 1 20.3 21.72 z M 17 11 A 6 6 0 1 0 5 11 A 6 6 0 0 0 17 11", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.89 3.05 A 3.58 3.58 0 1 1 20.95 8.11 L 20.06 9 L 15 3.94 z M 13.94 5 L 3.94 15 A 3.1 3.1 0 0 0 3.12 16.48 L 2.02 21.08 A 0.75 0.75 0 0 0 2.92 21.98 L 7.53 20.88 A 3.1 3.1 0 0 0 9 20.06 L 19 10.06 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17.5 11 A 5.5 5.5 0 1 1 17.5 22 A 5.5 5.5 0 0 1 17.5 11 z M 19.75 6.5 A 2.25 2.25 0 0 1 21.98 8.44 L 21.99 8.6 L 22 8.75 V 11.81 A 6.5 6.5 0 0 0 12.02 20 H 4.25 A 2.25 2.25 0 0 1 2 17.9 L 2 17.75 V 10.5 L 8.21 10.5 L 8.4 10.49 A 2.25 2.25 0 0 0 9.49 10.1 L 9.65 9.98 L 13.82 6.5 H 19.75 z M 17.79 13.54 L 17.72 13.59 L 17.65 13.65 L 17.59 13.72 A 0.5 0.5 0 0 0 17.59 14.28 L 17.65 14.35 L 19.29 16 H 14 L 13.91 16.01 A 0.5 0.5 0 0 0 13.51 16.41 L 13.5 16.5 L 13.51 16.59 A 0.5 0.5 0 0 0 13.91 16.99 L 14 17 H 19.29 L 17.65 18.65 L 17.59 18.72 A 0.5 0.5 0 0 0 18.28 19.41 L 18.35 19.35 L 20.88 16.82 L 20.93 16.76 L 20.96 16.69 L 20.98 16.63 L 21 16.55 L 21 16.5 L 21 16.45 L 20.98 16.37 L 20.95 16.29 L 20.91 16.21 L 20.87 16.16 L 18.35 13.65 L 18.28 13.59 A 0.5 0.5 0 0 0 17.79 13.54 z M 8.21 4 C 8.67 4 9.12 4.14 9.49 4.4 L 9.65 4.52 L 11.75 6.27 L 8.69 8.83 L 8.6 8.89 A 0.75 0.75 0 0 1 8.31 8.99 L 8.21 9 L 2 9 V 6.25 A 2.25 2.25 0 0 1 4.1 4 L 4.25 4 H 8.21 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8.5 13.75 A 4.25 4.25 0 0 0 12.75 18 H 14.49 A 3.25 3.25 0 0 1 11.25 21 H 6.25 A 3.25 3.25 0 0 1 3 17.75 V 10.25 A 3.25 3.25 0 0 1 6.25 7 H 8.5 z M 17.75 3 A 3.25 3.25 0 0 1 21 6.25 V 13.75 A 3.25 3.25 0 0 1 17.75 17 H 12.75 A 3.25 3.25 0 0 1 9.5 13.75 V 6.25 A 3.25 3.25 0 0 1 12.75 3 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 5 H 14 A 2 2 0 1 0 10 5 M 8.5 5 A 3.5 3.5 0 1 1 15.5 5 H 21.25 A 0.75 0.75 0 0 1 21.25 6.5 H 19.93 L 18.76 18.61 A 3.75 3.75 0 0 1 15.03 22 H 8.97 A 3.75 3.75 0 0 1 5.24 18.61 L 4.07 6.5 H 2.75 A 0.75 0.75 0 0 1 2.75 5 z M 10.5 9.75 A 0.75 0.75 0 0 0 9 9.75 V 17.25 A 0.75 0.75 0 0 0 10.5 17.25 z M 14.25 9 A 0.75 0.75 0 0 0 13.5 9.75 V 17.25 A 0.75 0.75 0 0 0 15 17.25 V 9.75 A 0.75 0.75 0 0 0 14.25 9", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 2 C 17.52 2 22 6.48 22 12 C 22 17.52 17.52 22 12 22 C 6.48 22 2 17.52 2 12 C 2 6.48 6.48 2 12 2 M 12 10.5 A 0.75 0.75 0 0 0 11.25 11.25 V 16.25 A 0.75 0.75 0 0 0 12.75 16.25 V 11.25 A 0.75 0.75 0 0 0 12 10.5 M 12 9 A 1 1 0 1 0 12 7 A 1 1 0 0 0 12 9", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FLUENT_FILLED_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.5 2 V 5.25 A 0.75 0.75 0 0 0 14 5.25 V 2 H 15 V 6.25 A 0.75 0.75 0 0 0 16.5 6.25 V 2 H 18.25 A 0.75 0.75 0 0 1 19 2.75 V 11 H 5 V 2.75 A 0.75 0.75 0 0 1 5.75 2 z M 5 12.5 V 14.25 A 2.25 2.25 0 0 0 7.25 16.5 H 10 V 20 A 2 2 0 1 0 14 20 V 16.5 H 16.75 A 2.25 2.25 0 0 0 19 14.25 V 12.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 12 H 20 M 10 17.66 L 4.34 12 L 10 6.34", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 12 H 19 M 14 17.66 L 19.66 12 L 14 6.34", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 20 V 5 M 6.34 10 L 12 4.34 L 17.66 10", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.88 15.88 L 20.12 20.12 M 18 10.5 A 7.5 7.5 0 1 1 3 10.5 A 7.5 7.5 0 0 1 18 10.5 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18.86 8.67 L 15.33 5.14 M 8.96 18.57 L 20.28 7.26 A 2 2 0 0 0 20.28 4.43 L 19.57 3.72 A 2 2 0 0 0 16.74 3.72 L 5.43 15.04 L 4.37 19.63 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9.88 13 L 12 10.88 L 14.12 13 M 12 11.5 V 16 M 9.52 4 H 4 A 1 1 0 0 0 3 5 V 19 A 1 1 0 0 0 4 20 H 20 A 1 1 0 0 0 21 19 V 7.5 A 1 1 0 0 0 20 6.5 H 12.48 A 1 1 0 0 1 11.7 6.12 L 10.3 4.38 A 1 1 0 0 0 9.52 4", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 11 A 1 1 0 1 0 8 13 z M 12 13 A 1 1 0 1 0 12 11 z M 8 15 A 1 1 0 1 0 8 17 z M 10 17 A 1 1 0 1 0 10 15 z M 16 8 H 15 V 20 H 17 V 8 z M 15 21 V 20 H 5 V 22 H 15 z M 4 20 H 5 V 8 H 3 V 20 z M 5 7 V 8 H 15 V 6 H 5 z M 8 12 V 13 H 12 V 11 H 8 z M 8 16 V 17 H 10 V 15 H 8 z M 20 16 H 21 V 4 H 19 V 16 z M 19 3 V 2 H 9 V 4 H 19 z M 8 4 H 7 V 7 H 9 V 4 z M 16 17 V 18 H 19 V 16 H 16 z M 9 3 V 2 A 2 2 0 0 0 7 4 H 9 z M 20 4 H 21 A 2 2 0 0 0 19 2 V 4 z M 20 16 H 19 V 18 A 2 2 0 0 0 21 16 z M 5 21 V 20 H 3 A 2 2 0 0 0 5 22 z M 16 20 H 15 V 22 A 2 2 0 0 0 17 20 z M 16 8 H 17 A 2 2 0 0 0 15 6 V 8 z M 4 8 H 5 V 6 A 2 2 0 0 0 3 8 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 3 H 14 M 4 6 H 20 M 18.07 6 L 17.07 20.07 A 1 1 0 0 1 16.07 21 H 7.93 A 1 1 0 0 1 6.93 20.07 L 5.93 6", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 8 H 12 M 11 11 H 12 V 17 H 12.5 M 21 12 A 9 9 0 1 1 3 12 A 9 9 0 0 1 21 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_LINE_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 12.5 A 0.5 0.5 0 1 1 7 12.5 A 0.5 0.5 0 0 1 8 12.5 M 10 8.5 A 0.5 0.5 0 1 1 9 8.5 A 0.5 0.5 0 0 1 10 8.5 M 15 8.5 A 0.5 0.5 0 1 1 14 8.5 A 0.5 0.5 0 0 1 15 8.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 17.9 15.48 C 19.22 15.7 20.58 15.03 20.84 13.71 A 9 9 0 1 0 12.37 20.99 C 13.66 20.94 14.26 19.52 13.68 18.36 A 2.12 2.12 0 0 1 14.08 15.92 L 14.17 15.83 A 2.29 2.29 0 0 1 16.16 15.19 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8 12.5 A 0.5 0.5 0 1 1 7 12.5 A 0.5 0.5 0 0 1 8 12.5 Z M 10 8.5 A 0.5 0.5 0 1 1 9 8.5 A 0.5 0.5 0 0 1 10 8.5 Z M 15 8.5 A 0.5 0.5 0 1 1 14 8.5 A 0.5 0.5 0 0 1 15 8.5 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 12 H 20 M 10 17.66 L 4.34 12 L 10 6.34", fill: false, even: false, alpha: 1.0, w: 3.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 12 H 19 M 14 17.66 L 19.66 12 L 14 6.34", fill: false, even: false, alpha: 1.0, w: 3.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 20 V 5 M 6.34 10 L 12 4.34 L 17.66 10", fill: false, even: false, alpha: 1.0, w: 3.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10.5 2 A 8.5 8.5 0 0 1 19 10.5 A 8.46 8.46 0 0 1 17.17 15.76 L 20.83 19.41 A 1 1 0 0 1 19.41 20.83 L 15.76 17.17 A 8.46 8.46 0 0 1 10.5 19 A 8.5 8.5 0 0 1 10.5 2 M 10.5 4 A 6.5 6.5 0 1 0 10.5 17 A 6.5 6.5 0 0 0 10.5 4 M 10.5 5 A 5.5 5.5 0 1 1 10.5 16 A 5.5 5.5 0 0 1 10.5 5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9.63 19.32 A 1.5 1.5 0 0 1 8.9 19.72 L 4.45 20.75 A 1 1 0 0 1 3.25 19.55 L 4.28 15.1 A 1.5 1.5 0 0 1 4.68 14.37 L 13.77 5.28 L 18.72 10.23 z M 20.84 3.87 A 3 3 0 0 1 20.84 8.11 L 20.13 8.82 L 15.18 3.87 L 15.89 3.16 A 3 3 0 0 1 20.13 3.16 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9.52 3 A 2 2 0 0 1 11.08 3.75 L 12.48 5.5 H 20 A 2 2 0 0 1 22 7.5 V 19 A 2 2 0 0 1 20 21 H 4 A 2 2 0 0 1 2 19 V 5 A 2 2 0 0 1 4 3 z M 12 9.88 A 1 1 0 0 0 11.29 10.17 L 9.17 12.29 A 1 1 0 0 0 10.59 13.71 L 11 13.29 V 16 A 1 1 0 0 0 13 16 V 13.29 L 13.41 13.71 A 1 1 0 0 0 14.83 12.29 L 12.71 10.17 A 1 1 0 0 0 12 9.88", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19 2 A 2 2 0 0 1 21 4 V 16 A 2 2 0 0 1 19 18 H 17 V 20 A 2 2 0 0 1 15 22 H 5 A 2 2 0 0 1 3 20 V 8 A 2 2 0 0 1 5 6 H 7 V 4 A 2 2 0 0 1 9 2 z M 8 15 A 1 1 0 1 0 8 17 H 10 A 1 1 0 1 0 10 15 z M 9 6 H 15 A 2 2 0 0 1 17 8 V 16 H 19 V 4 H 9 z M 8 11 A 1 1 0 1 0 8 13 H 12 A 1 1 0 1 0 12 11 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9 3 A 1 1 0 0 1 10 2 H 14 A 1 1 0 1 1 14 4 H 10 A 1 1 0 0 1 9 3 M 5.92 5 H 4 A 1 1 0 0 0 4 7 H 5 L 5.94 20.14 A 2 2 0 0 0 7.93 22 H 16.07 A 2 2 0 0 0 18.06 20.14 L 19 7 H 20 A 1 1 0 1 0 20 5 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 2 C 17.52 2 22 6.48 22 12 S 17.52 22 12 22 S 2 17.52 2 12 S 6.48 2 12 2 M 11 10 A 1 1 0 1 0 11 12 V 17 A 1 1 0 0 0 12 18 H 12.5 A 1 1 0 0 0 13 16.14 V 11 A 1 1 0 0 0 12 10 z M 12 7 A 1 1 0 1 0 12 9 H 12 A 1 1 0 0 0 12 7 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MINGCUTE_FILL_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 2 C 17.52 2 22 6.48 22 12 C 22 12.65 21.94 13.28 21.82 13.9 C 21.43 15.9 19.42 16.75 17.74 16.47 L 16 16.18 A 1.29 1.29 0 0 0 14.87 16.54 L 14.79 16.63 C 14.45 16.97 14.36 17.49 14.58 17.92 C 15 18.76 15 19.73 14.63 20.52 A 2.53 2.53 0 0 1 12.41 21.99 C 12.27 22 12.14 22 12 22 C 6.48 22 2 17.52 2 12 S 6.48 2 12 2 M 7.5 11 A 1.5 1.5 0 1 0 7.5 14 A 1.5 1.5 0 0 0 7.5 11 M 9.5 7 A 1.5 1.5 0 1 0 9.5 10 A 1.5 1.5 0 0 0 9.5 7 M 14.5 7 A 1.5 1.5 0 1 0 14.5 10 A 1.5 1.5 0 0 0 14.5 7", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.83 13 L 12.73 17.9 C 12.93 18.1 13.02 18.33 13.01 18.6 C 13.01 18.87 12.9 19.1 12.7 19.3 C 12.5 19.48 12.27 19.58 12 19.59 C 11.73 19.6 11.5 19.5 11.3 19.3 L 4.7 12.7 C 4.6 12.6 4.53 12.49 4.49 12.37 C 4.45 12.26 4.42 12.13 4.43 12 C 4.43 11.87 4.45 11.74 4.49 11.62 C 4.53 11.51 4.6 11.4 4.7 11.3 L 11.3 4.7 C 11.48 4.52 11.71 4.43 11.99 4.43 C 12.26 4.43 12.5 4.52 12.7 4.7 C 12.9 4.9 13 5.14 13 5.41 C 13 5.69 12.9 5.93 12.7 6.13 L 7.83 11 H 19 C 19.28 11 19.52 11.1 19.71 11.29 C 19.91 11.48 20 11.72 20 12 C 20 12.28 19.9 12.52 19.71 12.71 C 19.52 12.91 19.28 13 19 13 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16.18 13 H 5 C 4.72 13 4.48 12.9 4.29 12.71 C 4.1 12.52 4 12.28 4 12 C 4 11.72 4.1 11.48 4.29 11.29 C 4.48 11.1 4.72 11 5 11 H 16.18 L 11.28 6.1 C 11.07 5.9 10.98 5.67 10.99 5.4 C 11 5.13 11.1 4.9 11.3 4.7 C 11.5 4.52 11.73 4.42 12 4.41 C 12.27 4.4 12.5 4.5 12.7 4.7 L 19.3 11.3 C 19.4 11.4 19.47 11.51 19.51 11.62 C 19.55 11.74 19.58 11.87 19.57 12 C 19.57 12.13 19.55 12.26 19.51 12.37 C 19.47 12.49 19.4 12.6 19.3 12.7 L 12.7 19.3 C 12.52 19.48 12.29 19.57 12.01 19.57 C 11.74 19.57 11.5 19.48 11.3 19.3 C 11.1 19.1 11 18.86 11 18.59 C 11 18.31 11.1 18.08 11.3 17.88 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11 7.83 L 6.1 12.73 C 5.9 12.93 5.67 13.02 5.4 13.01 C 5.13 13.01 4.9 12.9 4.7 12.7 C 4.52 12.5 4.42 12.27 4.41 12 C 4.4 11.73 4.5 11.5 4.7 11.3 L 11.3 4.7 C 11.4 4.6 11.51 4.53 11.62 4.49 C 11.74 4.45 11.87 4.43 12 4.42 C 12.13 4.42 12.26 4.45 12.38 4.49 C 12.49 4.53 12.6 4.6 12.7 4.7 L 19.3 11.3 C 19.48 11.48 19.57 11.71 19.57 11.99 C 19.57 12.26 19.48 12.5 19.3 12.7 C 19.1 12.9 18.86 13 18.59 13 C 18.31 13 18.08 12.9 17.87 12.7 L 13 7.83 V 19 C 13 19.28 12.9 19.52 12.71 19.71 C 12.52 19.91 12.28 20 12 20 C 11.72 20 11.48 19.9 11.29 19.71 C 11.1 19.52 11 19.28 11 19 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9.5 16 C 7.68 16 6.15 15.37 4.89 14.11 C 3.63 12.85 3 11.32 3 9.5 C 3 7.68 3.63 6.15 4.89 4.89 C 6.15 3.63 7.68 3 9.5 3 C 11.32 3 12.85 3.63 14.11 4.89 C 15.37 6.15 16 7.68 16 9.5 C 16 10.23 15.88 10.92 15.65 11.57 C 15.42 12.22 15.1 12.8 14.7 13.3 L 20.3 18.9 C 20.48 19.08 20.57 19.32 20.57 19.6 C 20.57 19.88 20.48 20.12 20.3 20.3 C 20.12 20.48 19.88 20.57 19.6 20.57 C 19.32 20.57 19.08 20.48 18.9 20.3 L 13.3 14.7 C 12.8 15.1 12.22 15.42 11.57 15.65 C 10.92 15.88 10.23 16 9.5 16 M 9.5 14 C 10.75 14 11.81 13.56 12.69 12.69 C 13.56 11.81 14 10.75 14 9.5 C 14 8.25 13.56 7.19 12.69 6.31 C 11.81 5.44 10.75 5 9.5 5 C 8.25 5 7.19 5.44 6.31 6.31 C 5.44 7.19 5 8.25 5 9.5 C 5 10.75 5.44 11.81 6.31 12.69 C 7.19 13.57 8.25 14 9.5 14", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 21 C 3.72 21 3.48 20.9 3.29 20.71 C 3.1 20.52 3 20.28 3 20 V 17.57 C 3 17.31 3.05 17.05 3.15 16.81 C 3.25 16.57 3.39 16.36 3.57 16.17 L 16.2 3.58 C 16.4 3.39 16.62 3.25 16.86 3.15 C 17.11 3.05 17.36 3 17.62 3 C 17.89 3 18.15 3.05 18.4 3.15 C 18.65 3.25 18.87 3.4 19.05 3.6 L 20.43 5 C 20.62 5.18 20.77 5.4 20.86 5.65 C 20.95 5.9 21 6.15 21 6.4 C 21 6.67 20.95 6.92 20.86 7.16 C 20.77 7.41 20.62 7.63 20.42 7.83 L 7.82 20.43 C 7.64 20.61 7.43 20.75 7.19 20.85 C 6.94 20.95 6.69 21 6.42 21 z M 17.6 7.8 L 19 6.4 L 17.6 5 L 16.2 6.4 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 20 C 3.45 20 2.98 19.8 2.59 19.41 C 2.2 19.02 2 18.55 2 18 V 6 C 2 5.45 2.2 4.98 2.59 4.59 C 2.98 4.2 3.45 4 4 4 H 9.18 C 9.44 4 9.7 4.05 9.94 4.15 C 10.18 4.25 10.39 4.39 10.58 4.58 L 12 6 H 20 C 20.55 6 21.02 6.2 21.41 6.59 C 21.8 6.98 22 7.45 22 8 V 18 C 22 18.55 21.8 19.02 21.41 19.41 C 21.02 19.8 20.55 20 20 20 z M 12.2 14 L 11.27 14.93 C 11.09 15.11 11 15.34 11 15.62 C 11 15.91 11.09 16.14 11.27 16.32 C 11.46 16.51 11.69 16.6 11.97 16.6 C 12.26 16.6 12.49 16.51 12.67 16.32 L 15.3 13.7 C 15.5 13.5 15.6 13.27 15.6 13 C 15.6 12.73 15.5 12.5 15.3 12.3 L 12.68 9.68 C 12.49 9.49 12.26 9.4 11.98 9.4 C 11.69 9.4 11.46 9.49 11.28 9.68 C 11.09 9.86 11 10.09 11 10.38 C 11 10.66 11.09 10.89 11.28 11.07 L 12.2 12 H 9 C 8.72 12 8.48 12.1 8.29 12.29 C 8.1 12.48 8 12.72 8 13 C 8 13.28 8.1 13.52 8.29 13.71 C 8.48 13.91 8.72 14 9 14 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 20 C 3.45 20 2.98 19.8 2.59 19.41 C 2.2 19.02 2 18.55 2 18 V 15 H 11 C 11.55 15 12.02 14.8 12.41 14.41 C 12.81 14.02 13 13.55 13 13 V 4 H 20 C 20.55 4 21.02 4.2 21.41 4.59 C 21.8 4.98 22 5.45 22 6 V 18 C 22 18.55 21.8 19.02 21.41 19.41 C 21.02 19.8 20.55 20 20 20 z M 2 14 V 8 C 2 7.45 2.2 6.98 2.59 6.59 C 2.98 6.2 3.45 6 4 6 H 10 z M 5.83 13 L 11 7.83 V 13 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 21 C 6.45 21 5.98 20.8 5.59 20.41 C 5.2 20.02 5 19.55 5 19 V 6 C 4.72 6 4.48 5.9 4.29 5.71 C 4.1 5.52 4 5.28 4 5 C 4 4.72 4.1 4.48 4.29 4.29 C 4.48 4.1 4.72 4 5 4 H 9 C 9 3.72 9.1 3.48 9.29 3.29 C 9.48 3.1 9.72 3 10 3 H 14 C 14.28 3 14.52 3.1 14.71 3.29 C 14.9 3.48 15 3.72 15 4 H 19 C 19.28 4 19.52 4.1 19.71 4.29 C 19.91 4.48 20 4.72 20 5 C 20 5.28 19.9 5.52 19.71 5.71 C 19.52 5.91 19.28 6 19 6 V 19 C 19 19.55 18.8 20.02 18.41 20.41 C 18.02 20.8 17.55 21 17 21 z M 10.71 16.71 C 10.9 16.52 11 16.28 11 16 V 9 C 11 8.72 10.9 8.48 10.71 8.29 C 10.52 8.1 10.28 8 10 8 C 9.72 8 9.48 8.1 9.29 8.29 C 9.1 8.48 9 8.72 9 9 V 16 C 9 16.28 9.1 16.52 9.29 16.71 C 9.48 16.91 9.72 17 10 17 C 10.28 17 10.52 16.9 10.71 16.71 M 14.71 16.71 C 14.9 16.52 15 16.28 15 16 V 9 C 15 8.72 14.9 8.48 14.71 8.29 C 14.52 8.1 14.28 8 14 8 C 13.72 8 13.48 8.1 13.29 8.29 C 13.1 8.48 13 8.72 13 9 V 16 C 13 16.28 13.1 16.52 13.29 16.71 C 13.48 16.91 13.72 17 14 17 C 14.28 17 14.52 16.9 14.71 16.71", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.71 16.71 C 12.9 16.52 13 16.28 13 16 V 12 C 13 11.72 12.9 11.48 12.71 11.29 C 12.52 11.1 12.28 11 12 11 C 11.72 11 11.48 11.1 11.29 11.29 C 11.1 11.48 11 11.72 11 12 V 16 C 11 16.28 11.1 16.52 11.29 16.71 C 11.48 16.91 11.72 17 12 17 C 12.28 17 12.52 16.9 12.71 16.71 M 12.71 8.71 C 12.9 8.52 13 8.28 13 8 C 13 7.72 12.9 7.48 12.71 7.29 C 12.52 7.1 12.28 7 12 7 C 11.72 7 11.48 7.1 11.29 7.29 C 11.1 7.48 11 7.72 11 8 C 11 8.28 11.1 8.52 11.29 8.71 C 11.48 8.91 11.72 9 12 9 C 12.28 9 12.52 8.9 12.71 8.71 M 12 22 C 10.62 22 9.32 21.74 8.1 21.21 C 6.88 20.69 5.82 19.97 4.92 19.07 C 4.03 18.18 3.31 17.12 2.79 15.9 C 2.26 14.68 2 13.38 2 12 C 2 10.62 2.26 9.32 2.79 8.1 C 3.31 6.88 4.03 5.82 4.93 4.92 C 5.82 4.03 6.88 3.31 8.1 2.79 C 9.32 2.26 10.62 2 12 2 C 13.38 2 14.68 2.26 15.9 2.79 C 17.12 3.31 18.18 4.03 19.07 4.93 C 19.97 5.82 20.69 6.88 21.21 8.1 C 21.74 9.32 22 10.62 22 12 C 22 13.38 21.74 14.68 21.21 15.9 C 20.69 17.12 19.98 18.18 19.07 19.07 C 18.17 19.97 17.12 20.69 15.9 21.21 C 14.68 21.74 13.38 22 12 22", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MS_ROUNDED_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 22 C 10.63 22 9.34 21.74 8.12 21.21 C 6.91 20.69 5.85 19.97 4.94 19.06 C 4.03 18.15 3.31 17.09 2.79 15.88 C 2.26 14.66 2 13.37 2 12 C 2 10.62 2.27 9.32 2.81 8.1 C 3.35 6.88 4.09 5.82 5.01 4.92 C 5.94 4.03 7.02 3.31 8.25 2.79 C 9.48 2.26 10.8 2 12.2 2 C 13.53 2 14.79 2.23 15.97 2.69 C 17.16 3.15 18.2 3.78 19.09 4.59 C 19.98 5.4 20.69 6.35 21.21 7.46 C 21.74 8.57 22 9.77 22 11.05 C 22 12.97 21.42 14.44 20.25 15.46 C 19.08 16.49 17.67 17 16 17 H 14.15 C 14 17 13.9 17.04 13.84 17.12 C 13.78 17.21 13.75 17.3 13.75 17.4 C 13.75 17.6 13.88 17.89 14.13 18.26 C 14.38 18.64 14.5 19.07 14.5 19.55 C 14.5 20.38 14.27 21 13.81 21.4 C 13.36 21.8 12.75 22 12 22 M 7.58 12.57 C 7.86 12.29 8 11.93 8 11.5 C 8 11.07 7.86 10.71 7.58 10.43 C 7.29 10.14 6.93 10 6.5 10 C 6.07 10 5.71 10.14 5.42 10.43 C 5.14 10.71 5 11.07 5 11.5 C 5 11.93 5.14 12.29 5.42 12.57 C 5.71 12.86 6.07 13 6.5 13 C 6.93 13 7.29 12.86 7.58 12.57 M 10.57 8.57 C 10.86 8.29 11 7.93 11 7.5 C 11 7.07 10.86 6.71 10.57 6.42 C 10.29 6.14 9.93 6 9.5 6 C 9.07 6 8.71 6.14 8.43 6.42 C 8.14 6.71 8 7.07 8 7.5 C 8 7.93 8.14 8.29 8.43 8.57 C 8.71 8.86 9.07 9 9.5 9 C 9.93 9 10.29 8.86 10.57 8.57 M 15.57 8.57 C 15.86 8.29 16 7.93 16 7.5 C 16 7.07 15.86 6.71 15.57 6.42 C 15.29 6.14 14.93 6 14.5 6 C 14.07 6 13.71 6.14 13.43 6.42 C 13.14 6.71 13 7.07 13 7.5 C 13 7.93 13.14 8.29 13.43 8.57 C 13.71 8.86 14.07 9 14.5 9 C 14.93 9 15.29 8.86 15.57 8.57 M 18.57 12.57 C 18.86 12.29 19 11.93 19 11.5 C 19 11.07 18.86 10.71 18.57 10.43 C 18.29 10.14 17.93 10 17.5 10 C 17.07 10 16.71 10.14 16.43 10.43 C 16.14 10.71 16 11.07 16 11.5 C 16 11.93 16.14 12.29 16.43 12.57 C 16.71 12.86 17.07 13 17.5 13 C 17.93 13 18.29 12.86 18.57 12.57", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18.41 7.41 L 17 6 L 11 12 L 17 18 L 18.41 16.59 L 13.83 12 z M 12.41 7.41 L 11 6 L 5 12 L 11 18 L 12.41 16.59 L 7.83 12 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.59 7.41 L 7 6 L 13 12 L 7 18 L 5.59 16.59 L 10.17 12 z M 11.59 7.41 L 13 6 L 19 12 L 13 18 L 11.59 16.59 L 16.17 12 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13 20 H 11 V 8 L 5.5 13.5 L 4.08 12.08 L 12 4.16 L 19.92 12.08 L 18.5 13.5 L 13 8 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9.5 3 A 6.5 6.5 0 0 1 16 9.5 C 16 11.11 15.41 12.59 14.44 13.73 L 14.71 14 H 15.5 L 20.5 19 L 19 20.5 L 14 15.5 V 14.71 L 13.73 14.44 A 6.52 6.52 0 0 1 9.5 16 A 6.5 6.5 0 0 1 3 9.5 A 6.5 6.5 0 0 1 9.5 3 M 9.5 5 C 7 5 5 7 5 9.5 S 7 14 9.5 14 S 14 12 14 9.5 S 12 5 9.5 5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20.71 7.04 C 21.1 6.65 21.1 6 20.71 5.63 L 18.37 3.29 C 18 2.9 17.35 2.9 16.96 3.29 L 15.12 5.12 L 18.87 8.87 M 3 17.25 V 21 H 6.75 L 17.81 9.93 L 14.06 6.18 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14 18 V 15 H 10 V 11 H 14 V 8 L 19 13 M 20 6 H 12 L 10 4 H 4 C 2.89 4 2 4.89 2 6 V 18 A 2 2 0 0 0 4 20 H 20 A 2 2 0 0 0 22 18 V 8 A 2 2 0 0 0 20 6", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19 3 H 14.82 C 14.4 1.84 13.3 1 12 1 S 9.6 1.84 9.18 3 H 5 A 2 2 0 0 0 3 5 V 19 A 2 2 0 0 0 5 21 H 19 A 2 2 0 0 0 21 19 V 5 A 2 2 0 0 0 19 3 M 12 3 A 1 1 0 0 1 13 4 A 1 1 0 0 1 12 5 A 1 1 0 0 1 11 4 A 1 1 0 0 1 12 3", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19 4 H 15.5 L 14.5 3 H 9.5 L 8.5 4 H 5 V 6 H 19 M 6 19 A 2 2 0 0 0 8 21 H 16 A 2 2 0 0 0 18 19 V 7 H 6 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13 9 H 11 V 7 H 13 M 13 17 H 11 V 11 H 13 M 12 2 A 10 10 0 0 0 2 12 A 10 10 0 0 0 12 22 A 10 10 0 0 0 22 12 A 10 10 0 0 0 12 2", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MDI_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17.5 12 A 1.5 1.5 0 0 1 16 10.5 A 1.5 1.5 0 0 1 17.5 9 A 1.5 1.5 0 0 1 19 10.5 A 1.5 1.5 0 0 1 17.5 12 M 14.5 8 A 1.5 1.5 0 0 1 13 6.5 A 1.5 1.5 0 0 1 14.5 5 A 1.5 1.5 0 0 1 16 6.5 A 1.5 1.5 0 0 1 14.5 8 M 9.5 8 A 1.5 1.5 0 0 1 8 6.5 A 1.5 1.5 0 0 1 9.5 5 A 1.5 1.5 0 0 1 11 6.5 A 1.5 1.5 0 0 1 9.5 8 M 6.5 12 A 1.5 1.5 0 0 1 5 10.5 A 1.5 1.5 0 0 1 6.5 9 A 1.5 1.5 0 0 1 8 10.5 A 1.5 1.5 0 0 1 6.5 12 M 12 3 A 9 9 0 0 0 3 12 A 9 9 0 0 0 12 21 A 1.5 1.5 0 0 0 13.5 19.5 C 13.5 19.11 13.35 18.76 13.11 18.5 C 12.88 18.23 12.73 17.88 12.73 17.5 A 1.5 1.5 0 0 1 14.23 16 H 16 A 5 5 0 0 0 21 11 C 21 6.58 16.97 3 12 3", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_0: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 27 28 A 1 1 0 0 1 26.5 27.87 L 7.5 16.87 A 1 1 0 0 1 7.5 15.13 L 26.5 4.13 A 1 1 0 0 1 27.5 4.13 A 1 1 0 0 1 28 5 V 27 A 1 1 0 0 1 27 28 M 10 16 L 26 25.27 V 6.73 Z M 2 4 H 4 V 28 H 2 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_1: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 5 28 A 1 1 0 0 1 4 27 V 5 A 1 1 0 0 1 4.5 4.13 A 1 1 0 0 1 5.5 4.13 L 24.5 15.13 A 1 1 0 0 1 24.5 16.87 L 5.5 27.87 A 1 1 0 0 1 5 28 M 6 6.73 V 25.27 L 22 16 Z M 28 4 H 30 V 28 H 28 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_2: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 16 4 L 6 14 L 7.41 15.41 L 15 7.83 V 28 H 17 V 7.83 L 24.59 15.41 L 26 14 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_3: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 29 27.59 L 21.45 20.03 A 11.02 11.02 0 1 0 20.03 21.45 L 27.59 29 Z M 4 13 A 9 9 0 1 1 13 22 A 9.01 9.01 0 0 1 4 13", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_4: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 2 26 H 30 V 28 H 2 z M 25.4 9 C 26.2 8.2 26.2 7 25.4 6.2 L 21.8 2.6 C 21 1.8 19.8 1.8 19 2.6 L 4 17.6 V 24 H 10.4 z M 20.4 4 L 24 7.6 L 21 10.6 L 17.4 7 z M 6 22 V 18.4 L 16 8.4 L 19.6 12 L 9.6 22 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_5: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 18 13 L 16.59 14.41 L 19.17 17 H 10 V 19 H 19.17 L 16.59 21.59 L 18 23 L 23 18 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 11.17 6 L 14.59 9.41 L 15.17 10 H 28 V 26 H 4 V 6 z M 11.17 4 H 4 A 2 2 0 0 0 2 6 V 26 A 2 2 0 0 0 4 28 H 28 A 2 2 0 0 0 30 26 V 10 A 2 2 0 0 0 28 8 H 16 L 12.59 4.59 A 2 2 0 0 0 11.17 4", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_6: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 28 10 V 28 H 10 V 10 z M 28 8 H 10 A 2 2 0 0 0 8 10 V 28 A 2 2 0 0 0 10 30 H 28 A 2 2 0 0 0 30 28 V 10 A 2 2 0 0 0 28 8", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4 18 H 2 V 4 A 2 2 0 0 1 4 2 H 18 V 4 H 4 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_7: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 29 26 H 12 A 1 1 0 0 1 11.29 25.71 L 2.29 16.71 A 1 1 0 0 1 2.29 15.29 L 11.29 6.29 A 1 1 0 0 1 12 6 H 29 A 1 1 0 0 1 30 7 V 25 A 1 1 0 0 1 29 26 M 12.41 24 H 28 V 8 H 12.41 L 4.41 16 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 20.41 16 L 25 11.41 L 23.59 10 L 19 14.59 L 14.41 10 L 13 11.41 L 17.59 16 L 13 20.59 L 14.41 22 L 19 17.41 L 23.59 22 L 25 20.59 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_8: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 17 22 V 14 H 13 V 16 H 15 V 22 H 12 V 24 H 20 V 22 z M 16 8 A 1.5 1.5 0 1 0 17.5 9.5 A 1.5 1.5 0 0 0 16 8", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 16 30 A 14 14 0 1 1 30 16 A 14 14 0 0 1 16 30 M 16 4 A 12 12 0 1 0 28 16 A 12 12 0 0 0 16 4", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CARBON_9: Glyph = Glyph {
    grid: 32.0,
    parts: &[
        Part { d: "M 28.83 23.17 L 23 17.33 V 13 A 1 1 0 0 0 22.71 12.29 L 12.71 2.29 A 1 1 0 0 0 11.29 2.29 L 2.29 11.29 A 1 1 0 0 0 2.29 12.71 L 12.29 22.71 A 1 1 0 0 0 13 23 H 17.34 L 23.17 28.84 A 4 4 0 0 0 28.83 23.18 Z M 6 10.41 L 8.29 12.71 L 9.71 11.29 L 7.41 9 L 9 7.41 L 13.29 11.71 L 14.71 10.29 L 10.41 6 L 12 4.41 L 18.59 11 L 11 18.59 L 4.41 12 Z M 27.41 27.41 A 2 2 0 0 1 24.59 27.41 L 18.46 21.29 A 1.8 1.8 0 0 0 17.75 21 H 13.41 L 12.41 20 L 20 12.41 L 21 13.41 V 17.75 A 1 1 0 0 0 21.29 18.45 L 27.41 24.59 A 2 2 0 0 1 27.41 27.41", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_0: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 5.8 24 H 41.8 M 17.8 36 L 5.8 24 L 17.8 12", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_1: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 42 24 H 6 M 30 12 L 42 24 L 30 36", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_2: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 24 6 V 42 M 12 18 L 24 6 L 36 18", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_3: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 21 38 C 30.39 38 38 30.39 38 21 S 30.39 4 21 4 S 4 11.61 4 21 S 11.61 38 21 38 Z", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 26.66 14.34 A 7.98 7.98 0 0 0 21 12 A 7.98 7.98 0 0 0 15.34 14.34 M 33.22 33.22 L 41.71 41.71", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_4: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 31 9 L 39 17 M 8 32 L 36 4 L 44 12 L 16 40 L 6 42 z M 31 9 L 39 17 M 9 32 L 16 39 M 13 35 L 35 13", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_5: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 5 8 A 2 2 0 0 1 7 6 H 19 L 24 12 H 41 A 2 2 0 0 1 43 14 V 40 A 2 2 0 0 1 41 42 H 7 A 2 2 0 0 1 5 40 z", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 30 25.99 L 24 20 L 18 26 M 24 20 V 34", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_6: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 13 12.43 V 7.81 A 2.81 2.81 0 0 1 15.81 5 H 40.19 A 2.81 2.81 0 0 1 43 7.81 V 32.19 A 2.81 2.81 0 0 1 40.19 35 H 35.52", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 32.19 13 H 7.81 A 2.81 2.81 0 0 0 5 15.81 V 40.19 A 2.81 2.81 0 0 0 7.81 43 H 32.19 A 2.81 2.81 0 0 0 35 40.19 V 15.81 A 2.81 2.81 0 0 0 32.19 13 Z", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_7: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 9 10 V 44 H 39 V 10 z", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 20 20 V 33 M 28 20 V 33 M 4 10 H 44", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 16 10 L 19.29 4 H 28.78 L 32 10 z", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_8: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 24 44 A 19.94 19.94 0 0 0 38.14 38.14 A 19.94 19.94 0 0 0 44 24 A 19.94 19.94 0 0 0 38.14 9.86 A 19.94 19.94 0 0 0 24 4 A 19.94 19.94 0 0 0 9.86 9.86 A 19.94 19.94 0 0 0 4 24 A 19.94 19.94 0 0 0 9.86 38.14 A 19.94 19.94 0 0 0 24 44 Z", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 24 11 A 2.5 2.5 0 1 1 24 16 A 2.5 2.5 0 0 1 24 11", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 24.5 34 V 20 H 22.5 M 21 34 H 28", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONPARK_LINE_9: Glyph = Glyph {
    grid: 48.0,
    parts: &[
        Part { d: "M 15.54 22.9 L 25.43 32.8 M 15.54 22.9 L 7.05 31.38 A 7 7 0 1 0 16.95 41.28 L 25.43 32.8 M 15.54 32.8 L 11.29 37.04 M 25.43 32.8 L 40.34 26.36 C 43.03 25.2 44.15 21.99 42.43 19.62 C 38.26 13.9 32.65 8.89 28.51 5.82 C 26.22 4.13 23.18 5.18 22.05 7.8 L 15.54 22.9 z", fill: false, even: false, alpha: 1.0, w: 4.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_0: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 872 474 H 286.9 L 637.1 170 C 642.7 165.1 639.3 156 631.9 156 H 543.4 C 539.5 156 535.8 157.4 532.9 159.9 L 155 487.8 A 31.96 31.96 0 0 0 155 536.1 L 535.1 866 C 536.6 867.3 538.4 868 540.3 868 H 631.8 C 639.2 868 642.6 858.8 637 854 L 286.9 550 H 872 C 876.4 550 880 546.4 880 542 V 482 C 880 477.6 876.4 474 872 474", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_1: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 869 487.8 L 491.2 159.9 C 488.3 157.4 484.6 156 480.7 156 H 392.2 C 384.8 156 381.4 165.2 387 170 L 737.2 474 H 152 C 147.6 474 144 477.6 144 482 V 542 C 144 546.4 147.6 550 152 550 H 737.1 L 386.9 854 C 381.3 858.9 384.7 868 392.1 868 H 483.6 C 485.5 868 487.4 867.3 488.8 866 L 869 536.2 A 32.07 32.07 0 0 0 869 487.8", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_2: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 868 545.5 L 536.1 163 A 31.96 31.96 0 0 0 487.8 163 L 156 545.5 A 7.97 7.97 0 0 0 162 558.7 H 243 C 247.6 558.7 252 556.7 255.1 553.2 L 474 300.9 V 864 C 474 868.4 477.6 872 482 872 H 542 C 546.4 872 550 868.4 550 864 V 300.9 L 768.9 553.2 C 771.9 556.7 776.3 558.7 781 558.7 H 862 C 868.8 558.7 872.5 550.7 868 545.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_3: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 909.6 854.5 L 649.9 594.8 C 690.2 542.7 712 479 712 412 C 712 331.8 680.7 256.6 624.1 199.9 S 492.1 112 412 112 S 256.5 143.3 199.9 199.9 C 143.2 256.5 112 331.8 112 412 C 112 492.1 143.3 567.5 199.9 624.1 C 256.5 680.8 331.8 712 412 712 C 479 712 542.6 690.2 594.7 650 L 854.4 909.6 A 8.2 8.2 0 0 0 866 909.6 L 909.6 866.1 A 8.2 8.2 0 0 0 909.6 854.5 M 570.4 570.4 C 528 612.7 471.8 636 412 636 S 296 612.7 253.6 570.4 C 211.3 528 188 471.8 188 412 S 211.3 295.9 253.6 253.6 C 296 211.3 352.2 188 412 188 S 528.1 211.2 570.4 253.6 S 636 352.2 636 412 S 612.7 528.1 570.4 570.4", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_4: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 257.7 752 C 259.7 752 261.7 751.8 263.7 751.5 L 431.9 722 C 433.9 721.6 435.8 720.7 437.2 719.2 L 861.1 295.3 A 9.96 9.96 0 0 0 861.1 281.2 L 694.9 114.9 C 693 113 690.5 112 687.8 112 S 682.6 113 680.7 114.9 L 256.8 538.8 C 255.3 540.3 254.4 542.1 254 544.1 L 224.5 712.3 A 33.5 33.5 0 0 0 233.9 742.1 C 240.5 748.5 248.8 752 257.7 752 M 325.1 577.6 L 687.8 215 L 761.1 288.3 L 398.4 650.9 L 309.5 666.6 z M 880 836 H 144 C 126.3 836 112 850.3 112 868 V 904 C 112 908.4 115.6 912 120 912 H 904 C 908.4 912 912 908.4 912 904 V 868 C 912 850.3 897.7 836 880 836", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_5: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 880 912 H 144 C 126.3 912 112 897.7 112 880 V 144 C 112 126.3 126.3 112 144 112 H 504 C 508.4 112 512 115.6 512 120 V 176 C 512 180.4 508.4 184 504 184 H 184 V 840 H 840 V 520 C 840 515.6 843.6 512 848 512 H 904 C 908.4 512 912 515.6 912 520 V 880 C 912 897.7 897.7 912 880 912 M 770.87 199.13 L 718.67 146.93 C 713.97 142.23 716.77 134.13 723.37 133.33 L 902.77 112.33 C 907.87 111.73 912.27 116.03 911.67 121.23 L 890.67 300.63 C 889.87 307.23 881.77 310.03 877.07 305.33 L 824.67 252.93 L 568.47 509.13 C 565.37 512.23 560.27 512.23 557.17 509.13 L 514.77 466.73 C 511.67 463.63 511.67 458.53 514.77 455.43 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_6: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 832 64 H 296 C 291.6 64 288 67.6 288 72 V 128 C 288 132.4 291.6 136 296 136 H 792 V 824 C 792 828.4 795.6 832 800 832 H 856 C 860.4 832 864 828.4 864 824 V 96 C 864 78.3 849.7 64 832 64 M 704 192 H 192 C 174.3 192 160 206.3 160 224 V 754.7 C 160 763.2 163.4 771.3 169.4 777.3 L 342.7 950.6 C 344.9 952.8 347.4 954.6 350.1 956.1 V 958 H 354.3 C 357.8 959.3 361.5 960 365.3 960 H 704 C 721.7 960 736 945.7 736 928 V 224 C 736 206.3 721.7 192 704 192 M 350 856.2 L 263.9 770 H 350 z M 664 888 H 414 V 746 C 414 723.9 396.1 706 374 706 H 232 V 264 H 664 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_7: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 360 184 H 352 C 356.4 184 360 180.4 360 176 z H 664 V 176 C 664 180.4 667.6 184 672 184 H 664 V 256 H 736 V 176 C 736 140.7 707.3 112 672 112 H 352 C 316.7 112 288 140.7 288 176 V 256 H 360 z M 864 256 H 160 C 142.3 256 128 270.3 128 288 V 320 C 128 324.4 131.6 328 136 328 H 196.4 L 221.1 851 C 222.7 885.1 250.9 912 285 912 H 739 C 773.2 912 801.3 885.2 802.9 851 L 827.6 328 H 888 C 892.4 328 896 324.4 896 320 V 288 C 896 270.3 881.7 256 864 256 M 731.3 840 H 292.7 L 268.5 328 H 755.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_8: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 512 64 C 264.6 64 64 264.6 64 512 S 264.6 960 512 960 S 960 759.4 960 512 S 759.4 64 512 64 M 512 884 C 306.6 884 140 717.4 140 512 S 306.6 140 512 140 S 884 306.6 884 512 S 717.4 884 512 884", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 464 336 A 48 48 0 1 0 560 336 A 48 48 0 1 0 464 336 M 536 448 H 488 C 483.6 448 480 451.6 480 456 V 728 C 480 732.4 483.6 736 488 736 H 536 C 540.4 736 544 732.4 544 728 V 456 C 544 451.6 540.4 448 536 448", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ANTD_OUTLINE_9: Glyph = Glyph {
    grid: 1024.0,
    parts: &[
        Part { d: "M 840 192 H 784 V 120 C 784 106.7 773.3 96 760 96 H 168 C 154.7 96 144 106.7 144 120 V 392 C 144 405.3 154.7 416 168 416 H 760 C 773.3 416 784 405.3 784 392 V 256 H 816 V 456 H 465 C 442.9 456 425 473.9 425 496 V 632 H 381 C 376.6 632 373 635.6 373 640 V 868 C 373 868.6 373.1 869.3 373.2 869.9 C 373.1 871.9 373 874 373 876 C 373 922.4 410.6 960 457 960 S 541 922.4 541 876 C 541 873.9 540.9 871.9 540.8 869.9 C 540.9 869.3 541 868.7 541 868 V 640 C 541 635.6 537.4 632 533 632 H 489 V 520 H 840 C 862.1 520 880 502.1 880 480 V 232 C 880 209.9 862.1 192 840 192 M 720 352 H 208 V 160 H 720 z M 477 876 C 477 887 468 896 457 896 S 437 887 437 876 V 696 H 477 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_0: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 6.22 3.08 A 0.5 0.5 0 0 1 6.92 3.78 L 6.85 3.85 L 3.71 7 H 12.5 A 0.5 0.5 0 0 1 12.5 8 H 3.71 L 6.85 11.15 L 6.92 11.22 A 0.5 0.5 0 0 1 6.23 11.92 L 6.15 11.85 L 2.15 7.85 A 0.5 0.5 0 0 1 2.15 7.15 L 6.15 3.15 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_1: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 8.15 3.15 A 0.5 0.5 0 0 1 8.78 3.08 L 8.85 3.15 L 12.85 7.15 A 0.5 0.5 0 0 1 12.85 7.85 L 8.85 11.85 A 0.5 0.5 0 1 1 8.15 11.15 L 11.29 8 H 2.5 A 0.5 0.5 0 0 1 2.5 7 H 11.29 L 8.15 3.85 L 8.08 3.78 A 0.5 0.5 0 0 1 8.15 3.15", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_2: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 7.22 2.08 A 0.5 0.5 0 0 1 7.85 2.15 L 11.85 6.15 L 11.92 6.23 A 0.5 0.5 0 0 1 11.22 6.92 L 11.15 6.85 L 8 3.71 V 12.5 A 0.5 0.5 0 0 1 7 12.5 V 3.71 L 3.85 6.85 A 0.5 0.5 0 1 1 3.15 6.15 L 7.15 2.15 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_3: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 6.5 2 A 4.5 4.5 0 0 1 10.02 9.31 L 12.85 12.15 L 12.92 12.22 A 0.5 0.5 0 0 1 12.23 12.92 L 12.15 12.85 L 9.31 10.01 A 4.5 4.5 0 1 1 6.5 2 M 6.5 3 A 3.5 3.5 0 1 0 6.5 10 A 3.5 3.5 0 0 0 6.5 3", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_4: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 11.22 1.08 A 0.5 0.5 0 0 1 11.85 1.15 L 13.85 3.15 L 13.92 3.23 A 0.5 0.5 0 0 1 13.85 3.85 L 6.42 11.29 A 1 1 0 0 1 6.19 11.46 L 6.11 11.5 L 2.7 12.96 A 0.5 0.5 0 0 1 2.04 12.3 L 3.5 8.89 L 3.54 8.81 C 3.59 8.72 3.65 8.65 3.71 8.58 L 11.15 1.15 z M 4.42 9.29 L 3.78 10.78 L 4.22 11.22 L 5.71 10.58 L 12.79 3.5 L 11.5 2.21 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_5: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 7.5 9 A 0.5 0.5 0 0 1 8 9.5 V 12.61 L 8.93 11.68 A 0.45 0.45 0 0 1 9.57 12.32 L 7.82 14.07 A 0.45 0.45 0 0 1 7.25 14.13 L 7.18 14.07 L 5.43 12.32 L 5.37 12.25 A 0.45 0.45 0 0 1 6 11.62 L 6.07 11.68 L 7 12.61 V 9.5 A 0.5 0.5 0 0 1 7.5 9 M 11.68 5.43 A 0.45 0.45 0 0 1 12.25 5.37 L 12.32 5.43 L 14.07 7.18 L 14.13 7.25 A 0.45 0.45 0 0 1 14.07 7.82 L 12.32 9.57 A 0.45 0.45 0 0 1 11.68 8.93 L 12.61 8 H 9.5 A 0.5 0.5 0 0 1 9.5 7 H 12.61 L 11.68 6.07 L 11.62 6 A 0.45 0.45 0 0 1 11.68 5.43 M 2.68 5.43 A 0.45 0.45 0 0 1 3.32 6.07 L 2.39 7 H 5.5 L 5.6 7.01 A 0.5 0.5 0 0 1 5.6 7.99 L 5.5 8 H 2.39 L 3.32 8.93 L 3.38 9 A 0.45 0.45 0 0 1 2.75 9.63 L 2.68 9.57 L 0.93 7.82 A 0.45 0.45 0 0 1 0.93 7.18 z M 7.25 0.87 A 0.45 0.45 0 0 1 7.82 0.93 L 9.57 2.68 L 9.63 2.75 A 0.45 0.45 0 0 1 9 3.38 L 8.93 3.32 L 8 2.39 V 5.5 L 7.99 5.6 A 0.5 0.5 0 0 1 7.01 5.6 L 7 5.5 V 2.39 L 6.07 3.32 A 0.45 0.45 0 0 1 5.43 2.68 L 7.18 0.93 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_6: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 10 4 V 2.5 A 0.5 0.5 0 0 0 9.5 2 H 2.5 A 0.5 0.5 0 0 0 2 2.5 V 9.5 A 0.5 0.5 0 0 0 2.5 10 H 4 V 5.5 A 1.5 1.5 0 0 1 5.5 4 z M 5.5 5 A 0.5 0.5 0 0 0 5 5.5 V 12.5 A 0.5 0.5 0 0 0 5.5 13 H 12.5 A 0.5 0.5 0 0 0 13 12.5 V 5.5 A 0.5 0.5 0 0 0 12.5 5 z M 12.5 4 A 1.5 1.5 0 0 1 14 5.5 V 12.5 A 1.5 1.5 0 0 1 12.5 14 H 5.5 A 1.5 1.5 0 0 1 4 12.5 V 11 H 2.5 A 1.5 1.5 0 0 1 1 9.5 V 2.5 A 1.5 1.5 0 0 1 2.5 1 H 9.5 A 1.5 1.5 0 0 1 11 2.5 V 4 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_7: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 11.5 3 A 0.5 0.5 0 0 1 11.5 4 H 11 V 12 L 10.99 12.1 A 1 1 0 0 1 10.1 13 L 10 13 H 5 L 4.9 12.99 A 1 1 0 0 1 4 12.1 L 4 12 V 4 H 3.5 A 0.5 0.5 0 0 1 3.5 3 z M 5 12 H 10 V 4 H 5 z M 9.5 1 A 0.5 0.5 0 0 1 9.5 2 H 5.5 A 0.5 0.5 0 0 1 5.5 1 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_8: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 7.5 0.88 A 6.62 6.62 0 1 1 7.5 14.12 A 6.62 6.62 0 0 1 7.5 0.88 M 7.5 1.83 A 5.67 5.67 0 1 0 7.5 13.17 A 5.67 5.67 0 0 0 7.5 1.82 M 7.6 6.01 A 0.5 0.5 0 0 1 8 6.5 V 10 H 9 V 11 H 6 V 10 H 7 V 7 H 6 V 6 H 7.5 z M 7.5 3.75 A 0.75 0.75 0 1 1 7.5 5.25 A 0.75 0.75 0 0 1 7.5 3.75", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_RADIX_9: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 7.5 0.88 A 6.62 6.62 0 1 1 7.5 14.12 A 6.62 6.62 0 0 1 7.5 0.88 M 3.78 11.78 A 5.65 5.65 0 0 0 7.1 13.16 V 8.47 z M 7.9 13.16 A 5.64 5.64 0 0 0 11.21 11.78 L 7.9 8.47 z M 1.84 7.9 A 5.65 5.65 0 0 0 3.21 11.22 L 6.53 7.9 z M 11.78 11.22 A 5.64 5.64 0 0 0 13.16 7.9 H 8.47 z M 8.47 7.1 H 13.16 A 5.65 5.65 0 0 0 11.78 3.78 z M 3.22 3.78 A 5.65 5.65 0 0 0 1.84 7.1 H 6.53 z M 7.1 1.84 A 5.65 5.65 0 0 0 3.78 3.22 L 7.1 6.53 z M 7.9 6.53 L 11.22 3.22 A 5.64 5.64 0 0 0 7.9 1.84 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.71 5.29 A 1 1 0 0 1 11.71 6.71 L 6.41 12 L 11.71 17.29 A 1 1 0 0 1 10.29 18.71 L 4.29 12.71 A 1 1 0 0 1 4.29 11.29 L 10.29 5.29 A 1 1 0 0 1 11.71 5.29 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 19.71 5.29 A 1 1 0 0 1 19.71 6.71 L 14.41 12 L 19.71 17.29 A 1 1 0 0 1 18.29 18.71 L 12.29 12.71 A 1 1 0 0 1 12.29 11.29 L 18.29 5.29 A 1 1 0 0 1 19.71 5.29 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.29 5.29 A 1 1 0 0 0 12.29 6.71 L 17.59 12 L 12.29 17.29 A 1 1 0 0 0 13.71 18.71 L 19.71 12.71 A 1 1 0 0 0 19.71 11.29 L 13.71 5.29 A 1 1 0 0 0 12.29 5.29 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4.29 5.29 A 1 1 0 0 0 4.29 6.71 L 9.59 12 L 4.29 17.29 A 1 1 0 1 0 5.71 18.71 L 11.71 12.71 A 1 1 0 0 0 11.71 11.29 L 5.71 5.29 A 1 1 0 0 0 4.29 5.29 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 5 L 18 11 M 12 5 L 6 11 M 12 5 V 19", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 20 L 15.95 15.95 M 15.95 15.95 A 7 7 0 1 0 6.05 6.05 A 7 7 0 0 0 15.95 15.95", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.59 3 A 2 2 0 0 1 18.41 3 L 21 5.59 A 2 2 0 0 1 21 8.41 L 9 20.41 A 2 2 0 0 1 7.59 21 H 5 A 2 2 0 0 1 3 19 V 16.41 A 2 2 0 0 1 3.59 15 L 15.59 3 z M 15.41 6 L 18 8.59 L 19.59 7 L 17 4.41 L 15.41 6 z M 16.59 10 L 14 7.41 L 5 16.41 V 19 H 7.59 L 16.59 10 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 12 L 13.6 5 V 8.5 C 10.4 8.5 4 10.6 4 19 C 4 17.83 5.92 15.5 13.6 15.5 V 19 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16 6 A 4 4 0 0 0 12 2 H 6 A 4 4 0 0 0 2 6 V 12 A 4 4 0 0 0 6 16 H 8 V 18 A 4 4 0 0 0 12 22 H 18 A 4 4 0 0 0 22 18 V 12 A 4 4 0 0 0 18 8 H 16 V 6 z M 14 8 H 12 A 4 4 0 0 0 8 12 V 14 H 6 A 2 2 0 0 1 4 12 V 6 A 2 2 0 0 1 6 4 H 12 A 2 2 0 0 1 14 6 V 8 z M 18 10 A 2 2 0 0 1 20 12 V 18 A 2 2 0 0 1 18 20 H 12 A 2 2 0 0 1 10 18 V 12 A 2 2 0 0 1 12 10 H 18 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 7 A 1 1 0 0 1 5 6 H 19 A 1 1 0 1 1 19 8 V 18 A 3 3 0 0 1 16 21 H 8 A 3 3 0 0 1 5 18 V 8 A 1 1 0 0 1 4 7 z M 7 8 V 18 A 1 1 0 0 0 8 19 H 16 A 1 1 0 0 0 17 18 V 8 H 7 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 11 5 A 1 1 0 0 0 10 6 V 7 A 1 1 0 0 1 8 7 V 6 A 3 3 0 0 1 11 3 H 13 A 3 3 0 0 1 16 6 V 7 A 1 1 0 1 1 14 7 V 6 A 1 1 0 0 0 13 5 H 11 z M 10 10 A 1 1 0 0 1 11 11 V 16 A 1 1 0 1 1 9 16 V 11 A 1 1 0 0 1 10 10 z M 14 10 A 1 1 0 0 1 15 11 V 16 A 1 1 0 1 1 13 16 V 11 A 1 1 0 0 1 14 10 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 12 A 9 9 0 1 0 21 12 A 9 9 0 1 0 3 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 15 V 12 M 12 9", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_LINE_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 5 A 3 3 0 0 1 5 2 H 9 A 3 3 0 0 1 12 5 V 21 A 1 1 0 0 1 11 22 H 5 A 3 3 0 0 1 2 19 V 5 z M 5 4 A 1 1 0 0 0 4 5 V 19 A 1 1 0 0 0 5 20 H 10 V 5 A 1 1 0 0 0 9 4 H 5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 14.71 7.12 A 1 1 0 0 0 13.29 7.12 L 11.71 8.71 A 1 1 0 1 1 10.29 7.29 L 11.88 5.71 A 3 3 0 0 1 16.12 5.71 L 18.29 7.88 A 3 3 0 0 1 18.41 12 H 19 A 3 3 0 0 1 22 15 V 19 A 3 3 0 0 1 19 22 H 11 A 1 1 0 1 1 11 20 H 19 A 1 1 0 0 0 20 19 V 15 A 1 1 0 0 0 19 14 H 16.41 L 11.71 18.71 A 1 1 0 1 1 10.29 17.29 L 16.88 10.71 A 1 1 0 0 0 16.88 9.29 L 14.71 7.12 z M 7 15 A 1 1 0 0 1 8 16 V 16 A 1 1 0 1 1 6 16 V 16 A 1 1 0 0 1 7 15 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11 8 A 1 1 0 0 0 9.29 7.29 L 5.29 11.29 A 1 1 0 0 0 5.29 12.71 L 9.29 16.71 A 1 1 0 0 0 11 16 V 8 z M 19 8 A 1 1 0 0 0 17.29 7.29 L 13.29 11.29 A 1 1 0 0 0 13.29 12.71 L 17.29 16.71 A 1 1 0 0 0 19 16 V 8 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 16 A 1 1 0 0 0 6.71 16.71 L 10.71 12.71 A 1 1 0 0 0 10.71 11.29 L 6.71 7.29 A 1 1 0 0 0 5 8 V 16 z M 13 16 A 1 1 0 0 0 14.71 16.71 L 18.71 12.71 A 1 1 0 0 0 18.71 11.29 L 14.71 7.29 A 1 1 0 0 0 13 8 V 16 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 5 L 18 11 M 12 5 L 6 11 M 12 5 V 19", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 11 A 6 6 0 1 1 17 11 A 6 6 0 0 1 5 11 M 11 3 A 8 8 0 1 0 15.91 17.32 L 19.29 20.71 A 1 1 0 0 0 20.71 19.29 L 17.32 15.91 A 8 8 0 0 0 11 3 M 11 15 A 4 4 0 1 0 11 7 A 4 4 0 0 0 11 15", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.59 3 A 2 2 0 0 1 18.41 3 L 21 5.59 A 2 2 0 0 1 21 8.41 L 19.41 10 L 14 4.59 L 15.59 3 z M 12.59 6 L 3.59 15 A 2 2 0 0 0 3 16.41 V 19 A 2 2 0 0 0 5 21 H 7.59 A 2 2 0 0 0 9 20.41 L 18 11.41 L 12.59 6 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 12 L 13.6 5 V 8.5 C 10.4 8.5 4 10.6 4 19 C 4 17.83 5.92 15.5 13.6 15.5 V 19 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 20 12 L 13.6 5 V 8.5 C 10.4 8.5 4 10.6 4 19 C 4 17.83 5.92 15.5 13.6 15.5 V 19 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16 6 A 4 4 0 0 0 12 2 H 6 A 4 4 0 0 0 2 6 V 12 A 4 4 0 0 0 6 16 H 7 V 11.5 C 7 9.74 7.75 8.59 8.71 7.89 C 9.64 7.22 10.75 7 11.5 7 H 16 V 6 z M 16.5 8 H 11.5 C 10.91 8 10.02 8.18 9.29 8.71 C 8.59 9.21 8 10.06 8 11.5 V 18 A 4 4 0 0 0 12 22 H 18 A 4 4 0 0 0 22 18 V 12 A 4 4 0 0 0 18 8 H 16.5 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11 5 A 1 1 0 0 0 10 6 H 14 A 1 1 0 0 0 13 5 H 11 z M 11 3 A 3 3 0 0 0 8 6 H 4 A 1 1 0 0 0 4 8 H 5 V 18 A 3 3 0 0 0 8 21 H 16 A 3 3 0 0 0 19 18 V 8 H 20 A 1 1 0 1 0 20 6 H 16 A 3 3 0 0 0 13 3 H 11 z M 11 11 A 1 1 0 1 0 9 11 V 16 A 1 1 0 1 0 11 16 V 11 z M 15 11 A 1 1 0 1 0 13 11 V 16 A 1 1 0 1 0 15 16 V 11 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 2 C 6.48 2 2 6.48 2 12 S 6.48 22 12 22 S 22 17.52 22 12 S 17.52 2 12 2 M 12 16 A 1 1 0 0 0 13 15 V 12 A 1 1 0 1 0 11 12 V 15 A 1 1 0 0 0 12 16 M 13 9 A 1 1 0 1 1 11 9 A 1 1 0 1 1 13 9", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MAJESTICONS_SOLID_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19 22 A 3 3 0 0 0 22 19 V 15 A 3 3 0 0 0 19.64 12.07 L 13 18.71 V 19.5 C 13 20.08 12.83 20.94 12.31 21.67 A 3.21 3.21 0 0 1 12.03 22 H 19 z M 13 17.29 L 18.74 11.55 A 3 3 0 0 0 18.29 7.88 L 16.12 5.71 A 3 3 0 0 0 13 5 V 17.29 z M 9 2 A 3 3 0 0 1 12 5 V 19.5 C 12 19.92 11.87 20.56 11.49 21.08 C 11.14 21.58 10.54 22 9.5 22 H 5 A 3 3 0 0 1 2 19 V 5 A 3 3 0 0 1 5 2 H 9 z M 7 18 A 1 1 0 1 0 7 16 A 1 1 0 0 0 7 18 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 11 V 13 H 4 V 11 z M 8 13 V 15 H 6 V 13 z M 10 15 V 17 H 8 V 15 z M 12 17 V 19 H 10 V 17 z M 8 11 V 9 H 6 V 11 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 10 15 V 7 H 8 V 15 z M 12 17 V 5 H 10 V 17 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 11 V 13 H 20 V 11 z M 16 13 V 15 H 18 V 13 z M 14 15 V 17 H 16 V 15 z M 12 17 V 19 H 14 V 17 z M 16 11 V 9 H 18 V 11 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 14 15 V 7 H 16 V 15 z M 12 17 V 5 H 14 V 17 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 4 H 20 V 2 H 4 z M 11 22 H 13 V 6 H 11 z M 13 10 H 15 V 8 H 13 z M 15 12 H 17 V 10 H 15 z M 17 14 H 19 V 12 H 17 z M 9 10 H 11 V 8 H 9 z M 7 12 H 9 V 10 H 7 z M 5 14 H 7 V 12 H 5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 22 22 H 20 V 20 H 22 z M 20 20 H 18 V 18 H 20 z M 14 18 H 6 V 16 H 14 z M 18 18 H 16 V 16 H 18 z M 6 16 H 4 V 14 H 6 z M 16 16 H 14 V 14 H 16 z M 4 14 H 2 V 6 H 4 z M 18 14 H 16 V 6 H 18 z M 6 6 H 4 V 4 H 6 z M 16 6 H 14 V 4 H 16 z M 14 4 H 6 V 2 H 14 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 16 H 6 V 18 H 8 V 20 H 10 V 22 H 2 V 14 H 4 z M 12 20 H 10 V 18 H 12 z M 14 18 H 12 V 16 H 14 z M 10 16 H 8 V 14 H 10 z M 16 16 H 14 V 14 H 16 z M 6 14 H 4 V 12 H 6 z M 12 14 H 10 V 12 H 12 z M 18 14 H 16 V 12 H 18 z M 8 12 H 6 V 10 H 8 z M 14 12 H 12 V 10 H 14 z M 20 12 H 18 V 10 H 20 z M 10 10 H 8 V 8 H 10 z M 18 10 H 16 V 8 H 18 z M 22 10 H 20 V 8 H 22 z M 12 8 H 10 V 6 H 12 z M 16 8 H 14 V 6 H 16 z M 20 8 H 18 V 6 H 20 z M 14 6 H 12 V 4 H 14 z M 18 6 H 16 V 4 H 18 z M 16 4 H 14 V 2 H 16 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13 2 H 15 V 4 H 17 V 6 H 13 V 11 H 18 V 7 H 20 V 9 H 22 V 11 H 24 V 13 H 22 V 15 H 20 V 17 H 18 V 13 H 13 V 18 H 17 V 20 H 15 V 22 H 13 V 24 H 11 V 22 H 9 V 20 H 7 V 18 H 11 V 13 H 6 V 17 H 4 V 15 H 2 V 13 H 0 V 11 H 2 V 9 H 4 V 7 H 6 V 11 H 11 V 6 H 7 V 4 H 9 V 2 H 11 V 0 H 13 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 6 H 20 V 8 H 8 z M 4 2 H 16 V 4 H 4 z M 6 8 H 8 V 20 H 6 z M 2 4 H 4 V 16 H 2 z M 8 20 H 20 V 22 H 8 z M 20 8 H 22 V 20 H 20 z M 16 4 H 18 V 6 H 16 z M 4 16 H 6 V 18 H 4 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18 22 H 6 V 20 H 18 z M 9 6 H 15 V 4 H 17 V 6 H 22 V 8 H 20 V 20 H 18 V 8 H 6 V 20 H 4 V 8 H 2 V 6 H 7 V 4 H 9 z M 15 4 H 9 V 2 H 15 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 2 H 20 V 4 H 4 z M 4 20 H 20 V 22 H 4 z M 2 4 H 4 V 20 H 2 z M 20 4 H 22 V 20 H 20 z M 11 9 H 13 V 7 H 11 z M 11 17 H 13 V 11 H 11 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PIXELARTICONS_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 2 H 17 V 4 H 7 z M 5 4 H 7 V 14 H 5 z M 17 2 H 19 V 14 H 17 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 13 2 H 15 V 8 H 13 z M 9 2 H 11 V 6 H 9 z M 5 10 H 19 V 12 H 5 z M 7 14 H 17 V 16 H 7 z M 9 16 H 11 V 20 H 9 z M 13 16 H 15 V 20 H 13 z M 9 20 H 15 V 22 H 9 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10.78 19.03 A 0.75 0.75 0 0 1 9.72 19.03 L 3.47 12.78 A 0.75 0.75 0 0 1 3.47 11.72 L 9.72 5.47 A 0.75 0.75 0 0 1 10.99 5.8 A 0.75 0.75 0 0 1 10.78 6.53 L 5.81 11.5 H 20.25 A 0.75 0.75 0 0 1 20.25 13 H 5.81 L 10.78 17.97 A 0.75 0.75 0 0 1 10.78 19.03", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13.22 19.03 A 0.75 0.75 0 0 1 13.22 17.97 L 18.19 13 H 3.75 A 0.75 0.75 0 0 1 3.75 11.5 H 18.19 L 13.22 6.53 A 0.75 0.75 0 0 1 13.55 5.26 A 0.75 0.75 0 0 1 14.28 5.47 L 20.53 11.72 A 0.75 0.75 0 0 1 20.53 12.78 L 14.28 19.03 A 0.75 0.75 0 0 1 13.22 19.03", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18.66 10.4 A 0.75 0.75 0 0 1 17.6 10.4 L 12.63 5.43 V 19.88 A 0.75 0.75 0 0 1 11.13 19.88 V 5.43 L 6.16 10.4 A 0.75 0.75 0 0 1 4.88 10.08 A 0.75 0.75 0 0 1 5.1 9.34 L 11.35 3.09 A 0.75 0.75 0 0 1 12.41 3.09 L 18.66 9.34 A 0.75 0.75 0 0 1 18.66 10.4", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10.25 2 A 8.25 8.25 0 0 1 16.59 15.53 L 22.28 21.22 A 0.75 0.75 0 0 1 21.95 22.49 A 0.75 0.75 0 0 1 21.22 22.28 L 15.53 16.59 A 8.25 8.25 0 1 1 10.25 2 M 3.5 10.25 A 6.75 6.75 0 1 0 17 10.25 A 6.75 6.75 0 0 0 3.5 10.25", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17.26 2.18 A 1.75 1.75 0 0 1 19.74 2.18 L 22.32 4.76 A 1.75 1.75 0 0 1 22.32 7.24 L 19.53 10.03 L 19.52 10.04 L 8.69 20.38 A 1.75 1.75 0 0 1 7.99 20.79 L 2.47 22.47 A 0.75 0.75 0 0 1 1.72 22.28 A 0.75 0.75 0 0 1 1.53 21.53 L 3.21 16.03 A 1.75 1.75 0 0 1 3.67 15.28 L 14.48 4.96 Z M 4.71 16.36 A 0.26 0.26 0 0 0 4.64 16.47 L 3.38 20.62 L 7.55 19.35 A 0.25 0.25 0 0 0 7.65 19.29 L 17.93 9.49 L 14.99 6.55 z M 19 8.44 L 21.26 6.18 A 0.25 0.25 0 0 0 21.26 5.82 L 18.68 3.24 A 0.25 0.25 0 0 0 18.32 3.24 L 16.06 5.5 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.5 9.75 V 20.25 C 5.5 20.39 5.61 20.5 5.75 20.5 H 18.25 A 0.25 0.25 0 0 0 18.5 20.25 V 9.75 A 0.25 0.25 0 0 0 18.25 9.5 H 15.75 A 0.75 0.75 0 0 1 15.75 8 H 18.25 C 19.22 8 20 8.78 20 9.75 V 20.25 A 1.75 1.75 0 0 1 18.25 22 H 5.75 A 1.75 1.75 0 0 1 4 20.25 V 9.75 C 4 8.78 4.78 8 5.75 8 H 8.25 A 0.75 0.75 0 0 1 8.25 9.5 H 5.75 A 0.25 0.25 0 0 0 5.5 9.75 M 12.53 1.22 L 15.78 4.47 A 0.75 0.75 0 0 1 15.45 5.75 A 0.75 0.75 0 0 1 14.72 5.53 L 12.75 3.56 V 14.25 A 0.75 0.75 0 0 1 11.25 14.25 V 3.56 L 9.28 5.53 A 0.75 0.75 0 0 1 8.24 5.51 A 0.75 0.75 0 0 1 8.22 4.47 L 11.47 1.22 A 0.75 0.75 0 0 1 12.53 1.22", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.02 3.75 C 7.02 2.78 7.81 2 8.77 2 H 20.25 C 21.22 2 22 2.78 22 3.75 V 15.25 A 1.75 1.75 0 0 1 20.25 17 H 8.77 A 1.75 1.75 0 0 1 7.02 15.25 Z M 8.77 3.5 A 0.25 0.25 0 0 0 8.52 3.75 V 15.25 C 8.52 15.39 8.64 15.5 8.77 15.5 H 20.25 A 0.25 0.25 0 0 0 20.5 15.25 V 3.75 A 0.25 0.25 0 0 0 20.25 3.5 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 2 10.75 A 1.75 1.75 0 0 1 3.75 9 H 5.25 A 0.75 0.75 0 1 1 5.25 10.5 H 3.75 A 0.25 0.25 0 0 0 3.5 10.75 L 3.5 20.25 C 3.5 20.39 3.61 20.5 3.75 20.5 H 13.25 A 0.25 0.25 0 0 0 13.5 20.25 V 18.74 A 0.75 0.75 0 1 1 15 18.74 V 20.25 A 1.75 1.75 0 0 1 13.25 22 H 3.75 A 1.75 1.75 0 0 1 2 20.25 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16 1.75 V 3 H 21.25 A 0.75 0.75 0 0 1 21.25 4.5 H 2.75 A 0.75 0.75 0 0 1 2.75 3 H 8 V 1.75 C 8 0.78 8.78 0 9.75 0 H 14.25 C 15.22 0 16 0.78 16 1.75 M 9.5 1.75 V 3 H 14.5 V 1.75 A 0.25 0.25 0 0 0 14.25 1.5 H 9.75 A 0.25 0.25 0 0 0 9.5 1.75 M 5 6.18 A 0.75 0.75 0 1 0 3.5 6.32 L 4.92 20.92 A 1.75 1.75 0 0 0 6.66 22.5 H 17.34 A 1.75 1.75 0 0 0 19.08 20.92 L 20.5 6.32 A 0.75 0.75 0 0 0 19 6.18 L 17.59 20.77 A 0.25 0.25 0 0 1 17.34 21 H 6.66 A 0.25 0.25 0 0 1 6.41 20.77 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 9.21 7.5 A 0.75 0.75 0 0 1 10 8.21 L 10.5 16.71 A 0.75 0.75 0 1 1 9 16.79 L 8.5 8.29 A 0.75 0.75 0 0 1 9.21 7.5 Z M 15.5 8.29 A 0.75 0.75 0 1 0 14 8.21 L 13.5 16.71 A 0.75 0.75 0 0 0 15 16.79 L 15.5 8.29 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13 7.5 A 1 1 0 1 1 11 7.5 A 1 1 0 0 1 13 7.5 M 10 11.25 A 0.75 0.75 0 0 1 10.75 10.5 H 12.25 A 0.75 0.75 0 0 1 13 11.25 V 15.5 H 13.75 A 0.75 0.75 0 0 1 13.75 17 H 10.75 A 0.75 0.75 0 0 1 10.75 15.5 H 11.5 V 12 H 10.75 A 0.75 0.75 0 0 1 10 11.25", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 1 C 18.07 1 23 5.92 23 12 S 18.07 23 12 23 S 1 18.07 1 12 S 5.92 1 12 1 M 2.5 12 A 9.5 9.5 0 0 0 12 21.5 A 9.5 9.5 0 0 0 21.5 12 A 9.5 9.5 0 0 0 12 2.5 A 9.5 9.5 0 0 0 2.5 12", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_OCTICON_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19.64 6.65 C 18.33 8.46 16.39 10.51 14.7 12.23 L 14.61 12.32 L 11.92 9.63 L 12.01 9.53 C 13.7 7.82 15.75 5.88 17.56 4.57 C 18.47 3.91 19.27 3.46 19.91 3.25 C 20.56 3.05 20.83 3.16 20.95 3.29 C 21.06 3.4 21.18 3.65 20.97 4.3 C 20.76 4.94 20.3 5.74 19.64 6.65 M 10.9 10.74 L 13.51 13.34 C 12.62 14.1 11.82 14.64 11.13 15 A 4.83 4.83 0 0 0 9.24 13.11 C 9.6 12.42 10.14 11.62 10.9 10.74 M 7.81 12.59 A 4.83 4.83 0 0 0 3.48 13.92 C 2.96 14.45 2.6 15.22 2.35 16 C 2.1 16.79 1.92 17.68 1.81 18.51 A 19 19 0 0 0 1.64 20.72 C 1.64 21.02 1.64 21.29 1.66 21.5 C 1.67 21.6 1.68 21.71 1.71 21.81 C 1.72 21.86 1.73 21.92 1.76 21.99 C 1.78 22.04 1.83 22.17 1.95 22.29 C 2.07 22.4 2.19 22.46 2.25 22.48 C 2.31 22.5 2.38 22.52 2.42 22.53 C 2.52 22.55 2.63 22.56 2.73 22.57 C 2.95 22.59 3.21 22.6 3.51 22.59 C 4.12 22.59 4.9 22.53 5.73 22.42 C 6.55 22.31 7.44 22.14 8.24 21.88 C 9.01 21.63 9.79 21.28 10.32 20.75 A 4.83 4.83 0 0 0 11.64 16.42 C 12.85 15.82 14.26 14.8 15.76 13.28 C 17.46 11.57 19.47 9.45 20.86 7.53 C 21.54 6.58 22.11 5.63 22.39 4.77 C 22.67 3.93 22.73 2.94 22.01 2.22 C 21.29 1.51 20.31 1.55 19.46 1.82 C 18.59 2.1 17.63 2.67 16.68 3.35 C 14.76 4.74 12.64 6.76 10.95 8.48 C 9.45 10 8.42 11.37 7.81 12.59 M 9.25 14.98 A 3.33 3.33 0 0 1 9.25 19.69 C 8.98 19.96 8.48 20.23 7.78 20.45 C 7.09 20.67 6.3 20.83 5.53 20.94 C 4.76 21.04 4.03 21.09 3.49 21.1 A 9 9 0 0 1 3.14 21.09 A 9 9 0 0 1 3.14 20.74 C 3.15 20.2 3.2 19.48 3.3 18.71 C 3.4 17.94 3.56 17.14 3.78 16.46 C 4.01 15.76 4.27 15.25 4.55 14.98 A 3.33 3.33 0 0 1 9.26 14.98 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 6 L 6 12 L 12 18 M 18 6 L 12 12 L 18 18", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 18 L 18 12 L 12 6 M 6 18 L 12 12 L 6 6", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 19.5 V 4.5 M 18 10.12 L 12 4.5 L 6 10.12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19 11.5 A 7.5 7.5 0 1 1 4 11.5 A 7.5 7.5 0 0 1 19 11.5 M 16.89 16.92 L 19.97 20", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18.77 10.22 L 8.8 20.2 A 2.73 2.73 0 0 1 6.87 21 H 3 V 17.16 C 3 16.43 3.29 15.74 3.8 15.22 M 8.8 20.2 L 3.8 15.22 M 3.8 15.22 L 13.78 5.23 M 13.78 5.23 L 15.21 3.8 A 2.73 2.73 0 0 1 19.08 3.8 L 20.2 4.93 A 2.73 2.73 0 0 1 20.2 8.79 L 18.77 10.22 M 13.78 5.23 L 18.77 10.22", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.5 12 H 16.5 M 13 8.5 L 16.5 12 L 13 15.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 3 9.4 C 3 7.16 3 6.04 3.44 5.18 A 4 4 0 0 1 5.18 3.44 C 6.04 3 7.16 3 9.4 3 H 14.6 C 16.84 3 17.96 3 18.82 3.44 A 4 4 0 0 1 20.56 5.18 C 21 6.04 21 7.16 21 9.4 V 14.6 C 21 16.84 21 17.96 20.56 18.82 A 4 4 0 0 1 18.82 20.56 C 17.96 21 16.84 21 14.6 21 H 9.4 C 7.16 21 6.04 21 5.18 20.56 A 4 4 0 0 1 3.44 18.82 C 3 17.96 3 16.84 3 14.6 z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20.83 12.86 C 21 12.45 21 11.92 21 10.88 S 21 9.3 20.83 8.89 A 2.25 2.25 0 0 0 19.61 7.67 C 19.2 7.5 18.67 7.5 17.62 7.5 H 11.1 C 9.84 7.5 9.21 7.5 8.73 7.74 A 2.25 2.25 0 0 0 7.74 8.73 C 7.5 9.21 7.5 9.84 7.5 11.1 V 17.62 C 7.5 18.67 7.5 19.2 7.67 19.61 C 7.9 20.16 8.34 20.6 8.89 20.83 C 9.3 21 9.83 21 10.88 21 S 12.45 21 12.86 20.83 M 20.83 12.86 A 2.25 2.25 0 0 1 19.61 14.08 C 19.2 14.25 18.67 14.25 17.62 14.25 S 16.05 14.25 15.64 14.42 A 2.25 2.25 0 0 0 14.42 15.64 C 14.25 16.05 14.25 16.58 14.25 17.62 S 14.25 19.2 14.08 19.61 A 2.25 2.25 0 0 1 12.86 20.83 M 20.83 12.86 A 11.68 11.68 0 0 1 13.08 20.76 L 12.86 20.83 M 16.5 7.5 V 6.6 C 16.5 5.34 16.5 4.71 16.25 4.23 A 2.25 2.25 0 0 0 15.27 3.24 C 14.79 3 14.16 3 12.9 3 H 6.6 C 5.34 3 4.71 3 4.23 3.25 A 2.25 2.25 0 0 0 3.24 4.23 C 3 4.71 3 5.34 3 6.6 V 12.9 C 3 14.16 3 14.79 3.25 15.27 C 3.46 15.69 3.81 16.04 4.23 16.25 C 4.71 16.5 5.34 16.5 6.6 16.5 H 7.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18 9 L 17.16 17.4 C 17.03 18.67 16.97 19.31 16.68 19.79 A 2.5 2.5 0 0 1 15.61 20.76 C 15.1 21 14.46 21 13.18 21 H 10.82 C 9.54 21 8.9 21 8.39 20.76 A 2.5 2.5 0 0 1 7.32 19.79 C 7.03 19.31 6.97 18.67 6.84 17.4 L 6 9 M 13.5 15.5 V 10.5 M 10.5 15.5 V 10.5 M 4.5 6.5 H 9.12 M 9.12 6.5 L 9.5 3.83 C 9.61 3.34 10.02 3 10.48 3 H 13.52 C 13.98 3 14.39 3.34 14.5 3.83 L 14.88 6.5 M 9.12 6.5 H 14.88 M 14.88 6.5 H 19.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 12 A 9 9 0 1 1 3 12 A 9 9 0 0 1 21 12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 16 V 11 H 11.5 M 11.5 16 H 12.5 M 12 8.5 V 8", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_OUTLINE_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16.5 5.5 H 17.5 C 18.61 5.5 19.5 5.89 19.5 7 V 9 A 2 2 0 0 1 17.5 11 H 10.5 A 1 1 0 0 0 9.5 12 V 20 A 1 1 0 0 0 10.5 21 H 11.5 M 16.5 5.5 V 4 A 1 1 0 0 0 15.5 3 H 5.5 A 1 1 0 0 0 4.5 4 V 7 A 1 1 0 0 0 5.5 8 H 15.5 A 1 1 0 0 0 16.5 7 z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.5 5.47 A 0.75 0.75 0 0 0 11.44 5.47 L 5.44 11.47 A 0.75 0.75 0 0 0 5.44 12.53 L 11.44 18.53 A 0.75 0.75 0 0 0 12.5 18.53 V 13.59 L 17.44 18.53 A 0.75 0.75 0 0 0 18.5 18.53 V 5.47 A 0.75 0.75 0 0 0 17.44 5.47 L 12.5 10.41 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.47 5.47 A 0.75 0.75 0 0 1 12.53 5.47 L 18.53 11.47 A 0.75 0.75 0 0 1 18.53 12.53 L 12.53 18.53 A 0.75 0.75 0 0 1 11.47 18.53 V 13.59 L 6.53 18.53 A 0.75 0.75 0 0 1 5.47 18.53 V 5.47 A 0.75 0.75 0 0 1 6.53 5.47 L 11.47 10.41 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18.51 9.58 A 0.75 0.75 0 0 1 18.55 10.64 H 12.75 V 19.5 A 0.75 0.75 0 0 1 11.25 19.5 V 10.64 H 5.45 A 0.75 0.75 0 0 1 5.49 9.58 L 11.49 3.95 A 0.75 0.75 0 0 1 12.51 3.95 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.5 3.25 A 8.25 8.25 0 1 0 16.77 17.85 L 19.44 20.53 A 0.75 0.75 0 0 0 20.5 19.47 L 17.83 16.79 A 8.25 8.25 0 0 0 11.5 3.25", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14.68 3.27 A 3.48 3.48 0 0 1 19.61 3.27 L 20.73 4.4 A 3.48 3.48 0 0 1 20.73 9.32 L 9.33 20.73 A 3.48 3.48 0 0 1 6.87 21.75 H 3 A 0.75 0.75 0 0 1 2.25 21 V 17.16 A 3.48 3.48 0 0 1 3.27 14.69 z M 18.55 4.33 A 1.98 1.98 0 0 0 15.74 4.33 L 14.84 5.23 L 18.77 9.16 L 19.67 8.26 A 1.98 1.98 0 0 0 19.67 5.46 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9.37 2.25 H 14.63 C 15.73 2.25 16.59 2.25 17.29 2.31 C 18 2.36 18.61 2.49 19.16 2.77 A 4.75 4.75 0 0 1 21.23 4.84 C 21.51 5.39 21.63 6 21.69 6.71 C 21.75 7.41 21.75 8.28 21.75 9.37 V 14.63 C 21.75 15.72 21.75 16.59 21.69 17.29 C 21.63 18 21.51 18.61 21.23 19.16 A 4.75 4.75 0 0 1 19.16 21.23 C 18.6 21.51 18 21.63 17.29 21.69 C 16.59 21.75 15.72 21.75 14.63 21.75 H 9.37 C 8.28 21.75 7.41 21.75 6.71 21.69 C 6 21.63 5.4 21.51 4.84 21.23 A 4.75 4.75 0 0 1 2.77 19.16 C 2.49 18.6 2.37 18 2.31 17.29 C 2.25 16.59 2.25 15.72 2.25 14.63 V 9.37 C 2.25 8.28 2.25 7.41 2.31 6.71 C 2.37 6 2.49 5.4 2.77 4.84 A 4.75 4.75 0 0 1 4.84 2.77 C 5.39 2.49 6 2.37 6.71 2.31 C 7.41 2.25 8.28 2.25 9.37 2.25 M 12.47 7.97 A 0.75 0.75 0 0 0 12.47 9.03 L 14.69 11.25 H 7.5 A 0.75 0.75 0 0 0 7.5 12.75 H 14.69 L 12.47 14.97 A 0.75 0.75 0 1 0 13.53 16.03 L 17.03 12.53 A 0.75 0.75 0 0 0 17.03 11.47 L 13.53 7.97 A 0.75 0.75 0 0 0 12.47 7.97", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6.57 2.25 H 12.93 C 13.54 2.25 14.03 2.25 14.44 2.28 C 14.86 2.32 15.25 2.39 15.61 2.58 A 3 3 0 0 1 16.92 3.89 C 17.11 4.25 17.18 4.64 17.22 5.06 C 17.25 5.47 17.25 5.96 17.25 6.57 V 6.75 H 17.65 C 18.15 6.75 18.57 6.75 18.91 6.77 C 19.26 6.8 19.59 6.85 19.9 6.98 A 3 3 0 0 1 21.52 8.6 C 21.65 8.92 21.7 9.24 21.73 9.59 C 21.75 9.93 21.75 10.35 21.75 10.85 V 10.9 C 21.75 11.4 21.75 11.82 21.73 12.16 A 3.2 3.2 0 0 1 21.54 13.11 A 12.43 12.43 0 0 1 13.3 21.48 L 13.11 21.54 A 3.1 3.1 0 0 1 12.16 21.73 C 11.82 21.75 11.4 21.75 10.9 21.75 H 10.85 C 10.35 21.75 9.93 21.75 9.59 21.73 C 9.24 21.7 8.91 21.65 8.6 21.52 A 3 3 0 0 1 6.98 19.9 C 6.85 19.59 6.8 19.26 6.77 18.91 C 6.75 18.57 6.75 18.15 6.75 17.65 V 17.25 H 6.57 C 5.97 17.25 5.47 17.25 5.06 17.22 C 4.64 17.18 4.25 17.11 3.89 16.92 A 3 3 0 0 1 2.58 15.61 C 2.39 15.25 2.32 14.86 2.28 14.44 C 2.25 14.03 2.25 13.54 2.25 12.93 V 6.57 C 2.25 5.96 2.25 5.47 2.28 5.06 C 2.32 4.64 2.39 4.25 2.58 3.89 A 3 3 0 0 1 3.89 2.58 C 4.25 2.39 4.64 2.32 5.06 2.28 C 5.47 2.25 5.96 2.25 6.57 2.25 M 8.25 17.62 C 8.25 18.16 8.25 18.52 8.27 18.8 C 8.29 19.08 8.32 19.22 8.36 19.32 A 1.5 1.5 0 0 0 9.18 20.14 C 9.28 20.18 9.42 20.21 9.69 20.23 C 9.98 20.25 10.34 20.25 10.87 20.25 S 11.77 20.25 12.05 20.23 C 12.33 20.21 12.47 20.18 12.57 20.14 A 1.5 1.5 0 0 0 13.39 19.32 C 13.43 19.22 13.46 19.08 13.48 18.8 C 13.5 18.52 13.5 18.16 13.5 17.62 V 17.6 C 13.5 17.1 13.5 16.68 13.52 16.34 C 13.55 15.99 13.6 15.66 13.73 15.35 A 3 3 0 0 1 15.35 13.73 C 15.66 13.6 15.99 13.55 16.34 13.52 C 16.68 13.5 17.1 13.5 17.6 13.5 H 17.62 C 18.16 13.5 18.52 13.5 18.8 13.48 C 19.08 13.46 19.22 13.43 19.32 13.39 A 1.5 1.5 0 0 0 20.14 12.57 C 20.18 12.47 20.21 12.33 20.23 12.06 C 20.25 11.77 20.25 11.41 20.25 10.88 S 20.25 9.98 20.23 9.7 C 20.21 9.42 20.18 9.28 20.14 9.18 A 1.5 1.5 0 0 0 19.32 8.36 C 19.22 8.32 19.08 8.29 18.8 8.27 C 18.52 8.25 18.16 8.25 17.62 8.25 H 11.1 C 10.46 8.25 10.02 8.25 9.68 8.28 C 9.35 8.31 9.19 8.35 9.07 8.41 A 1.5 1.5 0 0 0 8.41 9.07 C 8.35 9.19 8.3 9.35 8.28 9.68 C 8.25 10.02 8.25 10.46 8.25 11.1 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.24 3.72 L 15.53 5.75 H 19.5 A 0.75 0.75 0 0 1 19.5 7.25 H 18.73 L 17.86 17.44 C 17.8 18.06 17.76 18.57 17.69 18.98 C 17.62 19.41 17.52 19.79 17.31 20.15 A 3.25 3.25 0 0 1 15.91 21.43 C 15.53 21.61 15.14 21.68 14.71 21.72 C 14.29 21.75 13.78 21.75 13.16 21.75 H 10.84 C 10.22 21.75 9.71 21.75 9.29 21.72 C 8.86 21.68 8.47 21.61 8.09 21.43 A 3.25 3.25 0 0 1 6.69 20.15 C 6.48 19.79 6.38 19.41 6.31 18.98 C 6.24 18.56 6.2 18.06 6.14 17.44 L 5.27 7.25 H 4.5 A 0.75 0.75 0 0 1 4.5 5.75 H 8.47 L 8.76 3.72 L 8.77 3.66 C 8.95 2.87 9.63 2.25 10.48 2.25 H 13.52 C 14.37 2.25 15.05 2.87 15.23 3.66 z M 9.98 5.75 H 14.02 L 13.76 3.97 C 13.71 3.81 13.59 3.75 13.52 3.75 H 10.48 C 10.41 3.75 10.29 3.81 10.24 3.97 z M 11.25 10.5 A 0.75 0.75 0 0 0 9.75 10.5 V 15.5 A 0.75 0.75 0 0 0 11.25 15.5 z M 14.25 10.5 A 0.75 0.75 0 0 0 12.75 10.5 V 15.5 A 0.75 0.75 0 0 0 14.25 15.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 2.25 C 6.62 2.25 2.25 6.62 2.25 12 S 6.62 21.75 12 21.75 S 21.75 17.38 21.75 12 S 17.39 2.25 12 2.25 M 12 7.25 A 0.75 0.75 0 0 1 12.75 8 V 8.5 A 0.75 0.75 0 0 1 11.25 8.5 V 8 A 0.75 0.75 0 0 1 12 7.25 M 10.75 16 A 0.75 0.75 0 0 1 11.25 15.29 V 11.71 A 0.75 0.75 0 0 1 11.5 10.25 H 12 A 0.75 0.75 0 0 1 12.75 11 V 15.29 A 0.75 0.75 0 0 1 12.5 16.75 H 11.5 A 0.75 0.75 0 0 1 10.75 16", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MYNAUI_SOLID_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.5 2.25 A 1.75 1.75 0 0 0 3.75 4 V 7 C 3.75 7.97 4.53 8.75 5.5 8.75 H 15.5 A 1.75 1.75 0 0 0 17.25 7 V 6.25 H 17.5 C 17.98 6.25 18.32 6.34 18.5 6.46 C 18.64 6.55 18.75 6.69 18.75 7 V 9 C 18.75 9.69 18.19 10.25 17.5 10.25 H 10.5 A 1.75 1.75 0 0 0 8.75 12 V 20 C 8.75 20.97 9.53 21.75 10.5 21.75 H 11.5 A 0.75 0.75 0 0 0 11.5 20.25 H 10.5 A 0.25 0.25 0 0 1 10.25 20 V 12 A 0.25 0.25 0 0 1 10.5 11.75 H 17.5 A 2.75 2.75 0 0 0 20.25 9 V 7 C 20.25 6.21 19.91 5.59 19.32 5.21 C 18.79 4.86 18.12 4.75 17.5 4.75 H 17.25 V 4 A 1.75 1.75 0 0 0 15.5 2.25 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8.06 12.63 C 8.31 13.84 9.67 14.71 12.37 16.45 C 15.32 18.34 16.79 19.29 17.98 18.92 C 18.39 18.8 18.76 18.58 19.07 18.29 C 19.99 17.42 19.99 15.61 19.99 12 S 19.99 6.58 19.07 5.71 A 2.8 2.8 0 0 0 17.98 5.08 C 16.79 4.71 15.32 5.66 12.37 7.55 C 9.66 9.29 8.31 10.16 8.06 11.37 A 3.1 3.1 0 0 0 8.06 12.63 Z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 3.99 4 V 20", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.93 12.63 C 15.67 13.84 14.32 14.71 11.61 16.45 C 8.67 18.34 7.19 19.29 6 18.92 A 2.8 2.8 0 0 1 4.91 18.29 C 3.99 17.42 3.99 15.61 3.99 12 S 3.99 6.58 4.91 5.71 A 2.8 2.8 0 0 1 6 5.08 C 7.19 4.71 8.67 5.66 11.61 7.55 C 14.32 9.29 15.67 10.16 15.93 11.37 C 16.01 11.79 16.01 12.21 15.93 12.63 Z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 19.99 5 V 19", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18 15 S 13.58 9 12 9 S 6 15 6 15", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17 17 L 21 21 M 19 11 A 8 8 0 1 0 3 11 A 8 8 0 0 0 19 11 M 7.5 11 H 14.5 M 11 7.5 V 14.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.99 16 L 8.72 13.1 A 3.96 3.96 0 0 1 9.76 11.26 L 16.42 4.6 L 17.41 3.61 A 2.1 2.1 0 0 1 20.38 6.58 L 19.39 7.57 L 12.73 14.23 A 3.96 3.96 0 0 1 10.89 15.28 z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 18.99 13.5 C 18.99 16.79 18.99 18.43 18.08 19.54 A 4 4 0 0 1 17.53 20.09 C 16.42 21 14.78 21 11.49 21 H 10.99 C 7.22 21 5.34 21 4.16 19.83 S 2.99 16.77 2.99 13 V 12.5 C 2.99 9.21 2.99 7.57 3.9 6.46 A 4 4 0 0 1 4.46 5.91 C 5.56 5 7.21 5 10.49 5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 7 H 16.75 C 18.86 7 19.91 7 20.67 7.51 A 3 3 0 0 1 21.49 8.33 C 22 9.09 22 10.14 22 12.25 V 13 M 12 7 L 11.37 5.73 C 10.84 4.68 10.36 3.63 9.2 3.19 C 8.69 3 8.11 3 6.94 3 C 5.13 3 4.22 3 3.54 3.38 A 3 3 0 0 0 2.38 4.54 C 2 5.22 2 6.13 2 7.94 V 11 C 2 15.71 2 18.07 3.46 19.54 C 4.82 20.89 6.94 20.99 11 21", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 19.5 16 C 20.01 16.49 22 17.8 22 18.5 M 19.5 21 C 20.01 20.51 22 19.2 22 18.5 M 22 18.5 H 14", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 11 V 9 C 7 5.7 7 4.05 8.03 3.03 S 10.7 2 14 2 S 18.95 2 19.98 3.02 S 21 5.7 21 9 V 11 C 21 14.3 21 15.95 19.98 16.98 S 17.3 18 14 18 S 9.05 18 8.03 16.98 S 7 14.3 7 11", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 3 6 V 15 C 3 18.3 3 19.95 4.03 20.98 S 6.7 22 10 22 H 17", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19.5 5.5 L 18.88 15.53 C 18.72 18.09 18.64 19.37 18 20.29 A 4 4 0 0 1 16.8 21.42 C 15.84 22 14.56 22 11.99 22 C 9.42 22 8.14 22 7.18 21.41 A 4 4 0 0 1 5.98 20.29 C 5.34 19.36 5.26 18.08 5.11 15.52 L 4.5 5.5 M 3 5.5 H 21 M 16.06 5.5 L 15.37 4.09 C 14.92 3.16 14.69 2.69 14.3 2.4 A 2 2 0 0 0 14.03 2.23 C 13.59 2 13.07 2 12.04 2 C 10.97 2 10.44 2 10 2.23 A 2 2 0 0 0 9.72 2.41 C 9.32 2.72 9.1 3.2 8.66 4.17 L 8.05 5.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 12 A 10 10 0 1 0 22 12 A 10 10 0 1 0 2 12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 16 V 12 M 11.88 8.25 H 12 M 11.75 8.25 A 0.25 0.25 0 1 1 12.25 8.25 A 0.25 0.25 0 0 1 11.75 8.25", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_HUGEICONS_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21.82 10.41 C 22.06 11.85 22.18 12.57 21.24 13.93 C 21.15 14.07 20.9 14.37 20.78 14.49 C 19.61 15.66 18.43 15.66 16.07 15.66 H 14.14 C 13.51 15.66 13.19 15.66 12.96 15.71 A 2.02 2.02 0 0 0 11.45 17.98 C 11.49 18.21 11.6 18.51 11.84 19.1 C 11.93 19.34 11.98 19.47 12.01 19.73 C 12.13 20.71 11.09 21.95 10.12 21.99 C 9.85 22 9.84 22 9.8 21.99 A 9.93 9.93 0 0 1 4.18 18.44 C 0.74 14.08 1.43 7.71 5.74 4.22 S 16.32 1.43 19.77 5.79 A 10.14 10.14 0 0 1 21.82 10.41 Z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 7.37 7.75 H 7.24 M 7.49 7.75 A 0.25 0.25 0 1 1 6.99 7.75 A 0.25 0.25 0 0 1 7.49 7.75 M 7.37 15.75 H 7.24 M 7.49 15.75 A 0.25 0.25 0 1 1 6.99 15.75 A 0.25 0.25 0 0 1 7.49 15.75 M 11.87 5.75 H 11.74 M 11.99 5.75 A 0.25 0.25 0 1 1 11.49 5.75 A 0.25 0.25 0 0 1 11.99 5.75 M 16.37 7.75 H 16.24 M 16.49 7.75 A 0.25 0.25 0 1 1 15.99 7.75 A 0.25 0.25 0 0 1 16.49 7.75 M 18.37 11.75 H 18.24 M 18.49 11.75 A 0.25 0.25 0 1 1 17.99 11.75 A 0.25 0.25 0 0 1 18.49 11.75 M 5.87 11.75 H 5.74 M 5.99 11.75 A 0.25 0.25 0 1 1 5.49 11.75 A 0.25 0.25 0 0 1 5.99 11.75", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19 20 L 9 12 L 19 4 z M 5 19 V 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 4 L 15 12 L 5 20 z M 19 5 V 19", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 19 V 5 M 5 12 L 12 5 L 19 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 11 A 8 8 0 1 0 19 11 A 8 8 0 1 0 3 11", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 21 21 L 16.65 16.65", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11 4 H 4 A 2 2 0 0 0 2 6 V 20 A 2 2 0 0 0 4 22 H 18 A 2 2 0 0 0 20 20 V 13", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 18.5 2.5 A 2.12 2.12 0 0 1 21.5 5.5 L 12 15 L 8 16 L 9 12 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 9 L 2 12 L 5 15 M 9 5 L 12 2 L 15 5 M 15 19 L 12 22 L 9 19 M 19 9 L 22 12 L 19 15 M 2 12 H 22 M 12 2 V 22", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11 9 H 20 A 2 2 0 0 1 22 11 V 20 A 2 2 0 0 1 20 22 H 11 A 2 2 0 0 1 9 20 V 11 A 2 2 0 0 1 11 9 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 5 15 H 4 A 2 2 0 0 1 2 13 V 4 A 2 2 0 0 1 4 2 H 13 A 2 2 0 0 1 15 4 V 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 6 H 21 M 19 6 V 20 A 2 2 0 0 1 17 22 H 7 A 2 2 0 0 1 5 20 V 6 M 8 6 V 4 A 2 2 0 0 1 10 2 H 14 A 2 2 0 0 1 16 4 V 6", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 12 A 10 10 0 1 0 22 12 A 10 10 0 1 0 2 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 16 V 12 M 12 8 H 12.01", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FEATHER_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 12.79 A 9 9 0 1 1 11.21 3 A 7 7 0 0 0 21 12.79", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SYSTEM_UICONS_0: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 7.5 6.5 L 3.5 10.5 L 7.5 14.5 M 16.5 10.5 H 3.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SYSTEM_UICONS_1: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 13.5 6.5 L 17.5 10.5 L 13.5 14.5 M 4.5 10.5 H 17.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SYSTEM_UICONS_2: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 14.5 7.5 L 10.5 3.5 L 6.47 7.5 M 10.5 3.5 V 16.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SYSTEM_UICONS_3: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 3.5 8.5 A 5 5 0 1 0 13.5 8.5 A 5 5 0 1 0 3.5 8.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 17.57 17.5 L 12 12", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SYSTEM_UICONS_4: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 17 4 A 2.12 2.12 0 0 1 17 7 L 7.5 16.5 L 3.5 17.5 L 4.5 13.56 L 14 4 A 2.12 2.12 0 0 1 16.87 3.88 z M 15.5 6.5 L 16.5 7.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SYSTEM_UICONS_5: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 19.5 10.5 H 1.5 M 10.5 1.5 V 19.5 M 16.5 13.5 L 19.5 10.5 L 16.5 7.5 M 4.5 13.5 L 1.5 10.5 L 4.5 7.5 M 7.5 4.5 L 10.5 1.5 L 13.5 4.5 M 7.5 16.5 L 10.5 19.5 L 13.5 16.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SYSTEM_UICONS_6: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 18.5 12.5 V 4.5 A 2 2 0 0 0 16.5 2.5 H 8.5 A 2 2 0 0 0 6.5 4.5 V 12.5 A 2 2 0 0 0 8.5 14.5 H 16.5 A 2 2 0 0 0 18.5 12.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 6.5 6.5 H 4.5 A 2 2 0 0 0 2.5 8.5 V 16.5 A 2 2 0 0 0 4.5 18.5 H 4.5 L 12.5 18.49 A 2 2 0 0 0 14.5 16.49 V 14.5 M 12.5 5.5 V 11.5 M 15.5 8.5 H 9.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SYSTEM_UICONS_7: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 5.5 4.5 H 15.5 V 16.5 A 2 2 0 0 1 13.5 18.5 H 7.5 A 2 2 0 0 1 5.5 16.5 z M 10.5 2.5 A 2 2 0 0 1 12.5 4.35 L 12.5 4.5 H 8.5 A 2 2 0 0 1 10.5 2.5 M 3.5 4.5 H 17.5 M 8.5 7.5 V 15.5 M 12.5 7.5 V 15.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SYSTEM_UICONS_8: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 0.5 8.5 A 8 8 0 1 0 16.5 8.5 A 8 8 0 1 0 0.5 8.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 2.0, dy: 2.0 },
        Part { d: "M 8.5 12.5 V 8.5 H 7.5 M 7.5 12.5 H 9.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 2.0, dy: 2.0 },
        Part { d: "M 7.5 5.5 A 1 1 0 1 0 9.5 5.5 A 1 1 0 1 0 7.5 5.5", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 2.0, dy: 2.0 },
    ],
};
static G_SYSTEM_UICONS_9: Glyph = Glyph {
    grid: 21.0,
    parts: &[
        Part { d: "M 11.5 3.5 C 12.83 3.5 14.07 3.87 15.13 4.51 A 6 6 0 0 0 15.13 16.49 A 7 7 0 1 1 11.5 3.5", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 5 L 3 10 L 8 15", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 3 10 H 11 C 16.52 10 21 14.48 21 20 V 21", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16 5 L 21 10 L 16 15", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 21 10 H 13 C 7.48 10 3 14.48 3 20 V 21", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 20 V 4 M 5 11 L 12 4 L 19 11", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 21 L 16.51 16.51 M 19 10.5 A 8.5 8.5 0 1 1 2 10.5 A 8.5 8.5 0 0 1 19 10.5 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4.33 16.05 L 16.57 3.81 A 2.56 2.56 0 0 1 20.19 7.43 L 7.95 19.67 A 2 2 0 0 1 6.93 20.21 L 3 21 L 3.79 17.07 A 2 2 0 0 1 4.33 16.05", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 14.5 6.5 L 17.5 9.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 18 6 L 6 18 M 8 6 H 18 V 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 4 V 16 A 2 2 0 0 0 10 18 H 18 A 2 2 0 0 0 20 16 V 7.24 A 2 2 0 0 0 19.4 5.81 L 16.08 2.57 A 2 2 0 0 0 14.69 2 H 10 A 2 2 0 0 0 8 4", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 16 18 V 20 A 2 2 0 0 1 14 22 H 6 A 2 2 0 0 1 4 20 V 9 A 2 2 0 0 1 6 7 H 8", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 4 L 3.81 12.13 C 3.94 13.38 4.16 14.61 4.47 15.82 L 5.14 18.37 A 4 4 0 0 0 7.8 21.17 L 8.37 21.35 A 12 12 0 0 0 15.63 21.35 L 16.2 21.17 A 4 4 0 0 0 18.86 18.37 L 19.53 15.82 C 19.84 14.61 20.06 13.38 20.19 12.13 L 21 4", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 3 4 A 9 2 0 1 0 21 4 A 9 2 0 1 0 3 4", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 12 A 10 10 0 1 0 22 12 A 10 10 0 1 0 2 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 7 H 12.01", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 10 11 H 12 V 16 M 10 16 H 14", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_AKAR_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20.96 15.32 C 21.16 14.84 20.58 14.42 20.09 14.64 A 7.7 7.7 0 0 1 16.99 15.29 C 12.8 15.29 9.41 11.96 9.41 7.86 A 7.3 7.3 0 0 1 10.55 3.95 C 10.83 3.51 10.49 2.89 9.97 3.02 C 5.96 4.04 3 7.61 3 11.86 C 3 16.91 7.17 21 12.33 21 C 16.23 21 19.57 18.66 20.96 15.32 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 15.61 3.1 C 15.08 2.75 14.45 3.38 14.8 3.91 L 15.43 4.86 A 2.33 2.33 0 0 1 15.43 7.44 L 14.8 8.39 C 14.45 8.92 15.08 9.55 15.61 9.2 L 16.56 8.57 A 2.33 2.33 0 0 1 19.14 8.57 L 20.09 9.2 C 20.62 9.55 21.25 8.92 20.9 8.39 L 20.27 7.44 A 2.33 2.33 0 0 1 20.27 4.86 L 20.9 3.91 C 21.25 3.38 20.62 2.75 20.09 3.1 L 19.14 3.73 A 2.33 2.33 0 0 1 16.56 3.73 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11 6.5 L 5.5 12 L 11 17.5 M 6.75 12 H 19.75", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13 17.5 L 18.5 12 L 13 6.5 M 17.25 12 H 4.25", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17.5 10.5 L 12 5 L 6.5 10.5 M 12 6.25 V 19.25", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.8 15.8 L 21.11 21.11 M 15.8 15.8 A 7.5 7.5 0 1 1 5.2 5.2 A 7.5 7.5 0 0 1 15.8 15.8 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14.11 6 L 4.79 15.32 L 4 20 L 8.68 19.21 L 17.99 9.89 M 14.1 6 L 17.99 9.89 M 14.1 6 L 17.16 2.95 L 21.05 6.84 L 17.99 9.89", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 3.5 H 9 L 11 6 H 22 V 20 H 2 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12.61 15.5 L 15 13 L 12.61 10.5 M 13.75 13 H 9", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 10 H 21 V 21 H 10 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 14 6.5 V 3 H 3 V 14 H 6.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 5 H 3 M 5 5 H 19 L 18.5 22 H 5.5 z M 8.5 2 H 15.5 V 5 H 8.5 z M 12 9 V 18", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 12 C 2 6.48 6.48 2 12 2 S 22 6.48 22 12 S 17.52 22 12 22 S 2 17.52 2 12 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 16.5 V 11 M 12 7.5 H 12 V 7.5 H 12 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TDESIGN_OUTLINE_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 22 11.46 C 21.86 13.54 20.48 15.09 18.27 15.69 C 16.41 16.19 14.32 15.19 12.88 16.39 C 11.45 17.59 11.68 19.05 12.33 20.08 C 12.75 20.75 13.51 21.9 11.65 21.9 C 6.13 21.9 2 17.52 2 12 S 6.48 2 12 2 S 22.37 5.95 22 11.46 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 11.75 8 A 1 1 0 1 1 11.75 6 A 1 1 0 0 1 11.75 8 Z M 7.03 11 A 1 1 0 1 1 7.03 9 A 1 1 0 0 1 7.03 11 Z M 16.52 11 A 1 1 0 1 1 16.52 9 A 1 1 0 0 1 16.52 11 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17 2 L 7 12 L 17 22", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 2 L 17 12 L 7 22", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 16 L 12.17 6.18 L 22 16.35", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15 15 L 22 22 z M 9.5 17 A 7.5 7.5 0 1 0 9.5 2 A 7.5 7.5 0 0 0 9.5 17 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14 4 L 20 10 z M 22.29 5.29 C 22.68 5.68 22.68 6.32 22.29 6.71 L 9 20 L 2 22 L 4 15 L 17.29 1.71 A 1 1 0 0 1 18.71 1.71 z M 3 19 L 5 21 M 7 17 L 15 9", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 9 V 1 H 19.5 L 23 4.5 V 23 H 4 M 18 1 V 6 H 23 M 8 12 L 4 16 L 8 20 M 4 16 H 15", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9 15 H 17 z M 9 11 H 19 z M 9 7 H 13 z M 16 1 V 7 H 22 M 6 5 H 2 V 23 H 18 V 19 M 22 19 H 6 V 1 H 17 L 22 6 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 5 H 20 V 23 H 4 z M 1 5 H 23 M 9 1 H 15 V 5 H 9 z M 9 1 H 15 V 5 H 9 z M 15 9 V 19 M 9 9 V 19", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 22 C 17.52 22 22 17.52 22 12 S 17.52 2 12 2 S 2 6.48 2 12 S 6.48 22 12 22 Z M 12 10 V 18 M 12 6 V 8", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GROMMET_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 10 H 2 C 2 4.48 2 1 12 1 S 22 4.48 22 10 H 20 M 12 10 C 16.42 10 20 9.11 20 8 S 16.42 6 12 6 S 4 6.89 4 8 S 7.58 10 12 10 Z M 4 20 C 4 21.66 7.58 23 12 23 S 20 21.66 20 20 M 20 8 V 20 z M 4 20 V 8 z M 8 13 V 17 M 13 13 V 17 M 7 12 C 5.5 12 4 11 4 8 M 18 13.5 V 12 C 18 10 20 11 20 8 M 8 13 A 1 1 0 0 0 7 12 M 13 13 A 1 1 0 0 1 15 13 V 13.5 M 15 13.5 A 1.5 1.5 0 0 0 18 13.5 M 8 17 A 2.5 2.5 0 1 0 13 17", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14.75 8 A 0.75 0.75 0 0 1 14 8.75 H 3.81 L 6.53 11.47 A 0.75 0.75 0 1 1 5.47 12.53 L 1.47 8.53 A 0.75 0.75 0 0 1 1.47 7.47 L 5.47 3.47 A 0.75 0.75 0 0 1 6.53 4.53 L 3.81 7.25 H 14 A 0.75 0.75 0 0 1 14.75 8", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 1.25 8 A 0.75 0.75 0 0 1 2 7.25 H 12.19 L 9.47 4.53 A 0.75 0.75 0 0 1 10.53 3.47 L 14.53 7.47 A 0.75 0.75 0 0 1 14.53 8.53 L 10.53 12.53 A 0.75 0.75 0 1 1 9.47 11.47 L 12.19 8.75 H 2 A 0.75 0.75 0 0 1 1.25 8", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 14.75 A 0.75 0.75 0 0 1 7.25 14 V 3.81 L 4.53 6.53 A 0.75 0.75 0 0 1 3.47 5.47 L 7.47 1.47 A 0.75 0.75 0 0 1 8.53 1.47 L 12.53 5.47 A 0.75 0.75 0 0 1 11.47 6.53 L 8.75 3.81 V 14 A 0.75 0.75 0 0 1 8 14.75", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.5 7 A 4.5 4.5 0 1 1 2.5 7 A 4.5 4.5 0 0 1 11.5 7 M 10.68 11.74 A 6 6 0 1 1 11.74 10.68 L 14.53 13.47 A 0.75 0.75 0 1 1 13.47 14.53 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.42 1 A 3.58 3.58 0 0 1 15 4.58 C 15 4.85 14.89 5.11 14.7 5.3 L 14.17 5.83 L 12.2 7.8 L 7.14 12.86 A 3 3 0 0 1 5.61 13.68 L 2.97 14.21 A 1 1 0 0 1 1.79 13.03 L 2.32 10.39 A 3 3 0 0 1 3.14 8.86 L 8.2 3.8 L 10.7 1.3 C 10.89 1.11 11.15 1 11.42 1 M 9.11 5.01 L 4.2 9.92 A 1.5 1.5 0 0 0 3.79 10.69 L 3.41 12.59 L 5.31 12.21 A 1.5 1.5 0 0 0 6.08 11.8 L 10.99 6.89 A 2.08 2.08 0 0 0 9.11 5.01 M 12.21 5.67 A 3.6 3.6 0 0 0 10.33 3.79 L 11.61 2.51 C 12.61 2.6 13.4 3.39 13.49 4.39 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6.44 4.06 L 6.88 4.5 H 12.5 A 1.5 1.5 0 0 1 14 6 V 11 A 1.5 1.5 0 0 1 12.5 12.5 H 3.5 A 1.5 1.5 0 0 1 2 11 V 4.5 A 1.5 1.5 0 0 1 3.5 3 H 4.76 A 1.5 1.5 0 0 1 5.82 3.44 z M 0.5 4.5 A 3 3 0 0 1 3.5 1.5 H 4.76 A 3 3 0 0 1 6.88 2.38 L 7.5 3 H 12.5 A 3 3 0 0 1 15.5 6 V 11 A 3 3 0 0 1 12.5 14 H 3.5 A 3 3 0 0 1 0.5 11 z M 10.53 9.03 A 0.75 0.75 0 1 0 9.47 7.97 L 8.75 8.69 V 6.5 A 0.75 0.75 0 1 0 7.25 6.5 V 8.69 L 6.53 7.97 A 0.75 0.75 0 1 0 5.47 9.03 L 7.47 11.03 A 0.75 0.75 0 0 0 8.53 11.03 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 2.5 H 8 A 1.5 1.5 0 0 0 6.5 4 V 5 H 8 A 3 3 0 0 1 11 8 V 9.5 H 12 A 1.5 1.5 0 0 0 13.5 8 V 4 A 1.5 1.5 0 0 0 12 2.5 M 11 11 H 12 A 3 3 0 0 0 15 8 V 4 A 3 3 0 0 0 12 1 H 8 A 3 3 0 0 0 5 4 V 5 H 4 A 3 3 0 0 0 1 8 V 12 A 3 3 0 0 0 4 15 H 8 A 3 3 0 0 0 11 12 z M 4 6.5 H 8 A 1.5 1.5 0 0 1 9.5 8 V 12 A 1.5 1.5 0 0 1 8 13.5 H 4 A 1.5 1.5 0 0 1 2.5 12 V 8 A 1.5 1.5 0 0 1 4 6.5", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.44 3.5 H 12.5 A 1.5 1.5 0 0 1 14 5 V 11 A 1.5 1.5 0 0 1 12.5 12.5 H 5.44 A 1.5 1.5 0 0 1 4.27 11.94 L 1.8 8.84 A 1.35 1.35 0 0 1 1.8 7.16 L 4.27 4.06 A 1.5 1.5 0 0 1 5.44 3.5 M 3.1 3.13 A 3 3 0 0 1 5.44 2 H 12.5 A 3 3 0 0 1 15.5 5 V 11 A 3 3 0 0 1 12.5 14 H 5.44 A 3 3 0 0 1 3.1 12.87 L 0.62 9.78 A 2.85 2.85 0 0 1 0.62 6.22 z M 7.28 5.47 A 0.75 0.75 0 0 0 6.22 6.53 L 7.69 8 L 6.22 9.47 A 0.75 0.75 0 1 0 7.28 10.53 L 8.75 9.06 L 10.22 10.53 A 0.75 0.75 0 1 0 11.28 9.47 L 9.81 8 L 11.28 6.53 A 0.75 0.75 0 0 0 10.22 5.47 L 8.75 6.94 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 13.5 A 5.5 5.5 0 1 0 8 2.5 A 5.5 5.5 0 0 0 8 13.5 M 8 15 A 7 7 0 1 0 8 1 A 7 7 0 0 0 8 15 M 9 5.5 A 1 1 0 1 1 7 5.5 A 1 1 0 0 1 9 5.5 M 8.75 8.5 A 0.75 0.75 0 0 0 7.25 8.5 V 11 A 0.75 0.75 0 0 0 8.75 11 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_GRAVITY_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.01 10 C 12.44 10 12.78 9.85 13.01 9.59 C 13.26 9.32 13.5 8.83 13.5 8 A 5.5 5.5 0 1 0 8.33 13.49 A 4.3 4.3 0 0 1 8.22 12.6 A 2.37 2.37 0 0 1 8.72 11.12 C 9.11 10.63 9.64 10.36 10.17 10.21 C 10.69 10.06 11.3 10 12 10 z M 8.43 14.01 V 14 z M 12 11.5 C 13.66 11.51 15 10.25 15 8 A 7 7 0 1 0 8 15 C 10.19 15 10.01 14.17 9.83 13.32 C 9.63 12.42 9.43 11.5 12 11.5 M 9 5 A 1 1 0 1 1 7 5 A 1 1 0 0 1 9 5 M 11 7.75 A 1 1 0 1 0 11 5.75 A 1 1 0 0 0 11 7.75 M 6.25 6.75 A 1 1 0 1 1 4.25 6.75 A 1 1 0 0 1 6.25 6.75 M 5.75 11 A 1 1 0 1 0 5.75 9 A 1 1 0 0 0 5.75 11", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.83 3.13 A 0.5 0.5 0 0 1 7.87 3.84 L 4.17 8.01 L 7.87 12.18 A 0.5 0.5 0 0 1 7.12 12.84 L 3.12 8.34 A 0.5 0.5 0 0 1 3.12 7.68 L 7.12 3.18 A 0.5 0.5 0 0 1 7.83 3.14 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 11.8 3.13 A 0.5 0.5 0 0 1 11.84 3.84 L 8.14 8.01 L 11.84 12.18 A 0.5 0.5 0 1 1 11.09 12.84 L 7.09 8.34 A 0.5 0.5 0 0 1 7.09 7.68 L 11.09 3.18 A 0.5 0.5 0 0 1 11.8 3.14 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4.17 12.9 A 0.5 0.5 0 0 1 4.13 12.19 L 7.83 8.02 L 4.13 3.85 A 0.5 0.5 0 0 1 4.88 3.19 L 8.88 7.69 A 0.5 0.5 0 0 1 8.88 8.35 L 4.88 12.85 A 0.5 0.5 0 0 1 4.17 12.9 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8.17 12.9 A 0.5 0.5 0 0 1 8.13 12.19 L 11.83 8.02 L 8.13 3.85 A 0.5 0.5 0 0 1 8.88 3.19 L 12.88 7.69 A 0.5 0.5 0 0 1 12.88 8.35 L 8.88 12.85 A 0.5 0.5 0 0 1 8.17 12.9 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.85 2.15 A 0.5 0.5 0 0 0 7.14 2.15 L 2.14 7.15 A 0.5 0.5 0 0 0 2.85 7.86 L 7 3.71 V 13.5 A 0.5 0.5 0 0 0 8 13.5 V 3.71 L 12.15 7.86 A 0.5 0.5 0 0 0 12.86 7.15 L 7.86 2.15 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6.5 1 C 7.55 1 8.57 1.3 9.45 1.86 A 5.51 5.51 0 0 1 10.72 10.03 L 14.86 14.16 A 0.5 0.5 0 1 1 14.15 14.87 L 10.02 10.74 A 5.5 5.5 0 0 1 5.06 11.82 A 5.5 5.5 0 0 1 2.77 10.56 A 5.49 5.49 0 0 1 1.99 3.36 A 5.54 5.54 0 0 1 3.95 1.64 A 5.5 5.5 0 0 1 6.49 1.02 z M 6.5 2 A 4.5 4.5 0 0 0 4.43 2.5 C 3.79 2.83 3.24 3.31 2.83 3.9 S 2.14 5.16 2.04 5.88 A 4.5 4.5 0 0 0 3.42 9.78 A 4.51 4.51 0 0 0 9.33 10.02 L 9.54 9.84 L 9.69 9.69 L 9.88 9.49 L 10.03 9.32 L 10.14 9.17 A 4.5 4.5 0 0 0 8.87 2.68 A 4.45 4.45 0 0 0 6.52 2.01 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.6 0 C 13.3 0 13.97 0.28 14.46 0.77 L 15.21 1.52 C 15.71 2.02 15.98 2.68 15.98 3.38 C 15.98 4.09 15.71 4.75 15.21 5.24 L 4.71 15.84 A 0.5 0.5 0 0 1 4.36 15.99 H 0.49 A 0.5 0.5 0 0 1 -0.01 15.49 V 11.74 A 0.5 0.5 0 0 1 0.13 11.39 L 10.73 0.79 A 2.62 2.62 0 0 1 12.59 0.02 z M 9.73 3.2 L 0.99 11.96 V 15 H 4.15 L 12.8 6.27 z M 12.6 1 C 12.16 1 11.75 1.17 11.44 1.48 L 10.43 2.49 L 13.5 5.56 L 14.51 4.55 C 14.78 4.28 14.95 3.93 14.98 3.55 L 14.99 3.39 C 14.99 2.96 14.82 2.54 14.51 2.23 L 13.76 1.48 A 1.62 1.62 0 0 0 12.6 1 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 1.85 5.23 A 3 3 0 0 1 3 5 H 3.5 A 0.5 0.5 0 0 1 3.5 6 H 3 A 2 2 0 0 0 1 8 V 13 A 2 2 0 0 0 3 14.99 H 12 C 12.53 14.99 13.04 14.78 13.41 14.41 S 14 13.52 14 13 V 8 A 2 2 0 0 0 12 6 H 11.5 A 0.5 0.5 0 0 1 11.5 5 H 12 C 12.8 5 13.56 5.32 14.12 5.88 S 15 7.21 15 8 V 13 C 15 13.8 14.68 14.56 14.12 15.12 S 12.79 16 12 16 H 3 A 3 3 0 0 1 0 13 V 8 A 2.99 2.99 0 0 1 1.85 5.23 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 7.85 0.15 A 0.5 0.5 0 0 0 7.14 0.15 L 4.14 3.15 A 0.5 0.5 0 0 0 4.85 3.85 L 7 1.7 V 9.49 A 0.5 0.5 0 0 0 8 9.49 V 1.7 L 10.15 3.85 A 0.5 0.5 0 0 0 10.86 3.15 L 7.86 0.15 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.5 3 A 2.5 2.5 0 0 0 5 5.5 V 13.5 A 2.5 2.5 0 0 0 7.5 16 H 13.5 A 2.5 2.5 0 0 0 16 13.5 V 5.5 A 2.5 2.5 0 0 0 13.5 3 z M 6 5.5 A 1.5 1.5 0 0 1 7.5 4 H 13.5 A 1.5 1.5 0 0 1 15 5.5 V 13.5 A 1.5 1.5 0 0 1 13.5 15 H 7.5 A 1.5 1.5 0 0 1 6 13.5 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8 6.5 A 0.5 0.5 0 0 1 8.5 6 H 12.5 A 0.5 0.5 0 0 1 12.5 7 H 8.5 A 0.5 0.5 0 0 1 8 6.5 M 8 9.5 A 0.5 0.5 0 0 1 8.5 9 H 12.5 A 0.5 0.5 0 0 1 12.5 10 H 8.5 A 0.5 0.5 0 0 1 8 9.5 M 8 12.5 A 0.5 0.5 0 0 1 8.5 12 H 11.5 A 0.5 0.5 0 0 1 11.5 13 H 8.5 A 0.5 0.5 0 0 1 8 12.5 M 0 2.5 A 2.5 2.5 0 0 1 2.5 0 H 8.5 C 9.48 0 10.33 0.56 10.74 1.38 C 10.89 1.68 10.64 2 10.3 2 C 10.07 2 9.88 1.85 9.75 1.66 A 1.5 1.5 0 0 0 8.51 1 H 2.51 A 1.5 1.5 0 0 0 1.01 2.5 V 10.5 A 1.5 1.5 0 0 0 2.51 12 H 3.51 A 0.5 0.5 0 0 1 3.51 13 H 2.51 A 2.5 2.5 0 0 1 0.01 10.5 V 2.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6.5 7 A 0.5 0.5 0 0 1 7 7.5 V 11.5 A 0.5 0.5 0 0 1 6 11.5 V 7.5 A 0.5 0.5 0 0 1 6.5 7 M 10 7.5 A 0.5 0.5 0 0 0 9 7.5 V 11.5 A 0.5 0.5 0 0 0 10 11.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8.12 0 H 7.88 C 7.48 0 7.14 0 6.87 0.02 C 6.59 0.04 6.33 0.09 6.09 0.22 A 2 2 0 0 0 5.21 1.09 C 5.09 1.33 5.04 1.59 5.02 1.87 C 5 2.15 5 2.48 5 2.88 V 3.01 H 1.5 A 0.5 0.5 0 0 0 1.5 4.01 H 2.04 L 2.63 11.17 C 2.7 11.92 2.75 12.53 2.82 13.01 C 2.9 13.5 3.02 13.91 3.23 14.28 C 3.59 14.88 4.11 15.37 4.74 15.67 C 5.13 15.85 5.55 15.93 6.04 15.97 C 6.53 16.01 7.13 16.01 7.89 16.01 H 8.1 C 8.86 16.01 9.46 16.01 9.95 15.97 C 10.45 15.93 10.87 15.85 11.25 15.67 A 3.5 3.5 0 0 0 12.76 14.28 C 12.98 13.91 13.09 13.5 13.17 13.01 C 13.25 12.53 13.3 11.93 13.37 11.17 L 13.96 4.01 H 14.5 A 0.5 0.5 0 0 0 14.5 3.01 H 11 V 2.89 C 11 2.48 11 2.15 10.98 1.88 C 10.96 1.59 10.91 1.34 10.79 1.09 A 2 2 0 0 0 9.91 0.22 C 9.67 0.09 9.41 0.04 9.13 0.02 C 8.86 0 8.53 0 8.12 0 M 3.04 4 L 3.63 11.06 C 3.69 11.84 3.74 12.4 3.81 12.84 C 3.88 13.27 3.97 13.54 4.1 13.76 A 2.5 2.5 0 0 0 5.18 14.76 C 5.41 14.87 5.69 14.93 6.12 14.97 C 6.57 15 7.12 15 7.91 15 H 8.08 C 8.87 15 9.43 15 9.87 14.97 C 10.31 14.93 10.59 14.87 10.82 14.76 A 2.5 2.5 0 0 0 11.9 13.76 C 12.02 13.54 12.11 13.27 12.18 12.84 C 12.26 12.4 12.3 11.84 12.37 11.06 L 12.96 4 H 3.05 z M 6 3 H 10 V 2.9 C 10 2.47 10 2.18 9.98 1.96 C 9.96 1.74 9.93 1.62 9.89 1.55 A 1 1 0 0 0 9.45 1.11 C 9.38 1.07 9.26 1.04 9.04 1.02 A 13 13 0 0 0 8.1 1 H 7.9 C 7.47 1 7.18 1 6.96 1.02 C 6.74 1.04 6.62 1.07 6.55 1.11 A 1 1 0 0 0 6.11 1.55 C 6.07 1.62 6.04 1.74 6.02 1.96 C 6 2.18 6 2.47 6 2.9 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 7 A 0.75 0.75 0 0 1 8.75 7.75 V 11.25 A 0.75 0.75 0 0 1 7.25 11.25 V 7.75 A 0.75 0.75 0 0 1 8 7 M 8 6 A 1 1 0 1 0 8 4 A 1 1 0 0 0 8 6", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8 16 C 12.42 16 16 12.42 16 8 S 12.42 0 8 0 S 0 3.58 0 8 S 3.58 16 8 16 M 8 15 C 11.87 15 15 11.87 15 8 S 11.87 1 8 1 S 1 4.13 1 8 S 4.13 15 8 15", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_QLEMENTINE_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10.2 5 A 1.25 1.25 0 1 0 10.2 2.5 A 1.25 1.25 0 0 0 10.2 5 M 14 6.75 A 1.25 1.25 0 1 1 11.5 6.75 A 1.25 1.25 0 0 1 14 6.75 M 6.25 5.5 A 1.25 1.25 0 1 0 6.25 3 A 1.25 1.25 0 0 0 6.25 5.5 M 5.5 11.2 A 1.25 1.25 0 1 1 3 11.2 A 1.25 1.25 0 0 1 5.5 11.2 M 3.75 8.5 A 1.25 1.25 0 1 0 3.75 6 A 1.25 1.25 0 0 0 3.75 8.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8.5 0 C 3.81 0 0 3.81 0 8.5 V 9.82 C 0 13.23 2.77 16 6.18 16 H 6.82 C 8.62 16 10.16 14.73 10.52 12.97 L 10.55 12.81 C 10.77 11.71 11.68 10.88 12.8 10.77 A 3.55 3.55 0 0 0 16 7.23 V 6.27 C 16 2.81 13.19 0 9.73 0 H 8.5 z M 1 8.5 C 1 4.36 4.36 1 8.5 1 H 9.73 A 5.27 5.27 0 0 1 15 6.27 V 7.23 C 15 8.54 14.01 9.64 12.7 9.77 A 3.55 3.55 0 0 0 9.57 12.61 L 9.54 12.77 A 2.78 2.78 0 0 1 6.82 15 H 6.18 C 3.32 15 1 12.68 1 9.82 V 8.5 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_0: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 1.5 7.5 L 5.5 3.5 M 1.5 7.5 L 5.5 11.5 M 1.5 7.5 H 14", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_1: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 13.5 7.5 L 9.5 3.5 M 13.5 7.5 L 9.5 11.5 M 13.5 7.5 H 1", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_2: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 7.5 1.5 L 7.85 1.15 L 7.5 0.79 L 7.15 1.15 z M 7.15 1.85 L 11.15 5.85 L 11.85 5.15 L 7.85 1.15 z M 7.15 1.15 L 3.15 5.15 L 3.85 5.85 L 7.85 1.85 z M 7 1.5 V 14 H 8 V 1.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_3: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 14.5 14.5 L 10.5 10.5 M 6.5 12.5 A 6 6 0 1 1 6.5 0.5 A 6 6 0 0 1 6.5 12.5 Z", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_4: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 0.5 10.5 L 0.15 10.15 L 0 10.29 V 10.5 z M 10.5 0.5 L 10.85 0.15 A 0.5 0.5 0 0 0 10.15 0.15 z M 14.5 4.5 L 14.85 4.85 A 0.5 0.5 0 0 0 14.85 4.15 z M 4.5 14.5 V 15 H 4.71 L 4.85 14.85 z M 0.5 14.5 H 0 A 0.5 0.5 0 0 0 0.5 15 z M 0.85 10.85 L 10.85 0.85 L 10.15 0.15 L 0.15 10.15 z M 10.15 0.85 L 14.15 4.85 L 14.85 4.15 L 10.85 0.15 z M 14.15 4.15 L 4.15 14.15 L 4.85 14.85 L 14.85 4.85 z M 4.5 14 H 0.5 V 15 H 4.5 z M 1 14.5 V 10.5 H 0 V 14.5 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_5: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 4.5 7.5 H 8.5 M 11 4 L 8.5 7.5 L 11 11 M 14.5 2.5 A 2 2 0 0 1 10.5 2.5 A 2 2 0 0 1 14.5 2.5 Z M 14.5 12.49 A 2 2 0 0 1 10.5 12.49 A 2 2 0 0 1 14.5 12.49 Z M 4.5 7.5 A 2 2 0 0 1 0.5 7.5 A 2 2 0 0 1 4.5 7.5 Z", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_6: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 11 1.5 H 13.5 V 13.5 A 1 1 0 0 1 12.5 14.5 H 2.5 A 1 1 0 0 1 1.5 13.5 V 1.5 H 4 M 4.5 0.5 H 10.5 V 2.5 A 1 1 0 0 1 9.5 3.5 H 5.5 A 1 1 0 0 1 4.5 2.5 z", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_7: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 4.5 3 V 1.5 A 1 1 0 0 1 5.5 0.5 H 9.5 A 1 1 0 0 1 10.5 1.5 V 3 M 0 3.5 H 15 M 1.5 3.5 V 13.5 A 1 1 0 0 0 2.5 14.5 H 12.5 A 1 1 0 0 0 13.5 13.5 V 3.5 M 7.5 7 V 12 M 4.5 9 V 12 M 10.5 9 V 12", fill: false, even: false, alpha: 1.0, w: 1.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_8: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 7 4.5 V 5 H 8 V 4.5 z M 8 4.49 V 3.99 H 7 V 4.49 z M 8 11 V 7 H 7 V 11 z M 8 4.5 V 4.49 H 7 V 4.5 z M 6 8 H 7.5 V 7 H 6 z M 6 11 H 9 V 10 H 6 z M 7.5 1 A 6.5 6.5 0 0 1 14 7.5 H 15 A 7.5 7.5 0 0 0 7.5 0 z M 1 7.5 A 6.5 6.5 0 0 1 7.5 1 V 0 A 7.5 7.5 0 0 0 0 7.5 z M 7.5 14 A 6.5 6.5 0 0 1 1 7.5 H 0 A 7.5 7.5 0 0 0 7.5 15 z M 7.5 15 A 7.5 7.5 0 0 0 15 7.5 H 14 A 6.5 6.5 0 0 1 7.5 14 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_TEENYICONS_9: Glyph = Glyph {
    grid: 15.0,
    parts: &[
        Part { d: "M 0.5 14 L 0.02 13.86 A 0.5 0.5 0 0 0 0.78 14.42 z M 1.47 10.58 L 1.95 10.71 z M 1.01 13.66 L 0.73 13.24 z M 3.07 13.51 L 2.86 13.96 z M 5 6 L 4.75 5.57 A 0.5 0.5 0 0 0 4.65 6.35 z M 9 10 L 8.65 10.35 A 0.5 0.5 0 0 0 9.43 10.25 z M 14.5 0.5 L 14.93 0.75 A 0.5 0.5 0 0 0 14.25 0.07 z M 0.98 14.14 L 1.95 10.71 L 0.99 10.44 L 0.02 13.86 z M 3.94 9.21 H 4 V 8.21 H 3.94 z M 6 11.21 V 11.27 H 7 V 11.21 z M 4.05 13.21 H 3.97 V 14.21 H 4.05 z M 0.73 13.24 L 0.22 13.58 L 0.78 14.42 L 1.29 14.08 z M 3.29 13.06 A 2.6 2.6 0 0 0 0.73 13.24 L 1.29 14.07 A 1.6 1.6 0 0 1 2.86 13.96 z M 3.97 13.21 C 3.74 13.21 3.5 13.16 3.29 13.06 L 2.86 13.96 C 3.21 14.13 3.59 14.21 3.97 14.21 z M 6 11.27 A 1.95 1.95 0 0 1 4.05 13.21 V 14.21 A 2.95 2.95 0 0 0 7 11.27 z M 4 9.21 A 2 2 0 0 1 6 11.21 H 7 A 3 3 0 0 0 4 8.21 z M 1.95 10.71 A 2.06 2.06 0 0 1 3.94 9.21 V 8.21 C 2.57 8.21 1.36 9.12 0.99 10.44 z M 4.65 6.35 L 8.65 10.35 L 9.35 9.65 L 5.35 5.65 z M 14.25 0.07 L 4.75 5.57 L 5.25 6.43 L 14.75 0.93 z M 9.43 10.25 L 14.93 0.75 L 14.07 0.25 L 8.57 9.75 z M 7.15 4.85 L 10.15 7.85 L 10.85 7.15 L 7.85 4.15 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 12 H 4.12 M 10.25 18.75 L 4.56 13.06 A 1.5 1.5 0 0 1 4.12 12 M 10.25 5.25 L 4.56 10.94 A 1.5 1.5 0 0 0 4.12 12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 12 H 19.88 M 13.75 18.75 L 19.44 13.06 C 19.73 12.77 19.88 12.38 19.88 12 M 13.75 5.25 L 19.44 10.94 C 19.73 11.23 19.88 11.62 19.88 12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 20 V 4.12 M 5.25 10.25 L 10.94 4.56 A 1.5 1.5 0 0 1 12 4.12 M 18.75 10.25 L 13.06 4.56 A 1.5 1.5 0 0 0 12 4.12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.55 15.55 A 7.06 7.06 0 1 0 5.57 5.57 A 7.06 7.06 0 0 0 15.55 15.55 M 15.55 15.55 L 20 20", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14.44 5.78 L 4.2 16.02 A 2 2 0 0 0 3.63 17.14 L 3.08 20.92 L 6.86 20.37 A 2 2 0 0 0 7.98 19.8 L 18.22 9.56 M 14.44 5.78 L 16.67 3.55 A 1.6 1.6 0 0 1 18.93 3.55 L 20.45 5.07 A 1.6 1.6 0 0 1 20.45 7.33 L 18.22 9.56 M 14.44 5.78 L 18.22 9.56", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 9.01 V 2.75 M 9 5.32 L 11.27 3.05 C 11.47 2.85 11.74 2.75 12 2.75 M 15 5.32 L 12.73 3.05 C 12.53 2.85 12.26 2.75 12 2.75 M 14.99 12 H 21.25 M 18.68 9 L 20.95 11.27 C 21.15 11.47 21.25 11.74 21.25 12 M 18.68 15 L 20.95 12.73 C 21.15 12.53 21.25 12.26 21.25 12 M 12 14.99 V 21.25 M 9 18.68 L 11.27 20.95 C 11.47 21.15 11.74 21.25 12 21.25 M 15 18.68 L 12.73 20.95 C 12.53 21.15 12.26 21.25 12 21.25 M 9.01 12 H 2.75 M 5.32 9 L 3.05 11.27 C 2.85 11.47 2.75 11.74 2.75 12 M 5.32 15 L 3.05 12.73 C 2.85 12.53 2.75 12.26 2.75 12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16.75 5.75 A 3 3 0 0 0 13.75 2.75 H 7.25 A 3 3 0 0 0 4.25 5.75 V 15.25 A 3 3 0 0 0 7.25 18.25 H 13.75 A 3 3 0 0 0 16.75 15.25 z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 19.75 6.75 V 15.25 A 6 6 0 0 1 13.75 21.25 H 8.25", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4.69 6.21 L 6.8 18.98 A 2.5 2.5 0 0 0 9.27 21.07 H 12.61 M 19.31 6.21 L 17.2 18.98 A 2.5 2.5 0 0 1 14.73 21.07 H 11.39 M 10.02 11.12 V 16.16 M 13.98 11.12 V 16.16 M 2.75 6.21 H 21.25 M 14.78 6.21 V 4.43 A 1.5 1.5 0 0 0 13.28 2.93 H 10.72 A 1.5 1.5 0 0 0 9.22 4.43 V 6.21 z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2.75 12 A 9.25 9.25 0 1 0 21.25 12 A 9.25 9.25 0 1 0 2.75 12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 11.81 V 16.81", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 10.75 8.44 A 1.25 1.25 0 1 0 13.25 8.44 A 1.25 1.25 0 1 0 10.75 8.44", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_PROICONS_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.32 5.75 A 3 3 0 0 1 8.32 2.75 H 15.68 A 3 3 0 0 1 18.68 5.75 V 12 H 5.32 z M 18.68 12 H 5.32 V 14.61 A 1.5 1.5 0 0 0 6.82 16.11 H 10.2 V 19.45 A 1.8 1.8 0 0 0 13.8 19.45 V 16.11 H 17.18 A 1.5 1.5 0 0 0 18.68 14.61 z M 15.5 2.75 V 6.5 M 12.5 2.75 V 5.25", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_0: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 244 400 L 100 256 L 244 112 M 120 256 H 412", fill: false, even: false, alpha: 1.0, w: 48.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_1: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 268 112 L 412 256 L 268 400 M 392 256 H 100", fill: false, even: false, alpha: 1.0, w: 48.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_2: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 112 244 L 256 100 L 400 244 M 256 120 V 412", fill: false, even: false, alpha: 1.0, w: 48.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_3: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 456.69 421.39 L 362.6 327.3 A 173.8 173.8 0 0 0 397.44 222.72 C 397.44 126.38 319.06 48 222.72 48 S 48 126.38 48 222.72 S 126.38 397.44 222.72 397.44 A 173.8 173.8 0 0 0 327.3 362.6 L 421.39 456.69 A 25 25 0 0 0 456.69 421.39 M 97.92 222.72 A 124.8 124.8 0 1 1 222.72 347.52 A 124.95 124.95 0 0 1 97.92 222.72", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_4: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 358.62 129.28 L 86.49 402.08 L 70 442 L 109.92 425.51 L 382.72 153.38 z M 413.07 74.84 L 401.28 86.62 L 425.38 110.72 L 437.17 98.93 A 16.51 16.51 0 0 0 437.17 75.59 L 436.42 74.84 A 16.51 16.51 0 0 0 413.07 74.84", fill: false, even: false, alpha: 1.0, w: 44.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_5: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 176 112 L 256 32 L 336 112 M 255.98 32 L 256 480 M 176 400 L 256 480 L 336 400 M 400 176 L 480 256 L 400 336 M 112 176 L 32 256 L 112 336 M 32 256 H 480", fill: false, even: false, alpha: 1.0, w: 32.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_6: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 408 480 H 184 A 72 72 0 0 1 112 408 V 184 A 72 72 0 0 1 184 112 H 408 A 72 72 0 0 1 480 184 V 408 A 72 72 0 0 1 408 480", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 160 80 H 395.88 A 72.12 72.12 0 0 0 328 32 H 104 A 72 72 0 0 0 32 104 V 328 A 72.12 72.12 0 0 0 80 395.88 V 160 A 80 80 0 0 1 160 80", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_7: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 432 96 H 336 V 72 A 40 40 0 0 0 296 32 H 216 A 40 40 0 0 0 176 72 V 96 H 80 A 16 16 0 0 0 80 128 H 97 L 116 432.92 C 117.42 459.77 138 480 164 480 H 348 C 374.13 480 394.3 460.22 396 433 L 415 128 H 432 A 16 16 0 0 0 432 96 M 192.57 416 H 192 A 16 16 0 0 1 176 400.57 L 168 176.57 A 16 16 0 1 1 200 175.43 L 208 399.43 A 16 16 0 0 1 192.57 416 M 272 400 A 16 16 0 0 1 240 400 V 176 A 16 16 0 0 1 272 176 Z M 304 96 H 208 V 72 A 7.91 7.91 0 0 1 216 64 H 296 A 7.91 7.91 0 0 1 304 72 Z M 336 400.57 A 16 16 0 0 1 320 416 H 319.42 A 16 16 0 0 1 304 399.43 L 312 175.43 A 16 16 0 1 1 344 176.57 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_8: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 196 220 H 260 V 392", fill: false, even: false, alpha: 1.0, w: 40.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 187 396 H 325", fill: false, even: false, alpha: 1.0, w: 40.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 256 160 A 32 32 0 1 1 288 128 A 32 32 0 0 1 256 160", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_FAMICONS_9: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 233.15 360.11 A 15.7 15.7 0 0 1 228.23 359.34 A 16 16 0 0 1 217.31 346.34 C 215.16 331.34 197.36 313.88 180.69 310.49 A 16 16 0 0 1 172 284.16 L 383.09 49.06 C 383.28 48.84 383.48 48.63 383.68 48.43 A 56.57 56.57 0 0 1 463.57 48.43 A 56.51 56.51 0 0 1 463.68 128.21 L 244.68 355.21 A 16 16 0 0 1 233.15 360.11 M 119.89 480.11 C 87.75 480.11 54.44 463.22 35.04 437.11 A 16 16 0 0 1 47.89 411.57 C 53.23 411.57 67.89 406.7 67.89 391 C 67.89 351.93 99.29 320.14 137.89 320.14 S 207.89 351.93 207.89 391 C 207.89 440.12 168.41 480.11 119.89 480.11", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_0: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 405.32 234.67 H 188.35 L 258.6 164.42 L 228.43 134.25 L 106.68 256 L 228.43 377.75 L 258.6 347.58 L 188.35 277.34 H 405.32 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_1: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 106.68 277.33 H 323.65 L 253.4 347.58 L 283.57 377.75 L 405.32 256 L 283.57 134.25 L 253.4 164.42 L 323.65 234.66 H 106.68 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_2: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 277.33 405.32 V 188.35 L 347.58 258.6 L 377.75 228.43 L 256 106.68 L 134.25 228.43 L 164.42 258.6 L 234.66 188.35 V 405.32 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_3: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 469.3 439.13 L 347.98 317.82 C 370.47 288.91 384 252.71 384 213.33 C 384 119.23 307.44 42.67 213.33 42.67 S 42.67 119.23 42.67 213.33 S 119.23 384 213.33 384 C 252.71 384 288.91 370.47 317.81 347.98 L 439.13 469.3 z M 213.33 341.33 C 142.75 341.33 85.33 283.92 85.33 213.33 C 85.33 142.75 142.75 85.33 213.33 85.33 C 283.92 85.33 341.33 142.75 341.33 213.33 C 341.33 283.92 283.92 341.33 213.33 341.33", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_4: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 469.33 426.67 V 469.33 H 42.67 V 426.67 z M 320 42.67 L 426.67 149.33 L 192 384 H 85.33 V 277.33 z M 249.75 173.25 L 128 295 V 341.33 H 174.34 L 296.08 219.59 z M 320 103 L 279.92 143.08 L 326.25 189.42 L 366.34 149.33 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_5: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 234.67 85.33 V 128 H 106.67 V 405.33 H 405.33 V 277.33 H 448 V 448 H 64 V 85.33 z M 377.75 48.91 L 478.17 149.33 L 377.75 249.75 L 347.58 219.58 L 396.49 170.67 H 341.33 C 307.25 170.67 279.39 197.31 277.44 230.91 L 277.33 234.67 V 341.33 H 234.67 V 234.67 C 234.67 177.31 279.94 130.52 336.71 128.1 L 341.33 128 L 396.48 128 L 347.58 79.08 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_6: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 426.67 128 V 448 H 170.67 V 128 z M 384 170.67 H 213.33 V 405.33 H 384 z M 320 42.67 V 85.33 H 128 V 341.33 H 85.33 V 42.67 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_7: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 320 85.33 H 192 V 42.67 H 320 z M 234.67 213.33 H 192 V 384 H 234.67 z M 320 213.33 H 277.33 V 384 H 320 z M 448 128 V 170.67 H 405.33 V 469.33 H 106.67 V 170.67 H 64 V 128 z M 362.67 170.67 H 149.33 V 426.67 H 362.67 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 21 H 3 V 3 H 21 z M 5 5 V 19 H 19 V 5 z M 13.25 8 C 13.25 8.72 12.72 9.25 12.01 9.25 C 11.27 9.25 10.75 8.72 10.75 7.99 C 10.75 7.28 11.28 6.75 12.01 6.75 C 12.72 6.75 13.25 7.28 13.25 8 M 11 11 H 13 V 17 H 11 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_IX_9: Glyph = Glyph {
    grid: 512.0,
    parts: &[
        Part { d: "M 256 42.67 C 373.64 42.67 469.33 128.5 469.33 234 A 125.44 125.44 0 0 1 432.58 322.54 C 409.09 346.03 374.55 362.63 341.33 362.67 H 304.25 C 299.54 362.67 295.23 363.09 291.9 366.42 C 288.56 369.75 287.29 372.35 287.29 377.06 A 18.8 18.8 0 0 0 291.9 389 A 47.62 47.62 0 0 1 304.25 421.08 L 304.02 425.85 A 48.27 48.27 0 0 1 290.12 455.21 L 286.58 458.4 A 48.25 48.25 0 0 1 256 469.33 C 138.36 469.33 42.67 373.64 42.67 256 S 138.36 42.67 256 42.67 M 256 85.33 C 161.93 85.33 85.33 161.93 85.33 256 S 161.93 426.67 256 426.67 C 257.48 426.67 258.91 426.07 259.96 425.02 A 5.58 5.58 0 0 0 261.58 421.08 V 421.02 A 4.94 4.94 0 0 0 260.31 417.69 L 259.96 417.31 L 259.62 416.94 A 61.5 61.5 0 0 1 244.65 377.87 L 244.63 377.46 V 377.06 C 244.63 370.83 245.46 362.68 249.29 354 C 252.99 345.6 258.15 339.81 261.71 336.25 L 264.88 333.31 C 279.85 320.52 297 320.01 303.17 320 H 341.29 C 361.41 319.98 385.55 309.24 402.42 292.38 A 82.8 82.8 0 0 0 426.67 234 L 426.46 226.73 C 422.13 151.94 351.48 85.33 256 85.33 M 138.67 213.33 C 156.34 213.33 170.67 227.66 170.67 245.33 S 156.34 277.33 138.67 277.33 S 106.67 263.01 106.67 245.33 S 120.99 213.33 138.67 213.33 M 373.33 213.33 C 391.01 213.33 405.33 227.66 405.33 245.33 S 391.01 277.33 373.33 277.33 S 341.33 263.01 341.33 245.33 S 355.66 213.33 373.33 213.33 M 202.67 128 C 220.34 128 234.67 142.33 234.67 160 S 220.34 192 202.67 192 S 170.67 177.67 170.67 160 S 184.99 128 202.67 128 M 309.33 128 C 327.01 128 341.33 142.33 341.33 160 S 327.01 192 309.33 192 S 277.33 177.67 277.33 160 S 291.66 128 309.33 128", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.85 4.15 L 6.56 4.85 L 3.91 7.5 H 12.96 V 8.5 H 3.91 L 6.56 11.15 L 5.85 11.85 L 2 8 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.04 7.5 L 9.4 4.85 L 10.1 4.15 L 13.96 8 L 10.1 11.85 L 9.4 11.15 L 12.04 8.5 H 3 V 7.5 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.98 2.02 L 11.83 5.88 L 11.12 6.58 L 8.48 3.94 V 12.98 H 7.48 V 3.94 L 4.83 6.58 L 4.12 5.88 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6.5 2 A 4.5 4.5 0 1 0 6.5 11 A 4.5 4.5 0 0 0 6.5 2 M 1 6.5 A 5.5 5.5 0 1 1 10.73 10.02 L 13.85 13.15 L 13.15 13.85 L 10.02 10.73 A 5.5 5.5 0 0 1 1 6.5", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9.64 1.65 A 0.5 0.5 0 0 1 10.35 1.65 L 12.47 3.77 A 0.5 0.5 0 0 1 12.47 4.47 L 6.15 10.79 L 2.72 11.54 L 3.28 8.13 A 0.5 0.5 0 0 1 3.42 7.86 z M 14 14 H 2 V 13 H 14 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 2 H 7.06 V 3 H 3 V 13 H 13 V 9 H 14 V 14 H 2 z M 9.97 2.08 H 14.01 V 6 H 13.01 V 3.79 L 8.28 8.53 L 7.57 7.82 L 12.3 3.08 H 9.97 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 2.5 A 0.5 0.5 0 0 1 5.5 2 H 13.5 A 0.5 0.5 0 0 1 14 2.5 V 10.5 A 0.5 0.5 0 0 1 13.5 11 H 12 V 10 H 13 V 3 H 6 V 4 H 5 z M 2 5.5 A 0.5 0.5 0 0 1 2.5 5 H 10.5 A 0.5 0.5 0 0 1 11 5.5 V 13.5 A 0.5 0.5 0 0 1 10.5 14 H 2.5 A 0.5 0.5 0 0 1 2 13.5 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6 3 H 10 V 2 H 6 z M 2 5 H 3 V 13.5 A 0.5 0.5 0 0 0 3.5 14 H 12.5 A 0.5 0.5 0 0 0 13 13.5 V 5 H 14 V 4 H 2 z M 6 7 V 11 H 7 V 7 z M 9 7 V 11 H 10 V 7 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 1 A 7 7 0 1 0 8 15 A 7 7 0 0 0 8 1 M 8 4 A 0.75 0.75 0 1 0 8 5.5 A 0.75 0.75 0 0 0 8 4 M 7 7.5 H 7.5 V 11 H 6.5 V 12 H 9.5 V 11 H 8.5 V 6.5 H 7 z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_LSICON_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6 2.5 A 0.5 0.5 0 0 1 6.5 2 H 9.5 A 0.5 0.5 0 0 1 10 2.5 V 5 H 13.5 A 0.5 0.5 0 0 1 14 5.5 V 7 H 2 V 5.5 A 0.5 0.5 0 0 1 2.5 5 H 6 z M 2 8 V 13.5 A 0.5 0.5 0 0 0 2.5 14 H 5.5 V 10.5 H 6.5 V 14 H 7.5 V 9.5 H 8.5 V 14 H 9.5 V 10.5 H 10.5 V 14 H 13.5 A 0.5 0.5 0 0 0 14 13.5 V 8 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 19 L 5 12 L 12 5 M 19 12 H 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 19 L 19 12 L 12 5 M 19 12 H 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 5 V 19 M 19 12 L 12 5 L 5 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 10 A 7 7 0 1 0 17 10 A 7 7 0 1 0 3 10", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 21 21 L 14.95 14.95", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16.95 2.81 A 1 1 0 0 1 21.19 7.05 L 7.76 20.49 L 2 22 L 3.51 16.24 Z M 13 21 H 21", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 17 L 17 7 M 7 7 H 17 V 17", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 15 H 4 A 2 2 0 0 1 2 13 V 4 A 2 2 0 0 1 4 2 H 13 A 2 2 0 0 1 15 4 V 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 11 9 H 20 A 2 2 0 0 1 22 11 V 20 A 2 2 0 0 1 20 22 H 11 A 2 2 0 0 1 9 20 V 11 A 2 2 0 0 1 11 9 Z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 6 H 21 M 19 6 V 20 A 2 2 0 0 1 17 22 H 7 A 2 2 0 0 1 5 20 V 6 M 8 6 V 4 A 2 2 0 0 1 10 2 H 14 A 2 2 0 0 1 16 4 V 6", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 12 A 10 10 0 1 0 22 12 A 10 10 0 1 0 2 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 8 Z M 12 12 V 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_METEOR_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13.5 7.5 A 0.5 0.5 0 1 0 14.5 7.5 A 0.5 0.5 0 1 0 13.5 7.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8.6 8 A 0.5 0.5 0 1 0 9.6 8 A 0.5 0.5 0 1 0 8.6 8", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 6.5 12.5 A 0.5 0.5 0 1 0 7.5 12.5 A 0.5 0.5 0 1 0 6.5 12.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 22 C 14 22 14.67 21 14 19 C 13.33 17 14.5 16 17.5 16 C 20.5 16 22 14.67 22 12 A 10 10 0 1 0 12 22", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.25 3.75 L 2.75 8.25 L 7.25 12.75 M 13.25 8.25 H 2.75", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8.75 3.25 L 13.25 7.75 L 8.75 12.25 M 2.75 7.75 H 13.25", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3.75 7.25 L 8.25 2.75 L 12.75 7.25 M 8.25 13.25 V 2.75", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.25 11.25 L 14.25 14.25", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 2.75 7.5 A 4.75 4.75 0 1 0 12.25 7.5 A 4.75 4.75 0 1 0 2.75 7.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 1.75 11.25 L 1.75 14.25 L 4.75 14.25 L 14.25 4.75 L 11.25 1.75 Z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 8.75 4.75 L 11.25 7.25", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.25 10.25 L 14.25 8 L 12.25 5.75 M 10.25 3.75 L 8 1.75 L 5.75 3.75 M 3.75 5.75 L 1.75 8 L 3.75 10.25 M 5.75 12.25 L 8 14.25 L 10.25 12.25 M 8 1.75 V 13.75 M 13.75 8 H 1.75", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.25 4.25 V 1.75 H 1.75 V 11.25 H 4.25 M 4.75 4.75 V 14.25 H 14.25 V 4.75 z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5.75 4.25 V 1.75 H 10.25 V 4.25 M 3.75 5.25 V 14.25 H 12.25 V 5.25 M 2.75 4.75 H 13.25", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 1.75 8 A 6.25 6.25 0 1 0 14.25 8 A 6.25 6.25 0 1 0 1.75 8", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 8 5.25 V 5.25 M 8 11.25 V 7.75", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_CHARM_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 1.75 8 C 1.75 11.45 4.55 14.25 8 14.25 C 11.41 14.25 14.25 11.25 14.25 8.25 C 13.25 8.75 10.25 9.75 8.25 7.75 S 7.25 2.75 7.75 1.75 C 4.75 1.75 1.75 4.59 1.75 8 z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 17.71 5.29 A 1 1 0 0 1 17.71 6.71 L 12.41 12 L 17.71 17.29 A 1 1 0 0 1 16.29 18.71 L 10.29 12.71 A 1 1 0 0 1 10.29 11.29 L 16.29 5.29 A 1 1 0 0 1 17.71 5.29 M 11.71 5.29 A 1 1 0 0 1 11.71 6.71 L 6.41 12 L 11.71 17.29 A 1 1 0 0 1 10.29 18.71 L 4.29 12.71 A 1 1 0 0 1 4.29 11.29 L 10.29 5.29 A 1 1 0 0 1 11.71 5.29", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.29 5.29 A 1 1 0 0 1 13.71 5.29 L 19.71 11.29 A 1 1 0 0 1 19.71 12.71 L 13.71 18.71 A 1 1 0 0 1 12.29 17.29 L 17.59 12 L 12.29 6.71 A 1 1 0 0 1 12.29 5.29 M 6.29 5.29 A 1 1 0 0 1 7.71 5.29 L 13.71 11.29 A 1 1 0 0 1 13.71 12.71 L 7.71 18.71 A 1 1 0 0 1 6.29 17.29 L 11.59 12 L 6.29 6.71 A 1 1 0 0 1 6.29 5.29", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 4 A 1 1 0 0 1 12.71 4.29 L 18.71 10.29 A 1 1 0 0 1 17.29 11.71 L 13 7.41 V 19 A 1 1 0 1 1 11 19 V 7.41 L 6.71 11.71 A 1 1 0 0 1 5.29 10.29 L 11.29 4.29 A 1 1 0 0 1 12 4", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 4 A 6 6 0 1 0 10 16 A 6 6 0 0 0 10 4 M 2 10 A 8 8 0 1 1 16.32 14.91 L 21.71 20.29 A 1 1 0 0 1 20.29 21.71 L 14.91 16.32 A 8 8 0 0 1 2 10", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16.29 2.29 A 1 1 0 0 1 17.71 2.29 L 21.71 6.29 A 1 1 0 0 1 21.71 7.71 L 8.71 20.71 A 1 1 0 0 1 8 21 H 4 A 1 1 0 0 1 3 20 V 16 A 1 1 0 0 1 3.29 15.29 L 13.29 5.29 z M 14 7.41 L 5 16.41 V 19 H 7.59 L 16.59 10 z M 18 8.59 L 19.59 7 L 17 4.41 L 15.41 6 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.29 2.29 A 1 1 0 0 1 12.71 2.29 L 16.71 6.29 A 1 1 0 0 1 15.29 7.71 L 13 5.41 V 16 A 1 1 0 1 1 11 16 V 5.41 L 8.71 7.71 A 1 1 0 0 1 7.29 6.29 z M 5 17 A 1 1 0 0 1 6 18 V 20 H 18 V 18 A 1 1 0 1 1 20 18 V 20 A 2 2 0 0 1 18 22 H 6 A 2 2 0 0 1 4 20 V 18 A 1 1 0 0 1 5 17", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 4 A 2 2 0 0 1 4 2 H 14 A 2 2 0 0 1 16 4 V 8 H 20 A 2 2 0 0 1 22 10 V 20 A 2 2 0 0 1 20 22 H 10 A 2 2 0 0 1 8 20 V 16 H 4 A 2 2 0 0 1 2 14 z M 10 16 V 20 H 20 V 10 H 16 V 14 A 2 2 0 0 1 14 16 z M 14 14 V 4 H 4 V 14 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 4 A 2 2 0 0 1 9 2 H 15 A 2 2 0 0 1 17 4 V 6 H 21 A 1 1 0 1 1 21 8 H 19.93 L 19.06 20.14 A 2 2 0 0 1 17.07 22 H 6.93 A 2 2 0 0 1 4.93 20.14 L 4.07 8 H 3 A 1 1 0 0 1 3 6 H 7 z M 9 6 H 15 V 4 H 9 z M 6.07 8 L 6.93 20 H 17.07 L 17.93 8 z M 10 10 A 1 1 0 0 1 11 11 V 17 A 1 1 0 1 1 9 17 V 11 A 1 1 0 0 1 10 10 M 14 10 A 1 1 0 0 1 15 11 V 17 A 1 1 0 1 1 13 17 V 11 A 1 1 0 0 1 14 10", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 4 A 8 8 0 1 0 12 20 A 8 8 0 0 0 12 4 M 2 12 C 2 6.48 6.48 2 12 2 S 22 6.48 22 12 S 17.52 22 12 22 S 2 17.52 2 12", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 10 A 1 1 0 0 1 13 11 V 17 A 1 1 0 1 1 11 17 V 11 A 1 1 0 0 1 12 10 M 13.5 7.5 A 1.5 1.5 0 1 1 10.5 7.5 A 1.5 1.5 0 0 1 13.5 7.5", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_MONO_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9.36 3.29 A 1 1 0 0 1 9.55 4.45 A 7.45 7.45 0 0 0 19.55 14.45 A 1 1 0 0 1 20.89 15.8 A 9.45 9.45 0 1 1 8.2 3.11 A 1 1 0 0 1 9.36 3.29 z M 6.82 6.67 A 7.45 7.45 0 0 0 17.33 17.18 A 9.45 9.45 0 0 1 6.82 6.67 z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 12 H 19 M 12 5 L 5 12 L 12 19", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 19 12 H 5 M 12 19 L 19 12 L 12 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 5 V 19 M 19 12 L 12 5 L 5 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 21 21 L 16.66 16.66 M 16.66 16.66 A 8 8 0 1 0 5.34 5.34 A 8 8 0 0 0 16.66 16.66", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 16 L 4 20 L 8 19 L 19.59 7.41 A 2 2 0 0 0 19.59 4.59 L 19.41 4.41 A 2 2 0 0 0 16.59 4.41 z M 15 6 L 18 9 M 13 20 H 21", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14 5 L 12 3 M 12 3 L 10 5 M 12 3 V 21 M 12 21 L 14 19 M 12 21 L 10 19 M 19 14 L 21 12 M 21 12 L 19 10 M 21 12 H 3 M 3 12 L 5 14 M 3 12 L 5 10", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 16 3 H 4 V 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 8 7 H 20 V 19 A 2 2 0 0 1 18 21 H 10 A 2 2 0 0 1 8 19 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14 11 V 17 M 10 11 V 17 M 6 7 V 19 A 2 2 0 0 0 8 21 H 16 A 2 2 0 0 0 18 19 V 7 M 4 7 H 20 M 7 7 L 9 3 H 15 L 17 7", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 12 A 9 9 0 1 0 21 12 A 9 9 0 1 0 3 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 8 H 12.01 V 8.01 H 12 z", fill: false, even: false, alpha: 1.0, w: 3.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 12 V 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_ICONAMOON_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 4 H 6 V 6 H 4 z M 4 18 H 6 V 20 H 4 z M 18 4 H 20 V 6 H 18 z M 18 11 H 20 V 13 H 18 z M 11 11 H 13 V 13 H 11 z M 4 11 H 6 V 13 H 4 z M 11 4 H 13 V 6 H 11 z M 11 18 H 13 V 20 H 11 z M 18 18 H 20 V 20 H 18 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 11.25 C 20.41 11.25 20.75 11.59 20.75 12 C 20.75 12.41 20.41 12.75 20 12.75 H 10.75 L 10.75 18 C 10.75 18.3 10.57 18.58 10.29 18.69 C 10.01 18.81 9.68 18.74 9.47 18.53 L 3.47 12.53 C 3.33 12.39 3.25 12.2 3.25 12 C 3.25 11.8 3.33 11.61 3.47 11.47 L 9.47 5.47 C 9.68 5.26 10.01 5.19 10.29 5.31 C 10.57 5.42 10.75 5.7 10.75 6 L 10.75 11.25 H 20 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 11.25 C 3.59 11.25 3.25 11.59 3.25 12 C 3.25 12.41 3.59 12.75 4 12.75 H 13.25 V 18 C 13.25 18.3 13.43 18.58 13.71 18.69 C 13.99 18.81 14.32 18.74 14.53 18.53 L 20.53 12.53 C 20.67 12.39 20.75 12.2 20.75 12 C 20.75 11.8 20.67 11.61 20.53 11.47 L 14.53 5.47 C 14.32 5.26 13.99 5.19 13.71 5.31 C 13.43 5.42 13.25 5.7 13.25 6 V 11.25 H 4 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12.75 20 C 12.75 20.41 12.41 20.75 12 20.75 C 11.59 20.75 11.25 20.41 11.25 20 L 11.25 10.75 H 6 C 5.7 10.75 5.42 10.57 5.31 10.29 C 5.19 10.01 5.26 9.68 5.47 9.47 L 11.47 3.47 C 11.61 3.33 11.8 3.25 12 3.25 C 12.2 3.25 12.39 3.33 12.53 3.47 L 18.53 9.47 C 18.74 9.68 18.81 10.01 18.69 10.29 C 18.58 10.57 18.3 10.75 18 10.75 H 12.75 L 12.75 20 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.5 2 C 16.75 2 21 6.25 21 11.5 C 21 13.85 20.14 16.01 18.72 17.67 C 18.73 17.67 18.74 17.68 18.75 17.69 L 22.53 21.47 C 22.82 21.76 22.82 22.24 22.53 22.53 C 22.24 22.82 21.76 22.82 21.47 22.53 L 17.69 18.75 C 17.68 18.74 17.67 18.73 17.66 18.72 C 16.01 20.14 13.85 21 11.5 21 C 6.25 21 2 16.75 2 11.5 C 2 6.25 6.25 2 11.5 2 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11.4 18.16 L 11.4 18.16 L 18.8 10.77 C 17.79 10.35 16.6 9.66 15.47 8.53 C 14.34 7.4 13.65 6.21 13.23 5.2 L 5.84 12.6 L 5.84 12.6 C 5.26 13.18 4.97 13.47 4.72 13.78 C 4.43 14.16 4.18 14.57 3.98 14.99 C 3.8 15.36 3.67 15.75 3.42 16.52 L 2.05 20.6 C 1.93 20.99 2.03 21.41 2.31 21.69 C 2.59 21.97 3.01 22.07 3.4 21.95 L 7.48 20.58 C 8.25 20.33 8.64 20.2 9 20.02 C 9.43 19.82 9.84 19.57 10.22 19.28 C 10.53 19.03 10.82 18.74 11.4 18.16 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 20.85 8.71 C 22.38 7.18 22.38 4.69 20.85 3.15 C 19.31 1.62 16.82 1.62 15.29 3.15 L 14.4 4.04 C 14.41 4.08 14.42 4.11 14.44 4.15 C 14.76 5.09 15.38 6.32 16.53 7.47 C 17.68 8.62 18.91 9.24 19.85 9.56 C 19.89 9.58 19.92 9.59 19.96 9.6 L 20.85 8.71 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 22 11.8 V 14 C 22 17.77 22 19.66 20.83 20.83 C 19.66 22 17.77 22 14 22 H 10 C 6.23 22 4.34 22 3.17 20.83 C 2.2 19.85 2.03 18.39 2.01 15.75 H 10.69 L 8.43 17.39 C 8.1 17.64 8.02 18.11 8.27 18.44 C 8.51 18.78 8.98 18.85 9.32 18.61 L 13.44 15.61 C 13.64 15.47 13.75 15.24 13.75 15 C 13.75 14.76 13.64 14.53 13.44 14.39 L 9.32 11.39 C 8.98 11.15 8.51 11.22 8.27 11.56 C 8.02 11.89 8.1 12.36 8.43 12.61 L 10.69 14.25 H 2 L 2 14 L 2 6.95 C 2 6.07 2 5.63 2.07 5.26 C 2.37 3.64 3.64 2.37 5.26 2.07 C 5.63 2 6.07 2 6.95 2 C 7.34 2 7.53 2 7.72 2.02 C 8.52 2.09 9.28 2.41 9.9 2.92 C 10.04 3.04 10.18 3.18 10.45 3.45 L 11 4 C 11.82 4.82 12.22 5.22 12.71 5.5 C 12.98 5.64 13.27 5.76 13.56 5.85 C 14.1 6 14.67 6 15.83 6 H 16.2 C 18.83 6 20.15 6 21.01 6.77 C 21.08 6.84 21.16 6.92 21.23 6.99 C 22 7.85 22 9.17 22 11.8 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15.24 5.61 C 17.96 5.61 19.31 5.61 20.16 6.46 C 21 7.31 21 8.67 21 11.4 V 16.22 C 21 18.94 21 20.31 20.16 21.15 C 19.31 22 17.95 22 15.24 22 H 12.36 C 9.65 22 8.29 22 7.44 21.15 C 6.6 20.31 6.6 18.94 6.6 16.22 V 11.4 C 6.6 8.67 6.6 7.31 7.44 6.46 C 8.29 5.61 9.65 5.61 12.36 5.61 H 15.24 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 15.24 2 C 16.76 2 18.06 2.95 18.59 4.29 C 17.71 4.17 16.61 4.17 15.34 4.17 H 12.27 C 10.99 4.17 9.89 4.17 9.01 4.29 C 8.06 4.41 7.16 4.7 6.43 5.44 C 5.69 6.18 5.41 7.08 5.28 8.03 C 5.16 8.92 5.16 10.02 5.16 11.3 V 16.31 C 5.16 17.37 5.16 18.65 5.23 19.56 C 3.92 19.02 3 17.72 3 16.22 V 10.38 C 3 8.61 3 7.21 3.15 6.11 C 3.3 4.98 3.62 4.06 4.34 3.34 C 5.05 2.62 5.97 2.3 7.09 2.15 C 8.18 2 9.58 2 11.35 2 H 15.24 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2.75 6.17 C 2.75 5.71 3.1 5.33 3.52 5.33 L 6.19 5.33 C 6.72 5.32 7.18 4.95 7.36 4.42 C 7.37 4.4 7.37 4.39 7.39 4.32 L 7.51 3.95 C 7.58 3.72 7.64 3.52 7.72 3.35 C 8.06 2.64 8.69 2.16 9.41 2.03 C 9.59 2 9.79 2 10.01 2 H 13.49 C 13.71 2 13.91 2 14.09 2.03 C 14.81 2.16 15.44 2.64 15.78 3.35 C 15.86 3.52 15.92 3.72 15.99 3.95 L 16.11 4.32 C 16.13 4.39 16.13 4.4 16.14 4.42 C 16.32 4.95 16.88 5.32 17.41 5.33 H 19.98 C 20.4 5.33 20.75 5.71 20.75 6.17 C 20.75 6.63 20.4 7 19.98 7 H 3.52 C 3.1 7 2.75 6.63 2.75 6.17 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 11.61 22 H 12.39 C 15.1 22 16.45 22 17.34 21.14 C 18.22 20.27 18.31 18.86 18.49 16.03 L 18.75 11.95 C 18.84 10.41 18.89 9.64 18.45 9.15 C 18.01 8.67 17.26 8.67 15.77 8.67 H 8.23 C 6.74 8.67 5.99 8.67 5.55 9.15 C 5.11 9.64 5.16 10.41 5.26 11.95 L 5.51 16.03 C 5.7 18.86 5.79 20.27 6.67 21.14 C 7.55 22 8.9 22 11.61 22 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 22 12 C 22 17.52 17.52 22 12 22 C 6.48 22 2 17.52 2 12 C 2 6.48 6.48 2 12 2 C 17.52 2 22 6.48 22 12 Z M 12 17.75 C 12.41 17.75 12.75 17.41 12.75 17 V 11 C 12.75 10.59 12.41 10.25 12 10.25 C 11.59 10.25 11.25 10.59 11.25 11 V 17 C 11.25 17.41 11.59 17.75 12 17.75 Z M 12 7 C 12.55 7 13 7.45 13 8 C 13 8.55 12.55 9 12 9 C 11.45 9 11 8.55 11 8 C 11 7.45 11.45 7 12 7 Z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_BOLD_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 6 V 18 C 10 19.4 10 20.1 9.73 20.64 C 9.49 21.11 9.11 21.49 8.63 21.73 C 8.1 22 7.4 22 6 22 C 4.6 22 3.9 22 3.37 21.73 C 2.89 21.49 2.51 21.11 2.27 20.64 C 2 20.1 2 19.4 2 18 V 6 C 2 4.6 2 3.9 2.27 3.37 C 2.51 2.89 2.89 2.51 3.37 2.27 C 3.9 2 4.6 2 6 2 C 7.4 2 8.1 2 8.63 2.27 C 9.11 2.51 9.49 2.89 9.73 3.37 C 10 3.9 10 4.6 10 6 Z M 7 19.75 C 7.41 19.75 7.75 19.41 7.75 19 C 7.75 18.59 7.41 18.25 7 18.25 H 5 C 4.59 18.25 4.25 18.59 4.25 19 C 4.25 19.41 4.59 19.75 5 19.75 H 7 Z", fill: true, even: true, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 19.06 10.61 L 13.22 16.7 C 12.49 17.47 12.13 17.85 11.81 17.72 C 11.5 17.59 11.5 17.07 11.5 16.01 L 11.5 7.77 C 11.5 7.11 11.76 6.48 12.23 6.01 L 13.28 4.96 L 13.71 4.63 C 14.72 3.86 15.22 3.48 15.77 3.36 C 16.22 3.26 16.69 3.29 17.12 3.44 C 17.65 3.63 18.1 4.07 19 4.96 C 20 5.96 20.5 6.46 20.68 7.03 C 20.84 7.52 20.85 8.04 20.7 8.53 C 20.52 9.11 20.03 9.61 19.06 10.61 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12.79 22 H 17.9 C 19.3 22 20 22 20.53 21.73 C 21 21.49 21.39 21.11 21.63 20.64 C 21.9 20.1 21.9 19.4 21.9 18 C 21.9 16.6 21.9 15.9 21.63 15.37 C 21.39 14.89 21 14.51 20.53 14.27 C 20 14 19.3 14 17.9 14 H 17.68 L 11.88 19.8 C 11.64 20.04 11.5 20.34 11.5 20.68 C 11.5 21.39 12.08 22 12.79 22 Z", fill: true, even: false, alpha: 1.0, w: 0.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_0: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 12 H 4 M 10 18 L 4 12 L 10 6", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_1: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 12 H 20 M 14 18 L 20 12 L 14 6", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_2: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 20 L 12 4 M 6 10 L 12 4 L 18 10", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_3: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 11.5 A 9.5 9.5 0 1 0 21 11.5 A 9.5 9.5 0 1 0 2 11.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 18.22 18.22 L 22 22", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_4: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14.36 4.08 L 15.29 3.15 C 16.82 1.62 19.31 1.62 20.85 3.15 C 22.38 4.69 22.38 7.18 20.85 8.71 L 19.92 9.64 M 14.36 4.08 C 14.36 4.08 14.48 6.05 16.21 7.79 C 17.95 9.52 19.92 9.64 19.92 9.64 M 19.92 9.64 L 11.4 18.16 C 10.82 18.74 10.53 19.03 10.22 19.28 C 9.84 19.57 9.43 19.82 9 20.02 C 8.64 20.2 8.25 20.33 7.48 20.58 L 4.2 21.68 L 3.4 21.95 C 3.01 22.07 2.59 21.97 2.31 21.69 C 2.03 21.41 1.93 20.99 2.05 20.6 L 2.32 19.8 L 3.42 16.52 C 3.67 15.75 3.8 15.36 3.98 14.99 C 4.18 14.57 4.43 14.16 4.72 13.78 C 4.97 13.47 5.26 13.18 5.84 12.6 L 14.36 4.08 M 4.2 21.68 L 2.32 19.8", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_5: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 12 V 6.95 C 2 6.07 2 5.63 2.07 5.26 C 2.37 3.64 3.64 2.37 5.26 2.07 C 5.63 2 6.07 2 6.95 2 C 7.34 2 7.53 2 7.72 2.02 C 8.52 2.09 9.28 2.41 9.9 2.92 C 10.04 3.04 10.18 3.18 10.45 3.45 L 11 4 C 11.82 4.82 12.22 5.22 12.71 5.5 C 12.98 5.64 13.27 5.76 13.56 5.85 C 14.1 6 14.67 6 15.83 6 H 16.2 C 18.83 6 20.15 6 21.01 6.77 C 21.08 6.84 21.16 6.92 21.23 6.99 C 22 7.85 22 9.17 22 11.8 V 14 C 22 17.77 22 19.66 20.83 20.83 C 19.66 22 17.77 22 14 22 H 10 C 6.23 22 4.34 22 3.17 20.83 C 2.52 20.18 2.23 19.3 2.1 18", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 2 15 C 8.44 15 6.56 15 13 15 M 8.88 18 L 13 15 L 8.88 12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_6: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6 11 C 6 8.17 6 6.76 6.88 5.88 C 7.76 5 9.17 5 12 5 H 15 C 17.83 5 19.24 5 20.12 5.88 C 21 6.76 21 8.17 21 11 V 16 C 21 18.83 21 20.24 20.12 21.12 C 19.24 22 17.83 22 15 22 H 12 C 9.17 22 7.76 22 6.88 21.12 C 6 20.24 6 18.83 6 16 V 11 Z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 6 19 C 4.34 19 3 17.66 3 16 V 10 C 3 6.23 3 4.34 4.17 3.17 C 5.34 2 7.23 2 11 2 H 15 C 16.66 2 18 3.34 18 5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_7: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20.5 6 H 3.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 18.83 8.5 L 18.37 15.4 C 18.2 18.05 18.11 19.38 17.24 20.19 C 16.38 21 15.05 21 12.39 21 H 11.61 C 8.95 21 7.62 21 6.76 20.19 C 5.89 19.38 5.8 18.05 5.63 15.4 L 5.17 8.5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 6.5 6 C 6.56 6 6.58 6 6.61 6 C 7.43 5.98 8.16 5.45 8.44 4.68 C 8.45 4.66 8.46 4.63 8.47 4.58 L 8.57 4.29 C 8.65 4.04 8.7 3.91 8.75 3.81 C 8.97 3.39 9.38 3.09 9.84 3.02 C 9.96 3 10.09 3 10.36 3 H 13.64 C 13.91 3 14.04 3 14.16 3.02 C 14.62 3.09 15.03 3.39 15.25 3.81 C 15.3 3.91 15.35 4.04 15.43 4.29 L 15.53 4.58 C 15.54 4.63 15.55 4.66 15.56 4.68 C 15.84 5.45 16.57 5.98 17.39 6 C 17.42 6 17.44 6 17.5 6", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_8: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 12 A 10 10 0 1 0 22 12 A 10 10 0 1 0 2 12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 17 V 11", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 8 H 12", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};
static G_SOLAR_LINEAR_9: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 2 6 C 2 4.6 2 3.9 2.27 3.37 C 2.51 2.89 2.89 2.51 3.37 2.27 C 3.9 2 4.6 2 6 2 C 7.4 2 8.1 2 8.63 2.27 C 9.11 2.51 9.49 2.89 9.73 3.37 C 10 3.9 10 4.6 10 6 V 18 C 10 19.4 10 20.1 9.73 20.64 C 9.49 21.11 9.11 21.49 8.63 21.73 C 8.1 22 7.4 22 6 22 C 4.6 22 3.9 22 3.37 21.73 C 2.89 21.49 2.51 21.11 2.27 20.64 C 2 20.1 2 19.4 2 18 V 6 Z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 7 19 H 5", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 13.31 4.93 L 11.17 7.07 C 10.59 7.65 10.3 7.94 10.15 8.31 C 10 8.67 10 9.08 10 9.9 L 10 19.56 L 18.97 10.59 C 19.96 9.6 20.46 9.1 20.64 8.53 C 20.8 8.03 20.8 7.49 20.64 6.98 C 20.46 6.41 19.96 5.92 18.97 4.93 C 17.98 3.94 17.49 3.44 16.91 3.26 C 16.41 3.1 15.87 3.1 15.37 3.26 C 14.8 3.44 14.3 3.94 13.31 4.93 Z", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
        Part { d: "M 6 22 L 18 22 C 19.4 22 20.1 22 20.64 21.73 C 21.11 21.49 21.49 21.11 21.73 20.64 C 22 20.1 22 19.4 22 18 C 22 16.6 22 15.9 21.73 15.37 C 21.49 14.89 21.11 14.51 20.64 14.27 C 20.1 14 19.4 14 18 14 L 15.5 14", fill: false, even: false, alpha: 1.0, w: 1.5, dx: 0.0, dy: 0.0 },
    ],
};

pub static SETS: &[Set] = &[
    Set { key: "tabler_line", label: "Tabler · 线", glyphs: [Some(&G_TABLER_LINE_0), Some(&G_TABLER_LINE_1), Some(&G_TABLER_LINE_2), Some(&G_TABLER_LINE_3), Some(&G_TABLER_LINE_4), Some(&G_TABLER_LINE_5), Some(&G_TABLER_LINE_6), Some(&G_TABLER_LINE_7), Some(&G_TABLER_LINE_8), Some(&G_TABLER_LINE_9)] },
    Set { key: "tabler_filled", label: "Tabler · 实心", glyphs: [Some(&G_TABLER_FILLED_0), Some(&G_TABLER_FILLED_1), Some(&G_TABLER_FILLED_2), Some(&G_TABLER_FILLED_3), Some(&G_TABLER_FILLED_4), Some(&G_TABLER_FILLED_5), Some(&G_TABLER_FILLED_6), Some(&G_TABLER_FILLED_7), Some(&G_TABLER_FILLED_8), Some(&G_TABLER_FILLED_9)] },
    Set { key: "lucide", label: "Lucide · 线", glyphs: [Some(&G_LUCIDE_0), Some(&G_LUCIDE_1), Some(&G_LUCIDE_2), Some(&G_LUCIDE_3), Some(&G_LUCIDE_4), Some(&G_LUCIDE_5), Some(&G_LUCIDE_6), Some(&G_LUCIDE_7), Some(&G_LUCIDE_8), Some(&G_LUCIDE_9)] },
    Set { key: "bi", label: "Bootstrap", glyphs: [Some(&G_BI_0), Some(&G_BI_1), Some(&G_BI_2), Some(&G_BI_3), Some(&G_BI_4), Some(&G_BI_5), Some(&G_BI_6), Some(&G_BI_7), Some(&G_BI_8), Some(&G_BI_9)] },
    Set { key: "ph_regular", label: "Phosphor · 常规", glyphs: [Some(&G_PH_REGULAR_0), Some(&G_PH_REGULAR_1), Some(&G_PH_REGULAR_2), Some(&G_PH_REGULAR_3), Some(&G_PH_REGULAR_4), Some(&G_PH_REGULAR_5), Some(&G_PH_REGULAR_6), Some(&G_PH_REGULAR_7), Some(&G_PH_REGULAR_8), Some(&G_PH_REGULAR_9)] },
    Set { key: "ph_bold", label: "Phosphor · 粗", glyphs: [Some(&G_PH_BOLD_0), Some(&G_PH_BOLD_1), Some(&G_PH_BOLD_2), Some(&G_PH_BOLD_3), Some(&G_PH_BOLD_4), Some(&G_PH_BOLD_5), Some(&G_PH_BOLD_6), Some(&G_PH_BOLD_7), Some(&G_PH_BOLD_8), Some(&G_PH_BOLD_9)] },
    Set { key: "ph_fill", label: "Phosphor · 实心", glyphs: [Some(&G_PH_FILL_0), Some(&G_PH_FILL_1), Some(&G_PH_FILL_2), Some(&G_PH_FILL_3), Some(&G_PH_FILL_4), Some(&G_PH_FILL_5), Some(&G_PH_FILL_6), Some(&G_PH_FILL_7), Some(&G_PH_FILL_8), Some(&G_PH_FILL_9)] },
    Set { key: "ph_duotone", label: "Phosphor · 双色", glyphs: [Some(&G_PH_DUOTONE_0), Some(&G_PH_DUOTONE_1), Some(&G_PH_DUOTONE_2), Some(&G_PH_DUOTONE_3), Some(&G_PH_DUOTONE_4), Some(&G_PH_DUOTONE_5), Some(&G_PH_DUOTONE_6), Some(&G_PH_DUOTONE_7), Some(&G_PH_DUOTONE_8), Some(&G_PH_DUOTONE_9)] },
    Set { key: "ph_light", label: "Phosphor · 细", glyphs: [Some(&G_PH_LIGHT_0), Some(&G_PH_LIGHT_1), Some(&G_PH_LIGHT_2), Some(&G_PH_LIGHT_3), Some(&G_PH_LIGHT_4), Some(&G_PH_LIGHT_5), Some(&G_PH_LIGHT_6), Some(&G_PH_LIGHT_7), Some(&G_PH_LIGHT_8), Some(&G_PH_LIGHT_9)] },
    Set { key: "iconoir", label: "Iconoir", glyphs: [Some(&G_ICONOIR_0), Some(&G_ICONOIR_1), Some(&G_ICONOIR_2), Some(&G_ICONOIR_3), Some(&G_ICONOIR_4), Some(&G_ICONOIR_5), Some(&G_ICONOIR_6), Some(&G_ICONOIR_7), Some(&G_ICONOIR_8), Some(&G_ICONOIR_9)] },
    Set { key: "hero_outline", label: "Heroicons · 线", glyphs: [Some(&G_HERO_OUTLINE_0), Some(&G_HERO_OUTLINE_1), Some(&G_HERO_OUTLINE_2), Some(&G_HERO_OUTLINE_3), Some(&G_HERO_OUTLINE_4), Some(&G_HERO_OUTLINE_5), Some(&G_HERO_OUTLINE_6), Some(&G_HERO_OUTLINE_7), Some(&G_HERO_OUTLINE_8), Some(&G_HERO_OUTLINE_9)] },
    Set { key: "hero_solid", label: "Heroicons · 实心", glyphs: [Some(&G_HERO_SOLID_0), Some(&G_HERO_SOLID_1), Some(&G_HERO_SOLID_2), Some(&G_HERO_SOLID_3), Some(&G_HERO_SOLID_4), Some(&G_HERO_SOLID_5), Some(&G_HERO_SOLID_6), Some(&G_HERO_SOLID_7), Some(&G_HERO_SOLID_8), Some(&G_HERO_SOLID_9)] },
    Set { key: "fluent_regular", label: "Fluent · 线", glyphs: [Some(&G_FLUENT_REGULAR_0), Some(&G_FLUENT_REGULAR_1), Some(&G_FLUENT_REGULAR_2), Some(&G_FLUENT_REGULAR_3), Some(&G_FLUENT_REGULAR_4), Some(&G_FLUENT_REGULAR_5), Some(&G_FLUENT_REGULAR_6), Some(&G_FLUENT_REGULAR_7), Some(&G_FLUENT_REGULAR_8), Some(&G_FLUENT_REGULAR_9)] },
    Set { key: "fluent_filled", label: "Fluent · 实心", glyphs: [Some(&G_FLUENT_FILLED_0), Some(&G_FLUENT_FILLED_1), Some(&G_FLUENT_FILLED_2), Some(&G_FLUENT_FILLED_3), Some(&G_FLUENT_FILLED_4), Some(&G_FLUENT_FILLED_5), Some(&G_FLUENT_FILLED_6), Some(&G_FLUENT_FILLED_7), Some(&G_FLUENT_FILLED_8), Some(&G_FLUENT_FILLED_9)] },
    Set { key: "mingcute_line", label: "MingCute · 线", glyphs: [Some(&G_MINGCUTE_LINE_0), Some(&G_MINGCUTE_LINE_1), Some(&G_MINGCUTE_LINE_2), Some(&G_MINGCUTE_LINE_3), Some(&G_MINGCUTE_LINE_4), Some(&G_MINGCUTE_LINE_5), Some(&G_MINGCUTE_LINE_6), Some(&G_MINGCUTE_LINE_7), Some(&G_MINGCUTE_LINE_8), Some(&G_MINGCUTE_LINE_9)] },
    Set { key: "mingcute_fill", label: "MingCute · 实心", glyphs: [Some(&G_MINGCUTE_FILL_0), Some(&G_MINGCUTE_FILL_1), Some(&G_MINGCUTE_FILL_2), Some(&G_MINGCUTE_FILL_3), Some(&G_MINGCUTE_FILL_4), Some(&G_MINGCUTE_FILL_5), Some(&G_MINGCUTE_FILL_6), Some(&G_MINGCUTE_FILL_7), Some(&G_MINGCUTE_FILL_8), Some(&G_MINGCUTE_FILL_9)] },
    Set { key: "ms_rounded", label: "Material Symbols", glyphs: [Some(&G_MS_ROUNDED_0), Some(&G_MS_ROUNDED_1), Some(&G_MS_ROUNDED_2), Some(&G_MS_ROUNDED_3), Some(&G_MS_ROUNDED_4), Some(&G_MS_ROUNDED_5), Some(&G_MS_ROUNDED_6), Some(&G_MS_ROUNDED_7), Some(&G_MS_ROUNDED_8), Some(&G_MS_ROUNDED_9)] },
    Set { key: "mdi", label: "Material Design Icons", glyphs: [Some(&G_MDI_0), Some(&G_MDI_1), Some(&G_MDI_2), Some(&G_MDI_3), Some(&G_MDI_4), Some(&G_MDI_5), Some(&G_MDI_6), Some(&G_MDI_7), Some(&G_MDI_8), Some(&G_MDI_9)] },
    Set { key: "carbon", label: "Carbon", glyphs: [Some(&G_CARBON_0), Some(&G_CARBON_1), Some(&G_CARBON_2), Some(&G_CARBON_3), Some(&G_CARBON_4), Some(&G_CARBON_5), Some(&G_CARBON_6), Some(&G_CARBON_7), Some(&G_CARBON_8), Some(&G_CARBON_9)] },
    Set { key: "iconpark_line", label: "IconPark · 线", glyphs: [Some(&G_ICONPARK_LINE_0), Some(&G_ICONPARK_LINE_1), Some(&G_ICONPARK_LINE_2), Some(&G_ICONPARK_LINE_3), Some(&G_ICONPARK_LINE_4), Some(&G_ICONPARK_LINE_5), Some(&G_ICONPARK_LINE_6), Some(&G_ICONPARK_LINE_7), Some(&G_ICONPARK_LINE_8), Some(&G_ICONPARK_LINE_9)] },
    Set { key: "antd_outline", label: "Ant Design · 线", glyphs: [Some(&G_ANTD_OUTLINE_0), Some(&G_ANTD_OUTLINE_1), Some(&G_ANTD_OUTLINE_2), Some(&G_ANTD_OUTLINE_3), Some(&G_ANTD_OUTLINE_4), Some(&G_ANTD_OUTLINE_5), Some(&G_ANTD_OUTLINE_6), Some(&G_ANTD_OUTLINE_7), Some(&G_ANTD_OUTLINE_8), Some(&G_ANTD_OUTLINE_9)] },
    Set { key: "radix", label: "Radix", glyphs: [Some(&G_RADIX_0), Some(&G_RADIX_1), Some(&G_RADIX_2), Some(&G_RADIX_3), Some(&G_RADIX_4), Some(&G_RADIX_5), Some(&G_RADIX_6), Some(&G_RADIX_7), Some(&G_RADIX_8), Some(&G_RADIX_9)] },
    Set { key: "majesticons_line", label: "Majesticons · 线", glyphs: [Some(&G_MAJESTICONS_LINE_0), Some(&G_MAJESTICONS_LINE_1), Some(&G_MAJESTICONS_LINE_2), Some(&G_MAJESTICONS_LINE_3), Some(&G_MAJESTICONS_LINE_4), Some(&G_MAJESTICONS_LINE_5), Some(&G_MAJESTICONS_LINE_6), Some(&G_MAJESTICONS_LINE_7), Some(&G_MAJESTICONS_LINE_8), Some(&G_MAJESTICONS_LINE_9)] },
    Set { key: "majesticons_solid", label: "Majesticons · 实心", glyphs: [Some(&G_MAJESTICONS_SOLID_0), Some(&G_MAJESTICONS_SOLID_1), Some(&G_MAJESTICONS_SOLID_2), Some(&G_MAJESTICONS_SOLID_3), Some(&G_MAJESTICONS_SOLID_4), Some(&G_MAJESTICONS_SOLID_5), Some(&G_MAJESTICONS_SOLID_6), Some(&G_MAJESTICONS_SOLID_7), Some(&G_MAJESTICONS_SOLID_8), Some(&G_MAJESTICONS_SOLID_9)] },
    Set { key: "pixelarticons", label: "Pixelarticons", glyphs: [Some(&G_PIXELARTICONS_0), Some(&G_PIXELARTICONS_1), Some(&G_PIXELARTICONS_2), Some(&G_PIXELARTICONS_3), Some(&G_PIXELARTICONS_4), Some(&G_PIXELARTICONS_5), Some(&G_PIXELARTICONS_6), Some(&G_PIXELARTICONS_7), Some(&G_PIXELARTICONS_8), Some(&G_PIXELARTICONS_9)] },
    Set { key: "octicon", label: "Octicons", glyphs: [Some(&G_OCTICON_0), Some(&G_OCTICON_1), Some(&G_OCTICON_2), Some(&G_OCTICON_3), Some(&G_OCTICON_4), Some(&G_OCTICON_5), Some(&G_OCTICON_6), Some(&G_OCTICON_7), Some(&G_OCTICON_8), Some(&G_OCTICON_9)] },
    Set { key: "mynaui_outline", label: "Mynaui · 线", glyphs: [Some(&G_MYNAUI_OUTLINE_0), Some(&G_MYNAUI_OUTLINE_1), Some(&G_MYNAUI_OUTLINE_2), Some(&G_MYNAUI_OUTLINE_3), Some(&G_MYNAUI_OUTLINE_4), Some(&G_MYNAUI_OUTLINE_5), Some(&G_MYNAUI_OUTLINE_6), Some(&G_MYNAUI_OUTLINE_7), Some(&G_MYNAUI_OUTLINE_8), Some(&G_MYNAUI_OUTLINE_9)] },
    Set { key: "mynaui_solid", label: "Mynaui · 实心", glyphs: [Some(&G_MYNAUI_SOLID_0), Some(&G_MYNAUI_SOLID_1), Some(&G_MYNAUI_SOLID_2), Some(&G_MYNAUI_SOLID_3), Some(&G_MYNAUI_SOLID_4), Some(&G_MYNAUI_SOLID_5), Some(&G_MYNAUI_SOLID_6), Some(&G_MYNAUI_SOLID_7), Some(&G_MYNAUI_SOLID_8), Some(&G_MYNAUI_SOLID_9)] },
    Set { key: "hugeicons", label: "Hugeicons", glyphs: [Some(&G_HUGEICONS_0), Some(&G_HUGEICONS_1), Some(&G_HUGEICONS_2), Some(&G_HUGEICONS_3), Some(&G_HUGEICONS_4), Some(&G_HUGEICONS_5), Some(&G_HUGEICONS_6), Some(&G_HUGEICONS_7), Some(&G_HUGEICONS_8), Some(&G_HUGEICONS_9)] },
    Set { key: "feather", label: "Feather", glyphs: [Some(&G_FEATHER_0), Some(&G_FEATHER_1), Some(&G_FEATHER_2), Some(&G_FEATHER_3), Some(&G_FEATHER_4), Some(&G_FEATHER_5), Some(&G_FEATHER_6), Some(&G_FEATHER_7), Some(&G_FEATHER_8), Some(&G_FEATHER_9)] },
    Set { key: "system_uicons", label: "System UIcons", glyphs: [Some(&G_SYSTEM_UICONS_0), Some(&G_SYSTEM_UICONS_1), Some(&G_SYSTEM_UICONS_2), Some(&G_SYSTEM_UICONS_3), Some(&G_SYSTEM_UICONS_4), Some(&G_SYSTEM_UICONS_5), Some(&G_SYSTEM_UICONS_6), Some(&G_SYSTEM_UICONS_7), Some(&G_SYSTEM_UICONS_8), Some(&G_SYSTEM_UICONS_9)] },
    Set { key: "akar", label: "Akar", glyphs: [Some(&G_AKAR_0), Some(&G_AKAR_1), Some(&G_AKAR_2), Some(&G_AKAR_3), Some(&G_AKAR_4), Some(&G_AKAR_5), Some(&G_AKAR_6), Some(&G_AKAR_7), Some(&G_AKAR_8), Some(&G_AKAR_9)] },
    Set { key: "tdesign_outline", label: "TDesign · 线", glyphs: [Some(&G_TDESIGN_OUTLINE_0), Some(&G_TDESIGN_OUTLINE_1), Some(&G_TDESIGN_OUTLINE_2), Some(&G_TDESIGN_OUTLINE_3), Some(&G_TDESIGN_OUTLINE_4), Some(&G_TDESIGN_OUTLINE_5), Some(&G_TDESIGN_OUTLINE_6), Some(&G_TDESIGN_OUTLINE_7), Some(&G_TDESIGN_OUTLINE_8), Some(&G_TDESIGN_OUTLINE_9)] },
    Set { key: "grommet", label: "Grommet", glyphs: [Some(&G_GROMMET_0), Some(&G_GROMMET_1), Some(&G_GROMMET_2), Some(&G_GROMMET_3), Some(&G_GROMMET_4), Some(&G_GROMMET_5), Some(&G_GROMMET_6), Some(&G_GROMMET_7), Some(&G_GROMMET_8), Some(&G_GROMMET_9)] },
    Set { key: "gravity", label: "Gravity UI", glyphs: [Some(&G_GRAVITY_0), Some(&G_GRAVITY_1), Some(&G_GRAVITY_2), Some(&G_GRAVITY_3), Some(&G_GRAVITY_4), Some(&G_GRAVITY_5), Some(&G_GRAVITY_6), Some(&G_GRAVITY_7), Some(&G_GRAVITY_8), Some(&G_GRAVITY_9)] },
    Set { key: "qlementine", label: "Qlementine", glyphs: [Some(&G_QLEMENTINE_0), Some(&G_QLEMENTINE_1), Some(&G_QLEMENTINE_2), Some(&G_QLEMENTINE_3), Some(&G_QLEMENTINE_4), Some(&G_QLEMENTINE_5), Some(&G_QLEMENTINE_6), Some(&G_QLEMENTINE_7), Some(&G_QLEMENTINE_8), Some(&G_QLEMENTINE_9)] },
    Set { key: "teenyicons", label: "Teenyicons", glyphs: [Some(&G_TEENYICONS_0), Some(&G_TEENYICONS_1), Some(&G_TEENYICONS_2), Some(&G_TEENYICONS_3), Some(&G_TEENYICONS_4), Some(&G_TEENYICONS_5), Some(&G_TEENYICONS_6), Some(&G_TEENYICONS_7), Some(&G_TEENYICONS_8), Some(&G_TEENYICONS_9)] },
    Set { key: "proicons", label: "ProIcons", glyphs: [Some(&G_PROICONS_0), Some(&G_PROICONS_1), Some(&G_PROICONS_2), Some(&G_PROICONS_3), Some(&G_PROICONS_4), Some(&G_PROICONS_5), Some(&G_PROICONS_6), Some(&G_PROICONS_7), Some(&G_PROICONS_8), Some(&G_PROICONS_9)] },
    Set { key: "famicons", label: "Famicons", glyphs: [Some(&G_FAMICONS_0), Some(&G_FAMICONS_1), Some(&G_FAMICONS_2), Some(&G_FAMICONS_3), Some(&G_FAMICONS_4), Some(&G_FAMICONS_5), Some(&G_FAMICONS_6), Some(&G_FAMICONS_7), Some(&G_FAMICONS_8), Some(&G_FAMICONS_9)] },
    Set { key: "ix", label: "Siemens IX", glyphs: [Some(&G_IX_0), Some(&G_IX_1), Some(&G_IX_2), Some(&G_IX_3), Some(&G_IX_4), Some(&G_IX_5), Some(&G_IX_6), Some(&G_IX_7), Some(&G_IX_8), Some(&G_IX_9)] },
    Set { key: "lsicon", label: "LSIcon", glyphs: [Some(&G_LSICON_0), Some(&G_LSICON_1), Some(&G_LSICON_2), Some(&G_LSICON_3), Some(&G_LSICON_4), Some(&G_LSICON_5), Some(&G_LSICON_6), Some(&G_LSICON_7), Some(&G_LSICON_8), Some(&G_LSICON_9)] },
    Set { key: "meteor", label: "Meteor", glyphs: [Some(&G_METEOR_0), Some(&G_METEOR_1), Some(&G_METEOR_2), Some(&G_METEOR_3), Some(&G_METEOR_4), Some(&G_METEOR_5), Some(&G_METEOR_6), Some(&G_METEOR_7), Some(&G_METEOR_8), Some(&G_METEOR_9)] },
    Set { key: "charm", label: "Charm", glyphs: [Some(&G_CHARM_0), Some(&G_CHARM_1), Some(&G_CHARM_2), Some(&G_CHARM_3), Some(&G_CHARM_4), Some(&G_CHARM_5), Some(&G_CHARM_6), Some(&G_CHARM_7), Some(&G_CHARM_8), Some(&G_CHARM_9)] },
    Set { key: "mono", label: "Mono Icons", glyphs: [Some(&G_MONO_0), Some(&G_MONO_1), Some(&G_MONO_2), Some(&G_MONO_3), Some(&G_MONO_4), Some(&G_MONO_5), Some(&G_MONO_6), Some(&G_MONO_7), Some(&G_MONO_8), Some(&G_MONO_9)] },
    Set { key: "iconamoon", label: "Iconamoon", glyphs: [Some(&G_ICONAMOON_0), Some(&G_ICONAMOON_1), Some(&G_ICONAMOON_2), Some(&G_ICONAMOON_3), Some(&G_ICONAMOON_4), Some(&G_ICONAMOON_5), Some(&G_ICONAMOON_6), Some(&G_ICONAMOON_7), Some(&G_ICONAMOON_8), Some(&G_ICONAMOON_9)] },
    Set { key: "solar_bold", label: "Solar · 粗", glyphs: [Some(&G_SOLAR_BOLD_0), Some(&G_SOLAR_BOLD_1), Some(&G_SOLAR_BOLD_2), Some(&G_SOLAR_BOLD_3), Some(&G_SOLAR_BOLD_4), Some(&G_SOLAR_BOLD_5), Some(&G_SOLAR_BOLD_6), Some(&G_SOLAR_BOLD_7), Some(&G_SOLAR_BOLD_8), Some(&G_SOLAR_BOLD_9)] },
    Set { key: "solar_linear", label: "Solar · 线", glyphs: [Some(&G_SOLAR_LINEAR_0), Some(&G_SOLAR_LINEAR_1), Some(&G_SOLAR_LINEAR_2), Some(&G_SOLAR_LINEAR_3), Some(&G_SOLAR_LINEAR_4), Some(&G_SOLAR_LINEAR_5), Some(&G_SOLAR_LINEAR_6), Some(&G_SOLAR_LINEAR_7), Some(&G_SOLAR_LINEAR_8), Some(&G_SOLAR_LINEAR_9)] },
];

pub fn find(key: &str) -> Option<&'static Set> {
    SETS.iter().find(|s| s.key == key)
}

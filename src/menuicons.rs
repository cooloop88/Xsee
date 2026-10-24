// 由 work/qc/gen_menuicons.py 生成，别手改。
//
// 菜单（顶部菜单、右键菜单、子菜单）左边那一列图标。固定用 Tabler·线这一套，
// 不跟着版式换：菜单是密排的文字列表，图标粗细随版式跳会变乱。
// 图形原样取自 tabler，命令已摊平成运行时解析器认识的那几种。

use crate::glyphs::{Glyph, Part};

static G_adjustments: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 10 A 2 2 0 1 0 8 10 A 2 2 0 0 0 4 10 M 6 4 V 8 M 6 12 V 20 M 10 16 A 2 2 0 1 0 14 16 A 2 2 0 0 0 10 16 M 12 4 V 14 M 12 18 V 20 M 16 7 A 2 2 0 1 0 20 7 A 2 2 0 0 0 16 7 M 18 4 V 5 M 18 9 V 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_alphabet_latin: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6 10 H 8 A 2 2 0 0 1 10 12 V 17 H 7 A 2 2 0 1 1 7 13 H 10 M 14 7 V 17 M 14 12 A 2 2 0 0 1 16 10 H 17 A 2 2 0 0 1 19 12 V 15 A 2 2 0 0 1 17 17 H 16 A 2 2 0 0 1 14 15 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_apps: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 5 A 1 1 0 0 1 5 4 H 9 A 1 1 0 0 1 10 5 V 9 A 1 1 0 0 1 9 10 H 5 A 1 1 0 0 1 4 9 z M 4 15 A 1 1 0 0 1 5 14 H 9 A 1 1 0 0 1 10 15 V 19 A 1 1 0 0 1 9 20 H 5 A 1 1 0 0 1 4 19 z M 14 15 A 1 1 0 0 1 15 14 H 19 A 1 1 0 0 1 20 15 V 19 A 1 1 0 0 1 19 20 H 15 A 1 1 0 0 1 14 19 z M 14 7 H 20 M 17 4 V 10", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_arrow_bar_up: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 4 V 14 M 12 4 L 16 8 M 12 4 L 8 8 M 4 20 H 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_arrow_forward_up: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15 14 L 19 10 L 15 6", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 19 10 H 8 A 4 4 0 1 0 8 18 H 9", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_arrow_left: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 12 H 19 M 5 12 L 11 18 M 5 12 L 11 6", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_arrow_right: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 12 H 19 M 13 18 L 19 12 M 13 6 L 19 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_arrows_sort: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 9 L 7 5 L 11 9 M 7 5 V 19 M 21 15 L 17 19 L 13 15 M 17 19 V 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_browser: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 8 H 20 M 4 6 A 2 2 0 0 1 6 4 H 18 A 2 2 0 0 1 20 6 V 18 A 2 2 0 0 1 18 20 H 6 A 2 2 0 0 1 4 18 z M 8 4 V 8", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_browser_plus: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 8 H 20 M 12 20 H 6 A 2 2 0 0 1 4 18 V 6 A 2 2 0 0 1 6 4 H 18 A 2 2 0 0 1 20 6 V 12 M 8 4 V 8 M 16 19 H 22 M 19 16 V 22", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_calendar: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 7 A 2 2 0 0 1 6 5 H 18 A 2 2 0 0 1 20 7 V 19 A 2 2 0 0 1 18 21 H 6 A 2 2 0 0 1 4 19 z M 16 3 V 7 M 8 3 V 7 M 4 11 H 20 M 11 15 H 12 M 12 15 V 18", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_category: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 4 H 10 V 10 H 4 z M 14 4 H 20 V 10 H 14 z M 4 14 H 10 V 20 H 4 z M 14 17 A 3 3 0 1 0 20 17 A 3 3 0 1 0 14 17", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_clipboard: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9 5 H 7 A 2 2 0 0 0 5 7 V 19 A 2 2 0 0 0 7 21 H 17 A 2 2 0 0 0 19 19 V 7 A 2 2 0 0 0 17 5 H 15", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 9 5 A 2 2 0 0 1 11 3 H 13 A 2 2 0 0 1 15 5 A 2 2 0 0 1 13 7 H 11 A 2 2 0 0 1 9 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_clock_plus: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20.98 12.54 A 9 9 0 1 0 12.52 20.98 M 16 19 H 22 M 19 16 V 22", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 7 V 12 L 15 15", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_copy: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 9.67 A 2.67 2.67 0 0 1 9.67 7 H 18.33 A 2.67 2.67 0 0 1 21 9.67 V 18.33 A 2.67 2.67 0 0 1 18.33 21 H 9.67 A 2.67 2.67 0 0 1 7 18.33 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4.01 16.74 A 2 2 0 0 1 3 15 V 5 C 3 3.9 3.9 3 5 3 H 15 C 15.75 3 16.16 3.38 16.5 4", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_copy_plus: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7 9.67 A 2.67 2.67 0 0 1 9.67 7 H 18.33 A 2.67 2.67 0 0 1 21 9.67 V 18.33 A 2.67 2.67 0 0 1 18.33 21 H 9.67 A 2.67 2.67 0 0 1 7 18.33 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4.01 16.74 A 2 2 0 0 1 3 15 V 5 C 3 3.9 3.9 3 5 3 H 15 C 15.75 3 16.16 3.38 16.5 4 M 11 14 H 17 M 14 11 V 17", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_database_cog: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 6 C 4 7.66 7.58 9 12 9 S 20 7.66 20 6 S 16.42 3 12 3 S 4 4.34 4 6", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4 6 V 12 C 4 13.66 7.58 15 12 15 C 12.21 15 12.42 15 12.63 14.99 M 20 11.5 V 6", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4 12 V 18 C 4 19.66 7.58 21 12 21 M 17 19 A 2 2 0 1 0 21 19 A 2 2 0 1 0 17 19 M 19 15.5 V 17 M 19 21 V 22.5 M 22.03 17.25 L 20.73 18 M 17.27 20 L 15.97 20.75 M 15.97 17.25 L 17.27 18 M 20.73 20 L 22.03 20.75", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_device_desktop: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 5 A 1 1 0 0 1 4 4 H 20 A 1 1 0 0 1 21 5 V 15 A 1 1 0 0 1 20 16 H 4 A 1 1 0 0 1 3 15 z M 7 20 H 17 M 9 16 V 20 M 15 16 V 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_external_link: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 6 H 6 A 2 2 0 0 0 4 8 V 18 A 2 2 0 0 0 6 20 H 16 A 2 2 0 0 0 18 18 V 12 M 11 13 L 20 4 M 15 4 H 20 V 9", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_eye: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 10 12 A 2 2 0 1 0 14 12 A 2 2 0 0 0 10 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 21 12 C 18.6 16 15.6 18 12 18 C 8.4 18 5.4 16 3 12 C 5.4 8 8.4 6 12 6 C 15.6 6 18.6 8 21 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_file_info: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14 3 V 7 A 1 1 0 0 0 15 8 H 19", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 17 21 H 7 A 2 2 0 0 1 5 19 V 5 A 2 2 0 0 1 7 3 H 14 L 19 8 V 19 A 2 2 0 0 1 17 21", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 11 14 H 12 V 18 H 13 M 12 11 H 12.01", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_file_off: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 3 L 21 21 M 7 3 H 14 L 19 8 V 15 M 19 19 A 2 2 0 0 1 17 21 H 7 A 2 2 0 0 1 5 19 V 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_file_symlink: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 21 V 17 A 3 3 0 0 1 7 14 H 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 9 17 L 12 14 L 9 11 M 14 3 V 7 A 1 1 0 0 0 15 8 H 19", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 5 11 V 5 A 2 2 0 0 1 7 3 H 14 L 19 8 V 19 A 2 2 0 0 1 17 21 H 7.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_filter: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 4 H 20 V 6.17 A 2 2 0 0 1 19.41 7.59 L 15 12 V 19 L 9 21 V 12.5 L 4.52 7.57 A 2 2 0 0 1 4 6.23 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_folder: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 4 H 9 L 12 7 H 19 A 2 2 0 0 1 21 9 V 17 A 2 2 0 0 1 19 19 H 5 A 2 2 0 0 1 3 17 V 6 A 2 2 0 0 1 5 4", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_folder_off: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 4 H 9 L 12 7 H 19 A 2 2 0 0 1 21 9 V 17 M 19 19 H 5 A 2 2 0 0 1 3 17 V 6 A 2 2 0 0 1 4.19 4.17 M 3 3 L 21 21", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_folder_open: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 19 L 7.76 11.65 A 1 1 0 0 1 8.69 11 H 21 A 1 1 0 0 1 21.99 12.16 L 20.99 17.38 A 2 2 0 0 1 19.03 19 z A 2 2 0 0 1 3 17 V 6 A 2 2 0 0 1 5 4 H 9 L 12 7 H 19 A 2 2 0 0 1 21 9 V 11", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_folder_plus: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 12 19 H 5 A 2 2 0 0 1 3 17 V 6 A 2 2 0 0 1 5 4 H 9 L 12 7 H 19 A 2 2 0 0 1 21 9 V 12.5 M 16 19 H 22 M 19 16 V 22", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_folder_search: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11 19 H 5 A 2 2 0 0 1 3 17 V 6 A 2 2 0 0 1 5 4 H 9 L 12 7 H 19 A 2 2 0 0 1 21 9 V 11.5 M 15 18 A 3 3 0 1 0 21 18 A 3 3 0 1 0 15 18 M 20.2 20.2 L 22 22", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_folder_share: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13 19 H 5 A 2 2 0 0 1 3 17 V 6 A 2 2 0 0 1 5 4 H 9 L 12 7 H 19 A 2 2 0 0 1 21 9 V 13 M 16 22 L 21 17 M 21 21.5 V 17 H 16.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_help_circle: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 12 A 9 9 0 1 0 21 12 A 9 9 0 0 0 3 12 M 12 16 V 16.01", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 12 13 A 2 2 0 0 0 12.91 9.22 A 1.98 1.98 0 0 0 10.5 9.7", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_info_circle: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 12 A 9 9 0 1 0 21 12 A 9 9 0 0 0 3 12 M 12 9 H 12.01", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 11 12 H 12 V 16 H 13", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_layout_bottombar: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 6 A 2 2 0 0 1 6 4 H 18 A 2 2 0 0 1 20 6 V 18 A 2 2 0 0 1 18 20 H 6 A 2 2 0 0 1 4 18 z M 4 15 H 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_layout_collage: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 6 A 2 2 0 0 1 6 4 H 18 A 2 2 0 0 1 20 6 V 18 A 2 2 0 0 1 18 20 H 6 A 2 2 0 0 1 4 18 z M 10 4 L 14 20 M 12 12 L 4 14", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_layout_grid: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 5 A 1 1 0 0 1 5 4 H 9 A 1 1 0 0 1 10 5 V 9 A 1 1 0 0 1 9 10 H 5 A 1 1 0 0 1 4 9 z M 14 5 A 1 1 0 0 1 15 4 H 19 A 1 1 0 0 1 20 5 V 9 A 1 1 0 0 1 19 10 H 15 A 1 1 0 0 1 14 9 z M 4 15 A 1 1 0 0 1 5 14 H 9 A 1 1 0 0 1 10 15 V 19 A 1 1 0 0 1 9 20 H 5 A 1 1 0 0 1 4 19 z M 14 15 A 1 1 0 0 1 15 14 H 19 A 1 1 0 0 1 20 15 V 19 A 1 1 0 0 1 19 20 H 15 A 1 1 0 0 1 14 19 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_layout_list: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 6 A 2 2 0 0 1 6 4 H 18 A 2 2 0 0 1 20 6 V 8 A 2 2 0 0 1 18 10 H 6 A 2 2 0 0 1 4 8 z M 4 16 A 2 2 0 0 1 6 14 H 18 A 2 2 0 0 1 20 16 V 18 A 2 2 0 0 1 18 20 H 6 A 2 2 0 0 1 4 18 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_layout_navbar: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 6 A 2 2 0 0 1 6 4 H 18 A 2 2 0 0 1 20 6 V 18 A 2 2 0 0 1 18 20 H 6 A 2 2 0 0 1 4 18 z M 4 9 H 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_layout_navbar_collapse: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 18 V 6 A 2 2 0 0 1 6 4 H 18 A 2 2 0 0 1 20 6 V 18 A 2 2 0 0 1 18 20 H 6 A 2 2 0 0 1 4 18 M 4 9 H 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 10 16 L 12 14 L 14 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_layout_sidebar: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 6 A 2 2 0 0 1 6 4 H 18 A 2 2 0 0 1 20 6 V 18 A 2 2 0 0 1 18 20 H 6 A 2 2 0 0 1 4 18 z M 9 4 V 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_layout_sidebar_right: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 6 A 2 2 0 0 1 6 4 H 18 A 2 2 0 0 1 20 6 V 18 A 2 2 0 0 1 18 20 H 6 A 2 2 0 0 1 4 18 z M 15 4 V 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_list: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 9 6 H 20 M 9 12 H 20 M 9 18 H 20 M 5 6 V 6.01 M 5 12 V 12.01 M 5 18 V 18.01", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_list_details: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 13 5 H 21 M 13 9 H 18 M 13 15 H 21 M 13 19 H 18 M 3 5 A 1 1 0 0 1 4 4 H 8 A 1 1 0 0 1 9 5 V 9 A 1 1 0 0 1 8 10 H 4 A 1 1 0 0 1 3 9 z M 3 15 A 1 1 0 0 1 4 14 H 8 A 1 1 0 0 1 9 15 V 19 A 1 1 0 0 1 8 20 H 4 A 1 1 0 0 1 3 19 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_list_numbers: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 11 6 H 20 M 11 12 H 20 M 12 18 H 20 M 4 16 A 2 2 0 1 1 8 16 C 8 16.59 7.5 17 7 17.5 L 4 20 H 8 M 6 10 V 4 L 4 6", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_logout: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 14 8 V 6 A 2 2 0 0 0 12 4 H 5 A 2 2 0 0 0 3 6 V 18 A 2 2 0 0 0 5 20 H 12 A 2 2 0 0 0 14 18 V 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 9 12 H 21 L 18 9 M 18 15 L 21 12", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_notes: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 5 5 A 2 2 0 0 1 7 3 H 17 A 2 2 0 0 1 19 5 V 19 A 2 2 0 0 1 17 21 H 7 A 2 2 0 0 1 5 19 z M 9 7 H 15 M 9 11 H 15 M 9 15 H 13", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_pencil: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 20 H 8 L 18.5 9.5 A 2.83 2.83 0 1 0 14.5 5.5 L 4 16 z M 13.5 6.5 L 17.5 10.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_photo: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15 8 H 15.01 M 3 6 A 3 3 0 0 1 6 3 H 18 A 3 3 0 0 1 21 6 V 18 A 3 3 0 0 1 18 21 H 6 A 3 3 0 0 1 3 18 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 3 16 L 8 11 C 8.93 10.11 10.07 10.11 11 11 L 16 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 14 14 L 15 13 C 15.93 12.11 17.07 12.11 18 13 L 21 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_photo_edit: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15 8 H 15.01 M 11 20 H 7 A 3 3 0 0 1 4 17 V 7 A 3 3 0 0 1 7 4 H 17 A 3 3 0 0 1 20 7 V 11", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 4 15 L 8 11 C 8.93 10.11 10.07 10.11 11 11 L 14 14 M 14 14 L 15 13 C 15.31 12.7 15.64 12.5 15.99 12.4 M 18.42 15.61 A 2.1 2.1 0 0 1 21.39 18.58 L 18 22 H 15 V 19 z", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_photo_search: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15 8 H 15.01 M 11.5 21 H 6 A 3 3 0 0 1 3 18 V 6 A 3 3 0 0 1 6 3 H 18 A 3 3 0 0 1 21 6 V 11.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 15 18 A 3 3 0 1 0 21 18 A 3 3 0 1 0 15 18 M 20.2 20.2 L 22 22 M 3 16 L 8 11 C 8.93 10.11 10.07 10.11 11 11 L 13 13", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_refresh: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 20 11 A 8.1 8.1 0 0 0 4.5 9 M 4 5 V 9 H 8 M 4 13 A 8.1 8.1 0 0 0 19.5 15 M 20 19 V 15 H 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_scissors: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 7 A 3 3 0 1 0 9 7 A 3 3 0 1 0 3 7 M 3 17 A 3 3 0 1 0 9 17 A 3 3 0 1 0 3 17 M 8.6 8.6 L 19 19 M 8.6 15.4 L 19 5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_search: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 10 A 7 7 0 1 0 17 10 A 7 7 0 1 0 3 10 M 21 21 L 15 15", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_select_all: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 8 9 A 1 1 0 0 1 9 8 H 15 A 1 1 0 0 1 16 9 V 15 A 1 1 0 0 1 15 16 H 9 A 1 1 0 0 1 8 15 z M 12 20 V 20.01 M 16 20 V 20.01 M 8 20 V 20.01 M 4 20 V 20.01 M 4 16 V 16.01 M 4 12 V 12.01 M 4 8 V 8.01 M 4 4 V 4.01 M 8 4 V 4.01 M 12 4 V 4.01 M 16 4 V 4.01 M 20 4 V 4.01 M 20 8 V 8.01 M 20 12 V 12.01 M 20 16 V 16.01 M 20 20 V 20.01", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_sort_ascending: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 6 H 11 M 4 12 H 11 M 4 18 H 13 M 15 9 L 18 6 L 21 9 M 18 6 V 18", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_sort_ascending_letters: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 15 10 V 5 C 15 3.62 15.62 3 17 3 S 19 3.62 19 5 V 10 M 19 7 H 15 M 19 21 H 15 L 19 14 H 15 M 4 15 L 7 18 L 10 15 M 7 6 V 18", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_sort_ascending_numbers: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 15 L 7 18 L 10 15 M 7 6 V 18 M 17 3 A 2 2 0 0 1 19 5 V 8 A 2 2 0 1 1 15 8 V 5 A 2 2 0 0 1 17 3 M 15 16 A 2 2 0 1 0 19 16 A 2 2 0 1 0 15 16", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 19 16 V 19 A 2 2 0 0 1 17 21 H 15.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_sort_descending: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 6 H 13 M 4 12 H 11 M 4 18 H 11 M 15 15 L 18 18 L 21 15 M 18 6 V 18", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_tag_off: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 7.15 7.14 A 0.5 0.5 0 0 0 7.5 8 A 0.5 0.5 0 0 0 7.84 7.87", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 3.88 3.88 A 3 3 0 0 0 3 6 V 11.17 A 2 2 0 0 0 3.59 12.59 L 11.3 20.3 A 2.41 2.41 0 0 0 14.7 20.3 L 17.5 17.5 M 19.5 15.5 L 20.29 14.71 A 2.41 2.41 0 0 0 20.29 11.3 L 12.58 3.59 A 2 2 0 0 0 11.17 3 H 7 M 3 3 L 21 21", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_tag_plus: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 6.5 7.5 A 1 1 0 1 0 8.5 7.5 A 1 1 0 1 0 6.5 7.5", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
        Part { d: "M 21 13 A 2.4 2.4 0 0 0 20.3 11.3 L 12.59 3.59 A 2 2 0 0 0 11.17 3 H 6 A 3 3 0 0 0 3 6 V 11.17 A 2 2 0 0 0 3.59 12.59 L 11.3 20.3 C 11.77 20.77 12.38 21 13 21 M 16 19 H 22 M 19 16 V 22", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_transform: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 3 6 A 3 3 0 1 0 9 6 A 3 3 0 0 0 3 6 M 21 11 V 8 A 2 2 0 0 0 19 6 H 13 L 16 9 M 16 3 L 13 6 M 3 13 V 16 A 2 2 0 0 0 5 18 H 11 L 8 15 M 8 21 L 11 18 M 15 18 A 3 3 0 1 0 21 18 A 3 3 0 0 0 15 18", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_trash: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 7 H 20 M 10 11 V 17 M 14 11 V 17 M 5 7 L 6 19 A 2 2 0 0 0 8 21 H 16 A 2 2 0 0 0 18 19 L 19 7 M 9 7 V 4 A 1 1 0 0 1 10 3 H 14 A 1 1 0 0 1 15 4 V 7", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};
static G_typography: Glyph = Glyph {
    grid: 24.0,
    parts: &[
        Part { d: "M 4 20 H 7 M 14 20 H 21 M 6.9 15 H 13.8 M 10.2 6.3 L 16 20 M 5 20 L 11 4 H 13 L 20 20", fill: false, even: false, alpha: 1.0, w: 2.0, dx: 0.0, dy: 0.0 },
    ],
};

pub static MAP: &[(usize, &Glyph)] = &[
    (101, &G_folder_open),
    (102, &G_folder_search),
    (103, &G_refresh),
    (104, &G_logout),
    (105, &G_arrow_left),
    (106, &G_arrow_right),
    (107, &G_arrow_bar_up),
    (109, &G_search),
    (111, &G_select_all),
    (113, &G_eye),
    (116, &G_folder_share),
    (117, &G_copy_plus),
    (118, &G_trash),
    (119, &G_file_info),
    (120, &G_layout_grid),
    (121, &G_apps),
    (122, &G_list),
    (123, &G_list_details),
    (161, &G_info_circle),
    (260, &G_sort_ascending_letters),
    (261, &G_alphabet_latin),
    (262, &G_sort_ascending_numbers),
    (263, &G_category),
    (264, &G_calendar),
    (265, &G_adjustments),
    (266, &G_notes),
    (267, &G_sort_ascending),
    (268, &G_sort_descending),
    (269, &G_folder_plus),
    (270, &G_clipboard),
    (271, &G_scissors),
    (272, &G_copy),
    (273, &G_pencil),
    (274, &G_photo),
    (275, &G_transform),
    (276, &G_clock_plus),
    (277, &G_help_circle),
    (278, &G_browser_plus),
    (279, &G_photo_edit),
    (280, &G_external_link),
    (282, &G_list_numbers),
    (284, &G_layout_sidebar_right),
    (288, &G_trash),
    (289, &G_copy),
    (290, &G_clipboard),
    (291, &G_pencil),
    (292, &G_tag_plus),
    (300, &G_folder),
    (301, &G_eye),
    (303, &G_typography),
    (304, &G_apps),
    (305, &G_layout_list),
    (306, &G_layout_grid),
    (307, &G_layout_navbar_collapse),
    (308, &G_browser),
    (309, &G_layout_bottombar),
    (312, &G_database_cog),
    (314, &G_folder_off),
    (315, &G_file_off),
    (316, &G_tag_off),
    (317, &G_photo_search),
    (318, &G_transform),
    (331, &G_layout_collage),
    (332, &G_file_symlink),
];

pub static SUBS: &[(&str, &Glyph)] = &[
    ("主工具栏", &G_layout_navbar),
    ("壁纸", &G_device_desktop),
    ("排列图标", &G_arrows_sort),
    ("查看", &G_eye),
    ("浏览器窗格", &G_layout_sidebar),
    ("转到", &G_arrow_forward_up),
    ("过滤", &G_filter),
];

/// 按命令号取图标；没登记的号返回 None，那一行就空着。
pub fn find(id: usize) -> Option<&'static Glyph> {
    MAP.iter().find(|(k, _)| *k == id).map(|(_, g)| *g)
}

/// 带子菜单的那几行没有命令号，按名字前缀认。
pub fn find_label(label: &str) -> Option<&'static Glyph> {
    let head = label.split('(').next().unwrap_or(label);
    SUBS.iter().find(|(k, _)| *k == head).map(|(_, g)| *g)
}

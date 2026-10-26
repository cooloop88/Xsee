# 第三方图标许可清单

界面里那 47 档工具栏图标不是本项目画的，是从 36 个开源图标项目里各取十枚，把 SVG 路径摊平后存成 `skins/图标/*.icons`（一套一个文件，仓库里带着）。菜单（顶部菜单、右键菜单、子菜单）左边那一列 62 枚固定在 Tabler·线 这一套上，不跟版式换，出处同样落在下面第一条。这里逐条记下上游、许可证、版权行和各自的硬性要求。

取图途径统一走 Iconify 的 `@iconify-json/<集合>` 数据包，许可证按上游项目的算。每一档每一枚用的是上游哪个图标名，记在 `LICENSES/glyph-names.json`（档位 key → 命令 → 图标名），下游要逐枚核对出处就照这份查。

**“要求”一栏的口径**：MIT / ISC = 把版权行和许可声明随副本一起给（本文件 + `LICENSES/` 即满足），不要求在界面署名，不传染；Apache-2.0 = 除随附许可全文外，还要保留源文件里的声明并注明这些文件被改过（路径摊平就是改动）；CC BY 4.0 = 必须点名署名 + 给出许可名与链接 + 说明改过，但无 ShareAlike，所以不会传染，与本项目的 GPL-3.0 不冲突。

## 核对过、已经处理掉的三处

1. **Remix Icon（原 `ri_line` / `ri_fill`）**——上游在 2026-01-25 把许可证从 Apache-2.0 换成了自定的 “Remix Icon License v1.0”，第 9 条写明图标始终受该协议约束，并警告它与强制开源（copyleft / share-alike）类许可可能不兼容。本项目按 GPL-3.0 发布正好撞上这一条，**这两档已整体摘除**，`skins/图标/` 下不再有它们的文件，代码里也不再留图形和路径。
2. **Basil（原 `basil`）**——Iconify 与 npm 都记 CC BY 4.0，但上游 Craftwork 没有带 LICENSE 的代码仓库，而其现行条款（2026-10-06 生效）写着“不得拿图标做图标库 / npm 包给别人”“免费素材不得再分发或分享”。两种说法直接冲突且取不到作者第一手原文，**这一档也已整体摘除**。
3. **品牌图标（`mdi`、`bi`）**——Material Design Icons 的许可表脚注写明 Brand / Logos 那一类图标不在此许可覆盖内；Bootstrap Icons 的仓库里也混着 github、facebook 这类品牌图形，而 MIT/Apache 都不授予商标权。这两档各十枚的实际图标名已在 `LICENSES/glyph-names.json` 里逐枚列出：`bi` 是 chevron-double-left / chevron-double-right、arrow-bar-up、search、pencil、arrow-right-square、copy、trash、info-circle、palette；`mdi` 是 chevron-double-left / chevron-double-right、arrow-up、search、pencil、folder-move、clipboard、trash、info-circle、palette。两档**全是功能图形，没有一枚品牌 logo**，因此保留，不必删。新增图标时仍要按这一条口径复查。

## 逐套清单

| 界面档位（key） | 上游项目 | 仓库 | 许可证 | 版权行（照抄上游 LICENSE） |
| --- | --- | --- | --- | --- |
| `tabler_line` `tabler_filled` | Tabler Icons | github.com/tabler/tabler-icons | MIT | Copyright (c) 2020-2026 Paweł Kuna |
| `lucide` | Lucide | github.com/lucide-icons/lucide | ISC + MIT（复合） | ISC：Copyright (c) 2026 Lucide Icons and Contributors；这一档里凡是从 Feather 继承下来的那几枚另按 MIT：Copyright (c) 2013-present Cole Bemis。两行都随副本给出去了，哪几枚算 Feather 衍生以下游可自查上游 LICENSE 为准（见末节） |
| `bi` | Bootstrap Icons | github.com/twbs/icons | MIT | Copyright (c) 2019-2024 The Bootstrap Authors |
| `ph_regular` `ph_bold` `ph_fill` `ph_duotone` `ph_light` | Phosphor Icons | github.com/phosphor-icons/core | MIT | Copyright (c) 2023 Phosphor Icons |
| `iconoir` | Iconoir | github.com/iconoir-icons/iconoir | MIT | Copyright (c) 2021 Luca Burgio |
| `hero_outline` `hero_solid` | Heroicons | github.com/tailwindlabs/heroicons | MIT | Copyright (c) Tailwind Labs, Inc.（上游原文没有年份） |
| `fluent_regular` `fluent_filled` | Fluent UI System Icons | github.com/microsoft/fluentui-system-icons | MIT | Copyright (c) 2020 Microsoft Corporation |
| `mingcute_line` `mingcute_fill` | MingCute Icons | github.com/mingcute-design/mingcute-icons | Apache-2.0 | 上游 LICENSE 的版权占位符未填写，署名 MingCute Design |
| `ms_rounded` | Material Symbols（Google） | github.com/google/material-design-icons | Apache-2.0 | 上游 LICENSE 版权占位符未填写，归属 Google |
| `mdi` | Material Design Icons（Pictogrammers） | github.com/Templarian/MaterialDesign | 图标 Apache-2.0，代码 MIT | 上游许可文本没有版权行，署名 Pictogrammers |
| `carbon` | Carbon Icons（IBM） | github.com/carbon-design-system/carbon（`packages/icons`） | Apache-2.0 | Copyright 2015 IBM Corp. |
| `iconpark_line` | IconPark（字节跳动） | github.com/bytedance/IconPark | Apache-2.0 | Copyright 2019-present Bytedance Inc. |
| `antd_outline` | Ant Design Icons | github.com/ant-design/ant-design-icons | MIT | Copyright (c) 2018-present Ant UED, https://xtech.antfin.com/ |
| `radix` | Radix Icons | github.com/radix-ui/icons | MIT | Copyright (c) 2022 WorkOS |
| `majesticons_line` `majesticons_solid` | Majesticons（免费档） | github.com/halfmage/majesticons | MIT | Copyright (c) 2021 Gerrit Halfmann |
| `pixelarticons` | Pixelarticons | github.com/halfmage/pixelarticons | MIT | Copyright (c) 2019 Gerrit Halfmann |
| `octicon` | Primer Octicons | github.com/primer/octicons | MIT | Copyright (c) 2026 GitHub Inc. |
| `mynaui_outline` `mynaui_solid` | Myna UI Icons | github.com/praveenjuge/mynaui-icons | MIT | Copyright (c) Praveen Juge（上游原文没有年份） |
| `hugeicons` | Hugeicons（免费档） | github.com/hugeicons/hugeicons | MIT | Copyright (c) 2025 Hugeicons |
| `feather` | Feather Icons | github.com/feathericons/feather | MIT | Copyright (c) 2013-2023 Cole Bemis |
| `system_uicons` | System UIcons | github.com/CoreyGinnivan/system-uicons | Unlicense | 无版权行（Unlicense 是放弃版权，零要求，不必署名也不必随附文本） |
| `akar` | Akar Icons | github.com/artcoholic/akar-icons | MIT | Copyright (c) 2020 Arturo Wibawa |
| `tdesign_outline` | TDesign Icons（腾讯） | github.com/Tencent/tdesign-icons | MIT | Copyright (c) 2021-present TDesign |
| `grommet` | Grommet Icons | github.com/grommet/grommet-icons | Apache-2.0 | SPDX-FileCopyrightText: © Hewlett Packard Enterprise Development LP（无年份） |
| `gravity` | Gravity UI Icons | github.com/gravity-ui/icons | MIT | Copyright (c) 2022 YANDEX LLC |
| `qlementine` | Qlementine Icons | github.com/oclero/qlementine-icons | MIT | Copyright (c) 2023 Olivier Cléro |
| `teenyicons` | Teenyicons | github.com/teenyicons/teenyicons.com | MIT | Copyright (c) 2020, Anja van Staden |
| `proicons` | ProIcons | github.com/ProCode-Software/proicons | MIT | Copyright (c) 2025 ProCode |
| `famicons` | Famicons | github.com/familyjs/famicons | MIT | Copyright (c) 2022-present Family |
| `ix` | Siemens IX Icons | github.com/siemens/ix-icons | MIT | Copyright (c) 2022 Siemens AG |
| `lsicon` | LSIcon（Wis Design） | github.com/wisdesignsystem/lsicon | MIT | Copyright (c) 2024 Wis Design |
| `meteor` | Meteor Icons | github.com/zkreations/meteor | MIT | Copyright (c) 2022 zkreations |
| `charm` | Charm Icons | github.com/jaynewey/charm-icons | MIT | Copyright (c) 2021 Jay Newey |
| `mono` | Mono Icons | github.com/mono-company/mono-icons | MIT | Copyright 2020 Mono Company BV |
| `iconamoon` | IconaMoon | github.com/dariushhpg1/IconaMoon | 上游仓库没有 LICENSE 文件；Iconify 记 CC BY 4.0。上游 README 原文是“完全免费，想怎么用就怎么用，可以拷走改成你自己的” | 署名 Dariush Habibpour |
| `solar_bold` `solar_linear` | Solar（480 Design） | 只有 Figma 社区文件，没有官方仓库 | CC BY 4.0 | 署名 480 Design |

## 随附文件

`LICENSES/` 目录放五种许可全文：`MIT.txt`、`ISC.txt`、`Apache-2.0.txt`、`Unlicense.txt`、`CC-BY-4.0.txt`，外加 `glyph-names.json`（47 档 × 10 枚对应的上游图标名）。Apache-2.0 那几家（MingCute、Material Symbols、MDI 图标、Carbon、IconPark、Grommet）要求“注明这些文件被改过”：`skins/图标/*.icons` 里的路径是从上游 SVG 摊平、并把 Q/T 二次曲线换成三次贝塞尔之后的结果，属于已修改；这层说明写在每个 `.icons` 的头部注释里，也写在 `src/glyphs.rs` 的头部注释里。

`system_uicons` 是 Unlicense，随附与否都不欠；这里仍然列出来，只是为了让下游能追溯到上游。

## 没查实的项目

- Ant Design 图标的上游 README 要求投稿图标须来自它在 iconfont.cn 的官方合集，那个合集页面的许可声明取不到原文。仓库侧能举证的只有 MIT 这一份。
- MingCute、Material Symbols、MDI 三家的 LICENSE 是模板、版权行没有填写，只能按项目名/组织署名，年份不编。
- CC BY 4.0 与 GPL-3.0 的兼容：CC BY 4.0 没有 ShareAlike、只额外要求署名，通行做法是可与 GPL-3.0 并存；但 GNU 的许可兼容性页面只明确写了 CC BY-SA 4.0 可单向转 GPLv3，没有直接覆盖 CC BY 4.0 这一条。这句留给下游自己判断，不作为已验证结论。
- Lucide 那十枚里具体哪几枚属于 Feather 衍生（也就是归 MIT 而非 ISC），本机上取不到上游 LICENSE 原文核对（访问 github 被网络挡住），所以没有逐枚标死。做法是把两行版权声明都随副本附上：不管哪一枚落在哪一边，两边要求的“随附声明”都已经满足。下游要精确到枚，照 `LICENSES/glyph-names.json` 的名字去比对上游仓库的 Feather 衍生标记即可。

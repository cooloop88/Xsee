use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Clone)]
pub struct Cat {
    pub id: i64,
    pub name: String,
    pub parent: i64,
}

/// 类别叠加层实际是从哪读出来的，启动流程据此提示用户。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OverlayLoad {
    Empty,
    Main,
    Backup,
    Temp,
    Corrupt,
}

fn overlay_has_rows(text: &str) -> bool {
    text.lines()
        .any(|l| matches!(l.split('\t').next().unwrap_or(""), "N" | "X" | "O" | "A" | "R"))
}

fn read_overlay(path: &Path) -> Option<String> {
    let t = std::fs::read_to_string(path).ok()?;
    overlay_has_rows(&t).then_some(t)
}

/// 自含库的头部一行：CAT 类别数 归类条数。正文里的行数要和它对得上，
/// 对不上就说明这一份是半截的（哪怕它仍是一份合法文本）。
pub fn catalog_ok(text: &str) -> bool {
    let mut lines = text.lines();
    let Some(head) = lines.next() else { return false };
    let mut f = head.split('\t');
    if f.next() != Some("CAT") {
        return false;
    }
    let want_c: usize = f.next().and_then(|x| x.parse().ok()).unwrap_or(usize::MAX);
    let want_r: usize = f.next().and_then(|x| x.parse().ok()).unwrap_or(usize::MAX);
    // 第三档是入库时间条数；早先写下的头没有它，那就只要求没有 T 行
    let want_t: usize = f.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let (mut c, mut r, mut t) = (0usize, 0usize, 0usize);
    for ln in lines {
        match ln.as_bytes().first() {
            Some(b'N') => c += 1,
            Some(b'A') => r += 1,
            Some(b'T') => t += 1,
            _ => {}
        }
    }
    c == want_c && r == want_r && t == want_t
}

fn read_catalog(path: &Path) -> Option<String> {
    let t = std::fs::read_to_string(path).ok()?;
    catalog_ok(&t).then_some(t)
}

/// 先把文本落进临时文件并刷到盘，再原子替换主文件，上一版留成 `.bak`。
pub fn write_catalog(path: &Path, text: &str) -> std::io::Result<()> {
    write_atomic(path, text)
}

/// 先写临时文件并刷到盘，再原子替换：断电/蓝屏时要么旧文件、要么新文件，
/// 不会出现原文件已经删掉、新文件还没改名的那一下空窗。上一版另留一份 `.bak`。
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let tmp = path.with_extension("tmp");
    write_synced(&tmp, text.as_bytes())?;
    // 上一版留一份；主文件本身已经坏了就不要拿它去盖好备份
    if let Ok(old) = std::fs::read_to_string(path) {
        if catalog_ok(&old) {
            let bt = path.with_extension("bak.tmp");
            if write_synced(&bt, old.as_bytes()).is_ok() {
                let _ = replace_file(&bt, &path.with_extension("bak"));
            }
        }
    }
    replace_file(&tmp, path)
}

fn tmp_is_newer(tmp: &Path, main: &Path) -> bool {
    let stamp = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    match (stamp(tmp), stamp(main)) {
        (Some(t), Some(m)) => t >= m,
        (Some(_), None) => true,
        _ => false,
    }
}

fn write_synced(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    f.write_all(data)?;
    f.sync_all()
}

/// 同盘原子替换并等目录项落盘。失败时原文件和临时文件都原样留着。
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };
        use windows::core::PCWSTR;
        let wide = |p: &Path| {
            p.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect::<Vec<u16>>()
        };
        let (a, b) = (wide(from), wide(to));
        unsafe {
            if MoveFileExW(
                PCWSTR(a.as_ptr()),
                PCWSTR(b.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
            .is_ok()
            {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(from, to)
    }
}

/// 可编辑的类别树。类别、顺序、图片归属全都记在自己那份自含库里，
/// 读写都走同一份文件，不依赖任何外部数据库。
pub struct CatEdit {
    pub cats: Vec<Cat>,
    idx: HashMap<i64, usize>,
    pub children: HashMap<i64, Vec<i64>>,
    pub roots: Vec<i64>,
    /// 类别 -> 直属图片路径（不含子类别）。
    direct: HashMap<i64, HashSet<String>>,
    /// 路径 -> 它直属的类别，画缩略图角标用。
    by_path: HashMap<String, HashSet<i64>>,
    base: HashMap<i64, HashSet<String>>,
    adds: HashMap<i64, HashSet<String>>,
    rems: HashMap<i64, HashSet<String>>,
    /// 路径 -> 加入本库的时间（秒）。第一次见到那张图时盖当下时间，之后不再改。
    pub times: HashMap<String, i64>,
    counts: HashMap<i64, usize>,
    new_ids: HashSet<i64>,
    renamed: HashSet<i64>,
    /// 被拖去改了父级的原有类别；父级本身要靠 N 行落盘，O 行只管同级顺序。
    reparented: HashSet<i64>,
    removed: HashSet<i64>,
    next_id: i64,
}

/// 复制类别时拍下的子树快照，粘贴时按它重建。
#[derive(Clone)]
pub struct CatSnap {
    pub name: String,
    pub paths: Vec<String>,
    pub kids: Vec<CatSnap>,
}

impl CatEdit {
    /// 空库起步：新机器上还没有分类库时就是这个样子，类别照建、归类照存，
    /// 全部内容都落在自己那份自含库里。
    pub fn empty() -> CatEdit {
        let mut me = CatEdit {
            cats: Vec::new(),
            idx: HashMap::new(),
            children: HashMap::new(),
            roots: Vec::new(),
            direct: HashMap::new(),
            by_path: HashMap::new(),
            base: HashMap::new(),
            adds: HashMap::new(),
            rems: HashMap::new(),
            times: HashMap::new(),
            counts: HashMap::new(),
            new_ids: HashSet::new(),
            renamed: HashSet::new(),
            reparented: HashSet::new(),
            removed: HashSet::new(),
            // 本地新建的类别 id 从十万往上起号，留出一大段低号位给以后可能接进来的外部库
            next_id: 100_001,
        };
        me.rebuild_counts();
        me
    }

    pub fn name(&self, id: i64) -> String {
        self.idx.get(&id).map(|i| self.cats[*i].name.clone()).unwrap_or_default()
    }

    pub fn child_ids(&self, id: i64) -> &[i64] {
        self.children.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn count(&self, id: i64) -> usize {
        self.counts.get(&id).copied().unwrap_or(0)
    }

    /// 图片是否直属这个类别（不含子类别）。
    pub fn direct_has(&self, id: i64, path: &str) -> bool {
        self.direct.get(&id).map(|s| s.contains(path)).unwrap_or(false)
    }

    /// 图片挂在任意类别上（用来画缩略图角标）。
    pub fn any_cat(&self, path: &str) -> bool {
        self.by_path.contains_key(path)
    }

    /// 至少挂在一个类别上的图片张数。
    pub fn n_categorized(&self) -> usize {
        self.by_path.len()
    }

    /// 同上，给出名单，按路径排序。一张图挂在几个类别里也只出现一次。
    /// 和 files() 同一个办法：先排引用，最后一步才复制。21338 张那一档要是
    /// 每个键都先转成小写字符串，光排序就几十毫秒。
    pub fn categorized_files(&self) -> Vec<String> {
        let mut refs: Vec<&str> = self.by_path.keys().map(|s| s.as_str()).collect();
        refs.sort_by(|a, b| cmp_path(a, b));
        refs.into_iter().map(|p| p.to_string()).collect()
    }

    /// (paths 里有几张直属这个类别, paths 一共几张)
    pub fn state(&self, paths: &[String], id: i64) -> (usize, usize) {
        let n = paths.iter().filter(|p| self.direct_has(id, p)).count();
        (n, paths.len())
    }

    fn in_subtree(&self, mut id: i64, root: i64) -> bool {
        let mut guard = 0;
        loop {
            if id == root {
                return true;
            }
            let Some(&ci) = self.idx.get(&id) else { return false };
            let p = self.cats[ci].parent;
            if p == 0 || guard >= 64 {
                return false;
            }
            id = p;
            guard += 1;
        }
    }

    fn path_in(&self, path: &str, id: i64) -> bool {
        let Some(set) = self.by_path.get(path) else { return false };
        set.iter().any(|&c| self.in_subtree(c, id))
    }

    /// (paths 里有几张在这个类别的整棵子树里, paths 一共几张)。
    /// 父类别默认折叠，所以勾选状态得把子类别里的图也算上。
    pub fn state_sub(&self, paths: &[String], id: i64) -> (usize, usize) {
        let n = paths.iter().filter(|p| self.path_in(p, id)).count();
        (n, paths.len())
    }

    /// 含全部子类别的图片，排序去重。点开大类是常事：先拿引用排好、去重，最后一步才
    /// 复制成 owned，排序也不给每个键另开一份小写字符串——13842 张那一档
    /// 实测从 44 毫秒降到 14 毫秒。
    pub fn files(&self, id: i64) -> Vec<String> {
        let mut refs: Vec<&str> = Vec::new();
        let mut seen: HashSet<i64> = HashSet::new();
        let mut stack = vec![id];
        while let Some(c) = stack.pop() {
            if !seen.insert(c) {
                continue;
            }
            if let Some(v) = self.direct.get(&c) {
                refs.extend(v.iter().map(|s| s.as_str()));
            }
            if let Some(k) = self.children.get(&c) {
                stack.extend(k.iter().copied());
            }
        }
        refs.sort_by(|a, b| cmp_path(a, b));
        refs.dedup();
        refs.into_iter().map(|p| p.to_string()).collect()
    }

    pub fn toggle_paths(&mut self, id: i64, paths: &[String], add: bool) {
        for p in paths {
            if add {
                self.mark_add(id, p);
            } else {
                self.mark_remove(id, p);
            }
        }
        self.rebuild_counts();
    }

    fn mark_add(&mut self, id: i64, p: &str) {
        if self.base.get(&id).map(|b| b.contains(p)).unwrap_or(false) {
            self.adds.entry(id).or_default().remove(p);
        } else {
            self.adds.entry(id).or_default().insert(p.to_string());
        }
        self.rems.entry(id).or_default().remove(p);
        self.direct.entry(id).or_default().insert(p.to_string());
        self.by_path.entry(p.to_string()).or_default().insert(id);
    }

    fn mark_remove(&mut self, id: i64, p: &str) {
        let had = match self.direct.get_mut(&id) {
            Some(s) => s.remove(p),
            None => false,
        };
        if had && self.base.get(&id).map(|b| b.contains(p)).unwrap_or(false) {
            self.rems.entry(id).or_default().insert(p.to_string());
        }
        self.adds.entry(id).or_default().remove(p);
        if let Some(set) = self.by_path.get_mut(p) {
            set.remove(&id);
            if set.is_empty() {
                self.by_path.remove(p);
            }
        }
    }

    /// 父类别行的勾选框：加入只落在父类别自己身上，移出则清整棵子树，
    /// 否则点一下取消勾选，折叠状态看起来毫无变化。
    pub fn toggle_paths_sub(&mut self, id: i64, paths: &[String], add: bool) {
        if add {
            for p in paths {
                self.mark_add(id, p);
            }
        } else {
            let ids = self.subtree_ids(id);
            for p in paths {
                for c in &ids {
                    self.mark_remove(*c, p);
                }
            }
        }
        self.rebuild_counts();
    }

    fn subtree_ids(&self, id: i64) -> Vec<i64> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![id];
        while let Some(c) = stack.pop() {
            if !seen.insert(c) {
                continue;
            }
            out.push(c);
            if let Some(k) = self.children.get(&c) {
                stack.extend(k.iter().copied());
            }
        }
        out
    }

    /// 软件内把图片挪到别处：分类里的路径跟着换，重启后还能找回原图。
    pub fn rename_path(&mut self, old: &str, new: &str) {
        if old.is_empty() || new.is_empty() || old == new {
            return;
        }
        let new_s = new.to_string();
        let ids: Vec<i64> = self
            .direct
            .iter()
            .filter(|(_, s)| s.contains(old))
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            if self.base.get(&id).map(|b| b.contains(old)).unwrap_or(false) {
                if let Some(b) = self.base.get_mut(&id) {
                    b.remove(old);
                }
                self.rems.entry(id).or_default().insert(old.to_string());
                self.adds.entry(id).or_default().insert(new_s.clone());
            } else if self.adds.get_mut(&id).map(|s| s.remove(old)).unwrap_or(false) {
                self.adds.entry(id).or_default().insert(new_s.clone());
            }
            self.rems.entry(id).or_default().remove(&new_s);
            if let Some(s) = self.direct.get_mut(&id) {
                s.remove(old);
                s.insert(new_s.clone());
            }
        }
        if let Some(set) = self.by_path.remove(old) {
            self.by_path.entry(new_s.clone()).or_default().extend(set);
        }
        // 入库时间跟着搬：不然新路径没有 T 行，"按入库时间"名单里这张图就丢了，
        // 等以后浏览到才以当下时间重新记入，原始时间永久丢失。
        if let Some(t) = self.times.remove(old) {
            self.times.entry(new_s.clone()).or_insert(t);
        }
        self.rebuild_counts();
    }

    /// 文件被删除：把分类里的路径一并摘掉，免得类别打开时全是死条目。
    pub fn forget_path(&mut self, p: &str) {
        let ids: Vec<i64> = self
            .direct
            .iter()
            .filter(|(_, s)| s.contains(p))
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            self.mark_remove(id, p);
        }
        // T 行跟着摘掉，不然死条目要等"检查数据库"才清；同路径再来新文件
        // 还会沿用旧时间戳。
        self.times.remove(p);
        self.rebuild_counts();
    }

    pub fn rename_cat(&mut self, id: i64, name: &str) -> bool {
        let Some(&ci) = self.idx.get(&id) else { return false };
        if self.cats[ci].name == name {
            return true;
        }
        self.cats[ci].name = name.to_string();
        self.renamed.insert(id);
        true
    }

    /// 删除类别连同整棵子树，返回被删的 id 列表。
    pub fn del_cat(&mut self, id: i64) -> Vec<i64> {
        if id == 0 || !self.idx.contains_key(&id) {
            return Vec::new();
        }
        let was_new = self.new_ids.contains(&id);
        let ids = self.remove_subtree(id);
        for c in &ids {
            self.removed.remove(c);
        }
        if !was_new {
            self.removed.insert(id);
        }
        self.rebuild_counts();
        ids
    }

    fn remove_subtree(&mut self, id: i64) -> Vec<i64> {
        let parent = self
            .idx
            .get(&id)
            .and_then(|&ci| self.cats.get(ci))
            .map(|c| c.parent)
            .unwrap_or(0);
        let ids = self.subtree_ids(id);
        if ids.is_empty() {
            return ids;
        }
        {
            let list = if parent == 0 || !self.idx.contains_key(&parent) {
                &mut self.roots
            } else {
                self.children.entry(parent).or_default()
            };
            list.retain(|x| *x != id);
        }
        for c in &ids {
            self.children.remove(c);
            self.idx.remove(c);
            self.direct.remove(c);
            self.base.remove(c);
            self.adds.remove(c);
            self.rems.remove(c);
            self.counts.remove(c);
            self.new_ids.remove(c);
            self.renamed.remove(c);
            self.reparented.remove(c);
        }
        for set in self.by_path.values_mut() {
            for c in &ids {
                set.remove(c);
            }
        }
        self.by_path.retain(|_, s| !s.is_empty());
        ids
    }

    pub fn snapshot(&self, id: i64) -> Option<CatSnap> {
        if id == 0 || !self.idx.contains_key(&id) {
            return None;
        }
        let mut paths: Vec<String> = self.direct.get(&id).map(|s| s.iter().cloned().collect()).unwrap_or_default();
        paths.sort();
        let kids = self.children.get(&id).cloned().unwrap_or_default();
        Some(CatSnap {
            name: self.name(id),
            paths,
            kids: kids.iter().filter_map(|k| self.snapshot(*k)).collect(),
        })
    }

    pub fn paste_cat(&mut self, parent: i64, snap: &CatSnap) -> i64 {
        let parent = if parent != 0 && self.idx.contains_key(&parent) { parent } else { 0 };
        let name = self.unique_name(parent, &snap.name);
        let id = self.new_cat(parent, &name);
        for p in &snap.paths {
            self.mark_add(id, p);
        }
        for k in &snap.kids {
            self.paste_cat(id, k);
        }
        self.rebuild_counts();
        id
    }

    fn unique_name(&self, parent: i64, base: &str) -> String {
        let siblings: &[i64] = if parent == 0 {
            &self.roots
        } else {
            self.children.get(&parent).map(|v| v.as_slice()).unwrap_or(&[])
        };
        let taken = |name: &str| siblings.iter().any(|id| self.name(*id).eq_ignore_ascii_case(name));
        if !taken(base) {
            return base.to_string();
        }
        let first = format!("{base} 副本");
        if !taken(&first) {
            return first;
        }
        for n in 2..1000 {
            let s = format!("{base} 副本 {n}");
            if !taken(&s) {
                return s;
            }
        }
        format!("{base} 副本 {}", self.next_id)
    }

    pub fn new_cat(&mut self, parent: i64, name: &str) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        let parent = if parent != 0 && self.idx.contains_key(&parent) { parent } else { 0 };
        self.cats.push(Cat { id, name: name.to_string(), parent });
        self.idx.insert(id, self.cats.len() - 1);
        if parent == 0 {
            self.roots.push(id);
        } else {
            self.children.entry(parent).or_default().push(id);
        }
        self.new_ids.insert(id);
        self.counts.insert(id, 0);
        id
    }

    /// 在同一级里挪动位置，delta=-1 上移、+1 下移。
    pub fn move_cat(&mut self, id: i64, delta: i32) -> bool {
        let Some(&ci) = self.idx.get(&id) else { return false };
        let parent = self.cats[ci].parent;
        let is_root = parent == 0 || !self.idx.contains_key(&parent);
        let list = if is_root {
            &mut self.roots
        } else {
            self.children.entry(parent).or_default()
        };
        let Some(pos) = list.iter().position(|x| *x == id) else { return false };
        let np = pos as i32 + delta;
        if np < 0 || np as usize >= list.len() {
            return false;
        }
        list.swap(pos, np as usize);
        true
    }

    /// 把类别挂到另一个类别下面，parent=0 表示回到顶层。
    /// 拖进自己的子树会成环，直接拒掉。
    pub fn set_parent(&mut self, id: i64, parent: i64) -> bool {
        if id == 0 || !self.idx.contains_key(&id) {
            return false;
        }
        if parent != 0 && !self.idx.contains_key(&parent) {
            return false;
        }
        if self.in_subtree(parent, id) {
            return false;
        }
        let ci = self.idx[&id];
        let old = self.cats[ci].parent;
        if old == parent {
            return true;
        }
        {
            let list = if old == 0 || !self.idx.contains_key(&old) {
                &mut self.roots
            } else {
                self.children.entry(old).or_default()
            };
            list.retain(|x| *x != id);
        }
        self.cats[ci].parent = parent;
        if parent == 0 {
            self.roots.push(id);
        } else {
            self.children.entry(parent).or_default().push(id);
        }
        self.reparented.insert(id);
        self.rebuild_counts();
        true
    }

    pub fn load_overlay(&mut self, path: &Path) -> OverlayLoad {
        let bak = path.with_extension("bak");
        let tmp = path.with_extension("tmp");
        // 临时文件还在且完整、又不比主文件旧，说明上次保存写完盘但没来得及替换——
        // 它才是最新的，优先拿它，死机前最后那一笔操作才不会白做。
        let main_text = read_overlay(path);
        let (text, state) = if let Some(t) =
            read_overlay(&tmp).filter(|_| main_text.is_none() || tmp_is_newer(&tmp, path))
        {
            let _ = replace_file(&tmp, path);
            (t, OverlayLoad::Temp)
        } else if let Some(t) = main_text {
            (t, OverlayLoad::Main)
        } else if let Some(t) = read_overlay(&bak) {
            let _ = write_synced(&tmp, t.as_bytes())
                .and_then(|()| replace_file(&tmp, path));
            (t, OverlayLoad::Backup)
        } else if path.exists() {
            // 主文件在但读不出有效内容：先把现场改名留档，
            // 不能让下一次保存把它直接盖掉。
            let _ = std::fs::rename(path, path.with_extension("corrupt"));
            (String::new(), OverlayLoad::Corrupt)
        } else {
            (String::new(), OverlayLoad::Empty)
        };
        for line in text.lines() {
            let f: Vec<&str> = line.splitn(4, '\t').collect();
            if f.len() < 4 || f[0] != "N" {
                continue;
            }
            let id: i64 = f[1].parse().unwrap_or(0);
            let parent: i64 = f[2].parse().unwrap_or(0);
            let name = unesc(f[3]);
            if id == 0 || name.is_empty() {
                continue;
            }
            if let Some(&ci) = self.idx.get(&id) {
                // 库里原有的类别：改名要认，换父级也要认。原来只改名，于是被拖到
                // 顶层的类别一重启又回到原来的父级列表里，而下面的 O 行还会把它
                // 写进 roots——同一个类别在树里就成了两行，点一个另一个跟着亮。
                self.cats[ci].name = name;
                self.renamed.insert(id);
                let cur = self.cats[ci].parent;
                let want = if parent != 0 && self.idx.contains_key(&parent) { parent } else { 0 };
                if cur != want {
                    self.set_parent(id, want);
                }
                continue;
            }
            let parent = if parent != 0 && self.idx.contains_key(&parent) { parent } else { 0 };
            self.cats.push(Cat { id, name, parent });
            self.idx.insert(id, self.cats.len() - 1);
            if parent == 0 {
                self.roots.push(id);
            } else {
                self.children.entry(parent).or_default().push(id);
            }
            self.new_ids.insert(id);
            self.next_id = self.next_id.max(id.saturating_add(1));
        }
        // 删除标记要在排序和归属之前落地，后面的 O/A/R 自然只认还活着的类别
        let mut dels: Vec<i64> = Vec::new();
        for line in text.lines() {
            let f: Vec<&str> = line.splitn(2, '\t').collect();
            if f.len() < 2 || f[0] != "X" {
                continue;
            }
            if let Ok(id) = f[1].trim().parse::<i64>() {
                if id != 0 {
                    dels.push(id);
                }
            }
        }
        for id in dels {
            self.remove_subtree(id);
            self.removed.insert(id);
        }
        for line in text.lines() {
            let f: Vec<&str> = line.splitn(3, '\t').collect();
            if f.len() < 3 || f[0] != "O" {
                continue;
            }
            let parent: i64 = f[1].parse().unwrap_or(0);
            // O 行只管同一层里的先后顺序，不管归属：只有本来就是这个父级的才收进来。
            // 早先按"id 还活着"过滤，于是 N 行已经把类别挪到顶层之后，父级那一行的
            // O 顺序表又把它塞回原来的子列表——顶层和子列表各一行，选中一个另一个跟着亮。
            let mut ids: Vec<i64> = f[2]
                .split(',')
                .filter_map(|x| x.parse::<i64>().ok())
                .filter(|i| self.idx.get(i).is_some_and(|c| self.cats[*c].parent == parent))
                .collect();
            let cur: Vec<i64> = if parent == 0 {
                self.roots.clone()
            } else {
                self.children.get(&parent).cloned().unwrap_or_default()
            };
            for id in cur {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            if parent == 0 {
                self.roots = ids;
            } else {
                self.children.insert(parent, ids);
            }
        }
        let mut extra: Vec<(i64, String)> = Vec::new();
        for line in text.lines() {
            let f: Vec<&str> = line.splitn(3, '\t').collect();
            if f.len() < 3 {
                continue;
            }
            let cid: i64 = f[1].parse().unwrap_or(0);
            if cid == 0 || !self.idx.contains_key(&cid) {
                continue;
            }
            let p = unesc(f[2]);
            if p.is_empty() {
                continue;
            }
            match f[0] {
                "R" => {
                    self.adds.entry(cid).or_default().remove(&p);
                    self.rems.entry(cid).or_default().insert(p.clone());
                    self.raw_remove(cid, &p);
                }
                "A" => {
                    self.rems.entry(cid).or_default().remove(&p);
                    self.adds.entry(cid).or_default().insert(p.clone());
                    extra.push((cid, p));
                }
                _ => {}
            }
        }
        for (cid, p) in extra {
            self.raw_add(cid, &p);
        }
        self.rebuild_counts();
        state
    }

    /// 读那份自含全量库。临时文件比主文件新，说明上次写完盘没来得及替换，先认它；
    /// 主文件正文行数对不上头部记的数就退到上一版；上一版也不对就把现场改名留档，从空库开始。
    pub fn load_full(path: &Path) -> (CatEdit, OverlayLoad) {
        let bak = path.with_extension("bak");
        let tmp = path.with_extension("tmp");
        let main_text = read_catalog(path);
        let (text, state) = if let Some(t) =
            read_catalog(&tmp).filter(|_| main_text.is_none() || tmp_is_newer(&tmp, path))
        {
            let _ = replace_file(&tmp, path);
            (t, OverlayLoad::Temp)
        } else if let Some(t) = main_text {
            (t, OverlayLoad::Main)
        } else if let Some(t) = read_catalog(&bak) {
            let _ = write_atomic(path, &t);
            (t, OverlayLoad::Backup)
        } else if path.exists() || tmp.exists() || bak.exists() {
            // 文件在但读不出自洽的内容：先把现场改名留档，不能让下一次保存盖掉它
            let _ = std::fs::rename(path, path.with_extension("corrupt"));
            (String::new(), OverlayLoad::Corrupt)
        } else {
            (String::new(), OverlayLoad::Empty)
        };
        let mut me = CatEdit::empty();
        let mut rows: Vec<(i64, i64, String)> = Vec::new();
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.splitn(4, '\t').collect();
            if f.len() < 4 || f[0] != "N" {
                continue;
            }
            let id: i64 = f[1].parse().unwrap_or(0);
            let parent: i64 = f[2].parse().unwrap_or(0);
            let name = unesc(f[3]);
            if id == 0 || name.is_empty() {
                continue;
            }
            rows.push((id, parent, name));
        }
        for (id, _, name) in &rows {
            if me.idx.contains_key(id) {
                continue;
            }
            me.cats.push(Cat { id: *id, name: name.clone(), parent: 0 });
            me.idx.insert(*id, me.cats.len() - 1);
        }
        // 父级那一行可能排在后面，第二遍才挂树
        for (id, parent, _) in &rows {
            let ci = me.idx[id];
            let want = if *parent != 0 && me.idx.contains_key(parent) { *parent } else { 0 };
            me.cats[ci].parent = want;
            if want == 0 {
                me.roots.push(*id);
            } else {
                me.children.entry(want).or_default().push(*id);
            }
        }
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.splitn(3, '\t').collect();
            if f.len() < 3 || f[0] != "O" {
                continue;
            }
            let parent: i64 = f[1].parse().unwrap_or(0);
            let ids: Vec<i64> = f[2]
                .split(',')
                .filter_map(|x| x.parse::<i64>().ok())
                .filter(|i| me.idx.get(i).is_some_and(|c| me.cats[*c].parent == parent))
                .collect();
            if parent == 0 {
                me.roots = ids;
            } else if !ids.is_empty() {
                me.children.insert(parent, ids);
            }
        }
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.splitn(3, '\t').collect();
            if f.len() < 3 || f[0] != "A" {
                continue;
            }
            let cid: i64 = f[1].parse().unwrap_or(0);
            let p = unesc(f[2]);
            if cid == 0 || p.is_empty() || !me.idx.contains_key(&cid) {
                continue;
            }
            me.raw_add(cid, &p);
        }
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.splitn(3, '\t').collect();
            if f.len() < 3 || f[0] != "T" {
                continue;
            }
            let p = unesc(f[1]);
            let t: i64 = f[2].parse().unwrap_or(0);
            if !p.is_empty() && t != 0 {
                me.times.insert(p, t);
            }
        }
        // 本地新建的类别从十万往上起号，低号段整片留空，以后接别处的库也不会撞号
        me.next_id = rows.iter().map(|r| r.0).max().unwrap_or(0).saturating_add(1).max(100_001);
        me.base = me.direct.clone();
        me.rebuild_counts();
        (me, state)
    }

    /// 当下这份库的全量文本。落盘和"从别的库翻译过来"共用一条序列化路径，
    /// 免得两种写法各说各话。整份重排在每笔操作后都要跑一遍，所以一路直接写进
    /// 同一个 String：不逐行 format!，不逐行转义复制。
    pub fn dump(&self) -> String {
        use std::fmt::Write;
        let mut ids: Vec<i64> = self.idx.keys().copied().collect();
        ids.sort_unstable();
        let rows: usize = self.direct.values().map(|s| s.len()).sum();
        let mut out = String::with_capacity(64 + rows * 90 + self.times.len() * 90 + ids.len() * 40);
        let _ = write!(out, "CAT\t{}\t{rows}\t{}\n", ids.len(), self.times.len());
        for id in &ids {
            let c = &self.cats[self.idx[id]];
            let _ = write!(out, "N\t{id}\t{}\t", c.parent);
            esc_into(&mut out, &c.name);
            out.push('\n');
        }
        let _ = write!(out, "O\t0\t");
        for (i, k) in self.roots.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(out, "{k}");
        }
        out.push('\n');
        let mut parents: Vec<(i64, &[i64])> = self.children.iter().map(|(p, v)| (*p, v.as_slice())).collect();
        parents.sort_unstable_by_key(|x| x.0);
        for (p, kids) in parents {
            if kids.is_empty() {
                continue;
            }
            let _ = write!(out, "O\t{p}\t");
            for (i, k) in kids.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{k}");
            }
            out.push('\n');
        }
        for id in &ids {
            let Some(set) = self.direct.get(id) else { continue };
            let mut ps: Vec<&String> = set.iter().collect();
            ps.sort_unstable();
            for p in ps {
                let _ = write!(out, "A\t{id}\t");
                esc_into(&mut out, p);
                out.push('\n');
            }
        }
        let mut ts: Vec<(&String, &i64)> = self.times.iter().collect();
        ts.sort_unstable_by(|a, b| a.0.cmp(b.0));
        for (p, t) in ts {
            out.push_str("T\t");
            esc_into(&mut out, p);
            let _ = write!(out, "\t{t}\n");
        }
        out
    }

    /// 一份自含的全量库落盘：类别、每层先后、每条归类都写进去，不依赖别的文件。
    /// 开头一行先记类别数和归类条数，读的时候数对不上就当这一份坏了，往上一版退。
    pub fn save_full(&self, path: &Path) -> std::io::Result<()> {
        write_atomic(path, &self.dump())
    }

    /// 库里记着的所有图片路径（归类 ∪ 入库时间名单）。检查时先把名单取出去，
    /// 到锁外面逐张问硬盘在不在，再回来摘——攥着锁 stat 四万张会把界面冻住。
    pub fn known_paths(&self) -> Vec<String> {
        let mut all: HashSet<String> = HashSet::new();
        for set in self.direct.values() {
            all.extend(set.iter().cloned());
        }
        all.extend(self.times.keys().cloned());
        let mut v: Vec<String> = all.into_iter().collect();
        v.sort();
        v
    }

    /// 把这些已经不在盘上的图片从归类关系和入库时间名单里一并摘掉；返回动了多少张。
    pub fn forget_paths(&mut self, gone: &[String]) -> usize {
        let mut n = 0usize;
        for p in gone {
            let had = self.times.remove(p).is_some();
            if let Some(cids) = self.by_path.remove(p) {
                for cid in cids {
                    if let Some(s) = self.direct.get_mut(&cid) {
                        s.remove(p);
                    }
                }
                n += 1;
            } else if had {
                n += 1;
            }
        }
        if n > 0 {
            self.rebuild_counts();
        }
        n
    }

    /// 这张图入库的时间（秒）。没记过就是 None，排序时排到最后。
    pub fn time_of(&self, path: &str) -> Option<i64> {
        self.times.get(path).copied()
    }

    /// 第一次在本库里见到这张图就把时间记下；已经有的不动（那是它当年入库的时间）。
    pub fn note_time(&mut self, path: &str, secs: i64) -> bool {
        if path.is_empty() || self.times.contains_key(path) {
            return false;
        }
        self.times.insert(path.to_string(), secs);
        true
    }

    /// 库里记着的图片条数（按入库时间那份名单算）。
    pub fn n_assets(&self) -> usize {
        self.times.len()
    }

    /// 记着的归类条数（检查时拿来报数）。
    pub fn n_rows(&self) -> usize {
        self.direct.values().map(|s| s.len()).sum()
    }

    /// 还活着的类别数。
    pub fn n_cats(&self) -> usize {
        self.idx.len()
    }

    fn raw_add(&mut self, cid: i64, p: &str) {
        self.direct.entry(cid).or_default().insert(p.to_string());
        self.by_path.entry(p.to_string()).or_default().insert(cid);
    }

    fn raw_remove(&mut self, cid: i64, p: &str) {
        if let Some(s) = self.direct.get_mut(&cid) {
            s.remove(p);
        }
        if let Some(s) = self.by_path.get_mut(p) {
            s.remove(&cid);
            if s.is_empty() {
                self.by_path.remove(p);
            }
        }
    }

    fn rebuild_counts(&mut self) {
        self.counts.clear();
        let mut seen = HashSet::new();
        let roots = self.roots.clone();
        for id in roots {
            count_node(id, &self.children, &self.direct, &mut self.counts, &mut seen, 0);
        }
        let all: Vec<i64> = self.idx.keys().copied().collect();
        for id in all {
            if !self.counts.contains_key(&id) {
                count_node(id, &self.children, &self.direct, &mut self.counts, &mut seen, 0);
            }
        }
    }
}

/// 按路径比大小写不敏感的顺序。逐字节比，不先转一份小写字符串出来——
/// 结果和 to_ascii_lowercase 之后直接比完全一样（小写化不改长度，前缀短的一方靠前）。
fn cmp_path(a: &str, b: &str) -> std::cmp::Ordering {
    let (x, y) = (a.as_bytes(), b.as_bytes());
    let n = x.len().min(y.len());
    for i in 0..n {
        let (cx, cy) = (x[i].to_ascii_lowercase(), y[i].to_ascii_lowercase());
        if cx != cy {
            return cx.cmp(&cy);
        }
    }
    x.len().cmp(&y.len())
}

fn count_node(
    id: i64,
    children: &HashMap<i64, Vec<i64>>,
    direct: &HashMap<i64, HashSet<String>>,
    out: &mut HashMap<i64, usize>,
    seen: &mut HashSet<i64>,
    depth: usize,
) -> HashSet<String> {
    if depth > 64 || !seen.insert(id) {
        return HashSet::new();
    }
    let mut set = direct.get(&id).cloned().unwrap_or_default();
    if let Some(kids) = children.get(&id) {
        for k in kids {
            set.extend(count_node(*k, children, direct, out, seen, depth + 1));
        }
    }
    out.insert(id, set.len());
    set
}

/// 一行里的反斜杠/制表/换行写成转义形式，直接拼进整份文本。
/// 不用连着四次 replace 的写法：65000 行就是 26 万次临时字符串分配，
/// 实测整份重排 70 毫秒，一大半花在这里。
fn esc_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            x => out.push(x),
        }
    }
}

fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(x) => {
                out.push('\\');
                out.push(x);
            }
            None => out.push('\\'),
        }
    }
    out
}

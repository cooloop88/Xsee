use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, unbounded};

use crate::job;
use crate::Thumb;

pub struct Done {
    pub idx: usize,
    pub ms: f64,
    pub from_preview: bool,
    /// 原图尺寸。(0,0) 表示没要（格子视图不显示尺寸，就不为此多开一次文件）。
    pub dims: (u32, u32),
    /// 位深；0 = 未知，界面显示扩展名。
    pub bpp: u32,
    /// 重建模式下：缓存里原来那张和原图对不上（错位）。正常出图恒为 false。
    pub mismatch: bool,
    /// 这一趟只要原图尺寸（详细资料档那一列），没解像素，`res` 是空的。
    pub dims_only: bool,
    pub res: Result<Thumb, String>,
}

struct Queue {
    pending: Mutex<VecDeque<usize>>,
    cv: Condvar,
    closed: AtomicBool,
}

/// 缩略图工作池。队列是可见优先的双端队列：界面滚动时把新看见的几张插到队头，
/// 已经在解的那张解完由界面丢掉——滚过就弃算。
pub struct Pool {
    q: Arc<Queue>,
    done: Receiver<Done>,
    workers: Vec<JoinHandle<()>>,
}

impl Pool {
    /// `fresh` = 重建档：每张都绕开缓存重解一遍，顺带回报缓存里那张对不对。
    /// `dims_only` = 详细资料档：只读文件头拿原图尺寸，一张缩略图也不解。
    pub fn start(
        paths: Vec<String>,
        bw: u32,
        bh: u32,
        workers: usize,
        want_dims: bool,
        fresh: bool,
        dims_only: bool,
    ) -> Pool {
        let q = Arc::new(Queue {
            pending: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
            closed: AtomicBool::new(false),
        });
        let (done_tx, done) = unbounded::<Done>();
        let paths = Arc::new(paths);
        let handles = (0..workers)
            .map(|_| {
                let (q, done_tx, paths) = (q.clone(), done_tx.clone(), paths.clone());
                std::thread::spawn(move || {
                    job::init_com();
                    loop {
                        let mut guard = q.pending.lock().unwrap();
                        while guard.is_empty() && !q.closed.load(Ordering::Acquire) {
                            guard = q.cv.wait(guard).unwrap();
                        }
                        let idx = match guard.pop_front() {
                            Some(i) => i,
                            None => break,
                        };
                        drop(guard);
                        let t = std::time::Instant::now();
                        let (made, mismatch) = if dims_only {
                            (Err(windows_core::Error::empty()), false)
                        } else if fresh {
                            match job::remake(&paths[idx], bw, bh) {
                                Ok((th, ok)) => (Ok(th), !ok),
                                Err(e) => (Err(e), false),
                            }
                        } else {
                            (job::make(&paths[idx], bw, bh), false)
                        };
                        let from_preview = made.as_ref().map(|x| x.from_preview).unwrap_or(false);
                        // 详情视图那一列要原图尺寸。开一次文件头不读像素，
                        // 但每张都多这一笔不值当，所以只在真的要显示尺寸时才要。
                        let meta = if want_dims {
                            job::open_meta(&paths[idx]).unwrap_or((0, 0, 0))
                        } else {
                            (0, 0, 0)
                        };
                        let _ = done_tx.send(Done {
                            idx,
                            ms: t.elapsed().as_secs_f64() * 1000.0,
                            from_preview,
                            dims: (meta.0, meta.1),
                            bpp: meta.2,
                            mismatch,
                            dims_only,
                            res: made.map_err(|e| e.to_string()),
                        });
                    }
                })
            })
            .collect();
        drop(done_tx);
        Pool { q, done, workers: handles }
    }

    pub fn submit(&self, idxs: &[usize]) {
        {
            let mut q = self.q.pending.lock().unwrap();
            q.extend(idxs.iter().copied());
        }
        self.q.cv.notify_all();
    }

    /// 测试用：还没被工作线程取走的任务数。
    pub fn depth(&self) -> usize {
        self.q.pending.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// 把 visible 提到队头，其余顺序不变。
    pub fn prioritize(&self, visible: &[usize]) {
        let mut q = self.q.pending.lock().unwrap();
        let front: HashSet<usize> = visible.iter().copied().collect();
        let want: Vec<usize> = q.iter().filter(|i| front.contains(i)).copied().collect();
        if want.is_empty() {
            return;
        }
        q.retain(|i| !front.contains(i));
        for i in want.into_iter().rev() {
            q.push_front(i);
        }
        drop(q);
        self.q.cv.notify_all();
    }

    pub fn pending(&self) -> usize {
        self.q.pending.lock().unwrap().len()
    }

    pub fn recv(&self, timeout: Duration) -> Option<Done> {
        self.done.recv_timeout(timeout).ok()
    }

    pub fn shutdown(&self) {
        self.q.closed.store(true, Ordering::Release);
        self.q.cv.notify_all();
    }

    pub fn join(self) {
        for h in self.workers {
            let _ = h.join();
        }
    }
}

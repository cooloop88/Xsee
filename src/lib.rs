pub mod appicon;
pub mod assoc;
pub mod cache;
pub mod clip;
pub mod catdb;
pub mod dec;
pub mod drag;
pub mod drop;
pub mod edit;
pub mod encode;
pub mod fmticon;
pub mod glyphs;
pub mod job;
pub mod menuicons;
pub mod pool;
pub mod preview;
pub mod psd;
pub mod refwin;
pub mod scan;
pub mod sim;
pub mod theme;
pub mod viewer;

pub struct Rgba {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

pub struct Thumb {
    pub img: Rgba,
    /// 内嵌预览直接来的，没解全图
    pub from_preview: bool,
}

/// 看图用：BGRA 行对齐位图 + 原图尺寸
pub struct View {
    pub w: u32,
    pub h: u32,
    pub nw: u32,
    pub nh: u32,
    pub px: Vec<u8>,
}

/// 进程峰值工作集（MB）。看图这条线只用这一个口径量内存。
pub fn peak_mb() -> f64 {
    use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let mut c = PROCESS_MEMORY_COUNTERS::default();
        c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let _ = K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb);
        c.PeakWorkingSetSize as f64 / 1048576.0
    }
}

use std::sync::OnceLock;

use crossbeam_channel::{Receiver, Sender, bounded};
use windows::core::{Error, HSTRING, Result};
use windows::Win32::Foundation::{E_FAIL, GENERIC_READ};
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};

use crate::Rgba;
use crate::Thumb;
use crate::View;
use crate::preview;

pub fn init_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

thread_local! {
    static FACTORY: IWICImagingFactory = unsafe {
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
            .expect("WIC imaging factory")
    };
}

pub fn make(path: &str, bw: u32, bh: u32) -> Result<Thumb> {
    // 命中缓存：只解一张几十 KB 的小 JPEG，原图连文件都不打开，只 stat 一次。
    // 缓存里的像素是摆正过的，所以这条线不再读方向。
    if let Some((img, has_fp)) = crate::cache::lookup_full(path, bw, bh) {
        // 这条记录里还没有指纹就顺手补一个：浏览到哪，相似库就补到哪。
        if !has_fp {
            crate::sim::note(path, bw, bh, &img);
        }
        return Ok(Thumb {
            img,
            from_preview: false,
        });
    }
    if crate::cache::failed(path) {
        return Err(Error::new(E_FAIL, "这张上次就解不开，跳过重试"));
    }
    let t = decode(path, bw, bh)?;
    stash(path, bw, bh, &t.img, false);
    Ok(t)
}

/// 缩略图和它的指纹一次入库：JPEG 只编一遍，索引只写一条记录。
/// 指纹吃的就是这串要落盘的字节，和查询时从盘上读回来解的是同一串，
/// 所以"入库和查询必须吃同一份像素"这条仍然成立，但省掉写完再读回来那一趟。
fn stash(path: &str, bw: u32, bh: u32, img: &Rgba, force: bool) {
    let Some(jpg) = crate::cache::encode(img) else {
        return;
    };
    let fp = crate::sim::fp_of_jpg(path, bw, bh, &jpg);
    if force {
        crate::cache::replace_jpg(path, bw, bh, img, jpg, fp);
    } else {
        crate::cache::store_jpg(path, bw, bh, img, jpg, fp);
    }
    if let Some(fp) = fp {
        crate::sim::adopt(path, bw, bh, Some(fp));
    }
}

/// 重建：绕开缓存把原图重解一遍，顺手和缓存里那张比一眼。
/// 返回 (新图, 缓存里原来那张对不对)。本来就没缓存也算"不对"——反正都要现做一张。
pub fn remake(path: &str, bw: u32, bh: u32) -> Result<(Thumb, bool)> {
    let old = crate::cache::peek(path, bw, bh);
    let t = decode(path, bw, bh)?;
    let ok = old.as_ref().is_some_and(|o| same_pic(o, &t.img));
    stash(path, bw, bh, &t.img, true);
    Ok((t, ok))
}

/// 缓存里那张和原图重解出来那张是不是同一个东西。缩略图错位就是"格子里是 A、点开是 B"，
/// 靠元数据看不出来，只能把两张摆在一起比。JPEG 有损，同图重编也就差几个色阶；
/// 换了一张图差得远。取几百个点求平均色差，超过 12 判成对不上。
fn same_pic(a: &Rgba, b: &Rgba) -> bool {
    if a.w != b.w || a.h != b.h {
        return false;
    }
    let n = (a.w * a.h) as usize;
    if n == 0 {
        return true;
    }
    let step = (n / 400).max(1);
    let (mut sum, mut cnt) = (0u64, 0u64);
    let mut i = 0usize;
    while i < n {
        let o = i * 4;
        if o + 3 >= a.px.len() || o + 3 >= b.px.len() {
            break;
        }
        for c in 0..3 {
            sum += (a.px[o + c] as i64 - b.px[o + c] as i64).unsigned_abs();
        }
        cnt += 3;
        i += step;
    }
    cnt > 0 && sum / cnt <= 12
}

/// 老实出一次缩略图，不查缓存也不写缓存。
fn decode(path: &str, bw: u32, bh: u32) -> Result<Thumb> {
    // 换成 image 的 inflate 试过：只解压快 1.4~2.4 倍，但整张位图要落到我们自己的堆
    // （44MP 峰值 350MB）且 16 位上慢 7 倍，端到端持平，所以仍旧只走 WIC。
    // 档内预览坏掉不能把整张图判死：内嵌段解不动就回落到老实开原图。
    // （xmppmax_v1_10_1_5_g.jpg 正是这种：内嵌那段报 0x88982F61，原图 WIC 全解 243ms 好好的。）
    // 一次开档拿回来的头，档内预览和方向两件事都从它读；整幅解码那条要读全文件，
    // 只能再开一次，但不再为"问个方向"单独开第三趟。
    // 只有本来就要看档内预览的格式才读这一截：PNG、TIFF、BMP 这些没预览也没方向，
    // 给它们读 256KB 头纯属白花一次盘。
    let head = if preview::has_preview(path) || preview::is_raw(path) {
        preview::read_head(path, preview::HEAD).unwrap_or_default()
    } else {
        Vec::new()
    };
    let made = from_embedded(path, bw, bh, &head)
        .map(Ok)
        .unwrap_or_else(|| {
            // PSD 没有 WIC 解码器，档内预览又不可用时只剩自己解合成图这一条。
            if let Some(t) = from_psd(path, bw, bh) {
                return t;
            }
            // 没有可用的档内预览才要开原图，这一步才是内存大户。
            let _p = if big_file(path) { big_permit() } else { None };
            make_wic(path, bw, bh)
        });
    let mut t = match made {
        Ok(t) => t,
        Err(e) => {
            // 解不开的是格式不是运气：记一笔，下次滚回这一屏别再重演一遍失败。
            crate::cache::note_fail(path);
            return Err(e);
        }
    };
    crate::dec::orient(
        &mut t.img.px,
        &mut t.img.w,
        &mut t.img.h,
        preview::orient_head(&head),
    );
    Ok(t)
}

/// 6 MB 以上的档全尺寸位图就上百 MB，按这个线卡并发。
/// 大小先认扫描线从目录枚举带回来的那份，问不到才去 stat。
fn big_file(path: &str) -> bool {
    crate::cache::file_stamp(path).is_some_and(|(len, _)| len > 6_000_000)
}

struct Permit(Sender<()>);

impl Drop for Permit {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

/// 两槽令牌通道：同时最多两张大图在解，第四第五个 worker 在这儿停着等，
/// 不占 CPU。宁可慢一点，也不要一屏把 800MB 顶上去。
fn big_permit() -> Option<Permit> {
    static SLOT: OnceLock<(Sender<()>, Receiver<()>)> = OnceLock::new();
    let (tx, rx) = SLOT.get_or_init(|| {
        let (tx, rx) = bounded::<()>(2);
        let _ = tx.send(());
        let _ = tx.send(());
        (tx, rx)
    });
    rx.recv().ok().map(|_| Permit(tx.clone()))
}

/// 整份读进内存的上限。超过的（全尺寸 TIF 那种几百 MB 起的）还是让 WIC 直读，
/// 宁可慢也不把整档塞进内存。
const MEM_DECODE_MAX: u64 = 160 * 1024 * 1024;

fn read_all(path: &str) -> Option<Vec<u8>> {
    // 大小先认扫描线带回来的那份，问不到才去 stat。
    let len = crate::cache::file_stamp(path)
        .map(|(l, _)| l)
        .or_else(|| std::fs::metadata(path).ok().map(|m| m.len()))?;
    if len > MEM_DECODE_MAX {
        return None;
    }
    let mut f = std::fs::File::open(path).ok()?;
    let mut v = Vec::with_capacity(len as usize);
    std::io::Read::read_to_end(&mut f, &mut v).ok()?;
    Some(v)
}

/// 只走 WIC 的那条路，给对比测量当基线用。
/// WIC 从文件名解码是按 4 KB 小缓冲往盘上读的：112 MB 的 BMP 在机械盘上要 4.7 秒，
/// 换成顺序 read 只要 0.45 秒，而解码本身只要 0.05 秒。所以先整份读进内存再交给
/// WIC，内存流解不开（个别第三方解码器不认）就退回文件名直读。
pub fn make_wic(path: &str, bw: u32, bh: u32) -> Result<Thumb> {
    let factory = FACTORY.with(|f| f.clone());
    if let Some(bytes) = read_all(path) {
        if let Ok(t) = make_wic_mem(&factory, &bytes, bw, bh) {
            return Ok(t);
        }
    }
    unsafe {
        let decoder = factory.CreateDecoderFromFilename(
            &HSTRING::from(path),
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )?;
        let frame = decoder.GetFrame(0)?;
        crate::dec::source_to_box(&factory, &frame, bw, bh).map(|img| Thumb {
            img,
            from_preview: false,
        })
    }
}

fn make_wic_mem(
    factory: &IWICImagingFactory,
    bytes: &[u8],
    bw: u32,
    bh: u32,
) -> Result<Thumb> {
    unsafe {
        let stream = factory.CreateStream()?;
        stream.InitializeFromMemory(bytes)?;
        let decoder = factory.CreateDecoderFromStream(
            &stream,
            std::ptr::null(),
            WICDecodeMetadataCacheOnDemand,
        )?;
        let frame = decoder.GetFrame(0)?;
        crate::dec::source_to_box(factory, &frame, bw, bh).map(|img| Thumb {
            img,
            from_preview: false,
        })
    }
}

/// 档内预览开出来的一帧，连同它的数据来源。WIC 的内存流只借用不复制，
/// 所以这块字节（和读它时占的内存坑位）必须跟帧一起活到像素拷完。
struct Embedded {
    factory: IWICImagingFactory,
    frame: IWICBitmapFrameDecode,
    _bytes: Vec<u8>,
    _permit: Option<Permit>,
}

/// 档内自带预览开成一个 WIC 帧。格式不对、没有预览、或预览最大边还短于
/// `min_short` 返回 None。门槛由各调用方自己定：缩略图要求够大，
/// 看图第一帧只求别是 32px 图标，糊一点也比空白强。
/// `head` 是调用方一次开档拿回来的头，预览段就从它里找。
fn embedded_frame(path: &str, min_short: u32, head: &[u8]) -> Option<Embedded> {
    let raw = preview::is_raw(path);
    if !(raw || preview::has_preview(path)) {
        return None;
    }
    // RAW 没有预览段索引，只能整档扫字节，几十 MB 起跳，和大图解码共用两槽令牌。
    let permit = if raw && big_file(path) { big_permit() } else { None };
    let bytes = if raw {
        preview::raw_preview(path)
    } else {
        preview::embedded_bytes_at(path, head)
    };
    let bytes = bytes?;
    let factory = FACTORY.with(|f| f.clone());
    unsafe {
        let stream = factory.CreateStream().ok()?;
        stream.InitializeFromMemory(&bytes).ok()?;
        let decoder = factory
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)
            .ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let (mut pw, mut ph) = (0u32, 0u32);
        frame.GetSize(&mut pw, &mut ph).ok()?;
        if (pw.max(ph) as usize) < min_short as usize {
            return None;
        }
        Some(Embedded {
            factory,
            frame,
            _bytes: bytes,
            _permit: permit,
        })
    }
}

/// 看图第一帧：只吃档内预览，零全尺寸解码。JPEG 巨型档这条是 1ms 出图。
pub fn embedded_view(path: &str, bw: u32, bh: u32) -> Option<View> {
    let head = preview::read_head(path, preview::HEAD)?;
    let e = embedded_frame(path, 64, &head)?;
    let (mut nw, mut nh) = (0u32, 0u32);
    unsafe { e.frame.GetSize(&mut nw, &mut nh).ok()? };
    let (mut w, mut h, mut px) = crate::dec::source_to_box_bgra(&e.factory, &e.frame, bw, bh).ok()?;
    let o = preview::orient_head(&head);
    crate::dec::orient(&mut px, &mut w, &mut h, o);
    let (nw, nh) = if o >= 5 { (nh, nw) } else { (nw, nh) };
    Some(View { w, h, nw, nh, px })
}

/// 有档内预览就直接用，省掉整张解码。预览撑不满目标框、或预览段解不出像素
/// （建得出帧、CopyPixels 失败的坏预览）都返回 None，让调用方回落老实解码。
/// PSD 单独放宽：Max 写进档里的预览长边一律不超过 160，而合成图整幅几百 MB，
/// 为两百上下的格位去解它要几百毫秒、还要占住"大文件只许两路"的名额；
/// 实测同一张 PSD 两种来路的缩略图拿去搜同场景的导出图最多差 16 位（认亲线 40），
/// 所以小中格位直接用预览。但放大过 1.5 倍就不干了——418×288 那种大格位
/// 铺一张 160 的小图是糊，不是降取样档次，那种仍旧整幅解。
fn from_embedded(path: &str, bw: u32, bh: u32, head: &[u8]) -> Option<Thumb> {
    let psd = preview::is_psd(path);
    let min_short = if psd { 0 } else { bw.min(bh) };
    let e = embedded_frame(path, min_short, head)?;
    let img = crate::dec::source_to_box(&e.factory, &e.frame, bw, bh).ok()?;
    if psd && (bw.max(bh) as f64) > 1.5 * (img.w.max(img.h) as f64) {
        return None;
    }
    Some(Thumb {
        img,
        from_preview: true,
    })
}

/// 解 PSD 合成图并缩到目标框，返回 (原图宽, 原图高, 出图宽, 出图高, BGRA)。
/// 非 PSD、或合成图不在支持表里（灰度/索引/16 位以上/超大）返回 None。
/// 那份几十上百 MB 的合成图缓冲在这个函数里就随 `bgra` 一起释放，往外只交缩好的，
/// 所以令牌要占到这里为止——不然八个 worker 一人一份合成图，一屏就是七八百 MB。
fn psd_scaled(path: &str, bw: u32, bh: u32) -> Option<(u32, u32, u32, u32, Vec<u8>)> {
    if !preview::is_psd(path) {
        return None;
    }
    let _p = big_permit()?;
    let (sw, sh, bgra) = crate::psd::decode(path)?;
    let (bw, bh) = if bw == 0 || bh == 0 { (sw, sh) } else { (bw, bh) };
    let factory = FACTORY.with(|f| f.clone());
    unsafe {
        let bmp = factory
            .CreateBitmapFromMemory(sw, sh, &GUID_WICPixelFormat32bppBGRA, sw * 4, &bgra)
            .ok()?;
        // WIC 把像素抄进自己的位图了，这份先撒手，别让两份 111 MB 同时活着。
        drop(bgra);
        let (w, h, px) = crate::dec::source_to_box_bgra(&factory, &bmp, bw, bh).ok()?;
        Some((sw, sh, w, h, px))
    }
}

fn from_psd(path: &str, bw: u32, bh: u32) -> Option<Result<Thumb>> {
    let (_, _, w, h, bgra) = psd_scaled(path, bw, bh)?;
    Some(Ok(Thumb {
        img: Rgba {
            w,
            h,
            px: crate::dec::bgra_to_rgba(&bgra, w * 4, w, h),
        },
        from_preview: false,
    }))
}

/// 看图这条线的 PSD 档。合成图没有方向标记（`orientation` 对它恒为 1），
/// 所以这条不再走摆正，省一次开文件。
fn psd_view(path: &str, bw: u32, bh: u32) -> Option<View> {
    let (nw, nh, w, h, px) = psd_scaled(path, bw, bh)?;
    Some(View { w, h, nw, nh, px })
}

/// 打开文件头读尺寸和位深，不解像素。位深返回 0 表示未知，界面改显示格式名。
/// 尺寸是摆正之后的口径，和出图一致。
pub fn open_meta(path: &str) -> Option<(u32, u32, u32)> {
    let (w, h) = open_size(path).ok()?;
    let lower = path.to_ascii_lowercase();
    let bpp = if lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".jpe") {
        24
    } else {
        0
    };
    Some((w, h, bpp))
}

/// 原图宽高，摆正之后的口径：查看器拿它算盒子，所以必须是屏幕上那个方向。
/// 先读文件头嗅，认不出再开解码器。
pub fn open_size(path: &str) -> Result<(u32, u32)> {
    let (sniffed, o) = preview::sniff_orient(path);
    let (w, h) = match sniffed {
        Some(v) => v,
        None => wic_size(path)?,
    };
    Ok(if o >= 5 { (h, w) } else { (w, h) })
}

/// 只开文件读存储尺寸，不解像素，也不管方向。用来量"打开"这一步的开销。
pub fn wic_size(path: &str) -> Result<(u32, u32)> {
    unsafe {
        let factory = FACTORY.with(|f| f.clone());
        let decoder = factory.CreateDecoderFromFilename(
            &HSTRING::from(path),
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )?;
        let frame = decoder.GetFrame(0)?;
        let (mut nw, mut nh) = (0u32, 0u32);
        frame.GetSize(&mut nw, &mut nh)?;
        Ok((nw, nh))
    }
}

/// 看图用：一律真解码，绝不走内嵌预览。bw/bh 传 0 表示按原图尺寸出图。
pub fn decode_view(path: &str, bw: u32, bh: u32) -> Result<View> {
    decode_view_mode(path, bw, bh, None)
}

/// 同上，但可以指定缩放插值模式。
pub fn decode_view_mode(
    path: &str,
    bw: u32,
    bh: u32,
    mode: Option<WICBitmapInterpolationMode>,
) -> Result<View> {
    if let Some(v) = psd_view(path, bw, bh) {
        return Ok(v);
    }
    unsafe {
        let factory = FACTORY.with(|f| f.clone());
        let decoder = factory.CreateDecoderFromFilename(
            &HSTRING::from(path),
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )?;
        let frame = decoder.GetFrame(0)?;
        let (mut nw, mut nh) = (0u32, 0u32);
        frame.GetSize(&mut nw, &mut nh)?;
        let (bw, bh) = if bw == 0 || bh == 0 { (nw, nh) } else { (bw, bh) };
        let (mut w, mut h, mut px) =
            crate::dec::source_to_box_bgra_mode(&factory, &frame, bw, bh, mode)?;
        let o = preview::orientation(path);
        crate::dec::orient(&mut px, &mut w, &mut h, o);
        let (nw, nh) = if o >= 5 { (nh, nw) } else { (nw, nh) };
        Ok(View {
            w,
            h,
            nw,
            nh,
            px,
        })
    }
}

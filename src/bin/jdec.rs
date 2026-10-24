/// 容错 JPEG 兜底选型：同一张 WIC 拒解的文件，分别喂给两个纯 Rust 解码器，
/// 看谁能解开、各花多久。WIC 那条也跑一遍当基线。
use std::fs::File;
use std::time::Instant;

use xsee_host::job;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    job::init_com();
    for path in &args {
        println!("── {path}");

        let t = Instant::now();
        match job::decode_view(path, 0, 0) {
            Ok(v) => println!(
                "   WIC  全解  ok {}x{}  {:.0} ms",
                v.nw, v.nh,
                t.elapsed().as_secs_f64() * 1000.0
            ),
            Err(e) => println!("   WIC  全解  失败 {:.0} ms：{}", t.elapsed().as_secs_f64() * 1000.0, e),
        }

        let t = Instant::now();
        let mut jd = jpeg_decoder::Decoder::new(File::open(path).unwrap());
        match jd.read_info().and_then(|_| jd.decode()) {
            Ok(buf) => println!(
                "   jpeg-decoder 全解 ok {} 字节  {:.0} ms",
                buf.len(),
                t.elapsed().as_secs_f64() * 1000.0
            ),
            Err(e) => println!(
                "   jpeg-decoder 全解 失败 {:.0} ms：{e}",
                t.elapsed().as_secs_f64() * 1000.0
            ),
        }

        let t = Instant::now();
        match std::fs::read(path) {
            Ok(bytes) => {
                let mut cur = std::io::Cursor::new(bytes);
                let r = image::codecs::jpeg::JpegDecoder::new(&mut cur).and_then(|mut d| {
                    use image::ImageDecoder;
                    let (w, h) = d.dimensions();
                    let mut buf = vec![0u8; (w as usize * h as usize * 3).max(1)];
                    d.read_image(&mut buf)?;
                    Ok((w, h, buf.len()))
                });
                match r {
                    Ok((w, h, n)) => println!(
                        "   image/zune 全解 ok {w}x{h} {n} 字节  {:.0} ms",
                        t.elapsed().as_secs_f64() * 1000.0
                    ),
                    Err(e) => println!(
                        "   image/zune 全解 失败 {:.0} ms：{e}",
                        t.elapsed().as_secs_f64() * 1000.0
                    ),
                }
            }
            Err(e) => println!("   读文件失败 {e}"),
        }
    }
}

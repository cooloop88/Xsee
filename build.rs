//! 把 resources/xsee.ico 编进 exe 的资源段，任务栏和桌面才有图标。
//!
//! 只调 Windows SDK 自带的 rc.exe，不引入任何 crate 依赖；找不到 rc.exe 或编译失败
//! 时只报一条 warning 就跳过，不会把构建搞挂。
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 在 SDK 的 bin 目录里挑版本号最高的那份 rc.exe。
fn sdk_rc() -> Option<PathBuf> {
    let mut best: Option<(u32, PathBuf)> = None;
    for ver in std::fs::read_dir(Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin")).ok()? {
        let dir = ver.ok()?.path();
        let exe = dir.join("x64").join("rc.exe");
        if !exe.is_file() {
            continue;
        }
        let major = dir
            .file_name()?
            .to_string_lossy()
            .split('.')
            .next()?
            .parse()
            .unwrap_or(0);
        if best.as_ref().is_none_or(|(m, _)| major >= *m) {
            best = Some((major, exe));
        }
    }
    best.map(|(_, p)| p)
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=resources/xsee.rc");
    println!("cargo:rerun-if-changed=resources/xsee.ico");
    println!("cargo:rerun-if-changed=resources/fmt");

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let resources = manifest.join("resources");
    if !resources.join("xsee.rc").is_file() || !resources.join("xsee.ico").is_file() {
        return;
    }
    let Some(rc) = sdk_rc() else {
        println!("cargo:warning=没找到 Windows SDK 的 rc.exe，图标未嵌入，程序功能不受影响");
        return;
    };
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("xsee.res");
    // rc.exe 按当前目录解析 .rc 里的相对路径，所以在这里 cwd 到 resources。
    let built = Command::new(&rc)
        .current_dir(&resources)
        .arg("/nologo")
        .arg("/fo")
        .arg(&out)
        .arg("xsee.rc")
        .status()
        .is_ok_and(|s| s.success());
    if built {
        println!("cargo:rustc-link-arg-bins={}", out.display());
    } else {
        println!("cargo:warning=rc.exe 编译 xsee.rc 失败，图标未嵌入");
    }
}

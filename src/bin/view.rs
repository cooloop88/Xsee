#![windows_subsystem = "windows"]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let some = args.first().cloned().unwrap_or_default();
    if some.is_empty() || some.starts_with('-') {
        eprintln!("用法: view <文件夹或某张图>");
        std::process::exit(2);
    }
    xsee_host::viewer::run_cli(&some);
}

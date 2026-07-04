use std::process::Command;

/// 运行命令并获取 stdout（修剪空白）
fn run(args: &[&str]) -> Option<String> {
    let program = args.first()?;
    let cmd_args: Vec<&str> = args[1..].to_vec();
    Command::new(program)
        .args(&cmd_args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

fn main() {
    // unrar_sys 编译 unrar C++ 源码时遗漏了部分 Windows 系统库的链接
    println!("cargo:rustc-link-lib=advapi32");

    // ── 目标三元组：x86_64-pc-windows-msvc / x86_64-unknown-linux-musl ──
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=BUILD_TARGET={}", target);

    // ── Git 短提交哈希 ──
    let commit = run(&["git", "rev-parse", "--short", "HEAD"])
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=BUILD_COMMIT={}", commit);

    // ── Git 提交日期（短格式 YYYY-MM-DD） ──
    let date = run(&["git", "log", "-1", "--format=%cs"])
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=BUILD_DATE={}", date);
}
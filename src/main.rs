mod cli;
mod cracker;
mod dictionary;
mod password;
mod style;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use walkdir::WalkDir;

use crate::cli::Cli;
use crate::cracker::{dictionary_attack, numeric_bruteforce};
use crate::dictionary::{load_embedded_passwords, load_passwords_from_file};

fn main() {
    // ---- 解析 CLI 参数（优先执行，--help/--version 时直接退出） ----
    let args = Cli::parse();

    // ── 启动横幅 ──
    println!("{}", style::banner("╔══════════════════════════════════════╗"));
    println!("{}", style::banner(&format!(
        "║        RAR 密码破解工具 v{}        ║",
        env!("CARGO_PKG_VERSION")
    )));
    println!("{}", style::banner("╚══════════════════════════════════════╝"));
    println!();

    // 检查RAR文件是否存在
    if !args.file.exists() {
        eprintln!(
            "{} {}",
            style::error("✖ 错误:"),
            style::highlight(&format!("RAR文件 '{}' 不存在", args.file.display()))
        );
        std::process::exit(1);
    }

    // 确定线程数
    let num_threads = if args.threads == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        args.threads
    };

    println!("📂 {}  {}", style::value("目标文件:"), args.file.display());
    println!("⚙  {}  {}", style::value("线程数:"), num_threads);
    println!();

    // 共享状态
    let found = Arc::new(AtomicBool::new(false));
    let counter = Arc::new(AtomicUsize::new(0));
    let start_time = Instant::now();

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  阶段1: 数字暴力破解（始终执行）
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let found_password = numeric_bruteforce(&args.file, &found, &counter, start_time, num_threads);

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  阶段2: 内嵌字典 password_list.txt（始终执行）
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let found_password = found_password.or_else(|| {
        println!();
        println!("{}", style::stage("━━━ 📖 阶段2: 内嵌字典破解 ━━━"));
        println!("  {} 使用内嵌密码列表 (共 {} 条)",
            style::value("→"),
            style::progress_num(&{
                // 先读取一次以获取数量
                // 实际破解时会重新读取传入 dictionary_attack
                let p = load_embedded_passwords();
                p.len().to_string()
            })
        );

        let passwords = load_embedded_passwords();
        dictionary_attack(&args.file, &passwords, &found, &counter, start_time, num_threads)
    });

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  阶段3: 用户指定字典文件（--dictionary）
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let found_password = found_password.or_else(|| {
        let dict_path = args.dictionary.as_ref()?;
        if dict_path.exists() {
            println!();
            println!("{}", style::stage("━━━ 📂 阶段3: 字典文件破解 ━━━"));
            let passwords = load_passwords_from_file(dict_path);
            dictionary_attack(&args.file, &passwords, &found, &counter, start_time, num_threads)
        } else {
            eprintln!(
                "{} {}",
                style::warning("⚠ 警告:"),
                format!("字典文件 '{}' 不存在", dict_path.display())
            );
            None
        }
    });

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    //  阶段4: 用户指定字典目录（--dictionary-dir）
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let found_password = found_password.or_else(|| {
        let dir_path = args.dictionary_dir.as_ref()?;
        if dir_path.is_dir() {
            println!();
            println!("{}", style::stage("━━━ 📁 阶段4: 字典目录破解 ━━━"));
            println!("  {}", style::value(&format!("扫描目录: {}", dir_path.display())));

            // 收集目录中的所有文件
            let dict_files: Vec<_> = WalkDir::new(dir_path)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .collect();

            if dict_files.is_empty() {
                println!("  {} 目录中没有文件", style::warning("⚠"));
                return None;
            }

            println!(
                "  {} 发现 {} 个字典文件",
                style::value("🔍"),
                style::progress_num(&dict_files.len().to_string())
            );

            for dict_file in &dict_files {
                if found.load(Ordering::Relaxed) {
                    break;
                }

                println!();
                println!("  📄 处理字典: {}", style::highlight(&dict_file.display().to_string()));
                let passwords = load_passwords_from_file(dict_file);
                let pwd = dictionary_attack(
                    &args.file,
                    &passwords,
                    &found,
                    &counter,
                    start_time,
                    num_threads,
                );

                if pwd.is_some() {
                    return pwd;
                }
            }

            None
        } else {
            eprintln!(
                "{} {}",
                style::warning("⚠ 警告:"),
                format!("字典目录 '{}' 不存在或不是一个目录", dir_path.display())
            );
            None
        }
    });

    // ---- 输出最终结果 ----
    println!();
    println!("{}", style::banner("════════════════════════════════════════"));
    let elapsed = start_time.elapsed();
    let total_attempts = counter.load(Ordering::Relaxed);

    match found_password {
        Some(pwd) => {
            println!("{}  {}", style::success("✅  破解成功!"), style::found_password(&pwd));
            println!("  📂 {}  {}", style::value("文件:"), args.file.display());
            println!("  🔑 {}  {}", style::value("密码:"), style::found_password(&pwd));
            println!("  ⏱ {}  {:.2} 秒", style::value("用时:"), elapsed.as_secs_f64());
            println!("  🔢 {}  {}", style::value("尝试次数:"), total_attempts);
            println!(
                "  🚀 {}  {:.0} 次/秒",
                style::value("速度:"),
                total_attempts as f64 / elapsed.as_secs_f64()
            );
        }
        None => {
            println!("{}", style::error("❌  破解失败"));
            println!("  📂 {}  {}", style::value("文件:"), args.file.display());
            println!("  ⏱ {}  {:.2} 秒", style::value("用时:"), elapsed.as_secs_f64());
            println!("  🔢 {}  {}", style::value("尝试次数:"), total_attempts);
            println!();

            // 如果用户没有指定字典参数，提示使用 --dictionary
            if args.dictionary.is_none() && args.dictionary_dir.is_none() {
                println!("  💡 {}", style::warning("内嵌字典与数字穷举均未破解成功"));
                println!("     {}", style::value("请使用 --dictionary 参数指定一个更大的字典文件:"));
                println!("     {}", style::highlight(&format!(
                    "     {} --dictionary <FILE>",
                    std::env::args().next().unwrap_or_else(|| "rar-cracker".into())
                )));
            } else {
                println!("  💡 {}", style::warning("建议: 尝试以下方法"));
                println!("     {}", style::value("1. 使用更大的字典文件"));
                println!("     {}", style::value("2. 增加密码长度范围"));
                println!("     {}", style::value("3. 使用混合字符集字典"));
            }
        }
    }
}
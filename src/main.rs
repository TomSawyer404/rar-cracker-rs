mod cli;
mod cracker;
mod dictionary;
mod password;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use walkdir::WalkDir;

use crate::cli::Cli;
use crate::cracker::{dictionary_attack, numeric_bruteforce};
use crate::dictionary::load_passwords_from_file;

fn main() {
    println!("╔══════════════════════════════════════╗");
    println!("║        RAR 密码破解工具 v1.0.0        ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // ---- 解析 CLI 参数 ----
    let args = Cli::parse();

    // 检查RAR文件是否存在
    if !args.file.exists() {
        eprintln!("错误: RAR文件 '{}' 不存在", args.file.display());
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

    println!("📂 目标文件: {}", args.file.display());
    println!("⚙  线程数:   {}", num_threads);
    println!();

    // 共享状态
    let found = Arc::new(AtomicBool::new(false));
    let counter = Arc::new(AtomicUsize::new(0));
    let start_time = Instant::now();

    // ---- 阶段1: 数字暴力破解 ----
    let found_password = numeric_bruteforce(
        &args.file,
        &found,
        &counter,
        start_time,
        num_threads,
    );

    // ---- 阶段2: 字典文件破解 ----
    let found_password = found_password.or_else(|| {
        let dict_path = args.dictionary.as_ref()?;
        if dict_path.exists() {
            println!("\n━━━ 阶段2: 字典文件破解 ━━━");
            let passwords = load_passwords_from_file(dict_path);
            dictionary_attack(
                &args.file,
                &passwords,
                &found,
                &counter,
                start_time,
                num_threads,
            )
        } else {
            eprintln!("警告: 字典文件 '{}' 不存在", dict_path.display());
            None
        }
    });

    // ---- 阶段3: 字典目录破解 ----
    let found_password = found_password.or_else(|| {
        let dir_path = args.dictionary_dir.as_ref()?;
        if dir_path.is_dir() {
            println!("\n━━━ 阶段3: 字典目录破解 ━━━");
            println!("  扫描目录: {}", dir_path.display());

            // 收集目录中的所有文件
            let dict_files: Vec<_> = WalkDir::new(dir_path)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .collect();

            if dict_files.is_empty() {
                println!("  ℹ  目录中没有文件");
                return None;
            }

            println!("  发现 {} 个字典文件", dict_files.len());

            for dict_file in &dict_files {
                if found.load(Ordering::Relaxed) {
                    break;
                }

                println!("\n  📄 处理字典: {}", dict_file.display());
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
            eprintln!("警告: 字典目录 '{}' 不存在或不是一个目录", dir_path.display());
            None
        }
    });

    // ---- 输出最终结果 ----
    println!();
    println!("════════════════════════════════════════");
    let elapsed = start_time.elapsed();
    let total_attempts = counter.load(Ordering::Relaxed);

    match found_password {
        Some(pwd) => {
            println!("✅  破解成功!");
            println!("  文件:     {}", args.file.display());
            println!("  密码:     [{}]", pwd);
            println!("  用时:     {:.2} 秒", elapsed.as_secs_f64());
            println!("  尝试次数: {}", total_attempts);
            println!("  速度:     {:.0} 次/秒",
                total_attempts as f64 / elapsed.as_secs_f64()
            );
        }
        None => {
            println!("❌  破解失败");
            println!("  文件:     {}", args.file.display());
            println!("  用时:     {:.2} 秒", elapsed.as_secs_f64());
            println!("  尝试次数: {}", total_attempts);
            println!("  建议: 尝试以下方法");
            println!("        1. 使用更大的字典文件");
            println!("        2. 增加密码长度范围");
            println!("        3. 使用混合字符集字典");
        }
    }
}
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rayon::prelude::*;

use crate::password::check_password;
use crate::style;

/// 打印视觉进度条（使用 \r 覆盖同一行）
fn print_progress_bar(count: usize, total: u64, start_time: Instant, current: &str) {
    let elapsed = start_time.elapsed().as_secs_f64();
    let rate = if elapsed > 0.0 {
        count as f64 / elapsed
    } else {
        0.0
    };
    let pct = if total > 0 {
        count as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    // 视觉进度条: [████████░░░░░░░░░░░░]
    const BAR_WIDTH: usize = 22;
    let filled = ((pct / 100.0) * BAR_WIDTH as f64).round() as usize;
    let empty = BAR_WIDTH.saturating_sub(filled);
    let bar: String = "█".repeat(filled) + &"░".repeat(empty);

    // 当前密码截断显示
    let display = if current.len() > 18 {
        format!("{}...", &current[..18])
    } else {
        format!("{:22}", current)
    };

    print!(
        "\r    [{}] {:>6.2}% | {:>8}/{} | {:>8.0}/秒 | {}",
        bar,
        pct,
        style::progress_num(&count.to_string()),
        total,
        rate,
        style::value(&display)
    );
    io::stdout().flush().unwrap();
}

/// 尝试所有4位数字组合（0000-9999）
///
/// 总计: 10,000 种组合
pub fn numeric_bruteforce(
    file_path: &Path,
    found: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
    start_time: Instant,
    num_threads: usize,
) -> Option<String> {
    let file_buf = file_path.to_path_buf();
    let total_combinations: u64 = 10_000;
    let progress_lock = Arc::new(Mutex::new(()));
    let local_counter = Arc::new(AtomicUsize::new(0));

    println!("{}", style::stage("━━━ 🔢 阶段1: 数字暴力破解 (4位) ━━━"));
    println!("  {} {} 种组合 (0000-9999)",
        style::value("共计"),
        style::progress_num(&total_combinations.to_string())
    );

    // 创建独立的线程池用于此阶段
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .expect("无法创建线程池");

    pool.install(|| {
        let start = 0u64;
        let end = 10_000u64;

        let found_in_len = (start..end).into_par_iter().find_any(|&i| {
            if found.load(Ordering::Relaxed) {
                return true;
            }

            let pwd = format!("{:04}", i);
            let correct = check_password(&file_buf, &pwd);

            counter.fetch_add(1, Ordering::Relaxed);
            let local_count = local_counter.fetch_add(1, Ordering::Relaxed) + 1;

            if local_count % 100 == 0 || local_count == 1 {
                let _guard = progress_lock.lock().unwrap();
                print_progress_bar(local_count, total_combinations, start_time, &pwd);
            }

            if correct {
                let _guard = progress_lock.lock().unwrap();
                println!();
                println!("  {} 找到密码: [{}]",
                    style::success("✔"),
                    style::found_password(&pwd)
                );
                found.store(true, Ordering::Relaxed);
            }

            correct
        });

        // 结束进度条（换行）
        let _guard = progress_lock.lock().unwrap();
        println!();

        if let Some(i) = found_in_len {
            let pwd = format!("{:04}", i);
            Some(pwd)
        } else {
            None
        }
    })
}

/// 使用给定的密码列表进行字典攻击（带视觉进度条）
pub fn dictionary_attack(
    file_path: &Path,
    passwords: &[String],
    found: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
    start_time: Instant,
    num_threads: usize,
) -> Option<String> {
    if passwords.is_empty() {
        println!("  {} 字典为空，跳过", style::warning("⚠"));
        return None;
    }

    println!("  {} 字典大小: {} 个唯一密码",
        style::value("→"),
        style::progress_num(&passwords.len().to_string())
    );

    let file_buf = file_path.to_path_buf();
    let total = passwords.len() as u64;
    let progress_lock = Arc::new(Mutex::new(()));
    // 本地计数器：仅用于本阶段进度显示，不受全局 counter 影响
    let local_counter = Arc::new(AtomicUsize::new(0));

    // 创建独立的线程池
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .expect("无法创建线程池");

    pool.install(|| {
        let result = passwords.par_iter().find_any(|pwd| {
            if found.load(Ordering::Relaxed) {
                return true;
            }

            let correct = check_password(&file_buf, pwd);

            // 全局计数（最终统计用）
            counter.fetch_add(1, Ordering::Relaxed);
            // 本地计数（本阶段进度条用）
            let local_count = local_counter.fetch_add(1, Ordering::Relaxed) + 1;

            if local_count % 1000 == 0 || local_count == 1 {
                let display = if pwd.len() > 18 {
                    format!("{}...", &pwd[..18])
                } else {
                    pwd.to_string()
                };
                let _guard = progress_lock.lock().unwrap();
                print_progress_bar(local_count, total, start_time, &display);
            }

            if correct {
                let _guard = progress_lock.lock().unwrap();
                println!();
                println!("  {} 找到密码: [{}]",
                    style::success("✔"),
                    style::found_password(pwd)
                );
                found.store(true, Ordering::Relaxed);
            }

            correct
        });

        // 结束进度条（换行）
        let _guard = progress_lock.lock().unwrap();
        println!();

        result.cloned()
    })
}
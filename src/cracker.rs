use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use rayon::prelude::*;

use crate::password::check_password;
use crate::style;

/// 打印彩色进度信息
fn print_progress(count: usize, total: u64, start_time: Instant, current: &str) {
    let elapsed = start_time.elapsed().as_secs_f64();
    let rate = if elapsed > 0.0 {
        count as f64 / elapsed
    } else {
        0.0
    };
    let progress = (count as f64 / total as f64) * 100.0;
    println!(
        "    {} {:>6.2}% | {} {:>8} | {} {:>8.0}/秒 | {} {:>6.1}秒 | {} {}",
        style::progress_num("进度:"),
        progress,
        style::progress_num("已尝试:"),
        count,
        style::progress_num("速度:"),
        rate,
        style::progress_num("用时:"),
        elapsed,
        style::value("当前:"),
        current
    );
}

/// 尝试所有1-6位数字组合
///
/// 总计: 10 + 100 + 1,000 + 10,000 + 100,000 + 1,000,000 = 1,111,110 种组合
pub fn numeric_bruteforce(
    file_path: &Path,
    found: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
    start_time: Instant,
    num_threads: usize,
) -> Option<String> {
    let file_buf = file_path.to_path_buf();
    let total_combinations: u64 = 1_111_110;

    println!("{}", style::stage("━━━ 🔢 阶段1: 数字暴力破解 (1-6位) ━━━"));
    println!("  {} {} 种组合",
        style::value("共计"),
        style::progress_num(&total_combinations.to_string())
    );

    // 创建独立的线程池用于此阶段
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .expect("无法创建线程池");

    pool.install(|| {
        for len in 1..=6 {
            if found.load(Ordering::Relaxed) {
                return None;
            }

            let start = if len == 1 {
                0
            } else {
                10u64.pow(len as u32 - 1)
            };
            let end = 10u64.pow(len as u32);
            let range_size = end - start;

            println!(
                "  {} [{}-位数字] 范围: {:0width$}-{:0width$} ({}种)",
                style::value("→"),
                len,
                start,
                end - 1,
                style::progress_num(&range_size.to_string()),
                width = len as usize
            );

            let found_in_len = (start..end).into_par_iter().find_any(|&i| {
                if found.load(Ordering::Relaxed) {
                    return true;
                }

                let pwd = format!("{:0width$}", i, width = len as usize);
                let correct = check_password(&file_buf, &pwd);

                let count = counter.fetch_add(1, Ordering::Relaxed) + 1;

                if count % 100_000 == 0 || count == 1 {
                    print_progress(count, total_combinations, start_time, &pwd);
                }

                if correct {
                    println!();
                    println!("  {} 找到密码: [{}]",
                        style::success("✔"),
                        style::found_password(&pwd)
                    );
                    found.store(true, Ordering::Relaxed);
                }

                correct
            });

            if let Some(i) = found_in_len {
                let pwd = format!("{:0width$}", i, width = len as usize);
                return Some(pwd);
            }
        }

        None
    })
}

/// 使用给定的密码列表进行字典攻击
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
            let count = counter.fetch_add(1, Ordering::Relaxed) + 1;

            if count % 10_000 == 0 || count == 1 {
                let display = if pwd.len() > 20 {
                    format!("{}...", &pwd[..20])
                } else {
                    pwd.to_string()
                };
                print_progress(count, total, start_time, &display);
            }

            if correct {
                println!();
                println!("  {} 找到密码: [{}]",
                    style::success("✔"),
                    style::found_password(pwd)
                );
                found.store(true, Ordering::Relaxed);
            }

            correct
        });

        result.cloned()
    })
}
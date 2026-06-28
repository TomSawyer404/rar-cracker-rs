use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use unrar::Archive;
use walkdir::WalkDir;


use std::process::Command;

/// 嵌入到二进制中的 UnRAR.exe（Windows 专用）
/// 编译时会自动将项目根目录的 UnRAR.exe 打包进 .exe
#[cfg(target_os = "windows")]
const EMBEDDED_UNRAR: &[u8] = include_bytes!("../UnRAR.exe");


// ============================================================
// CLI 参数定义
// ============================================================

#[derive(Parser)]
#[command(
    name = "rar-cracker",
    version = "1.0.0",
    about = "RAR文件密码暴力破解工具",
    long_about = "RAR文件密码暴力破解工具\n支持1-6位数字暴力破解、字典文件破解、字典目录破解"
)]
struct Cli {
    /// RAR文件路径（必填）
    file: PathBuf,

    /// 字典文件路径
    #[arg(short, long)]
    dictionary: Option<PathBuf>,

    /// 字典目录路径（将使用该目录下所有文件作为字典）
    #[arg(short = 'D', long)]
    dictionary_dir: Option<PathBuf>,

    /// 线程数（默认：CPU核心数，0表示使用所有核心）
    #[arg(short, long, default_value_t = 0)]
    threads: usize,
}

// ============================================================
// RAR密码验证核心函数
// ============================================================

// ┌──────────────────────────────────────────────────────────┐
// │  验证策略（两阶段）：                                     │
// │  阶段1 - test(): 快速完整性扫描（crate库，有误判可能）   │
// │  阶段2 - UnRAR.exe: 内置命令行工具最终确认（100%正确）  │
// │                                                           │
// │  crate的RARProcessFile不返回错误码，故将UnRAR.exe直接    │
// │  嵌入二进制以备调用的场景（无外部依赖）                  │
// └──────────────────────────────────────────────────────────┘

/// 快速扫描：遍历所有文件用 test() 测试完整性（不解压）
/// ⚠️ crate编译库不验证密码，只能作粗筛
fn scan_password_fast(file_path: &Path, password: &str) -> bool {
    let archive = match Archive::with_password(file_path, password).open_for_processing() {
        Ok(a) => a,
        Err(_) => return false,
    };

    let mut archive = archive;
    loop {
        match archive.read_header() {
            Ok(Some(header)) => {
                match header.test() {
                    Ok(a) => archive = a,
                    Err(_) => return false,
                }
            }
            Ok(None) => return true,
            Err(_) => return false,
        }
    }
}

/// 获取可用的 UnRAR 可执行文件路径
/// Windows：优先系统PATH，回退到内嵌版本
/// 其他平台：仅系统PATH
fn resolve_unrar_exe() -> PathBuf {
    // 先尝试系统PATH已有的 UnRAR 命令
    for name in &["UnRAR.exe", "unrar.exe", "unrar"] {
        if Command::new(name)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .is_ok()
        {
            return name.to_string().into();
        }
    }

    // Windows 专属：从内嵌字节解压到临时目录
    #[cfg(target_os = "windows")]
    {
        let cache_dir = std::env::temp_dir().join("rar_cracker_embedded");
        let _ = fs::create_dir_all(&cache_dir);
        let exe_path = cache_dir.join("UnRAR.exe");
        if !exe_path.exists() {
            fs::write(&exe_path, EMBEDDED_UNRAR)
                .expect("无法释放内嵌的 UnRAR.exe");
        }
        return exe_path;
    }

    // 所有方式均失败（非 Windows 且未安装 unrar）
    #[cfg(not(target_os = "windows"))]
    {
        if cfg!(target_os = "linux") {
            eprintln!("  错误: 未找到 unrar 命令。请在终端执行: sudo apt install unrar  (Ubuntu/Debian)");
            eprintln!("                                sudo yum install unrar  (CentOS/RHEL)");
            eprintln!("                                sudo pacman -S unrar    (Arch Linux)");
        } else if cfg!(target_os = "macos") {
            eprintln!("  错误: 未找到 unrar 命令。请先安装 Homebrew (https://brew.sh)，然后执行:");
            eprintln!("         brew install unrar");
        } else {
            eprintln!("  错误: 未找到 unrar 命令。请先安装 unrar 命令行工具。");
        }
        std::process::exit(1);
    }
}

/// 使用 UnRAR 命令行工具验证密码（100% 准确）
/// unrar t <file> -p<password> 返回 0 表示正确
fn verify_with_unrar(file_path: &Path, password: &str) -> bool {
    let exe = resolve_unrar_exe();
    let file_str = file_path.to_str().expect("非法文件路径");
    let pwd_arg = format!("-p{}", password);

    match Command::new(&exe)
        .args(["t", file_str, &pwd_arg])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
    {
        Ok(output) => output.status.success(),
        Err(e) => {
            eprintln!("     错误: 无法执行 UnRAR ({}): {}", exe.display(), e);
            false
        }
    }
}

/// 综合密码验证
fn check_password(file_path: &Path, password: &str) -> bool {
    // 阶段1：遍历所有文件做 test() 快速扫描
    if !scan_password_fast(file_path, password) {
        return false;
    }

    // 阶段2：UnRAR 命令行最终确认
    verify_with_unrar(file_path, password)
}



// ============================================================
// 阶段1: 数字暴力破解 (1-6位)
// ============================================================

/// 尝试所有1-6位数字组合
/// 总计: 10 + 100 + 1,000 + 10,000 + 100,000 + 1,000,000 = 1,111,110 种组合
fn numeric_bruteforce(
    file_path: &Path,
    found: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
    start_time: Instant,
    num_threads: usize,
) -> Option<String> {
    let file_buf = file_path.to_path_buf();
    let total_combinations: u64 = 1_111_110;

    println!("━━━ 阶段1: 数字暴力破解 (1-6位) ━━━");
    println!("  共计 {}-{} 种组合", total_combinations, total_combinations);

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

            println!("  → [{}-位数字] 范围: {:0width$}-{:0width$} ({}种)",
                len,
                start,
                end - 1,
                range_size,
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
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let rate = if elapsed > 0.0 {
                        count as f64 / elapsed
                    } else {
                        0.0
                    };
                    let progress = (count as f64 / total_combinations as f64) * 100.0;
                    println!(
                        "    进度: {:6.2}% | 已尝试: {:>8} | 速度: {:>8.0}/秒 | 用时: {:>6.1}秒 | 当前: {}",
                        progress, count, rate, elapsed, pwd
                    );
                }

                if correct {
                    println!("\n  ✓ 找到密码: [{}]", pwd);
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

// ============================================================
// 字典加载
// ============================================================

/// 从字典文件加载密码列表（自动去重）
fn load_passwords_from_file(path: &Path) -> Vec<String> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("  错误: 无法打开字典文件 '{}': {}", path.display(), e);
            return Vec::new();
        }
    };

    let reader = BufReader::new(file);
    let mut passwords: Vec<String> = reader
        .lines()
        .filter_map(|line| line.ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // 去重并保持顺序
    passwords.sort();
    passwords.dedup();

    passwords
}

// ============================================================
// 阶段2/3: 字典攻击
// ============================================================

/// 使用给定的密码列表进行字典攻击
fn dictionary_attack(
    file_path: &Path,
    passwords: &[String],
    found: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
    start_time: Instant,
    _source_name: &str,
    num_threads: usize,
) -> Option<String> {
    if passwords.is_empty() {
        println!("  ℹ  字典为空，跳过");
        return None;
    }

    println!("  → 字典大小: {} 个唯一密码", passwords.len());

    let file_buf = file_path.to_path_buf();
    let total = passwords.len();

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
                let elapsed = start_time.elapsed().as_secs_f64();
                let rate = if elapsed > 0.0 {
                    count as f64 / elapsed
                } else {
                    0.0
                };
                let progress = (count as f64 / total as f64) * 100.0;
                println!(
                    "    进度: {:6.2}% | 已尝试: {:>8} | 速度: {:>8.0}/秒 | 用时: {:>6.1}秒 | 当前: {}",
                    progress, count, rate, elapsed,
                    if pwd.len() > 20 {
                        format!("{}...", &pwd[..20])
                    } else {
                        //pwd.clone()
                        pwd.to_string()
                    }
                );
            }

            if correct {
                println!("\n  ✓ 找到密码: [{}]", pwd);
                found.store(true, Ordering::Relaxed);
            }

            correct
        });

        result.cloned()
    })
}

// ============================================================
// 程序入口
// ============================================================

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
        if let Some(dict_path) = &args.dictionary {
            if dict_path.exists() {
                println!("\n━━━ 阶段2: 字典文件破解 ━━━");
                let passwords = load_passwords_from_file(dict_path);
                dictionary_attack(
                    &args.file,
                    &passwords,
                    &found,
                    &counter,
                    start_time,
                    &dict_path.display().to_string(),
                    num_threads,
                )
            } else {
                eprintln!("警告: 字典文件 '{}' 不存在", dict_path.display());
                None
            }
        } else {
            None
        }
    });

    // ---- 阶段3: 字典目录破解 ----
    let found_password = found_password.or_else(|| {
        if let Some(dir_path) = &args.dictionary_dir {
            if dir_path.is_dir() {
                println!("\n━━━ 阶段3: 字典目录破解 ━━━");
                println!("  扫描目录: {}", dir_path.display());

                // 收集目录中的所有文件
                let dict_files: Vec<PathBuf> = WalkDir::new(dir_path)
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
                        &dict_file.display().to_string(),
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
        } else {
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
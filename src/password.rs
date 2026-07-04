use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use unrar::Archive;

/// 嵌入到二进制中的 UnRAR.exe（Windows 专用）
/// 编译时会自动将项目根目录的 UnRAR.exe 打包进 .exe
#[cfg(target_os = "windows")]
const EMBEDDED_UNRAR: &[u8] = include_bytes!("../UnRAR.exe");

// ┌──────────────────────────────────────────────────────────┐
// │  验证策略（两阶段）：                                     │
// │  阶段1 - test(): 快速完整性扫描（crate库，有误判可能）   │
// │  阶段2 - UnRAR.exe: 内置命令行工具最终确认（100%正确）  │
// │                                                           │
// │  crate的RARProcessFile不返回错误码，故将UnRAR.exe直接    │
// │  嵌入二进制以备调用的场景（无外部依赖）                  │
// └──────────────────────────────────────────────────────────┘

/// 快速扫描：遍历所有文件用 test() 测试完整性（不解压）
///
/// ⚠️ crate编译库不验证密码，只能作粗筛
pub fn scan_password_fast(file_path: &Path, password: &str) -> bool {
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
///
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
            eprintln!("  错误: 未找到 unrar 命令。请在终端执行:");
            eprintln!("    sudo apt install unrar  (Ubuntu/Debian)");
            eprintln!("    sudo yum install unrar  (CentOS/RHEL)");
            eprintln!("    sudo pacman -S unrar    (Arch Linux)");
        } else if cfg!(target_os = "macos") {
            eprintln!("  错误: 未找到 unrar 命令。请先安装 Homebrew (https://brew.sh)，然后执行:");
            eprintln!("    brew install unrar");
        } else {
            eprintln!("  错误: 未找到 unrar 命令。请先安装 unrar 命令行工具。");
        }
        std::process::exit(1);
    }
}

/// 使用 UnRAR 命令行工具验证密码（100% 准确）
///
/// `unrar t <file> -p<password>` 返回 0 表示正确
pub fn verify_with_unrar(file_path: &Path, password: &str) -> bool {
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

/// 综合密码验证（两阶段确认）
///
/// 阶段1：遍历所有文件做 test() 快速扫描
/// 阶段2：UnRAR 命令行最终确认
pub fn check_password(file_path: &Path, password: &str) -> bool {
    if !scan_password_fast(file_path, password) {
        return false;
    }

    verify_with_unrar(file_path, password)
}
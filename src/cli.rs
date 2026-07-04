use clap::Parser;
use std::path::PathBuf;

/// RAR文件密码暴力破解工具
///
/// 支持1-6位数字暴力破解、字典文件破解、字典目录破解
#[derive(Parser)]
#[command(
    name = "rar-cracker",
    version = "1.0.0",
    about = "RAR文件密码暴力破解工具"
)]
pub struct Cli {
    /// RAR文件路径（必填）
    pub file: PathBuf,

    /// 字典文件路径
    #[arg(short, long)]
    pub dictionary: Option<PathBuf>,

    /// 字典目录路径（将使用该目录下所有文件作为字典）
    #[arg(short = 'D', long)]
    pub dictionary_dir: Option<PathBuf>,

    /// 线程数（默认：CPU核心数，0表示使用所有核心）
    #[arg(short, long, default_value_t = 0)]
    pub threads: usize,
}
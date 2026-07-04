use clap::builder::styling::{AnsiColor, Styles};
use clap::Parser;
use std::path::PathBuf;

/// 自定义 clap 帮助信息样式
const CLAP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::BrightCyan.on_default().bold())
    .usage(AnsiColor::BrightCyan.on_default().bold())
    .literal(AnsiColor::BrightGreen.on_default().bold())
    .placeholder(AnsiColor::Yellow.on_default())
    .error(AnsiColor::Red.on_default().bold())
    .valid(AnsiColor::BrightGreen.on_default().bold())
    .invalid(AnsiColor::Red.on_default().bold());

/// RAR文件密码暴力破解工具
///
/// 支持1-6位数字暴力破解、字典文件破解、字典目录破解
#[derive(Parser)]
#[command(
    name = "rar-cracker",
    version = env!("CARGO_PKG_VERSION"),
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        " (", env!("BUILD_TARGET"), ")",
        " [commit ", env!("BUILD_COMMIT"), "]",
        " built ", env!("BUILD_DATE"),
    ),
    about = "RAR文件密码暴力破解工具",
    styles = CLAP_STYLES
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
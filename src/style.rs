use colored::*;

/// 标题/横幅样式（亮青色粗体）
pub fn banner(text: &str) -> String {
    text.bright_cyan().bold().to_string()
}

/// 阶段标题样式（亮蓝色粗体）
pub fn stage(text: &str) -> String {
    text.bright_blue().bold().to_string()
}

/// 成功消息样式（绿色粗体）
pub fn success(text: &str) -> String {
    text.green().bold().to_string()
}

/// 错误消息样式（红色粗体）
pub fn error(text: &str) -> String {
    text.red().bold().to_string()
}

/// 警告消息样式（黄色粗体）
pub fn warning(text: &str) -> String {
    text.yellow().bold().to_string()
}

/// 高亮值样式（亮黄色），用于突出显示关键数据（如密码、路径）
pub fn highlight(text: &str) -> String {
    text.bright_yellow().to_string()
}

/// 信息值样式（亮青色），用于显示统计数据、数字等
pub fn value(text: &str) -> String {
    text.bright_cyan().to_string()
}

/// 找到密码时的醒目样式（亮绿色粗体 + 反白背景）
pub fn found_password(text: &str) -> String {
    text.bright_green().on_black().bold().to_string()
}

/// 进度条数值样式（亮白色粗体）
pub fn progress_num(text: &str) -> String {
    text.bright_white().bold().to_string()
}
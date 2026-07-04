use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::style;

/// 从字典文件加载密码列表（自动去重）
pub fn load_passwords_from_file(path: &Path) -> Vec<String> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("  {} 无法打开字典文件 '{}': {}",
                style::error("✖"),
                path.display(),
                e
            );
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

/// 从二进制内嵌的 password_list.txt 加载密码列表（编译时嵌入，无需外部文件）
pub fn load_embedded_passwords() -> Vec<String> {
    let content = include_str!("../password_list.txt");
    let mut passwords: Vec<String> = content
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    passwords.sort();
    passwords.dedup();

    passwords
}
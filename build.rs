fn main() {
    // unrar_sys 编译 unrar C++ 源码时遗漏了部分 Windows 系统库的链接
    // 这里补充链接必要的系统库
    println!("cargo:rustc-link-lib=advapi32");
}
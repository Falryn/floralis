fn main() {
    // 默认不监听图标文件：同名替换图标时 cargo 不会重编译，
    // 导致 exe 继续内嵌旧图标。显式声明监听 icons 目录。
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build();
}

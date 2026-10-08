use std::env;
use std::path::PathBuf;

fn main() {
    // C++ 製 USB ドライバ (kernel/cpp 以下, `make` で libmikan_usb.a を生成) をリンクする
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let lib_dir = PathBuf::from(manifest_dir).join("cpp");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=mikan_usb");

    // ライブラリ更新時に再リンクさせる
    println!("cargo:rerun-if-changed=cpp/libmikan_usb.a");
}

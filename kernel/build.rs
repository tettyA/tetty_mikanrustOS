use std::env;
use std::path::PathBuf;

fn main() {
    // Cargo.tomlが存在するディレクトリ（kernel/）の絶対パスを環境変数から取得します
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let lib_path = PathBuf::from(manifest_dir).join("usb_wrapper");

    // 絶対パスを用いて検索ディレクトリを登録します
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=static=usb_wrapper");

    // キャッシュの誤判定を防ぐため、ライブラリファイル更新時に再実行されるよう指定します
    println!("cargo:rerun-if-changed=usb_wrapper/libusb_wrapper.a");
    //
}

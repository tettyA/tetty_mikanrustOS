#!/bin/bash
set -e
ORIGINAL_DIR=$(pwd)

# Build Bootloader
cd $HOME/edk2
source edksetup.sh

[ -L MikanLoaderPkg ] && unlink MikanLoaderPkg
ln -s $ORIGINAL_DIR/MikanLoaderPkg ./
rm -rf Build/MikanLoaderX64

build

mv Build/MikanLoaderX64/DEBUG_CLANG38/X64/Loader.efi $ORIGINAL_DIR/Loader.efi

# Build C++ USB driver library (kernel/cpp)

source $HOME/osbook/devenv/buildenv.sh

cd "$ORIGINAL_DIR/kernel/cpp/"
make

# Build Rust Kernel
cd "$ORIGINAL_DIR"

cargo build --release -p kernel

cp target/x86_64-unknown-none-elf/release/kernel $ORIGINAL_DIR/kernel.elf

# apps 配下の全アプリを自動ビルト
if [ -d "$ORIGINAL_DIR/apps" ]; then
    for MK in $(ls $ORIGINAL_DIR/apps/*/Makefile 2>/dev/null || true);
do
        APP_DIR=$(dirname $MK)
        APP=$(basename $APP_DIR)
        make -C "$APP_DIR" "$APP"
    done
fi

# ─── ディスクイメージの作成とファイルのコピー ───
# 1. ベースイメージの作成
$HOME/osbook/devenv/make_image.sh ./disk.img ./mnt $ORIGINAL_DIR/Loader.efi $ORIGINAL_DIR/kernel.elf
# 2. マウントして apps 配下の実行ファイルをすべてコピー
$HOME/osbook/devenv/mount_image.sh ./disk.img ./mnt
if [ -d "$ORIGINAL_DIR/apps" ]; then
    for APP in $(ls "$ORIGINAL_DIR/apps"); do
        if [ -f "$ORIGINAL_DIR/apps/$APP/$APP" ]; then
            sudo cp "$ORIGINAL_DIR/apps/$APP/$APP" ./mnt/
        fi
    done
fi

sleep 0.5
sudo umount ./mnt

# 3. QEMU で起動
$HOME/osbook/devenv/run_image.sh ./disk.img
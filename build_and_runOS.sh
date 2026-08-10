#!/bin/bash
set -e
ORIGINAL_DIR=$(pwd)

# Build Bootloader
cd $HOME/edk2
source edksetup.sh

unlink MikanLoaderPkg
ln -s $ORIGINAL_DIR/MikanLoaderPkg ./
rm -rf Build/MikanLoaderX64

build

mv Build/MikanLoaderX64/DEBUG_CLANG38/X64/Loader.efi $ORIGINAL_DIR/Loader.efi

# Build Rust Kernel
cd "$ORIGINAL_DIR"

cargo build --release -p kernel

cp target

# Run the OS in QEMU
$HOME/osbook/devenv/run_qemu.sh $ORIGINAL_DIR/Loader.efi


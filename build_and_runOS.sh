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

# Run the OS in QEMU
$HOME/osbook/devenv/run_qemu.sh $ORIGINAL_DIR/Loader.efi $ORIGINAL_DIR/kernel.elf


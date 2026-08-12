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

# Build C++ Wraper Library

source $HOME/osbook/devenv/buildenv.sh
cd "$ORIGINAL_DIR"/kernel/usb_wrapper/
nasm -f elf64 -o asmfunc.o asmfunc.asm
clang++ $CPPFLAGS -I. -O2 -Wall -g --target=x86_64-elf -ffreestanding -mno-red-zone -fno-exceptions -fno-rtti -std=c++17 -c pci.cpp usb_wrapper.cpp
ar rcs $ORIGINAL_DIR/kernel/usb_wrapper/libusb_wrapper.a pci.o usb_wrapper.o asmfunc.o


# Build Rust Kernel
cd "$ORIGINAL_DIR"

cargo build --release -p kernel

cp target/x86_64-unknown-none-elf/release/kernel $ORIGINAL_DIR/kernel.elf

# Run the OS in QEMU
$HOME/osbook/devenv/run_qemu.sh $ORIGINAL_DIR/Loader.efi $ORIGINAL_DIR/kernel.elf


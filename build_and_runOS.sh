#!/bin/bash
set -e
ORIGINAL_DIR=$(pwd)
#:
# Build the OS
cd $HOME/edk2

unlink MikanLoaderPkg
ln -s $ORIGINAL_DIR/../MikanLoaderPkg ./
rm -rf Build/MikanLoaderX64

#source edksetup.sh
build

mv Build/MikanLoaderX64/DEBUG_CLANG38/X64/Loader.efi $ORIGINAL_DIR/Loader.efi

cd "$ORIGINAL_DIR"
# Run the OS in QEMU
$HOME/osbook/devenv/run_qemu.sh $ORIGINAL_DIR/Loader.efi


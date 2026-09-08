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

# Build Rust Kernel
cd "$ORIGINAL_DIR"

cargo build --release -p kernel

cp target/x86_64-unknown-none-elf/release/kernel $ORIGINAL_DIR/kernel.elf

# Run the OS in QEMU. Use QEMU_MODE=manual only for the final interactive check.
QEMU_MODE=${QEMU_MODE:-timeout}
QEMU_TIMEOUT_SECONDS=${QEMU_TIMEOUT_SECONDS:-150}
if [[ "$QEMU_MODE" == "manual" ]]; then
	"$HOME/osbook/devenv/run_qemu.sh" "$ORIGINAL_DIR/Loader.efi" "$ORIGINAL_DIR/kernel.elf"
else
	timeout --foreground --signal=TERM "${QEMU_TIMEOUT_SECONDS}s" \
		"$HOME/osbook/devenv/run_qemu.sh" "$ORIGINAL_DIR/Loader.efi" "$ORIGINAL_DIR/kernel.elf" || {
			status=$?
			if [[ $status -ne 124 ]]; then
				exit "$status"
			fi
			echo "QEMU stopped after ${QEMU_TIMEOUT_SECONDS}s (timeout mode)."
	}
fi


# kernel/cpp : MikanOS 由来の C++ コード (USB ドライバ, PCI) を静的ライブラリ
# libmikan_usb.a にまとめる．Rust 側 (kernel/build.rs) がこれをリンクする．
#
# 使い方: source $HOME/osbook/devenv/buildenv.sh してから `make` (build_and_runOS.sh が実行する)
TARGET = libmikan_usb.a

OBJS = usb/memory.o usb/device.o \
       usb/xhci/ring.o usb/xhci/trb.o usb/xhci/xhci.o usb/xhci/port.o \
       usb/xhci/device.o usb/xhci/devmgr.o usb/xhci/registers.o \
       usb/classdriver/base.o usb/classdriver/hid.o \
       usb/classdriver/keyboard.o usb/classdriver/mouse.o \
       pci.o asmfunc.o logger.o \
       support/cxx_stubs.o support/libcxx_support.o \
       bridge/rust_bridge.o

CFLAGS   += -O2 -Wall -g --target=x86_64-elf -ffreestanding -mno-red-zone
CPPFLAGS += -I.
CXXFLAGS += -O2 -Wall -g --target=x86_64-elf -ffreestanding -mno-red-zone \
            -fno-exceptions -fno-rtti -std=c++17

.PHONY: all
all: $(TARGET)

.PHONY: clean
clean:
	rm -f $(OBJS) $(TARGET)

%.o: %.cpp
	clang++ $(CPPFLAGS) $(CXXFLAGS) -c $< -o $@

%.o: %.c
	clang $(CPPFLAGS) $(CFLAGS) -c $< -o $@

%.o: %.asm
	nasm -f elf64 -o $@ $<

$(TARGET): $(OBJS)
	ar rcs $@ $^

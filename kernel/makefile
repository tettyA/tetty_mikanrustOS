TARGET=usb_wrapper/libusb_wrapper.a
OBJS = usb/memory.o usb/device.o usb/xhci/ring.o usb/xhci/trb.o usb/xhci/xhci.o \
       usb/xhci/port.o usb/xhci/device.o usb/xhci/devmgr.o usb/xhci/registers.o \
       usb/classdriver/base.o usb/classdriver/hid.o usb/classdriver/keyboard.o \
       usb/classdriver/mouse.o \
	   usb_wrapper/asmfunc.o \
	   usb_wrapper/pci.o usb_wrapper/usb_wrapper.o


CPPFLAGS += -I.
CXXFLAGS += -O2 -Wall -g --target=x86_64-elf -ffreestanding -mno-red-zone \
            -fno-exceptions -fno-rtti -std=c++17

.PHONY: all
all: $(TARGET)

.PHONY: clean
clean:
	rm -rf *.o


%.o: %.cpp 
	clang++ $(CPPFLAGS) $(CXXFLAGS) -c $< -o $@

%.o: %.asm 
	nasm -f elf64 -o $@ $<

$(TARGET): $(OBJS)
	ar rcs $@ $^
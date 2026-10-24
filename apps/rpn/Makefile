TARGET = rpn

RUSTFLAGS = --target x86_64-unknown-none \
            -C relocation-model=static \
            -C link-arg=--entry=main \
            -C link-arg=-z \
            -C link-arg=norelro \
            -C code-model=large \
            -C link-arg=--image-base=0xffff800000000000 \
            -C link-arg=--static \
            -C link-arg=--nmagic \
            -O

.PHONY: all
all: $(TARGET)

rpn: main.rs Makefile
	rustc $(RUSTFLAGS) main.rs -o $@
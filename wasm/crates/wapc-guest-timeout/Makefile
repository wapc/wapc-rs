.PHONY: build clean

NAME=wapc_guest_timeout
TARGET=wasm32-wasip1

build: build/$(NAME).wasm

build/$(NAME).wasm: target/$(TARGET)/release/$(NAME).wasm
	mkdir -p build && cp $< $@

target/$(TARGET)/release/$(NAME).wasm:
	cargo build --target $(TARGET) --release

clean:
	cargo clean
	rm -Rf build

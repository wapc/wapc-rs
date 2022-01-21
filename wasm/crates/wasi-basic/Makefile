.PHONY: build clean

NAME=wasi_basic
TARGET=wasm32-wasi

build: build/$(NAME).wasm

build/$(NAME).wasm: target/$(TARGET)/release/$(NAME).wasm
	mkdir -p build && cp $< $@

target/$(TARGET)/release/$(NAME).wasm:
	cargo build --target $(TARGET) --release

clean:
	cargo clean
	rm -Rf build

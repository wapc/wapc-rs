.PHONY: build clean

NAME=wasm_basic
TARGET=wasm32-unknown-unknown

build: build/$(NAME).wasm

build/$(NAME).wasm: target/$(TARGET)/release/$(NAME).wasm
	mkdir -p build && cp $< $@

target/$(TARGET)/release/$(NAME).wasm:
	cargo build --target $(TARGET) --release

clean:
	cargo clean
	rm -Rf build

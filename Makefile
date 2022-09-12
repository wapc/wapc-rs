# Enforce bash as the shell for consistency
SHELL := bash
# Use bash strict mode
.SHELLFLAGS := -eu -o pipefail -c
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules
MAKEFLAGS += --no-print-directory

WASM_PROJECT_DIR=./wasm/crates

WAPC_GUEST_DIR=$(WASM_PROJECT_DIR)/wapc-guest-test/
WAPC_GUEST_WASM=$(WAPC_GUEST_DIR)/build/wapc_guest_test.wasm

TEST_WASM_DIR=$(WASM_PROJECT_DIR)/wasm-basic/
TEST_WASM_WASM=$(TEST_WASM_DIR)/build/wasm_basic.wasm
TEST_WASI_DIR=$(WASM_PROJECT_DIR)/wasi-basic/
TEST_WASI_WASM=$(TEST_WASI_DIR)/build/wasi_basic.wasm
TEST_WAPC_TIMEOUT_DIR=$(WASM_PROJECT_DIR)/wapc-guest-timeout/
TEST_WAPC_TIMEOUT_WASM=$(TEST_WASI_DIR)/build/wapc_guest_timeout.wasm

.PHONY: all
all: build

.PHONY: clean
clean:
	cargo clean
	$(MAKE) -C $(WAPC_GUEST_DIR) clean
	$(MAKE) -C $(TEST_WASM_DIR) clean
	$(MAKE) -C $(TEST_WASI_DIR) clean
	$(MAKE) -C $(TEST_WAPC_TIMEOUT_DIR) clean

.PHONY: build
build:
	cargo build --workspace --all

$(WAPC_GUEST_WASM):
	$(MAKE) -C $(WAPC_GUEST_DIR)

$(TEST_WASI_WASM):
	$(MAKE) -C $(TEST_WASI_DIR)

$(TEST_WASM_WASM):
	$(MAKE) -C $(TEST_WASM_DIR)

$(TEST_WAPC_TIMEOUT_WASM):
	$(MAKE) -C $(TEST_WAPC_TIMEOUT_DIR)

.PHONY: wasm
wasm: $(WAPC_GUEST_WASM) $(TEST_WASI_WASM) $(TEST_WASM_WASM) $(TEST_WAPC_TIMEOUT_WASM)

.PHONY: check
check:
	cargo +nightly fmt --check
	cargo clippy

.PHONY: tidy
tidy:
	cargo +nightly fmt

.PHONY: test
test: wasm
	cargo test --workspace

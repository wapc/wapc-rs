
# Enforce bash as the shell for consistency
SHELL := bash
# Use bash strict mode
.SHELLFLAGS := -eu -o pipefail -c
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules
MAKEFLAGS += --no-print-directory

.PHONY: test
test:
	cargo test
	cargo test --no-default-features
	cargo test --features cache

.PHONY: lint
lint:
	cargo check
	cargo clippy -- -D warnings
	cargo fmt --all -- --check

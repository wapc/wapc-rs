
# Enforce bash as the shell for consistency
SHELL := bash
# Use bash strict mode
.SHELLFLAGS := -eu -o pipefail -c
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules
MAKEFLAGS += --no-print-directory

.PHONY: test
test:
	@echo "Running tests with default features"
	cargo test
	@echo "Running tests with cache feature enabled"
	cargo test --no-default-features --features cache
	@echo "Running tests with default features disabled"
	cargo test --no-default-features
	@echo "Running tests with only cache feature enabled"
	cargo test --no-default-features --features cache
	@echo "Running tests with only wasi feature enabled"
	cargo test --no-default-features --features wasi
	@echo "Running tests with only async feature enabled"
	cargo test --no-default-features --features async

.PHONY: lint
lint:
	cargo check
	cargo clippy -- -D warnings
	cargo fmt --all -- --check

WASM_TARGET ?= wasm32v1-none

.PHONY: build test lint fmt optimize coverage clean

build:
	stellar contract build

test:
	cargo test --workspace

lint:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

optimize:
	stellar contract build

coverage:
	cargo llvm-cov --workspace --summary-only

clean:
	cargo clean

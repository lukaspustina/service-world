all: check test build

build:
	cargo build

check:
	cargo check

test:
	cargo test

clippy:
	rustup run nightly cargo clippy

fmt:
	rustup run nightly cargo fmt

duplicate_libs:
	cargo tree -d

_update-clippy_n_fmt:
	rustup update
	rustup run nightly cargo install clippy --force
	rustup run nightly cargo install rustfmt --force


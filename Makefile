default: fmt clippy test check

test:
	cargo test --all --all-features

clippy:
	cargo clippy  --all --all-features --all-targets

fmt:
	cargo fmt --all -- --check

check:
	cargo check --no-default-features

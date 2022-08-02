default: fmt clippy test bench-test check

test:
	cargo test --all --features "std blake2b" && cargo test --features utf8-keys -- --skip tests

bench-test:
	cargo bench -- --test

clippy:
	cargo clippy  --all --all-features --all-targets

fmt:
	cargo fmt --all -- --check

check:
	cargo check --no-default-features

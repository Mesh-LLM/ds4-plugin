build:
    cargo build --locked

verify:
    cargo fmt --all --check
    cargo check --locked --all-targets
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked

clean:
    cargo clean

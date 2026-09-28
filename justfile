build:
    cargo build --locked

verify:
    cargo fmt --all --check
    cargo check --locked --all-targets
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked

clean:
    cargo clean

# Build one self-contained trial plugin, including pinned DwarfStar and Metal assets.
package-macos source:
    bash scripts/package-macos.sh '{{source}}'

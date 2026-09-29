build:
    cargo build --locked

verify:
    cargo fmt --all --check
    cargo check --locked --all-targets
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked
    PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p 'test_*.py'

# Opt-in: caller owns this already running endpoint; performs inference.
acceptance base_url:
    PYTHONDONTWRITEBYTECODE=1 python3 scripts/acceptance.py --base-url '{{base_url}}'

clean:
    cargo clean

# Build one self-contained release plugin, including pinned DwarfStar and Metal assets.
package-macos source:
    bash scripts/package-macos.sh '{{source}}'

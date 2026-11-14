set dotenv-load

scope-check:
    bash scripts/check-card-scope.sh

check: scope-check
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace

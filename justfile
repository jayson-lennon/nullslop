test:
    cargo nextest run --workspace --all-features
    cargo test --workspace --doc

check:
    cargo check --workspace

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

fmt:
    cargo fmt -- --check

fmt-fix:
    cargo fmt

# Run all linters (check + clippy + fmt check)
lint:
    cargo check --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt -- --check

# Full CI pipeline (lint + test + docs)
ci: lint
    cargo nextest run --workspace --all-features
    cargo test --workspace --doc
    cargo doc --workspace --no-deps

# Build and open documentation
docs:
    cargo doc --workspace --no-deps --open

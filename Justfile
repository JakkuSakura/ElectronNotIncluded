default:
    cargo run -p eni

check:
    cargo check --workspace

test:
    cargo test --workspace

format:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

verify: format check test

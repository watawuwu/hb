name := "hb"

SHELL := "bash"

export RUST_LOG := "hb=info"
export RUST_BACKTRACE := "full"
# for tracing
export RUSTFLAGS := "--cfg tokio_unstable"

default:
    just --list

# Execute a main.rs
run *args:
    cargo run --bin {{name}} {{ args }}

# Run the tests
test: fix fmt clippy
    cargo nextest run

# Check syntax, but don't build object files
check: fix fmt clippy
    cargo check

# Build all project
build:
    cargo build

# Build all project
release-build:
    cargo build --release

# Check module version
check-lib:
    cargo outdated -R

# Update modules
update:
    cargo update

# Remove the target directory
clean:
    cargo clean

# Run fmt
fix:
    cargo fix --allow-staged --allow-dirty

# Run fmt
fmt:
    cargo fmt

# Run fmt
fmt-check:
    cargo fmt --all -- --check

# Run clippy
clippy:
    cargo clippy --all-features -- -D warnings

# Run benchmark
bench:
    cargo bench

# Audit your dependencies for crates with security vulnerabilities reported
audit:
    cargo audit

# Build container
container version *options:
    docker buildx build --platform=linux/amd64 -t ghcr.io/watawuwu/{{name}}:{{version}} {{options}} .
    docker buildx build --platform=linux/arm64 -t ghcr.io/watawuwu/{{name}}:{{version}} {{options}} .

# SouceCode base coverage
coverage:
    cargo llvm-cov --open

# Watch task
watch *args:
    cargo watch -x "{{ args }}"

# Watch test
watch-test *options:
    cargo watch -x 'nextest run {{ options }}'

dev *args:
    cargo run -- -d 60s --otlp-endpoint="http://localhost:9090/api/v1/otlp/v1/metrics" http://localhost:8080/ {{ args }}

# syntax=docker/dockerfile:1

FROM rust AS builder

ARG APP_NAME

WORKDIR /app

RUN --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=bind,source=client/src,target=client/src \
    --mount=type=bind,source=client/Cargo.toml,target=client/Cargo.toml \
    --mount=type=bind,source=controller/src,target=controller/src \
    --mount=type=bind,source=controller/Cargo.toml,target=controller/Cargo.toml \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --locked --release --bin $APP_NAME
cp ./target/release/$APP_NAME /bin/app
EOF

FROM gcr.io/distroless/cc AS final

COPY --from=builder /bin/app /bin/app

ENTRYPOINT ["/bin/app"]

FROM rust AS builder

WORKDIR /app

COPY . .

RUN cargo build --release --bin hb

FROM gcr.io/distroless/cc

COPY --from=builder /app/target/release/hb /bin/hb

ENTRYPOINT ["/bin/hb"]


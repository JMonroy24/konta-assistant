FROM rust:1.80-slim as builder
WORKDIR /app
COPY control-x0 /app/control-x0
WORKDIR /app/control-x0
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/control-x0/target/release/control-x0 /app/control-x0
EXPOSE 8082
CMD ["/app/control-x0"]

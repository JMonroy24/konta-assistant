FROM rust:1.80-slim as builder
WORKDIR /app
COPY mock-hacienda /app/mock-hacienda
WORKDIR /app/mock-hacienda
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/mock-hacienda/target/release/mock-hacienda /app/mock-hacienda
EXPOSE 8081
CMD ["/app/mock-hacienda"]

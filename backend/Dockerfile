FROM rust:1.80-slim as builder
WORKDIR /app
COPY backend /app/backend
WORKDIR /app/backend
RUN cargo build --release -p api

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/backend/target/release/api /app/konta-api
EXPOSE 8080
CMD ["/app/konta-api"]

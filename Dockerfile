# Build stage
FROM rust:1.76.0 AS builder
WORKDIR /app
RUN apt update && apt install lld clang -y
COPY . .
RUN cargo build --release

# Runtime stage
FROM rust:1.76.0 AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/uniswap-watcher uniswap-watcher
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./uniswap-watcher"]
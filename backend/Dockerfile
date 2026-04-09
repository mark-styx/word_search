FROM rust:1.82-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Cache deps by building a dummy lib
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/word-search-backend ./server
COPY migrations ./migrations
RUN mkdir -p /app/data

ENV DATABASE_PATH=/app/data/wordsearch.db
ENV PORT=3001
ENV HOST=0.0.0.0
ENV JWT_SECRET=change-me-in-production
ENV RUST_LOG=info

EXPOSE 3001
CMD ["./server"]

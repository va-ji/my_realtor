# Multi-stage build for API server
FROM rust:latest as builder

WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Skip SQLx compile-time verification (use offline mode)
ENV SQLX_OFFLINE=true

# Build the API server binary in release mode
RUN cargo build --release --bin api-server

# Runtime stage - use debian slim for smaller image
FROM debian:bookworm-slim

# Install required runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from builder
COPY --from=builder /app/target/release/api-server /usr/local/bin/api-server

# Set environment variables
ENV RUST_LOG=info
ENV API_PORT=3001

# Expose the API port
EXPOSE 3001

# Run the API server
CMD ["api-server"]

# Multi-stage build for VectorLawDB
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# Build release binaries
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries
COPY --from=builder /app/target/release/vldb /usr/local/bin/
COPY --from=builder /app/target/release/vectorlawdb-server /usr/local/bin/

# Create data directory
RUN mkdir -p /data

EXPOSE 8000

# Default to server mode
CMD ["vectorlawdb-server"]

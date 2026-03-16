# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache git musl-dev sqlite-dev

# Set working directory
WORKDIR /build

# Copy Cargo.toml files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY agfs-sdk ./agfs-sdk
COPY agfs-server ./agfs-server
COPY agfs-fuse ./agfs-fuse

# Build the workspace
RUN cargo build --workspace --release

# Runtime stage
FROM alpine:latest

# Install runtime dependencies (including Python 3 for agfs-shell and fuse3 for agfs-fuse)
RUN apk add --no-cache ca-certificates sqlite-libs python3 jq oniguruda fuse3 wget tar

# Create app directory
WORKDIR /app

# Copy binaries from builder
COPY --from=builder /build/target/release/agfs-server .
COPY --from=builder /build/target/release/agfs-fuse /usr/local/bin/agfs-fuse

# TODO: Copy agfs-shell when Python integration is ready
# COPY --from=builder /build-shell/dist/agfs-shell-portable /usr/local/agfs-shell

# Create directories
RUN mkdir -p /data /mnt/agfs

# Expose default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -q --spider http://localhost:8080/api/v1/health || exit 1

# Run agfs-server
ENTRYPOINT ["./agfs-server"]

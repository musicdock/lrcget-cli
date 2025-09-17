# Multi-stage build for smaller final image
FROM rust:1.80-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig

# Set the working directory
WORKDIR /app

# Copy source code
COPY . .

# Build the application in release mode
RUN cargo build --release

# Runtime stage - use minimal Alpine image
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    sqlite \
    && rm -rf /var/cache/apk/*

# Create a non-root user
RUN adduser -D -s /bin/sh lrcget

# Create data directory
RUN mkdir -p /data && chown lrcget:lrcget /data

# Copy the binary from builder stage
COPY --from=builder /app/target/release/lrcget /usr/local/bin/lrcget

# Set executable permissions
RUN chmod +x /usr/local/bin/lrcget

# Switch to non-root user
USER lrcget

# Set working directory
WORKDIR /data

# Set environment variables for Docker
ENV DOCKER=1
ENV LRCGET_DATABASE_PATH=/data/lrcget.db
ENV LRCGET_LRCLIB_INSTANCE=https://lrclib.net

# Expose volume for persistent data
VOLUME ["/data"]

# Default command
ENTRYPOINT ["/usr/local/bin/lrcget"]
CMD ["--help"]
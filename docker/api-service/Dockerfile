# Multi-stage build for minimal final image
FROM rust:1.84-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Final stage - minimal image
FROM alpine:latest

# Install ca-certificates for HTTPS
RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/docker-api-service /app/docker-api-service

# Expose port
EXPOSE 8000

# Run the binary
CMD ["/app/docker-api-service"]

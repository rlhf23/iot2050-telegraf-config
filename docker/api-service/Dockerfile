# Multi-stage build for minimal final image
FROM rust:1.84-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy main project files (needed for dependency)
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY build.rs ./

# Copy api-service files
COPY docker/api-service/Cargo.toml ./docker/api-service/
COPY docker/api-service/src ./docker/api-service/src

# Build api-service
WORKDIR /build/docker/api-service
RUN cargo build --release

# Final stage - minimal image
FROM alpine:latest

# Install ca-certificates for HTTPS and wget for health checks
RUN apk add --no-cache ca-certificates wget

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/docker/api-service/target/release/docker-api-service /app/docker-api-service

# Expose port
EXPOSE 8000

# Run the binary
CMD ["/app/docker-api-service"]

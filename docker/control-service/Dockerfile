# Dockerfile for building from source (development)
# For production, use Dockerfile.prebuilt with pre-compiled binaries

FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests (paths relative to docker/ build context)
COPY control-service/Cargo.toml ./

# Create src directory and copy source files
COPY control-service/src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/release/control-service /app/control-service

# Make it executable
RUN chmod +x /app/control-service

EXPOSE 8002

ENV CONTROL_CONFIG=/config/control/config.toml

CMD ["/app/control-service"]
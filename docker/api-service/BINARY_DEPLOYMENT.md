# Pre-built Binary Deployment Guide

This guide explains how to build and deploy the api-service using pre-built binaries instead of compiling on the IoT device.

## Why Pre-built Binaries?

- ✅ **Fast deployment** - No 10+ minute compilation on device
- ✅ **No Rust toolchain needed** on device
- ✅ **Consistent builds** - Built in controlled Nix environment
- ✅ **Works on resource-constrained devices**

## Building the Binary (NixOS)

### Prerequisites

Ensure your `flake.nix` has:
- `cargo-cross` in buildInputs
- `aarch64-unknown-linux-gnu` target
- `pkgsCross.aarch64-multiplatform.stdenv.cc` for cross-compilation
- Environment variables for cross-compiled OpenSSL

### Build Steps

```bash
# Enter Nix development shell
nix develop

# Navigate to api-service directory
cd docker/api-service

# Build for aarch64 (ARM64)
cargo build --target aarch64-unknown-linux-gnu --release

# or possibly
cross build --target aarch64-unknown-linux-gnu --features vendored --release

# Copy and patch the binary
cp target/aarch64-unknown-linux-gnu/release/docker-api-service ./docker-api-service
patchelf --set-interpreter /lib/ld-linux-aarch64.so.1 docker-api-service
chmod +x docker-api-service

# Verify it's correct
file docker-api-service
# Should show: ELF 64-bit LSB pie executable, ARM aarch64, interpreter /lib/ld-linux-aarch64.so.1

# Commit and push
git add docker-api-service
git commit -m "chore: Update pre-built aarch64 api-service binary"
git push origin webui-config-generator
```

### Why patchelf?

The Nix-built binary has a hardcoded interpreter path pointing to `/nix/store/...`. We patch it to use the standard system path `/lib/ld-linux-aarch64.so.1` so it works in the Debian Docker container.

## Deploying to Device

On the IoT device:

```bash
cd ~/monitoring  # or wherever you cloned the repo
git pull origin webui-config-generator

# Build and start the service
docker compose build api-service
docker compose up -d api-service

# Check logs
docker compose logs -f api-service
```

## Configuration Files

### `.cargo/config.toml`
```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-unknown-linux-gnu-gcc"
```

### `Dockerfile.prebuilt`
Uses Debian bookworm-slim (glibc-compatible) instead of Alpine (musl):
```dockerfile
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3
COPY api-service/docker-api-service /app/docker-api-service
CMD ["/app/docker-api-service"]
```

## Checking Device Architecture

```bash
uname -m
# x86_64  → use x86_64 target
# aarch64 → use aarch64 target (ARM64)
```

## Troubleshooting

**"no such file or directory" error:**
- Binary interpreter path is wrong - run `patchelf` to fix it
- Check with: `file docker-api-service`

**Missing libraries:**
- Check dependencies: `ldd docker-api-service`
- Ensure Dockerfile.prebuilt has required packages (libssl3, etc.)

**Build fails:**
- Reload Nix shell: `exit` then `nix develop`
- Check cross-compiler is available: `which aarch64-unknown-linux-gnu-gcc`
- Verify OpenSSL environment variables are set

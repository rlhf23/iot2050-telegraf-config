# Pre-built Binary Deployment Guide

This guide explains how to deploy the api-service using pre-built binaries instead of compiling on the IoT device.

## Why Pre-built Binaries?

- ✅ **Fast deployment** - No 10+ minute compilation on device
- ✅ **No Rust toolchain needed** on device
- ✅ **Consistent builds** - Built in controlled environment
- ✅ **Works on resource-constrained devices**

## The Workflow

### 1. Trigger GitHub Actions Build

When you need to rebuild the binary (after code changes):

**Option A: Via GitHub Web UI**
1. Go to: https://github.com/rlhf23/iot2050-telegraf-config/actions
2. Click "Build API Service Binary" workflow
3. Click "Run workflow" button
4. Select branch (e.g., `webui-config-generator`)
5. Click "Run workflow"

**Option B: Via GitHub CLI**
```bash
gh workflow run "Build API Service Binary" --ref webui-config-generator
```

**Option C: Automatic**
The workflow also runs automatically when you push changes to:
- `docker/api-service/**`
- `src/**`
- `Cargo.toml`
- `Cargo.lock`

### 2. Wait for Build to Complete (~5 minutes)

Monitor at: https://github.com/rlhf23/iot2050-telegraf-config/actions

The workflow builds binaries for:
- `amd64` (x86_64) - Most IoT2050 devices
- `arm64` (aarch64) - ARM-based devices

### 3. Download the Binary

**Option A: Using GitHub CLI (Easiest)**
```bash
cd /home/nix/iot2050-telegraf-config/docker/api-service

# Download amd64 binary
gh run download --repo rlhf23/iot2050-telegraf-config -n api-service-amd64

# Or download arm64 binary
gh run download --repo rlhf23/iot2050-telegraf-config -n api-service-arm64
```

**Option B: Manual Download**
1. Go to the workflow run page
2. Scroll to "Artifacts" section
3. Download `api-service-amd64` or `api-service-arm64`
4. Extract the zip file
5. Rename to `docker-api-service`

### 4. Commit the Binary to Git

```bash
cd /home/nix/iot2050-telegraf-config/docker/api-service

# Make sure the binary is named correctly
mv docker-api-service-amd64 docker-api-service
# Or: mv docker-api-service-arm64 docker-api-service

# Make it executable
chmod +x docker-api-service

# Add to git
git add docker-api-service

# Commit
git commit -m "chore: Update pre-built api-service binary"

# Push
git push origin webui-config-generator
```

### 5. Deploy to Device

On the IoT device:

```bash
cd ~/monitoring  # or wherever you cloned the repo
git pull origin webui-config-generator

cd docker
docker compose build api-service  # Uses Dockerfile.prebuilt
docker compose up -d api-service
```

## Dockerfile Configuration

The `Dockerfile.prebuilt` is configured to use the pre-built binary:

```dockerfile
FROM alpine:latest
RUN apk add --no-cache ca-certificates wget libgcc
COPY docker-api-service /app/docker-api-service
CMD ["/app/docker-api-service"]
```

To use it, update `docker-compose.yml`:

```yaml
api-service:
  build:
    context: .
    dockerfile: api-service/Dockerfile.prebuilt
```

## Checking Device Architecture

If you're not sure which binary to use:

```bash
# On the IoT device
uname -m

# Output:
# x86_64  → use amd64 binary
# aarch64 → use arm64 binary
```

## Pros & Cons

### Pros:
- ✅ Super fast deployment (no compilation)
- ✅ Simple workflow
- ✅ No Rust toolchain needed on device
- ✅ Consistent, reproducible builds

### Cons:
- ⚠️ Binary in git (adds ~10-20MB to repo)
- ⚠️ Need to remember to rebuild after code changes
- ⚠️ Architecture-specific (need right binary for device)

## Alternative: Build Locally

If you prefer to build on your machine instead of GitHub Actions:

```bash
# On your NixOS machine
cd /home/nix/iot2050-telegraf-config/docker/api-service

# Build for amd64
cargo build --release --target x86_64-unknown-linux-musl

# Or build for arm64
cargo build --release --target aarch64-unknown-linux-musl

# Copy the binary
cp ../../target/x86_64-unknown-linux-musl/release/docker-api-service ./

# Commit and push
git add docker-api-service
git commit -m "chore: Update api-service binary"
git push
```

## Troubleshooting

**Binary won't run on device:**
- Check architecture matches (`uname -m`)
- Ensure binary is executable (`chmod +x`)
- Check for missing libraries (`ldd docker-api-service`)

**GitHub Actions build fails:**
- Check the Actions tab for error logs
- Ensure all dependencies are in Cargo.toml
- Verify the build context is correct

**Docker build fails:**
- Ensure binary exists in `docker/api-service/` directory
- Check Dockerfile.prebuilt is being used
- Verify binary is executable

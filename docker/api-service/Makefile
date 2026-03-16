# Makefile for api-service cross-compilation
# Usage: make TARGET=aarch64-unknown-linux-gnu (default: arm64)
#        make TARGET=x86_64-unknown-linux-gnu (for amd64)

# Default to arm64 (for IoT devices)
TARGET ?= aarch64-unknown-linux-gnu
BINARY_NAME = docker-api-service

# Auto-detect cross or cargo
CARGO_CMD := $(shell command -v cross 2> /dev/null)
ifndef CARGO_CMD
    CARGO_CMD := cargo
else
    CARGO_CMD := cross
endif

# Determine arch suffix and settings from target
ifeq ($(TARGET),aarch64-unknown-linux-gnu)
    ARCH = arm64
    RELEASE_DIR = target/aarch64-unknown-linux-gnu/release
    INTERPRETER = /lib/ld-linux-aarch64.so.1
    NEEDS_PATCH = true
endif

ifeq ($(TARGET),x86_64-unknown-linux-gnu)
    ARCH = amd64
    RELEASE_DIR = target/release
    NEEDS_PATCH = false
    # For x86_64, use native cargo (not cross)
    CARGO_CMD = cargo
endif

.PHONY: all build patch copy clean test build-arm64 build-amd64 all-archs

# Default target: build for the configured TARGET
all: build patch copy

# Convenience targets for specific architectures
build-arm64:
	$(MAKE) TARGET=aarch64-unknown-linux-gnu

build-amd64:
	$(MAKE) TARGET=x86_64-unknown-linux-gnu

# Build the release binary
build:
	@echo "Building $(BINARY_NAME) for $(TARGET) (arch: $(ARCH)) using $(CARGO_CMD)..."
	$(CARGO_CMD) build --target $(TARGET) --features vendored --release

# Patch the binary with correct interpreter (ARM only)
patch: build
ifeq ($(NEEDS_PATCH),true)
	@echo "Patching interpreter path for ARM64..."
	patchelf --set-interpreter $(INTERPRETER) $(RELEASE_DIR)/$(BINARY_NAME)
else
	@echo "No patching needed for x86_64"
endif

# Copy the binary to current directory with arch suffix
copy: patch
	@echo "Copying binary to current directory..."
	cp $(RELEASE_DIR)/$(BINARY_NAME) ./$(BINARY_NAME)-$(ARCH)
	chmod +x ./$(BINARY_NAME)-$(ARCH)
	@echo "✓ Binary ready: ./$(BINARY_NAME)-$(ARCH)"
	@ls -lh ./$(BINARY_NAME)-$(ARCH)

# Build both architectures
all-archs: build-arm64 build-amd64

# Run tests
test:
	@echo "Running tests..."
	cargo test --lib

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -f ./$(BINARY_NAME)-arm64 ./$(BINARY_NAME)-amd64

# Quick rebuild (skips clean)
rebuild: clean all

# Help target
help:
	@echo "Available targets:"
	@echo "  make all              - Build, patch, and copy binary (default: arm64)"
	@echo "  make build            - Build release binary for configured TARGET"
	@echo "  make build-arm64      - Build for ARM64 (aarch64)"
	@echo "  make build-amd64      - Build for x86_64 (amd64)"
	@echo "  make all-archs        - Build both architectures"
	@echo "  make patch            - Patch interpreter path (ARM only)"
	@echo "  make copy             - Copy binary to current directory with arch suffix"
	@echo "  make test             - Run unit tests"
	@echo "  make clean            - Remove build artifacts"
	@echo "  make rebuild          - Clean and rebuild everything"
	@echo "  make help             - Show this help message"
	@echo ""
	@echo "Environment variables:"
	@echo "  TARGET=<target>       - Override build target (e.g., x86_64-unknown-linux-gnu)"

# Makefile for api-service cross-compilation

TARGET = aarch64-unknown-linux-gnu
BINARY_NAME = docker-api-service
RELEASE_DIR = target/$(TARGET)/release
INTERPRETER = /lib/ld-linux-aarch64.so.1

.PHONY: all build patch copy clean test

# Default target: build, patch, and copy
all: build patch copy

# Build the release binary for aarch64
build:
	@echo "Building $(BINARY_NAME) for $(TARGET)..."
	cargo build --target $(TARGET) --release

# Patch the binary with correct interpreter
patch: build
	@echo "Patching interpreter path..."
	patchelf --set-interpreter $(INTERPRETER) $(RELEASE_DIR)/$(BINARY_NAME)

# Copy the patched binary to current directory
copy: patch
	@echo "Copying binary to current directory..."
	cp $(RELEASE_DIR)/$(BINARY_NAME) ./$(BINARY_NAME)
	chmod +x ./$(BINARY_NAME)
	@echo "✓ Binary ready: ./$(BINARY_NAME)"
	@ls -lh ./$(BINARY_NAME)

# Run tests
test:
	@echo "Running tests..."
	cargo test --lib

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -f ./$(BINARY_NAME)

# Quick rebuild (skips clean)
rebuild: all

# Help target
help:
	@echo "Available targets:"
	@echo "  make all      - Build, patch, and copy binary (default)"
	@echo "  make build    - Build release binary for aarch64"
	@echo "  make patch    - Patch interpreter path"
	@echo "  make copy     - Copy binary to current directory"
	@echo "  make test     - Run unit tests"
	@echo "  make clean    - Remove build artifacts"
	@echo "  make rebuild  - Clean and rebuild everything"
	@echo "  make help     - Show this help message"

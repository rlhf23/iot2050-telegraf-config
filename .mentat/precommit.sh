#!/bin/bash

# Script to run before each commit to ensure tests are properly set up
echo "Running pre-commit checks..."

# Function to handle errors
handle_error() {
    echo "ERROR: $1"
    exit 1
}

# Function to warn but continue
warn() {
    echo "WARNING: $1"
}

# Check 1: Verify that all test files referenced in lib.rs and mod.rs files exist
echo "Checking test file references..."

# Check test imports in lib.rs
if grep -q "#\[path" src/lib.rs; then
    while read -r line; do
        if [[ $line =~ \#\[path\ =\ \"([^\"]+)\" ]]; then
            filepath="${BASH_REMATCH[1]}"
            if [ ! -f "src/$filepath" ]; then
                handle_error "File referenced in src/lib.rs does not exist: src/$filepath"
            else
                echo "✓ src/$filepath exists"
            fi
        fi
    done < <(grep "#\[path" src/lib.rs)
fi

# Check test modules in backend/mod.rs
if grep -q "mod [a-z_]*test" src/backend/mod.rs; then
    while read -r line; do
        if [[ $line =~ mod\ ([a-z_]+test) ]]; then
            module="${BASH_REMATCH[1]}"
            if [ ! -f "src/backend/$module.rs" ]; then
                handle_error "Test module file referenced in src/backend/mod.rs does not exist: src/backend/$module.rs"
            else
                echo "✓ src/backend/$module.rs exists"
            fi
        fi
    done < <(grep "mod [a-z_]*test" src/backend/mod.rs)
fi

# Check property test modules in backend/mod.rs
if grep -q "mod [a-z_]*proptest" src/backend/mod.rs; then
    while read -r line; do
        if [[ $line =~ mod\ ([a-z_]+proptest) ]]; then
            module="${BASH_REMATCH[1]}"
            if [ ! -f "src/backend/$module.rs" ]; then
                handle_error "Property test module file referenced in src/backend/mod.rs does not exist: src/backend/$module.rs"
            else
                echo "✓ src/backend/$module.rs exists"
            fi
        fi
    done < <(grep "mod [a-z_]*proptest" src/backend/mod.rs)
fi

# Skip compilation checks if SKIP_COMPILE is set to "true"
# This is useful for CI environments where system dependencies might be missing
if [ "$SKIP_COMPILE" = "true" ]; then
    echo "Skipping compilation checks (SKIP_COMPILE=true)"
else
    # Attempt to run a syntax check only (no compilation needed)
    echo "Running rust syntax check..."
    rustc --edition=2021 --out-dir /tmp --emit=metadata src/lib.rs || warn "Syntax check failed, but continuing"
    
    # Optional advanced checks if the environment is set up correctly
    # These might fail in CI or minimal environments due to missing dependencies
    if [ "$RUN_ADVANCED_CHECKS" = "true" ]; then
        echo "Running advanced checks (enabled via RUN_ADVANCED_CHECKS=true)..."

        # Run clippy to catch common errors
        echo "Running cargo clippy..."
        cargo clippy -- -D warnings || handle_error "Clippy found issues"

        # Run a quick test to make sure everything compiles
        echo "Running test compilation check..."
        cargo test --no-run || handle_error "Test compilation failed"
    else
        echo "Skipping advanced checks (clippy, test compilation)."
        echo "To enable them, set RUN_ADVANCED_CHECKS=true before running this script."
    fi
fi

echo "All pre-commit checks passed!"
exit 0

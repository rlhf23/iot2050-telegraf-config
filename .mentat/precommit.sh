#!/bin/bash

# Script to run before each commit to ensure tests are properly set up
echo "Running pre-commit checks..."

# Check 1: Verify that all test files referenced in lib.rs and mod.rs files exist
echo "Checking test file references..."

# Function to handle errors
handle_error() {
    echo "ERROR: $1"
    exit 1
}

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

# Run clippy to catch common errors
echo "Running cargo clippy..."
cargo clippy -- -D warnings || handle_error "Clippy found issues"

# Run a quick test to make sure everything compiles
echo "Running quick test compilation check..."
cargo test --no-run || handle_error "Test compilation failed"

echo "All pre-commit checks passed!"
exit 0

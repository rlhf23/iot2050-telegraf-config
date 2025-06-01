#!/bin/bash

# Precommit script to run clippy and catch linting issues
echo "Running cargo clippy..."
cargo clippy -- -D warnings

# Check if clippy passed
if [ $? -ne 0 ]; then
    echo "❌ Clippy found issues. Please fix them before committing."
    exit 1
fi

echo "✅ Clippy checks passed!"

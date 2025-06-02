#!/bin/bash

# Precommit script to run clippy and catch serious linting issues
echo "Running cargo clippy..."
cargo clippy

# Check if clippy passed (only fail on errors, not warnings)
if [ $? -ne 0 ]; then
    echo "❌ Clippy found errors. Please fix them before committing."
    exit 1
fi

echo "✅ Clippy checks passed!"

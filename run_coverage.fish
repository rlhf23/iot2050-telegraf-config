#!/usr/bin/env fish
# Script to run test coverage and generate reports using fish shell

# Set environment variables if not already set
set -q DEFAULT_IP; or set -gx DEFAULT_IP 192.168.1.1
set -q DEFAULT_USERNAME; or set -gx DEFAULT_USERNAME user
set -q DEFAULT_PASSWORD; or set -gx DEFAULT_PASSWORD pass
set -q DEFAULT_IOT_USERNAME; or set -gx DEFAULT_IOT_USERNAME iotuser
set -q DEFAULT_IOT_PASSWORD; or set -gx DEFAULT_IOT_PASSWORD iotpass
set -q DEFAULT_IOT_IP; or set -gx DEFAULT_IOT_IP 192.168.1.2:22

echo "Running cargo test to ensure all tests pass..."
cargo test

echo "Running cargo tarpaulin to generate coverage reports..."
cargo tarpaulin --config .tarpaulin.toml

echo "Coverage reports have been generated in the 'coverage' directory."
echo "Open coverage/tarpaulin-report.html in your browser to view the HTML report."

# Print summary of coverage
if test -f coverage/tarpaulin-report.json
    set total_coverage (grep -o '"line_coverage":[0-9.]*' coverage/tarpaulin-report.json | cut -d':' -f2)
    echo "Total line coverage: $total_coverage%"
    
    # Print coverage by file
    echo "Coverage by file:"
    grep -o '"file":"[^"]*","line_coverage":[0-9.]*' coverage/tarpaulin-report.json | \
        sed 's/"file":"\(.*\)","line_coverage":\(.*\)/\1: \2%/'
end

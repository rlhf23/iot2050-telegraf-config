#!/bin/bash
# Script to run test coverage and generate reports

# Set environment variables if not already set
export DEFAULT_IP=${DEFAULT_IP:-192.168.1.1}
export DEFAULT_USERNAME=${DEFAULT_USERNAME:-user}
export DEFAULT_PASSWORD=${DEFAULT_PASSWORD:-pass}
export DEFAULT_IOT_USERNAME=${DEFAULT_IOT_USERNAME:-iotuser}
export DEFAULT_IOT_PASSWORD=${DEFAULT_IOT_PASSWORD:-iotpass}
export DEFAULT_IOT_IP=${DEFAULT_IOT_IP:-192.168.1.2:22}

echo "Running cargo test to ensure all tests pass..."
cargo test

echo "Running cargo tarpaulin to generate coverage reports..."
cargo tarpaulin --config .tarpaulin.toml

echo "Coverage reports have been generated in the 'coverage' directory."
echo "Open coverage/tarpaulin-report.html in your browser to view the HTML report."

# Print summary of coverage
if [ -f coverage/tarpaulin-report.json ]; then
    total_coverage=$(grep -o '"line_coverage":[0-9.]*' coverage/tarpaulin-report.json | cut -d':' -f2)
    echo "Total line coverage: $total_coverage%"
    
    # Print coverage by file
    echo "Coverage by file:"
    grep -o '"file":"[^"]*","line_coverage":[0-9.]*' coverage/tarpaulin-report.json | \
        sed 's/"file":"\(.*\)","line_coverage":\(.*\)/\1: \2%/'
fi

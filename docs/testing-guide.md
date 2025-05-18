# Testing Guide

This document provides guidance on how to use the testing tools available in this project, including property-based testing and coverage reporting.

## Running Tests

### Standard Tests

To run all tests:

```bash
cargo test
```

To run a specific test:

```bash
cargo test test_name
```

### Property-Based Tests

This project uses [proptest](https://docs.rs/proptest) for property-based testing. Property-based tests generate random inputs to test properties of your code, rather than testing specific input/output pairs.

To run just the property tests:

```bash
cargo test -- tests::format_proptest
cargo test -- tests::opcua_poller_proptest
```

### Integration Tests

To run integration tests:

```bash
cargo test --test integration_test
```

## Coverage Reporting

This project uses [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) for test coverage reporting.

### Installation

If you haven't installed tarpaulin yet:

```bash
cargo install cargo-tarpaulin
```

### Running Coverage Analysis

Basic coverage report:

```bash
cargo tarpaulin
```

Using the project's configuration:

```bash
cargo tarpaulin --config .tarpaulin.toml
```

### Coverage Report Formats

The project is configured to generate reports in multiple formats:

- HTML: Open `tarpaulin-report.html` in a browser
- JSON: For programmatic analysis
- LCOV: For integration with tools like codecov.io

## Writing New Property Tests

When adding new functionality, consider adding property tests to test the general behavior rather than just specific cases.

Example property test pattern:

```rust
proptest! {
    #[test]
    fn test_property_name(
        // Generate random test inputs within reasonable ranges
        input1 in 1u32..1000,
        input2 in "a[a-z]{1,10}",
    ) {
        // Test that a property holds for all generated inputs
        let result = my_function(input1, &input2);
        
        // Assert properties that should always be true
        prop_assert!(result.is_valid());
        
        // For reversible operations, test round-trip conversions
        let original = reverse_function(result);
        prop_assert_eq!(original.input1, input1);
        prop_assert_eq!(original.input2, input2);
    }
}
```

## CI Integration

For continuous integration, add the following to your workflow:

```yaml
- name: Install tarpaulin
  run: cargo install cargo-tarpaulin

- name: Run tests with coverage
  run: cargo tarpaulin --config .tarpaulin.toml

- name: Upload coverage reports
  # Add your preferred coverage reporting service integration here
```

# Claude-Usage Test Suite

This directory contains comprehensive tests for the claude-usage integration with claude-keeper.

## Test Files Overview

### Core Integration Tests

#### `integration_e2e_test.rs`
End-to-end integration tests using realistic Claude Desktop data patterns:
- **test_e2e_basic_analysis**: Basic analysis workflow with realistic JSONL data
- **test_e2e_with_malformed_data**: Handles malformed JSON lines gracefully  
- **test_e2e_vm_exclusion**: Tests VM exclusion functionality
- **test_e2e_keeper_schema_resilience**: Schema variation handling (keeper feature only)
- **test_e2e_date_filtering**: Date range filtering functionality
- **test_e2e_deduplication**: Duplicate entry removal

#### `performance_comparison_test.rs`
Performance validation and comparison tests:
- **test_performance_comparison**: Compares legacy vs keeper parser performance
- **test_memory_efficiency**: Large file processing without OOM
- **test_unified_parser_performance_baseline**: Establishes performance baseline
- **test_malformed_data_performance**: Performance with malformed data
- **test_concurrent_parsing_performance**: Multi-threaded parsing efficiency
- **test_keeper_specific_performance**: Keeper-specific optimizations (feature only)
- **test_empty_file_performance**: Edge case performance

#### `parser_wrapper_test.rs`
Tests for the unified parser wrapper:
- Basic functionality tests
- Feature flag switching validation
- Error handling verification
- Interface consistency checks

#### `keeper_integration_test.rs`
Keeper-specific integration tests:
- Schema resilience validation
- Error recovery testing
- Field mapping verification

#### `test_suite_validation.rs`
Meta-tests that validate the test suite itself:
- Import validation
- Feature flag compilation
- Test utility functionality
- Realistic test data patterns

### Test Utilities

#### `common/mod.rs`
Shared test utilities:
- `create_test_jsonl()`: Helper for creating test JSONL files

## Running Tests

### Basic Test Execution

```bash
# Run all tests (legacy parser)
cargo test

# Run all tests (with keeper integration)
cargo test --features keeper-integration

# Run specific test file
cargo test --test integration_e2e_test
cargo test --test performance_comparison_test

# Run with verbose output
cargo test -- --nocapture
```

### Test Categories

```bash
# End-to-end integration tests
cargo test --test integration_e2e_test
cargo test --test integration_e2e_test --features keeper-integration

# Performance tests
cargo test --test performance_comparison_test
cargo test --test performance_comparison_test --features keeper-integration

# Parser wrapper tests
cargo test --test parser_wrapper_test
cargo test --test parser_wrapper_test --features keeper-integration

# Meta-validation tests
cargo test --test test_suite_validation
```

## Real-World Testing

### Automated Script
```bash
# Run comprehensive real-world test
./scripts/test_real_data.sh
```

This script:
1. Tests both legacy and keeper parsers
2. Validates error handling with malformed data
3. Performs timing comparisons
4. Creates mock data if no real Claude data exists

### Manual Testing
```bash
# Test with actual Claude Desktop data
cargo run -- daily --limit 10
cargo run --features keeper-integration -- daily --limit 10

# Compare outputs
cargo run -- daily --json > legacy_output.json
cargo run --features keeper-integration -- daily --json > keeper_output.json
diff legacy_output.json keeper_output.json
```

## Test Data Patterns

The tests use realistic patterns based on actual Claude Desktop JSONL files:

### Pattern 1: Full Featured Entry
```json
{"timestamp":"2025-01-15T10:30:00Z","message":{"id":"msg_1","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":5,"cache_read_input_tokens":10}},"costUSD":0.005,"requestId":"req_1"}
```

### Pattern 2: Alternative Field Names
```json
{"timestamp":"2025-01-15T10:31:00Z","message":{"id":"msg_2","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":150,"output_tokens":250}},"cost_usd":0.004,"request_id":"req_2"}
```

### Pattern 3: Minimal Entry
```json
{"timestamp":"2025-01-15T10:32:00Z","message":{"id":"msg_3","model":"claude-3-5-sonnet-20241022"},"requestId":"req_3"}
```

### Pattern 4: Schema Evolution
```json
{"timestamp":"2025-01-15T10:33:00Z","message":{"id":"msg_4","model":"claude-3-5-sonnet-20241022","usage":{"inputTokens":200,"outputTokens":300}},"cost":0.005,"req_id":"req_4","newField":"future_value"}
```

## Mock Directory Structure

Tests create realistic Claude Desktop directory structures:

```
temp_dir/
└── .claude/
    ├── projects/
    │   └── test_project_main/
    │       └── conversation_test.jsonl
    └── vms/
        └── test_vm/
            └── projects/
                └── test_project_vm/
                    └── conversation_vm.jsonl
```

## Performance Expectations

Based on the performance tests:

- **10,000 lines**: < 1 second
- **100,000 lines**: Should not OOM  
- **Malformed data**: < 10% performance impact
- **Concurrent parsing**: Scales with CPU cores
- **Empty files**: < 10ms

## Feature Flag Testing

Tests are designed to work with both feature flag states:

```bash
# Without keeper-integration (legacy mode)
cargo test
# Uses FileParser directly
# May fail on malformed data
# Existing behavior preserved

# With keeper-integration (enhanced mode)  
cargo test --features keeper-integration
# Uses UnifiedParser -> KeeperIntegration
# Graceful malformed data handling
# Schema evolution support
```

## Debugging Tests

### Verbose Output
```bash
cargo test -- --nocapture
```

### Single Test
```bash
cargo test test_e2e_basic_analysis -- --nocapture
```

### Environment Variables
```bash
RUST_LOG=debug cargo test
```

### Test Data Inspection
```bash
# Create test data and inspect it
cargo test test_realistic_jsonl_patterns -- --nocapture
```

## CI/CD Integration

Tests are structured for automated CI/CD:

```yaml
# Example GitHub Actions
- name: Test Legacy Mode
  run: cargo test

- name: Test Keeper Integration  
  run: cargo test --features keeper-integration

- name: Performance Tests
  run: cargo test --test performance_comparison_test

- name: Real Data Test
  run: ./scripts/test_real_data.sh
```

## Troubleshooting

### Common Issues

1. **Missing claude-keeper dependency**
   ```bash
   # Ensure claude-keeper-v3 is available
   ls ../claude-keeper
   ```

2. **Feature flag confusion**
   ```bash
   # Clear and rebuild
   cargo clean
   cargo build --features keeper-integration
   ```

3. **Test data permissions**
   ```bash
   # Ensure test script is executable
   chmod +x scripts/test_real_data.sh
   ```

4. **Tempfile cleanup**
   ```bash
   # Tests should auto-cleanup, but check /tmp if issues persist
   ls /tmp/rust_*
   ```

### Performance Issues

1. **Slow tests**: Check if running in debug mode
   ```bash
   cargo test --release
   ```

2. **Memory usage**: Run specific memory tests
   ```bash
   cargo test test_memory_efficiency
   ```

3. **Concurrency**: Limit parallel tests
   ```bash
   cargo test -- --test-threads=1
   ```

## Contributing

When adding new tests:

1. Follow existing patterns in test files
2. Use realistic Claude Desktop data patterns  
3. Test both feature flag states when relevant
4. Add performance expectations
5. Include error handling validation
6. Update this README with new test descriptions
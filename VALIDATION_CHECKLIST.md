# Claude-Usage Integration Validation Checklist

## Pre-Release Testing Checklist

### ✅ Core Functionality
- [ ] Daily usage analysis works with both parsers
- [ ] Monthly aggregation works with both parsers
- [ ] Live monitoring displays correctly
- [ ] JSON output format is consistent
- [ ] Date filtering works correctly
- [ ] VM exclusion flag works

### ✅ Parser Compatibility
- [ ] Legacy parser (no features) compiles and runs
- [ ] Keeper integration (--features keeper-integration) compiles and runs
- [ ] UnifiedParser switches correctly based on feature flag
- [ ] No performance regression in legacy mode

### ✅ Error Handling
- [ ] Malformed JSON lines are handled gracefully
- [ ] Missing files don't crash the application
- [ ] Empty directories are handled correctly
- [ ] Large files (>100MB) process without OOM
- [ ] Duplicate entries are deduplicated correctly

### ✅ Real-World Testing
- [ ] Works with actual Claude Desktop files from ~/.claude/
- [ ] Handles multiple Claude instances (main + VMs)
- [ ] Processes historical data correctly
- [ ] Token counts match Claude Desktop UI

### ✅ Performance Metrics
- [ ] Parsing 10,000 lines takes < 1 second
- [ ] Memory usage stays under 500MB for typical workloads
- [ ] CPU usage is reasonable (not 100% sustained)
- [ ] Parallel processing works correctly

### ✅ Integration Points
- [ ] ClaudeUsageAnalyzer uses UnifiedParser
- [ ] DeduplicationEngine accepts parser parameter
- [ ] LiveMonitor has parser fields configured
- [ ] File discovery still uses FileParser
- [ ] Timestamp parsing works correctly

## Test Commands

```bash
# Basic functionality test
cargo test

# Integration tests
cargo test --features keeper-integration

# End-to-end tests
cargo test --test integration_e2e_test
cargo test --test integration_e2e_test --features keeper-integration

# Performance tests
cargo bench --features keeper-integration

# Real data test
./scripts/test_real_data.sh

# Manual verification
cargo run -- daily --limit 5
cargo run --features keeper-integration -- daily --limit 5
```

## Release Criteria

All items in this checklist must be verified before release:

1. ✅ All tests pass in CI/CD
2. ✅ No performance regression
3. ✅ Real Claude Desktop data processes correctly
4. ✅ Documentation is updated
5. ✅ Feature flag works as expected

## Automated Test Coverage

### End-to-End Integration Tests (`integration_e2e_test.rs`)
- ✅ Basic analysis with realistic JSONL data
- ✅ Malformed data handling
- ✅ VM exclusion functionality
- ✅ Keeper schema resilience (feature-gated)
- ✅ Date filtering
- ✅ Deduplication

### Performance Comparison Tests (`performance_comparison_test.rs`)
- ✅ Large file processing (10K+ lines)
- ✅ Memory efficiency validation
- ✅ Performance baseline establishment
- ✅ Malformed data performance impact
- ✅ Concurrent parsing efficiency
- ✅ Keeper-specific performance (feature-gated)
- ✅ Empty file handling

### Real-World Data Script (`scripts/test_real_data.sh`)
- ✅ Tests both legacy and keeper parsers
- ✅ Error handling validation
- ✅ Performance comparison timing
- ✅ Mock data creation when no real data exists

## Manual Testing Scenarios

### 1. Schema Evolution Testing
Test with Claude Desktop files containing:
- Old field names (camelCase)
- New field names (snake_case)
- Missing optional fields
- Extra unknown fields
- Mixed naming conventions within same file

### 2. Large Dataset Testing
- Files > 100MB
- Sessions with > 10,000 messages
- Multiple concurrent sessions
- Historical data spanning months

### 3. Edge Cases
- Completely empty files
- Files with only malformed data
- Files with mixed line endings
- Files with Unicode characters
- Files with very long messages

### 4. Performance Regression Testing
- Compare processing times between releases
- Memory usage profiling
- CPU utilization monitoring
- Concurrent access scenarios

## Verification Steps

1. **Build Verification**
   ```bash
   # Legacy build
   cargo build --release
   
   # Keeper integration build  
   cargo build --release --features keeper-integration
   ```

2. **Unit Test Verification**
   ```bash
   # All tests without features
   cargo test
   
   # All tests with keeper integration
   cargo test --features keeper-integration
   ```

3. **Integration Test Verification**
   ```bash
   # E2E tests
   cargo test --test integration_e2e_test
   cargo test --test integration_e2e_test --features keeper-integration
   
   # Performance tests
   cargo test --test performance_comparison_test
   cargo test --test performance_comparison_test --features keeper-integration
   ```

4. **Real Data Verification**
   ```bash
   # Automated real data test
   ./scripts/test_real_data.sh
   
   # Manual verification with actual Claude data
   cargo run -- daily
   cargo run --features keeper-integration -- daily
   ```

5. **Performance Verification**
   ```bash
   # Benchmark comparison
   cargo bench
   cargo bench --features keeper-integration
   ```

## Success Criteria

- [ ] All automated tests pass
- [ ] No performance regression > 10%
- [ ] Memory usage remains stable
- [ ] Real Claude Desktop data processes without errors
- [ ] Error handling is graceful and informative
- [ ] Feature flag correctly switches parser implementations
- [ ] Documentation matches actual behavior

## Known Limitations

- Keeper integration requires claude-keeper-v3 dependency
- Some malformed JSON patterns may still cause issues
- Performance may vary significantly based on data patterns
- VM detection relies on file path patterns

## Post-Release Monitoring

- Monitor user reports of parsing failures
- Track performance metrics in production
- Watch for new Claude Desktop schema changes
- Validate deduplication effectiveness
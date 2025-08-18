# Claude-Usage Parser Benchmarks

This directory contains performance benchmarks comparing the legacy parser with the new claude-keeper integration.

## Running Benchmarks

### Quick Comparison
```bash
./scripts/compare_performance.sh
```

### Individual Benchmarks

**Legacy Parser Only:**
```bash
cargo bench --bench parser_benchmark
```

**With Keeper Integration:**
```bash
cargo bench --bench parser_benchmark --features keeper-integration
```

### Specific Benchmark Groups

Run specific benchmark groups:
```bash
cargo bench --bench parser_benchmark legacy_parser
cargo bench --bench parser_benchmark keeper_parser
cargo bench --bench parser_benchmark error_handling
cargo bench --bench parser_benchmark memory_usage
```

## Benchmark Scenarios

1. **Scale Testing**: 10, 100, 1000, 10000 lines
2. **Error Handling**: 10% malformed lines
3. **Memory Usage**: 50,000 line files
4. **Unified Parser**: Feature flag agnostic testing

## Performance Metrics

The benchmarks measure:
- **Throughput**: Lines parsed per second
- **Error Recovery**: Performance with malformed data
- **Memory Efficiency**: Large file handling
- **Feature Flag Overhead**: Cost of abstraction layer

## Interpreting Results

Results are saved in `target/criterion/`:
- HTML reports: `target/criterion/report/index.html`
- Raw data: `target/criterion/*/base/estimates.json`

### Expected Performance Characteristics

| Parser Type | Strengths | Trade-offs |
|------------|-----------|------------|
| Legacy | Simple, direct parsing | No error recovery |
| Keeper | Schema resilience, error recovery | Slight overhead for FlexObject |
| Unified | Consistent interface | Minimal abstraction cost |

## Optimization Opportunities

Based on benchmark results, consider:
1. Batch processing optimizations
2. Memory pooling for FlexObject creation
3. Parallel parsing with rayon
4. Streaming parser for very large files
# Claude-Usage Parser Performance Expectations

This document outlines the expected performance characteristics of the different parsing implementations.

## Parser Implementations

### Legacy Parser (`FileParser`)
- **Architecture**: Direct JSON deserialization with serde
- **Error Handling**: Fails on first malformed line
- **Memory Usage**: Loads entire file into memory
- **Performance**: Fastest for well-formed data

**Expected Characteristics:**
- **Throughput**: 50,000-100,000 lines/second for well-formed JSONL
- **Memory**: ~2x file size (original + parsed structures)
- **Error Recovery**: None - stops on first error
- **Latency**: Low variance, predictable timing

### Keeper Parser (`KeeperIntegration`) 
- **Architecture**: FlexObject-based parsing with schema adaptation
- **Error Handling**: Continues parsing after errors, logs issues
- **Memory Usage**: Efficient with lazy evaluation
- **Performance**: Slightly slower but more resilient

**Expected Characteristics:**
- **Throughput**: 30,000-70,000 lines/second (60-80% of legacy)
- **Memory**: ~1.5x file size (more efficient object storage)
- **Error Recovery**: Excellent - processes valid lines, skips broken ones
- **Latency**: Higher variance due to error handling paths

### Unified Parser (`UnifiedParser`)
- **Architecture**: Feature-flag based wrapper
- **Error Handling**: Depends on underlying implementation
- **Memory Usage**: Same as underlying parser
- **Performance**: Minimal overhead from abstraction

**Expected Characteristics:**
- **Throughput**: 95-99% of underlying parser performance
- **Memory**: Same as underlying parser + minimal wrapper overhead
- **Error Recovery**: Inherits from chosen implementation
- **Latency**: <1% additional overhead from abstraction layer

## Benchmark Scenarios

### Scale Testing (10, 100, 1000, 10000 lines)
- **Legacy**: Linear performance scaling
- **Keeper**: Slight performance degradation at larger scales due to FlexObject overhead
- **Unified**: Matches chosen implementation

### Error Handling (10% malformed lines)
- **Legacy**: Significant performance drop, early termination
- **Keeper**: Graceful degradation, continues processing
- **Unified**: Depends on feature flag configuration

### Memory Usage (50,000 lines)
- **Legacy**: Peak memory usage ~2x file size
- **Keeper**: Peak memory usage ~1.5x file size
- **Unified**: Matches chosen implementation

## Performance Optimization Opportunities

### Short-term Improvements
1. **Batch Processing**: Process files in chunks to reduce memory pressure
2. **Streaming Parser**: Read/parse incrementally for very large files
3. **Memory Pooling**: Reuse FlexObject instances to reduce allocation overhead

### Long-term Optimizations
1. **Parallel Processing**: Use rayon for multi-core parsing
2. **Custom Deserializer**: Optimize for Claude-specific JSON structure
3. **Compression-Aware**: Handle compressed JSONL files directly

## Benchmark Interpretation Guidelines

### Performance Comparison
- **Legacy vs Keeper**: Expect 20-40% overhead for keeper integration
- **Error-free vs With errors**: Legacy shows 50%+ degradation, Keeper shows 10-20%
- **Memory efficiency**: Keeper should use 20-30% less memory than legacy

### When to Use Which Parser
- **Legacy**: Use for guaranteed well-formed data, maximum performance
- **Keeper**: Use for production environments with potential data quality issues
- **Unified**: Use when you want runtime switching capability

### Red Flags in Results
- **Legacy parser > 200ms for 1000 lines**: Environment or IO issues
- **Keeper parser > 300ms for 1000 lines**: FlexObject performance regression  
- **Memory usage > 3x file size**: Memory leak or inefficient parsing
- **Error handling causing > 50% keeper degradation**: Schema adaptation issues

## Continuous Performance Monitoring

### Automated Benchmarks
Run benchmarks on every significant change:
```bash
./scripts/compare_performance.sh
```

### Performance Regression Detection
- Monitor for >20% performance degradation
- Check memory usage doesn't exceed 2.5x file size
- Ensure error handling doesn't cause >30% slowdown

### Optimization Verification
After implementing optimizations, verify:
- Throughput improvements match expectations
- Memory usage reductions are real
- Error handling robustness isn't compromised
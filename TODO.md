# Claude Usage Max - TODO

## Rust Port Investigation

### Performance Goals
- **Target speedup**: 3-5x faster overall performance
- **Primary benefits**: 
  - JSON parsing: 3-5x faster than Node.js
  - Memory efficiency: No GC pauses, lower memory usage
  - Parallel I/O: True parallelism across all VMs
  - Live monitoring: Much more responsive, less flickering

### Development Estimate
- **Timeline**: 3-4 weeks for complete port
- **Code size**: 2,500-4,000 lines (vs current ~672 lines + library)
- **Key libraries**: serde, tokio, clap, reqwest, crossterm, chrono

### Port Breakdown
- [ ] **Week 1-2**: Project setup, data structures, JSON parsing
- [ ] **Week 2-3**: File I/O, aggregation logic, CLI basics  
- [ ] **Week 3-4**: Live monitoring UI, HTTP client, polish

### Current Performance vs Expected
- **Current**: ~2-3 seconds full aggregation
- **Rust target**: ~0.5-1 second full aggregation
- **Live monitoring**: Could refresh much faster without flickering

## Current Issues to Fix First

### Live Monitoring Problems
- [ ] Fix flickering screen (too aggressive clearing)
- [ ] Fix 0 tokens display despite active usage
- [ ] Investigate blocks command showing NaN costs
- [ ] Improve active block detection across multiple VMs
- [ ] Reduce refresh rate for better UX

### Technical Debt
- [ ] Debug why `loadSessionBlockData` shows 0 tokens
- [ ] Fix multiple LiteLLM API calls (should share pricing data)
- [ ] Optimize VM path discovery caching
- [ ] Better error handling for corrupted JSONL files

## Decision Points
- **Rust port priority**: High performance gain vs significant development time
- **Current tool viability**: Already pretty snappy, issues are fixable
- **Learning curve**: Would require Rust expertise or learning time
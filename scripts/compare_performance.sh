#!/bin/bash
# Performance comparison script for legacy vs keeper parsing

echo "=== Claude-Usage Parser Performance Comparison ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building without keeper integration...${NC}"
cargo build --release --benches

echo -e "${YELLOW}Running legacy parser benchmarks...${NC}"
cargo bench --bench parser_benchmark -- --save-baseline legacy 2>/dev/null

echo ""
echo -e "${YELLOW}Building with keeper integration...${NC}"
cargo build --release --features keeper-integration --benches

echo -e "${YELLOW}Running keeper parser benchmarks...${NC}"
cargo bench --bench parser_benchmark --features keeper-integration -- --baseline legacy 2>/dev/null

echo ""
echo -e "${GREEN}Performance comparison complete!${NC}"
echo "Check target/criterion/report/index.html for detailed results"

# Generate summary
echo ""
echo "=== Quick Summary ==="
if [ -f "target/criterion/unified_parser/base/estimates.json" ]; then
    echo "Unified parser performance metrics available in target/criterion/"
fi
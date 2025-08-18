#!/bin/bash
# Test script to verify benchmark setup

echo "=== Testing Claude-Usage Benchmark Setup ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Checking benchmark dependencies...${NC}"

# Check if criterion is in Cargo.toml
if grep -q "criterion" Cargo.toml; then
    echo -e "${GREEN}✓${NC} Criterion dependency found"
else
    echo -e "${RED}✗${NC} Criterion dependency missing"
    exit 1
fi

# Check if bench configuration exists
if grep -q "\\[\\[bench\\]\\]" Cargo.toml; then
    echo -e "${GREEN}✓${NC} Benchmark configuration found"
else
    echo -e "${RED}✗${NC} Benchmark configuration missing"
    exit 1
fi

# Check benchmark files
echo ""
echo -e "${YELLOW}Checking benchmark files...${NC}"

if [ -f "benches/main.rs" ]; then
    echo -e "${GREEN}✓${NC} Main benchmark file exists"
else
    echo -e "${RED}✗${NC} Main benchmark file missing"
    exit 1
fi

if [ -f "benches/parser_benchmark.rs" ]; then
    echo -e "${GREEN}✓${NC} Parser benchmark file exists"
else
    echo -e "${YELLOW}!${NC} Parser benchmark file not found (optional)"
fi

# Check required source files
echo ""
echo -e "${YELLOW}Checking source dependencies...${NC}"

REQUIRED_FILES=(
    "src/parser.rs"
    "src/parser_wrapper.rs" 
    "src/keeper_integration.rs"
    "src/models.rs"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $file exists"
    else
        echo -e "${RED}✗${NC} $file missing"
    fi
done

# Check scripts
echo ""
echo -e "${YELLOW}Checking benchmark scripts...${NC}"

if [ -f "scripts/compare_performance.sh" ] && [ -x "scripts/compare_performance.sh" ]; then
    echo -e "${GREEN}✓${NC} Performance comparison script ready"
else
    echo -e "${RED}✗${NC} Performance comparison script missing or not executable"
fi

if [ -f "scripts/analyze_benchmarks.sh" ] && [ -x "scripts/analyze_benchmarks.sh" ]; then
    echo -e "${GREEN}✓${NC} Benchmark analysis script ready"
else
    echo -e "${RED}✗${NC} Benchmark analysis script missing or not executable"
fi

echo ""
echo -e "${GREEN}Benchmark setup verification complete!${NC}"
echo ""
echo "To run benchmarks:"
echo "  1. Legacy only:     cargo bench --bench main"
echo "  2. With keeper:     cargo bench --bench main --features keeper-integration"
echo "  3. Compare both:    ./scripts/compare_performance.sh"
echo "  4. Analyze results: ./scripts/analyze_benchmarks.sh"
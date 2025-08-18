#!/bin/bash
# Analysis script for benchmark results

echo "=== Claude-Usage Benchmark Analysis ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

CRITERION_DIR="target/criterion"

if [ ! -d "$CRITERION_DIR" ]; then
    echo -e "${RED}Error: No benchmark results found. Run benchmarks first:${NC}"
    echo "  cargo bench --bench main"
    echo "  cargo bench --bench main --features keeper-integration"
    exit 1
fi

echo -e "${BLUE}Available benchmark groups:${NC}"
for group in $(ls $CRITERION_DIR); do
    if [ -d "$CRITERION_DIR/$group" ]; then
        echo "  - $group"
    fi
done

echo ""
echo -e "${YELLOW}Performance Comparison Summary:${NC}"
echo ""

# Function to extract time estimate from estimates.json
extract_estimate() {
    local file="$1"
    if [ -f "$file" ]; then
        # Extract the mean estimate in nanoseconds and convert to milliseconds
        python3 -c "
import json
import sys
try:
    with open('$file', 'r') as f:
        data = json.load(f)
        mean_ns = data['mean']['point_estimate']
        mean_ms = mean_ns / 1_000_000
        print(f'{mean_ms:.2f} ms')
except:
    print('N/A')
" 2>/dev/null || echo "N/A"
    else
        echo "N/A"
    fi
}

# Compare parser performance if both exist
echo "Parser Performance Comparison:"
echo "------------------------------"

if [ -d "$CRITERION_DIR/legacy_parser_scaling" ]; then
    echo "Legacy Parser (1000 lines):"
    for size_dir in $(ls "$CRITERION_DIR/legacy_parser_scaling"); do
        if [[ "$size_dir" == "1000" ]]; then
            estimate_file="$CRITERION_DIR/legacy_parser_scaling/$size_dir/base/estimates.json"
            time=$(extract_estimate "$estimate_file")
            echo "  $size_dir lines: $time"
        fi
    done
fi

if [ -d "$CRITERION_DIR/keeper_parser_scaling" ]; then
    echo "Keeper Parser (1000 lines):"
    for size_dir in $(ls "$CRITERION_DIR/keeper_parser_scaling"); do
        if [[ "$size_dir" == "1000" ]]; then
            estimate_file="$CRITERION_DIR/keeper_parser_scaling/$size_dir/base/estimates.json"
            time=$(extract_estimate "$estimate_file")
            echo "  $size_dir lines: $time"
        fi
    done
fi

echo ""
echo "Error Handling Performance:"
echo "---------------------------"
if [ -f "$CRITERION_DIR/error_handling_performance/legacy_with_errors/base/estimates.json" ]; then
    time=$(extract_estimate "$CRITERION_DIR/error_handling_performance/legacy_with_errors/base/estimates.json")
    echo "Legacy with errors: $time"
fi

if [ -f "$CRITERION_DIR/error_handling_performance/keeper_with_errors/base/estimates.json" ]; then
    time=$(extract_estimate "$CRITERION_DIR/error_handling_performance/keeper_with_errors/base/estimates.json")
    echo "Keeper with errors: $time"
fi

echo ""
echo "Memory Usage (50k lines):"
echo "-------------------------"
if [ -f "$CRITERION_DIR/memory_usage_performance/legacy_large_file/base/estimates.json" ]; then
    time=$(extract_estimate "$CRITERION_DIR/memory_usage_performance/legacy_large_file/base/estimates.json")
    echo "Legacy large file: $time"
fi

if [ -f "$CRITERION_DIR/memory_usage_performance/keeper_large_file/base/estimates.json" ]; then
    time=$(extract_estimate "$CRITERION_DIR/memory_usage_performance/keeper_large_file/base/estimates.json")
    echo "Keeper large file: $time"
fi

echo ""
echo -e "${GREEN}For detailed analysis, open:${NC}"
echo "  file://$PWD/$CRITERION_DIR/report/index.html"
echo ""
echo -e "${BLUE}Raw data available in:${NC}"
echo "  $CRITERION_DIR/*/base/estimates.json"
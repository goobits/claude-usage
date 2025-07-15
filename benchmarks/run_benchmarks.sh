#!/bin/bash

set -e

echo "🏁 Claude Usage Performance Benchmarks"
echo "====================================="

# Create sample data if it doesn't exist
if [ ! -f "benchmarks/sample_data/large_conversation.jsonl" ]; then
    echo "📊 Creating sample data..."
    mkdir -p benchmarks/sample_data
    
    # Generate large JSONL file for benchmarking
    python3 -c "
import json
import sys
import os

os.makedirs('benchmarks/sample_data', exist_ok=True)

with open('benchmarks/sample_data/large_conversation.jsonl', 'w') as f:
    for i in range(10000):
        entry = {
            'timestamp': f'2024-01-01T12:{i%60:02d}:{i%60:02d}Z',
            'message': {
                'id': f'msg{i}',
                'model': 'claude-sonnet-4-20250514',
                'usage': {
                    'input_tokens': 100 + (i % 100),
                    'output_tokens': 50 + (i % 50),
                    'cache_creation_input_tokens': 0,
                    'cache_read_input_tokens': 0
                }
            },
            'requestId': f'req{i}',
            'costUSD': (i * 0.001) % 1.0
        }
        f.write(json.dumps(entry) + '\n')
"
fi

echo "⏱️  Running Python benchmark..."
PYTHON_START=$(date +%s%N)
python3 claude_usage.py daily --json > benchmarks/results/python_output.json 2>/dev/null || echo "No data found"
PYTHON_END=$(date +%s%N)
PYTHON_TIME=$((($PYTHON_END - $PYTHON_START) / 1000000))

echo "⏱️  Running Rust benchmark..."
cd rust
cargo build --release
RUST_START=$(date +%s%N)
./target/release/claude-usage daily --json > ../benchmarks/results/rust_output.json 2>/dev/null || echo "No data found"
RUST_END=$(date +%s%N)
RUST_TIME=$((($RUST_END - $RUST_START) / 1000000))
cd ..

echo "📊 Benchmark Results"
echo "==================="
echo "Python version: ${PYTHON_TIME}ms"
echo "Rust version:   ${RUST_TIME}ms"

if [ $RUST_TIME -gt 0 ]; then
    SPEEDUP=$(echo "scale=2; $PYTHON_TIME / $RUST_TIME" | bc -l)
    echo "Speedup:        ${SPEEDUP}x faster"
fi

echo "📁 Results saved to benchmarks/results/"

# Run Rust-specific benchmarks
echo "🦀 Running detailed Rust benchmarks..."
cd rust
cargo bench -- --output-format pretty
cd ..

echo "✅ Benchmarks complete!"
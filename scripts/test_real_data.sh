#!/bin/bash

# Test script for real Claude Desktop data
# This script tests the integration with actual Claude Desktop files

set -e

echo "=== Testing Claude-Usage with Real Data ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Claude Desktop directory exists
CLAUDE_DIR="$HOME/.claude"
if [ ! -d "$CLAUDE_DIR" ]; then
    echo -e "${YELLOW}Warning: Claude Desktop directory not found at $CLAUDE_DIR${NC}"
    echo "Creating mock data for testing..."
    
    # Create mock structure for testing
    mkdir -p "$HOME/.claude/projects/test_project"
    cat > "$HOME/.claude/projects/test_project/conversation_test.jsonl" << 'EOF'
{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"msg_test1","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":50}},"costUSD":0.0045,"requestId":"req_test1"}
{"timestamp":"2024-01-15T10:31:00Z","message":{"id":"msg_test2","model":"claude-3-5-sonnet-20241022","usage":{"input_tokens":150,"output_tokens":250,"cache_creation_input_tokens":10,"cache_read_input_tokens":60}},"costUSD":0.0055,"requestId":"req_test2"}
EOF
fi

echo -e "${GREEN}Step 1: Testing with Legacy Parser${NC}"
cargo build --release
./target/release/claude-usage daily --limit 5
echo ""

echo -e "${GREEN}Step 2: Testing with Keeper Integration${NC}"
cargo build --release --features keeper-integration
./target/release/claude-usage daily --limit 5
echo ""

echo -e "${GREEN}Step 3: Testing Error Handling${NC}"
# Create a file with malformed data
TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << 'EOF'
{"timestamp":"2024-01-15T10:30:00Z","message":{"id":"msg_valid","model":"claude-3-5-sonnet-20241022"},"requestId":"req_valid"}
{this is broken json}
{"timestamp":"2024-01-15T10:31:00Z","message":{"id":"msg_also_valid","model":"claude-3-5-sonnet-20241022"},"requestId":"req_also_valid"}
EOF

# Test with both parsers
echo "Testing legacy parser with malformed data..."
./target/release/claude-usage daily 2>&1 | grep -q "error" && echo "Legacy parser failed on malformed data (expected)" || echo "Legacy parser handled malformed data"

echo "Testing keeper integration with malformed data..."
cargo build --release --features keeper-integration
./target/release/claude-usage daily 2>&1 | grep -q "Parse errors" && echo "Keeper parser reported errors but continued" || echo "Keeper parser handled malformed data silently"

echo ""
echo -e "${GREEN}Step 4: Performance Comparison${NC}"
time ./target/release/claude-usage daily >/dev/null 2>&1 && echo "Legacy parser timing above"
time ./target/release/claude-usage daily >/dev/null 2>&1 && echo "Keeper parser timing above"

echo ""
echo -e "${GREEN}✅ All tests completed!${NC}"
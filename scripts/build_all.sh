#!/bin/bash

set -e

echo "🔨 Building Claude Usage - All Implementations"
echo "============================================="

# Build Python version
echo "📦 Building Python version..."
python3 -m pip install --upgrade pip
pip install -e .
echo "✅ Python version built successfully"

# Build Rust version
echo "🦀 Building Rust version..."
cd rust
cargo build --release
echo "✅ Rust version built successfully"

# Copy binaries to a common location
echo "📁 Setting up binaries..."
mkdir -p ../bin
cp target/release/claude-usage ../bin/claude-usage-rust
cd ..

# Create wrapper scripts
cat > bin/claude-usage-python << 'EOF'
#!/bin/bash
python3 claude_usage.py "$@"
EOF

chmod +x bin/claude-usage-python bin/claude-usage-rust

echo "🎉 Build complete!"
echo "   Python version: ./bin/claude-usage-python"
echo "   Rust version:   ./bin/claude-usage-rust"
echo "   Or use:         python3 claude_usage.py"
echo "   Or use:         cd rust && cargo run --release --"
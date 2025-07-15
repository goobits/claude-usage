#!/bin/bash

set -e

echo "🦀 Installing Claude Usage - Rust Implementation"
echo "==============================================="

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Install from current directory
echo "📦 Installing from source..."
cd rust
cargo install --path .

echo "✅ Installation complete!"
echo "   You can now use: claude-usage"
echo "   Or run directly: cargo run --release --"

# Test the installation
echo "🧪 Testing installation..."
if command -v claude-usage &> /dev/null; then
    echo "✅ claude-usage command is available"
    claude-usage --help
else
    echo "⚠️  claude-usage command not found in PATH"
    echo "   Make sure ~/.cargo/bin is in your PATH"
fi
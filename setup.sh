#!/bin/bash

# Claude Usage - Rust Setup Script
# This script builds and installs the Rust version of claude-usage

set -e  # Exit on any error

echo "🚀 Setting up Claude Usage (Rust version)..."
echo

# Check if we're in the right directory
if [ ! -d "rust" ]; then
    echo "❌ Error: rust/ directory not found"
    echo "Please run this script from the project root directory"
    exit 1
fi

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo "📦 Rust not found. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
    echo "✅ Rust installed successfully"
else
    echo "✅ Rust is already installed ($(rustc --version))"
fi

# Ensure cargo is available
if ! command -v cargo &> /dev/null; then
    echo "📦 Loading Rust environment..."
    source $HOME/.cargo/env
fi

# Create local bin directory if it doesn't exist
mkdir -p ~/.local/bin

echo "🔨 Building Rust version..."
cd rust
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful"
else
    echo "❌ Build failed"
    exit 1
fi

echo "📦 Installing to ~/.local/bin/claude-usage..."
cp target/release/claude-usage ~/.local/bin/claude-usage
chmod +x ~/.local/bin/claude-usage

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️  ~/.local/bin is not in your PATH"
    echo "Add this to your shell profile (e.g., ~/.bashrc, ~/.zshrc):"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo
fi

echo "🎉 Installation complete!"
echo
echo "Usage:"
echo "    claude-usage daily"
echo "    claude-usage session --last 10"
echo "    claude-usage live --snapshot"
echo "    claude-usage --help"
echo

# Test the installation
if command -v claude-usage &> /dev/null; then
    echo "✅ claude-usage is ready to use!"
    echo "Version: $(claude-usage --version)"
else
    echo "⚠️  claude-usage not found in PATH. You may need to:"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo "    or restart your terminal"
fi
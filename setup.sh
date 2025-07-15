#!/bin/bash

# Claude Usage - Rust Setup Script
# This script builds and installs the Rust version of claude-usage

set -e  # Exit on any error

BINARY_NAME="claude-usage"
INSTALL_DIR="$HOME/.local/bin"
BINARY_PATH="$INSTALL_DIR/$BINARY_NAME"

show_help() {
    echo "Claude Usage - Rust Setup Script"
    echo
    echo "Usage: ./setup.sh [COMMAND]"
    echo
    echo "Commands:"
    echo "  install    Build and install claude-usage (default)"
    echo "  uninstall  Remove claude-usage from system"
    echo "  help       Show this help message"
    echo
    echo "Examples:"
    echo "  ./setup.sh install"
    echo "  ./setup.sh uninstall"
}

install_claude_usage() {
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
    mkdir -p "$INSTALL_DIR"

    echo "🔨 Building Rust version..."
    cd rust
    cargo build --release

    if [ $? -eq 0 ]; then
        echo "✅ Build successful"
    else
        echo "❌ Build failed"
        exit 1
    fi

    echo "📦 Installing to $BINARY_PATH..."
    cp target/release/claude-usage "$BINARY_PATH"
    chmod +x "$BINARY_PATH"

    # Check if ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo "⚠️  $INSTALL_DIR is not in your PATH"
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
}

uninstall_claude_usage() {
    echo "🗑️  Uninstalling Claude Usage..."
    echo

    # Check if binary exists
    if [ -f "$BINARY_PATH" ]; then
        echo "📦 Removing $BINARY_PATH..."
        rm -f "$BINARY_PATH"
        echo "✅ claude-usage removed successfully"
    else
        echo "⚠️  claude-usage not found at $BINARY_PATH"
    fi

    # Check if it's still in PATH (might be installed elsewhere)
    if command -v claude-usage &> /dev/null; then
        CURRENT_PATH=$(which claude-usage)
        echo "⚠️  claude-usage still found at: $CURRENT_PATH"
        echo "You may need to remove it manually if it's installed elsewhere"
    else
        echo "✅ claude-usage completely removed from system"
    fi

    # Clean up build artifacts (optional)
    if [ -d "rust/target" ]; then
        read -p "🧹 Remove build artifacts (rust/target/)? [y/N]: " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "🧹 Cleaning build artifacts..."
            rm -rf rust/target
            echo "✅ Build artifacts cleaned"
        fi
    fi

    echo "🎉 Uninstallation complete!"
}

# Parse command line arguments
case "${1:-install}" in
    install)
        install_claude_usage
        ;;
    uninstall)
        uninstall_claude_usage
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "❌ Unknown command: $1"
        echo
        show_help
        exit 1
        ;;
esac
#!/bin/bash

# 🚀 Claude Usage - Lightning Fast Rust Setup
# Transform your Claude usage analysis with blazing performance

set -e  # Exit on any error

BINARY_NAME="claude-usage"
INSTALL_DIR="$HOME/.local/bin"
BINARY_PATH="$INSTALL_DIR/$BINARY_NAME"

show_help() {
    echo "✨ Claude Usage - Lightning Fast Setup"
    echo
    echo "A delightfully simple way to install the high-performance Rust version"
    echo "of claude-usage for instant insights into your Claude Code sessions."
    echo
    echo "Usage: ./setup.sh [COMMAND]"
    echo
    echo "Commands:"
    echo "  install    🔧 Build and install claude-usage (default)"
    echo "  uninstall  🗑️  Remove claude-usage from your system"
    echo "  help       📖 Show this help message"
    echo
    echo "Examples:"
    echo "  ./setup.sh install    # Quick start"
    echo "  ./setup.sh uninstall  # Clean removal"
}

install_claude_usage() {
    echo "🚀 Welcome to the Claude Usage setup experience!"
    echo
    echo "We're about to transform your Claude usage analysis with lightning-fast"
    echo "performance. This process will take just a minute or two."
    echo

    # Check if we're in the right directory
    if [ ! -d "rust" ]; then
        echo "❌ Oops! We can't find the rust/ directory here."
        echo "💡 Please run this script from the project root directory"
        exit 1
    fi

    # Check if Rust is installed
    if ! command -v rustc &> /dev/null; then
        echo "🦀 Time to install Rust! This powerful language will give you"
        echo "   incredible performance gains over the Python version."
        echo
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
        echo "✨ Rust installed successfully! Welcome to the future of performance."
    else
        echo "✅ Perfect! Rust is already installed ($(rustc --version))"
        echo "   You're all set for blazing-fast execution."
    fi

    # Ensure cargo is available
    if ! command -v cargo &> /dev/null; then
        echo "🔧 Loading your Rust environment..."
        source $HOME/.cargo/env
    fi

    # Create local bin directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    echo "📁 Preparing installation directory at $INSTALL_DIR"

    echo
    echo "🔨 Building the high-performance Rust version..."
    echo "   This might take a moment, but the speed gains are worth it!"
    cd rust
    cargo build --release

    if [ $? -eq 0 ]; then
        echo "✨ Build completed successfully! Your new tool is ready."
    else
        echo "❌ Build encountered an issue. Let's troubleshoot this together."
        exit 1
    fi

    echo
    echo "📦 Installing claude-usage to your local bin..."
    cp target/release/claude-usage "$BINARY_PATH"
    chmod +x "$BINARY_PATH"

    # Check if ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo
        echo "🔍 Almost there! We need to add $INSTALL_DIR to your PATH"
        echo "   Add this line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo
        echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo
        echo "   Then restart your terminal or run: source ~/.bashrc"
        echo
    fi

    echo "🎉 Installation complete! Welcome to lightning-fast usage analysis."
    echo
    echo "🚀 Ready to explore? Try these commands:"
    echo "   claude-usage daily           # Beautiful daily breakdowns"
    echo "   claude-usage session --last 10   # Recent session insights"
    echo "   claude-usage live --snapshot      # Real-time usage snapshot"
    echo "   claude-usage --help              # Full command reference"
    echo

    # Test the installation
    if command -v claude-usage &> /dev/null; then
        echo "✅ Perfect! claude-usage is ready to use right now."
        echo "📊 $(claude-usage --version) is at your service!"
    else
        echo "⚠️  Almost ready! You'll need to update your PATH first:"
        echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo "    Then restart your terminal to start using claude-usage"
    fi
}

uninstall_claude_usage() {
    echo "🔍 Checking for claude-usage installations..."
    echo

    # Check if binary exists
    if [ -f "$BINARY_PATH" ]; then
        echo "📦 Found claude-usage at $BINARY_PATH"
        rm -f "$BINARY_PATH"
        echo "✅ Successfully removed claude-usage"
        REMOVED_SOMETHING=true
    else
        echo "ℹ️  No claude-usage found at $BINARY_PATH"
        REMOVED_SOMETHING=false
    fi

    # Check if it's still in PATH (might be installed elsewhere)
    if command -v claude-usage &> /dev/null; then
        CURRENT_PATH=$(which claude-usage)
        echo "🔍 Found another installation at: $CURRENT_PATH"
        echo "   You may want to remove this one manually if needed"
        REMOVED_SOMETHING=true
    fi

    # Clean up build artifacts (optional)
    if [ -d "rust/target" ]; then
        echo
        read -p "🧹 Clean up build artifacts? [y/N]: " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf rust/target
            echo "✅ Build artifacts cleaned"
            REMOVED_SOMETHING=true
        fi
    fi

    echo
    if [ "$REMOVED_SOMETHING" = true ]; then
        echo "🎉 Cleanup complete!"
    else
        echo "ℹ️  Nothing to remove - claude-usage was not found"
    fi
    echo "💡 Run './setup.sh install' to install claude-usage"
}

# Parse command line arguments
case "${1:-help}" in
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
        echo "🤔 Hmm, '$1' isn't a command we recognize."
        echo
        show_help
        exit 1
        ;;
esac
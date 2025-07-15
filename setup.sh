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
    echo
    echo "💡 Pro tip: The Rust version is significantly faster than Python!"
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
    echo "👋 So long, and thanks for all the insights!"
    echo
    echo "We're sorry to see you go. Let's cleanly remove claude-usage"
    echo "from your system while keeping your precious data intact."
    echo

    # Check if binary exists
    if [ -f "$BINARY_PATH" ]; then
        echo "📦 Removing claude-usage from $BINARY_PATH..."
        rm -f "$BINARY_PATH"
        echo "✅ Successfully removed the binary"
    else
        echo "🤔 Hmm, we couldn't find claude-usage at $BINARY_PATH"
        echo "   It might have been installed elsewhere or already removed"
    fi

    # Check if it's still in PATH (might be installed elsewhere)
    if command -v claude-usage &> /dev/null; then
        CURRENT_PATH=$(which claude-usage)
        echo
        echo "🔍 Wait! We found another claude-usage installation at:"
        echo "   $CURRENT_PATH"
        echo
        echo "   You might want to remove this one manually if it's no longer needed"
    else
        echo "✅ Perfect! claude-usage has been completely removed from your system"
    fi

    # Clean up build artifacts (optional)
    if [ -d "rust/target" ]; then
        echo
        read -p "🧹 Would you like to clean up build artifacts too? [y/N]: " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "🧹 Cleaning up build artifacts..."
            rm -rf rust/target
            echo "✨ Build artifacts cleaned - your workspace is now pristine!"
        else
            echo "👍 No worries! Build artifacts kept in case you want to reinstall later"
        fi
    fi

    echo
    echo "🎉 Uninstallation complete!"
    echo "💡 You can always reinstall by running: ./setup.sh install"
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
        echo "🤔 Hmm, '$1' isn't a command we recognize."
        echo
        show_help
        exit 1
        ;;
esac
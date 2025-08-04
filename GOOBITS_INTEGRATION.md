# Goobits Integration for claude-usage

This document shows how to integrate Goobits features into your existing Rust claude-usage CLI.

## What Goobits Generated

1. **Enhanced setup.sh** - Rust-specific installation script with:
   - Rust version checking
   - Cargo installation options
   - Development mode with symlinks
   - Shell completion generation
   - System dependency checking

2. **Project structure files**:
   - `src/hooks.rs` - Hook system for extensibility
   - `src/config.rs` - Configuration management
   - `src/styling.rs` - Terminal styling utilities
   - `src/plugins.rs` - Plugin system
   - Enhanced `Cargo.toml` with proper metadata

3. **Documentation**:
   - Generated README.md with installation instructions
   - Proper .gitignore for Rust projects

## Integration Options

### Option 1: Use Goobits setup.sh only
Keep your existing Rust code but use the generated setup script for better installation:
```bash
# Your existing code stays the same
# Just use the new setup.sh for installation
./setup.sh --dev  # Development installation with symlinks
./setup.sh        # Production installation with cargo install
```

### Option 2: Add Goobits features selectively
Cherry-pick useful features from the generated code:

1. **Add shell completion support** to your existing CLI:
   ```rust
   // In your main.rs, add completion generation
   use clap_complete::{generate, Generator, Shell};
   
   #[derive(Parser)]
   struct Cli {
       /// Generate shell completions
       #[arg(long = "generate", value_enum)]
       generator: Option<Shell>,
       
       #[command(subcommand)]
       command: Option<Commands>,
   }
   ```

2. **Add the hook system** for extensibility:
   - Copy `goobits-generated/hooks.rs` to your src/
   - Allow users to extend functionality without modifying core code

3. **Use the styling module** for better terminal output:
   - Copy `goobits-generated/styling.rs` to your src/
   - Replace your current display logic with styled output

### Option 3: Full Goobits compliance
Restructure your project to follow Goobits conventions:
- Move business logic to hooks.rs
- Use goobits.yaml as the source of truth
- Let Goobits generate the CLI structure
- Your existing analyzer, parser, etc. become the hook implementations

## Recommended Approach

For your existing, working Rust CLI, I recommend **Option 1**:

1. Keep your existing Rust implementation as-is
2. Use the Goobits-generated `setup.sh` for better installation UX
3. Optionally add shell completion support
4. Keep `goobits.yaml` as documentation of your CLI structure

This gives you the benefits of Goobits (installation, documentation) without disrupting your working code.

## Quick Start

```bash
# Use the enhanced setup script
./setup.sh --dev         # Development mode
./setup.sh --completions # Generate shell completions
./setup.sh --test        # Run tests before installation

# The 'cusage' alias is already configured in goobits.yaml
# After installation, users can use: cusage daily, cusage monthly, cusage live
```
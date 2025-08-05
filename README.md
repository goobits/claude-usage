# Claude Usage Analytics

Fast Rust implementation for Claude usage analysis across multiple VMs

## Installation

### From crates.io (when published)
```bash
cargo install claude-usage
```

### For development
```bash
# Clone the repository
git clone <your-repo-url>
cd claude-usage

# Build and install locally
cargo build --release
# The binary will be available at target/release/claude-usage
```

## Usage

```bash
claude-usage --help
```

## Commands

- `daily` - Show daily usage with project breakdown
- `monthly` - Show monthly usage aggregation
- `live` - Show live monitoring

## Development

To build in development mode:
```bash
cargo build
```

To run tests:
```bash
cargo test
```

To run in development mode:
```bash
cargo run -- [arguments]
```

## License

MIT

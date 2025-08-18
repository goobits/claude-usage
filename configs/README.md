# Configuration Examples

This directory contains example configuration files for different deployment scenarios.

## Available Configurations

### `development.toml`
- **Use case**: Local development and debugging
- **Features**: 
  - Debug logging with pretty formatting
  - Smaller batch sizes for easier debugging
  - Lower memory limits
  - More frequent progress updates
  - Pretty JSON output with metadata

### `production.toml`
- **Use case**: Production deployments
- **Features**:
  - JSON logging to both console and files
  - Larger batch sizes for performance
  - Higher memory limits
  - Longer deduplication windows
  - Optimized for performance and reliability

### `docker.toml`
- **Use case**: Docker containers and Kubernetes
- **Features**:
  - JSON logging to console (for log aggregation)
  - Conservative resource usage
  - Container-friendly paths
  - Compact output format

## Usage

Copy the appropriate configuration file to your project root:

```bash
# For development
cp configs/development.toml claude-usage.toml

# For production
cp configs/production.toml claude-usage.toml

# For Docker
cp configs/docker.toml claude-usage.toml
```

Or reference them directly:
```bash
# Using environment variable
export CLAUDE_USAGE_CONFIG=configs/production.toml

# Using symbolic link
ln -s configs/production.toml claude-usage.toml
```

## Customization

You can customize any configuration by:

1. Copying a base configuration
2. Modifying values as needed
3. Using environment variables for runtime overrides

Example environment overrides:
```bash
export LOG_LEVEL=DEBUG
export CLAUDE_USAGE_MAX_MEMORY_MB=1024
export CLAUDE_USAGE_BATCH_SIZE=15
```
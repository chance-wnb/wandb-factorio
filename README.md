# Factorio W&B Integration

A Factorio mod integrated with Weights & Biases (W&B) for tracking game metrics and statistics.

## Project Structure

- `mod/` - Factorio mod files
- `rust_client/` - Rust client application for W&B integration
- `Makefile` - Build and run automation

## Setup

### Prerequisites

- Factorio game installed
- Rust toolchain (cargo)
- W&B account and SDK

### Environment Configuration

1. Copy the environment template:

```bash
cp .env.template .env
```

2. Edit `.env` and configure the following variables:

```bash
# W&B Core Path
# Path to the wandb binary for core functionality
_WANDB_CORE_PATH=/path/to/wandb/bin

# W&B Rust SDK Path (for local development)
# Path to your local wandb Rust SDK repository
WANDB_SDK_PATH=/path/to/wandb/experimental/rust-sdk

# Rust Logging Level (optional, uncomment to enable)
# Options: error, warn, info, debug, trace
# RUST_LOG=debug
```

**Important**:

- `_WANDB_CORE_PATH` should point to the `bin` directory of your W&B installation
- `WANDB_SDK_PATH` should point to the root of the wandb Rust SDK repository
- Both paths must be absolute paths

### Building

The Rust client uses a template-based `Cargo.toml` that's generated from environment variables:

```bash
# Generate Cargo.toml and build the Rust client
make build-rust-client

# Or just generate the Cargo.toml
make setup-rust-client
```

The `setup-rust-client` target will:

1. Validate that `WANDB_SDK_PATH` is set
2. Generate `rust_client/Cargo.toml` from the template
3. Substitute environment variables in the configuration

## Running

### Rust Client

```bash
# Run with W&B tracking
make run-rust-client

# Run with debug logging
make run-rust-client-debug
```

## Development

### File Structure

- `.env` - Your local environment configuration (not committed to git)
- `.env.template` - Template showing required environment variables (committed to git)
- `rust_client/Cargo.toml.template` - Template for Rust dependencies (committed to git)
- `rust_client/Cargo.toml` - Generated file with actual paths (not committed to git)

## Troubleshooting

### "WANDB_SDK_PATH not set" error

Make sure you've:

1. Created a `.env` file from `.env.template`
2. Set `WANDB_SDK_PATH` to your local wandb Rust SDK path
3. Used an absolute path (not relative)

### Build fails with dependency errors

Verify that `WANDB_SDK_PATH` points to a valid wandb Rust SDK repository with a `Cargo.toml` file.

### envsubst command not found

On macOS:

```bash
brew install gettext
brew link --force gettext
```

On Ubuntu/Debian:

```bash
sudo apt-get install gettext
```

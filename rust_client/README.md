# Factorio Rust Client with W&B

A simple Rust client that tracks Factorio metrics using Weights & Biases.

## Prerequisites

- Rust toolchain installed
- wandb-core binary built at `/Users/your_folder/develop/wandb/wandb/bin/wandb-core`
- W&B API key configured (run `wandb login` or set `WANDB_API_KEY` environment variable)

## Running

```bash
cd /Users/chance.an/develop/factorio/rust_client
_WANDB_CORE_PATH=/Users/chance.an/develop/wandb/wandb/bin cargo run
```

Or with debug logging to see Unix socket communication:

```bash
RUST_LOG=debug _WANDB_CORE_PATH=/Users/chance.an/develop/wandb/wandb/bin cargo run
```

## What it does

This example:

1. Connects to wandb-core via Unix socket (not TCP!)
2. Creates a run under `wandb/factorio-experiments`
3. Logs example Factorio metrics:
   - Science packs per minute
   - Power consumption (MW)
   - Pollution per minute
4. Finishes the run

## Configuration

Edit `src/main.rs` to:

- Change the entity: `settings.proto.entity = Some("your-username".to_string());`
- Change the project: `let project = Some("your-project".to_string());`
- Log different metrics

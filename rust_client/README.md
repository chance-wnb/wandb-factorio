# Factorio Rust Client

Rust client for reading and processing Factorio game events via named pipes.

## Features

- **Pipe Reader with Cache**: Continuously reads from a named pipe and caches events in memory
- **Thread-Safe Access**: Multiple parts of your application can access cached events concurrently
- **Flexible Consumption**: Non-destructive reads (peek) or destructive reads (drain)
- **Filtering Support**: Search and filter events by content
- **Optional Logging**: Can write events to a log file while caching
- **W&B Integration**: Example code for logging metrics to Weights & Biases

## Architecture

### Components

1. **`pipe_cache.rs`**: Thread-safe circular buffer for caching pipe events
   - Runs a background thread that continuously reads from the named pipe
   - Provides multiple access patterns (get all, get last N, drain, filter, etc.)
   - Automatically handles pipe reconnection

2. **`main.rs`**: Example application that monitors events
   - Reads from environment variables for configuration
   - Displays event statistics every 5 seconds

3. **`main_wandb_example.rs`**: Original W&B integration example (for reference)

## Prerequisites

- Rust toolchain installed
- For W&B integration: wandb-core binary built and W&B API key configured

## Configuration

The client uses environment variables (loaded from `.env` in project root):

```bash
# Named pipe path for receiving game events
FACTORIO_PIPE_PATH=$HOME/Library/Application Support/factorio/script-output/events.pipe

# Optional: Log file path for pipe data
FACTORIO_LOG_PATH=/tmp/factorio_events.log
```

## Usage

### Basic Usage

```bash
# Run the client (reads env vars from .env)
make run-rust-client

# Or directly with cargo
cargo run
```

### Programmatic Usage

```rust
use pipe_cache::PipeCache;

// Create cache with 10,000 event capacity
let cache = PipeCache::new(10000);

// Start reading from pipe
cache.start_reader(pipe_path, Some(log_path));

// Non-destructive read - peek at latest event
if let Some(latest) = cache.get_latest() {
    println!("Latest: {}", latest);
}

// Non-destructive read - get last N events
let recent = cache.get_last_n(10);

// Destructive read - consume all events
let events = cache.drain_all();
for event in events {
    // Process each event
}

// Filter events
let session_events = cache.find_containing("session_id");
```

## API Reference

### PipeCache Methods

#### Read Methods (Non-Destructive)
- `get_all() -> Vec<String>`: Get all cached events
- `get_last_n(n: usize) -> Vec<String>`: Get last N events
- `get_latest() -> Option<String>`: Get most recent event
- `filter<F>(predicate: F) -> Vec<String>`: Filter by custom predicate
- `find_containing(search: &str) -> Vec<String>`: Find events containing string
- `len() -> usize`: Get cache size
- `is_empty() -> bool`: Check if cache is empty

#### Write Methods (Destructive)
- `pop_front() -> Option<String>`: Remove and return oldest event
- `drain_all() -> Vec<String>`: Remove and return all events

#### Setup
- `new(capacity: usize) -> PipeCache`: Create new cache
- `start_reader(pipe_path: String, log_path: Option<String>)`: Start background reader

## Event Format

Events are JSON lines from the Factorio mod:

```json
{
  "session_id": "nauvis_0_123456",
  "cycle": 100,
  "tick": 12000,
  "products_production": {
    "iron-plate": 45.5,
    "copper-plate": 30.25
  },
  "materials_consumption": {
    "coal": 4,
    "iron-ore": 20.5
  }
}
```

## Integration with W&B

For W&B integration example, see `main_wandb_example.rs`. You can combine the pipe reader with W&B logging:

```rust
// Parse JSON events and log to W&B
let events = cache.drain_all();
for event_str in events {
    let event: serde_json::Value = serde_json::from_str(&event_str)?;

    // Log to W&B
    let mut metrics = HashMap::new();
    // ... extract metrics from event
    run.log(metrics);
}
```

## Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run with debug logging
RUST_LOG=debug cargo run
```

## Examples

See `examples/pipe_reader_usage.rs` for comprehensive usage examples.

```bash
cargo run --example pipe_reader_usage
```

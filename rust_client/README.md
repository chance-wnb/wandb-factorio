# Factorio Rust Client with WandB Integration

Rust client for reading Factorio game events via named pipes and logging metrics to Weights & Biases (WandB).

## Features

- **Pipe Reader with Cache**: Continuously reads from a named pipe and caches events in memory
- **Thread-Safe Access**: Multiple parts of your application can access cached events concurrently
- **WandB Session Management**: Automatic session lifecycle management with singleton pattern
- **Event Parsing**: JSONL event parsing with typed data structures
- **Auto-Recovery**: Creates WandB sessions automatically if stats arrive without initialization
- **Session Switching**: Detects session ID changes and manages transitions seamlessly

## Architecture

### Components

1. **`pipe_cache.rs`**: Thread-safe circular buffer for caching pipe events
   - Runs a background thread that continuously reads from the named pipe
   - Buffers up to 10,000 events in lock-free queue
   - Automatically handles pipe reconnection

2. **`wandb_manager.rs`**: WandB session manager singleton
   - Manages WandB session lifecycle (init, log, finish)
   - Handles `session_init` events to create new runs
   - Handles `stats` events to log metrics
   - Automatic session switching and recovery

3. **`main.rs`**: Main event processing loop
   - Parses JSONL events from Factorio
   - Routes events to WandB manager
   - Processes events every 5 seconds

4. **`main_wandb_example.rs`**: Original W&B integration example (for reference)

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

### session_init Event
Sent when Factorio starts a new game or loads a save.

```json
{
  "type": "session_init",
  "session_id": "nauvis_12345",
  "tick": 12345,
  "level_name": "nauvis"
}
```

**Behavior:** Closes any existing WandB run and starts a new one.

### stats Event
Sent every 120 ticks (2 seconds) with production and consumption metrics.

```json
{
  "type": "stats",
  "session_id": "nauvis_12345",
  "cycle": 100,
  "tick": 12000,
  "products_production": {
    "iron-plate": 45.5,
    "copper-plate": 30.25
  },
  "materials_consumption": {
    "coal": 4.0,
    "iron-ore": 20.5
  }
}
```

**Behavior:** Logs metrics to WandB. Creates session if none exists.

## WandB Integration

### Session Lifecycle

1. **Session Creation**
   - Triggered by `session_init` event or first `stats` event
   - Run name format: `{session_id}_{random_seed}`
   - Project: `factorio-experiments`
   - Entity: `wandb`

2. **Metric Logging**
   - Production metrics: `production/{item_name}`
   - Consumption metrics: `consumption/{item_name}`
   - Step number: Uses `cycle` field from stats event via `HistoryStep` protobuf field

3. **Session Termination**
   - Automatically closed when new `session_init` is received
   - Called on application shutdown via `Drop` trait

### Key Features

- **Singleton Pattern:** Only one active WandB session at a time
- **Auto-Recovery:** Creates session if stats arrive without active session
- **Session Switching:** Detects session ID changes and switches automatically
- **Thread-Safe:** Uses `Arc<Mutex<>>` for concurrent access

### Example Output

```
Starting Factorio Rust Client...
Pipe reader started. Monitoring events...

=== Processing Cycle ===
Drained 1 events from queue
  [1] SessionInit: nauvis_12345 (tick: 12345, level: nauvis)
📍 Session init received: nauvis_12345
🚀 Starting new WandB run: nauvis_12345_1847293
✅ WandB run initialized successfully

=== Processing Cycle ===
Drained 5 events from queue
  [1] Stats: cycle=1, tick=120, production_items=2, consumption_items=3
📊 Logged 5 metrics at cycle 1
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

# WandB JSONL Row Consumption Implementation

## Overview

Implemented a WandB central control system that automatically manages session lifecycle based on JSONL events from Factorio.

## Implementation Summary

### Files Created/Modified

1. **[rust_client/src/wandb_manager.rs](rust_client/src/wandb_manager.rs)** (NEW)
   - WandB session manager singleton
   - Handles session initialization, metrics logging, and session cleanup
   - Thread-safe implementation using `Arc<Mutex<>>`

2. **[rust_client/src/main.rs](rust_client/src/main.rs)** (MODIFIED)
   - Added JSONL event parsing with typed structures
   - Integrated WandB manager into event processing loop
   - Routes `session_init` and `stats` events to WandB manager

3. **[rust_client/Cargo.toml](rust_client/Cargo.toml)** (MODIFIED)
   - Added dependencies: `serde`, `serde_json`, `rand`

4. **[rust_client/README.md](rust_client/README.md)** (UPDATED)
   - Updated documentation with WandB integration details

## Key Features Implemented

### ✅ Session Initialization (`session_init`)
- **Behavior:** When `session_init` event is received:
  1. Closes any existing WandB session using `run.finish()`
  2. Creates new WandB run with `wandb.init()`
  3. Run name: `{session_id}_{random_seed}`
  4. Logs initial metadata: `session_id`, `level_name`, `start_tick`

### ✅ Metrics Logging (`stats`)
- **Behavior:** When `stats` event is received:
  1. Checks if active session exists
  2. If no session exists, creates one immediately
  3. If session ID mismatches, closes old and creates new
  4. Logs metrics using `run.log()` with step number from `cycle`

### ✅ Automatic Session Management
- **Singleton Pattern:** Only one active session at a time
- **Auto-Recovery:** Creates session if stats arrive before session_init
- **Session Switching:** Detects session ID changes and manages transitions
- **Clean Shutdown:** `Drop` trait ensures session cleanup on exit

## Event Flow

```
┌─────────────────────┐
│  Factorio Mod       │
│  (control.lua)      │
└──────────┬──────────┘
           │ Writes JSONL
           ▼
┌─────────────────────┐
│  Named Pipe         │
│  (events.pipe)      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  PipeCache          │
│  (Background Thread)│
└──────────┬──────────┘
           │ Buffers events
           ▼
┌─────────────────────┐
│  Main Event Loop    │
│  (main.rs)          │
└──────────┬──────────┘
           │ Parses JSONL
           ▼
     ┌─────┴─────┐
     │           │
     ▼           ▼
session_init   stats
     │           │
     └─────┬─────┘
           ▼
┌─────────────────────┐
│  WandBManager       │
│  (Singleton)        │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  WandB Backend      │
└─────────────────────┘
```

## WandB Configuration

- **Project:** `factorio-experiments`
- **Entity:** `wandb`
- **Run Name:** `{session_id}_{random_seed}`
- **Step:** Uses `cycle` from stats event

### Metrics Format

```
production/{item_name}  → Float value (e.g., "production/iron-plate": 450.5)
consumption/{item_name} → Float value (e.g., "consumption/iron-ore": 500.0)
```

**Step Specification:** The cycle number is passed via the `HistoryStep` protobuf field using `run.log_with_step()`, not as a metric.

## Example Usage

```bash
# Build the client
cd rust_client
cargo build --release

# Run the client
cargo run --release
```

## Testing Scenarios

### Scenario 1: Normal Flow
1. Factorio starts → `session_init` event
2. WandB creates new run
3. Stats arrive every 2 seconds → metrics logged

### Scenario 2: Stats Before Init
1. Stats arrive without session
2. WandB creates session immediately
3. Future stats use that session

### Scenario 3: Session Change
1. Active session exists
2. New `session_init` with different session_id
3. Old session finished, new session created

### Scenario 4: Application Shutdown
1. User stops Rust client
2. `Drop` trait calls `finish_current_session()`
3. WandB run closed properly

## Code Structure

### WandBManager Methods

```rust
pub fn new() -> Self
pub fn handle_session_init(&self, session_id, tick, level_name)
pub fn handle_stats_event(&self, session_id, cycle, tick, products, materials)
pub fn shutdown(&self)
```

### Event Types

```rust
enum FactorioEvent {
    SessionInit {
        session_id: String,
        tick: u64,
        level_name: String,
    },
    Stats {
        session_id: String,
        cycle: u64,
        tick: u64,
        products_production: HashMap<String, f64>,
        materials_consumption: HashMap<String, f64>,
    },
}
```

## Design Decisions

1. **Singleton Pattern:** Ensures only one WandB session active at a time
2. **Separation of Concerns:** WandB logic isolated in `wandb_manager.rs`
3. **Thread Safety:** `Arc<Mutex<>>` allows safe concurrent access
4. **Auto-Recovery:** Handles edge cases gracefully
5. **Random Seed in Run Name:** Ensures unique run IDs for same session

## Future Enhancements

- [ ] Add configuration file support for project/entity
- [ ] Implement retry logic for WandB API failures
- [ ] Add metric aggregation to reduce API calls
- [ ] Support for additional event types
- [ ] Add health check endpoint
- [ ] Implement graceful shutdown on SIGINT/SIGTERM

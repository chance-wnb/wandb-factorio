# Factorio Project Implementation Summary

## Overview
Successfully refactored the Factorio mod to dump production/consumption statistics as JSON to a named pipe, and implemented a Rust client with a thread-safe cache to consume the events.

## Components Implemented

### 1. Factorio Mod (`/mod`)

#### Files Created/Modified:
- **`control.lua`**: Main mod logic
  - Session ID generation with random suffix for uniqueness
  - Production/consumption stats collection (items + fluids)
  - JSON output to named pipe every 120 ticks (2 seconds)
  - Event handlers for game initialization and loading

- **`utils.lua`**: Utility functions
  - `format_number()`: Rounds numbers to 5 decimal places
  - `format_numbers_in_table()`: Recursively formats all numbers in a table

- **`data.lua`**: Empty prototype file (standard mod structure)

#### Key Features:
- **Session ID**: `{level_name}_{tick}_{random_6_digits}`
  - Regenerated on every game load
  - Ensures uniqueness even when loading from same tick
- **Cycle ID**: `math.floor(tick / 120)` - represents 2-second intervals
- **Combined stats**: Items and fluids merged into `products_production` and `materials_consumption`
- **Number formatting**: All rates rounded to 5 decimal places to avoid floating-point precision issues

#### JSON Output Format:
```json
{
  "session_id": "nauvis_5000_347821",
  "cycle": 100,
  "tick": 12000,
  "products_production": {
    "iron-plate": 45.5,
    "copper-plate": 30.25,
    "water": 1200.0
  },
  "materials_consumption": {
    "coal": 4,
    "iron-ore": 20.5,
    "crude-oil": 450.5
  }
}
```

### 2. Rust Client (`/rust_client`)

#### Files Created:
- **`src/pipe_cache.rs`**: Thread-safe event cache
  - Background thread continuously reads from named pipe
  - Circular buffer with 10,000 event capacity
  - Multiple consumption patterns (non-destructive/destructive reads)
  - Automatic pipe reconnection on failure
  - Optional log file writing

- **`src/main.rs`**: Main application
  - Reads from environment variables
  - Monitors cache and displays statistics
  - Example of continuous event monitoring

- **`src/main_wandb_example.rs`**: Original W&B example (preserved for reference)

- **`examples/pipe_reader_usage.rs`**: Comprehensive usage examples

- **`README.md`**: Documentation for the Rust client

#### PipeCache API:

**Non-Destructive Reads** (events stay in cache):
- `get_all()` - Get all cached events
- `get_last_n(n)` - Get last N events
- `get_latest()` - Get most recent event
- `filter(predicate)` - Filter by custom function
- `find_containing(str)` - Find events containing string
- `len()` / `is_empty()` - Cache info

**Destructive Reads** (events removed from cache):
- `pop_front()` - Remove and return oldest event
- `drain_all()` - Remove and return all events

### 3. Infrastructure

#### Makefile Targets:
```bash
make read-pipe          # Read from pipe and print to screen
make run-rust-client    # Run Rust client (uses .env)
make build-rust-client  # Build Rust client
```

#### Environment Variables (`.env`):
```bash
FACTORIO_PIPE_PATH=$HOME/Library/Application Support/factorio/script-output/events.pipe
FACTORIO_LOG_PATH=/tmp/factorio_events.log
WANDB_SDK_PATH=/path/to/wandb/experimental/rust-sdk
```

## Architecture

```
┌─────────────────┐
│  Factorio Game  │
│   (Lua Mod)     │
└────────┬────────┘
         │ JSON lines every 120 ticks
         ▼
┌─────────────────┐
│   Named Pipe    │
│  (events.pipe)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Rust Client    │
│  (PipeCache)    │
│  - Background   │
│    reader thread│
│  - Circular     │
│    buffer cache │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Application    │
│  (W&B, RL, etc) │
└─────────────────┘
```

## Named Pipe Behavior

**Problem**: Named pipes block on write if no reader is connected
**Solution**: Always run the Rust client (or `make read-pipe`) BEFORE starting Factorio

The Rust client automatically:
- Retries connection every 1 second if pipe doesn't exist
- Reconnects if pipe is closed/broken
- Maintains cache across reconnections

## Usage Workflow

1. **Start the pipe reader**:
   ```bash
   make read-pipe
   # OR
   make run-rust-client
   ```

2. **Start Factorio** with the mod installed

3. **Load or create a game**
   - New game: Session ID generated in `on_init`
   - Load save: Session ID regenerated on first tick

4. **Events flow automatically**:
   - Every 120 ticks (2 seconds) → JSON event → Pipe → Rust cache

5. **Consume events** in your application:
   ```rust
   let cache = PipeCache::new(10000);
   cache.start_reader(pipe_path, None);

   // Later...
   let events = cache.drain_all();
   // Process events
   ```

## Testing

1. **Test pipe reader**:
   ```bash
   make read-pipe
   ```

2. **Test Rust client**:
   ```bash
   make run-rust-client
   ```

3. **Check JSON output**:
   - Load Factorio game
   - Observe JSON lines in terminal
   - Verify session_id changes on each load

## Future Integration Points

### W&B Integration
The pipe cache can be integrated with W&B logging:

```rust
let events = cache.drain_all();
for event_str in events {
    let event: serde_json::Value = serde_json::from_str(&event_str)?;

    // Extract metrics
    let cycle = event["cycle"].as_i64()?;
    let products = &event["products_production"];

    // Log to W&B
    let mut metrics = HashMap::new();
    for (item, rate) in products.as_object()? {
        metrics.insert(
            format!("production/{}", item),
            wandb::run::Value::Float(rate.as_f64()?)
        );
    }
    run.log(metrics);
}
```

### RL Agent Integration
The cache provides immediate access to game state for decision-making:

```rust
// Get latest state
if let Some(latest) = cache.get_latest() {
    let state: GameState = serde_json::from_str(&latest)?;

    // RL agent uses state for action
    let action = agent.decide(state);

    // Execute action in game (via RCON or other mechanism)
}
```

## Key Design Decisions

1. **Cycle vs Tick**: Added cycle ID for easier 2-second interval tracking
2. **Combined Stats**: Merged items and fluids (no naming conflicts)
3. **Session ID with Random**: Ensures uniqueness across loads from same tick
4. **Number Rounding**: Prevents floating-point precision issues in JSON
5. **Thread-Safe Cache**: Allows concurrent access from multiple application components
6. **Non-blocking Reads**: Background thread prevents game from hanging

## Known Limitations

1. No real-world timestamp (Factorio Lua API limitation)
2. Session ID relies on random suffix (1 in 900,000 collision probability)
3. Pipe must have an active reader before Factorio starts writing
4. Named pipes are platform-specific (macOS/Linux only, not Windows)

## Files Summary

```
factorio/
├── mod/
│   ├── control.lua          # Main mod logic
│   ├── utils.lua            # Number formatting utilities
│   └── data.lua             # Empty prototype file
├── rust_client/
│   ├── src/
│   │   ├── main.rs          # Main application (monitoring)
│   │   ├── pipe_cache.rs    # Thread-safe event cache
│   │   └── main_wandb_example.rs  # W&B example (reference)
│   ├── examples/
│   │   └── pipe_reader_usage.rs   # Usage examples
│   └── README.md            # Rust client documentation
├── Makefile                 # Build and run targets
├── .env                     # Environment configuration
├── .env.template            # Environment template
└── IMPLEMENTATION_SUMMARY.md  # This file
```

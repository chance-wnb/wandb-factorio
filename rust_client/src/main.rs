mod pipe_cache;

use pipe_cache::PipeCache;
use std::env;
use std::thread;
use std::time::Duration;

fn main() {
    println!("Starting Factorio Rust Client...");

    // Get pipe path from environment variable
    let pipe_path = env::var("FACTORIO_PIPE_PATH")
        .unwrap_or_else(|_| {
            let home = env::var("HOME").expect("HOME environment variable not set");
            format!("{}/Library/Application Support/factorio/script-output/events.pipe", home)
        });

    // Get optional log path from environment variable
    let log_path = env::var("FACTORIO_LOG_PATH").ok();

    println!("Pipe path: {}", pipe_path);
    if let Some(ref log) = log_path {
        println!("Log path: {}", log);
    }

    // Create pipe cache with 10,000 event capacity
    let cache = PipeCache::new(10000);

    // Start the background reader thread
    cache.start_reader(pipe_path, log_path);

    println!("Pipe reader started. Monitoring events...\n");

    // Process events by draining the queue
    loop {
        thread::sleep(Duration::from_secs(5));

        // Drain all events from the cache
        let events = cache.drain_all();

        println!("=== Processing Cycle ===");
        println!("Drained {} events from queue", events.len());

        if !events.is_empty() {
            // Process each event
            for (i, event) in events.iter().enumerate() {
                println!("  [{}] {}", i + 1, event);
                // TODO: Parse JSON, extract metrics, log to W&B, etc.
            }
        }
        println!();
    }
}

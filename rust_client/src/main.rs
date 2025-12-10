mod event_mediator;
mod pipe_cache;
mod wandb_manager;

use event_mediator::EventMediator;
use pipe_cache::PipeCache;
use wandb_manager::WandbManager;
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

    // Create WandB manager and event mediator
    let wandb_manager = WandbManager::new();
    let mediator = EventMediator::new(wandb_manager);

    // Start the background reader thread
    cache.start_reader(pipe_path, log_path);

    println!("Pipe reader started. Monitoring events...\n");

    // Process events by draining the queue
    loop {
        thread::sleep(Duration::from_secs(5));

        // Drain all events from the cache
        let events = cache.drain_all();

        // Process events through the mediator
        mediator.process_events(events);
    }
}

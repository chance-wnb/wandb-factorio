mod event_mediator;
mod pipe_cache;
mod wandb_manager;
mod weave_client;
mod weave_manager;

use event_mediator::EventMediator;
use pipe_cache::PipeCache;
use wandb_manager::WandbManager;
use weave_manager::WeaveManager;
use std::env;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
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
    let cache = Arc::new(PipeCache::new(10000));

    // Create WandB manager, Weave manager, and event mediator
    let wandb_manager = WandbManager::new();
    let weave_manager = WeaveManager::new();
    let mediator = Arc::new(EventMediator::new(wandb_manager, weave_manager));

    // Start the background reader thread
    cache.start_reader(pipe_path, log_path);

    println!("Pipe reader started. Monitoring events...\n");

    // Set up graceful shutdown
    let mediator_shutdown = mediator.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("\n🛑 Received shutdown signal, cleaning up...");
        mediator_shutdown.shutdown().await;
        std::process::exit(0);
    });

    // Process events by draining the queue
    loop {
        sleep(Duration::from_secs(5)).await;

        // Drain all events from the cache
        let events = cache.drain_all();

        // Process events through the mediator (async)
        mediator.process_events(events).await;
    }
}

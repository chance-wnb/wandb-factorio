use crate::wandb_manager::WandbManager;
use crate::weave_manager::WeaveManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event types from Factorio
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum FactorioEvent {
    #[serde(rename = "session_init")]
    SessionInit {
        session_id: String,
        tick: u64,
        level_name: String,
    },
    #[serde(rename = "stats")]
    Stats {
        session_id: String,
        cycle: u64,
        tick: u64,
        products_production: HashMap<String, f64>,
        materials_consumption: HashMap<String, f64>,
    },
    // TODO: Add Weave-specific event types here
    // #[serde(rename = "trace_start")]
    // TraceStart { ... },
    // #[serde(rename = "trace_end")]
    // TraceEnd { ... },
}

/// Event mediator that routes Factorio events to WandB and Weave managers
pub struct EventMediator {
    wandb_manager: WandbManager,
    weave_manager: WeaveManager,
}

impl EventMediator {
    /// Creates a new event mediator
    pub fn new(wandb_manager: WandbManager, weave_manager: WeaveManager) -> Self {
        EventMediator {
            wandb_manager,
            weave_manager,
        }
    }

    /// Processes a batch of JSONL event strings (async)
    pub async fn process_events(&self, events: Vec<String>) {
        if events.is_empty() {
            return;
        }

        println!("=== Processing Cycle ===");
        println!("Drained {} events from queue", events.len());

        for (i, event_str) in events.iter().enumerate() {
            self.process_single_event(i + 1, event_str).await;
        }
        println!();
    }

    /// Processes a single JSONL event string (async)
    async fn process_single_event(&self, index: usize, event_str: &str) {
        match serde_json::from_str::<FactorioEvent>(event_str) {
            Ok(event) => {
                self.route_event(index, event).await;
            }
            Err(e) => {
                eprintln!(
                    "  [{}] Failed to parse event: {} - Error: {}",
                    index, event_str, e
                );
            }
        }
    }

    /// Routes a parsed event to the appropriate handler (async)
    async fn route_event(&self, index: usize, event: FactorioEvent) {
        match event {
            FactorioEvent::SessionInit {
                session_id,
                tick,
                level_name,
            } => {
                println!(
                    "  [{}] SessionInit: {} (tick: {}, level: {})",
                    index, session_id, tick, level_name
                );

                // Notify both WandB and Weave managers
                self.wandb_manager
                    .handle_session_init(session_id.clone(), tick, level_name.clone());
                self.weave_manager
                    .handle_session_init(session_id, tick, level_name)
                    .await;
            }
            FactorioEvent::Stats {
                session_id,
                cycle,
                tick,
                products_production,
                materials_consumption,
            } => {
                println!(
                    "  [{}] Stats: cycle={}, tick={}, production_items={}, consumption_items={}",
                    index,
                    cycle,
                    tick,
                    products_production.len(),
                    materials_consumption.len()
                );
                self.wandb_manager.handle_stats_event(
                    session_id,
                    cycle,
                    tick,
                    products_production,
                    materials_consumption,
                );
            }
        }
    }

    /// Shutdown both managers gracefully
    pub async fn shutdown(&self) {
        println!("Shutting down event mediator...");
        self.weave_manager.shutdown().await;
        println!("Event mediator shutdown complete");
    }
}

use crate::wandb_manager::WandbManager;
use crate::weave_manager::WeaveManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Position in the game world
#[derive(Debug, Deserialize, Serialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Player information from stats event
#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerInfo {
    pub position: Position,
    pub surface: String,
    pub health: f64,
}

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
        #[serde(default)]
        player: Option<PlayerInfo>,
        #[serde(default)]
        screenshot_path: Option<String>,
        products_production: HashMap<String, f64>,
        materials_consumption: HashMap<String, f64>,
    },
    #[serde(rename = "event")]
    GameEvent {
        event_name: String,
        session_id: String,
        tick: u64,
        #[serde(default)]
        player_index: Option<u32>,
        #[serde(default)]
        entity: Option<String>,
        #[serde(default)]
        position: Option<Position>,
        #[serde(default)]
        surface: Option<String>,
        #[serde(default)]
        tech_name: Option<String>,
        #[serde(default)]
        tech_level: Option<u32>,
        #[serde(default)]
        item: Option<String>,
        #[serde(default)]
        count: Option<u32>,
    },
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
                player,
                screenshot_path,
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
                    products_production.clone(),
                    materials_consumption,
                );

                // Log player snapshot to Weave if player and screenshot are present
                if let (Some(player_info), Some(screenshot)) = (player, screenshot_path) {
                    self.weave_manager
                        .handle_player_snapshot(tick, player_info, screenshot)
                        .await;
                }
            }
            FactorioEvent::GameEvent {
                event_name,
                session_id: _,
                tick,
                player_index,
                entity,
                position,
                surface,
                tech_name,
                tech_level,
                item,
                count,
            } => {
                println!("  [{}] GameEvent: {} (tick: {})", index, event_name, tick);

                // Route to appropriate handler based on event_name
                match event_name.as_str() {
                    "on_research_started" => {
                        if let (Some(name), Some(level)) = (tech_name, tech_level) {
                            self.weave_manager
                                .handle_research_started(tick, name, level)
                                .await;
                        }
                    }
                    "on_research_finished" => {
                        if let (Some(name), Some(level)) = (tech_name, tech_level) {
                            self.weave_manager
                                .handle_research_finished(tick, name, level)
                                .await;
                        }
                    }
                    "on_built_entity" => {
                        if let (Some(idx), Some(ent), Some(pos), Some(surf)) =
                            (player_index, entity, position, surface)
                        {
                            self.weave_manager
                                .handle_entity_built(tick, idx, ent, pos.x, pos.y, surf)
                                .await;
                        }
                    }
                    "on_player_mined_entity" => {
                        if let (Some(idx), Some(ent), Some(pos), Some(surf)) =
                            (player_index, entity, position, surface)
                        {
                            self.weave_manager
                                .handle_entity_mined(tick, idx, ent, pos.x, pos.y, surf)
                                .await;
                        }
                    }
                    "on_player_crafted_item" => {
                        if let (Some(idx), Some(itm), Some(cnt)) = (player_index, item, count) {
                            self.weave_manager
                                .handle_item_crafted(tick, idx, itm, cnt)
                                .await;
                        }
                    }
                    _ => {
                        eprintln!("  [{}] Unknown event type: {}", index, event_name);
                    }
                }
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

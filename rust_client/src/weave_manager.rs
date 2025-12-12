use crate::weave_client::{
    EndedCallSchemaForInsert, StartedCallSchemaForInsert, WeaveClient, WeaveConfig,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// A singleton service that manages Weave sessions for Factorio events.
/// Handles trace logging via start_call() and end_call() operations.
/// Weave sessions map 1:1 with WandB sessions using the same session_id.
pub struct WeaveManager {
    current_session_id: Arc<Mutex<Option<String>>>,
    active_calls: Arc<Mutex<HashMap<String, CallContext>>>,
    /// Cache for research events: key is "tech_name:tech_level", value is the call_id
    research_cache: Arc<Mutex<HashMap<String, String>>>,
    client: Arc<Mutex<Option<WeaveClient>>>,
    config: WeaveConfig,
}

/// Context for an active Weave call/trace
#[derive(Debug, Clone)]
struct CallContext {
    call_id: String,
    trace_id: String,
    session_id: String,
    start_tick: u64,
    inputs: HashMap<String, String>,
}

impl WeaveManager {
    /// Creates a new Weave manager instance
    pub fn new() -> Self {
        // Load config from environment
        let config = match WeaveConfig::from_env() {
            Ok(cfg) => {
                println!(
                    "✅ Weave config loaded: entity={}, project={}",
                    cfg.entity, cfg.project
                );
                cfg
            }
            Err(e) => {
                eprintln!("⚠️  Failed to load Weave config: {}", e);
                eprintln!("⚠️  Weave integration will be disabled");
                // Create a dummy config - client won't be initialized
                WeaveConfig {
                    entity: "unknown".to_string(),
                    project: "unknown".to_string(),
                    base_url: "https://trace.wandb.ai".to_string(),
                    api_key: "dummy".to_string(),
                    binary_path: std::path::PathBuf::from("/dev/null"),
                    socket_path: std::path::PathBuf::from("/dev/null"),
                }
            }
        };

        WeaveManager {
            current_session_id: Arc::new(Mutex::new(None)),
            active_calls: Arc::new(Mutex::new(HashMap::new())),
            research_cache: Arc::new(Mutex::new(HashMap::new())),
            client: Arc::new(Mutex::new(None)),
            config,
        }
    }

    /// Initialize the Weave client connection
    async fn ensure_client(&self) -> Result<(), String> {
        let mut client_guard = self.client.lock().await;

        if client_guard.is_some() {
            return Ok(());
        }

        let client = WeaveClient::new(self.config.clone());
        client.init().await?;

        *client_guard = Some(client);
        Ok(())
    }

    /// Handles a session_init event. Creates a new Weave session matching WandB.
    pub async fn handle_session_init(&self, session_id: String, tick: u64, level_name: String) {
        println!("🔷 Weave session init: {}", session_id);

        // End any active calls from previous session
        self.end_all_calls().await;

        // Clear research cache for new session
        self.research_cache.lock().await.clear();
        println!("🔷 Research cache cleared for new session");

        // Store new session ID
        *self.current_session_id.lock().await = Some(session_id.clone());

        // Ensure client is initialized
        if let Err(e) = self.ensure_client().await {
            eprintln!("⚠️  Failed to initialize Weave client: {}", e);
            return;
        }

        println!(
            "🔷 Weave session created: {} (tick: {}, level: {})",
            session_id, tick, level_name
        );

        // Log the session_init event as an atomic call
        let mut inputs = HashMap::new();
        inputs.insert("tick".to_string(), tick.to_string());
        inputs.insert("level_name".to_string(), level_name.clone());

        let mut outputs = HashMap::new();
        outputs.insert("session_id".to_string(), session_id.clone());
        outputs.insert("level_name".to_string(), level_name);

        self.log_call("session_init".to_string(), tick, inputs, outputs)
            .await;
    }

    /// Starts a new Weave call/trace
    pub async fn start_call(
        &self,
        call_id: String,
        operation: String,
        tick: u64,
        inputs: HashMap<String, String>,
    ) {
        // Ensure client is initialized (creates session if needed)
        if let Err(e) = self.ensure_client().await {
            eprintln!("⚠️  Failed to ensure Weave client: {}", e);
            return;
        }

        // Get or create session
        let mut session_guard = self.current_session_id.lock().await;
        let session_id = match session_guard.as_ref() {
            Some(id) => id.clone(),
            None => {
                // Create a just-in-time session if none exists
                let jit_session_id = format!("jit_session_{}", tick);
                println!("🔷 Creating JIT session: {}", jit_session_id);
                *session_guard = Some(jit_session_id.clone());
                jit_session_id
            }
        };
        drop(session_guard);

        // Now we're guaranteed to have a session_id
        // Generate UUIDs
        let weave_call_id = Uuid::now_v7().to_string();
        let trace_id = Uuid::now_v7().to_string();

        let context = CallContext {
            call_id: weave_call_id.clone(),
            trace_id: trace_id.clone(),
            session_id: session_id.clone(),
            start_tick: tick,
            inputs: inputs.clone(),
        };

        self.active_calls
            .lock()
            .await
            .insert(call_id.clone(), context);

        println!(
            "🔷 Weave call started: '{}' operation='{}' tick={} session={} weave_id={}",
            call_id, operation, tick, session_id, weave_call_id
        );

        // Send to Weave
        if let Err(e) = self
            .send_start_call(weave_call_id, trace_id, operation, tick, inputs)
            .await
        {
            eprintln!("⚠️  Failed to send start call to Weave: {}", e);
        }
    }

    /// Sends a start call to Weave
    async fn send_start_call(
        &self,
        call_id: String,
        trace_id: String,
        operation: String,
        tick: u64,
        inputs: HashMap<String, String>,
    ) -> Result<(), String> {
        let client_guard = self.client.lock().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| "Weave client not initialized".to_string())?;

        // Build attributes (metadata about the call)
        let mut attributes: HashMap<String, serde_json::Value> = HashMap::new();
        attributes.insert("tick".to_string(), serde_json::json!(tick));

        // Convert inputs to JSON values
        let mut inputs_json: HashMap<String, serde_json::Value> = HashMap::new();
        for (k, v) in inputs.iter() {
            inputs_json.insert(k.clone(), serde_json::json!(v));
        }

        let start = StartedCallSchemaForInsert {
            project_id: self.config.project_id(),
            id: Some(call_id),
            op_name: operation,
            display_name: None,
            trace_id: Some(trace_id),
            parent_id: None,
            thread_id: None,
            turn_id: None,
            started_at: Utc::now(),
            attributes,
            inputs: inputs_json,
        };

        client.start_call(start).await
    }

    /// Ends an active Weave call/trace
    pub async fn end_call(
        &self,
        call_id: String,
        tick: u64,
        outputs: HashMap<String, String>,
        success: bool,
    ) {
        let mut active_calls = self.active_calls.lock().await;

        match active_calls.remove(&call_id) {
            None => {
                eprintln!("⚠️  Cannot end Weave call '{}': call not found", call_id);
            }
            Some(context) => {
                let duration_ticks = tick - context.start_tick;

                println!(
                    "🔷 Weave call ended: '{}' duration={} ticks success={} session={} weave_id={}",
                    call_id, duration_ticks, success, context.session_id, context.call_id
                );

                // Send to Weave
                drop(active_calls); // Release lock before async call
                if let Err(e) = self
                    .send_end_call(context.call_id, tick, duration_ticks, outputs, success)
                    .await
                {
                    eprintln!("⚠️  Failed to send end call to Weave: {}", e);
                }
            }
        }
    }

    /// Sends an end call to Weave
    async fn send_end_call(
        &self,
        call_id: String,
        tick: u64,
        duration_ticks: u64,
        outputs: HashMap<String, String>,
        success: bool,
    ) -> Result<(), String> {
        let client_guard = self.client.lock().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| "Weave client not initialized".to_string())?;

        // Build output
        let mut output_map: HashMap<String, serde_json::Value> = HashMap::new();
        output_map.insert("success".to_string(), serde_json::json!(success));
        output_map.insert("tick".to_string(), serde_json::json!(tick));
        for (k, v) in outputs.iter() {
            output_map.insert(k.clone(), serde_json::json!(v));
        }

        // Build summary
        let mut summary: HashMap<String, serde_json::Value> = HashMap::new();
        summary.insert(
            "duration_ticks".to_string(),
            serde_json::json!(duration_ticks),
        );

        let end = EndedCallSchemaForInsert {
            project_id: self.config.project_id(),
            id: call_id,
            ended_at: Utc::now(),
            exception: if success {
                None
            } else {
                Some("Call failed".to_string())
            },
            output: Some(serde_json::to_value(output_map).unwrap()),
            summary,
        };

        client.end_call(end).await
    }

    /// Logs an atomic call to Weave (start and end at the same time).
    /// Useful for instant events that don't have duration.
    pub async fn log_call(
        &self,
        operation: String,
        tick: u64,
        inputs: HashMap<String, String>,
        outputs: HashMap<String, String>,
    ) {
        // Ensure client is initialized
        if let Err(e) = self.ensure_client().await {
            eprintln!("⚠️  Failed to ensure Weave client: {}", e);
            return;
        }

        // Get or create session
        let mut session_guard = self.current_session_id.lock().await;
        let session_id = match session_guard.as_ref() {
            Some(id) => id.clone(),
            None => {
                // Create a just-in-time session if none exists
                let jit_session_id = format!("jit_session_{}", tick);
                println!("🔷 Creating JIT session: {}", jit_session_id);
                *session_guard = Some(jit_session_id.clone());
                jit_session_id
            }
        };
        drop(session_guard);

        // Generate UUIDs
        let weave_call_id = Uuid::now_v7().to_string();
        let trace_id = Uuid::now_v7().to_string();

        println!(
            "🔷 Weave instant call: operation='{}' tick={} session={} weave_id={}",
            operation, tick, session_id, weave_call_id
        );

        // Send start and end calls
        if let Err(e) = self
            .send_start_call(
                weave_call_id.clone(),
                trace_id,
                operation.clone(),
                tick,
                inputs,
            )
            .await
        {
            eprintln!("⚠️  Failed to send start call to Weave: {}", e);
            return;
        }

        if let Err(e) = self
            .send_end_call(weave_call_id, tick, 0, outputs, true)
            .await
        {
            eprintln!("⚠️  Failed to send end call to Weave: {}", e);
        }
    }

    /// Handles research started event
    pub async fn handle_research_started(
        &self,
        tick: u64,
        tech_name: String,
        tech_level: u32,
    ) {
        let research_key = format!("{}:{}", tech_name, tech_level);

        let mut inputs = HashMap::new();
        inputs.insert("tech_name".to_string(), tech_name.clone());
        inputs.insert("tech_level".to_string(), tech_level.to_string());

        // Start a call and store the call_id in the research cache
        self.start_call(
            research_key.clone(),
            "research".to_string(),
            tick,
            inputs,
        )
        .await;
    }

    /// Handles research finished event
    pub async fn handle_research_finished(
        &self,
        tick: u64,
        tech_name: String,
        tech_level: u32,
    ) {
        let research_key = format!("{}:{}", tech_name, tech_level);

        let mut outputs = HashMap::new();
        outputs.insert("tech_name".to_string(), tech_name.clone());
        outputs.insert("tech_level".to_string(), tech_level.to_string());
        outputs.insert("completed".to_string(), "true".to_string());

        // End the call using the research key as call_id
        self.end_call(research_key, tick, outputs, true).await;
    }

    /// Handles entity built event
    pub async fn handle_entity_built(
        &self,
        tick: u64,
        player_index: u32,
        entity: String,
        position_x: f64,
        position_y: f64,
        surface: String,
    ) {
        let mut inputs = HashMap::new();
        inputs.insert("player_index".to_string(), player_index.to_string());
        inputs.insert("entity".to_string(), entity.clone());
        inputs.insert("position_x".to_string(), position_x.to_string());
        inputs.insert("position_y".to_string(), position_y.to_string());
        inputs.insert("surface".to_string(), surface.clone());

        let mut outputs = HashMap::new();
        outputs.insert("entity".to_string(), entity);
        outputs.insert("surface".to_string(), surface);

        self.log_call("on_built_entity".to_string(), tick, inputs, outputs)
            .await;
    }

    /// Handles entity mined event
    pub async fn handle_entity_mined(
        &self,
        tick: u64,
        player_index: u32,
        entity: String,
        position_x: f64,
        position_y: f64,
        surface: String,
    ) {
        let mut inputs = HashMap::new();
        inputs.insert("player_index".to_string(), player_index.to_string());
        inputs.insert("entity".to_string(), entity.clone());
        inputs.insert("position_x".to_string(), position_x.to_string());
        inputs.insert("position_y".to_string(), position_y.to_string());
        inputs.insert("surface".to_string(), surface.clone());

        let mut outputs = HashMap::new();
        outputs.insert("entity".to_string(), entity);
        outputs.insert("surface".to_string(), surface);

        self.log_call("on_player_mined_entity".to_string(), tick, inputs, outputs)
            .await;
    }

    /// Handles player crafted item event
    pub async fn handle_item_crafted(
        &self,
        tick: u64,
        player_index: u32,
        item: String,
        count: u32,
    ) {
        let mut inputs = HashMap::new();
        inputs.insert("player_index".to_string(), player_index.to_string());
        inputs.insert("item".to_string(), item.clone());
        inputs.insert("count".to_string(), count.to_string());

        let mut outputs = HashMap::new();
        outputs.insert("item".to_string(), item);
        outputs.insert("count".to_string(), count.to_string());

        self.log_call("on_player_crafted_item".to_string(), tick, inputs, outputs)
            .await;
    }

    /// Ends all active calls (used during session transitions)
    async fn end_all_calls(&self) {
        // First, collect all calls to end
        let calls_to_end: Vec<CallContext> = {
            let mut active_calls = self.active_calls.lock().await;
            let call_count = active_calls.len();

            if call_count > 0 {
                println!(
                    "🔷 Ending {} active Weave calls due to session change",
                    call_count
                );
                active_calls.drain().map(|(_, context)| context).collect()
            } else {
                Vec::new()
            }
        };

        // Now end each call without holding the lock
        for context in calls_to_end {
            println!(
                "🔷 Force-ending Weave call: '{}' session={} weave_id={}",
                context.call_id, context.session_id, context.call_id
            );

            // Force end the call with failure
            if let Err(e) = self
                .send_end_call(
                    context.call_id,
                    context.start_tick,
                    0,
                    HashMap::new(),
                    false,
                )
                .await
            {
                eprintln!("⚠️  Failed to force-end call: {}", e);
            }
        }
    }

    /// Returns the count of currently active calls
    pub async fn active_call_count(&self) -> usize {
        self.active_calls.lock().await.len()
    }

    /// Checks if a specific call is active
    pub async fn is_call_active(&self, call_id: &str) -> bool {
        self.active_calls.lock().await.contains_key(call_id)
    }

    /// Public method to explicitly close the current session (e.g., on shutdown)
    pub async fn shutdown(&self) {
        println!("🔷 Shutting down Weave manager...");
        self.end_all_calls().await;
        *self.current_session_id.lock().await = None;

        // Flush and shutdown client
        let client_guard = self.client.lock().await;
        if let Some(client) = client_guard.as_ref() {
            if let Err(e) = client.wait_idle().await {
                eprintln!("⚠️  Failed to wait for idle: {}", e);
            }
            if let Err(e) = client.shutdown().await {
                eprintln!("⚠️  Failed to shutdown client: {}", e);
            }
        }
        drop(client_guard);

        println!("🔷 Weave manager shutdown complete");
    }
}

impl Drop for WeaveManager {
    fn drop(&mut self) {
        // Note: We can't call async shutdown from Drop
        // The user should call shutdown() explicitly before dropping
        println!("⚠️  WeaveManager dropped - ensure shutdown() was called first");
    }
}

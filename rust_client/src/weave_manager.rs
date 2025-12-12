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
    metadata: HashMap<String, String>,
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
        metadata: HashMap<String, String>,
    ) {
        let session_id = self.current_session_id.lock().await.clone();

        match session_id {
            None => {
                eprintln!(
                    "⚠️  Cannot start Weave call '{}': no active session",
                    call_id
                );
            }
            Some(session_id) => {
                // Generate UUIDs
                let weave_call_id = Uuid::now_v7().to_string();
                let trace_id = Uuid::now_v7().to_string();

                let context = CallContext {
                    call_id: weave_call_id.clone(),
                    trace_id: trace_id.clone(),
                    session_id: session_id.clone(),
                    start_tick: tick,
                    metadata: metadata.clone(),
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
                    .send_start_call(weave_call_id, trace_id, operation, tick, metadata)
                    .await
                {
                    eprintln!("⚠️  Failed to send start call to Weave: {}", e);
                }
            }
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
        let session_id = self.current_session_id.lock().await.clone();

        match session_id {
            None => {
                eprintln!(
                    "⚠️  Cannot log Weave call '{}': no active session",
                    operation
                );
            }
            Some(session_id) => {
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
        }
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

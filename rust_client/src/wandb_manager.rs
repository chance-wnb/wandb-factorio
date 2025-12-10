use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use wandb;

/// A singleton service that manages WandB sessions for Factorio events.
/// Handles session initialization, metrics logging, and session cleanup.
/// Tracks all seen items to report zeros for inactive production/consumption.
pub struct WandbManager {
    current_run: Arc<Mutex<Option<wandb::run::Run>>>,
    current_session_id: Arc<Mutex<Option<String>>>,
    seen_production_items: Arc<Mutex<HashSet<String>>>,
    seen_consumption_items: Arc<Mutex<HashSet<String>>>,
}

impl WandbManager {
    /// Creates a new WandB manager instance
    pub fn new() -> Self {
        WandbManager {
            current_run: Arc::new(Mutex::new(None)),
            current_session_id: Arc::new(Mutex::new(None)),
            seen_production_items: Arc::new(Mutex::new(HashSet::new())),
            seen_consumption_items: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Handles a session_init event. Closes any existing session and starts a new one.
    pub fn handle_session_init(&self, session_id: String, tick: u64, level_name: String) {
        println!("📍 Session init received: {}", session_id);

        // Close existing session if any
        self.finish_current_session();

        // Clear seen items for new session
        self.seen_production_items.lock().unwrap().clear();
        self.seen_consumption_items.lock().unwrap().clear();

        // Start new session
        self.start_new_session(session_id, tick, level_name);
    }

    /// Handles a stats event. Ensures a session exists and logs metrics.
    pub fn handle_stats_event(
        &self,
        session_id: String,
        cycle: u64,
        tick: u64,
        products_production: HashMap<String, f64>,
        materials_consumption: HashMap<String, f64>,
    ) {
        // Check if we need to start a new session
        let current_session = self.current_session_id.lock().unwrap().clone();

        match current_session {
            None => {
                // No active session - create one immediately
                println!("⚠️  Stats received without active session. Creating session: {}", session_id);
                self.start_new_session(session_id.clone(), tick, "unknown".to_string());
            }
            Some(ref current_id) if current_id != &session_id => {
                // Session ID mismatch - start new session
                println!("⚠️  Session ID mismatch. Switching from {} to {}", current_id, session_id);
                self.finish_current_session();
                self.start_new_session(session_id.clone(), tick, "unknown".to_string());
            }
            _ => {
                // Session matches, continue
            }
        }

        // Log metrics
        self.log_metrics(cycle, products_production, materials_consumption);
    }

    /// Starts a new WandB session
    fn start_new_session(&self, session_id: String, tick: u64, level_name: String) {
        // Generate run name with random seed
        let random_seed: u32 = rand::random();
        let run_name = format!("{}_{}", session_id, random_seed);

        println!("🚀 Starting new WandB run: {}", run_name);

        // Configure WandB settings
        let project = Some("factorio-experiments".to_string());
        let mut settings = wandb::settings::Settings::default();
        settings.proto.entity = Some("wandb".to_string());
        settings.proto.run_name = Some(run_name.clone());

        // Initialize run
        match wandb::init(project, Some(settings)) {
            Ok(run) => {
                // Store the run
                *self.current_run.lock().unwrap() = Some(run);
                *self.current_session_id.lock().unwrap() = Some(session_id);

                println!("✅ WandB run initialized successfully");
            }
            Err(e) => {
                eprintln!("❌ Failed to initialize WandB run: {:?}", e);
            }
        }
    }

    /// Logs metrics to the current WandB session
    fn log_metrics(
        &self,
        cycle: u64,
        products_production: HashMap<String, f64>,
        materials_consumption: HashMap<String, f64>,
    ) {
        let run_guard = self.current_run.lock().unwrap();

        if let Some(ref run) = *run_guard {
            // Update seen items and build complete metrics with zeros for inactive items
            let mut seen_prod = self.seen_production_items.lock().unwrap();
            let mut seen_cons = self.seen_consumption_items.lock().unwrap();

            // Add new items to the tracking sets
            for item_name in products_production.keys() {
                seen_prod.insert(item_name.clone());
            }
            for item_name in materials_consumption.keys() {
                seen_cons.insert(item_name.clone());
            }

            let mut metrics = HashMap::new();

            // Add production metrics (with zeros for inactive items)
            for item_name in seen_prod.iter() {
                let value = products_production.get(item_name).copied().unwrap_or(0.0);
                let key = format!("production/{}", item_name);
                metrics.insert(key, wandb::run::Value::Float(value));
            }

            // Add consumption metrics (with zeros for inactive items)
            for item_name in seen_cons.iter() {
                let value = materials_consumption.get(item_name).copied().unwrap_or(0.0);
                let key = format!("consumption/{}", item_name);
                metrics.insert(key, wandb::run::Value::Float(value));
            }

            let total_metrics = seen_prod.len() + seen_cons.len();
            let active_prod = products_production.len();
            let active_cons = materials_consumption.len();

            // Log metrics with step
            if !metrics.is_empty() {
                run.log_with_step(metrics, Some(cycle as i64));
                println!(
                    "📊 Logged {} total metrics ({} active: {}p/{}c) at step {}",
                    total_metrics, active_prod + active_cons, active_prod, active_cons, cycle
                );
            }
        } else {
            eprintln!("⚠️  Attempted to log metrics but no active run exists");
        }
    }

    /// Finishes the current WandB session if one exists
    fn finish_current_session(&self) {
        let mut run_guard = self.current_run.lock().unwrap();
        let session_id = self.current_session_id.lock().unwrap().clone();

        if let Some(mut run) = run_guard.take() {
            println!("🏁 Finishing WandB run for session: {:?}", session_id);
            run.finish();
            *self.current_session_id.lock().unwrap() = None;
            println!("✅ WandB run finished");
        }
    }

    /// Public method to explicitly finish the current session (e.g., on shutdown)
    pub fn shutdown(&self) {
        println!("🔚 Shutting down WandB manager...");
        self.finish_current_session();
    }
}

impl Drop for WandbManager {
    fn drop(&mut self) {
        // Ensure session is closed when manager is dropped
        self.finish_current_session();
    }
}

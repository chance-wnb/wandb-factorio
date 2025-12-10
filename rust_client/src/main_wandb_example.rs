// Original W&B example for reference
use std::collections::HashMap;
use wandb;

fn main() {
    println!("Starting Factorio Rust Client with W&B tracking...");

    // Configure W&B settings
    let project = Some("factorio-experiments".to_string());
    let mut settings = wandb::settings::Settings::default();

    // Set entity (username or team)
    settings.proto.entity = Some("wandb".to_string());

    // Initialize a W&B run
    let mut run = wandb::init(project, Some(settings)).unwrap();
    println!("W&B run initialized!");

    // Log some example metrics
    let mut metrics = HashMap::new();
    metrics.insert("science_packs_per_minute".to_string(), wandb::run::Value::Float(450.0));
    metrics.insert("power_consumption_mw".to_string(), wandb::run::Value::Float(125.5));
    metrics.insert("pollution_per_minute".to_string(), wandb::run::Value::Float(23.8));

    run.log(metrics);
    println!("Metrics logged!");

    // Finish the run
    run.finish();
    println!("W&B run finished!");
}

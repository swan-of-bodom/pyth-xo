use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    pub price_feed_id: String,
    pub symbol: String,
    pub deviation_threshold: f64,
    pub heartbeat_seconds: u64,
    pub networks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc_url: String,
    pub pyth_contract: String,
    #[serde(skip_deserializing)]
    pub private_key: String,
    pub native_feed_id: String,
    pub block_explorer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub networks: Vec<NetworkConfig>,
    pub feeds: Vec<FeedConfig>,
    pub pyth_hermes_url: String,
    pub poll_interval_seconds: u64,
}

pub fn load_config() -> Result<Config> {
    let config_str = std::fs::read_to_string("config.json")
        .context("No config found. Create a config.json based on config.example.json")?;

    let mut config: Config =
        serde_json::from_str(&config_str).context("Failed to parse config.json")?;

    let private_key =
        std::env::var("PRIVATE_KEY").context("PRIVATE_KEY environment variable not set")?;

    // Set the same private key for all networks
    for network in &mut config.networks {
        network.private_key = private_key.clone();
    }

    Ok(config)
}

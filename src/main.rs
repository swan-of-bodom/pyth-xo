/*!
Provides an automated service for updating pyth feeds to then be used as push oracles.
Not the way PYTH was intended, but the way we ended up ¯\_(ツ)_/¯
*/

mod config;
mod contract;
mod pyth_api;
mod updater;
mod utils;

use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let config = config::load_config().context("Failed to load config")?;
    let mut updater = updater::PythUpdater::new(config);
    updater.run().await?;

    Ok(())
}

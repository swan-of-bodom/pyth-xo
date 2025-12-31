use crate::config::{Config, FeedConfig, NetworkConfig};
use crate::contract::IPythContract;
use crate::pyth_api;
use crate::utils;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, FixedBytes},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::{error, info};
use std::{collections::HashMap, str::FromStr, time::Duration};

pub struct PythUpdater {
    config: Config,
    http_client: reqwest::Client,
    feed_states: HashMap<String, FeedState>,
}

#[derive(Debug, Clone)]
struct FeedState {
    last_price: f64,
    last_on_chain_update: DateTime<Utc>,
}

impl PythUpdater {
    pub fn new(config: Config) -> Self {
        let mut feed_states = HashMap::new();

        for feed in &config.feeds {
            for network_name in &feed.networks {
                let state = FeedState { last_price: 0.0, last_on_chain_update: Utc::now() };
                let key = utils::state_key(&feed.price_feed_id, network_name);
                feed_states.insert(key, state);
            }
        }

        Self { config, http_client: reqwest::Client::new(), feed_states }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Pyth Updater service");
        info!(
            "Monitoring {} feeds across {} networks",
            self.config.feeds.len(),
            self.config.networks.len()
        );

        if let Err(e) = self.initialize_feed_states().await {
            error!("Failed to initialize feed states from on-chain: {}", e);
        }

        loop {
            if let Err(e) = self.update_cycle().await {
                error!("Error in update cycle: {}", e);
            }

            tokio::time::sleep(Duration::from_secs(self.config.poll_interval_seconds)).await;
        }
    }

    async fn initialize_feed_states(&mut self) -> Result<()> {
        if self.config.networks.is_empty() {
            return Ok(());
        }

        for network in &self.config.networks {
            info!("Initializing feed states from {} on-chain data", network.name);

            let provider = ProviderBuilder::new().on_http(network.rpc_url.parse()?);
            let pyth_address = Address::from_str(&network.pyth_contract)?;
            let contract = IPythContract::new(pyth_address, &provider);

            for feed in &self.config.feeds {
                if !feed.networks.contains(&network.name) {
                    continue;
                }

                let feed_id_bytes = hex::decode(&feed.price_feed_id)?;
                let bytes32 = FixedBytes::<32>::from_slice(&feed_id_bytes);
                let state_key = utils::state_key(&feed.price_feed_id, &network.name);

                match contract.getPriceUnsafe(bytes32).call().await {
                    Ok(result) => {
                        let price = result.price;
                        let expo = result.expo;
                        let publish_time = result.publishTime;
                        let actual_price = (price as f64) * 10_f64.powi(expo);
                        let publish_datetime =
                            DateTime::from_timestamp(publish_time.try_into().unwrap_or(0), 0)
                                .unwrap_or_else(|| Utc::now());

                        if let Some(state) = self.feed_states.get_mut(&state_key) {
                            state.last_price = actual_price;
                            state.last_on_chain_update = publish_datetime;
                            info!(
                                "Initialized {} on {} from on-chain: ${:.2} (published {}s ago)",
                                feed.symbol,
                                network.name,
                                actual_price,
                                (Utc::now() - publish_datetime).num_seconds()
                            );
                        }
                    }
                    Err(e) => {
                        info!(
                            "No on-chain price found for {} on {} ({}), will update on first cycle",
                            feed.symbol, network.name, e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn update_cycle(&mut self) -> Result<()> {
        let feed_ids: Vec<String> =
            self.config.feeds.iter().map(|f| f.price_feed_id.clone()).collect();

        info!("----------------------------------------");
        info!("Fetching prices from Pyth Network");
        info!("----------------------------------------");

        let response =
            pyth_api::fetch_prices(&self.http_client, &self.config.pyth_hermes_url, &feed_ids)
                .await?;

        let mut prices: HashMap<String, &pyth_api::ParsedPrice> = HashMap::new();
        for price in &response.parsed {
            let id = price.id.trim_start_matches("0x").to_string();
            prices.insert(id, price);
        }

        let mut updates_by_network: HashMap<String, Vec<String>> = HashMap::new();

        for network in &self.config.networks {
            for feed in &self.config.feeds {
                if !feed.networks.contains(&network.name) {
                    continue;
                }

                if let Some(price_data) = prices.get(&feed.price_feed_id) {
                    let current_price = pyth_api::parse_price(price_data)?;
                    let state_key = utils::state_key(&feed.price_feed_id, &network.name);
                    let state = self.feed_states.get(&state_key).unwrap();

                    let deviation_pct = if state.last_price > 0.0 {
                        ((current_price - state.last_price) / state.last_price).abs() * 100.0
                    } else {
                        0.0
                    };

                    let should_update = self.should_update_feed(feed, state, current_price)?;

                    let time_since_publish = Utc::now() - state.last_on_chain_update;
                    let seconds_ago = time_since_publish.num_seconds();
                    let time_ago = utils::format_duration(seconds_ago);

                    let is_stablecoin = feed.deviation_threshold <= 0.1;

                    if should_update {
                        if is_stablecoin {
                            info!(
                                "✓ {:<12} on {:<10} | Price: ${:>10.4} | Last: ${:>10.4} | Deviation: {:>7.4}% | Published: {:<8} ago | UPDATING",
                                feed.symbol, network.name, current_price, state.last_price, deviation_pct, time_ago
                            );
                        } else {
                            info!(
                                "✓ {:<12} on {:<10} | Price: ${:>10.2} | Last: ${:>10.2} | Deviation: {:>7.4}% | Published: {:<8} ago | UPDATING",
                                feed.symbol, network.name, current_price, state.last_price, deviation_pct, time_ago
                            );
                        }
                        updates_by_network
                            .entry(network.name.clone())
                            .or_insert_with(Vec::new)
                            .push(feed.price_feed_id.clone());
                    } else {
                        if is_stablecoin {
                            info!(
                                "○ {:<12} on {:<10} | Price: ${:>10.4} | Last: ${:>10.4} | Deviation: {:>7.4}% | Published: {:<8} ago | Skipping (threshold: {:.2}%)",
                                feed.symbol, network.name, current_price, state.last_price, deviation_pct, time_ago, feed.deviation_threshold
                            );
                        } else {
                            info!(
                                "○ {:<12} on {:<10} | Price: ${:>10.2} | Last: ${:>10.2} | Deviation: {:>7.4}% | Published: {:<8} ago | Skipping (threshold: {:.2}%)",
                                feed.symbol, network.name, current_price, state.last_price, deviation_pct, time_ago, feed.deviation_threshold
                            );
                        }
                    }
                }
            }
        }

        for network in &self.config.networks {
            if let Some(feeds_to_update_on_network) = updates_by_network.get(&network.name) {
                info!("Updating {} feeds on {}", feeds_to_update_on_network.len(), network.name);

                if let Err(e) =
                    self.update_feeds_on_network(network, feeds_to_update_on_network).await
                {
                    error!("Failed to update feeds on {}: {}", network.name, e);
                } else {
                    let provider = ProviderBuilder::new().on_http(network.rpc_url.parse()?);
                    let pyth_address = Address::from_str(&network.pyth_contract)?;
                    let contract = IPythContract::new(pyth_address, &provider);

                    for feed_id in feeds_to_update_on_network {
                        if let Some(price_data) = prices.get(feed_id) {
                            let current_price = pyth_api::parse_price(price_data)?;
                            let feed_id_bytes = hex::decode(feed_id)?;
                            let bytes32 = FixedBytes::<32>::from_slice(&feed_id_bytes);

                            match contract.getPriceUnsafe(bytes32).call().await {
                                Ok(result) => {
                                    let on_chain_publish_time: u64 =
                                        result.publishTime.try_into().unwrap_or(0);
                                    let publish_datetime =
                                        DateTime::from_timestamp(on_chain_publish_time as i64, 0)
                                            .unwrap_or_else(|| Utc::now());

                                    let state_key = utils::state_key(feed_id, &network.name);
                                    if let Some(state) = self.feed_states.get_mut(&state_key) {
                                        state.last_price = current_price;
                                        state.last_on_chain_update = publish_datetime;
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to read on-chain publish time for {}: {}",
                                        feed_id, e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn should_update_feed(
        &self,
        feed: &FeedConfig,
        state: &FeedState,
        current_price: f64,
    ) -> Result<bool> {
        if state.last_price == 0.0 {
            info!("Initial update for {} - no previous price on-chain", feed.symbol);
            return Ok(true);
        }

        let time_since_update = Utc::now() - state.last_on_chain_update;
        if time_since_update.num_seconds() >= feed.heartbeat_seconds as i64 {
            info!(
                "Heartbeat triggered for {} ({}s >= {}s)",
                feed.symbol,
                time_since_update.num_seconds(),
                feed.heartbeat_seconds
            );
            return Ok(true);
        }

        let price_change_pct =
            ((current_price - state.last_price) / state.last_price).abs() * 100.0;
        if price_change_pct >= feed.deviation_threshold {
            return Ok(true);
        }

        Ok(false)
    }

    async fn update_feeds_on_network(
        &self,
        network: &NetworkConfig,
        feed_ids: &[String],
    ) -> Result<()> {
        let update_data = pyth_api::fetch_price_update_data(
            &self.http_client,
            &self.config.pyth_hermes_url,
            feed_ids,
        )
        .await?;

        let signer = PrivateKeySigner::from_str(&network.private_key)?;
        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(network.rpc_url.parse()?);

        let pyth_address = Address::from_str(&network.pyth_contract)?;
        let contract = IPythContract::new(pyth_address, &provider);

        let update_fee_result = contract
            .getUpdateFee(update_data.clone())
            .call()
            .await
            .context("Failed to get update fee")?;
        let update_fee = update_fee_result.feeAmount;

        let gas_price = provider.get_gas_price().await.context("Failed to get gas price")?;

        let tx = contract.updatePriceFeeds(update_data).value(update_fee).gas_price(gas_price);

        let pending_tx = tx.send().await.context("Failed to send update transaction")?;

        let receipt =
            pending_tx.get_receipt().await.context("Failed to get transaction receipt")?;

        let tx_fee_wei = receipt.gas_used * gas_price;
        let tx_fee_native = tx_fee_wei as f64 / 1e18;

        let native_price_usd = self.get_native_token_price(network).await.unwrap_or(0.0);
        let tx_fee_usd =
            if native_price_usd > 0.0 { tx_fee_native * native_price_usd } else { 0.0 };

        let price_info =
            if native_price_usd > 0.0 { format!("(${:.4})", tx_fee_usd) } else { String::new() };

        info!(
            "Feeds updated on {} at block {} | Tx: {}/tx/{:?} | Gas used: {} | Tx fee: {:.6} native {}",
            network.name,
            receipt.block_number.unwrap_or_default(),
            network.block_explorer,
            receipt.transaction_hash,
            receipt.gas_used,
            tx_fee_native,
            price_info
        );

        Ok(())
    }

    async fn get_native_token_price(&self, network: &NetworkConfig) -> Result<f64> {
        let response = pyth_api::fetch_prices(
            &self.http_client,
            &self.config.pyth_hermes_url,
            &[network.native_feed_id.clone()],
        )
        .await?;

        if let Some(price_data) = response.parsed.first() {
            let actual_price = pyth_api::parse_price(price_data)?;
            Ok(actual_price)
        } else {
            Err(anyhow::anyhow!("No price data found for native token"))
        }
    }
}

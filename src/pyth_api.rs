use alloy::primitives::Bytes;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PythPriceResponse {
    pub parsed: Vec<ParsedPrice>,
}

#[derive(Debug, Deserialize)]
pub struct ParsedPrice {
    pub id: String,
    pub price: PriceData,
}

#[derive(Debug, Deserialize)]
pub struct PriceData {
    pub price: String,
    pub expo: i32,
}

pub async fn fetch_prices(
    http_client: &reqwest::Client,
    hermes_url: &str,
    feed_ids: &[String],
) -> Result<PythPriceResponse> {
    let feed_ids_with_prefix: Vec<String> = feed_ids.iter().map(|f| format!("0x{}", f)).collect();
    let url = format!(
        "{}/v2/updates/price/latest?ids[]={}",
        hermes_url,
        feed_ids_with_prefix.join("&ids[]=")
    );

    let response = http_client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch Pyth prices")?
        .json::<PythPriceResponse>()
        .await
        .context("Failed to parse Pyth response")?;

    Ok(response)
}

pub async fn fetch_price_update_data(
    http_client: &reqwest::Client,
    hermes_url: &str,
    feed_ids: &[String],
) -> Result<Vec<Bytes>> {
    let feed_ids_with_prefix: Vec<String> = feed_ids.iter().map(|id| format!("0x{}", id)).collect();
    let url = format!(
        "{}/v2/updates/price/latest?ids[]={}",
        hermes_url,
        feed_ids_with_prefix.join("&ids[]=")
    );

    #[derive(Deserialize)]
    struct UpdateResponse {
        binary: BinaryData,
    }

    #[derive(Deserialize)]
    struct BinaryData {
        data: Vec<String>,
    }

    let response = http_client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch price update data")?
        .json::<UpdateResponse>()
        .await
        .context("Failed to parse update response")?;

    let update_data: Vec<Bytes> = response
        .binary
        .data
        .iter()
        .map(|hex_str| {
            let hex = hex_str.trim_start_matches("0x");
            Bytes::from(hex::decode(hex).expect("Failed to decode hex"))
        })
        .collect();

    Ok(update_data)
}

pub fn parse_price(price_data: &ParsedPrice) -> Result<f64> {
    let price: i64 = price_data.price.price.parse().context("Failed to parse price")?;
    let expo = price_data.price.expo;
    let actual_price = (price as f64) * 10_f64.powi(expo);
    Ok(actual_price)
}

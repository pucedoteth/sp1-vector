use std::env;
use std::str::FromStr;

use alloy::primitives::B256;
use alloy::providers::{Provider, RootProvider};
use alloy::transports::http::{Client, Http};
use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Get the gas limit associated with the chain id. Note: These values have been found through
/// trial and error and can be configured.
pub fn get_gas_limit(chain_id: u64) -> u128 {
    if chain_id == 42161 || chain_id == 421614 {
        25_000_000
    } else {
        1_500_000
    }
}

/// Get the gas fee cap associated with the chain id, using the provider to get the gas price. Note:
/// These values have been found through trial and error and can be configured.
pub async fn get_fee_cap(chain_id: u64, provider: &RootProvider<Http<Client>>) -> u128 {
    // Base percentage multiplier for the gas fee.
    let mut multiplier = 20;

    // Double the estimated gas fee cap for the testnets.
    if chain_id == 17000 || chain_id == 421614 || chain_id == 11155111 || chain_id == 84532 {
        multiplier = 100
    }

    // Get the gas price.
    let gas_price = provider.get_gas_price().await.unwrap();

    // Calculate the fee cap.
    (gas_price * (100 + multiplier)) / 100
}

#[derive(Serialize, Deserialize)]
pub enum KMSRelayStatus {
    Unknown = 0,
    Relayed = 1,
    PreflightError = 2,
    SimulationFailure = 3,
    RelayFailure = 4,
    InvalidAuthenticationToken = 5,
}

/// Relay request arguments for KMS relayer.
#[derive(Debug, Deserialize, Serialize)]
pub struct KMSRelayRequest {
    pub chain_id: u64,
    pub address: String,
    pub calldata: String,
    pub platform_request: bool,
}

/// Response from KMS relayer.
#[derive(Debug, Deserialize, Serialize)]
pub struct KMSRelayResponse {
    pub transaction_hash: Option<String>,
    pub message: Option<String>,
    pub status: u32,
}

/// Relay a transaction with KMS and return the transaction hash with retries.
/// Requires SECURE_RELAYER_ENDPOINT and SECURE_RELAYER_API_KEY to be set in the environment.
pub async fn relay_with_kms(args: &KMSRelayRequest, num_retries: u32) -> Result<B256> {
    let mut num_retries = num_retries;
    while num_retries > 0 {
        let response = send_kms_relay_request(args).await?;
        if response.status == KMSRelayStatus::Relayed as u32 {
            return Ok(B256::from_str(&response.transaction_hash.unwrap())?);
        }
        num_retries -= 1;
    }
    Err(anyhow::anyhow!("Transaction reverted!"))
}

/// Send a KMS relay request and get the response.
/// Requires SECURE_RELAYER_ENDPOINT and SECURE_RELAYER_API_KEY to be set in the environment.
async fn send_kms_relay_request(args: &KMSRelayRequest) -> Result<KMSRelayResponse> {
    info!("Sending KMS relay request: {:?}", args);
    // Read relayer endpoint from env
    let relayer_endpoint = env::var("SECURE_RELAYER_ENDPOINT").unwrap();
    let api_key = env::var("SECURE_RELAYER_API_KEY").unwrap();

    let client = Client::new();
    let response = client
        .post(format!("{}/relay", relayer_endpoint))
        .bearer_auth(api_key)
        .json(&json!(args))
        .send()
        .await?;
    let response_body = response.text().await?;
    let response_json: KMSRelayResponse = serde_json::from_str(&response_body)?;
    Ok(response_json)
}

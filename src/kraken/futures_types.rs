use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::kraken::error::KrakenError;
use crate::kraken::types::{ApiObject, ApiValue};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FuturesResponse<T> {
    pub result: Option<String>,
    pub error: Option<String>,
    #[serde(flatten)]
    pub payload: T,
}

pub fn check_response<T>(value: FuturesResponse<T>) -> Result<T, KrakenError> {
    if let Some(result) = value.result.as_deref() {
        if result.eq_ignore_ascii_case("error") {
            return Err(KrakenError::Api(
                value
                    .error
                    .unwrap_or_else(|| "Unknown futures API error".to_string()),
            ));
        }
    }
    if let Some(error) = value.error {
        if !error.is_empty() {
            return Err(KrakenError::Api(error));
        }
    }
    Ok(value.payload)
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesInstrument {
    pub symbol: Option<String>,
    pub tradeable: Option<bool>,
    pub has_funding: Option<bool>,
    pub tick_size: Option<String>,
    pub contract_size: Option<String>,
    pub underlying: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesInstrumentsResult {
    #[serde(default)]
    pub instruments: Vec<FuturesInstrument>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesInstrumentStatusResult {
    pub instrument: Option<String>,
    pub status: Option<String>,
    pub tradeable: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesTickerEntry {
    pub symbol: Option<String>,
    pub mark_price: Option<String>,
    pub last: Option<String>,
    pub bid: Option<String>,
    pub ask: Option<String>,
    pub volume: Option<String>,
    pub open_interest: Option<String>,
    pub funding_rate: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesTickersResult {
    #[serde(default)]
    pub tickers: Vec<FuturesTickerEntry>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesTickerResult {
    pub ticker: Option<FuturesTickerEntry>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesBookLevel {
    pub price: Option<String>,
    pub qty: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesOrderbookResult {
    pub symbol: Option<String>,
    #[serde(default)]
    pub bids: Vec<FuturesBookLevel>,
    #[serde(default)]
    pub asks: Vec<FuturesBookLevel>,
    pub timestamp: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesTradeHistoryResult {
    #[serde(default)]
    pub history: Vec<ApiObject>,
    #[serde(default)]
    pub trades: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesFeeSchedulesResult {
    #[serde(default)]
    pub fee_schedules: Vec<ApiObject>,
    #[serde(default, rename = "feeSchedules")]
    pub fee_schedules_alt: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesFundingRatesResult {
    #[serde(default)]
    pub rates: Vec<ApiObject>,
    #[serde(default)]
    pub historical_funding_rates: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesAccountsResult {
    #[serde(default)]
    pub accounts: Vec<ApiObject>,
    #[serde(default)]
    pub cash_accounts: Vec<ApiObject>,
    #[serde(default)]
    pub margin_accounts: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesOpenOrdersResult {
    #[serde(default)]
    pub open_orders: Vec<ApiObject>,
    #[serde(default)]
    pub orders: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesOpenPositionsResult {
    #[serde(default)]
    pub open_positions: Vec<ApiObject>,
    #[serde(default)]
    pub positions: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesFillsResult {
    #[serde(default)]
    pub fills: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesTransfersResult {
    #[serde(default)]
    pub transfers: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesOrderStatusResult {
    #[serde(default)]
    pub orders: Vec<ApiObject>,
    #[serde(default)]
    pub statuses: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesLeverageResult {
    pub symbol: Option<String>,
    pub max_leverage: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesPnlResult {
    pub symbol: Option<String>,
    pub pnl_preference: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesSendOrderResult {
    pub order_id: Option<String>,
    pub cli_ord_id: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesCancelResult {
    pub status: Option<String>,
    pub cancelled: Option<u64>,
    #[serde(default)]
    pub order_ids: Vec<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesBatchResult {
    pub status: Option<String>,
    #[serde(default)]
    pub results: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesTransferResult {
    pub transfer_id: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

pub type FuturesWithdrawalResult = FuturesTransferResult;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FuturesBatchInstruction {
    pub order: String,
    #[serde(rename = "orderType")]
    pub order_type: Option<String>,
    pub symbol: Option<String>,
    pub side: Option<String>,
    pub size: Option<ApiValue>,
    #[serde(rename = "limitPrice")]
    pub limit_price: Option<ApiValue>,
    #[serde(rename = "stopPrice")]
    pub stop_price: Option<ApiValue>,
    #[serde(rename = "order_id")]
    pub order_id: Option<String>,
    #[serde(rename = "cli_ord_id")]
    pub cli_ord_id: Option<String>,
}

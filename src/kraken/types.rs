use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generic Kraken API response wrapper.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KrakenResponse<T> {
    pub error: Vec<String>,
    pub result: Option<T>,
}

/// Strongly-typed JSON value used for flexible Kraken fields.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum ApiValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<ApiValue>),
    Object(HashMap<String, ApiValue>),
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ApiObject {
    #[serde(flatten)]
    pub fields: HashMap<String, ApiValue>,
}

// === Public market/account reference types ===

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerTimeResult {
    pub unixtime: u64,
    pub rfc1123: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemStatusResult {
    pub status: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AssetInfo {
    pub aclass: Option<String>,
    pub altname: Option<String>,
    pub decimals: Option<u32>,
    pub display_decimals: Option<u32>,
    pub status: Option<String>,
}

pub type AssetsResult = HashMap<String, AssetInfo>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AssetPairInfo {
    pub altname: Option<String>,
    pub wsname: Option<String>,
    pub aclass_base: Option<String>,
    pub base: Option<String>,
    pub aclass_quote: Option<String>,
    pub quote: Option<String>,
    pub cost_decimals: Option<u32>,
    pub pair_decimals: Option<u32>,
    pub lot_decimals: Option<u32>,
    pub lot_multiplier: Option<u32>,
    pub leverage_buy: Option<Vec<u32>>,
    pub leverage_sell: Option<Vec<u32>>,
    pub fees: Option<Vec<Vec<f64>>>,
    pub fees_maker: Option<Vec<Vec<f64>>>,
    pub fee_volume_currency: Option<String>,
    pub margin_call: Option<u32>,
    pub margin_stop: Option<u32>,
    pub ordermin: Option<String>,
    pub costmin: Option<String>,
    pub tick_size: Option<String>,
    pub status: Option<String>,
}

pub type AssetPairsResult = HashMap<String, AssetPairInfo>;

// === Market types ===

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TickerInfo {
    /// Ask [price, whole_lot_volume, lot_volume]
    pub a: Vec<String>,
    /// Bid [price, whole_lot_volume, lot_volume]
    pub b: Vec<String>,
    /// Last trade [price, lot_volume]
    pub c: Vec<String>,
    /// Volume [today, last_24h]
    pub v: Vec<String>,
    /// Volume weighted average price [today, last_24h]
    pub p: Vec<String>,
    /// Number of trades [today, last_24h]
    pub t: Vec<u64>,
    /// Low [today, last_24h]
    pub l: Vec<String>,
    /// High [today, last_24h]
    pub h: Vec<String>,
    /// Opening price today
    pub o: String,
}

pub type TickerResult = HashMap<String, TickerInfo>;

/// Orderbook — parsed manually from Kraken's array format.
#[derive(Debug, Clone, Serialize)]
pub struct OrderBook {
    pub asks: Vec<OrderBookLevel>,
    pub bids: Vec<OrderBookLevel>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderBookLevel {
    pub price: String,
    pub volume: String,
    pub timestamp: u64,
}

pub type OhlcResult = HashMap<String, ApiValue>;
pub type RecentTradesResult = HashMap<String, ApiValue>;
pub type SpreadResult = HashMap<String, ApiValue>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GroupedBookLevel {
    pub price: Option<String>,
    pub qty: Option<String>,
    pub volume: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GroupedBookResult {
    pub pair: Option<String>,
    pub grouping: Option<u32>,
    #[serde(default)]
    pub bids: Vec<GroupedBookLevel>,
    #[serde(default)]
    pub asks: Vec<GroupedBookLevel>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PreTradeResult {
    pub symbol: Option<String>,
    #[serde(default)]
    pub bids: Vec<ApiObject>,
    #[serde(default)]
    pub asks: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PostTradeResult {
    #[serde(default)]
    pub trades: Vec<ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Level3Order {
    pub order_id: Option<String>,
    pub price: Option<String>,
    pub qty: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Level3Book {
    #[serde(default)]
    pub asks: Vec<Level3Order>,
    #[serde(default)]
    pub bids: Vec<Level3Order>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Level3Result {
    Direct(Level3Book),
    ByPair(HashMap<String, Level3Book>),
}

// === Account types ===

pub type BalanceResult = HashMap<String, String>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CreditLinesResult {
    #[serde(default)]
    pub asset_details: HashMap<String, ApiObject>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ExtendedBalanceEntry {
    pub balance: Option<String>,
    pub credit: Option<String>,
    pub credit_used: Option<String>,
    pub hold_trade: Option<String>,
}

pub type ExtendedBalanceResult = HashMap<String, ExtendedBalanceEntry>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TradeBalanceResult {
    pub eb: Option<String>,
    pub tb: Option<String>,
    pub m: Option<String>,
    pub n: Option<String>,
    pub c: Option<String>,
    pub v: Option<String>,
    pub e: Option<String>,
    pub mf: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradeInfo {
    pub ordertxid: Option<String>,
    pub pair: Option<String>,
    pub time: Option<f64>,
    #[serde(rename = "type")]
    pub trade_type: Option<String>,
    pub ordertype: Option<String>,
    pub price: Option<String>,
    pub cost: Option<String>,
    pub fee: Option<String>,
    pub vol: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradesHistoryResult {
    pub trades: Option<HashMap<String, TradeInfo>>,
    pub count: Option<u64>,
}

pub type TradesInfoResult = HashMap<String, TradeInfo>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OrderInfo {
    pub status: Option<String>,
    pub opentm: Option<f64>,
    pub descr: Option<OrderDescr>,
    pub vol: Option<String>,
    pub vol_exec: Option<String>,
    pub cost: Option<String>,
    pub fee: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OrderDescr {
    pub pair: Option<String>,
    #[serde(rename = "type")]
    pub order_type: Option<String>,
    pub ordertype: Option<String>,
    pub price: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OpenOrdersResult {
    pub open: Option<HashMap<String, OrderInfo>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ClosedOrdersResult {
    pub closed: Option<HashMap<String, OrderInfo>>,
    pub count: Option<u64>,
}

pub type QueryOrdersResult = HashMap<String, OrderInfo>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PositionInfo {
    pub ordertxid: Option<String>,
    pub pair: Option<String>,
    #[serde(rename = "type")]
    pub position_type: Option<String>,
    pub vol: Option<String>,
    pub vol_closed: Option<String>,
    pub cost: Option<String>,
    pub fee: Option<String>,
    pub margin: Option<String>,
    pub value: Option<String>,
    pub net: Option<String>,
}

pub type OpenPositionsResult = HashMap<String, PositionInfo>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LedgerEntry {
    pub aclass: Option<String>,
    pub asset: Option<String>,
    pub amount: Option<String>,
    pub balance: Option<String>,
    pub fee: Option<String>,
    pub refid: Option<String>,
    pub time: Option<f64>,
    #[serde(rename = "type")]
    pub ledger_type: Option<String>,
    pub subtype: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LedgerResult {
    pub ledger: Option<HashMap<String, LedgerEntry>>,
    pub count: Option<u64>,
}

pub type QueryLedgerResult = HashMap<String, LedgerEntry>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TradeVolumeFeeInfo {
    pub fee: Option<String>,
    pub minfee: Option<String>,
    pub maxfee: Option<String>,
    pub nextfee: Option<String>,
    pub tiervolume: Option<String>,
    pub nextvolume: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TradeVolumeResult {
    pub currency: Option<String>,
    pub volume: Option<String>,
    #[serde(default)]
    pub fees: HashMap<String, TradeVolumeFeeInfo>,
    #[serde(default)]
    pub fees_maker: HashMap<String, TradeVolumeFeeInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AddExportResult {
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ExportStatusEntry {
    pub id: Option<String>,
    pub descr: Option<String>,
    #[serde(rename = "type")]
    pub report_type: Option<String>,
    pub subtype: Option<String>,
    pub status: Option<String>,
    pub fields: Option<String>,
    pub createdtm: Option<String>,
    pub starttm: Option<String>,
    pub endtm: Option<String>,
    pub completedtm: Option<String>,
    pub datastarttm: Option<String>,
    pub dataendtm: Option<String>,
    pub asset: Option<String>,
    pub format: Option<String>,
}

pub type ExportStatusResult = Vec<ExportStatusEntry>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RemoveExportResult {
    pub delete: Option<bool>,
    pub cancel: Option<bool>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OrderAmendEntry {
    #[serde(flatten)]
    pub fields: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OrderAmendsResult {
    #[serde(default)]
    pub amends: Vec<OrderAmendEntry>,
    pub count: Option<u64>,
}

// === Trading types ===

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddOrderResult {
    pub descr: Option<AddOrderDescr>,
    pub txid: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddOrderDescr {
    pub order: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelOrderResult {
    pub count: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AddOrderBatchResult {
    #[serde(default)]
    pub orders: Vec<ApiObject>,
    pub descr: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AmendOrderResult {
    pub txid: Option<String>,
    pub amend_id: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EditOrderResult {
    pub status: Option<String>,
    pub originaltxid: Option<String>,
    pub txid: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CancelAllAfterResult {
    pub current_time: Option<String>,
    pub trigger_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CancelOrderBatchResult {
    #[serde(default)]
    pub orders: Vec<ApiObject>,
    pub count: Option<u64>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AddOrderBatchOrder {
    pub ordertype: String,
    #[serde(rename = "type")]
    pub order_side: String,
    pub volume: String,
    pub price: Option<String>,
    pub price2: Option<String>,
    pub leverage: Option<String>,
    pub oflags: Option<String>,
    pub timeinforce: Option<String>,
    pub starttm: Option<String>,
    pub expiretm: Option<String>,
    pub cl_ord_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CancelOrderBatchItem {
    pub txid: Option<String>,
    pub userref: Option<u64>,
    pub cl_ord_id: Option<String>,
}

// === Funding types ===

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DepositMethod {
    pub method: Option<String>,
    pub limit: Option<bool>,
    pub fee: Option<String>,
    pub address_setup_fee: Option<String>,
}

pub type DepositMethodsResult = Vec<DepositMethod>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DepositAddress {
    pub address: Option<String>,
    pub expiretm: Option<String>,
    pub new: Option<bool>,
    pub memo: Option<String>,
}

pub type DepositAddressesResult = Vec<DepositAddress>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DepositStatusEntry {
    pub method: Option<String>,
    pub aclass: Option<String>,
    pub asset: Option<String>,
    pub refid: Option<String>,
    pub txid: Option<String>,
    pub info: Option<String>,
    pub amount: Option<String>,
    pub fee: Option<String>,
    pub time: Option<u64>,
    pub status: Option<String>,
}

pub type DepositStatusResult = Vec<DepositStatusEntry>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WithdrawMethod {
    pub method: Option<String>,
    pub network: Option<String>,
    pub minimum: Option<String>,
    pub fee: Option<String>,
}

pub type WithdrawMethodsResult = Vec<WithdrawMethod>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WithdrawAddress {
    pub address: Option<String>,
    pub asset: Option<String>,
    pub method: Option<String>,
    pub key: Option<String>,
    pub verified: Option<bool>,
}

pub type WithdrawAddressesResult = Vec<WithdrawAddress>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WithdrawInfoResult {
    pub method: Option<String>,
    pub limit: Option<String>,
    pub fee: Option<String>,
    pub amount: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WithdrawResult {
    pub refid: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WithdrawStatusEntry {
    pub method: Option<String>,
    pub aclass: Option<String>,
    pub asset: Option<String>,
    pub refid: Option<String>,
    pub txid: Option<String>,
    pub info: Option<String>,
    pub amount: Option<String>,
    pub fee: Option<String>,
    pub time: Option<u64>,
    pub status: Option<String>,
}

pub type WithdrawStatusResult = Vec<WithdrawStatusEntry>;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CancelWithdrawResult {
    pub pending: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WalletTransferResult {
    pub transfer_id: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

// === Earn types ===

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EarnStrategiesResult {
    #[serde(default)]
    pub items: Vec<ApiObject>,
    pub next_cursor: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EarnAllocationsResult {
    #[serde(default)]
    pub items: Vec<ApiObject>,
    pub next_cursor: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EarnAllocateResult {
    pub status: Option<String>,
    pub strategy_id: Option<String>,
    pub pending: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EarnDeallocateResult {
    pub status: Option<String>,
    pub strategy_id: Option<String>,
    pub pending: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EarnAllocationStatusResult {
    pub pending: Option<bool>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

pub type EarnDeallocationStatusResult = EarnAllocationStatusResult;

// === Subaccount types ===

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CreateSubaccountResult {
    pub account: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AccountTransferResult {
    pub transfer_id: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, ApiValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WsTokenResult {
    pub token: String,
}

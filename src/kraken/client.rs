use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::error::KrakenError;
use super::types::*;

type HmacSha512 = Hmac<Sha512>;

const DEFAULT_BASE_URL: &str = "https://api.kraken.com";

#[derive(Serialize)]
struct EmptyParams;

#[derive(Clone)]
pub struct KrakenClient {
    http: reqwest::Client,
    api_key: Option<String>,
    api_secret: Option<String>,
    base_url: String,
}

impl KrakenClient {
    pub fn from_env() -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: std::env::var("KRAKEN_API_KEY").ok(),
            api_secret: std::env::var("KRAKEN_API_SECRET").ok(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn new_with_url(
        base_url: String,
        api_key: Option<String>,
        api_secret: Option<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            api_secret,
            base_url,
        }
    }

    fn require_auth(&self) -> Result<(&str, &str), KrakenError> {
        match (&self.api_key, &self.api_secret) {
            (Some(key), Some(secret)) => Ok((key.as_str(), secret.as_str())),
            _ => Err(KrakenError::AuthRequired),
        }
    }

    fn nonce() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    fn sign(path: &str, nonce: u64, post_data: &str, secret: &str) -> Result<String, KrakenError> {
        let secret_bytes = BASE64
            .decode(secret)
            .map_err(|e| KrakenError::Api(format!("Invalid API secret: {e}")))?;

        let mut sha256 = Sha256::new();
        sha256.update(format!("{nonce}{post_data}"));
        let sha256_digest = sha256.finalize();

        let mut hmac = HmacSha512::new_from_slice(&secret_bytes)
            .map_err(|e| KrakenError::Api(format!("HMAC error: {e}")))?;
        hmac.update(path.as_bytes());
        hmac.update(&sha256_digest);

        Ok(BASE64.encode(hmac.finalize().into_bytes()))
    }

    fn encode_with_nonce<P: Serialize + ?Sized>(
        params: &P,
        nonce: u64,
    ) -> Result<String, KrakenError> {
        let encoded = serde_urlencoded::to_string(params)
            .map_err(|e| KrakenError::Api(format!("Encode error: {e}")))?;
        if encoded.is_empty() {
            Ok(format!("nonce={nonce}"))
        } else {
            Ok(format!("{encoded}&nonce={nonce}"))
        }
    }

    async fn public<T, P>(&self, method: &str, params: &P) -> Result<T, KrakenError>
    where
        T: serde::de::DeserializeOwned,
        P: Serialize + ?Sized,
    {
        let url = format!("{}/0/public/{method}", self.base_url);
        let resp = self.http.get(&url).query(params).send().await?;
        let body: KrakenResponse<T> = resp.json().await?;
        if !body.error.is_empty() {
            return Err(KrakenError::from_api_errors(body.error));
        }
        body.result
            .ok_or_else(|| KrakenError::InvalidResponse("Missing result".into()))
    }

    async fn private<T, P>(&self, method: &str, params: &P) -> Result<T, KrakenError>
    where
        T: serde::de::DeserializeOwned,
        P: Serialize + ?Sized,
    {
        let (api_key, api_secret) = self.require_auth()?;
        let nonce = Self::nonce();

        let path = format!("/0/private/{method}");
        let post_data = Self::encode_with_nonce(params, nonce)?;
        let signature = Self::sign(&path, nonce, &post_data, api_secret)?;

        let url = format!("{}{path}", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("API-Key", api_key)
            .header("API-Sign", &signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(post_data)
            .send()
            .await?;

        let body: KrakenResponse<T> = resp.json().await?;
        if !body.error.is_empty() {
            return Err(KrakenError::from_api_errors(body.error));
        }
        body.result
            .ok_or_else(|| KrakenError::InvalidResponse("Missing result".into()))
    }

    /// Private endpoint using JSON body (for endpoints like AddOrderBatch that need nested arrays).
    async fn private_json<T, B>(&self, method: &str, body: &B) -> Result<T, KrakenError>
    where
        T: serde::de::DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let (api_key, api_secret) = self.require_auth()?;
        let nonce = Self::nonce();

        let mut payload = serde_json::to_value(body)
            .map_err(|e| KrakenError::Api(format!("JSON encode error: {e}")))?;
        payload["nonce"] = serde_json::Value::Number(nonce.into());

        let path = format!("/0/private/{method}");
        let post_data = serde_json::to_string(&payload)
            .map_err(|e| KrakenError::Api(format!("JSON encode error: {e}")))?;
        let signature = Self::sign(&path, nonce, &post_data, api_secret)?;

        let url = format!("{}{path}", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("API-Key", api_key)
            .header("API-Sign", &signature)
            .header("Content-Type", "application/json")
            .body(post_data)
            .send()
            .await?;

        let response_body: KrakenResponse<T> = resp.json().await?;
        if !response_body.error.is_empty() {
            return Err(KrakenError::from_api_errors(response_body.error));
        }
        response_body
            .result
            .ok_or_else(|| KrakenError::InvalidResponse("Missing result".into()))
    }

    /// Private endpoint returning raw bytes (for RetrieveExport).
    async fn private_bytes<P: Serialize + ?Sized>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<Vec<u8>, KrakenError> {
        let (api_key, api_secret) = self.require_auth()?;
        let nonce = Self::nonce();

        let path = format!("/0/private/{method}");
        let post_data = Self::encode_with_nonce(params, nonce)?;
        let signature = Self::sign(&path, nonce, &post_data, api_secret)?;

        let url = format!("{}{path}", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("API-Key", api_key)
            .header("API-Sign", &signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(post_data)
            .send()
            .await?;

        Ok(resp.bytes().await?.to_vec())
    }

    // === Public endpoints ===

    pub async fn server_time(&self) -> Result<ServerTimeResult, KrakenError> {
        self.public("Time", &EmptyParams).await
    }

    pub async fn system_status(&self) -> Result<SystemStatusResult, KrakenError> {
        self.public("SystemStatus", &EmptyParams).await
    }

    pub async fn assets(
        &self,
        asset: Option<&str>,
        aclass: Option<&str>,
    ) -> Result<AssetsResult, KrakenError> {
        #[derive(Serialize)]
        struct AssetsRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            asset: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            aclass: Option<&'a str>,
        }
        self.public("Assets", &AssetsRequest { asset, aclass })
            .await
    }

    pub async fn asset_pairs(
        &self,
        pair: Option<&str>,
        info: Option<&str>,
    ) -> Result<AssetPairsResult, KrakenError> {
        #[derive(Serialize)]
        struct AssetPairsRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            pair: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            info: Option<&'a str>,
        }
        self.public("AssetPairs", &AssetPairsRequest { pair, info })
            .await
    }

    pub async fn ticker(&self, pair: &str) -> Result<TickerResult, KrakenError> {
        #[derive(Serialize)]
        struct TickerRequest<'a> {
            pair: &'a str,
        }
        self.public("Ticker", &TickerRequest { pair }).await
    }

    pub async fn orderbook(
        &self,
        pair: &str,
        count: Option<u32>,
    ) -> Result<OrderBook, KrakenError> {
        #[derive(Serialize)]
        struct DepthRequest<'a> {
            pair: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            count: Option<u32>,
        }
        #[derive(serde::Deserialize)]
        struct RawDepthBook {
            asks: Vec<(String, String, u64)>,
            bids: Vec<(String, String, u64)>,
        }

        let raw: HashMap<String, RawDepthBook> =
            self.public("Depth", &DepthRequest { pair, count }).await?;
        let pair_data = raw
            .into_values()
            .next()
            .ok_or_else(|| KrakenError::InvalidResponse("Empty orderbook".into()))?;

        Ok(OrderBook {
            asks: pair_data
                .asks
                .into_iter()
                .map(|(price, volume, timestamp)| OrderBookLevel {
                    price,
                    volume,
                    timestamp,
                })
                .collect(),
            bids: pair_data
                .bids
                .into_iter()
                .map(|(price, volume, timestamp)| OrderBookLevel {
                    price,
                    volume,
                    timestamp,
                })
                .collect(),
        })
    }

    pub async fn ohlc(
        &self,
        pair: &str,
        interval: Option<u32>,
        since: Option<u64>,
    ) -> Result<OhlcResult, KrakenError> {
        #[derive(Serialize)]
        struct OhlcRequest<'a> {
            pair: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            interval: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            since: Option<u64>,
        }
        self.public(
            "OHLC",
            &OhlcRequest {
                pair,
                interval,
                since,
            },
        )
        .await
    }

    pub async fn recent_trades(
        &self,
        pair: &str,
        since: Option<&str>,
        count: Option<u32>,
    ) -> Result<RecentTradesResult, KrakenError> {
        #[derive(Serialize)]
        struct TradesRequest<'a> {
            pair: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            since: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            count: Option<u32>,
        }
        self.public("Trades", &TradesRequest { pair, since, count })
            .await
    }

    pub async fn spread(
        &self,
        pair: &str,
        since: Option<u64>,
    ) -> Result<SpreadResult, KrakenError> {
        #[derive(Serialize)]
        struct SpreadRequest<'a> {
            pair: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            since: Option<u64>,
        }
        self.public("Spread", &SpreadRequest { pair, since }).await
    }

    pub async fn grouped_book(
        &self,
        pair: &str,
        group: Option<u32>,
        levels: Option<u32>,
    ) -> Result<GroupedBookResult, KrakenError> {
        #[derive(Serialize)]
        struct GroupedBookRequest<'a> {
            pair: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            group: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            levels: Option<u32>,
        }
        self.public(
            "GroupedBook",
            &GroupedBookRequest {
                pair,
                group,
                levels,
            },
        )
        .await
    }

    pub async fn pre_trade(&self, symbol: &str) -> Result<PreTradeResult, KrakenError> {
        #[derive(Serialize)]
        struct PreTradeRequest<'a> {
            symbol: &'a str,
        }
        self.public("PreTrade", &PreTradeRequest { symbol }).await
    }

    pub async fn post_trade(
        &self,
        symbol: Option<&str>,
        from_ts: Option<&str>,
        to_ts: Option<&str>,
        count: Option<u32>,
    ) -> Result<PostTradeResult, KrakenError> {
        #[derive(Serialize)]
        struct PostTradeRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            symbol: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            from_ts: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            to_ts: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            count: Option<u32>,
        }
        self.public(
            "PostTrade",
            &PostTradeRequest {
                symbol,
                from_ts,
                to_ts,
                count,
            },
        )
        .await
    }

    pub async fn level3(
        &self,
        pair: &str,
        depth: Option<u32>,
    ) -> Result<Level3Result, KrakenError> {
        #[derive(Serialize)]
        struct Level3Request<'a> {
            pair: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            depth: Option<u32>,
        }
        self.private("Level3", &Level3Request { pair, depth }).await
    }

    // === Private — Account ===

    pub async fn balance(&self) -> Result<BalanceResult, KrakenError> {
        self.private("Balance", &EmptyParams).await
    }

    pub async fn credit_lines(&self) -> Result<CreditLinesResult, KrakenError> {
        self.private("CreditLines", &EmptyParams).await
    }

    pub async fn extended_balance(&self) -> Result<ExtendedBalanceResult, KrakenError> {
        self.private("BalanceEx", &EmptyParams).await
    }

    pub async fn trade_balance(
        &self,
        asset: Option<&str>,
    ) -> Result<TradeBalanceResult, KrakenError> {
        #[derive(Serialize)]
        struct TradeBalanceRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            asset: Option<&'a str>,
        }
        self.private("TradeBalance", &TradeBalanceRequest { asset })
            .await
    }

    pub async fn open_orders(&self) -> Result<OpenOrdersResult, KrakenError> {
        self.private("OpenOrders", &EmptyParams).await
    }

    pub async fn closed_orders(
        &self,
        trades: Option<bool>,
        start: Option<u64>,
        end: Option<u64>,
        ofs: Option<u32>,
        closetime: Option<&str>,
    ) -> Result<ClosedOrdersResult, KrakenError> {
        #[derive(Serialize)]
        struct ClosedOrdersRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            trades: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            start: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            end: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            ofs: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            closetime: Option<&'a str>,
        }
        self.private(
            "ClosedOrders",
            &ClosedOrdersRequest {
                trades,
                start,
                end,
                ofs,
                closetime,
            },
        )
        .await
    }

    pub async fn query_orders(
        &self,
        txid: &str,
        trades: Option<bool>,
    ) -> Result<QueryOrdersResult, KrakenError> {
        #[derive(Serialize)]
        struct QueryOrdersRequest<'a> {
            txid: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            trades: Option<bool>,
        }
        self.private("QueryOrders", &QueryOrdersRequest { txid, trades })
            .await
    }

    pub async fn order_amends(&self, txid: &str) -> Result<OrderAmendsResult, KrakenError> {
        #[derive(Serialize)]
        struct OrderAmendsRequest<'a> {
            txid: &'a str,
        }
        self.private("OrderAmends", &OrderAmendsRequest { txid })
            .await
    }

    pub async fn trade_history(
        &self,
        offset: Option<u32>,
    ) -> Result<TradesHistoryResult, KrakenError> {
        #[derive(Serialize)]
        struct TradesHistoryRequest {
            #[serde(skip_serializing_if = "Option::is_none", rename = "ofs")]
            offset: Option<u32>,
        }
        self.private("TradesHistory", &TradesHistoryRequest { offset })
            .await
    }

    pub async fn trades_info(
        &self,
        txid: &str,
        trades: Option<bool>,
    ) -> Result<TradesInfoResult, KrakenError> {
        #[derive(Serialize)]
        struct TradesInfoRequest<'a> {
            txid: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            trades: Option<bool>,
        }
        self.private("QueryTrades", &TradesInfoRequest { txid, trades })
            .await
    }

    pub async fn open_positions(
        &self,
        txid: Option<&str>,
        docalcs: Option<bool>,
    ) -> Result<OpenPositionsResult, KrakenError> {
        #[derive(Serialize)]
        struct OpenPositionsRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            txid: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            docalcs: Option<bool>,
        }
        self.private("OpenPositions", &OpenPositionsRequest { txid, docalcs })
            .await
    }

    pub async fn ledger(
        &self,
        asset: Option<&str>,
        ledger_type: Option<&str>,
        start: Option<u64>,
        end: Option<u64>,
        ofs: Option<u32>,
    ) -> Result<LedgerResult, KrakenError> {
        #[derive(Serialize)]
        struct LedgerRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            asset: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
            ledger_type: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            start: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            end: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            ofs: Option<u32>,
        }
        self.private(
            "Ledgers",
            &LedgerRequest {
                asset,
                ledger_type,
                start,
                end,
                ofs,
            },
        )
        .await
    }

    pub async fn query_ledger(
        &self,
        id: &str,
        trades: Option<bool>,
    ) -> Result<QueryLedgerResult, KrakenError> {
        #[derive(Serialize)]
        struct QueryLedgerRequest<'a> {
            id: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            trades: Option<bool>,
        }
        self.private("QueryLedgers", &QueryLedgerRequest { id, trades })
            .await
    }

    pub async fn trade_volume(&self, pair: Option<&str>) -> Result<TradeVolumeResult, KrakenError> {
        #[derive(Serialize)]
        struct TradeVolumeRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            pair: Option<&'a str>,
        }
        self.private("TradeVolume", &TradeVolumeRequest { pair })
            .await
    }

    pub async fn add_export(
        &self,
        report: &str,
        description: &str,
        format: Option<&str>,
        starttm: Option<u64>,
        endtm: Option<u64>,
    ) -> Result<AddExportResult, KrakenError> {
        #[derive(Serialize)]
        struct AddExportRequest<'a> {
            report: &'a str,
            description: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            format: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            starttm: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            endtm: Option<u64>,
        }
        self.private(
            "AddExport",
            &AddExportRequest {
                report,
                description,
                format,
                starttm,
                endtm,
            },
        )
        .await
    }

    pub async fn export_status(&self, report: &str) -> Result<ExportStatusResult, KrakenError> {
        #[derive(Serialize)]
        struct ExportStatusRequest<'a> {
            report: &'a str,
        }
        self.private("ExportStatus", &ExportStatusRequest { report })
            .await
    }

    pub async fn retrieve_export(&self, id: &str) -> Result<Vec<u8>, KrakenError> {
        #[derive(Serialize)]
        struct RetrieveExportRequest<'a> {
            id: &'a str,
        }
        self.private_bytes("RetrieveExport", &RetrieveExportRequest { id })
            .await
    }

    pub async fn remove_export(
        &self,
        id: &str,
        remove_type: &str,
    ) -> Result<RemoveExportResult, KrakenError> {
        #[derive(Serialize)]
        struct RemoveExportRequest<'a> {
            id: &'a str,
            #[serde(rename = "type")]
            remove_type: &'a str,
        }
        self.private("RemoveExport", &RemoveExportRequest { id, remove_type })
            .await
    }

    // === Private — Trading ===

    pub async fn add_order(
        &self,
        pair: &str,
        direction: &str,
        order_type: &str,
        volume: &str,
        price: Option<&str>,
        validate: bool,
    ) -> Result<AddOrderResult, KrakenError> {
        #[derive(Serialize)]
        struct AddOrderRequest<'a> {
            pair: &'a str,
            #[serde(rename = "type")]
            direction: &'a str,
            ordertype: &'a str,
            volume: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            price: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            validate: Option<bool>,
        }
        self.private(
            "AddOrder",
            &AddOrderRequest {
                pair,
                direction,
                ordertype: order_type,
                volume,
                price,
                validate: validate.then_some(true),
            },
        )
        .await
    }

    pub async fn add_order_batch(
        &self,
        pair: &str,
        orders: Vec<AddOrderBatchOrder>,
        validate: Option<bool>,
        deadline: Option<&str>,
    ) -> Result<AddOrderBatchResult, KrakenError> {
        #[derive(Serialize)]
        struct AddOrderBatchRequest<'a> {
            pair: &'a str,
            orders: Vec<AddOrderBatchOrder>,
            #[serde(skip_serializing_if = "Option::is_none")]
            validate: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            deadline: Option<&'a str>,
        }
        self.private_json(
            "AddOrderBatch",
            &AddOrderBatchRequest {
                pair,
                orders,
                validate,
                deadline,
            },
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn amend_order(
        &self,
        txid: Option<&str>,
        cl_ord_id: Option<&str>,
        order_qty: Option<&str>,
        limit_price: Option<&str>,
        trigger_price: Option<&str>,
        post_only: Option<bool>,
        pair: Option<&str>,
    ) -> Result<AmendOrderResult, KrakenError> {
        #[derive(Serialize)]
        struct AmendOrderRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            txid: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            cl_ord_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            order_qty: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            limit_price: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            trigger_price: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            post_only: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pair: Option<&'a str>,
        }
        self.private(
            "AmendOrder",
            &AmendOrderRequest {
                txid,
                cl_ord_id,
                order_qty,
                limit_price,
                trigger_price,
                post_only,
                pair,
            },
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn edit_order(
        &self,
        txid: &str,
        pair: &str,
        volume: Option<&str>,
        price: Option<&str>,
        price2: Option<&str>,
        oflags: Option<&str>,
        validate: Option<bool>,
    ) -> Result<EditOrderResult, KrakenError> {
        #[derive(Serialize)]
        struct EditOrderRequest<'a> {
            txid: &'a str,
            pair: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            volume: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            price: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            price2: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            oflags: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            validate: Option<bool>,
        }
        self.private(
            "EditOrder",
            &EditOrderRequest {
                txid,
                pair,
                volume,
                price,
                price2,
                oflags,
                validate,
            },
        )
        .await
    }

    pub async fn cancel_order(&self, txid: &str) -> Result<CancelOrderResult, KrakenError> {
        #[derive(Serialize)]
        struct CancelOrderRequest<'a> {
            txid: &'a str,
        }
        self.private("CancelOrder", &CancelOrderRequest { txid })
            .await
    }

    pub async fn cancel_all_orders(&self) -> Result<CancelOrderResult, KrakenError> {
        self.private("CancelAll", &EmptyParams).await
    }

    pub async fn cancel_all_after(
        &self,
        timeout: u32,
    ) -> Result<CancelAllAfterResult, KrakenError> {
        #[derive(Serialize)]
        struct CancelAllAfterRequest {
            timeout: u32,
        }
        self.private("CancelAllOrdersAfter", &CancelAllAfterRequest { timeout })
            .await
    }

    pub async fn cancel_order_batch(
        &self,
        orders: Vec<CancelOrderBatchItem>,
    ) -> Result<CancelOrderBatchResult, KrakenError> {
        #[derive(Serialize)]
        struct CancelOrderBatchRequest {
            orders: Vec<CancelOrderBatchItem>,
        }
        self.private_json("CancelOrderBatch", &CancelOrderBatchRequest { orders })
            .await
    }

    // === Private — Funding ===

    pub async fn deposit_methods(&self, asset: &str) -> Result<DepositMethodsResult, KrakenError> {
        #[derive(Serialize)]
        struct DepositMethodsRequest<'a> {
            asset: &'a str,
        }
        self.private("DepositMethods", &DepositMethodsRequest { asset })
            .await
    }

    pub async fn deposit_addresses(
        &self,
        asset: &str,
        method: &str,
        new: Option<bool>,
    ) -> Result<DepositAddressesResult, KrakenError> {
        #[derive(Serialize)]
        struct DepositAddressesRequest<'a> {
            asset: &'a str,
            method: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            new: Option<bool>,
        }
        self.private(
            "DepositAddresses",
            &DepositAddressesRequest { asset, method, new },
        )
        .await
    }

    pub async fn deposit_status(
        &self,
        asset: Option<&str>,
        method: Option<&str>,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<DepositStatusResult, KrakenError> {
        #[derive(Serialize)]
        struct DepositStatusRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            asset: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            method: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            cursor: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            limit: Option<u32>,
        }
        self.private(
            "DepositStatus",
            &DepositStatusRequest {
                asset,
                method,
                cursor,
                limit,
            },
        )
        .await
    }

    pub async fn withdraw_methods(
        &self,
        asset: Option<&str>,
        aclass: Option<&str>,
        network: Option<&str>,
    ) -> Result<WithdrawMethodsResult, KrakenError> {
        #[derive(Serialize)]
        struct WithdrawMethodsRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            asset: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            aclass: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            network: Option<&'a str>,
        }
        self.private(
            "WithdrawMethods",
            &WithdrawMethodsRequest {
                asset,
                aclass,
                network,
            },
        )
        .await
    }

    pub async fn withdraw_addresses(
        &self,
        asset: Option<&str>,
        aclass: Option<&str>,
        method: Option<&str>,
        key: Option<&str>,
        verified: Option<bool>,
    ) -> Result<WithdrawAddressesResult, KrakenError> {
        #[derive(Serialize)]
        struct WithdrawAddressesRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            asset: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            aclass: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            method: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            key: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            verified: Option<bool>,
        }
        self.private(
            "WithdrawAddresses",
            &WithdrawAddressesRequest {
                asset,
                aclass,
                method,
                key,
                verified,
            },
        )
        .await
    }

    pub async fn withdraw_info(
        &self,
        asset: &str,
        key: &str,
        amount: &str,
    ) -> Result<WithdrawInfoResult, KrakenError> {
        #[derive(Serialize)]
        struct WithdrawInfoRequest<'a> {
            asset: &'a str,
            key: &'a str,
            amount: &'a str,
        }
        self.private("WithdrawInfo", &WithdrawInfoRequest { asset, key, amount })
            .await
    }

    pub async fn withdraw(
        &self,
        asset: &str,
        key: &str,
        amount: &str,
        address: Option<&str>,
        max_fee: Option<&str>,
    ) -> Result<WithdrawResult, KrakenError> {
        #[derive(Serialize)]
        struct WithdrawRequest<'a> {
            asset: &'a str,
            key: &'a str,
            amount: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            address: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_fee: Option<&'a str>,
        }
        self.private(
            "Withdraw",
            &WithdrawRequest {
                asset,
                key,
                amount,
                address,
                max_fee,
            },
        )
        .await
    }

    pub async fn withdraw_status(
        &self,
        asset: Option<&str>,
        method: Option<&str>,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<WithdrawStatusResult, KrakenError> {
        #[derive(Serialize)]
        struct WithdrawStatusRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            asset: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            method: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            cursor: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            limit: Option<u32>,
        }
        self.private(
            "WithdrawStatus",
            &WithdrawStatusRequest {
                asset,
                method,
                cursor,
                limit,
            },
        )
        .await
    }

    pub async fn cancel_withdraw(
        &self,
        asset: &str,
        refid: &str,
    ) -> Result<CancelWithdrawResult, KrakenError> {
        #[derive(Serialize)]
        struct CancelWithdrawRequest<'a> {
            asset: &'a str,
            refid: &'a str,
        }
        self.private("WithdrawCancel", &CancelWithdrawRequest { asset, refid })
            .await
    }

    pub async fn wallet_transfer(
        &self,
        asset: &str,
        amount: &str,
    ) -> Result<WalletTransferResult, KrakenError> {
        #[derive(Serialize)]
        struct WalletTransferRequest<'a> {
            asset: &'a str,
            from: &'a str,
            to: &'a str,
            amount: &'a str,
        }
        self.private(
            "WalletTransfer",
            &WalletTransferRequest {
                asset,
                from: "Spot Wallet",
                to: "Futures Wallet",
                amount,
            },
        )
        .await
    }

    // === Private — Earn ===

    pub async fn earn_strategies(
        &self,
        asset: Option<&str>,
        limit: Option<u32>,
        cursor: Option<&str>,
        ascending: Option<bool>,
    ) -> Result<EarnStrategiesResult, KrakenError> {
        #[derive(Serialize)]
        struct EarnStrategiesRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            asset: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            limit: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            cursor: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            ascending: Option<bool>,
        }
        self.private(
            "Earn/Strategies",
            &EarnStrategiesRequest {
                asset,
                limit,
                cursor,
                ascending,
            },
        )
        .await
    }

    pub async fn earn_allocations(
        &self,
        converted_asset: Option<&str>,
        hide_zero: Option<bool>,
        ascending: Option<bool>,
    ) -> Result<EarnAllocationsResult, KrakenError> {
        #[derive(Serialize)]
        struct EarnAllocationsRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            converted_asset: Option<&'a str>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                rename = "hide_zero_allocations"
            )]
            hide_zero: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            ascending: Option<bool>,
        }
        self.private(
            "Earn/Allocations",
            &EarnAllocationsRequest {
                converted_asset,
                hide_zero,
                ascending,
            },
        )
        .await
    }

    pub async fn earn_allocate(
        &self,
        strategy_id: &str,
        amount: &str,
    ) -> Result<EarnAllocateResult, KrakenError> {
        #[derive(Serialize)]
        struct EarnAllocateRequest<'a> {
            strategy_id: &'a str,
            amount: &'a str,
        }
        self.private(
            "Earn/Allocate",
            &EarnAllocateRequest {
                strategy_id,
                amount,
            },
        )
        .await
    }

    pub async fn earn_deallocate(
        &self,
        strategy_id: &str,
        amount: &str,
    ) -> Result<EarnDeallocateResult, KrakenError> {
        #[derive(Serialize)]
        struct EarnDeallocateRequest<'a> {
            strategy_id: &'a str,
            amount: &'a str,
        }
        self.private(
            "Earn/Deallocate",
            &EarnDeallocateRequest {
                strategy_id,
                amount,
            },
        )
        .await
    }

    pub async fn earn_allocate_status(
        &self,
        strategy_id: &str,
    ) -> Result<EarnAllocationStatusResult, KrakenError> {
        #[derive(Serialize)]
        struct EarnStatusRequest<'a> {
            strategy_id: &'a str,
        }
        self.private("Earn/AllocateStatus", &EarnStatusRequest { strategy_id })
            .await
    }

    pub async fn earn_deallocate_status(
        &self,
        strategy_id: &str,
    ) -> Result<EarnDeallocationStatusResult, KrakenError> {
        #[derive(Serialize)]
        struct EarnStatusRequest<'a> {
            strategy_id: &'a str,
        }
        self.private("Earn/DeallocateStatus", &EarnStatusRequest { strategy_id })
            .await
    }

    // === Private — Subaccounts ===

    pub async fn create_subaccount(
        &self,
        username: &str,
        email: &str,
    ) -> Result<CreateSubaccountResult, KrakenError> {
        #[derive(Serialize)]
        struct CreateSubaccountRequest<'a> {
            username: &'a str,
            email: &'a str,
        }
        self.private(
            "CreateSubaccount",
            &CreateSubaccountRequest { username, email },
        )
        .await
    }

    pub async fn account_transfer(
        &self,
        asset: &str,
        amount: &str,
        from: &str,
        to: &str,
    ) -> Result<AccountTransferResult, KrakenError> {
        #[derive(Serialize)]
        struct AccountTransferRequest<'a> {
            asset: &'a str,
            amount: &'a str,
            from: &'a str,
            to: &'a str,
        }
        self.private(
            "AccountTransfer",
            &AccountTransferRequest {
                asset,
                amount,
                from,
                to,
            },
        )
        .await
    }

    // === WebSocket token ===

    /// Fetch a short-lived token for authenticating WebSocket v2 subscriptions.
    pub async fn get_websockets_token(&self) -> Result<String, KrakenError> {
        let result: WsTokenResult = self.private("GetWebSocketsToken", &EmptyParams).await?;
        Ok(result.token)
    }
}

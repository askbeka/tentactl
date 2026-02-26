use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256, Sha512};
use std::time::{SystemTime, UNIX_EPOCH};

use super::error::KrakenError;
use super::futures_types::{
    check_response, FuturesAccountsResult, FuturesBatchInstruction, FuturesBatchResult,
    FuturesCancelResult, FuturesFeeSchedulesResult, FuturesFillsResult, FuturesFundingRatesResult,
    FuturesInstrumentStatusResult, FuturesInstrumentsResult, FuturesLeverageResult,
    FuturesOpenOrdersResult, FuturesOpenPositionsResult, FuturesOrderStatusResult,
    FuturesOrderbookResult, FuturesPnlResult, FuturesResponse, FuturesSendOrderResult,
    FuturesTickerResult, FuturesTickersResult, FuturesTradeHistoryResult, FuturesTransferResult,
    FuturesTransfersResult, FuturesWithdrawalResult,
};

type HmacSha512 = Hmac<Sha512>;

const BASE_URL: &str = "https://futures.kraken.com";

#[derive(Serialize)]
struct EmptyParams;

/// Kraken Futures REST client.
///
/// Auth algorithm (from Go SDK):
///   SHA256(post_body_or_query_string || nonce || trimmed_path)
///   → HMAC-SHA512 with Base64-decoded secret
///   → Base64-encode result
///   Headers: APIKey, Authent, Nonce
#[derive(Clone)]
pub struct FuturesClient {
    http: reqwest::Client,
    api_key: Option<String>,
    api_secret: Option<String>,
    base_url: String,
}

impl FuturesClient {
    pub fn from_env() -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: std::env::var("KRAKEN_FUTURES_KEY").ok(),
            api_secret: std::env::var("KRAKEN_FUTURES_SECRET").ok(),
            base_url: BASE_URL.to_string(),
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
            _ => Err(KrakenError::FuturesAuthRequired),
        }
    }

    fn nonce() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string()
    }

    /// Futures signing algorithm.
    ///
    /// `data`: POST body or URL query string
    /// `nonce`: millisecond timestamp string
    /// `path`: full API path (e.g. `/derivatives/api/v3/accounts`)
    ///
    /// Returns Base64(HMAC-SHA512(SHA256(data || nonce || trimmed_path), secret))
    fn sign(secret: &str, data: &str, nonce: &str, path: &str) -> Result<String, KrakenError> {
        let secret_bytes = BASE64
            .decode(secret)
            .map_err(|e| KrakenError::Api(format!("Invalid futures API secret: {e}")))?;

        // Strip "/derivatives" prefix from path as per the reference SDK
        let trimmed = path.strip_prefix("/derivatives").unwrap_or(path);

        let mut sha256 = Sha256::new();
        sha256.update(data.as_bytes());
        sha256.update(nonce.as_bytes());
        sha256.update(trimmed.as_bytes());
        let digest = sha256.finalize();

        let mut hmac = HmacSha512::new_from_slice(&secret_bytes)
            .map_err(|e| KrakenError::Api(format!("HMAC error: {e}")))?;
        hmac.update(&digest);
        Ok(BASE64.encode(hmac.finalize().into_bytes()))
    }

    // === Internal request helpers ===

    async fn public<T, P>(&self, path: &str, params: &P) -> Result<T, KrakenError>
    where
        T: serde::de::DeserializeOwned,
        P: Serialize + ?Sized,
    {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.http.get(&url).query(params).send().await?;
        let value: FuturesResponse<T> = resp.json().await?;
        check_response(value)
    }

    async fn private_get<T, P>(&self, path: &str, params: &P) -> Result<T, KrakenError>
    where
        T: serde::de::DeserializeOwned,
        P: Serialize + ?Sized,
    {
        let (api_key, api_secret) = self.require_auth()?;
        let nonce = Self::nonce();

        // For GET: hash the URL-encoded query string
        let query_string = serde_urlencoded::to_string(params)
            .map_err(|e| KrakenError::Api(format!("Encode error: {e}")))?;
        let authent = Self::sign(api_secret, &query_string, &nonce, path)?;

        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .query(params)
            .header("APIKey", api_key)
            .header("Authent", &authent)
            .header("Nonce", &nonce)
            .send()
            .await?;

        let value: FuturesResponse<T> = resp.json().await?;
        check_response(value)
    }

    async fn private_post<T, P>(&self, path: &str, params: &P) -> Result<T, KrakenError>
    where
        T: serde::de::DeserializeOwned,
        P: Serialize + ?Sized,
    {
        let (api_key, api_secret) = self.require_auth()?;
        let nonce = Self::nonce();

        let body = serde_urlencoded::to_string(params)
            .map_err(|e| KrakenError::Api(format!("Encode error: {e}")))?;
        let authent = Self::sign(api_secret, &body, &nonce, path)?;

        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .header("APIKey", api_key)
            .header("Authent", &authent)
            .header("Nonce", &nonce)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        let value: FuturesResponse<T> = resp.json().await?;
        check_response(value)
    }

    async fn private_put<T, P>(&self, path: &str, params: &P) -> Result<T, KrakenError>
    where
        T: serde::de::DeserializeOwned,
        P: Serialize + ?Sized,
    {
        let (api_key, api_secret) = self.require_auth()?;
        let nonce = Self::nonce();

        let body = serde_urlencoded::to_string(params)
            .map_err(|e| KrakenError::Api(format!("Encode error: {e}")))?;
        let authent = Self::sign(api_secret, &body, &nonce, path)?;

        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .put(&url)
            .header("APIKey", api_key)
            .header("Authent", &authent)
            .header("Nonce", &nonce)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        let value: FuturesResponse<T> = resp.json().await?;
        check_response(value)
    }

    // === Public endpoints ===

    pub async fn instruments(&self) -> Result<FuturesInstrumentsResult, KrakenError> {
        self.public("/derivatives/api/v3/instruments", &EmptyParams)
            .await
    }

    pub async fn instrument_status(
        &self,
        instrument: &str,
    ) -> Result<FuturesInstrumentStatusResult, KrakenError> {
        let path = format!("/derivatives/api/v3/instruments/{instrument}/status");
        self.public(&path, &EmptyParams).await
    }

    pub async fn tickers(&self) -> Result<FuturesTickersResult, KrakenError> {
        self.public("/derivatives/api/v3/tickers", &EmptyParams)
            .await
    }

    pub async fn ticker(&self, symbol: &str) -> Result<FuturesTickerResult, KrakenError> {
        let path = format!("/derivatives/api/v3/tickers/{symbol}");
        self.public(&path, &EmptyParams).await
    }

    pub async fn orderbook(&self, symbol: &str) -> Result<FuturesOrderbookResult, KrakenError> {
        #[derive(Serialize)]
        struct OrderbookRequest<'a> {
            symbol: &'a str,
        }
        self.public(
            "/derivatives/api/v3/orderbook",
            &OrderbookRequest { symbol },
        )
        .await
    }

    pub async fn trade_history(
        &self,
        symbol: &str,
        last_time: Option<&str>,
    ) -> Result<FuturesTradeHistoryResult, KrakenError> {
        #[derive(Serialize)]
        struct TradeHistoryRequest<'a> {
            symbol: &'a str,
            #[serde(skip_serializing_if = "Option::is_none", rename = "lastTime")]
            last_time: Option<&'a str>,
        }
        self.public(
            "/derivatives/api/v3/history",
            &TradeHistoryRequest { symbol, last_time },
        )
        .await
    }

    pub async fn fee_schedules(&self) -> Result<FuturesFeeSchedulesResult, KrakenError> {
        self.public("/derivatives/api/v3/feeschedules", &EmptyParams)
            .await
    }

    pub async fn historical_funding_rates(
        &self,
        symbol: &str,
    ) -> Result<FuturesFundingRatesResult, KrakenError> {
        #[derive(Serialize)]
        struct FundingRatesRequest<'a> {
            symbol: &'a str,
        }
        self.public(
            "/derivatives/api/v3/historicalfundingrates",
            &FundingRatesRequest { symbol },
        )
        .await
    }

    // === Private — Account ===

    pub async fn accounts(&self) -> Result<FuturesAccountsResult, KrakenError> {
        self.private_get("/derivatives/api/v3/accounts", &EmptyParams)
            .await
    }

    pub async fn open_orders(&self) -> Result<FuturesOpenOrdersResult, KrakenError> {
        self.private_get("/derivatives/api/v3/openorders", &EmptyParams)
            .await
    }

    pub async fn open_positions(&self) -> Result<FuturesOpenPositionsResult, KrakenError> {
        self.private_get("/derivatives/api/v3/openpositions", &EmptyParams)
            .await
    }

    pub async fn fills(
        &self,
        last_fill_time: Option<&str>,
    ) -> Result<FuturesFillsResult, KrakenError> {
        #[derive(Serialize)]
        struct FillsRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none", rename = "lastFillTime")]
            last_fill_time: Option<&'a str>,
        }
        self.private_get(
            "/derivatives/api/v3/fills",
            &FillsRequest { last_fill_time },
        )
        .await
    }

    pub async fn transfers(
        &self,
        last_transfer_time: Option<&str>,
    ) -> Result<FuturesTransfersResult, KrakenError> {
        #[derive(Serialize)]
        struct TransfersRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none", rename = "lastTransferTime")]
            last_transfer_time: Option<&'a str>,
        }
        self.private_get(
            "/derivatives/api/v3/transfers",
            &TransfersRequest { last_transfer_time },
        )
        .await
    }

    pub async fn order_status(
        &self,
        order_ids: &str,
    ) -> Result<FuturesOrderStatusResult, KrakenError> {
        #[derive(Serialize)]
        struct OrderStatusRequest<'a> {
            #[serde(rename = "orderIds")]
            order_ids: &'a str,
        }
        self.private_get(
            "/derivatives/api/v3/orders/status",
            &OrderStatusRequest { order_ids },
        )
        .await
    }

    pub async fn get_leverage_setting(
        &self,
        symbol: Option<&str>,
    ) -> Result<FuturesLeverageResult, KrakenError> {
        #[derive(Serialize)]
        struct LeverageRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            symbol: Option<&'a str>,
        }
        self.private_get(
            "/derivatives/api/v3/leveragesetting",
            &LeverageRequest { symbol },
        )
        .await
    }

    pub async fn set_leverage_setting(
        &self,
        symbol: &str,
        max_leverage: &str,
    ) -> Result<FuturesLeverageResult, KrakenError> {
        #[derive(Serialize)]
        struct SetLeverageRequest<'a> {
            symbol: &'a str,
            #[serde(rename = "maxLeverage")]
            max_leverage: &'a str,
        }
        self.private_put(
            "/derivatives/api/v3/leveragesetting",
            &SetLeverageRequest {
                symbol,
                max_leverage,
            },
        )
        .await
    }

    pub async fn get_pnl_preference(
        &self,
        symbol: Option<&str>,
    ) -> Result<FuturesPnlResult, KrakenError> {
        #[derive(Serialize)]
        struct PnlPreferenceRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            symbol: Option<&'a str>,
        }
        self.private_get(
            "/derivatives/api/v3/pnlcurrencypreference",
            &PnlPreferenceRequest { symbol },
        )
        .await
    }

    pub async fn set_pnl_preference(
        &self,
        symbol: &str,
        pnl_preference: &str,
    ) -> Result<FuturesPnlResult, KrakenError> {
        #[derive(Serialize)]
        struct SetPnlPreferenceRequest<'a> {
            symbol: &'a str,
            #[serde(rename = "pnlPreference")]
            pnl_preference: &'a str,
        }
        self.private_put(
            "/derivatives/api/v3/pnlcurrencypreference",
            &SetPnlPreferenceRequest {
                symbol,
                pnl_preference,
            },
        )
        .await
    }

    // === Private — Trading ===

    #[allow(clippy::too_many_arguments)]
    pub async fn send_order(
        &self,
        order_type: &str,
        symbol: &str,
        side: &str,
        size: &str,
        limit_price: Option<&str>,
        stop_price: Option<&str>,
        client_order_id: Option<&str>,
        reduce_only: Option<bool>,
    ) -> Result<FuturesSendOrderResult, KrakenError> {
        #[derive(Serialize)]
        struct SendOrderRequest<'a> {
            #[serde(rename = "orderType")]
            order_type: &'a str,
            symbol: &'a str,
            side: &'a str,
            size: &'a str,
            #[serde(skip_serializing_if = "Option::is_none", rename = "limitPrice")]
            limit_price: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "stopPrice")]
            stop_price: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "cliOrdId")]
            client_order_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "reduceOnly")]
            reduce_only: Option<bool>,
        }
        self.private_post(
            "/derivatives/api/v3/sendorder",
            &SendOrderRequest {
                order_type,
                symbol,
                side,
                size,
                limit_price,
                stop_price,
                client_order_id,
                reduce_only,
            },
        )
        .await
    }

    pub async fn edit_order(
        &self,
        order_id: Option<&str>,
        client_order_id: Option<&str>,
        size: Option<&str>,
        limit_price: Option<&str>,
        stop_price: Option<&str>,
    ) -> Result<FuturesSendOrderResult, KrakenError> {
        #[derive(Serialize)]
        struct EditOrderRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none", rename = "orderId")]
            order_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "cliOrdId")]
            client_order_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            size: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "limitPrice")]
            limit_price: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "stopPrice")]
            stop_price: Option<&'a str>,
        }
        self.private_post(
            "/derivatives/api/v3/editorder",
            &EditOrderRequest {
                order_id,
                client_order_id,
                size,
                limit_price,
                stop_price,
            },
        )
        .await
    }

    pub async fn cancel_order(
        &self,
        order_id: Option<&str>,
        client_order_id: Option<&str>,
    ) -> Result<FuturesCancelResult, KrakenError> {
        #[derive(Serialize)]
        struct CancelOrderRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none", rename = "order_id")]
            order_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none", rename = "cliOrdId")]
            client_order_id: Option<&'a str>,
        }
        self.private_post(
            "/derivatives/api/v3/cancelorder",
            &CancelOrderRequest {
                order_id,
                client_order_id,
            },
        )
        .await
    }

    pub async fn cancel_all(
        &self,
        symbol: Option<&str>,
    ) -> Result<FuturesCancelResult, KrakenError> {
        #[derive(Serialize)]
        struct CancelAllRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            symbol: Option<&'a str>,
        }
        self.private_post(
            "/derivatives/api/v3/cancelallorders",
            &CancelAllRequest { symbol },
        )
        .await
    }

    /// Batch order — body contains a `json` field with a stringified JSON array
    /// of order instructions (send/cancel/edit).
    pub async fn batch_order(
        &self,
        instructions: Vec<FuturesBatchInstruction>,
    ) -> Result<FuturesBatchResult, KrakenError> {
        let instructions_json = serde_json::to_string(&instructions)
            .map_err(|e| KrakenError::Api(format!("Invalid batch instructions: {e}")))?;

        #[derive(Serialize)]
        struct BatchOrderRequest<'a> {
            json: &'a str,
        }
        self.private_post(
            "/derivatives/api/v3/batchorder",
            &BatchOrderRequest {
                json: &instructions_json,
            },
        )
        .await
    }

    pub async fn transfer(
        &self,
        from_account: &str,
        to_account: &str,
        unit: &str,
        amount: &str,
    ) -> Result<FuturesTransferResult, KrakenError> {
        #[derive(Serialize)]
        struct TransferRequest<'a> {
            #[serde(rename = "fromAccount")]
            from_account: &'a str,
            #[serde(rename = "toAccount")]
            to_account: &'a str,
            unit: &'a str,
            amount: &'a str,
        }
        self.private_post(
            "/derivatives/api/v3/transfer",
            &TransferRequest {
                from_account,
                to_account,
                unit,
                amount,
            },
        )
        .await
    }

    pub async fn withdrawal(
        &self,
        target_address: &str,
        currency: &str,
        amount: &str,
    ) -> Result<FuturesWithdrawalResult, KrakenError> {
        #[derive(Serialize)]
        struct WithdrawalRequest<'a> {
            #[serde(rename = "targetAddress")]
            target_address: &'a str,
            currency: &'a str,
            amount: &'a str,
        }
        self.private_post(
            "/derivatives/api/v3/withdrawal",
            &WithdrawalRequest {
                target_address,
                currency,
                amount,
            },
        )
        .await
    }
}

#![allow(dead_code)]
// Create a KrakenClient pointing at a mock server.
// We can't import from a bin crate, so we inline a minimal client here.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashMap;

type HmacSha512 = Hmac<Sha512>;

#[derive(Debug, Deserialize)]
pub struct KrakenResponse<T> {
    pub error: Vec<String>,
    pub result: Option<T>,
}

// Re-export API types used by tests.
pub use tentactl::kraken::types::*;

#[derive(Debug)]
pub enum KrakenError {
    Http(reqwest::Error),
    Api(String),
    AuthRequired,
    InvalidResponse(String),
}

impl std::fmt::Display for KrakenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KrakenError::Http(e) => write!(f, "HTTP error: {e}"),
            KrakenError::Api(e) => write!(f, "{e}"),
            KrakenError::AuthRequired => write!(
                f,
                "Authentication required: set KRAKEN_API_KEY and KRAKEN_API_SECRET"
            ),
            KrakenError::InvalidResponse(e) => write!(f, "Invalid response: {e}"),
        }
    }
}

impl From<reqwest::Error> for KrakenError {
    fn from(e: reqwest::Error) -> Self {
        KrakenError::Http(e)
    }
}

fn classify_errors(errors: Vec<String>) -> KrakenError {
    if errors.is_empty() {
        return KrakenError::Api("Unknown error".into());
    }
    let msg = errors.join("; ");
    if msg.contains("EAPI:Rate limit") {
        KrakenError::Api("Rate limited — slow down requests".into())
    } else if msg.contains("EOrder:Insufficient funds") {
        KrakenError::Api("Insufficient funds for this order".into())
    } else if msg.contains("EQuery:Unknown asset pair") {
        KrakenError::Api(format!("Unknown asset pair: {msg}"))
    } else {
        KrakenError::Api(msg)
    }
}

#[derive(Clone)]
pub struct TestKrakenClient {
    http: reqwest::Client,
    api_key: Option<String>,
    api_secret: Option<String>,
    base_url: String,
}

pub fn mock_client(
    base_url: &str,
    api_key: Option<String>,
    api_secret: Option<String>,
) -> TestKrakenClient {
    TestKrakenClient {
        http: reqwest::Client::new(),
        api_key,
        api_secret,
        base_url: base_url.to_string(),
    }
}

impl TestKrakenClient {
    fn require_auth(&self) -> Result<(&str, &str), KrakenError> {
        match (&self.api_key, &self.api_secret) {
            (Some(k), Some(s)) => Ok((k.as_str(), s.as_str())),
            _ => Err(KrakenError::AuthRequired),
        }
    }

    fn sign(path: &str, nonce: u64, post_data: &str, secret: &str) -> String {
        let secret_bytes = BASE64.decode(secret).expect("valid base64");
        let mut sha256 = Sha256::new();
        sha256.update(format!("{nonce}{post_data}"));
        let sha256_digest = sha256.finalize();
        let mut hmac = HmacSha512::new_from_slice(&secret_bytes).expect("valid key");
        hmac.update(path.as_bytes());
        hmac.update(&sha256_digest);
        BASE64.encode(hmac.finalize().into_bytes())
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
            return Err(classify_errors(body.error));
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
        let nonce = 1234567890u64; // fixed for tests

        let path = format!("/0/private/{method}");
        let encoded =
            serde_urlencoded::to_string(params).map_err(|e| KrakenError::Api(e.to_string()))?;
        let post_data = if encoded.is_empty() {
            format!("nonce={nonce}")
        } else {
            format!("{encoded}&nonce={nonce}")
        };
        let signature = Self::sign(&path, nonce, &post_data, api_secret);

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
            return Err(classify_errors(body.error));
        }
        body.result
            .ok_or_else(|| KrakenError::InvalidResponse("Missing result".into()))
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
        #[derive(Deserialize)]
        struct RawDepthBook {
            asks: Vec<(String, String, u64)>,
            bids: Vec<(String, String, u64)>,
        }

        let result: HashMap<String, RawDepthBook> =
            self.public("Depth", &DepthRequest { pair, count }).await?;

        let pair_data = result
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

    pub async fn balance(&self) -> Result<BalanceResult, KrakenError> {
        self.private("Balance", &()).await
    }

    pub async fn credit_lines(&self) -> Result<CreditLinesResult, KrakenError> {
        self.private("CreditLines", &()).await
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

    pub async fn open_orders(&self) -> Result<OpenOrdersResult, KrakenError> {
        self.private("OpenOrders", &()).await
    }

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

    pub async fn cancel_order(&self, txid: &str) -> Result<CancelOrderResult, KrakenError> {
        #[derive(Serialize)]
        struct CancelOrderRequest<'a> {
            txid: &'a str,
        }
        self.private("CancelOrder", &CancelOrderRequest { txid })
            .await
    }
}

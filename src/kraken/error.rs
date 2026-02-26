use thiserror::Error;

#[derive(Error, Debug)]
pub enum KrakenError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Kraken API error: {0}")]
    Api(String),

    #[error(
        "Authentication required: set KRAKEN_API_KEY and KRAKEN_API_SECRET environment variables"
    )]
    AuthRequired,

    #[error(
        "Futures authentication required: set KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET environment variables"
    )]
    FuturesAuthRequired,

    #[error("Invalid API response: {0}")]
    InvalidResponse(String),

    #[error("Rate limited — slow down requests and retry after a few seconds")]
    RateLimited,

    #[error("Too many requests — Kraken is throttling your connection. Wait and retry.")]
    TooManyRequests,

    #[error("Insufficient funds for this order — check your balance")]
    InsufficientFunds,

    #[error("Unknown asset pair: {0} — check spelling (e.g. XBTUSD, ETHUSD)")]
    UnknownPair(String),

    #[error("Unknown asset: {0}")]
    UnknownAsset(String),

    #[error("Order not found: {0}")]
    OrderNotFound(String),

    #[error("Invalid order: {0}")]
    InvalidOrder(String),

    #[error("Permission denied — your API key lacks the required permissions. Check key settings at https://www.kraken.com/u/security/api")]
    PermissionDenied,

    #[error("Invalid API key — check that KRAKEN_API_KEY is correct and not expired")]
    InvalidKey,

    #[error("Invalid nonce — your system clock may be wrong, or another client is using the same API key. Use unique keys per client.")]
    InvalidNonce,

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Market is in cancel-only mode — only cancellations allowed right now")]
    MarketCancelOnly,

    #[error("Market is in post-only mode — only post-only limit orders allowed right now")]
    MarketPostOnly,

    #[error("Market is in limit-only mode — no market orders allowed right now")]
    MarketLimitOnly,

    #[error("Order amount too small for this asset")]
    AmountTooSmall,

    #[error("Order amount too large for this asset")]
    AmountTooLarge,

    #[error("Invalid price specified")]
    InvalidPrice,

    #[error("Service unavailable — Kraken may be under maintenance. Try again later.")]
    ServiceUnavailable,

    #[error("Service timeout — request took too long. Try again.")]
    ServiceTimeout,

    #[error("Unknown method — this endpoint may not be available on your account tier")]
    UnknownMethod,

    #[error("WebSocket error: {0}")]
    #[allow(dead_code)]
    WebSocket(String),
}

impl KrakenError {
    pub fn from_api_errors(errors: Vec<String>) -> Self {
        if errors.is_empty() {
            return KrakenError::Api("Unknown error".into());
        }
        let msg = errors.join("; ");

        // Auth / key errors
        if msg.contains("Invalid key") {
            return KrakenError::InvalidKey;
        }
        if msg.contains("Invalid nonce") || msg.contains("Invalid `api-nonce`") {
            return KrakenError::InvalidNonce;
        }
        if msg.contains("Permission denied") || msg.contains("Invalid permissions") {
            return KrakenError::PermissionDenied;
        }

        // Rate limiting
        if msg.contains("Rate limit") {
            return KrakenError::RateLimited;
        }
        if msg.contains("Too many requests") {
            return KrakenError::TooManyRequests;
        }

        // Order errors
        if msg.contains("Insufficient funds") {
            return KrakenError::InsufficientFunds;
        }
        if msg.contains("Unknown order") {
            return KrakenError::OrderNotFound(msg);
        }
        if msg.contains("Invalid order") {
            return KrakenError::InvalidOrder(msg);
        }
        if msg.contains("cancel_only mode") || msg.contains("cancel only") {
            return KrakenError::MarketCancelOnly;
        }
        if msg.contains("post_only mode") || msg.contains("post only") {
            return KrakenError::MarketPostOnly;
        }
        if msg.contains("limit_only mode") || msg.contains("limit only") {
            return KrakenError::MarketLimitOnly;
        }
        if msg.contains("too small") || msg.contains("Below min") {
            return KrakenError::AmountTooSmall;
        }
        if msg.contains("too large") || msg.contains("Above max") {
            return KrakenError::AmountTooLarge;
        }
        if msg.contains("Invalid price") {
            return KrakenError::InvalidPrice;
        }

        // Asset/pair errors
        if msg.contains("Unknown asset pair") || msg.contains("Invalid asset pair") {
            return KrakenError::UnknownPair(msg);
        }
        if msg.contains("Unknown asset") || msg.contains("Invalid asset") {
            return KrakenError::UnknownAsset(msg);
        }

        // Service errors
        if msg.contains("Unavailable") || msg.contains("Busy") {
            return KrakenError::ServiceUnavailable;
        }
        if msg.contains("Timeout") {
            return KrakenError::ServiceTimeout;
        }
        if msg.contains("Unknown method") {
            return KrakenError::UnknownMethod;
        }

        // General validation
        if msg.contains("Invalid arguments") {
            return KrakenError::InvalidArguments(msg);
        }

        KrakenError::Api(msg)
    }
}

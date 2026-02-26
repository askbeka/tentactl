use rmcp::schemars::{self, JsonSchema};
use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_router, ErrorData as McpError,
};
use serde::Deserialize;

use crate::server::KrakenMcpServer;

fn ok(v: &impl serde::Serialize) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(v).unwrap_or_default(),
    )]))
}

fn err(e: impl std::fmt::Display) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(e.to_string())]))
}

// === Param types ===

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InstrumentStatusParams {
    #[schemars(description = "Instrument symbol (e.g. PF_XBTUSD, PI_ETHUSD)")]
    pub instrument: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TickerParams {
    #[schemars(description = "Futures symbol (e.g. PF_XBTUSD, PI_ETHUSD)")]
    pub symbol: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OrderBookParams {
    #[schemars(description = "Futures symbol (e.g. PF_XBTUSD, PI_ETHUSD)")]
    pub symbol: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TradeHistoryParams {
    #[schemars(description = "Futures symbol (e.g. PF_XBTUSD, PI_ETHUSD)")]
    pub symbol: String,
    #[schemars(description = "ISO8601 timestamp; return only trades after this time (optional)")]
    pub last_time: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FundingRatesParams {
    #[schemars(description = "Futures symbol to filter by (e.g. PF_XBTUSD). Required.")]
    pub symbol: String,
}

// === Tool implementations ===

#[tool_router(router = futures_market_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    #[tool(
        name = "futures_instruments",
        description = "List all available Kraken Futures instruments (perpetual swaps, fixed-maturity futures, flex futures) with contract specs, margin schedules, and trading parameters. Public — no auth required."
    )]
    pub async fn futures_instruments(&self) -> Result<CallToolResult, McpError> {
        match self.futures_client.instruments().await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_instrument_status",
        description = "Get the trading status of a specific Kraken Futures instrument (tradeable, post-only, suspended, etc.). Public — no auth required."
    )]
    pub async fn futures_instrument_status(
        &self,
        Parameters(p): Parameters<InstrumentStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.futures_client.instrument_status(&p.instrument).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_tickers",
        description = "Get ticker data for all Kraken Futures instruments and indices: last price, bid/ask, 24h volume, open interest, funding rate, mark price. Public — no auth required."
    )]
    pub async fn futures_tickers(&self) -> Result<CallToolResult, McpError> {
        match self.futures_client.tickers().await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_ticker",
        description = "Get ticker data for a single Kraken Futures symbol: last price, bid/ask, 24h volume, open interest, funding rate. Public — no auth required."
    )]
    pub async fn futures_ticker(
        &self,
        Parameters(p): Parameters<TickerParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.futures_client.ticker(&p.symbol).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_orderbook",
        description = "Get the top-of-book bids and asks for a Kraken Futures symbol. Public — no auth required."
    )]
    pub async fn futures_orderbook(
        &self,
        Parameters(p): Parameters<OrderBookParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.futures_client.orderbook(&p.symbol).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_trade_history",
        description = "Get recent public trade history for a Kraken Futures symbol. Optionally filter to trades after a given timestamp. Public — no auth required."
    )]
    pub async fn futures_trade_history(
        &self,
        Parameters(p): Parameters<TradeHistoryParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .futures_client
            .trade_history(&p.symbol, p.last_time.as_deref())
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_fee_schedules",
        description = "List all Kraken Futures fee schedules including maker/taker rates by volume tier. Public — no auth required."
    )]
    pub async fn futures_fee_schedules(&self) -> Result<CallToolResult, McpError> {
        match self.futures_client.fee_schedules().await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_historical_funding_rates",
        description = "Get historical funding rates for a Kraken Futures perpetual swap symbol. Public — no auth required."
    )]
    pub async fn futures_historical_funding_rates(
        &self,
        Parameters(p): Parameters<FundingRatesParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .futures_client
            .historical_funding_rates(&p.symbol)
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }
}

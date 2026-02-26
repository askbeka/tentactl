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
pub struct GetTickerParams {
    #[schemars(description = "Trading pair (e.g. XBTUSD, ETHUSD, SOLUSD)")]
    pub pair: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetOrderbookParams {
    #[schemars(description = "Trading pair (e.g. XBTUSD, ETHUSD)")]
    pub pair: String,
    #[schemars(description = "Max number of asks/bids (1-500, default 10)")]
    pub count: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetOhlcParams {
    #[schemars(description = "Trading pair (e.g. XBTUSD, ETHUSD)")]
    pub pair: String,
    #[schemars(description = "Interval in minutes: 1, 5, 15, 30, 60, 240, 1440, 10080, 21600")]
    pub interval: Option<u32>,
    #[schemars(description = "Return data since UNIX timestamp")]
    pub since: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAssetsParams {
    #[schemars(
        description = "Comma-delimited list of assets to filter (e.g. XBT,ETH). Omit for all."
    )]
    pub asset: Option<String>,
    #[schemars(description = "Asset class filter (default: currency)")]
    pub aclass: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetAssetPairsParams {
    #[schemars(description = "Comma-delimited list of pairs (e.g. XBTUSD,ETHUSD). Omit for all.")]
    pub pair: Option<String>,
    #[schemars(description = "Info to retrieve: info, leverage, fees, margin (default: info)")]
    pub info: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRecentTradesParams {
    #[schemars(description = "Trading pair (e.g. XBTUSD)")]
    pub pair: String,
    #[schemars(description = "Return trades since this UNIX timestamp")]
    pub since: Option<String>,
    #[schemars(description = "Number of trades to return (1-1000, default 1000)")]
    pub count: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSpreadParams {
    #[schemars(description = "Trading pair (e.g. XBTUSD)")]
    pub pair: String,
    #[schemars(description = "Return spread data since this UNIX timestamp")]
    pub since: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetGroupedBookParams {
    #[schemars(description = "Trading pair (e.g. BTC/USD)")]
    pub pair: String,
    #[schemars(description = "Grouping/tick size for aggregated levels (optional)")]
    pub group: Option<u32>,
    #[schemars(description = "Max number of grouped levels per side (optional)")]
    pub levels: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLevel3Params {
    #[schemars(description = "Trading pair (e.g. XBTUSD, ETHUSD)")]
    pub pair: String,
    #[schemars(description = "Max number of levels per side (optional)")]
    pub depth: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPreTradeParams {
    #[schemars(description = "Trading symbol (e.g. BTC/USD)")]
    pub symbol: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPostTradeParams {
    #[schemars(description = "Filter by symbol (e.g. BTC/USD)")]
    pub symbol: Option<String>,
    #[schemars(description = "Return trades after this ISO-8601 timestamp")]
    pub from_ts: Option<String>,
    #[schemars(description = "Return trades before or at this ISO-8601 timestamp")]
    pub to_ts: Option<String>,
    #[schemars(description = "Maximum number of trades to return (1-1000)")]
    pub count: Option<u32>,
}

// === Tool implementations ===

#[tool_router(router = spot_market_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    #[tool(
        name = "get_server_time",
        description = "Get the Kraken server time (Unix timestamp and RFC1123). No auth required."
    )]
    pub async fn get_server_time(&self) -> Result<CallToolResult, McpError> {
        match self.client.server_time().await {
            Ok(t) => ok(&t),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_system_status",
        description = "Get current Kraken system status: online, maintenance, cancel_only, or post_only. No auth required."
    )]
    pub async fn get_system_status(&self) -> Result<CallToolResult, McpError> {
        match self.client.system_status().await {
            Ok(s) => ok(&s),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_assets",
        description = "Get info about tradable assets (decimals, altname, status). No auth required."
    )]
    pub async fn get_assets(
        &self,
        Parameters(p): Parameters<GetAssetsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .assets(p.asset.as_deref(), p.aclass.as_deref())
            .await
        {
            Ok(a) => ok(&a),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_asset_pairs",
        description = "Get tradable asset pairs with fees, leverage, margin, and precision info. No auth required."
    )]
    pub async fn get_asset_pairs(
        &self,
        Parameters(p): Parameters<GetAssetPairsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .asset_pairs(p.pair.as_deref(), p.info.as_deref())
            .await
        {
            Ok(a) => ok(&a),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_ticker",
        description = "Get current ticker for a trading pair: ask/bid price, last trade, 24h volume, high/low. No auth required."
    )]
    pub async fn get_ticker(
        &self,
        Parameters(p): Parameters<GetTickerParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.ticker(&p.pair).await {
            Ok(ticker) => ok(&ticker),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_orderbook",
        description = "Get order book (asks/bids) for a trading pair. No auth required."
    )]
    pub async fn get_orderbook(
        &self,
        Parameters(p): Parameters<GetOrderbookParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.orderbook(&p.pair, p.count).await {
            Ok(book) => ok(&book),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_ohlc",
        description = "Get OHLC candlestick data for a trading pair. No auth required."
    )]
    pub async fn get_ohlc(
        &self,
        Parameters(p): Parameters<GetOhlcParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.ohlc(&p.pair, p.interval, p.since).await {
            Ok(data) => ok(&data),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_recent_trades",
        description = "Get recent trades for a trading pair (up to 1000). No auth required."
    )]
    pub async fn get_recent_trades(
        &self,
        Parameters(p): Parameters<GetRecentTradesParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .recent_trades(&p.pair, p.since.as_deref(), p.count)
            .await
        {
            Ok(trades) => ok(&trades),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_spread",
        description = "Get recent bid/ask spreads (last ~200 entries) for a trading pair. No auth required."
    )]
    pub async fn get_spread(
        &self,
        Parameters(p): Parameters<GetSpreadParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.spread(&p.pair, p.since).await {
            Ok(spread) => ok(&spread),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_grouped_book",
        description = "Get grouped/aggregated order book depth for a trading pair. No auth required."
    )]
    pub async fn get_grouped_book(
        &self,
        Parameters(p): Parameters<GetGroupedBookParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.grouped_book(&p.pair, p.group, p.levels).await {
            Ok(book) => ok(&book),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_level3",
        description = "Get private Level-3 order book data (individual order IDs and timestamps). Requires API keys."
    )]
    pub async fn get_level3(
        &self,
        Parameters(p): Parameters<GetLevel3Params>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.level3(&p.pair, p.depth).await {
            Ok(book) => ok(&book),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_pre_trade",
        description = "Get pre-trade transparency data (top aggregated order book levels) for a symbol. No auth required."
    )]
    pub async fn get_pre_trade(
        &self,
        Parameters(p): Parameters<GetPreTradeParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.pre_trade(&p.symbol).await {
            Ok(data) => ok(&data),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_post_trade",
        description = "Get post-trade transparency data (spot trade prints) with optional symbol/time filters. No auth required."
    )]
    pub async fn get_post_trade(
        &self,
        Parameters(p): Parameters<GetPostTradeParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .post_trade(
                p.symbol.as_deref(),
                p.from_ts.as_deref(),
                p.to_ts.as_deref(),
                p.count,
            )
            .await
        {
            Ok(data) => ok(&data),
            Err(e) => err(e),
        }
    }
}

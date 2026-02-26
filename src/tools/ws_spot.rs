use rmcp::schemars::{self, JsonSchema};
use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_router, ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;

use crate::server::KrakenMcpServer;

fn ok(v: &impl serde::Serialize) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(v).unwrap_or_default(),
    )]))
}

fn err(e: impl std::fmt::Display) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(e.to_string())]))
}

/// Parse a comma-separated symbol string into a Vec.

// ── Param types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsSubscribeTickerParams {
    #[schemars(description = "Trading pairs in WS format, e.g. BTC/USD, ETH/USD")]
    pub symbols: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsSubscribeBookParams {
    #[schemars(description = "Trading pairs in WS format, e.g. BTC/USD, ETH/USD")]
    pub symbols: Vec<String>,
    #[schemars(description = "Depth: 10, 25, 100, 500, or 1000 (default 10)")]
    pub depth: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsSubscribeTradesParams {
    #[schemars(description = "Trading pairs, e.g. BTC/USD, ETH/USD")]
    pub symbols: Vec<String>,
    #[schemars(description = "Request a snapshot of the last 50 trades (default false)")]
    pub snapshot: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsSubscribeOhlcParams {
    #[schemars(description = "Trading pairs, e.g. BTC/USD, ETH/USD")]
    pub symbols: Vec<String>,
    #[schemars(
        description = "Candle interval in minutes: 1, 5, 15, 30, 60, 240, 1440, 10080, 21600 (default 1)"
    )]
    pub interval: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsSubscribeLevel3Params {
    #[schemars(description = "Comma-separated trading pairs (e.g. BTC/USD)")]
    pub symbols: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsUnsubscribeParams {
    #[schemars(
        description = "Channel name: ticker, book, trade, ohlc, instrument, level3, executions, balances"
    )]
    pub channel: String,
    #[schemars(description = "Symbols to unsubscribe (omit for all)")]
    pub symbols: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsAddOrderParams {
    #[schemars(
        description = "Order type: limit, market, iceberg, stop-loss, stop-loss-limit, take-profit, take-profit-limit, trailing-stop, trailing-stop-limit, settle-position"
    )]
    pub order_type: String,
    #[schemars(description = "Order side: buy or sell")]
    pub side: String,
    #[schemars(description = "Order quantity in base currency")]
    pub order_qty: f64,
    #[schemars(description = "Trading pair (e.g. BTC/USD)")]
    pub symbol: String,
    #[schemars(description = "Limit price (required for limit orders)")]
    pub limit_price: Option<f64>,
    #[schemars(description = "Time-in-force: gtc (default), gtd, ioc")]
    pub time_in_force: Option<String>,
    #[schemars(description = "Client order ID (alphanumeric, up to 18 chars)")]
    pub cl_ord_id: Option<String>,
    #[schemars(description = "Client order user-reference (integer)")]
    pub order_userref: Option<i64>,
    #[schemars(description = "Post-only order (limit orders only)")]
    pub post_only: Option<bool>,
    #[schemars(description = "Reduce-only order (margin positions)")]
    pub reduce_only: Option<bool>,
    #[schemars(description = "Validate only — does NOT place a real order")]
    pub validate: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsAmendOrderParams {
    #[schemars(description = "Kraken order ID to amend")]
    pub order_id: Option<String>,
    #[schemars(description = "Client order ID to amend")]
    pub cl_ord_id: Option<String>,
    #[schemars(description = "New order quantity")]
    pub order_qty: Option<f64>,
    #[schemars(description = "New limit price")]
    pub limit_price: Option<f64>,
    #[schemars(description = "New post-only flag")]
    pub post_only: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsEditOrderParams {
    #[schemars(description = "Original order ID to edit (creates a new order ID)")]
    pub order_id: String,
    #[schemars(description = "Trading pair (e.g. BTC/USD)")]
    pub symbol: String,
    #[schemars(description = "New order quantity")]
    pub order_qty: Option<f64>,
    #[schemars(description = "New limit price")]
    pub limit_price: Option<f64>,
    #[schemars(description = "New post-only flag")]
    pub post_only: Option<bool>,
    #[schemars(description = "New client order ID")]
    pub cl_ord_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsCancelOrderParams {
    #[schemars(description = "Kraken order IDs to cancel")]
    pub order_id: Option<Vec<String>>,
    #[schemars(description = "Client order IDs to cancel")]
    pub cl_ord_id: Option<Vec<String>>,
    #[schemars(description = "Order user-refs to cancel")]
    pub order_userref: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsCancelAfterParams {
    #[schemars(
        description = "Seconds until all open orders are cancelled. Set to 0 to disable the timer."
    )]
    pub timeout: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsBatchAddParams {
    #[schemars(
        description = r#"JSON array of order objects. Each object supports the same fields as ws_add_order params (order_type, side, order_qty, limit_price, cl_ord_id, …). Example: [{"order_type":"limit","side":"buy","order_qty":0.1,"limit_price":50000}]"#
    )]
    pub orders: String,
    #[schemars(description = "Trading pair for all orders in the batch (e.g. BTC/USD)")]
    pub symbol: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WsBatchCancelParams {
    #[schemars(
        description = r#"JSON array of cancel objects. Each object must have one of: {"order_id":"…"} or {"cl_ord_id":"…"}. Example: [{"order_id":"AA111-BB222-CC333"},{"order_id":"DD444-EE555-FF666"}]"#
    )]
    pub orders: String,
}

// ── Tool router ───────────────────────────────────────────────────────

#[tool_router(router = ws_spot_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    // ── Public subscriptions ──────────────────────────────────────────

    #[tool(
        description = "Subscribe to real-time Level-1 ticker data (best bid/ask + recent trade stats) for one or more symbols. Returns the latest buffered snapshot after subscribing."
    )]
    async fn ws_subscribe_ticker(
        &self,
        Parameters(params): Parameters<WsSubscribeTickerParams>,
    ) -> Result<CallToolResult, McpError> {
        let symbols = params.symbols.clone();
        if let Err(e) = self.ws_client.subscribe_ticker(symbols.clone()).await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws::WsClient::snapshot_wait()).await;
        let data = self.ws_client.get_tickers(&symbols).await;
        ok(&json!({"subscribed": symbols, "data": data}))
    }

    #[tool(
        description = "Subscribe to real-time Level-2 order book for one or more symbols. Depth: 10 (default), 25, 100, 500, 1000. Updates are applied incrementally; returns the current book snapshot."
    )]
    async fn ws_subscribe_book(
        &self,
        Parameters(params): Parameters<WsSubscribeBookParams>,
    ) -> Result<CallToolResult, McpError> {
        let symbols = params.symbols.clone();
        let depth = params.depth.unwrap_or(10);
        if let Err(e) = self.ws_client.subscribe_book(symbols.clone(), depth).await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws::WsClient::snapshot_wait()).await;
        let data = self.ws_client.get_books(&symbols).await;
        ok(&json!({"subscribed": symbols, "depth": depth, "data": data}))
    }

    #[tool(
        description = "Subscribe to real-time trade feed for one or more symbols. Each trade event includes price, quantity, side, and timestamp. Returns last 50 buffered trades per symbol."
    )]
    async fn ws_subscribe_trades(
        &self,
        Parameters(params): Parameters<WsSubscribeTradesParams>,
    ) -> Result<CallToolResult, McpError> {
        let symbols = params.symbols.clone();
        let snapshot = params.snapshot.unwrap_or(true);
        if let Err(e) = self
            .ws_client
            .subscribe_trades(symbols.clone(), snapshot)
            .await
        {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws::WsClient::snapshot_wait()).await;
        let data = self.ws_client.get_trades(&symbols).await;
        ok(&json!({"subscribed": symbols, "data": data}))
    }

    #[tool(
        description = "Subscribe to real-time OHLC (candlestick) data for one or more symbols. Interval in minutes: 1 (default), 5, 15, 30, 60, 240, 1440, 10080, 21600. Returns the latest candle per symbol."
    )]
    async fn ws_subscribe_ohlc(
        &self,
        Parameters(params): Parameters<WsSubscribeOhlcParams>,
    ) -> Result<CallToolResult, McpError> {
        let symbols = params.symbols.clone();
        let interval = params.interval.unwrap_or(1);
        if let Err(e) = self
            .ws_client
            .subscribe_ohlc(symbols.clone(), interval)
            .await
        {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws::WsClient::snapshot_wait()).await;
        let mut data = serde_json::Map::new();
        for sym in &symbols {
            if let Some(v) = self.ws_client.get_ohlc(sym, interval).await {
                data.insert(sym.clone(), v);
            }
        }
        ok(&json!({"subscribed": symbols, "interval": interval, "data": data}))
    }

    #[tool(
        description = "Subscribe to the instrument channel for real-time asset and trading-pair reference data (precision, status, min/max order sizes). Returns the full snapshot once received."
    )]
    async fn ws_subscribe_instrument(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.ws_client.subscribe_instrument().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws::WsClient::snapshot_wait()).await;
        let snap = self.ws_client.get_snapshot().await;
        ok(&json!({"subscribed": true, "data": snap.instrument}))
    }

    // ── Private subscriptions ─────────────────────────────────────────

    #[tool(
        description = "Subscribe to Level-3 individual order events for one or more symbols. Requires KRAKEN_API_KEY / KRAKEN_API_SECRET. Returns last 50 L3 order events per symbol after subscribing."
    )]
    async fn ws_subscribe_level3(
        &self,
        Parameters(params): Parameters<WsSubscribeLevel3Params>,
    ) -> Result<CallToolResult, McpError> {
        let symbols = params.symbols.clone();
        if let Err(e) = self.ws_client.subscribe_level3(symbols.clone()).await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws::WsClient::snapshot_wait()).await;
        ok(&json!({"subscribed": symbols, "note": "L3 events buffered in ws_status"}))
    }

    #[tool(
        description = "Subscribe to the executions channel for real-time order status and fill events. Requires KRAKEN_API_KEY / KRAKEN_API_SECRET. Returns a snapshot of open orders and the last 50 trades."
    )]
    async fn ws_subscribe_executions(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.ws_client.subscribe_executions().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws::WsClient::snapshot_wait()).await;
        let execs = self.ws_client.get_executions().await;
        ok(&json!({"subscribed": true, "executions": execs}))
    }

    #[tool(
        description = "Subscribe to the balances channel for real-time asset balance and ledger-entry events. Requires KRAKEN_API_KEY / KRAKEN_API_SECRET. Returns the current balance snapshot."
    )]
    async fn ws_subscribe_balances(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.ws_client.subscribe_balances().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws::WsClient::snapshot_wait()).await;
        let balances = self.ws_client.get_balances().await;
        ok(&json!({"subscribed": true, "balances": balances}))
    }

    // ── Trading requests ──────────────────────────────────────────────

    #[tool(
        description = "⚠️ REAL MONEY — places a new order via WebSocket (lower latency than REST). Requires KRAKEN_API_KEY / KRAKEN_API_SECRET. Always confirm with the user before calling. Use validate=true to test without trading."
    )]
    async fn ws_add_order(
        &self,
        Parameters(params): Parameters<WsAddOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut p = json!({
            "order_type": params.order_type,
            "side": params.side,
            "order_qty": params.order_qty,
            "symbol": params.symbol,
        });
        if let Some(v) = params.limit_price {
            p["limit_price"] = json!(v);
        }
        if let Some(v) = &params.time_in_force {
            p["time_in_force"] = json!(v);
        }
        if let Some(v) = &params.cl_ord_id {
            p["cl_ord_id"] = json!(v);
        }
        if let Some(v) = params.order_userref {
            p["order_userref"] = json!(v);
        }
        if let Some(v) = params.post_only {
            p["post_only"] = json!(v);
        }
        if let Some(v) = params.reduce_only {
            p["reduce_only"] = json!(v);
        }
        if let Some(v) = params.validate {
            p["validate"] = json!(v);
        }
        match self.ws_client.trading_request("add_order", p).await {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "⚠️ REAL MONEY — amends an open order in-place via WebSocket (maintains order ID and queue priority). Requires KRAKEN_API_KEY / KRAKEN_API_SECRET. Confirm with user first."
    )]
    async fn ws_amend_order(
        &self,
        Parameters(params): Parameters<WsAmendOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut p = json!({});
        if let Some(v) = &params.order_id {
            p["order_id"] = json!(v);
        }
        if let Some(v) = &params.cl_ord_id {
            p["cl_ord_id"] = json!(v);
        }
        if let Some(v) = params.order_qty {
            p["order_qty"] = json!(v);
        }
        if let Some(v) = params.limit_price {
            p["limit_price"] = json!(v);
        }
        if let Some(v) = params.post_only {
            p["post_only"] = json!(v);
        }
        match self.ws_client.trading_request("amend_order", p).await {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "⚠️ REAL MONEY — edits an open order via WebSocket (creates a new order ID; loses queue priority). Requires KRAKEN_API_KEY / KRAKEN_API_SECRET. Confirm with user first."
    )]
    async fn ws_edit_order(
        &self,
        Parameters(params): Parameters<WsEditOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut p = json!({
            "order_id": params.order_id,
            "symbol": params.symbol,
        });
        if let Some(v) = params.order_qty {
            p["order_qty"] = json!(v);
        }
        if let Some(v) = params.limit_price {
            p["limit_price"] = json!(v);
        }
        if let Some(v) = params.post_only {
            p["post_only"] = json!(v);
        }
        if let Some(v) = &params.cl_ord_id {
            p["cl_ord_id"] = json!(v);
        }
        match self.ws_client.trading_request("edit_order", p).await {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "⚠️ REAL MONEY — cancels one or more open orders via WebSocket. Provide order_id, cl_ord_id, or order_userref (comma-separated for multiple). Requires KRAKEN_API_KEY / KRAKEN_API_SECRET."
    )]
    async fn ws_cancel_order(
        &self,
        Parameters(params): Parameters<WsCancelOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut p = json!({});
        if let Some(v) = &params.order_id {
            p["order_id"] = json!(v);
        }
        if let Some(v) = &params.cl_ord_id {
            p["cl_ord_id"] = json!(v);
        }
        if let Some(v) = &params.order_userref {
            p["order_userref"] = json!(v);
        }
        match self.ws_client.trading_request("cancel_order", p).await {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "⚠️ REAL MONEY — cancels ALL open orders for this account via WebSocket. Requires KRAKEN_API_KEY / KRAKEN_API_SECRET. Confirm with user before calling."
    )]
    async fn ws_cancel_all(&self) -> Result<CallToolResult, McpError> {
        match self
            .ws_client
            .trading_request("cancel_all", json!({}))
            .await
        {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "Dead-man's switch: automatically cancel all open orders after `timeout` seconds unless the timer is reset or disabled. Set timeout=0 to disable. Requires KRAKEN_API_KEY / KRAKEN_API_SECRET."
    )]
    async fn ws_cancel_after(
        &self,
        Parameters(params): Parameters<WsCancelAfterParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .ws_client
            .trading_request("cancel_after", json!({"timeout": params.timeout}))
            .await
        {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "⚠️ REAL MONEY — places 2–15 orders for the same symbol in a single WebSocket request. Pass `orders` as a JSON array (same fields as ws_add_order). Requires KRAKEN_API_KEY / KRAKEN_API_SECRET."
    )]
    async fn ws_batch_add(
        &self,
        Parameters(params): Parameters<WsBatchAddParams>,
    ) -> Result<CallToolResult, McpError> {
        let orders: serde_json::Value = match serde_json::from_str(&params.orders) {
            Ok(v) => v,
            Err(e) => return err(format!("Invalid orders JSON: {e}")),
        };
        match self
            .ws_client
            .trading_request(
                "batch_add",
                json!({"orders": orders, "symbol": params.symbol}),
            )
            .await
        {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "⚠️ REAL MONEY — cancels 2–50 orders in a single WebSocket request. Pass `orders` as a JSON array of objects with order_id or cl_ord_id. Requires KRAKEN_API_KEY / KRAKEN_API_SECRET."
    )]
    async fn ws_batch_cancel(
        &self,
        Parameters(params): Parameters<WsBatchCancelParams>,
    ) -> Result<CallToolResult, McpError> {
        let orders: serde_json::Value = match serde_json::from_str(&params.orders) {
            Ok(v) => v,
            Err(e) => return err(format!("Invalid orders JSON: {e}")),
        };
        match self
            .ws_client
            .trading_request("batch_cancel", json!({"orders": orders}))
            .await
        {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    // ── Management ────────────────────────────────────────────────────

    #[tool(
        description = "Unsubscribe from a WebSocket channel. Specify channel name and optionally a comma-separated list of symbols. Omit symbols to unsubscribe all."
    )]
    async fn ws_unsubscribe(
        &self,
        Parameters(params): Parameters<WsUnsubscribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let symbols: Option<Vec<String>> = params.symbols.clone();
        match self.ws_client.unsubscribe(&params.channel, symbols).await {
            Ok(()) => ok(&json!({"unsubscribed": params.channel})),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "Returns the current WebSocket connection state: which channels are subscribed, buffered market data, balances, executions, and connection status."
    )]
    async fn ws_status(&self) -> Result<CallToolResult, McpError> {
        let snap = self.ws_client.get_snapshot().await;
        ok(&snap)
    }
}

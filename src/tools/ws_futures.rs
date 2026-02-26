//! Kraken Futures WebSocket tools (wf_* prefix).
//!
//! Subscription tools connect to `wss://futures.kraken.com/ws/v1` and buffer
//! live market data / account updates.
//!
//! Order tools (wf_send_order, wf_cancel_order, wf_batch_order) delegate to
//! the Futures REST API — the Futures WS v1 protocol does not support order
//! placement commands directly.

use rmcp::schemars::{self, JsonSchema};
use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_router, ErrorData as McpError,
};
use serde::Deserialize;
use serde_json::json;

use crate::kraken::futures_types::FuturesBatchInstruction;
use crate::server::KrakenMcpServer;

fn ok(v: &impl serde::Serialize) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(v).unwrap_or_default(),
    )]))
}

fn err(e: impl std::fmt::Display) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(e.to_string())]))
}

// ── Param types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WfSubscribeTickerParams {
    #[schemars(
        description = "Comma-separated Kraken Futures product IDs (e.g. PF_XBTUSD,PF_ETHUSD)"
    )]
    pub product_ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WfSubscribeBookParams {
    #[schemars(
        description = "Comma-separated Kraken Futures product IDs (e.g. PF_XBTUSD,PF_ETHUSD)"
    )]
    pub product_ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WfSubscribeTradesParams {
    #[schemars(
        description = "Comma-separated Kraken Futures product IDs (e.g. PF_XBTUSD,PF_ETHUSD)"
    )]
    pub product_ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WfUnsubscribeParams {
    #[schemars(
        description = "Feed name: ticker, ticker_lite, book, trade, fills, account_log, notifications_auth, open_orders, open_orders_verbose, open_positions, balances"
    )]
    pub feed: String,
    #[schemars(
        description = "Comma-separated product IDs to unsubscribe (omit for channel-wide unsubscribe)"
    )]
    pub product_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WfSendOrderParams {
    #[schemars(description = "Futures product ID (e.g. PF_XBTUSD)")]
    pub symbol: String,
    #[schemars(description = "Order side: buy or sell")]
    pub side: String,
    #[schemars(description = "Order type: lmt, mkt, stp, take_profit, ioc, post")]
    pub order_type: String,
    #[schemars(description = "Order size in contracts")]
    pub size: f64,
    #[schemars(description = "Limit price (required for lmt/post orders)")]
    pub limit_price: Option<f64>,
    #[schemars(description = "Stop price (for stp/take_profit orders)")]
    pub stop_price: Option<f64>,
    #[schemars(description = "Client order ID (alphanumeric, up to 100 chars)")]
    pub cli_ord_id: Option<String>,
    #[schemars(description = "Reduce-only order (won't increase position size)")]
    pub reduce_only: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WfCancelOrderParams {
    #[schemars(description = "Kraken order ID to cancel")]
    pub order_id: Option<String>,
    #[schemars(description = "Client order ID to cancel")]
    pub cli_ord_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WfBatchOrderParams {
    #[schemars(
        description = r#"JSON array of batch instructions. Each item must have "order": one of "send", "cancel", "edit", plus the relevant fields. Example: [{"order":"send","orderType":"lmt","symbol":"PF_XBTUSD","side":"buy","size":1,"limitPrice":50000}]"#
    )]
    pub orders: String,
}

// ── Tool router ───────────────────────────────────────────────────────

#[tool_router(router = ws_futures_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    // ── Public subscriptions ──────────────────────────────────────────

    #[tool(
        description = "Subscribe to real-time ticker data for one or more Kraken Futures products via WebSocket. Returns buffered ticker snapshots after subscribing."
    )]
    async fn wf_subscribe_ticker(
        &self,
        Parameters(params): Parameters<WfSubscribeTickerParams>,
    ) -> Result<CallToolResult, McpError> {
        let product_ids = params.product_ids.clone();
        if let Err(e) = self.futures_ws.subscribe_ticker(product_ids.clone()).await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let data = self.futures_ws.get_tickers(&product_ids).await;
        ok(&json!({"subscribed": product_ids, "data": data}))
    }

    #[tool(
        description = "Subscribe to lightweight real-time ticker data for one or more Kraken Futures products via WebSocket. Returns buffered ticker snapshots after subscribing."
    )]
    async fn wf_subscribe_ticker_lite(
        &self,
        Parameters(params): Parameters<WfSubscribeTickerParams>,
    ) -> Result<CallToolResult, McpError> {
        let product_ids = params.product_ids.clone();
        if let Err(e) = self
            .futures_ws
            .subscribe_ticker_lite(product_ids.clone())
            .await
        {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let data = self.futures_ws.get_tickers(&product_ids).await;
        ok(&json!({"subscribed": product_ids, "data": data}))
    }

    #[tool(
        description = "Subscribe to real-time Level-2 order book for one or more Kraken Futures products via WebSocket. Returns the current book snapshot after subscribing."
    )]
    async fn wf_subscribe_book(
        &self,
        Parameters(params): Parameters<WfSubscribeBookParams>,
    ) -> Result<CallToolResult, McpError> {
        let product_ids = params.product_ids.clone();
        if let Err(e) = self.futures_ws.subscribe_book(product_ids.clone()).await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let data = self.futures_ws.get_books(&product_ids).await;
        ok(&json!({"subscribed": product_ids, "data": data}))
    }

    #[tool(
        description = "Subscribe to real-time trade feed for one or more Kraken Futures products via WebSocket. Returns the last 50 buffered trades per product."
    )]
    async fn wf_subscribe_trades(
        &self,
        Parameters(params): Parameters<WfSubscribeTradesParams>,
    ) -> Result<CallToolResult, McpError> {
        let product_ids = params.product_ids.clone();
        if let Err(e) = self.futures_ws.subscribe_trades(product_ids.clone()).await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let data = self.futures_ws.get_trades(&product_ids).await;
        ok(&json!({"subscribed": product_ids, "data": data}))
    }

    // ── Private subscriptions ─────────────────────────────────────────

    #[tool(
        description = "Subscribe to the fills feed for real-time trade execution updates via Futures WebSocket. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET. Returns the last 50 buffered fills."
    )]
    async fn wf_subscribe_fills(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.futures_ws.subscribe_fills().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let snap = self.futures_ws.get_snapshot().await;
        ok(&json!({"subscribed": true, "fills": snap.fills}))
    }

    #[tool(
        description = "Subscribe to the account_log feed for real-time account activity entries via Futures WebSocket. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET."
    )]
    async fn wf_subscribe_account_log(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.futures_ws.subscribe_account_log().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let snap = self.futures_ws.get_snapshot().await;
        ok(&json!({"subscribed": true, "account_log": snap.account_log}))
    }

    #[tool(
        description = "Subscribe to authenticated account notifications via Futures WebSocket. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET."
    )]
    async fn wf_subscribe_notifications(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.futures_ws.subscribe_notifications().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let snap = self.futures_ws.get_snapshot().await;
        ok(&json!({"subscribed": true, "notifications": snap.notifications}))
    }

    #[tool(
        description = "Subscribe to the open_orders feed for real-time order status updates via Futures WebSocket. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET."
    )]
    async fn wf_subscribe_open_orders(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.futures_ws.subscribe_open_orders().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let snap = self.futures_ws.get_snapshot().await;
        ok(&json!({"subscribed": true, "open_orders": snap.open_orders}))
    }

    #[tool(
        description = "Subscribe to the open_orders_verbose feed for detailed real-time order status updates via Futures WebSocket. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET."
    )]
    async fn wf_subscribe_open_orders_verbose(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.futures_ws.subscribe_open_orders_verbose().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let snap = self.futures_ws.get_snapshot().await;
        ok(&json!({"subscribed": true, "open_orders": snap.open_orders}))
    }

    #[tool(
        description = "Subscribe to the open_positions feed for real-time position updates via Futures WebSocket. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET."
    )]
    async fn wf_subscribe_open_positions(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.futures_ws.subscribe_open_positions().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let snap = self.futures_ws.get_snapshot().await;
        ok(&json!({"subscribed": true, "open_positions": snap.open_positions}))
    }

    #[tool(
        description = "Subscribe to the balances feed for real-time account balance updates via Futures WebSocket. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET."
    )]
    async fn wf_subscribe_balances(&self) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.futures_ws.subscribe_balances().await {
            return err(e);
        }
        tokio::time::sleep(crate::kraken::ws_futures::FuturesWsClient::snapshot_wait()).await;
        let snap = self.futures_ws.get_snapshot().await;
        ok(&json!({"subscribed": true, "balances": snap.balances}))
    }

    // ── Order management (via REST) ───────────────────────────────────

    #[tool(
        description = "⚠️ REAL MONEY — place a new Futures order. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET. Always confirm with the user first. Uses REST API internally."
    )]
    async fn wf_send_order(
        &self,
        Parameters(params): Parameters<WfSendOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        let size_str = params.size.to_string();
        let limit_price_str = params.limit_price.map(|p| p.to_string());
        let stop_price_str = params.stop_price.map(|p| p.to_string());
        match self
            .futures_client
            .send_order(
                &params.order_type,
                &params.symbol,
                &params.side,
                &size_str,
                limit_price_str.as_deref(),
                stop_price_str.as_deref(),
                params.cli_ord_id.as_deref(),
                params.reduce_only,
            )
            .await
        {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "⚠️ REAL MONEY — cancel a Futures order by order_id or cli_ord_id. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET. Confirm with user before calling. Uses REST API internally."
    )]
    async fn wf_cancel_order(
        &self,
        Parameters(params): Parameters<WfCancelOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .futures_client
            .cancel_order(params.order_id.as_deref(), params.cli_ord_id.as_deref())
            .await
        {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "⚠️ REAL MONEY — execute a batch of Futures order instructions (send/cancel/edit) in a single request. Pass `orders` as a JSON array. Requires KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET. Uses REST API internally."
    )]
    async fn wf_batch_order(
        &self,
        Parameters(params): Parameters<WfBatchOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        let instructions: Vec<FuturesBatchInstruction> = match serde_json::from_str(&params.orders)
        {
            Ok(v) => v,
            Err(e) => return err(format!("Invalid orders JSON: {e}")),
        };
        if instructions.is_empty() {
            return err("orders must be a non-empty JSON array");
        }
        match self.futures_client.batch_order(instructions).await {
            Ok(resp) => ok(&resp),
            Err(e) => err(e),
        }
    }

    // ── Management ────────────────────────────────────────────────────

    #[tool(
        description = "Unsubscribe from a Kraken Futures WebSocket feed. Specify the feed name and optionally a comma-separated list of product IDs. Omit product_ids to unsubscribe all."
    )]
    async fn wf_unsubscribe(
        &self,
        Parameters(params): Parameters<WfUnsubscribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let product_ids: Option<Vec<String>> = params.product_ids.clone();
        match self.futures_ws.unsubscribe(&params.feed, product_ids).await {
            Ok(()) => ok(&json!({"unsubscribed": params.feed})),
            Err(e) => err(e),
        }
    }

    #[tool(
        description = "Returns the current Kraken Futures WebSocket connection state: subscribed feeds, buffered tickers, books, trades, fills, open orders/positions, balances, and connection status."
    )]
    async fn wf_status(&self) -> Result<CallToolResult, McpError> {
        let snap = self.futures_ws.get_snapshot().await;
        ok(&snap)
    }
}

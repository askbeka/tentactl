use rmcp::schemars::{self, JsonSchema};
use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_router, ErrorData as McpError,
};
use serde::Deserialize;

use crate::kraken::types::{AddOrderBatchOrder, CancelOrderBatchItem};
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
pub struct PlaceOrderParams {
    #[schemars(description = "Trading pair (e.g. XBTUSD, ETHUSD)")]
    pub pair: String,
    #[schemars(description = "Order direction: 'buy' or 'sell'")]
    pub direction: String,
    #[schemars(
        description = "Order type: 'market', 'limit', 'stop-loss', 'take-profit', 'stop-loss-limit', 'take-profit-limit', 'trailing-stop', 'trailing-stop-limit'"
    )]
    pub order_type: String,
    #[schemars(description = "Volume in base currency (e.g. '0.01' for 0.01 BTC)")]
    pub volume: String,
    #[schemars(description = "Limit/trigger price (required for non-market orders)")]
    pub price: Option<String>,
    #[schemars(description = "If true, validate without placing (dry run). Recommended.")]
    pub validate: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CancelOrderParams {
    #[schemars(
        description = "Transaction ID, user reference, or client order ID of the order to cancel"
    )]
    pub txid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddOrderBatchParams {
    #[schemars(description = "Trading pair for all orders (e.g. XBTUSD)")]
    pub pair: String,
    #[schemars(
        description = r#"JSON array of order objects (2-15 orders). Each order needs: ordertype, type (buy/sell), volume. Optional: price, price2, leverage, oflags, timeinforce, starttm, expiretm, cl_ord_id. Example: [{"ordertype":"limit","type":"buy","volume":"0.01","price":"30000"},{"ordertype":"limit","type":"sell","volume":"0.01","price":"35000"}]"#
    )]
    pub orders: String,
    #[schemars(description = "If true, validate without placing. Recommended for first test.")]
    pub validate: Option<bool>,
    #[schemars(description = "RFC3339 deadline after which to reject the batch (optional)")]
    pub deadline: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AmendOrderParams {
    #[schemars(
        description = "Kraken transaction ID of the order to amend (mutually exclusive with cl_ord_id)"
    )]
    pub txid: Option<String>,
    #[schemars(description = "Client order ID to amend (mutually exclusive with txid)")]
    pub cl_ord_id: Option<String>,
    #[schemars(description = "New order quantity in base asset")]
    pub order_qty: Option<String>,
    #[schemars(description = "New limit price (for limit/iceberg orders)")]
    pub limit_price: Option<String>,
    #[schemars(description = "New trigger price (for stop/take-profit orders)")]
    pub trigger_price: Option<String>,
    #[schemars(description = "If true, reject if order cannot be posted passively")]
    pub post_only: Option<bool>,
    #[schemars(description = "Pair (required for xstock pairs)")]
    pub pair: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditOrderParams {
    #[schemars(description = "Transaction ID of the order to edit")]
    pub txid: String,
    #[schemars(description = "Trading pair (e.g. XBTUSD)")]
    pub pair: String,
    #[schemars(description = "New order volume")]
    pub volume: Option<String>,
    #[schemars(description = "New limit/trigger price")]
    pub price: Option<String>,
    #[schemars(description = "New secondary price (for stop-loss-limit etc.)")]
    pub price2: Option<String>,
    #[schemars(description = "Comma-delimited order flags (e.g. 'post')")]
    pub oflags: Option<String>,
    #[schemars(description = "If true, validate only without executing")]
    pub validate: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CancelAllAfterParams {
    #[schemars(
        description = "Timeout in seconds (0 to disable). All orders cancelled after this. Use 60s with 15-30s refresh."
    )]
    pub timeout: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CancelOrderBatchParams {
    #[schemars(
        description = r#"JSON array of order identifiers (max 50). Each entry has one of: txid, userref, or cl_ord_id. Example: [{"txid":"OABC12-..."}, {"userref":12345}]"#
    )]
    pub orders: String,
}

// === Tool implementations ===

#[tool_router(router = spot_trading_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    #[tool(
        name = "place_order",
        description = "⚠️ REAL MONEY — Place a buy/sell order on Kraken. Market orders execute IMMEDIATELY at market price. Use validate=true to dry-run first. ALWAYS confirm with user before placing. Requires API keys."
    )]
    pub async fn place_order(
        &self,
        Parameters(p): Parameters<PlaceOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        let validate = p.validate.unwrap_or(false);
        match self
            .client
            .add_order(
                &p.pair,
                &p.direction,
                &p.order_type,
                &p.volume,
                p.price.as_deref(),
                validate,
            )
            .await
        {
            Ok(result) => {
                let mut text = if validate {
                    "VALIDATION ONLY (no order placed)\n\n".to_string()
                } else {
                    "ORDER PLACED — REAL TRADE EXECUTED\n\n".to_string()
                };
                if let Some(descr) = &result.descr {
                    if let Some(order) = &descr.order {
                        text.push_str(&format!("Order: {order}\n"));
                    }
                }
                if let Some(txids) = &result.txid {
                    text.push_str(&format!("Transaction IDs: {}\n", txids.join(", ")));
                }
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "add_order_batch",
        description = "⚠️ REAL MONEY — Place 2-15 orders atomically for a single pair. All orders validated before submission. Use validate=true first. Requires API keys."
    )]
    pub async fn add_order_batch(
        &self,
        Parameters(p): Parameters<AddOrderBatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let orders: Vec<AddOrderBatchOrder> = match serde_json::from_str(&p.orders) {
            Ok(v) => v,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid orders JSON: {e}"
                ))]));
            }
        };
        match self
            .client
            .add_order_batch(&p.pair, orders, p.validate, p.deadline.as_deref())
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "amend_order",
        description = "Amend an open order in-place (preserves queue priority). Requires txid or cl_ord_id. Requires API keys."
    )]
    pub async fn amend_order(
        &self,
        Parameters(p): Parameters<AmendOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .amend_order(
                p.txid.as_deref(),
                p.cl_ord_id.as_deref(),
                p.order_qty.as_deref(),
                p.limit_price.as_deref(),
                p.trigger_price.as_deref(),
                p.post_only,
                p.pair.as_deref(),
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "edit_order",
        description = "Edit an open order (cancels original, creates new with new txid). Consider amend_order for in-place edits. Requires API keys."
    )]
    pub async fn edit_order(
        &self,
        Parameters(p): Parameters<EditOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .edit_order(
                &p.txid,
                &p.pair,
                p.volume.as_deref(),
                p.price.as_deref(),
                p.price2.as_deref(),
                p.oflags.as_deref(),
                p.validate,
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "cancel_order",
        description = "Cancel an open order by transaction ID, userref, or cl_ord_id. Requires API keys."
    )]
    pub async fn cancel_order(
        &self,
        Parameters(p): Parameters<CancelOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.cancel_order(&p.txid).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Cancelled {} order(s)",
                result.count.unwrap_or(0)
            ))])),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "cancel_all_orders",
        description = "⚠️ Cancel ALL open orders. This affects every open order on the account. Requires API keys."
    )]
    pub async fn cancel_all_orders(&self) -> Result<CallToolResult, McpError> {
        match self.client.cancel_all_orders().await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Cancelled {} order(s)",
                result.count.unwrap_or(0)
            ))])),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "cancel_all_after",
        description = "Dead man's switch: cancel all orders after timeout seconds. Set timeout=0 to disable. Call every 15-30s with timeout=60 for protection. Requires API keys."
    )]
    pub async fn cancel_all_after(
        &self,
        Parameters(p): Parameters<CancelAllAfterParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.cancel_all_after(p.timeout).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "cancel_order_batch",
        description = "Cancel up to 50 orders by txid, userref, or cl_ord_id in a single request. Requires API keys."
    )]
    pub async fn cancel_order_batch(
        &self,
        Parameters(p): Parameters<CancelOrderBatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let orders: Vec<CancelOrderBatchItem> = match serde_json::from_str(&p.orders) {
            Ok(v) => v,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid orders JSON: {e}"
                ))]));
            }
        };
        match self.client.cancel_order_batch(orders).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }
}

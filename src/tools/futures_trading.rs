use rmcp::schemars::{self, JsonSchema};
use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_router, ErrorData as McpError,
};
use serde::Deserialize;

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

// === Param types ===

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FillsParams {
    #[schemars(description = "ISO8601 timestamp; return fills after this time (optional)")]
    pub last_fill_time: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TransfersParams {
    #[schemars(description = "ISO8601 timestamp; return transfers after this time (optional)")]
    pub last_transfer_time: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OrderStatusParams {
    #[schemars(description = "Comma-separated list of order IDs to query (e.g. 'abc123,def456')")]
    pub order_ids: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeverageSettingParams {
    #[schemars(description = "Futures symbol (e.g. PF_XBTUSD). Required when setting leverage.")]
    pub symbol: Option<String>,
    #[schemars(
        description = "Maximum leverage to set (e.g. '10'). If provided, updates leverage (PUT). If absent, retrieves current setting (GET)."
    )]
    pub max_leverage: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PnlPreferenceParams {
    #[schemars(description = "Futures symbol (e.g. PF_XBTUSD). Required when setting preference.")]
    pub symbol: Option<String>,
    #[schemars(
        description = "PnL currency preference (e.g. 'USD', 'BTC'). If provided, updates preference (PUT). If absent, retrieves current preference (GET)."
    )]
    pub pnl_preference: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendOrderParams {
    #[schemars(
        description = "Order type: 'lmt' (limit), 'mkt' (market), 'stp' (stop), 'take_profit', 'trailing_stop'"
    )]
    pub order_type: String,
    #[schemars(description = "Futures symbol (e.g. PF_XBTUSD, PI_ETHUSD)")]
    pub symbol: String,
    #[schemars(description = "Order side: 'buy' or 'sell'")]
    pub side: String,
    #[schemars(description = "Order size in contracts")]
    pub size: String,
    #[schemars(description = "Limit price (required for lmt orders)")]
    pub limit_price: Option<String>,
    #[schemars(description = "Stop/trigger price (for stop and take_profit orders)")]
    pub stop_price: Option<String>,
    #[schemars(description = "Client order ID for tracking (optional)")]
    pub client_order_id: Option<String>,
    #[schemars(description = "If true, only reduces existing position (optional)")]
    pub reduce_only: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditOrderParams {
    #[schemars(description = "Order ID to edit (mutually exclusive with client_order_id)")]
    pub order_id: Option<String>,
    #[schemars(description = "Client order ID to edit (mutually exclusive with order_id)")]
    pub client_order_id: Option<String>,
    #[schemars(description = "New order size in contracts")]
    pub size: Option<String>,
    #[schemars(description = "New limit price")]
    pub limit_price: Option<String>,
    #[schemars(description = "New stop/trigger price")]
    pub stop_price: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CancelOrderParams {
    #[schemars(description = "Order ID to cancel (mutually exclusive with client_order_id)")]
    pub order_id: Option<String>,
    #[schemars(description = "Client order ID to cancel (mutually exclusive with order_id)")]
    pub client_order_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CancelAllParams {
    #[schemars(
        description = "Cancel only orders for this symbol (optional; cancels all if omitted)"
    )]
    pub symbol: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchOrderParams {
    #[schemars(
        description = r#"JSON array of batch instructions. Each entry has an "order" field: "send", "cancel", or "edit", plus the relevant fields for that action. Example: [{"order":"send","orderType":"lmt","symbol":"PF_XBTUSD","side":"buy","size":1,"limitPrice":50000},{"order":"cancel","order_id":"abc123"}]"#
    )]
    pub instructions: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TransferParams {
    #[schemars(
        description = "Source account (e.g. 'Futures Wallet', 'Cash/Collateral Account', or sub-account name)"
    )]
    pub from_account: String,
    #[schemars(description = "Destination account (same format as from_account)")]
    pub to_account: String,
    #[schemars(description = "Currency/asset to transfer (e.g. 'USD', 'BTC', 'ETH')")]
    pub unit: String,
    #[schemars(description = "Amount to transfer")]
    pub amount: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WithdrawalParams {
    #[schemars(description = "Withdrawal destination address")]
    pub target_address: String,
    #[schemars(description = "Currency to withdraw (e.g. 'USD', 'BTC')")]
    pub currency: String,
    #[schemars(description = "Amount to withdraw")]
    pub amount: String,
}

// === Tool implementations ===

#[tool_router(router = futures_trading_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    #[tool(
        name = "futures_accounts",
        description = "Get Kraken Futures account balances, margin requirements, margin trigger estimates, and auxiliary info for all cash and margin accounts. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_accounts(&self) -> Result<CallToolResult, McpError> {
        match self.futures_client.accounts().await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_open_orders",
        description = "List all open Kraken Futures orders on the account. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_open_orders(&self) -> Result<CallToolResult, McpError> {
        match self.futures_client.open_orders().await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_open_positions",
        description = "List all open Kraken Futures positions on the account. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_open_positions(&self) -> Result<CallToolResult, McpError> {
        match self.futures_client.open_positions().await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_fills",
        description = "Get Kraken Futures trade fill history. Optionally filter to fills after a given time. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_fills(
        &self,
        Parameters(p): Parameters<FillsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.futures_client.fills(p.last_fill_time.as_deref()).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_transfers",
        description = "Get Kraken Futures transfer history between accounts. Optionally filter to transfers after a given time. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_transfers(
        &self,
        Parameters(p): Parameters<TransfersParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .futures_client
            .transfers(p.last_transfer_time.as_deref())
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_order_status",
        description = "Get the status of one or more Kraken Futures orders by ID. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_order_status(
        &self,
        Parameters(p): Parameters<OrderStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.futures_client.order_status(&p.order_ids).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_leverage_setting",
        description = "Get or set Kraken Futures leverage. Omit max_leverage to read the current setting. Provide symbol + max_leverage to update it. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_leverage_setting(
        &self,
        Parameters(p): Parameters<LeverageSettingParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = if let Some(ref lev) = p.max_leverage {
            let symbol = match p.symbol.as_deref() {
                Some(s) => s,
                None => {
                    return Ok(CallToolResult::error(vec![Content::text(
                        "symbol is required when setting max_leverage",
                    )]))
                }
            };
            self.futures_client.set_leverage_setting(symbol, lev).await
        } else {
            self.futures_client
                .get_leverage_setting(p.symbol.as_deref())
                .await
        };
        match result {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_pnl_preference",
        description = "Get or set the Kraken Futures PnL currency preference for a symbol. Omit pnl_preference to read the current setting. Provide symbol + pnl_preference to update it. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_pnl_preference(
        &self,
        Parameters(p): Parameters<PnlPreferenceParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = if let Some(ref pref) = p.pnl_preference {
            let symbol = match p.symbol.as_deref() {
                Some(s) => s,
                None => {
                    return Ok(CallToolResult::error(vec![Content::text(
                        "symbol is required when setting pnl_preference",
                    )]))
                }
            };
            self.futures_client.set_pnl_preference(symbol, pref).await
        } else {
            self.futures_client
                .get_pnl_preference(p.symbol.as_deref())
                .await
        };
        match result {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_send_order",
        description = "⚠️ REAL MONEY — Place a new Kraken Futures order. Supported order types: lmt, mkt, stp, take_profit, trailing_stop. ALWAYS confirm with user before placing. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_send_order(
        &self,
        Parameters(p): Parameters<SendOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .futures_client
            .send_order(
                &p.order_type,
                &p.symbol,
                &p.side,
                &p.size,
                p.limit_price.as_deref(),
                p.stop_price.as_deref(),
                p.client_order_id.as_deref(),
                p.reduce_only,
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_edit_order",
        description = "Edit an open Kraken Futures order (size and/or price). Provide order_id or client_order_id to identify the order. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_edit_order(
        &self,
        Parameters(p): Parameters<EditOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .futures_client
            .edit_order(
                p.order_id.as_deref(),
                p.client_order_id.as_deref(),
                p.size.as_deref(),
                p.limit_price.as_deref(),
                p.stop_price.as_deref(),
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_cancel_order",
        description = "Cancel an open Kraken Futures order by order_id or client_order_id. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_cancel_order(
        &self,
        Parameters(p): Parameters<CancelOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .futures_client
            .cancel_order(p.order_id.as_deref(), p.client_order_id.as_deref())
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_cancel_all",
        description = "⚠️ Cancel all open Kraken Futures orders, optionally filtered to a single symbol. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_cancel_all(
        &self,
        Parameters(p): Parameters<CancelAllParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.futures_client.cancel_all(p.symbol.as_deref()).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_batch_order",
        description = "⚠️ REAL MONEY — Execute a batch of Kraken Futures order operations (send/cancel/edit) in a single request. Pass a JSON array of instructions. ALWAYS confirm with user before submitting. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_batch_order(
        &self,
        Parameters(p): Parameters<BatchOrderParams>,
    ) -> Result<CallToolResult, McpError> {
        let instructions: Vec<FuturesBatchInstruction> = match serde_json::from_str(&p.instructions)
        {
            Ok(v) => v,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid instructions JSON: {e}"
                ))]))
            }
        };
        if instructions.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "instructions must be a non-empty JSON array",
            )]));
        }
        match self.futures_client.batch_order(instructions).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_transfer",
        description = "⚠️ Transfer funds between Kraken Futures accounts (e.g. between cash collateral and futures wallet). Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_transfer(
        &self,
        Parameters(p): Parameters<TransferParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .futures_client
            .transfer(&p.from_account, &p.to_account, &p.unit, &p.amount)
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "futures_withdrawal",
        description = "⚠️ REAL MONEY — Withdraw funds from a Kraken Futures account to an external address. CONFIRM with user before executing. Requires KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET."
    )]
    pub async fn futures_withdrawal(
        &self,
        Parameters(p): Parameters<WithdrawalParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .futures_client
            .withdrawal(&p.target_address, &p.currency, &p.amount)
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }
}

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
pub struct GetTradeHistoryParams {
    #[schemars(description = "Result offset for pagination")]
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTradeBalanceParams {
    #[schemars(description = "Base asset used to determine balance (default: ZUSD)")]
    pub asset: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetClosedOrdersParams {
    #[schemars(description = "Whether to include trades in results")]
    pub trades: Option<bool>,
    #[schemars(description = "Start timestamp (Unix) for filtering")]
    pub start: Option<u64>,
    #[schemars(description = "End timestamp (Unix) for filtering")]
    pub end: Option<u64>,
    #[schemars(description = "Result offset for pagination")]
    pub ofs: Option<u32>,
    #[schemars(description = "Time to use for filtering: open, close, both (default: both)")]
    pub closetime: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryOrdersParams {
    #[schemars(description = "Comma-delimited list of transaction IDs to query")]
    pub txid: String,
    #[schemars(description = "Whether to include related trades")]
    pub trades: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTradesInfoParams {
    #[schemars(description = "Comma-delimited list of trade transaction IDs")]
    pub txid: String,
    #[schemars(description = "Whether to include related trades")]
    pub trades: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetOpenPositionsParams {
    #[schemars(description = "Comma-delimited list of txids to filter (optional)")]
    pub txid: Option<String>,
    #[schemars(description = "Whether to include P&L calculations")]
    pub docalcs: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLedgerParams {
    #[schemars(description = "Comma-delimited list of assets to filter, or 'all' (default: all)")]
    pub asset: Option<String>,
    #[schemars(
        description = "Ledger type: all, trade, deposit, withdrawal, transfer, margin, rollover, credit, settled, staking, dividend (default: all)"
    )]
    pub ledger_type: Option<String>,
    #[schemars(description = "Start timestamp (Unix) for filtering")]
    pub start: Option<u64>,
    #[schemars(description = "End timestamp (Unix) for filtering")]
    pub end: Option<u64>,
    #[schemars(description = "Result offset for pagination")]
    pub ofs: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryLedgerParams {
    #[schemars(description = "Comma-delimited list of ledger IDs to query")]
    pub id: String,
    #[schemars(description = "Whether to include related trades")]
    pub trades: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTradeVolumeParams {
    #[schemars(description = "Comma-delimited list of pairs for fee schedule (optional)")]
    pub pair: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetOrderAmendsParams {
    #[schemars(description = "Transaction ID of the order to inspect amend history for")]
    pub txid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddExportParams {
    #[schemars(description = "Type of data to export: trades or ledgers")]
    pub report: String,
    #[schemars(description = "Human-readable description for the export")]
    pub description: String,
    #[schemars(description = "File format: CSV or TSV (default: CSV)")]
    pub format: Option<String>,
    #[schemars(description = "Start time as Unix timestamp")]
    pub starttm: Option<u64>,
    #[schemars(description = "End time as Unix timestamp")]
    pub endtm: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportStatusParams {
    #[schemars(description = "Type of report to check: trades or ledgers")]
    pub report: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RetrieveExportParams {
    #[schemars(description = "Report ID to retrieve (from add_export or export_status)")]
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveExportParams {
    #[schemars(description = "Report ID to delete or cancel")]
    pub id: String,
    #[schemars(
        description = "Action: 'delete' (processed reports) or 'cancel' (queued/processing)"
    )]
    pub remove_type: String,
}

// === Tool implementations ===

#[tool_router(router = spot_account_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    #[tool(
        name = "get_balance",
        description = "Get all non-zero account balances. Requires KRAKEN_API_KEY and KRAKEN_API_SECRET env vars."
    )]
    pub async fn get_balance(&self) -> Result<CallToolResult, McpError> {
        match self.client.balance().await {
            Ok(balances) => {
                let non_zero: std::collections::HashMap<_, _> = balances
                    .into_iter()
                    .filter(|(_, v)| v.parse::<f64>().unwrap_or(0.0) != 0.0)
                    .collect();
                ok(&non_zero)
            }
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_credit_lines",
        description = "Get account credit line details. Requires API keys."
    )]
    pub async fn get_credit_lines(&self) -> Result<CallToolResult, McpError> {
        match self.client.credit_lines().await {
            Ok(lines) => ok(&lines),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_extended_balance",
        description = "Get extended account balances including credit and held amounts. Requires API keys."
    )]
    pub async fn get_extended_balance(&self) -> Result<CallToolResult, McpError> {
        match self.client.extended_balance().await {
            Ok(b) => ok(&b),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_trade_balance",
        description = "Get collateral balances, margin valuations, and equity summary. Requires API keys."
    )]
    pub async fn get_trade_balance(
        &self,
        Parameters(p): Parameters<GetTradeBalanceParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.trade_balance(p.asset.as_deref()).await {
            Ok(b) => ok(&b),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_open_orders",
        description = "List all open orders. Requires API keys."
    )]
    pub async fn get_open_orders(&self) -> Result<CallToolResult, McpError> {
        match self.client.open_orders().await {
            Ok(orders) => ok(&orders.open),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_closed_orders",
        description = "Get closed (filled/cancelled) orders. 50 per page, most recent first. Requires API keys."
    )]
    pub async fn get_closed_orders(
        &self,
        Parameters(p): Parameters<GetClosedOrdersParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .closed_orders(p.trades, p.start, p.end, p.ofs, p.closetime.as_deref())
            .await
        {
            Ok(orders) => ok(&orders),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "query_orders",
        description = "Get info about specific orders by transaction ID. Requires API keys."
    )]
    pub async fn query_orders(
        &self,
        Parameters(p): Parameters<QueryOrdersParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.query_orders(&p.txid, p.trades).await {
            Ok(orders) => ok(&orders),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_order_amends",
        description = "Get amend audit trail for a specific order transaction ID. Requires API keys."
    )]
    pub async fn get_order_amends(
        &self,
        Parameters(p): Parameters<GetOrderAmendsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.order_amends(&p.txid).await {
            Ok(amends) => ok(&amends),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_trade_history",
        description = "Get recent executed trades with pair, price, volume, cost, fee. Requires API keys."
    )]
    pub async fn get_trade_history(
        &self,
        Parameters(p): Parameters<GetTradeHistoryParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.trade_history(p.offset).await {
            Ok(history) => ok(&history.trades),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_trades_info",
        description = "Get info about specific trades by transaction ID. Requires API keys."
    )]
    pub async fn get_trades_info(
        &self,
        Parameters(p): Parameters<GetTradesInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.trades_info(&p.txid, p.trades).await {
            Ok(t) => ok(&t),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_open_positions",
        description = "Get open margin positions with cost, fee, P&L. Requires API keys."
    )]
    pub async fn get_open_positions(
        &self,
        Parameters(p): Parameters<GetOpenPositionsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .open_positions(p.txid.as_deref(), p.docalcs)
            .await
        {
            Ok(pos) => ok(&pos),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_ledger",
        description = "Get ledger entries (trades, deposits, withdrawals, etc). 50 per page. Requires API keys."
    )]
    pub async fn get_ledger(
        &self,
        Parameters(p): Parameters<GetLedgerParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .ledger(
                p.asset.as_deref(),
                p.ledger_type.as_deref(),
                p.start,
                p.end,
                p.ofs,
            )
            .await
        {
            Ok(l) => ok(&l),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "query_ledger",
        description = "Get specific ledger entries by ID. Requires API keys."
    )]
    pub async fn query_ledger(
        &self,
        Parameters(p): Parameters<QueryLedgerParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.query_ledger(&p.id, p.trades).await {
            Ok(l) => ok(&l),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_trade_volume",
        description = "Get 30-day USD trading volume and fee schedule for a pair. Requires API keys."
    )]
    pub async fn get_trade_volume(
        &self,
        Parameters(p): Parameters<GetTradeVolumeParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.trade_volume(p.pair.as_deref()).await {
            Ok(v) => ok(&v),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "add_export",
        description = "Request an export of trades or ledger data. Returns a report ID. Requires API keys."
    )]
    pub async fn add_export(
        &self,
        Parameters(p): Parameters<AddExportParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .add_export(
                &p.report,
                &p.description,
                p.format.as_deref(),
                p.starttm,
                p.endtm,
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "export_status",
        description = "Get status of requested data exports (Queued/Processing/Processed). Requires API keys."
    )]
    pub async fn export_status(
        &self,
        Parameters(p): Parameters<ExportStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.export_status(&p.report).await {
            Ok(s) => ok(&s),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "retrieve_export",
        description = "Download a processed export report as a base64-encoded zip archive. Requires API keys."
    )]
    pub async fn retrieve_export(
        &self,
        Parameters(p): Parameters<RetrieveExportParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.retrieve_export(&p.id).await {
            Ok(bytes) => {
                use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
                let encoded = BASE64.encode(&bytes);
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Report ID: {}\nSize: {} bytes\nBase64 zip data:\n{}",
                    p.id,
                    bytes.len(),
                    encoded
                ))]))
            }
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "remove_export",
        description = "Delete or cancel a data export report. Use 'cancel' for queued/processing, 'delete' for processed. Requires API keys."
    )]
    pub async fn remove_export(
        &self,
        Parameters(p): Parameters<RemoveExportParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.remove_export(&p.id, &p.remove_type).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }
}

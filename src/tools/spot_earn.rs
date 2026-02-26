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
pub struct EarnStrategiesParams {
    #[schemars(description = "Filter by asset name (e.g. DOT, ETH)")]
    pub asset: Option<String>,
    #[schemars(description = "Number of items per page")]
    pub limit: Option<u32>,
    #[schemars(description = "Pagination cursor from previous response")]
    pub cursor: Option<String>,
    #[schemars(description = "If true, sort ascending; false (default) for descending")]
    pub ascending: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EarnAllocationsParams {
    #[schemars(description = "Currency to convert allocation values to (default: USD)")]
    pub converted_asset: Option<String>,
    #[schemars(description = "If true, hide strategies with zero balance")]
    pub hide_zero: Option<bool>,
    #[schemars(description = "If true, sort ascending")]
    pub ascending: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EarnAllocateParams {
    #[schemars(description = "Earn strategy ID (from earn_strategies)")]
    pub strategy_id: String,
    #[schemars(
        description = "Amount to allocate (⚠️ may have lock period — check strategy lock_type first)"
    )]
    pub amount: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EarnDeallocateParams {
    #[schemars(description = "Earn strategy ID (from earn_strategies)")]
    pub strategy_id: String,
    #[schemars(description = "Amount to deallocate (⚠️ bonded strategies have unbonding period)")]
    pub amount: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EarnStatusParams {
    #[schemars(description = "Earn strategy ID to check operation status for")]
    pub strategy_id: String,
}

// === Tool implementations ===

#[tool_router(router = spot_earn_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    #[tool(
        name = "earn_strategies",
        description = "List available earn/staking strategies with APR, lock types, and allocation limits. Requires API keys."
    )]
    pub async fn earn_strategies(
        &self,
        Parameters(p): Parameters<EarnStrategiesParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .earn_strategies(
                p.asset.as_deref(),
                p.limit,
                p.cursor.as_deref(),
                p.ascending,
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "earn_allocations",
        description = "List all current earn allocations including bonding/unbonding status. Requires API keys."
    )]
    pub async fn earn_allocations(
        &self,
        Parameters(p): Parameters<EarnAllocationsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .earn_allocations(p.converted_asset.as_deref(), p.hide_zero, p.ascending)
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "earn_allocate",
        description = "⚠️ Allocate funds to an earn strategy. Check strategy lock_type first — bonded strategies have lock periods. This is asynchronous; poll earn_allocate_status to confirm. Requires API keys."
    )]
    pub async fn earn_allocate(
        &self,
        Parameters(p): Parameters<EarnAllocateParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.earn_allocate(&p.strategy_id, &p.amount).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "earn_deallocate",
        description = "⚠️ Deallocate funds from an earn strategy. Bonded strategies have unbonding periods before funds are available. Asynchronous — poll earn_deallocate_status to confirm. Requires API keys."
    )]
    pub async fn earn_deallocate(
        &self,
        Parameters(p): Parameters<EarnDeallocateParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.earn_deallocate(&p.strategy_id, &p.amount).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "earn_allocate_status",
        description = "Check status of the last allocation request for a strategy. Returns pending=true if in progress. Requires API keys."
    )]
    pub async fn earn_allocate_status(
        &self,
        Parameters(p): Parameters<EarnStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.earn_allocate_status(&p.strategy_id).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "earn_deallocate_status",
        description = "Check status of the last deallocation request for a strategy. Returns pending=true if in progress. Requires API keys."
    )]
    pub async fn earn_deallocate_status(
        &self,
        Parameters(p): Parameters<EarnStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.earn_deallocate_status(&p.strategy_id).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }
}

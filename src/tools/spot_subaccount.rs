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
pub struct CreateSubaccountParams {
    #[schemars(description = "Username for the new subaccount")]
    pub username: String,
    #[schemars(description = "Email address for the new subaccount")]
    pub email: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AccountTransferParams {
    #[schemars(description = "Asset to transfer (e.g. XBT, ETH)")]
    pub asset: String,
    #[schemars(description = "Amount to transfer")]
    pub amount: String,
    #[schemars(description = "IIBAN of the source account")]
    pub from: String,
    #[schemars(description = "IIBAN of the destination account")]
    pub to: String,
}

// === Tool implementations ===

#[tool_router(router = spot_subaccount_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    #[tool(
        name = "create_subaccount",
        description = "Create a trading subaccount. Must be called with master account API key. Requires API keys."
    )]
    pub async fn create_subaccount(
        &self,
        Parameters(p): Parameters<CreateSubaccountParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.create_subaccount(&p.username, &p.email).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "account_transfer",
        description = "⚠️ Transfer funds between master and subaccounts using IIBANs. Must use master account API key. Requires API keys."
    )]
    pub async fn account_transfer(
        &self,
        Parameters(p): Parameters<AccountTransferParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .account_transfer(&p.asset, &p.amount, &p.from, &p.to)
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }
}

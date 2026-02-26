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
pub struct GetDepositMethodsParams {
    #[schemars(description = "Asset to get deposit methods for (e.g. XBT, ETH)")]
    pub asset: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDepositAddressesParams {
    #[schemars(description = "Asset to get deposit addresses for (e.g. XBT, ETH)")]
    pub asset: String,
    #[schemars(description = "Name of the deposit method (from get_deposit_methods)")]
    pub method: String,
    #[schemars(description = "If true, generate a new deposit address")]
    pub new: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDepositStatusParams {
    #[schemars(description = "Filter by asset (optional)")]
    pub asset: Option<String>,
    #[schemars(description = "Filter by method (optional)")]
    pub method: Option<String>,
    #[schemars(description = "Pagination cursor (optional)")]
    pub cursor: Option<String>,
    #[schemars(description = "Max results per page (optional)")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetWithdrawMethodsParams {
    #[schemars(description = "Filter by asset (optional)")]
    pub asset: Option<String>,
    #[schemars(description = "Filter by asset class (optional)")]
    pub aclass: Option<String>,
    #[schemars(description = "Filter by network (optional)")]
    pub network: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetWithdrawAddressesParams {
    #[schemars(description = "Filter by asset (optional)")]
    pub asset: Option<String>,
    #[schemars(description = "Filter by asset class (optional)")]
    pub aclass: Option<String>,
    #[schemars(description = "Filter by withdrawal method (optional)")]
    pub method: Option<String>,
    #[schemars(description = "Filter by withdrawal key name (optional)")]
    pub key: Option<String>,
    #[schemars(description = "If true, only return verified addresses")]
    pub verified: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetWithdrawInfoParams {
    #[schemars(description = "Asset to withdraw (e.g. XBT)")]
    pub asset: String,
    #[schemars(description = "Withdrawal key name (as configured in Kraken account)")]
    pub key: String,
    #[schemars(description = "Amount to withdraw")]
    pub amount: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WithdrawParams {
    #[schemars(description = "Asset to withdraw (e.g. XBT)")]
    pub asset: String,
    #[schemars(description = "Withdrawal key name (as configured in Kraken account)")]
    pub key: String,
    #[schemars(description = "Amount to withdraw")]
    pub amount: String,
    #[schemars(description = "Optional crypto address to confirm matches the key")]
    pub address: Option<String>,
    #[schemars(description = "Maximum acceptable fee — withdrawal fails if fee exceeds this")]
    pub max_fee: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetWithdrawStatusParams {
    #[schemars(description = "Filter by asset (optional)")]
    pub asset: Option<String>,
    #[schemars(description = "Filter by method (optional)")]
    pub method: Option<String>,
    #[schemars(description = "Pagination cursor (optional)")]
    pub cursor: Option<String>,
    #[schemars(description = "Max results per page (optional)")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CancelWithdrawParams {
    #[schemars(description = "Asset of the withdrawal to cancel (e.g. XBT)")]
    pub asset: String,
    #[schemars(description = "Reference ID of the withdrawal to cancel")]
    pub refid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WalletTransferParams {
    #[schemars(description = "Asset to transfer from Spot to Futures wallet (e.g. XBT)")]
    pub asset: String,
    #[schemars(description = "Amount to transfer")]
    pub amount: String,
}

// === Tool implementations ===

#[tool_router(router = spot_funding_router, vis = "pub(crate)")]
impl KrakenMcpServer {
    #[tool(
        name = "get_deposit_methods",
        description = "Get available deposit methods for an asset. Requires API keys."
    )]
    pub async fn get_deposit_methods(
        &self,
        Parameters(p): Parameters<GetDepositMethodsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.deposit_methods(&p.asset).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_deposit_addresses",
        description = "Get or generate deposit addresses for an asset and method. Requires API keys."
    )]
    pub async fn get_deposit_addresses(
        &self,
        Parameters(p): Parameters<GetDepositAddressesParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .deposit_addresses(&p.asset, &p.method, p.new)
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_deposit_status",
        description = "Get status of recent deposits (sorted newest first). Requires API keys."
    )]
    pub async fn get_deposit_status(
        &self,
        Parameters(p): Parameters<GetDepositStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .deposit_status(
                p.asset.as_deref(),
                p.method.as_deref(),
                p.cursor.as_deref(),
                p.limit,
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_withdraw_methods",
        description = "Get available withdrawal methods. Requires API keys."
    )]
    pub async fn get_withdraw_methods(
        &self,
        Parameters(p): Parameters<GetWithdrawMethodsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .withdraw_methods(
                p.asset.as_deref(),
                p.aclass.as_deref(),
                p.network.as_deref(),
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_withdraw_addresses",
        description = "Get configured withdrawal addresses. Requires API keys."
    )]
    pub async fn get_withdraw_addresses(
        &self,
        Parameters(p): Parameters<GetWithdrawAddressesParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .withdraw_addresses(
                p.asset.as_deref(),
                p.aclass.as_deref(),
                p.method.as_deref(),
                p.key.as_deref(),
                p.verified,
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_withdraw_info",
        description = "Get fee and limit info for a potential withdrawal. Requires API keys."
    )]
    pub async fn get_withdraw_info(
        &self,
        Parameters(p): Parameters<GetWithdrawInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.withdraw_info(&p.asset, &p.key, &p.amount).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "withdraw",
        description = "⚠️ REAL MONEY — Withdraw funds from Kraken to an external address. Use get_withdraw_info first to check fees. ALWAYS confirm with user before executing. Requires API keys."
    )]
    pub async fn withdraw(
        &self,
        Parameters(p): Parameters<WithdrawParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .withdraw(
                &p.asset,
                &p.key,
                &p.amount,
                p.address.as_deref(),
                p.max_fee.as_deref(),
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "get_withdraw_status",
        description = "Get status of recent withdrawals (sorted newest first). Requires API keys."
    )]
    pub async fn get_withdraw_status(
        &self,
        Parameters(p): Parameters<GetWithdrawStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client
            .withdraw_status(
                p.asset.as_deref(),
                p.method.as_deref(),
                p.cursor.as_deref(),
                p.limit,
            )
            .await
        {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "cancel_withdraw",
        description = "Cancel a pending withdrawal before it is processed. Requires API keys."
    )]
    pub async fn cancel_withdraw(
        &self,
        Parameters(p): Parameters<CancelWithdrawParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.cancel_withdraw(&p.asset, &p.refid).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }

    #[tool(
        name = "wallet_transfer",
        description = "⚠️ Transfer funds from Spot Wallet to Futures Wallet. Requires API keys."
    )]
    pub async fn wallet_transfer(
        &self,
        Parameters(p): Parameters<WalletTransferParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.wallet_transfer(&p.asset, &p.amount).await {
            Ok(r) => ok(&r),
            Err(e) => err(e),
        }
    }
}

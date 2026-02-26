use rmcp::{handler::server::tool::ToolRouter, model::*, tool_handler, ServerHandler};

use crate::kraken::client::KrakenClient;
use crate::kraken::futures_client::FuturesClient;
use crate::kraken::ws::WsClient;
use crate::kraken::ws_futures::FuturesWsClient;

/// The MCP server. Tool implementations live in `crate::tools::spot_*`,
/// `crate::tools::futures_*`, `crate::tools::ws_spot`, and
/// `crate::tools::ws_futures` modules, each contributing a named ToolRouter
/// that is composed here.
#[derive(Clone)]
pub struct KrakenMcpServer {
    pub(crate) client: KrakenClient,
    pub(crate) futures_client: FuturesClient,
    pub(crate) ws_client: WsClient,
    pub(crate) futures_ws: FuturesWsClient,
    tool_router: ToolRouter<Self>,
}

impl Default for KrakenMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl KrakenMcpServer {
    pub fn new() -> Self {
        let rest = KrakenClient::from_env();
        let tool_router = Self::spot_market_router()
            + Self::spot_account_router()
            + Self::spot_trading_router()
            + Self::spot_funding_router()
            + Self::spot_earn_router()
            + Self::spot_subaccount_router()
            + Self::futures_market_router()
            + Self::futures_trading_router()
            + Self::ws_spot_router()
            + Self::ws_futures_router();

        Self {
            ws_client: WsClient::new(rest.clone()),
            client: rest,
            futures_client: FuturesClient::from_env(),
            futures_ws: FuturesWsClient::from_env(),
            tool_router,
        }
    }
}

#[tool_handler]
impl ServerHandler for KrakenMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Kraken cryptocurrency exchange MCP server.\n\
                 Spot public tools (no auth): get_ticker, get_orderbook, get_ohlc, get_recent_trades, \
                 get_spread, get_assets, get_asset_pairs, get_server_time, get_system_status.\n\
                 Spot private tools require KRAKEN_API_KEY and KRAKEN_API_SECRET env vars.\n\
                 Futures public tools (no auth): futures_instruments, futures_tickers, futures_orderbook, \
                 futures_trade_history, futures_fee_schedules, futures_historical_funding_rates.\n\
                 Futures private tools require KRAKEN_FUTURES_KEY and KRAKEN_FUTURES_SECRET env vars.\n\
                 Spot WebSocket tools (ws_subscribe_*, ws_add_order, …): live subscriptions and \
                 low-latency trading. Private Spot WS tools require KRAKEN_API_KEY / KRAKEN_API_SECRET.\n\
                 Futures WebSocket tools (wf_subscribe_*, wf_send_order, …): live futures market data \
                 and account feeds. Private Futures WS tools require KRAKEN_FUTURES_KEY / KRAKEN_FUTURES_SECRET.\n\
                 ⚠️ Trading tools (place_order, ws_add_order, futures_send_order, wf_send_order, …) \
                 use REAL money — always confirm with user first.\n\
                 ⚠️ Funding tools (withdraw, futures_withdrawal, futures_transfer) move REAL funds."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

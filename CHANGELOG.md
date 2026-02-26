# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- 10 new tools (104 → 114 total):
  - Spot REST: `get_level3`, `get_grouped_book`, `get_credit_lines`, `get_order_amends`, `get_pre_trade`, `get_post_trade`
  - Futures WS: `wf_subscribe_ticker_lite`, `wf_subscribe_account_log`, `wf_subscribe_notifications`, `wf_subscribe_open_orders_verbose`
- Comprehensive error mapping with user-actionable messages for 20+ Kraken error types
- `docs/API_MAPPING.md` — full tool-to-API-endpoint reference with doc links
- `lib.rs` for direct test imports

### Fixed
- `scripts/kraken.sh` PATH fallback for `~/.cargo/bin` (common fresh install issue)
- Renamed all env file references from `.kraken-mcp.env` to `.tentactl.env`
- Updated README roadmap to reflect actual coverage

## [0.2.1] - 2025-02-25

### Added
- Spot WebSocket v2: all channels (ticker, book, OHLC, trades, executions, L3, balances, instrument) + order management (add, amend, edit, cancel, batch)
- Futures WebSocket v1: public feeds (ticker, book, trades) + private feeds (fills, open orders, open positions, balances) with challenge-response auth
- 30 new WebSocket tools (74 → 104 total)

### Fixed
- WebSocket parameter handling (`String` vs `Vec<String>` bug)

## [0.2.0] - 2025-02-25

### Added
- Full Spot REST coverage: market data, account, trading, funding, earn, export, subaccounts
- Full Futures REST coverage: accounts, orders, positions, fills, instruments, tickers, orderbook, funding rates, transfers
- 74 REST tools total
- HMAC-SHA512 signing for Spot private endpoints
- HMAC-SHA256/SHA512 signing for Futures endpoints
- CI/CD via GitHub Actions (build, test, clippy, publish to crates.io)
- OpenClaw skill (`npx clawhub install tentactl`)

## [0.1.0] - 2025-02-24

### Added
- Initial release
- Rust MCP server over stdio using `rmcp` crate
- Basic market data: `get_ticker`, `get_orderbook`, `get_ohlc`
- Account: `get_balance`, `get_trade_history`, `get_open_orders`
- Trading: `place_order`, `cancel_order` (with real-money warnings and validate-first flow)

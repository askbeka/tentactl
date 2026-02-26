# 🐙 tentactl

**Your Kraken exchange, as an AI tool.**

tentactl is an [MCP server](https://modelcontextprotocol.io) that gives any AI agent — Claude, Cursor, OpenClaw, or your own — the ability to interact with the Kraken cryptocurrency exchange through natural conversation.

```
You: "How's my portfolio doing?"
Agent: You're holding 0.5 BTC ($48,250), 3.2 ETH ($9,920), and 150 SOL ($22,500).
       Total: $80,670 — up 12.3% this month. SOL is your best performer at +28%.

You: "Set up a weekly DCA — $100 of ETH every Monday morning"
Agent: I'll validate the order first...
       ✅ Dry run passed: market buy 0.032 ETH (~$100) on ETHUSD.
       Want me to schedule this every Monday at 9:00 AM?
```

No dashboards. No config files. Just talk.

## Why MCP?

MCP (Model Context Protocol) is a standard that lets AI agents use external tools. tentactl exposes Kraken's API as MCP tools, which means:

- **Any MCP client works** — Claude Desktop, Cursor, OpenClaw, or anything that speaks MCP
- **Composable** — combine with other tools (news feeds, calendars, code) in the same agent
- **Conversational** — no API docs to read, just describe what you want
- **Auditable** — every action is visible in the agent's conversation log

## Quick Start

### Install

```bash
# From crates.io
cargo install tentactl

# Or download a binary from GitHub Releases
# https://github.com/askbeka/tentactl/releases
```

### Configure

Get API keys from [Kraken](https://www.kraken.com/u/security/api), then add tentactl to your MCP client:

**Claude Desktop / Cursor:**
```json
{
  "mcpServers": {
    "kraken": {
      "command": "tentactl",
      "env": {
        "KRAKEN_API_KEY": "your-key",
        "KRAKEN_API_SECRET": "your-secret"
      }
    }
  }
}
```

**OpenClaw:**
```bash
# Install the skill
npx clawhub install tentactl

# Set up API keys
~/.openclaw/workspace/skills/tentactl/scripts/setup-keys.sh
```

Market data works without API keys. Account and trading features require authentication.

### Use

That's it. Ask your agent about crypto — it'll use tentactl automatically.

## What You Can Do

### Market Data (no auth)

| Tool | What it does |
|------|-------------|
| `get_ticker` | Live price, volume, 24h high/low |
| `get_orderbook` | Order book depth — asks and bids |
| `get_ohlc` | Candlestick data for technical analysis |

> *"What's the current price of ETH?"*
> *"Show me the BTC order book — top 10 levels"*
> *"Pull hourly candles for SOL over the last week"*

### Portfolio (requires auth)

| Tool | What it does |
|------|-------------|
| `get_balance` | All holdings with current values |
| `get_trade_history` | Past trades with P&L |
| `get_open_orders` | Pending orders |

> *"What am I holding?"*
> *"Show my trades from last week"*
> *"Do I have any open limit orders?"*

### Trading (requires auth) ⚠️

| Tool | What it does |
|------|-------------|
| `place_order` | Buy/sell — market or limit |
| `cancel_order` | Cancel a pending order |

> *"Buy 0.01 BTC at market"*
> *"Place a limit buy for 1 ETH at $2,800"*
> *"Cancel that open order"*

**Safety:** All order placements go through a validate-first flow. The agent dry-runs the order, shows you exactly what will happen, and waits for your confirmation before executing.

## What Makes This Interesting

### Conversational Trading

Traditional flow: open app → find pair → choose order type → enter amount → review → confirm.

tentactl flow: *"Buy $100 of ETH"* → done.

The agent handles pair format, order sizing, validation, and confirmation. You think in intent, it handles execution.

### Agent-Native Automation

Because tentactl runs inside an AI agent, you get automation that understands context:

**DCA with conditions:**
> *"Buy $50 of BTC every Monday, but skip if RSI is above 70"*

The agent pulls OHLC data, calculates RSI, and conditionally executes. No scripting required.

**Smart alerts:**
> *"Tell me if BTC drops 5% in an hour, or if my SOL position is up 20%"*

Combine price monitoring with portfolio tracking — delivered to your preferred chat (Telegram, Signal, Discord).

**Portfolio intelligence:**
> *"How's my portfolio doing this month? What's my best and worst performer?"*

The agent combines balances, trade history, and live prices to give you an actual analysis — not just numbers.

### Composable with Other Tools

MCP tools compose naturally. An agent with tentactl plus other tools can:

- Read crypto news → assess impact → suggest trades
- Monitor your calendar → generate portfolio summaries before meetings
- Run backtests in code → execute the winning strategy
- Track on-chain events → hedge positions automatically

This is the power of MCP: tentactl doesn't need to build all of this. It just needs to be a good trading tool, and agents handle the orchestration.

## Roadmap

### Coming Soon
- **WebSocket streaming** — real-time price feeds, live order status updates, instant fill notifications
- **Staking** — stake/unstake, view rewards, auto-compound
- **Funding** — deposits, withdrawals, address generation
- **Full ledger** — complete transaction history, P&L calculations

### Planned
- **Advanced orders** — stop-loss, take-profit, OCO (one-cancels-other), trailing stops
- **Earn strategies** — yield optimization across Kraken's earn products
- **Multi-account** — subaccount management for portfolio isolation
- **Risk tools** — exposure analysis, concentration warnings, correlation tracking

## Getting Kraken API Keys

1. Log in to [Kraken](https://www.kraken.com)
2. Go to **Settings → API** (or visit `https://www.kraken.com/u/security/api`)
3. Click **Generate New Key**
4. For read-only: enable **Query Funds** and **Query Open Orders & Trades**
5. For trading: also enable **Create & Modify Orders**
6. Copy the API Key and Private Key

**Tip:** Create separate keys with minimal permissions. Read-only for monitoring, trade-enabled only when you need it.

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `KRAKEN_API_KEY` | For spot account/trading | Spot API key ([create here](https://www.kraken.com/u/security/api)) |
| `KRAKEN_API_SECRET` | For spot account/trading | Spot API secret |
| `KRAKEN_FUTURES_KEY` | For futures account/trading | Futures API key ([create here](https://futures.kraken.com/trade/settings/api)) |
| `KRAKEN_FUTURES_SECRET` | For futures account/trading | Futures API secret |
| `RUST_LOG` | No | Log level (`info`, `debug`) — logs to stderr |

## Development

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## License

MIT

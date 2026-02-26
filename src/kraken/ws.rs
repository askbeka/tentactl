//! Spot WebSocket v2 client.
//!
//! Manages persistent connections to:
//! - `wss://ws.kraken.com/v2`      — public channels
//! - `wss://ws-auth.kraken.com/v2` — authenticated channels
//!
//! Connections are established lazily on first use and run as background
//! tokio tasks.  All clones of [`WsClient`] share the same live connections
//! and buffered state via interior `Arc`s.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, warn};

use crate::kraken::client::KrakenClient;
use crate::kraken::error::KrakenError;

const PUBLIC_WS_URL: &str = "wss://ws.kraken.com/v2";
const PRIVATE_WS_URL: &str = "wss://ws-auth.kraken.com/v2";
/// How long (ms) to wait after a subscribe for the snapshot to arrive.
const SNAPSHOT_WAIT_MS: u64 = 1500;
/// Timeout (ms) for trading request responses.
const RESPONSE_TIMEOUT_MS: u64 = 8_000;

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct WsMessage<P> {
    method: String,
    params: P,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
struct WsChannelParams {
    channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    depth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snap_orders: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snap_trades: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WsMessageHeader {
    req_id: Option<u64>,
    method: Option<String>,
    success: Option<bool>,
    channel: Option<String>,
    #[serde(rename = "type")]
    msg_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WsDataMessage<T> {
    #[serde(default)]
    data: Vec<T>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WsBookLevel {
    price: Option<f64>,
    qty: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct WsBookData {
    symbol: Option<String>,
    checksum: Option<u32>,
    timestamp: Option<String>,
    #[serde(default)]
    bids: Vec<WsBookLevel>,
    #[serde(default)]
    asks: Vec<WsBookLevel>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
struct WsSymbolData {
    symbol: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
struct WsOhlcData {
    symbol: Option<String>,
    interval: Option<u64>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
struct WsBalanceData {
    asset: Option<String>,
    balance: Option<f64>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

// ── Book state ────────────────────────────────────────────────────────

#[derive(Default)]
struct BookState {
    /// price (string key, high-precision) → quantity
    bids: HashMap<String, f64>,
    asks: HashMap<String, f64>,
    checksum: u32,
    timestamp: String,
}

impl BookState {
    fn apply(
        &mut self,
        bids: &[WsBookLevel],
        asks: &[WsBookLevel],
        checksum: u32,
        timestamp: &str,
        is_snapshot: bool,
    ) {
        if is_snapshot {
            self.bids.clear();
            self.asks.clear();
        }
        for level in bids {
            let price = level.price.unwrap_or(0.0);
            let qty = level.qty.unwrap_or(0.0);
            let key = format!("{price:.10}");
            if qty == 0.0 {
                self.bids.remove(&key);
            } else {
                self.bids.insert(key, qty);
            }
        }
        for level in asks {
            let price = level.price.unwrap_or(0.0);
            let qty = level.qty.unwrap_or(0.0);
            let key = format!("{price:.10}");
            if qty == 0.0 {
                self.asks.remove(&key);
            } else {
                self.asks.insert(key, qty);
            }
        }
        self.checksum = checksum;
        self.timestamp = timestamp.to_string();
    }

    /// Serialize to JSON: bids descending by price, asks ascending.
    fn to_value(&self, symbol: &str) -> Value {
        let mut bids: Vec<(f64, f64)> = self
            .bids
            .iter()
            .filter_map(|(k, &v)| k.parse::<f64>().ok().map(|p| (p, v)))
            .collect();
        bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut asks: Vec<(f64, f64)> = self
            .asks
            .iter()
            .filter_map(|(k, &v)| k.parse::<f64>().ok().map(|p| (p, v)))
            .collect();
        asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        json!({
            "symbol": symbol,
            "bids": bids.iter().map(|(p, q)| json!({"price": p, "qty": q})).collect::<Vec<_>>(),
            "asks": asks.iter().map(|(p, q)| json!({"price": p, "qty": q})).collect::<Vec<_>>(),
            "checksum": self.checksum,
            "timestamp": self.timestamp,
        })
    }
}

// ── Shared WS state ───────────────────────────────────────────────────

#[derive(Default)]
struct WsState {
    tickers: HashMap<String, Value>,     // symbol → latest ticker
    books: HashMap<String, BookState>,   // symbol → book state
    trades: HashMap<String, Vec<Value>>, // symbol → last 50 trades
    ohlc: HashMap<String, Value>,        // "{symbol}:{interval}" → latest candle
    instrument: Option<Value>,           // latest instrument snapshot
    executions: Vec<Value>,              // last 50 execution reports
    balances: HashMap<String, Value>,    // asset → balance info
    level3: HashMap<String, Vec<Value>>, // symbol → last 50 L3 events
    subscriptions: HashSet<String>,      // active subscription keys
}

/// A snapshot of the current WS state, returned by MCP tools.
#[derive(serde::Serialize, Clone)]
pub struct WsStateSnapshot {
    pub subscriptions: Vec<String>,
    pub tickers: HashMap<String, Value>,
    pub books: HashMap<String, Value>,
    pub trades: HashMap<String, Vec<Value>>,
    pub ohlc: HashMap<String, Value>,
    pub instrument: Option<Value>,
    pub executions: Vec<Value>,
    pub balances: HashMap<String, Value>,
    pub public_connected: bool,
    pub private_connected: bool,
}

// ── Inner shared state ────────────────────────────────────────────────

struct WsInner {
    state: Arc<RwLock<WsState>>,
    public_tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    private_tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    /// Pending trading responses keyed by req_id.
    pending: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
    req_id: AtomicU64,
}

// ── Public client ─────────────────────────────────────────────────────

/// Spot WebSocket v2 client.
///
/// Cheap to clone — all clones share the same connections and state.
#[derive(Clone)]
pub struct WsClient {
    inner: Arc<WsInner>,
    rest: KrakenClient,
}

impl WsClient {
    pub fn new(rest: KrakenClient) -> Self {
        Self {
            inner: Arc::new(WsInner {
                state: Arc::new(RwLock::new(WsState::default())),
                public_tx: Mutex::new(None),
                private_tx: Mutex::new(None),
                pending: Mutex::new(HashMap::new()),
                req_id: AtomicU64::new(1),
            }),
            rest,
        }
    }

    // ── Connection management ─────────────────────────────────────────

    /// Ensure the public connection is live, connecting lazily if needed.
    pub async fn ensure_public(&self) -> Result<(), KrakenError> {
        let mut guard = self.inner.public_tx.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        *guard = Some(tx);
        drop(guard);
        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            run_connection(PUBLIC_WS_URL, true, rx, inner).await;
        });
        // Allow the TCP+TLS handshake to complete before the first send.
        tokio::time::sleep(Duration::from_millis(400)).await;
        Ok(())
    }

    /// Ensure the private (authenticated) connection is live.
    pub async fn ensure_private(&self) -> Result<(), KrakenError> {
        let mut guard = self.inner.private_tx.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        *guard = Some(tx);
        drop(guard);
        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            run_connection(PRIVATE_WS_URL, false, rx, inner).await;
        });
        tokio::time::sleep(Duration::from_millis(400)).await;
        Ok(())
    }

    pub async fn get_token(&self) -> Result<String, KrakenError> {
        self.rest.get_websockets_token().await
    }

    // ── Low-level send helpers ────────────────────────────────────────

    async fn send_public(&self, msg: String) -> Result<(), KrakenError> {
        let guard = self.inner.public_tx.lock().await;
        guard
            .as_ref()
            .ok_or_else(|| KrakenError::Api("Public WS not connected".into()))?
            .send(msg)
            .map_err(|e| KrakenError::Api(format!("WS send: {e}")))
    }

    async fn send_private_raw(&self, msg: String) -> Result<(), KrakenError> {
        let guard = self.inner.private_tx.lock().await;
        guard
            .as_ref()
            .ok_or_else(|| KrakenError::Api("Private WS not connected".into()))?
            .send(msg)
            .map_err(|e| KrakenError::Api(format!("WS send: {e}")))
    }

    fn next_req_id(&self) -> u64 {
        self.inner.req_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Send an authenticated trading request and await its response.
    pub async fn trading_request(
        &self,
        method: &str,
        mut params: Value,
    ) -> Result<Value, KrakenError> {
        let token = self.get_token().await?;
        params["token"] = json!(token);
        let req_id = self.next_req_id();
        let msg = json!({
            "method": method,
            "params": params,
            "req_id": req_id,
        });
        let (tx, rx) = oneshot::channel::<Value>();
        self.inner.pending.lock().await.insert(req_id, tx);
        self.ensure_private().await?;
        self.send_private_raw(msg.to_string()).await?;
        match tokio::time::timeout(Duration::from_millis(RESPONSE_TIMEOUT_MS), rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_)) => Err(KrakenError::Api("Response channel closed".into())),
            Err(_) => {
                self.inner.pending.lock().await.remove(&req_id);
                Err(KrakenError::Api(format!(
                    "WS '{method}' timed out after {RESPONSE_TIMEOUT_MS}ms"
                )))
            }
        }
    }

    // ── Public subscriptions ──────────────────────────────────────────

    pub async fn subscribe_ticker(&self, symbols: Vec<String>) -> Result<(), KrakenError> {
        self.ensure_public().await?;
        self.send_public(
            json!({"method":"subscribe","params":{"channel":"ticker","symbol":symbols}})
                .to_string(),
        )
        .await?;
        let mut s = self.inner.state.write().await;
        for sym in &symbols {
            s.subscriptions.insert(format!("ticker:{sym}"));
        }
        Ok(())
    }

    pub async fn subscribe_book(
        &self,
        symbols: Vec<String>,
        depth: u32,
    ) -> Result<(), KrakenError> {
        self.ensure_public().await?;
        self.send_public(
            json!({"method":"subscribe","params":{"channel":"book","symbol":symbols,"depth":depth}})
                .to_string(),
        )
        .await?;
        let mut s = self.inner.state.write().await;
        for sym in &symbols {
            s.subscriptions.insert(format!("book:{sym}:{depth}"));
        }
        Ok(())
    }

    pub async fn subscribe_trades(
        &self,
        symbols: Vec<String>,
        snapshot: bool,
    ) -> Result<(), KrakenError> {
        self.ensure_public().await?;
        self.send_public(
            json!({"method":"subscribe","params":{"channel":"trade","symbol":symbols,"snapshot":snapshot}})
                .to_string(),
        )
        .await?;
        let mut s = self.inner.state.write().await;
        for sym in &symbols {
            s.subscriptions.insert(format!("trade:{sym}"));
        }
        Ok(())
    }

    pub async fn subscribe_ohlc(
        &self,
        symbols: Vec<String>,
        interval: u32,
    ) -> Result<(), KrakenError> {
        self.ensure_public().await?;
        self.send_public(
            json!({"method":"subscribe","params":{"channel":"ohlc","symbol":symbols,"interval":interval}})
                .to_string(),
        )
        .await?;
        let mut s = self.inner.state.write().await;
        for sym in &symbols {
            s.subscriptions.insert(format!("ohlc:{sym}:{interval}"));
        }
        Ok(())
    }

    pub async fn subscribe_instrument(&self) -> Result<(), KrakenError> {
        self.ensure_public().await?;
        self.send_public(
            json!({"method":"subscribe","params":{"channel":"instrument"}}).to_string(),
        )
        .await?;
        self.inner
            .state
            .write()
            .await
            .subscriptions
            .insert("instrument".into());
        Ok(())
    }

    // ── Private subscriptions ─────────────────────────────────────────

    pub async fn subscribe_level3(&self, symbols: Vec<String>) -> Result<(), KrakenError> {
        self.ensure_private().await?;
        let token = self.get_token().await?;
        self.send_private_raw(
            json!({"method":"subscribe","params":{"channel":"level3","symbol":symbols,"token":token}})
                .to_string(),
        )
        .await?;
        let mut s = self.inner.state.write().await;
        for sym in &symbols {
            s.subscriptions.insert(format!("level3:{sym}"));
        }
        Ok(())
    }

    pub async fn subscribe_executions(&self) -> Result<(), KrakenError> {
        self.ensure_private().await?;
        let token = self.get_token().await?;
        self.send_private_raw(
            json!({"method":"subscribe","params":{
                "channel":"executions",
                "token":token,
                "snap_orders":true,
                "snap_trades":true
            }})
            .to_string(),
        )
        .await?;
        self.inner
            .state
            .write()
            .await
            .subscriptions
            .insert("executions".into());
        Ok(())
    }

    pub async fn subscribe_balances(&self) -> Result<(), KrakenError> {
        self.ensure_private().await?;
        let token = self.get_token().await?;
        self.send_private_raw(
            json!({"method":"subscribe","params":{
                "channel":"balances",
                "token":token,
                "snapshot":true
            }})
            .to_string(),
        )
        .await?;
        self.inner
            .state
            .write()
            .await
            .subscriptions
            .insert("balances".into());
        Ok(())
    }

    // ── Unsubscribe ───────────────────────────────────────────────────

    pub async fn unsubscribe(
        &self,
        channel: &str,
        symbols: Option<Vec<String>>,
    ) -> Result<(), KrakenError> {
        let is_private = matches!(channel, "executions" | "balances" | "level3");
        let mut params = json!({"channel": channel});
        if let Some(ref syms) = symbols {
            params["symbol"] = json!(syms);
        }
        if is_private {
            let token = self.get_token().await?;
            params["token"] = json!(token);
            self.ensure_private().await?;
            self.send_private_raw(json!({"method":"unsubscribe","params":params}).to_string())
                .await?;
        } else {
            self.ensure_public().await?;
            self.send_public(json!({"method":"unsubscribe","params":params}).to_string())
                .await?;
        }
        let mut s = self.inner.state.write().await;
        match symbols {
            Some(syms) => {
                for sym in &syms {
                    s.subscriptions
                        .retain(|k| !k.starts_with(&format!("{channel}:{sym}")));
                }
            }
            None => {
                s.subscriptions
                    .retain(|k| !k.starts_with(&format!("{channel}:")));
                s.subscriptions.remove(channel);
            }
        }
        Ok(())
    }

    // ── State accessors ───────────────────────────────────────────────

    pub async fn get_snapshot(&self) -> WsStateSnapshot {
        let s = self.inner.state.read().await;
        // Try to check connection status without blocking.
        let public_connected = self
            .inner
            .public_tx
            .try_lock()
            .map(|g| g.is_some())
            .unwrap_or(true);
        let private_connected = self
            .inner
            .private_tx
            .try_lock()
            .map(|g| g.is_some())
            .unwrap_or(true);
        WsStateSnapshot {
            subscriptions: s.subscriptions.iter().cloned().collect(),
            tickers: s.tickers.clone(),
            books: s
                .books
                .iter()
                .map(|(k, v)| (k.clone(), v.to_value(k)))
                .collect(),
            trades: s.trades.clone(),
            ohlc: s.ohlc.clone(),
            instrument: s.instrument.clone(),
            executions: s.executions.clone(),
            balances: s.balances.clone(),
            public_connected,
            private_connected,
        }
    }

    pub async fn get_tickers(&self, symbols: &[String]) -> HashMap<String, Value> {
        let s = self.inner.state.read().await;
        symbols
            .iter()
            .filter_map(|sym| s.tickers.get(sym).map(|v| (sym.clone(), v.clone())))
            .collect()
    }

    pub async fn get_books(&self, symbols: &[String]) -> HashMap<String, Value> {
        let s = self.inner.state.read().await;
        symbols
            .iter()
            .filter_map(|sym| s.books.get(sym).map(|b| (sym.clone(), b.to_value(sym))))
            .collect()
    }

    pub async fn get_trades(&self, symbols: &[String]) -> HashMap<String, Vec<Value>> {
        let s = self.inner.state.read().await;
        symbols
            .iter()
            .filter_map(|sym| s.trades.get(sym).map(|v| (sym.clone(), v.clone())))
            .collect()
    }

    pub async fn get_ohlc(&self, symbol: &str, interval: u32) -> Option<Value> {
        let s = self.inner.state.read().await;
        s.ohlc
            .get(&format!("{symbol}:{interval}"))
            .or_else(|| s.ohlc.get(symbol))
            .cloned()
    }

    pub async fn get_executions(&self) -> Vec<Value> {
        self.inner.state.read().await.executions.clone()
    }

    pub async fn get_balances(&self) -> HashMap<String, Value> {
        self.inner.state.read().await.balances.clone()
    }

    /// How long the tool should wait after subscribing to receive a snapshot.
    pub fn snapshot_wait() -> Duration {
        Duration::from_millis(SNAPSHOT_WAIT_MS)
    }
}

// ── Connection task ───────────────────────────────────────────────────

async fn run_connection(
    url: &'static str,
    is_public: bool,
    mut cmd_rx: mpsc::UnboundedReceiver<String>,
    inner: Arc<WsInner>,
) {
    let ws_stream = match connect_async(url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            warn!("WS connect failed ({url}): {e}");
            clear_tx(&inner, is_public).await;
            return;
        }
    };
    debug!("WS connected: {url}");
    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(text) => {
                        debug!("WS → {text}");
                        if let Err(e) = write.send(Message::Text(text)).await {
                            warn!("WS write error: {e}");
                            break;
                        }
                    }
                    None => break,
                }
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let s = text.to_string();
                        debug!("WS ← {s}");
                        if let Ok(v) = serde_json::from_str::<Value>(&s) {
                            handle_message(v, &inner).await;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        warn!("WS read error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    warn!("WS disconnected: {url}");
    clear_tx(&inner, is_public).await;
}

async fn clear_tx(inner: &Arc<WsInner>, is_public: bool) {
    if is_public {
        *inner.public_tx.lock().await = None;
    } else {
        *inner.private_tx.lock().await = None;
    }
}

// ── Message dispatcher ────────────────────────────────────────────────

async fn handle_message(msg: Value, inner: &Arc<WsInner>) {
    // Responses to trading requests carry a req_id.
    if let Some(req_id) = msg.get("req_id").and_then(Value::as_u64) {
        if let Some(tx) = inner.pending.lock().await.remove(&req_id) {
            let _ = tx.send(msg);
        }
        return;
    }

    // Subscribe/unsubscribe acks have "method" but no streaming data.
    if msg.get("method").is_some() {
        if let Some(false) = msg.get("success").and_then(Value::as_bool) {
            warn!("WS error response: {msg}");
        }
        return;
    }

    let channel = match msg.get("channel").and_then(Value::as_str) {
        Some(c) => c,
        None => return,
    };

    let msg_type = msg.get("type").and_then(Value::as_str).unwrap_or("update");
    let empty = vec![];
    let data = msg.get("data").and_then(Value::as_array).unwrap_or(&empty);

    let mut state = inner.state.write().await;

    match channel {
        "ticker" => {
            for item in data {
                if let Some(sym) = item.get("symbol").and_then(Value::as_str) {
                    state.tickers.insert(sym.to_owned(), item.clone());
                }
            }
        }

        "book" => {
            let is_snapshot = msg_type == "snapshot";
            for item in data {
                let sym = match item.get("symbol").and_then(Value::as_str) {
                    Some(s) => s.to_owned(),
                    None => continue,
                };
                let book_data: WsBookData =
                    serde_json::from_value(item.clone()).unwrap_or_default();
                let checksum = book_data.checksum.unwrap_or(0);
                let ts = book_data.timestamp.as_deref().unwrap_or("");
                state.books.entry(sym).or_default().apply(
                    &book_data.bids,
                    &book_data.asks,
                    checksum,
                    ts,
                    is_snapshot,
                );
            }
        }

        "trade" => {
            for item in data {
                if let Some(sym) = item.get("symbol").and_then(Value::as_str) {
                    let trades = state.trades.entry(sym.to_owned()).or_default();
                    trades.push(item.clone());
                    if trades.len() > 50 {
                        let excess = trades.len() - 50;
                        trades.drain(0..excess);
                    }
                }
            }
        }

        "ohlc" => {
            for item in data {
                if let Some(sym) = item.get("symbol").and_then(Value::as_str) {
                    let interval = item.get("interval").and_then(Value::as_u64).unwrap_or(1);
                    // Store under both keyed and plain entries.
                    state.ohlc.insert(format!("{sym}:{interval}"), item.clone());
                    state.ohlc.insert(sym.to_owned(), item.clone());
                }
            }
        }

        "instrument" => {
            if msg_type == "snapshot" {
                state.instrument = Some(msg.clone());
            } else if let Some(existing) = state.instrument.as_mut().and_then(Value::as_object_mut)
            {
                let updates: &mut Value = existing.entry("updates").or_insert_with(|| json!([]));
                if let Some(arr) = updates.as_array_mut() {
                    arr.push(msg.clone());
                    if arr.len() > 100 {
                        arr.remove(0);
                    }
                }
            }
        }

        "executions" => {
            for item in data {
                state.executions.push(item.clone());
            }
            if state.executions.len() > 50 {
                let excess = state.executions.len() - 50;
                state.executions.drain(0..excess);
            }
        }

        "balances" => {
            if msg_type == "snapshot" {
                state.balances.clear();
                for item in data {
                    if let Some(asset) = item.get("asset").and_then(Value::as_str) {
                        state.balances.insert(asset.to_owned(), item.clone());
                    }
                }
            } else {
                for item in data {
                    if let Some(asset) = item.get("asset").and_then(Value::as_str) {
                        if let Some(balance) = item.get("balance").and_then(Value::as_f64) {
                            if let Some(entry) = state.balances.get_mut(asset) {
                                entry["balance"] = json!(balance);
                            } else {
                                state.balances.insert(
                                    asset.to_owned(),
                                    json!({"asset": asset, "balance": balance}),
                                );
                            }
                        }
                    }
                }
            }
        }

        "level3" => {
            for item in data {
                if let Some(sym) = item.get("symbol").and_then(Value::as_str) {
                    let events = state.level3.entry(sym.to_owned()).or_default();
                    events.push(item.clone());
                    if events.len() > 50 {
                        let excess = events.len() - 50;
                        events.drain(0..excess);
                    }
                }
            }
        }

        "heartbeat" | "status" => {} // intentionally ignored
        _ => debug!("WS unhandled channel: {channel}"),
    }
}

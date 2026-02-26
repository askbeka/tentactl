//! Kraken Futures WebSocket v1 client.
//!
//! Single connection to `wss://futures.kraken.com/ws/v1`.
//! Public feeds (ticker, book, trade) require no auth.
//! Private feeds (fills, open_orders, open_positions, balances) use a
//! challenge-response authentication flow.
//!
//! Connections are established lazily on first use.  All clones share the
//! same live connection and buffered state via interior `Arc`s.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::{Digest, Sha256, Sha512};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, warn};

use crate::kraken::error::KrakenError;

type HmacSha512 = Hmac<Sha512>;

const WS_URL: &str = "wss://futures.kraken.com/ws/v1";
/// How long (ms) to wait after a subscribe for a snapshot to arrive.
const SNAPSHOT_WAIT_MS: u64 = 1500;
/// Timeout (ms) waiting for the auth challenge from the server.
const AUTH_TIMEOUT_MS: u64 = 5_000;

// ── Book state ────────────────────────────────────────────────────────

#[derive(Default)]
struct FuturesBookState {
    bids: HashMap<String, f64>,
    asks: HashMap<String, f64>,
    seq: u64,
}

impl FuturesBookState {
    fn apply_snapshot(&mut self, bids: &[Value], asks: &[Value], seq: u64) {
        self.bids.clear();
        self.asks.clear();
        self.seq = seq;
        for level in bids {
            if let (Some(p), Some(q)) = (
                level.get("price").and_then(Value::as_f64),
                level.get("qty").and_then(Value::as_f64),
            ) {
                if q > 0.0 {
                    self.bids.insert(format!("{p:.10}"), q);
                }
            }
        }
        for level in asks {
            if let (Some(p), Some(q)) = (
                level.get("price").and_then(Value::as_f64),
                level.get("qty").and_then(Value::as_f64),
            ) {
                if q > 0.0 {
                    self.asks.insert(format!("{p:.10}"), q);
                }
            }
        }
    }

    fn apply_update(&mut self, side: &str, price: f64, qty: f64, seq: u64) {
        if seq > self.seq {
            self.seq = seq;
        }
        let key = format!("{price:.10}");
        let book = if side == "buy" {
            &mut self.bids
        } else {
            &mut self.asks
        };
        if qty == 0.0 {
            book.remove(&key);
        } else {
            book.insert(key, qty);
        }
    }

    fn to_value(&self, product_id: &str) -> Value {
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
            "product_id": product_id,
            "seq": self.seq,
            "bids": bids.iter().map(|(p, q)| json!({"price": p, "qty": q})).collect::<Vec<_>>(),
            "asks": asks.iter().map(|(p, q)| json!({"price": p, "qty": q})).collect::<Vec<_>>(),
        })
    }
}

// ── Shared WS state ───────────────────────────────────────────────────

#[derive(Default)]
struct FuturesWsState {
    tickers: HashMap<String, Value>,
    books: HashMap<String, FuturesBookState>,
    trades: HashMap<String, Vec<Value>>,
    fills: Vec<Value>,
    account_log: Vec<Value>,
    notifications: Vec<Value>,
    open_orders: HashMap<String, Value>,
    open_positions: Vec<Value>,
    balances: Option<Value>,
    subscriptions: HashSet<String>,
}

/// Snapshot of the current Futures WS state, returned by MCP tools.
#[derive(serde::Serialize, Clone)]
pub struct FuturesWsStateSnapshot {
    pub subscriptions: Vec<String>,
    pub tickers: HashMap<String, Value>,
    pub books: HashMap<String, Value>,
    pub trades: HashMap<String, Vec<Value>>,
    pub fills: Vec<Value>,
    pub account_log: Vec<Value>,
    pub notifications: Vec<Value>,
    pub open_orders: HashMap<String, Value>,
    pub open_positions: Vec<Value>,
    pub balances: Option<Value>,
    pub connected: bool,
}

// ── Auth state ────────────────────────────────────────────────────────

#[derive(Clone)]
struct FuturesAuthState {
    api_key: String,
    original_challenge: String,
    signed_challenge: String,
}

// ── Inner shared state ────────────────────────────────────────────────

struct FuturesWsInner {
    state: Arc<RwLock<FuturesWsState>>,
    conn_tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    auth: RwLock<Option<FuturesAuthState>>,
    /// Receives the raw challenge string from the server during auth setup.
    challenge_ready: Mutex<Option<oneshot::Sender<String>>>,
}

// ── Public client ─────────────────────────────────────────────────────

/// Kraken Futures WebSocket v1 client.
///
/// Cheap to clone — all clones share the same connection and state.
#[derive(Clone)]
pub struct FuturesWsClient {
    inner: Arc<FuturesWsInner>,
    api_key: String,
    api_secret: String,
}

/// Sign a Kraken Futures WS challenge.
///
/// Algorithm: `Base64(HMAC-SHA512(SHA256(challenge), Base64Decode(secret)))`
fn sign_challenge(challenge: &str, secret_b64: &str) -> Result<String, KrakenError> {
    let hash = Sha256::digest(challenge.as_bytes());
    let secret = BASE64
        .decode(secret_b64)
        .map_err(|e| KrakenError::Api(format!("Invalid futures secret base64: {e}")))?;
    let mut mac = HmacSha512::new_from_slice(&secret)
        .map_err(|e| KrakenError::Api(format!("HMAC key error: {e}")))?;
    mac.update(&hash);
    Ok(BASE64.encode(mac.finalize().into_bytes()))
}

impl FuturesWsClient {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            inner: Arc::new(FuturesWsInner {
                state: Arc::new(RwLock::new(FuturesWsState::default())),
                conn_tx: Mutex::new(None),
                auth: RwLock::new(None),
                challenge_ready: Mutex::new(None),
            }),
            api_key,
            api_secret,
        }
    }

    pub fn from_env() -> Self {
        let key = std::env::var("KRAKEN_FUTURES_KEY").unwrap_or_default();
        let secret = std::env::var("KRAKEN_FUTURES_SECRET").unwrap_or_default();
        Self::new(key, secret)
    }

    // ── Connection management ─────────────────────────────────────────

    async fn ensure_connected(&self) -> Result<(), KrakenError> {
        let mut guard = self.inner.conn_tx.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        *guard = Some(tx);
        drop(guard);
        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            run_connection(rx, inner).await;
        });
        // Allow the TCP+TLS handshake to complete.
        tokio::time::sleep(Duration::from_millis(400)).await;
        Ok(())
    }

    async fn send(&self, msg: String) -> Result<(), KrakenError> {
        let guard = self.inner.conn_tx.lock().await;
        guard
            .as_ref()
            .ok_or_else(|| KrakenError::Api("Futures WS not connected".into()))?
            .send(msg)
            .map_err(|e| KrakenError::Api(format!("Futures WS send: {e}")))
    }

    // ── Auth ──────────────────────────────────────────────────────────

    /// Obtain (and cache) challenge-based auth credentials.
    ///
    /// On first call this will request a challenge from the server,
    /// sign it, and store the result.  Subsequent calls return the
    /// cached state without hitting the server again.
    async fn ensure_auth(&self) -> Result<FuturesAuthState, KrakenError> {
        // Fast path — already have auth.
        {
            let auth = self.inner.auth.read().await;
            if let Some(a) = auth.as_ref() {
                return Ok(a.clone());
            }
        }

        if self.api_key.is_empty() || self.api_secret.is_empty() {
            return Err(KrakenError::FuturesAuthRequired);
        }

        // Set up a channel to receive the challenge from the message handler.
        let (challenge_tx, challenge_rx) = oneshot::channel::<String>();
        *self.inner.challenge_ready.lock().await = Some(challenge_tx);

        // Ensure the connection is up before sending.
        self.ensure_connected().await?;

        // Request the challenge.
        self.send(
            json!({
                "event": "subscribe",
                "feed": "challenge",
                "api_key": self.api_key,
            })
            .to_string(),
        )
        .await?;

        // Wait for the server to echo back the challenge string.
        let challenge = tokio::time::timeout(Duration::from_millis(AUTH_TIMEOUT_MS), challenge_rx)
            .await
            .map_err(|_| KrakenError::Api("Futures WS auth challenge timed out".into()))?
            .map_err(|_| KrakenError::Api("Futures WS auth channel closed".into()))?;

        let signed = sign_challenge(&challenge, &self.api_secret)?;

        let auth_state = FuturesAuthState {
            api_key: self.api_key.clone(),
            original_challenge: challenge,
            signed_challenge: signed,
        };
        *self.inner.auth.write().await = Some(auth_state.clone());
        Ok(auth_state)
    }

    // ── Public subscriptions ──────────────────────────────────────────

    pub async fn subscribe_ticker(&self, product_ids: Vec<String>) -> Result<(), KrakenError> {
        self.ensure_connected().await?;
        self.send(
            json!({
                "event": "subscribe",
                "feed": "ticker",
                "product_ids": product_ids,
            })
            .to_string(),
        )
        .await?;
        let mut s = self.inner.state.write().await;
        for id in &product_ids {
            s.subscriptions.insert(format!("ticker:{id}"));
        }
        Ok(())
    }

    pub async fn subscribe_ticker_lite(&self, product_ids: Vec<String>) -> Result<(), KrakenError> {
        self.ensure_connected().await?;
        self.send(
            json!({
                "event": "subscribe",
                "feed": "ticker_lite",
                "product_ids": product_ids,
            })
            .to_string(),
        )
        .await?;
        let mut s = self.inner.state.write().await;
        for id in &product_ids {
            s.subscriptions.insert(format!("ticker_lite:{id}"));
        }
        Ok(())
    }

    pub async fn subscribe_book(&self, product_ids: Vec<String>) -> Result<(), KrakenError> {
        self.ensure_connected().await?;
        self.send(
            json!({
                "event": "subscribe",
                "feed": "book",
                "product_ids": product_ids,
            })
            .to_string(),
        )
        .await?;
        let mut s = self.inner.state.write().await;
        for id in &product_ids {
            s.subscriptions.insert(format!("book:{id}"));
        }
        Ok(())
    }

    pub async fn subscribe_trades(&self, product_ids: Vec<String>) -> Result<(), KrakenError> {
        self.ensure_connected().await?;
        self.send(
            json!({
                "event": "subscribe",
                "feed": "trade",
                "product_ids": product_ids,
            })
            .to_string(),
        )
        .await?;
        let mut s = self.inner.state.write().await;
        for id in &product_ids {
            s.subscriptions.insert(format!("trade:{id}"));
        }
        Ok(())
    }

    // ── Private subscriptions ─────────────────────────────────────────

    async fn subscribe_private_feed(&self, feed: &str) -> Result<(), KrakenError> {
        let auth = self.ensure_auth().await?;
        self.send(
            json!({
                "event": "subscribe",
                "feed": feed,
                "api_key": auth.api_key,
                "original_challenge": auth.original_challenge,
                "signed_challenge": auth.signed_challenge,
            })
            .to_string(),
        )
        .await?;
        self.inner
            .state
            .write()
            .await
            .subscriptions
            .insert(feed.to_string());
        Ok(())
    }

    pub async fn subscribe_fills(&self) -> Result<(), KrakenError> {
        self.subscribe_private_feed("fills").await
    }

    pub async fn subscribe_account_log(&self) -> Result<(), KrakenError> {
        self.subscribe_private_feed("account_log").await
    }

    pub async fn subscribe_notifications(&self) -> Result<(), KrakenError> {
        self.subscribe_private_feed("notifications_auth").await
    }

    pub async fn subscribe_open_orders(&self) -> Result<(), KrakenError> {
        self.subscribe_private_feed("open_orders").await
    }

    pub async fn subscribe_open_orders_verbose(&self) -> Result<(), KrakenError> {
        self.subscribe_private_feed("open_orders_verbose").await
    }

    pub async fn subscribe_open_positions(&self) -> Result<(), KrakenError> {
        self.subscribe_private_feed("open_positions").await
    }

    pub async fn subscribe_balances(&self) -> Result<(), KrakenError> {
        self.subscribe_private_feed("balances").await
    }

    // ── Unsubscribe ───────────────────────────────────────────────────

    pub async fn unsubscribe(
        &self,
        feed: &str,
        product_ids: Option<Vec<String>>,
    ) -> Result<(), KrakenError> {
        let is_private = matches!(
            feed,
            "fills"
                | "account_log"
                | "notifications"
                | "notifications_auth"
                | "open_orders"
                | "open_orders_verbose"
                | "open_positions"
                | "balances"
        );

        let mut msg = json!({
            "event": "unsubscribe",
            "feed": feed,
        });
        if let Some(ref ids) = product_ids {
            msg["product_ids"] = json!(ids);
        }
        if is_private {
            if let Some(auth) = self.inner.auth.read().await.as_ref() {
                msg["api_key"] = json!(auth.api_key.clone());
                msg["original_challenge"] = json!(auth.original_challenge.clone());
                msg["signed_challenge"] = json!(auth.signed_challenge.clone());
            }
        }

        self.ensure_connected().await?;
        self.send(msg.to_string()).await?;

        let mut s = self.inner.state.write().await;
        match product_ids {
            Some(ids) => {
                for id in &ids {
                    s.subscriptions
                        .retain(|k| !k.starts_with(&format!("{feed}:{id}")));
                }
            }
            None => {
                s.subscriptions
                    .retain(|k| !k.starts_with(&format!("{feed}:")) && k != feed);
                s.subscriptions.remove(feed);
            }
        }
        Ok(())
    }

    // ── State accessors ───────────────────────────────────────────────

    pub async fn get_snapshot(&self) -> FuturesWsStateSnapshot {
        let s = self.inner.state.read().await;
        let connected = self
            .inner
            .conn_tx
            .try_lock()
            .map(|g| g.is_some())
            .unwrap_or(true);
        FuturesWsStateSnapshot {
            subscriptions: s.subscriptions.iter().cloned().collect(),
            tickers: s.tickers.clone(),
            books: s
                .books
                .iter()
                .map(|(k, v)| (k.clone(), v.to_value(k)))
                .collect(),
            trades: s.trades.clone(),
            fills: s.fills.clone(),
            account_log: s.account_log.clone(),
            notifications: s.notifications.clone(),
            open_orders: s.open_orders.clone(),
            open_positions: s.open_positions.clone(),
            balances: s.balances.clone(),
            connected,
        }
    }

    pub async fn get_tickers(&self, product_ids: &[String]) -> HashMap<String, Value> {
        let s = self.inner.state.read().await;
        product_ids
            .iter()
            .filter_map(|id| s.tickers.get(id).map(|v| (id.clone(), v.clone())))
            .collect()
    }

    pub async fn get_books(&self, product_ids: &[String]) -> HashMap<String, Value> {
        let s = self.inner.state.read().await;
        product_ids
            .iter()
            .filter_map(|id| s.books.get(id).map(|b| (id.clone(), b.to_value(id))))
            .collect()
    }

    pub async fn get_trades(&self, product_ids: &[String]) -> HashMap<String, Vec<Value>> {
        let s = self.inner.state.read().await;
        product_ids
            .iter()
            .filter_map(|id| s.trades.get(id).map(|v| (id.clone(), v.clone())))
            .collect()
    }

    pub fn snapshot_wait() -> Duration {
        Duration::from_millis(SNAPSHOT_WAIT_MS)
    }
}

// ── Connection task ───────────────────────────────────────────────────

async fn run_connection(mut cmd_rx: mpsc::UnboundedReceiver<String>, inner: Arc<FuturesWsInner>) {
    let ws_stream = match connect_async(WS_URL).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            warn!("Futures WS connect failed ({WS_URL}): {e}");
            *inner.conn_tx.lock().await = None;
            return;
        }
    };
    debug!("Futures WS connected: {WS_URL}");
    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(text) => {
                        debug!("Futures WS → {text}");
                        if let Err(e) = write.send(Message::Text(text)).await {
                            warn!("Futures WS write error: {e}");
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
                        debug!("Futures WS ← {s}");
                        if let Ok(v) = serde_json::from_str::<Value>(&s) {
                            handle_message(v, &inner).await;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        warn!("Futures WS read error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    warn!("Futures WS disconnected");
    *inner.conn_tx.lock().await = None;
}

// ── Message dispatcher ────────────────────────────────────────────────

async fn handle_message(msg: Value, inner: &Arc<FuturesWsInner>) {
    let event = msg.get("event").and_then(Value::as_str).unwrap_or("");

    // Challenge response — resolve the waiting auth handshake.
    if event == "challenge" {
        if let Some(challenge) = msg.get("message").and_then(Value::as_str) {
            let mut tx = inner.challenge_ready.lock().await;
            if let Some(sender) = tx.take() {
                let _ = sender.send(challenge.to_string());
            }
        }
        return;
    }

    if event == "subscribed" || event == "unsubscribed" || event == "info" {
        return;
    }

    if event == "error" {
        warn!("Futures WS error: {msg}");
        return;
    }

    let feed = match msg.get("feed").and_then(Value::as_str) {
        Some(f) => f,
        None => return,
    };

    let mut state = inner.state.write().await;

    match feed {
        "ticker" | "ticker_lite" => {
            if let Some(product_id) = msg.get("product_id").and_then(Value::as_str) {
                state.tickers.insert(product_id.to_owned(), msg.clone());
            }
        }

        "book_snapshot" => {
            if let Some(product_id) = msg.get("product_id").and_then(Value::as_str) {
                let seq = msg.get("seq").and_then(Value::as_u64).unwrap_or(0);
                let empty = vec![];
                let bids = msg.get("bids").and_then(Value::as_array).unwrap_or(&empty);
                let asks = msg.get("asks").and_then(Value::as_array).unwrap_or(&empty);
                state
                    .books
                    .entry(product_id.to_owned())
                    .or_default()
                    .apply_snapshot(bids, asks, seq);
            }
        }

        "book" => {
            if let Some(product_id) = msg.get("product_id").and_then(Value::as_str) {
                let seq = msg.get("seq").and_then(Value::as_u64).unwrap_or(0);
                let side = msg.get("side").and_then(Value::as_str).unwrap_or("buy");
                let price = msg.get("price").and_then(Value::as_f64).unwrap_or(0.0);
                let qty = msg.get("qty").and_then(Value::as_f64).unwrap_or(0.0);
                state
                    .books
                    .entry(product_id.to_owned())
                    .or_default()
                    .apply_update(side, price, qty, seq);
            }
        }

        "trade" | "trade_snapshot" => {
            if let Some(product_id) = msg.get("product_id").and_then(Value::as_str) {
                let trades = state.trades.entry(product_id.to_owned()).or_default();
                trades.push(msg.clone());
                if trades.len() > 50 {
                    let excess = trades.len() - 50;
                    trades.drain(0..excess);
                }
            }
        }

        "fills" | "fills_snapshot" => {
            if let Some(fills) = msg.get("fills").and_then(Value::as_array) {
                for fill in fills {
                    state.fills.push(fill.clone());
                }
            } else {
                state.fills.push(msg.clone());
            }
            if state.fills.len() > 50 {
                let excess = state.fills.len() - 50;
                state.fills.drain(0..excess);
            }
        }

        "account_log" | "account_log_snapshot" => {
            if let Some(logs) = msg.get("logs").and_then(Value::as_array) {
                state.account_log = logs.to_vec();
            } else {
                state.account_log.push(msg.clone());
            }
            if state.account_log.len() > 200 {
                let excess = state.account_log.len() - 200;
                state.account_log.drain(0..excess);
            }
        }

        "notifications"
        | "notifications_snapshot"
        | "notifications_auth"
        | "notifications_auth_snapshot" => {
            if let Some(notifs) = msg.get("notifications").and_then(Value::as_array) {
                state.notifications = notifs.to_vec();
            } else {
                state.notifications.push(msg.clone());
            }
            if state.notifications.len() > 100 {
                let excess = state.notifications.len() - 100;
                state.notifications.drain(0..excess);
            }
        }

        "open_orders"
        | "open_orders_verbose"
        | "open_orders_snapshot"
        | "open_orders_verbose_snapshot" => {
            if let Some(orders) = msg.get("orders").and_then(Value::as_array) {
                // Snapshot: replace all open orders.
                state.open_orders.clear();
                for order in orders {
                    if let Some(oid) = order.get("order_id").and_then(Value::as_str) {
                        state.open_orders.insert(oid.to_owned(), order.clone());
                    }
                }
            } else if let Some(order_id) = msg.get("order_id").and_then(Value::as_str) {
                // Incremental update.
                let is_cancel = msg
                    .get("is_cancel")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if is_cancel {
                    state.open_orders.remove(order_id);
                } else {
                    state.open_orders.insert(order_id.to_owned(), msg.clone());
                }
            }
        }

        "open_positions" | "open_positions_snapshot" => {
            if let Some(positions) = msg.get("positions").and_then(Value::as_array) {
                state.open_positions = positions.to_vec();
            }
        }

        "balances" => {
            state.balances = Some(msg.clone());
        }

        "heartbeat" => {} // intentionally ignored
        _ => debug!("Futures WS unhandled feed: {feed}"),
    }
}

//! MCP server integration tests.
//! Spawns the actual binary, sends JSON-RPC over stdin, validates responses.

use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_tentactl");

/// Helper: spawn the server, send messages, collect responses.
/// `expect_responses` is how many JSON-RPC responses we expect (including init).
async fn mcp_session(messages: Vec<Value>) -> Vec<Value> {
    // Count expected responses: 1 for init + 1 for each message that has an "id"
    let expected = 1 + messages.iter().filter(|m| m.get("id").is_some()).count();

    let mut child = Command::new(BINARY)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn server");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();

    // Send initialize
    let init = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "0.1"}
        }
    });
    stdin
        .write_all(format!("{}\n", init).as_bytes())
        .await
        .unwrap();

    // Wait for init response before continuing
    let mut responses = Vec::new();
    if let Ok(Some(line)) = reader.next_line().await {
        if let Ok(val) = serde_json::from_str::<Value>(&line) {
            responses.push(val);
        }
    }

    // Send initialized notification
    let initialized_notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    stdin
        .write_all(format!("{}\n", initialized_notif).as_bytes())
        .await
        .unwrap();

    // Send user messages
    for msg in &messages {
        stdin
            .write_all(format!("{}\n", msg).as_bytes())
            .await
            .unwrap();
        stdin.flush().await.unwrap();
    }

    // Read remaining expected responses with a timeout
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);
    while responses.len() < expected {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, reader.next_line()).await {
            Ok(Ok(Some(line))) => {
                if let Ok(val) = serde_json::from_str::<Value>(&line) {
                    responses.push(val);
                }
            }
            _ => break,
        }
    }

    // Clean up
    drop(stdin);
    let _ = child.kill().await;
    responses
}

/// Helper: extract the init response
fn find_response(responses: &[Value], id: i64) -> Option<&Value> {
    responses.iter().find(|r| r["id"] == id)
}

// === Tests ===

#[tokio::test]
async fn server_initializes() {
    let responses = mcp_session(vec![]).await;

    let init_resp = find_response(&responses, 0).expect("should have init response");
    assert_eq!(init_resp["result"]["protocolVersion"], "2024-11-05");
    assert!(init_resp["result"]["capabilities"]["tools"].is_object());
    assert!(init_resp["result"]["serverInfo"].is_object());

    let instructions = init_resp["result"]["instructions"].as_str().unwrap_or("");
    assert!(
        instructions.contains("Kraken"),
        "instructions should mention Kraken"
    );
}

#[tokio::test]
async fn tools_list_returns_all_tools() {
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let responses = mcp_session(vec![list_req]).await;
    let resp = find_response(&responses, 1).expect("should have tools/list response");

    let tools = resp["result"]["tools"].as_array().expect("tools array");

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // Core tools that must always be present
    let expected = [
        "get_ticker",
        "get_orderbook",
        "get_ohlc",
        "get_balance",
        "get_trade_history",
        "get_open_orders",
        "place_order",
        "cancel_order",
        "get_server_time",
        "get_system_status",
        "get_assets",
        "get_asset_pairs",
        "get_grouped_book",
        "get_level3",
        "get_recent_trades",
        "get_spread",
        "get_pre_trade",
        "get_post_trade",
        "get_extended_balance",
        "get_credit_lines",
        "get_trade_balance",
        "get_closed_orders",
        "query_orders",
        "get_order_amends",
        "get_trades_info",
        "get_open_positions",
        "get_ledger",
        "query_ledger",
        "get_trade_volume",
        "add_export",
        "export_status",
        "retrieve_export",
        "remove_export",
        "add_order_batch",
        "amend_order",
        "edit_order",
        "cancel_all_orders",
        "cancel_all_after",
        "cancel_order_batch",
        "get_deposit_methods",
        "get_deposit_addresses",
        "get_deposit_status",
        "get_withdraw_methods",
        "get_withdraw_addresses",
        "get_withdraw_info",
        "withdraw",
        "get_withdraw_status",
        "cancel_withdraw",
        "wallet_transfer",
        "earn_strategies",
        "earn_allocations",
        "earn_allocate",
        "earn_deallocate",
        "earn_allocate_status",
        "earn_deallocate_status",
        "create_subaccount",
        "account_transfer",
        "futures_instruments",
        "futures_instrument_status",
        "futures_tickers",
        "futures_ticker",
        "futures_orderbook",
        "futures_trade_history",
        "futures_fee_schedules",
        "futures_historical_funding_rates",
        "futures_accounts",
        "futures_open_orders",
        "futures_open_positions",
        "futures_fills",
        "futures_transfers",
        "futures_order_status",
        "futures_leverage_setting",
        "futures_pnl_preference",
        "futures_send_order",
        "futures_edit_order",
        "futures_cancel_order",
        "futures_cancel_all",
        "futures_batch_order",
        "futures_transfer",
        "futures_withdrawal",
        "ws_subscribe_ticker",
        "ws_subscribe_book",
        "ws_subscribe_trades",
        "ws_subscribe_ohlc",
        "ws_subscribe_instrument",
        "ws_subscribe_level3",
        "ws_subscribe_executions",
        "ws_subscribe_balances",
        "ws_add_order",
        "ws_amend_order",
        "ws_edit_order",
        "ws_cancel_order",
        "ws_cancel_all",
        "ws_cancel_after",
        "ws_batch_add",
        "ws_batch_cancel",
        "ws_unsubscribe",
        "ws_status",
        "wf_subscribe_ticker",
        "wf_subscribe_ticker_lite",
        "wf_subscribe_book",
        "wf_subscribe_trades",
        "wf_subscribe_fills",
        "wf_subscribe_account_log",
        "wf_subscribe_notifications",
        "wf_subscribe_open_orders",
        "wf_subscribe_open_orders_verbose",
        "wf_subscribe_open_positions",
        "wf_subscribe_balances",
        "wf_send_order",
        "wf_cancel_order",
        "wf_batch_order",
        "wf_unsubscribe",
        "wf_status",
    ];
    for name in &expected {
        assert!(
            tool_names.contains(name),
            "missing tool: {name}. Got: {tool_names:?}"
        );
    }
    assert_eq!(tools.len(), 114, "should have exactly 114 tools");
}

#[tokio::test]
async fn tools_have_descriptions() {
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let responses = mcp_session(vec![list_req]).await;
    let resp = find_response(&responses, 1).unwrap();
    let tools = resp["result"]["tools"].as_array().unwrap();

    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let desc = tool["description"].as_str().unwrap_or("");
        assert!(!desc.is_empty(), "tool {name} should have a description");
    }
}

#[tokio::test]
async fn place_order_has_warning_in_description() {
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let responses = mcp_session(vec![list_req]).await;
    let resp = find_response(&responses, 1).unwrap();
    let tools = resp["result"]["tools"].as_array().unwrap();

    let place_order = tools
        .iter()
        .find(|t| t["name"] == "place_order")
        .expect("place_order tool");

    let desc = place_order["description"].as_str().unwrap();
    assert!(
        desc.contains("REAL MONEY"),
        "place_order should warn about real money. Got: {desc}"
    );
}

#[tokio::test]
async fn tools_have_input_schemas() {
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let responses = mcp_session(vec![list_req]).await;
    let resp = find_response(&responses, 1).unwrap();
    let tools = resp["result"]["tools"].as_array().unwrap();

    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        let schema = &tool["inputSchema"];
        assert!(schema.is_object(), "tool {name} should have an inputSchema");
        assert_eq!(
            schema["type"], "object",
            "tool {name} inputSchema should be type object"
        );
    }
}

#[tokio::test]
async fn get_ticker_schema_requires_pair() {
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let responses = mcp_session(vec![list_req]).await;
    let resp = find_response(&responses, 1).unwrap();
    let tools = resp["result"]["tools"].as_array().unwrap();

    let ticker = tools.iter().find(|t| t["name"] == "get_ticker").unwrap();

    let required = ticker["inputSchema"]["required"]
        .as_array()
        .expect("required array");
    let required_names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        required_names.contains(&"pair"),
        "get_ticker should require 'pair'"
    );
}

#[tokio::test]
async fn get_level3_schema_requires_pair() {
    let tool = get_tool_schema("get_level3").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        names.contains(&"pair"),
        "get_level3 should require 'pair'. Got: {names:?}"
    );
}

#[tokio::test]
async fn get_grouped_book_schema_requires_pair() {
    let tool = get_tool_schema("get_grouped_book").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        names.contains(&"pair"),
        "get_grouped_book should require 'pair'. Got: {names:?}"
    );
}

#[tokio::test]
async fn get_pre_trade_schema_requires_symbol() {
    let tool = get_tool_schema("get_pre_trade").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        names.contains(&"symbol"),
        "get_pre_trade should require 'symbol'. Got: {names:?}"
    );
}

#[tokio::test]
async fn place_order_schema_requires_fields() {
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let responses = mcp_session(vec![list_req]).await;
    let resp = find_response(&responses, 1).unwrap();
    let tools = resp["result"]["tools"].as_array().unwrap();

    let place_order = tools.iter().find(|t| t["name"] == "place_order").unwrap();

    let required = place_order["inputSchema"]["required"]
        .as_array()
        .expect("required array");
    let required_names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();

    for field in &["pair", "direction", "order_type", "volume"] {
        assert!(
            required_names.contains(field),
            "place_order should require '{field}'. Got: {required_names:?}"
        );
    }
}

#[tokio::test]
async fn get_balance_without_keys_returns_auth_error() {
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "get_balance",
            "arguments": {}
        }
    });

    let responses = mcp_session(vec![call]).await;
    let resp = find_response(&responses, 1).expect("should have response");

    // Should return an error content (not a JSON-RPC error, but isError in result)
    let content = &resp["result"]["content"][0]["text"];
    let text = content.as_str().unwrap_or("");
    assert!(
        text.contains("Authentication required"),
        "should mention auth. Got: {text}"
    );
    assert_eq!(resp["result"]["isError"], true);
}

#[tokio::test]
async fn call_unknown_tool_returns_error() {
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "nonexistent_tool",
            "arguments": {}
        }
    });

    let responses = mcp_session(vec![call]).await;
    let resp = find_response(&responses, 1).expect("should have response");

    // Should be a JSON-RPC error (method not found or similar)
    assert!(
        resp.get("error").is_some(),
        "unknown tool should return error. Got: {resp}"
    );
}

#[tokio::test]
async fn multiple_tool_calls_in_sequence() {
    let call1 = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let call2 = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "get_balance",
            "arguments": {}
        }
    });

    let responses = mcp_session(vec![call1, call2]).await;

    let resp1 = find_response(&responses, 1).expect("should have tools/list response");
    assert!(resp1["result"]["tools"].is_array());

    let resp2 = find_response(&responses, 2).expect("should have get_balance response");
    assert!(resp2["result"]["content"].is_array());
}

#[tokio::test]
async fn get_ticker_missing_pair_returns_error() {
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "get_ticker",
            "arguments": {}
        }
    });

    let responses = mcp_session(vec![call]).await;
    let resp = find_response(&responses, 1).expect("should have response");

    // Should get a parameter validation error
    assert!(
        resp.get("error").is_some() || resp["result"]["isError"] == true,
        "missing required param should error. Got: {resp}"
    );
}

// ── Phase 3: Spot WebSocket behavioral tests ──────────────────────────

/// Helper: fetch tools/list and find a named tool's schema.
async fn get_tool_schema(name: &str) -> serde_json::Value {
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let responses = mcp_session(vec![list_req]).await;
    let resp = find_response(&responses, 1).unwrap();
    let tools = resp["result"]["tools"].as_array().unwrap();
    tools
        .iter()
        .find(|t| t["name"] == name)
        .cloned()
        .unwrap_or_else(|| panic!("tool '{name}' not found"))
}

#[tokio::test]
async fn ws_subscribe_ticker_schema_requires_symbols() {
    let tool = get_tool_schema("ws_subscribe_ticker").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("ws_subscribe_ticker should have required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        names.contains(&"symbols"),
        "ws_subscribe_ticker should require 'symbols'. Got: {names:?}"
    );
}

#[tokio::test]
async fn ws_add_order_schema_requires_order_fields() {
    let tool = get_tool_schema("ws_add_order").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("ws_add_order should have required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    for field in &["order_type", "side", "order_qty", "symbol"] {
        assert!(
            names.contains(field),
            "ws_add_order should require '{field}'. Got: {names:?}"
        );
    }
}

#[tokio::test]
async fn ws_unsubscribe_schema_requires_channel() {
    let tool = get_tool_schema("ws_unsubscribe").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("ws_unsubscribe should have required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        names.contains(&"channel"),
        "ws_unsubscribe should require 'channel'. Got: {names:?}"
    );
    // symbols should be optional
    assert!(
        !names.contains(&"symbols"),
        "ws_unsubscribe 'symbols' should be optional, not required"
    );
}

#[tokio::test]
async fn ws_cancel_after_schema_requires_timeout() {
    let tool = get_tool_schema("ws_cancel_after").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("ws_cancel_after should have required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        names.contains(&"timeout"),
        "ws_cancel_after should require 'timeout'. Got: {names:?}"
    );
}

#[tokio::test]
async fn ws_add_order_warns_real_money() {
    let tool = get_tool_schema("ws_add_order").await;
    let desc = tool["description"].as_str().unwrap_or("");
    assert!(
        desc.contains("REAL MONEY"),
        "ws_add_order should warn about REAL MONEY in description. Got: {desc}"
    );
}

#[tokio::test]
async fn ws_subscribe_book_depth_is_optional() {
    let tool = get_tool_schema("ws_subscribe_book").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    assert!(
        !required.contains(&"depth"),
        "ws_subscribe_book 'depth' should be optional. Got required: {required:?}"
    );
    assert!(
        required.contains(&"symbols"),
        "ws_subscribe_book should require 'symbols'"
    );
}

#[tokio::test]
async fn ws_batch_add_schema_requires_orders_and_symbol() {
    let tool = get_tool_schema("ws_batch_add").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("ws_batch_add should have required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    for field in &["orders", "symbol"] {
        assert!(
            names.contains(field),
            "ws_batch_add should require '{field}'. Got: {names:?}"
        );
    }
}

#[tokio::test]
async fn ws_status_has_no_required_params() {
    let tool = get_tool_schema("ws_status").await;
    let props = &tool["inputSchema"]["properties"];
    // ws_status takes no parameters — properties should be empty or absent
    let prop_count = props.as_object().map(|m| m.len()).unwrap_or(0);
    assert_eq!(
        prop_count, 0,
        "ws_status should have no parameters, got {prop_count}"
    );
}

// ── Phase 4: Futures WebSocket behavioral tests ───────────────────────

#[tokio::test]
async fn wf_subscribe_ticker_schema_requires_product_ids() {
    let tool = get_tool_schema("wf_subscribe_ticker").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("wf_subscribe_ticker should have required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        names.contains(&"product_ids"),
        "wf_subscribe_ticker should require 'product_ids'. Got: {names:?}"
    );
}

#[tokio::test]
async fn wf_subscribe_ticker_lite_schema_requires_product_ids() {
    let tool = get_tool_schema("wf_subscribe_ticker_lite").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        names.contains(&"product_ids"),
        "wf_subscribe_ticker_lite should require 'product_ids'. Got: {names:?}"
    );
}

#[tokio::test]
async fn wf_send_order_schema_requires_fields() {
    let tool = get_tool_schema("wf_send_order").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("wf_send_order should have required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    for field in &["symbol", "side", "order_type", "size"] {
        assert!(
            names.contains(field),
            "wf_send_order should require '{field}'. Got: {names:?}"
        );
    }
}

#[tokio::test]
async fn wf_send_order_warns_real_money() {
    let tool = get_tool_schema("wf_send_order").await;
    let desc = tool["description"].as_str().unwrap_or("");
    assert!(
        desc.contains("REAL MONEY"),
        "wf_send_order should warn about REAL MONEY. Got: {desc}"
    );
}

#[tokio::test]
async fn wf_cancel_order_warns_real_money() {
    let tool = get_tool_schema("wf_cancel_order").await;
    let desc = tool["description"].as_str().unwrap_or("");
    assert!(
        desc.contains("REAL MONEY"),
        "wf_cancel_order should warn about REAL MONEY. Got: {desc}"
    );
}

#[tokio::test]
async fn wf_unsubscribe_schema_requires_feed() {
    let tool = get_tool_schema("wf_unsubscribe").await;
    let required = tool["inputSchema"]["required"]
        .as_array()
        .expect("wf_unsubscribe should have required array");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        names.contains(&"feed"),
        "wf_unsubscribe should require 'feed'. Got: {names:?}"
    );
    // product_ids should be optional
    assert!(
        !names.contains(&"product_ids"),
        "wf_unsubscribe 'product_ids' should be optional"
    );
}

#[tokio::test]
async fn wf_subscribe_private_feeds_have_no_required_params() {
    for feed_tool in &[
        "wf_subscribe_fills",
        "wf_subscribe_account_log",
        "wf_subscribe_notifications",
        "wf_subscribe_open_orders",
        "wf_subscribe_open_orders_verbose",
        "wf_subscribe_open_positions",
        "wf_subscribe_balances",
        "wf_status",
    ] {
        let tool = get_tool_schema(feed_tool).await;
        let props = &tool["inputSchema"]["properties"];
        let prop_count = props.as_object().map(|m| m.len()).unwrap_or(0);
        assert_eq!(
            prop_count, 0,
            "{feed_tool} should have no parameters, got {prop_count}"
        );
    }
}

#[tokio::test]
async fn wf_subscribe_fills_without_keys_returns_auth_error() {
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "wf_subscribe_fills",
            "arguments": {}
        }
    });

    let responses = mcp_session(vec![call]).await;
    let resp = find_response(&responses, 1).expect("should have response");

    let content = &resp["result"]["content"][0]["text"];
    let text = content.as_str().unwrap_or("");
    assert!(
        text.to_lowercase().contains("auth") || text.to_lowercase().contains("key"),
        "wf_subscribe_fills without keys should return auth error. Got: {text}"
    );
    assert_eq!(resp["result"]["isError"], true);
}

#[tokio::test]
async fn wf_send_order_without_keys_returns_auth_error() {
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "wf_send_order",
            "arguments": {
                "symbol": "PF_XBTUSD",
                "side": "buy",
                "order_type": "lmt",
                "size": 1.0
            }
        }
    });

    let responses = mcp_session(vec![call]).await;
    let resp = find_response(&responses, 1).expect("should have response");

    let content = &resp["result"]["content"][0]["text"];
    let text = content.as_str().unwrap_or("");
    assert!(
        text.to_lowercase().contains("auth") || text.to_lowercase().contains("key"),
        "wf_send_order without keys should return auth error. Got: {text}"
    );
    assert_eq!(resp["result"]["isError"], true);
}

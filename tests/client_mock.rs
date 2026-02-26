//! Integration tests using wiremock to mock Kraken API responses.

use base64::Engine;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

#[tokio::test]
async fn ticker_returns_parsed_response() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": [],
        "result": {
            "XXBTZUSD": {
                "a": ["50000.00000", "1", "1.000"],
                "b": ["49999.90000", "2", "2.000"],
                "c": ["50000.00000", "0.001"],
                "v": ["1000.00000", "2000.00000"],
                "p": ["49500.00000", "49800.00000"],
                "t": [10000, 20000],
                "l": ["48000.00000", "47000.00000"],
                "h": ["51000.00000", "52000.00000"],
                "o": "49000.00000"
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/0/public/Ticker"))
        .and(query_param("pair", "XBTUSD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = common::mock_client(&mock_server.uri(), None, None);
    let result = client.ticker("XBTUSD").await.unwrap();

    assert!(result.contains_key("XXBTZUSD"));
    let ticker = &result["XXBTZUSD"];
    assert_eq!(ticker.a[0], "50000.00000");
    assert_eq!(ticker.b[0], "49999.90000");
    assert_eq!(ticker.o, "49000.00000");
}

#[tokio::test]
async fn ticker_handles_api_error() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": ["EQuery:Unknown asset pair"],
        "result": null
    });

    Mock::given(method("GET"))
        .and(path("/0/public/Ticker"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = common::mock_client(&mock_server.uri(), None, None);
    let result = client.ticker("INVALIDPAIR").await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Unknown asset pair"), "got: {err}");
}

#[tokio::test]
async fn orderbook_parses_array_format() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": [],
        "result": {
            "XXBTZUSD": {
                "asks": [
                    ["50001.00000", "1.000", 1700000000],
                    ["50002.00000", "2.000", 1700000001]
                ],
                "bids": [
                    ["49999.00000", "3.000", 1700000000],
                    ["49998.00000", "4.000", 1700000001]
                ]
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/0/public/Depth"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = common::mock_client(&mock_server.uri(), None, None);
    let book = client.orderbook("XBTUSD", Some(2)).await.unwrap();

    assert_eq!(book.asks.len(), 2);
    assert_eq!(book.bids.len(), 2);
    assert_eq!(book.asks[0].price, "50001.00000");
    assert_eq!(book.asks[1].volume, "2.000");
    assert_eq!(book.bids[0].price, "49999.00000");
    assert_eq!(book.bids[1].volume, "4.000");
}

#[tokio::test]
async fn orderbook_handles_empty_book() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": [],
        "result": {
            "XXBTZUSD": {
                "asks": [],
                "bids": []
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/0/public/Depth"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = common::mock_client(&mock_server.uri(), None, None);
    let book = client.orderbook("XBTUSD", None).await.unwrap();

    assert!(book.asks.is_empty());
    assert!(book.bids.is_empty());
}

#[tokio::test]
async fn balance_requires_auth() {
    let mock_server = MockServer::start().await;
    let client = common::mock_client(&mock_server.uri(), None, None);
    let result = client.balance().await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Authentication required"), "got: {err}");
}

#[tokio::test]
async fn balance_with_auth_sends_headers() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": [],
        "result": {
            "ZUSD": "1000.0000",
            "XXBT": "0.5000",
            "XETH": "10.0000"
        }
    });

    Mock::given(method("POST"))
        .and(path("/0/private/Balance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let secret = base64::engine::general_purpose::STANDARD.encode(b"testsecretkey12345678");
    let client = common::mock_client(
        &mock_server.uri(),
        Some("test-api-key".to_string()),
        Some(secret),
    );
    let result = client.balance().await.unwrap();

    assert_eq!(result["ZUSD"], "1000.0000");
    assert_eq!(result["XXBT"], "0.5000");
    assert_eq!(result["XETH"], "10.0000");
}

#[tokio::test]
async fn balance_handles_rate_limit() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": ["EAPI:Rate limit exceeded"],
        "result": null
    });

    Mock::given(method("POST"))
        .and(path("/0/private/Balance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let secret = base64::engine::general_purpose::STANDARD.encode(b"testsecretkey12345678");
    let client = common::mock_client(
        &mock_server.uri(),
        Some("test-api-key".to_string()),
        Some(secret),
    );
    let result = client.balance().await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Rate limited"), "got: {err}");
}

#[tokio::test]
async fn open_orders_parses_empty() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": [],
        "result": {
            "open": {}
        }
    });

    Mock::given(method("POST"))
        .and(path("/0/private/OpenOrders"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let secret = base64::engine::general_purpose::STANDARD.encode(b"testsecretkey12345678");
    let client = common::mock_client(
        &mock_server.uri(),
        Some("test-api-key".to_string()),
        Some(secret),
    );
    let result = client.open_orders().await.unwrap();
    assert!(result.open.unwrap().is_empty());
}

#[tokio::test]
async fn add_order_validate_mode() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": [],
        "result": {
            "descr": {
                "order": "buy 0.01 XBTUSD @ market"
            }
        }
    });

    Mock::given(method("POST"))
        .and(path("/0/private/AddOrder"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let secret = base64::engine::general_purpose::STANDARD.encode(b"testsecretkey12345678");
    let client = common::mock_client(
        &mock_server.uri(),
        Some("test-api-key".to_string()),
        Some(secret),
    );
    let result = client
        .add_order("XBTUSD", "buy", "market", "0.01", None, true)
        .await
        .unwrap();

    assert_eq!(
        result.descr.unwrap().order.unwrap(),
        "buy 0.01 XBTUSD @ market"
    );
    assert!(result.txid.is_none()); // validate mode returns no txid
}

#[tokio::test]
async fn cancel_order_returns_count() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": [],
        "result": {
            "count": 1
        }
    });

    Mock::given(method("POST"))
        .and(path("/0/private/CancelOrder"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let secret = base64::engine::general_purpose::STANDARD.encode(b"testsecretkey12345678");
    let client = common::mock_client(
        &mock_server.uri(),
        Some("test-api-key".to_string()),
        Some(secret),
    );
    let result = client.cancel_order("TXID-123").await.unwrap();
    assert_eq!(result.count.unwrap(), 1);
}

#[tokio::test]
async fn insufficient_funds_error() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": ["EOrder:Insufficient funds"],
        "result": null
    });

    Mock::given(method("POST"))
        .and(path("/0/private/AddOrder"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let secret = base64::engine::general_purpose::STANDARD.encode(b"testsecretkey12345678");
    let client = common::mock_client(
        &mock_server.uri(),
        Some("test-api-key".to_string()),
        Some(secret),
    );
    let result = client
        .add_order("XBTUSD", "buy", "market", "9999", None, false)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Insufficient funds"), "got: {err}");
}

#[tokio::test]
async fn http_500_returns_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/0/public/Ticker"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = common::mock_client(&mock_server.uri(), None, None);
    let result = client.ticker("XBTUSD").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn trade_history_parses_response() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "error": [],
        "result": {
            "trades": {
                "TXID-001": {
                    "ordertxid": "ORDER-001",
                    "pair": "XXBTZUSD",
                    "time": 1700000000.0,
                    "type": "buy",
                    "ordertype": "market",
                    "price": "50000.00000",
                    "cost": "500.00000",
                    "fee": "0.50000",
                    "vol": "0.01000"
                }
            },
            "count": 1
        }
    });

    Mock::given(method("POST"))
        .and(path("/0/private/TradesHistory"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let secret = base64::engine::general_purpose::STANDARD.encode(b"testsecretkey12345678");
    let client = common::mock_client(
        &mock_server.uri(),
        Some("test-api-key".to_string()),
        Some(secret),
    );
    let result = client.trade_history(None).await.unwrap();
    let trades = result.trades.unwrap();
    assert_eq!(trades.len(), 1);
    let trade = &trades["TXID-001"];
    assert_eq!(trade.price.as_deref(), Some("50000.00000"));
    assert_eq!(trade.trade_type.as_deref(), Some("buy"));
}

#[tokio::test]
async fn grouped_book_uses_public_query_params() {
    let mock_server = MockServer::start().await;
    let body = serde_json::json!({
        "error": [],
        "result": { "pair": "BTC/USD", "grouping": 1000, "bids": [], "asks": [] }
    });
    Mock::given(method("GET"))
        .and(path("/0/public/GroupedBook"))
        .and(query_param("pair", "BTC/USD"))
        .and(query_param("group", "1000"))
        .and(query_param("levels", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;
    let client = common::mock_client(&mock_server.uri(), None, None);
    let result = client
        .grouped_book("BTC/USD", Some(1000), Some(10))
        .await
        .unwrap();
    assert_eq!(result.pair.as_deref(), Some("BTC/USD"));
}

#[tokio::test]
async fn pre_trade_uses_get_endpoint() {
    let mock_server = MockServer::start().await;
    let body = serde_json::json!({
        "error": [],
        "result": { "symbol": "BTC/USD", "bids": [], "asks": [] }
    });
    Mock::given(method("GET"))
        .and(path("/0/public/PreTrade"))
        .and(query_param("symbol", "BTC/USD"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;
    let client = common::mock_client(&mock_server.uri(), None, None);
    let result = client.pre_trade("BTC/USD").await.unwrap();
    assert_eq!(result.symbol.as_deref(), Some("BTC/USD"));
}

#[tokio::test]
async fn post_trade_uses_get_with_filters() {
    let mock_server = MockServer::start().await;
    let body = serde_json::json!({
        "error": [],
        "result": { "trades": [] }
    });
    Mock::given(method("GET"))
        .and(path("/0/public/PostTrade"))
        .and(query_param("symbol", "BTC/USD"))
        .and(query_param("count", "25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;
    let client = common::mock_client(&mock_server.uri(), None, None);
    let result = client
        .post_trade(Some("BTC/USD"), None, None, Some(25))
        .await
        .unwrap();
    assert!(result.trades.is_empty());
}

#[tokio::test]
async fn level3_requires_auth() {
    let mock_server = MockServer::start().await;
    let client = common::mock_client(&mock_server.uri(), None, None);
    let result = client.level3("XBTUSD", Some(10)).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Authentication required"), "got: {err}");
}

#[tokio::test]
async fn credit_lines_and_order_amends_private() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/CreditLines"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": [], "result": { "asset_details": {} }
        })))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/0/private/OrderAmends"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": [], "result": { "amends": [], "count": 0 }
        })))
        .mount(&mock_server)
        .await;
    let secret = base64::engine::general_purpose::STANDARD.encode(b"testsecretkey12345678");
    let client = common::mock_client(&mock_server.uri(), Some("test-key".into()), Some(secret));
    let cl = client.credit_lines().await.unwrap();
    assert!(cl.asset_details.is_empty());
    let oa = client.order_amends("TXID-123").await.unwrap();
    assert_eq!(oa.count, Some(0));
}

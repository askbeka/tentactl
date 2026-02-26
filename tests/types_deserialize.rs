use serde::de::DeserializeOwned;
use serde_json::json;

use tentactl::kraken::futures_types::*;
use tentactl::kraken::types::*;

fn deserialize_ok<T: DeserializeOwned>(value: serde_json::Value) -> T {
    serde_json::from_value(value).expect("expected type to deserialize")
}

#[test]
fn spot_core_response_types_deserialize() {
    let _: ServerTimeResult = deserialize_ok(json!({
        "unixtime": 1700000000,
        "rfc1123": "Wed, 15 Nov 2023 10:00:00 GMT"
    }));

    let _: SystemStatusResult = deserialize_ok(json!({
        "status": "online",
        "timestamp": "2024-01-01T00:00:00Z"
    }));

    let extended: ExtendedBalanceResult = deserialize_ok(json!({
        "ZUSD": {"balance": "100.0", "hold_trade": "1.0", "credit": "0", "credit_used": "0"}
    }));
    assert!(extended.contains_key("ZUSD"));

    let _: TradeBalanceResult = deserialize_ok(json!({
        "eb": "1000.0",
        "tb": "1000.0",
        "m": "0",
        "n": "0",
        "c": "0",
        "v": "0",
        "e": "1000.0",
        "mf": "1000.0"
    }));

    let _: TradeVolumeResult = deserialize_ok(json!({
        "currency": "ZUSD",
        "volume": "15000",
        "fees": {
            "XXBTZUSD": {"fee": "0.26", "minfee": "0", "maxfee": "0"}
        }
    }));

    let exports: ExportStatusResult = deserialize_ok(json!([
        {
            "id": "RID-1",
            "status": "Processed",
            "type": "trades"
        }
    ]));
    assert_eq!(exports.len(), 1);

    let _: CreditLinesResult = deserialize_ok(json!({
        "asset_details": {
            "XBT": {"limit": "1.0", "used": "0.1"}
        }
    }));

    let _: Level3Result = deserialize_ok(json!({
        "asks": [{"order_id": "OID-1", "price": "50000", "qty": "0.1", "timestamp": "1700000000"}],
        "bids": [{"order_id": "OID-2", "price": "49900", "qty": "0.2", "timestamp": "1700000001"}]
    }));

    let _: PreTradeResult = deserialize_ok(json!({
        "symbol": "BTC/USD",
        "bids": [{"price": "50000", "qty": "0.2"}],
        "asks": [{"price": "50010", "qty": "0.1"}]
    }));

    let post: PostTradeResult = deserialize_ok(json!({
        "trades": [{"price": "50000", "qty": "0.1"}]
    }));
    assert_eq!(post.trades.len(), 1);

    let _: OrderAmendsResult = deserialize_ok(json!({
        "amends": [{"txid": "OID-1", "old_price": "50000", "new_price": "49950"}],
        "count": 1
    }));

    let _: CancelAllAfterResult = deserialize_ok(json!({
        "currentTime": "2024-01-01T00:00:00Z",
        "triggerTime": "2024-01-01T00:01:00Z"
    }));

    let _: GroupedBookResult = deserialize_ok(json!({
        "pair": "BTC/USD",
        "grouping": 1000,
        "bids": [{"price": "49900", "qty": "1"}],
        "asks": [{"price": "50000", "qty": "1"}]
    }));
}

#[test]
fn futures_response_types_deserialize() {
    let _: FuturesInstrumentsResult = deserialize_ok(json!({
        "instruments": [{"symbol": "PF_XBTUSD", "tradeable": true}]
    }));

    let _: FuturesTickerResult = deserialize_ok(json!({
        "ticker": {"symbol": "PF_XBTUSD", "last": "50000", "bid": "49999", "ask": "50001"}
    }));

    let _: FuturesTickersResult = deserialize_ok(json!({
        "tickers": [{"symbol": "PF_XBTUSD", "last": "50000"}]
    }));

    let _: FuturesOrderbookResult = deserialize_ok(json!({
        "symbol": "PF_XBTUSD",
        "bids": [{"price": "49999", "qty": "10"}],
        "asks": [{"price": "50001", "qty": "12"}],
        "timestamp": "1700000000"
    }));

    let _: FuturesTradeHistoryResult = deserialize_ok(json!({
        "history": [{"symbol": "PF_XBTUSD", "price": "50000", "size": "2"}]
    }));

    let _: FuturesFeeSchedulesResult = deserialize_ok(json!({
        "feeSchedules": [{"name": "default", "maker": "0.02", "taker": "0.05"}]
    }));

    let _: FuturesFundingRatesResult = deserialize_ok(json!({
        "rates": [{"symbol": "PF_XBTUSD", "rate": "0.0001"}]
    }));

    let _: FuturesAccountsResult = deserialize_ok(json!({
        "accounts": [{"account_name": "Futures Wallet", "balance": "1000"}]
    }));

    let _: FuturesOpenOrdersResult = deserialize_ok(json!({
        "orders": [{"order_id": "OID-1", "symbol": "PF_XBTUSD"}]
    }));

    let _: FuturesOpenPositionsResult = deserialize_ok(json!({
        "positions": [{"symbol": "PF_XBTUSD", "size": "1"}]
    }));

    let _: FuturesFillsResult = deserialize_ok(json!({
        "fills": [{"symbol": "PF_XBTUSD", "price": "50000", "size": "1"}]
    }));

    let _: FuturesTransfersResult = deserialize_ok(json!({
        "transfers": [{"fromAccount": "A", "toAccount": "B", "amount": "100"}]
    }));

    let _: FuturesOrderStatusResult = deserialize_ok(json!({
        "orders": [{"order_id": "OID-1", "status": "filled"}]
    }));

    let _: FuturesLeverageResult = deserialize_ok(json!({
        "symbol": "PF_XBTUSD",
        "max_leverage": "10"
    }));

    let _: FuturesPnlResult = deserialize_ok(json!({
        "symbol": "PF_XBTUSD",
        "pnl_preference": "USD"
    }));

    let _: FuturesSendOrderResult = deserialize_ok(json!({
        "order_id": "OID-1",
        "status": "placed"
    }));

    let _: FuturesCancelResult = deserialize_ok(json!({
        "status": "cancelled",
        "cancelled": 1,
        "order_ids": ["OID-1"]
    }));

    let _: FuturesBatchResult = deserialize_ok(json!({
        "status": "ok",
        "results": [{"order": "send", "order_id": "OID-1"}]
    }));

    let _: FuturesTransferResult = deserialize_ok(json!({
        "transfer_id": "T-1",
        "status": "completed"
    }));
}

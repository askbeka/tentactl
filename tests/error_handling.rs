//! Tests for Kraken API error classification.
//! Tests the actual KrakenError::from_api_errors() — errors sourced from Kraken's API.

use tentactl::kraken::error::KrakenError;

fn classify(errors: Vec<&str>) -> String {
    KrakenError::from_api_errors(errors.into_iter().map(String::from).collect()).to_string()
}

#[test]
fn invalid_key_gives_actionable_message() {
    let msg = classify(vec!["EAPI:Invalid key"]);
    assert!(msg.contains("API key"), "{msg}");
}

#[test]
fn invalid_nonce_mentions_clock() {
    let msg = classify(vec!["EAPI:Invalid nonce"]);
    assert!(msg.contains("clock") || msg.contains("nonce"), "{msg}");
}

#[test]
fn permission_denied_links_to_settings() {
    let msg = classify(vec!["EGeneral:Permission denied"]);
    assert!(msg.contains("permission"), "{msg}");
}

#[test]
fn rate_limit() {
    let msg = classify(vec!["EAPI:Rate limit exceeded"]);
    assert!(
        msg.contains("Rate limited") || msg.contains("slow down"),
        "{msg}"
    );
}

#[test]
fn too_many_requests() {
    let msg = classify(vec!["EService:Too many requests"]);
    assert!(msg.contains("Too many requests"), "{msg}");
}

#[test]
fn insufficient_funds() {
    let msg = classify(vec!["EOrder:Insufficient funds"]);
    assert!(msg.contains("Insufficient funds"), "{msg}");
}

#[test]
fn unknown_order() {
    let msg = classify(vec!["EOrder:Unknown order"]);
    assert!(msg.contains("not found"), "{msg}");
}

#[test]
fn invalid_order() {
    let msg = classify(vec!["EOrder:Invalid order"]);
    assert!(msg.contains("Invalid order"), "{msg}");
}

#[test]
fn market_cancel_only() {
    let msg = classify(vec!["EOrder:Market in cancel_only mode"]);
    assert!(msg.contains("cancel"), "{msg}");
}

#[test]
fn market_post_only() {
    let msg = classify(vec!["EOrder:Market in post_only mode"]);
    assert!(
        msg.contains("post-only") || msg.contains("post only"),
        "{msg}"
    );
}

#[test]
fn market_limit_only() {
    let msg = classify(vec!["EOrder:Market in limit_only mode"]);
    assert!(
        msg.contains("limit") || msg.contains("market orders"),
        "{msg}"
    );
}

#[test]
fn amount_too_small() {
    let msg = classify(vec!["EOrder:Quantity is too small for asset"]);
    assert!(msg.contains("too small"), "{msg}");
}

#[test]
fn amount_too_large() {
    let msg = classify(vec!["EOrder:Quantity is too large for asset"]);
    assert!(msg.contains("too large"), "{msg}");
}

#[test]
fn invalid_price() {
    let msg = classify(vec!["EOrder:Invalid price"]);
    assert!(msg.contains("price"), "{msg}");
}

#[test]
fn unknown_asset_pair() {
    let msg = classify(vec!["EQuery:Unknown asset pair"]);
    assert!(msg.contains("pair"), "{msg}");
}

#[test]
fn unknown_asset() {
    let msg = classify(vec!["EQuery:Unknown asset"]);
    assert!(msg.contains("asset"), "{msg}");
}

#[test]
fn service_unavailable() {
    let msg = classify(vec!["EService:Unavailable"]);
    assert!(
        msg.contains("unavailable") || msg.contains("maintenance"),
        "{msg}"
    );
}

#[test]
fn service_timeout() {
    let msg = classify(vec!["EService:Timeout"]);
    assert!(msg.contains("timeout") || msg.contains("Timeout"), "{msg}");
}

#[test]
fn unknown_method() {
    let msg = classify(vec!["EService:Unknown method"]);
    assert!(msg.contains("method") || msg.contains("endpoint"), "{msg}");
}

#[test]
fn invalid_arguments() {
    let msg = classify(vec!["EGeneral:Invalid arguments"]);
    assert!(msg.contains("Invalid arguments"), "{msg}");
}

#[test]
fn empty_errors() {
    let msg = KrakenError::from_api_errors(vec![]).to_string();
    assert!(msg.contains("Unknown error"), "{msg}");
}

#[test]
fn multiple_errors_first_match_wins() {
    let msg = classify(vec!["EGeneral:Something", "EAPI:Rate limit exceeded"]);
    assert!(
        msg.contains("Rate limited") || msg.contains("slow down"),
        "{msg}"
    );
}

#[test]
fn unknown_error_preserves_raw_message() {
    let msg = classify(vec!["EGeneral:Some brand new error type"]);
    assert!(msg.contains("Some brand new error type"), "{msg}");
}

#[test]
fn auth_required_has_env_var_names() {
    let msg = KrakenError::AuthRequired.to_string();
    assert!(msg.contains("KRAKEN_API_KEY"), "{msg}");
    assert!(msg.contains("KRAKEN_API_SECRET"), "{msg}");
}

#[test]
fn futures_auth_required_has_env_var_names() {
    let msg = KrakenError::FuturesAuthRequired.to_string();
    assert!(msg.contains("KRAKEN_FUTURES_KEY"), "{msg}");
    assert!(msg.contains("KRAKEN_FUTURES_SECRET"), "{msg}");
}

use serde_json::{json, Value};

use crate::support::{response_field, Server};

#[test]
fn update_card_claims_and_releases_lease() {
    let mut s = Server::start();
    assert!(s
        .call(2, "create_card_in_backlog", json!({"title":"claimed work"}))
        .contains("KB-1"));
    let codex = s.call(3, "register_agent", json!({"requested_name":"codex"}));
    let codex_identity = response_field(&codex, "assigned_identity:").to_string();
    let codex_token = response_field(&codex, "claim_token:").to_string();
    let claude = s.call(4, "register_agent", json!({"requested_name":"claude"}));
    let claude_identity = response_field(&claude, "assigned_identity:").to_string();
    let claude_token = response_field(&claude, "claim_token:").to_string();

    assert!(s
        .call(
            5,
            "update_card",
            json!({"key":"KB-1","claim":codex_identity.clone(),"claim_token":codex_token.clone(),"lease_minutes":30})
        )
        .contains("updated KB-1"));
    let claimed = s.call(6, "get_card", json!({"key":"KB-1"}));
    assert!(claimed.contains(&format!("claim: {codex_identity} until lease_expires_at=")));
    let listed = s.call(7, "list_cards", json!({}));
    assert!(listed.contains(&format!("[claimed:{codex_identity}]")));

    s.send(&json!({"jsonrpc":"2.0","id":8,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","claim":claude_identity,"claim_token":claude_token}}}));
    let conflict = s.recv_id(8);
    assert!(
        conflict.get("error").is_some() || conflict["result"]["isError"] == Value::Bool(true),
        "got: {conflict}"
    );

    assert!(s
        .call(
            9,
            "update_card",
            json!({"key":"KB-1","release_claim":true,"claim_token":codex_token})
        )
        .contains("updated KB-1"));
    let released = s.call(10, "get_card", json!({"key":"KB-1"}));
    assert!(released.contains("claim: -"));
}

#[test]
fn due_dates_and_errors() {
    let mut s = Server::start();
    s.call(2, "create_card_in_backlog", json!({"title":"task"}));

    // A past date is flagged overdue (test data is well before any plausible run date).
    s.call(3, "update_card", json!({"key":"KB-1","due":"2000-01-01"}));
    assert!(s
        .call(4, "get_card", json!({"key":"KB-1"}))
        .contains("(overdue)"));
    assert!(s
        .call(5, "get_board", json!({}))
        .contains("!due:2000-01-01"));

    // Bad date -> JSON-RPC error, not a panic.
    s.send(&json!({
        "jsonrpc":"2.0","id":6,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","due":"2026-99-99"}}
    }));
    let resp = s.recv_id(6);
    assert!(
        resp.get("error").is_some() || resp["result"]["isError"] == serde_json::Value::Bool(true),
        "invalid date should surface as an error: {resp}"
    );

    // Clearing works.
    s.call(7, "update_card", json!({"key":"KB-1","due":""}));
    assert!(s
        .call(8, "get_card", json!({"key":"KB-1"}))
        .contains("due: -"));
}

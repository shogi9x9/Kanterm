use serde_json::{json, Value};

use crate::support::Server;

#[test]
fn update_card_rejects_invalid_execution_metadata() {
    let mut s = Server::start();
    assert!(s
        .call(
            2,
            "create_card_in_backlog",
            json!({"title":"execution metadata"})
        )
        .contains("KB-1"));

    s.send(&json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","agent_weight":0}}}));
    let bad_weight = s.recv_id(3);
    assert!(
        bad_weight.get("error").is_some() || bad_weight["result"]["isError"] == Value::Bool(true),
        "got: {bad_weight}"
    );

    s.send(&json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","expected_tokens":0}}}));
    let bad_tokens = s.recv_id(4);
    assert!(
        bad_tokens.get("error").is_some() || bad_tokens["result"]["isError"] == Value::Bool(true),
        "got: {bad_tokens}"
    );

    s.send(&json!({"jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","human_intervention":"maybe"}}}));
    let bad_human = s.recv_id(5);
    assert!(
        bad_human.get("error").is_some() || bad_human["result"]["isError"] == Value::Bool(true),
        "got: {bad_human}"
    );
}

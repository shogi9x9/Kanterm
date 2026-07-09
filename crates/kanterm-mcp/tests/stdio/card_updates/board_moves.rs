use kanterm_core::BoardColumnTemplate;
use serde_json::{json, Value};

use crate::support::Server;

#[test]
fn update_card_detects_stale_expected_updated_at() {
    let mut s = Server::start();
    assert!(s
        .call(2, "create_card_in_backlog", json!({"title":"task"}))
        .contains("KB-1"));

    s.send(&json!({
        "jsonrpc":"2.0",
        "id":3,
        "method":"tools/call",
        "params":{
            "name":"update_card",
            "arguments": {"key":"KB-1","title":"nope","expected_updated_at":0}
        }
    }));
    let resp = s.recv_id(3);
    assert!(resp.get("error").is_some() || resp["result"]["isError"] == Value::Bool(true));
    assert!(resp.to_string().contains("stale update"), "got: {resp}");
}

#[test]
fn board_param_addresses_distinct_boards() {
    // Seed a second board directly via the core before starting the server
    // (the MCP surface deliberately has no create-board tool).
    let db = Server::fresh_db();
    {
        let mut store = kanterm_core::Store::open(&db).unwrap();
        store.ensure_default_board().unwrap();
        let work = store
            .create_board("Work", BoardColumnTemplate::Planning)
            .unwrap();
        store
            .create_card(&work.id, Some("This week"), "ship it", "", "t")
            .unwrap();
    }
    let mut s = Server::start_at(db);

    // Default board (backlog) is empty; its directory lists both boards.
    let backlog = s.call(2, "get_board", json!({}));
    assert!(backlog.contains("boards: backlog (current), work"));
    assert!(!backlog.contains("ship it"));

    // Targeting the work board by slug shows its card and flips "current".
    let work = s.call(3, "get_board", json!({"board":"work"}));
    assert!(work.contains("WOR-1 ship it"), "got: {work}");
    assert!(work.contains("work (current)"));

    // Writes honour the board param too.
    assert!(s
        .call(4, "create_card", json!({"board":"work","title":"second"}))
        .contains("created"));
    assert!(s
        .call(5, "list_cards", json!({"board":"work"}))
        .contains("second"));
    // ...and don't leak onto backlog.
    assert!(s.call(6, "list_cards", json!({})).contains("no matching"));

    // Unknown board errors cleanly.
    s.send(&json!({"jsonrpc":"2.0","id":7,"method":"tools/call",
        "params":{"name":"get_board","arguments":{"board":"ghost"}}}));
    let resp = s.recv_id(7);
    assert!(resp.get("error").is_some() || resp["result"]["isError"] == Value::Bool(true));
}

#[test]
fn update_card_can_move_to_another_board() {
    let mut s = Server::start();

    s.call(
        2,
        "create_card_in_backlog",
        json!({"title":"migrate across boards"}),
    );
    assert!(s
        .call(
            3,
            "manage_boards",
            json!({"action":"create","name":"Work","template":"planning"})
        )
        .contains("slug: work"));
    assert!(s
        .call(
            4,
            "update_card",
            json!({
                "key":"KB-1","move_to_board":"work","column":"This week"
            })
        )
        .contains("updated"));

    assert!(!s
        .call(5, "list_cards", json!({}))
        .contains("migrate across boards"));
    let work = s.call(6, "list_cards", json!({"board":"work"}));
    assert!(work.contains("migrate across boards"));
    let detail = s.call(7, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(detail.contains("move_board KB-1 -> WOR-1; Backlog (backlog) -> Work (work)"));
}

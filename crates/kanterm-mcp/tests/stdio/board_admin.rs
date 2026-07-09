use serde_json::{json, Value};

use crate::support::Server;

#[test]
fn manage_boards_create_and_delete() {
    let mut s = Server::start();

    // create -> appears in the board directory and is addressable
    assert!(s
        .call(2, "manage_boards", json!({"action":"create","name":"Work"}))
        .contains("template: workflow"));
    let work_board = s.call(19, "get_board", json!({"board":"work"}));
    assert!(
        work_board.contains("## In progress (0)"),
        "got: {work_board}"
    );
    assert!(s
        .call(
            15,
            "manage_boards",
            json!({
                "action":"create",
                "name":"Side",
                "template":"simple",
                "agent_context":"Run cargo test -p side before complete_note."
            })
        )
        .contains("with agent_context"));
    let side_board = s.call(20, "get_board", json!({"board":"side"}));
    assert!(side_board.contains("board_agent_context:"));
    assert!(side_board.contains("Run cargo test -p side before complete_note."));
    assert!(side_board.contains("side [context] (current)"));
    s.send(&json!({"jsonrpc":"2.0","id":18,"method":"tools/call",
        "params":{"name":"manage_boards","arguments":{"action":"create","name":"Backlog","template":"planning"}}}));
    let duplicate_backlog = s.recv_id(18);
    assert!(
        duplicate_backlog.get("error").is_some()
            || duplicate_backlog["result"]["isError"] == Value::Bool(true),
        "expected duplicate Backlog board to be rejected: {duplicate_backlog}"
    );
    let dir = s.call(3, "get_board", json!({}));
    assert!(
        dir.contains("boards: backlog (current), work, side [context]"),
        "got: {dir}"
    );
    assert!(
        dir.contains("side [context]"),
        "context marker should appear in board directory: {dir}"
    );
    assert!(s
        .call(
            16,
            "manage_boards",
            json!({"action":"reorder","board":"side","direction":"up"})
        )
        .contains("moved board 'side' up"));
    let dir = s.call(17, "get_board", json!({}));
    assert!(
        dir.contains("boards: backlog (current), side [context], work"),
        "got: {dir}"
    );
    assert!(s
        .call(
            21,
            "manage_boards",
            json!({
                "action":"set_context",
                "board":"work",
                "agent_context":"Use cargo test --workspace before release."
            })
        )
        .contains("updated board 'work' agent_context"));
    s.call(4, "create_card", json!({"board":"work","title":"on work"}));
    let card = s.call(22, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(card.contains("board_agent_context: Use cargo test --workspace before release."));
    assert!(s
        .call(5, "list_cards", json!({"board":"work"}))
        .contains("on work"));
    assert!(s
        .call(
            23,
            "manage_boards",
            json!({"action":"clear_context","board":"work"})
        )
        .contains("cleared board 'work' agent_context"));
    let work_board = s.call(24, "get_board", json!({"board":"work"}));
    assert!(!work_board.contains("Use cargo test --workspace before release."));

    // Backlog is protected; unknown slug errors; deleting a non-archived board errors
    for args in [
        json!({"action":"delete","board":"backlog"}),
        json!({"action":"delete","board":"ghost"}),
        json!({"action":"archive","board":"backlog"}),
        json!({"action":"delete","board":"work"}),
    ] {
        s.send(&json!({"jsonrpc":"2.0","id":6,"method":"tools/call",
            "params":{"name":"manage_boards","arguments":args}}));
        let resp = s.recv_id(6);
        assert!(
            resp.get("error").is_some() || resp["result"]["isError"] == Value::Bool(true),
            "expected error for {args}"
        );
    }

    // archive: leaves the active list, shows up on the archived line, cards kept
    assert!(s
        .call(
            7,
            "manage_boards",
            json!({"action":"archive","board":"work"})
        )
        .contains("archived"));
    let dir = s.call(8, "get_board", json!({}));
    assert!(dir.contains("boards: backlog (current)"), "got: {dir}");
    assert!(dir.contains("archived boards: work"), "got: {dir}");
    assert!(s
        .call(9, "list_cards", json!({"board":"work"}))
        .contains("on work"));

    // unarchive restores it to the active list
    s.call(
        10,
        "manage_boards",
        json!({"action":"unarchive","board":"work"}),
    );
    let dir = s.call(11, "get_board", json!({}));
    assert!(
        dir.contains("boards: backlog (current), side [context], work"),
        "got: {dir}"
    );
    assert!(!dir.contains("archived boards:"), "got: {dir}");

    // archive then delete the work board for good
    s.call(
        12,
        "manage_boards",
        json!({"action":"archive","board":"work"}),
    );
    assert!(s
        .call(
            13,
            "manage_boards",
            json!({"action":"delete","board":"work"})
        )
        .contains("deleted"));
    let dir = s.call(14, "get_board", json!({}));
    assert!(!dir.contains("work"), "got: {dir}");
}

#[test]
fn manage_columns_add_rename_reorder_delete() {
    let mut s = Server::start();
    assert!(s
        .call(
            20,
            "manage_boards",
            json!({"action":"create","name":"Work","template":"planning"})
        )
        .contains("slug: work"));
    s.send(&json!({"jsonrpc":"2.0","id":21,"method":"tools/call",
        "params":{"name":"manage_columns","arguments":{"action":"add","name":"Today"}}}));
    let backlog_column_change = s.recv_id(21);
    assert!(
        backlog_column_change.get("error").is_some()
            || backlog_column_change["result"]["isError"] == Value::Bool(true),
        "expected Backlog board columns to be immutable: {backlog_column_change}"
    );

    // add
    assert!(s
        .call(
            2,
            "manage_columns",
            json!({"board":"work","action":"add","name":"保留"})
        )
        .contains("added"));
    assert!(s
        .call(3, "get_board", json!({"board":"work"}))
        .contains("## 保留 (0)"));

    // rename
    s.call(
        4,
        "manage_columns",
        json!({"board":"work","action":"rename","column":"保留","new_name":"アイスボックス"}),
    );
    assert!(s
        .call(5, "get_board", json!({"board":"work"}))
        .contains("## アイスボックス (0)"));

    // put a card in This week, then delete that column moving cards to Today
    s.call(
        6,
        "create_card",
        json!({"board":"work","title":"wip","column":"This week"}),
    );
    s.call(
        7,
        "manage_columns",
        json!({"board":"work","action":"delete","column":"This week","to":"Today"}),
    );
    let board = s.call(8, "get_board", json!({"board":"work"}));
    assert!(!board.contains("## This week"));
    assert!(
        board.contains("## Today (1)"),
        "card relocated to Today: {board}"
    );

    // delete requires a destination
    s.send(&json!({"jsonrpc":"2.0","id":9,"method":"tools/call",
        "params":{"name":"manage_columns","arguments":{"board":"work","action":"delete","column":"Today"}}}));
    let resp = s.recv_id(9);
    assert!(resp.get("error").is_some() || resp["result"]["isError"] == Value::Bool(true));
}

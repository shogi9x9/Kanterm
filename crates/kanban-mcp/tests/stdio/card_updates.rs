use kanban_core::BoardColumnTemplate;
use serde_json::{json, Value};

use crate::support::{response_field, Server};

#[test]
fn create_update_move_and_label_flow() {
    let mut s = Server::start();
    assert!(s
        .call(
            14,
            "manage_boards",
            json!({"action":"create","name":"Work","template":"planning"})
        )
        .contains("slug: work"));

    assert!(s
        .call(
            2,
            "create_card",
            json!({"board":"work","title":"fix bug","column":"Today"})
        )
        .contains("WOR-1"));
    s.call(
        3,
        "update_card",
        json!({
            "board":"work",
            "key":"WOR-1",
            "add_labels":["bug"],
            "priority":2,
            "column":"This week",
            "next_action":"write regression test",
            "blocked_reason":"needs reproduction",
            "acceptance_criteria":"get_card shows agent fields",
            "handoff_note":"resume from MCP get_card",
            "execution_note":"tried parser fixture approach; next resume from failing test",
            "agent_weight":3,
            "agent_effort":"high-reasoning",
            "suggested_model":"gpt-5",
            "expected_tokens":12000,
            "human_intervention":"review",
            "last_verification":{
                "command":"cargo test",
                "status":"passed",
                "summary":"stdio flow passed",
                "timestamp":12345
            }
        }),
    );
    s.call(
        13,
        "record_memory",
        json!({
            "title":"Remember card context",
            "body":"Memory is linked to this card",
            "kind":"decision",
            "card":"WOR-1"
        }),
    );

    let card = s.call(4, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(card.contains("column: This week"));
    assert!(card.contains("priority: [H]"));
    assert!(card.contains("labels: bug"));
    assert!(card.contains("\nagent_metadata:\n"));
    assert!(card.contains("agent_state: open"));
    assert!(card.contains("agent_weight: 3"));
    assert!(card.contains("agent_effort: high-reasoning"));
    assert!(card.contains("suggested_model: gpt-5"));
    assert!(card.contains("expected_tokens: 12000"));
    assert!(card.contains("human_intervention: review"));
    assert!(card.contains("\nbody:\n"));
    assert!(card.contains("next_action: write regression test"));
    assert!(card.contains("blocked_reason: needs reproduction"));
    assert!(card.contains("acceptance_criteria: get_card shows agent fields"));
    assert!(card.contains("handoff_note: resume from MCP get_card"));
    assert!(card.contains("last_verification: {\"command\":\"cargo test\""));
    assert!(card.contains("\nexecution_notes:\n- "));
    assert!(card.contains("tried parser fixture approach; next resume from failing test"));
    assert!(card.contains("\"status\":\"passed\""));
    assert!(card.contains("\"timestamp\":12345"));
    assert!(card.contains("activity:\n- "));
    assert!(card.contains("agent update WOR-1"));
    assert!(card.contains("related_memories:\n- M-1 [decision] Remember card context"));

    let board = s.call(5, "get_board", json!({"board":"work"}));
    assert!(board.contains("## This week (1)"));
    assert!(board.contains("WOR-1 fix bug"));
    assert!(board.contains("[w:3 human:review]"));

    // Filter by query.
    let listed = s.call(6, "list_cards", json!({"board":"work","query":"fix"}));
    assert!(listed.contains("WOR-1"));
    assert!(listed.contains("[blocked]"));
    assert!(listed.contains("[w:3 effort:high-reasoning model:gpt-5 tokens:12000 human:review]"));
    let by_next = s.call(
        7,
        "list_cards",
        json!({"board":"work","query":"regression"}),
    );
    assert!(by_next.contains("WOR-1"));
    let by_criteria = s.call(
        8,
        "list_cards",
        json!({"board":"work","query":"agent fields"}),
    );
    assert!(by_criteria.contains("WOR-1"));
    let by_execution_metadata = s.call(
        15,
        "list_cards",
        json!({
            "board":"work",
            "agent_weight_max":3,
            "agent_effort":"high-reasoning",
            "suggested_model":"gpt-5",
            "expected_tokens_max":15000,
            "human_intervention":"review"
        }),
    );
    assert!(by_execution_metadata.contains("WOR-1"));
    let too_small_budget = s.call(
        16,
        "list_cards",
        json!({"board":"work","expected_tokens_max":1000}),
    );
    assert!(too_small_budget.contains("no matching"));
    let by_metadata_query = s.call(
        17,
        "list_cards",
        json!({"board":"work","query":"high-reasoning"}),
    );
    assert!(by_metadata_query.contains("WOR-1"));
    s.call(
        9,
        "update_card",
        json!({"board":"work","key":"WOR-1","blocked_reason":""}),
    );
    let cleared = s.call(10, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(cleared.contains("blocked_reason: -"));
    let after_clear = s.call(
        11,
        "list_cards",
        json!({"board":"work","query":"regression"}),
    );
    assert!(after_clear.contains("[next]"));
    assert!(!after_clear.contains("[blocked]"));
    let empty = s.call(
        12,
        "list_cards",
        json!({"board":"work","query":"nonexistent"}),
    );
    assert!(empty.contains("no matching"));

    s.call(
        18,
        "update_card",
        json!({
            "board":"work",
            "key":"WOR-1",
            "agent_weight":null,
            "agent_effort":"",
            "suggested_model":"",
            "expected_tokens":null,
            "human_intervention":""
        }),
    );
    let cleared_metadata = s.call(19, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(cleared_metadata.contains("agent_weight: -"));
    assert!(cleared_metadata.contains("agent_effort: -"));
    assert!(cleared_metadata.contains("suggested_model: -"));
    assert!(cleared_metadata.contains("expected_tokens: -"));
    assert!(cleared_metadata.contains("human_intervention: -"));
}

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

#[test]
fn execution_notes_are_append_only_resume_history() {
    let mut s = Server::start();
    assert!(s
        .call(2, "create_card_in_backlog", json!({"title":"resume work"}))
        .contains("KB-1"));
    s.call(
        3,
        "update_card",
        json!({
            "key":"KB-1",
            "next_action":"continue from failing test",
            "acceptance_criteria":"tests pass",
            "execution_note":"first attempt hit stale fixture"
        }),
    );
    s.call(
        4,
        "update_card",
        json!({
            "key":"KB-1",
            "execution_note":"second attempt narrowed failure to renderer"
        }),
    );
    let card = s.call(5, "get_card", json!({"key":"KB-1"}));
    assert!(card.contains("execution_notes:\n- "));
    assert!(card.contains("first attempt hit stale fixture"));
    assert!(card.contains("second attempt narrowed failure to renderer"));

    s.call(
        6,
        "update_card",
        json!({"key":"KB-1","complete_note":"done after renderer fix"}),
    );
    let completed = s.call(7, "get_card", json!({"key":"KB-1"}));
    assert!(completed.contains("agent_state: done"));
    assert!(completed.contains("next_action: -"));
    assert!(completed.contains("execution_notes:\n- "));
    assert!(completed.contains("first attempt hit stale fixture"));
    assert!(completed.contains("second attempt narrowed failure to renderer"));
}

#[test]
fn update_card_with_complete_note_appends_body_and_archives() {
    let mut s = Server::start();

    assert!(s
        .call(
            2,
            "create_card_in_backlog",
            json!({"title":"release","body":"実装内容"})
        )
        .contains("KB-1"));

    assert!(s
        .call(
            3,
            "update_card",
            json!({"key":"KB-1","complete_note":"CI 通過を確認"})
        )
        .contains("updated"));

    let card = s.call(4, "get_card", json!({"key":"KB-1"}));
    assert!(card.contains("実装内容"));
    assert!(card.contains("[completion note] CI 通過を確認"));
    assert!(card.contains("agent_state: done"));
    assert!(card.contains("next_action: -"));
    assert!(card.contains("blocked_reason: -"));
    assert!(card.contains("handoff_note: -"));
    assert!(card.contains("claim: -"));

    let board = s.call(5, "list_cards", json!({}));
    assert!(!board.contains("KB-1"));
}

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
        let mut store = kanban_core::Store::open(&db).unwrap();
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

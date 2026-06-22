use serde_json::json;

use crate::support::Server;

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

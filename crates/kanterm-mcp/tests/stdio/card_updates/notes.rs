use serde_json::json;

use crate::support::Server;

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

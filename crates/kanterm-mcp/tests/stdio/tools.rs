use serde_json::json;

use crate::support::Server;

#[test]
fn exposes_the_expected_tools() {
    let mut s = Server::start();
    let mut names = s.tool_names(2);
    names.sort();
    assert_eq!(
        names,
        vec![
            "claim_handoff",
            "complete_handoff",
            "create_card",
            "create_card_in_backlog",
            "create_cards",
            "dependency_graph",
            "get_board",
            "get_card",
            "get_handoff",
            "list_cards",
            "list_handoffs",
            "manage_boards",
            "manage_columns",
            "recall_memories",
            "record_memory",
            "register_agent",
            "send_handoff",
            "status",
            "update_card",
        ],
    );
}

#[test]
fn status_reports_runtime_identity() {
    let mut s = Server::start();
    let status = s.call(2, "status", json!({}));
    assert!(status.contains("kanban_mcp_status:"));
    assert!(status.contains("version:"));
    assert!(status.contains("schema_version:"));
    assert!(status.contains("db_path:"));
    assert!(status.contains("working_directory:"));
    assert!(status.contains("default_board: backlog (Backlog)"));
}

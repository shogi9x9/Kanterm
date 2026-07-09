use serde_json::json;

use crate::support::Server;

#[test]
fn create_card_requires_board_and_does_not_create_card() {
    let mut s = Server::start();
    let err = s.call_error(2, "create_card", json!({"title":"missing board"}));
    assert!(err.contains("`board` is required"), "got: {err}");
    assert!(s.call(3, "list_cards", json!({})).contains("no matching"));
}

#[test]
fn create_cards_requires_board_and_does_not_create_card() {
    let mut s = Server::start();
    let err = s.call_error(
        2,
        "create_cards",
        json!({"cards":[{"title":"missing board"}]}),
    );
    assert!(err.contains("`board` is required"), "got: {err}");
    assert!(s.call(3, "list_cards", json!({})).contains("no matching"));
}

#[test]
fn create_card_existing_project_slug_reports_existing_board() {
    let mut s = Server::start();
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"Work","template":"workflow"}),
    );
    let created = s.call(
        3,
        "create_card",
        json!({"board":"work","title":"project task","column":"Todo"}),
    );
    assert!(created.contains("created WOR-1 in board 'work' (board: existing) column 'Todo'"));
}

#[test]
fn create_card_unknown_project_name_creates_board_and_card_there() {
    let mut s = Server::start();
    let created = s.call(
        2,
        "create_card",
        json!({"board":"New Project","title":"first project task"}),
    );
    assert!(created.contains("board 'new-project' (board: created)"));
    let board = s.call(3, "get_board", json!({"board":"new-project"}));
    assert!(board.contains("first project task"), "got: {board}");
    assert!(s.call(4, "list_cards", json!({})).contains("no matching"));
}

#[test]
fn create_card_rejects_backlog_board_with_guidance() {
    let mut s = Server::start();
    let err = s.call_error(
        2,
        "create_card",
        json!({"board":"backlog","title":"wrong inbox path"}),
    );
    assert!(
        err.contains(
            "create_card cannot target the Backlog board; use create_card_in_backlog instead."
        ),
        "got: {err}"
    );
}

#[test]
fn create_card_in_backlog_creates_card_in_backlog() {
    let mut s = Server::start();
    let created = s.call(2, "create_card_in_backlog", json!({"title":"inbox item"}));
    assert!(created.contains("created KB-1 in Backlog (board: backlog)"));
    let listed = s.call(3, "list_cards", json!({}));
    assert!(listed.contains("KB-1"), "got: {listed}");
    assert!(listed.contains("inbox item"), "got: {listed}");
}

#[test]
fn create_cards_unknown_project_name_creates_one_board_with_dependencies() {
    let mut s = Server::start();
    let created = s.call(
        2,
        "create_cards",
        json!({
            "board":"Launch Plan",
            "cards":[
                {"alias":"A","title":"A setup","column":"Todo"},
                {"alias":"B","title":"B ship","column":"Todo","depends_on":["A"]}
            ]
        }),
    );
    assert!(created.contains("created 2 cards in board 'launch-plan' (board: created)"));
    assert!(created.contains("1 LP-1 A setup"));
    assert!(created.contains("2 LP-2 B ship"));
    let board = s.call(3, "get_board", json!({"board":"launch-plan"}));
    assert!(board.contains("A setup"));
    assert!(board.contains("B ship"));
    let graph = s.call(4, "dependency_graph", json!({"board":"launch-plan"}));
    assert!(graph.contains("- LP-1 -> LP-2"), "got: {graph}");
}

#[test]
fn create_cards_imports_ordered_execution_plan() {
    let mut s = Server::start();
    assert!(s
        .call(
            2,
            "manage_boards",
            json!({"action":"create","name":"Plan","template":"workflow"})
        )
        .contains("slug: plan"));

    let created = s.call(
        3,
        "create_cards",
        json!({
            "board":"plan",
            "cards":[
                {
                    "alias":"A",
                    "title":"A prepare fixtures",
                    "body":"Set up test fixtures",
                    "column":"Todo",
                    "acceptance_criteria":"fixtures exist",
                    "next_action":"write fixture files",
                    "agent_weight":1,
                    "agent_effort":"low",
                    "suggested_model":"gpt-5-mini",
                    "expected_tokens":1000,
                    "human_intervention":"none"
                },
                {
                    "alias":"B",
                    "title":"B implement parser",
                    "body":"Implement parser changes",
                    "column":"Todo",
                    "acceptance_criteria":"parser tests pass",
                    "next_action":"change parser",
                    "depends_on":["A"],
                    "agent_weight":4,
                    "agent_effort":"high",
                    "suggested_model":"gpt-5",
                    "expected_tokens":12000,
                    "human_intervention":"review"
                }
            ]
        }),
    );
    assert!(created.contains("1 PLA-1 A prepare fixtures"));
    assert!(created.contains("2 PLA-2 B implement parser"));

    let listed = s.call(4, "list_cards", json!({"board":"plan"}));
    let first = listed.find("PLA-1").unwrap();
    let second = listed.find("PLA-2").unwrap();
    assert!(first < second, "{listed}");
    assert!(listed.contains("[w:1 effort:low model:gpt-5-mini tokens:1000]"));
    assert!(listed.contains("[w:4 effort:high model:gpt-5 tokens:12000 human:review]"));

    let card = s.call(5, "get_card", json!({"board":"plan","key":"PLA-2"}));
    assert!(card.contains("acceptance_criteria: parser tests pass"));
    assert!(card.contains("next_action: change parser"));
    assert!(card.contains("agent_weight: 4"));
    assert!(card.contains("human_intervention: review"));
    assert!(card.contains("upstream: PLA-1"));
    assert!(card.contains("readiness: dependency_blocked by PLA-1"));

    let graph = s.call(6, "dependency_graph", json!({"board":"plan"}));
    assert!(graph.contains("- PLA-1 -> PLA-2"));
    assert!(graph.contains("stage 1: PLA-1"));
    assert!(graph.contains("stage 2: PLA-2"));

    let followup = s.call(
        7,
        "create_cards",
        json!({
            "board":"plan",
            "cards":[
                {
                    "alias":"C",
                    "title":"C existing-key follow-up",
                    "column":"Todo",
                    "acceptance_criteria":"existing dependency works",
                    "next_action":"depend on existing card",
                    "depends_on":["PLA-1"]
                }
            ]
        }),
    );
    assert!(followup.contains("1 PLA-3 C existing-key follow-up"));
    let followup_card = s.call(8, "get_card", json!({"board":"plan","key":"PLA-3"}));
    assert!(followup_card.contains("upstream: PLA-1"));
}

#[test]
fn create_cards_rejects_unknown_dependency_alias_and_cycles() {
    let mut s = Server::start();
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"Bad Plan","template":"workflow"}),
    );
    let unknown = s.call_error(
        3,
        "create_cards",
        json!({
            "board":"bad-plan",
            "cards":[
                {"alias":"A","title":"A","depends_on":["missing"]}
            ]
        }),
    );
    assert!(unknown.contains("unknown alias or key 'missing'"));
    let after_unknown = s.call(4, "list_cards", json!({"board":"bad-plan"}));
    assert_eq!(after_unknown, "(no matching cards)");

    s.call(
        7,
        "create_card",
        json!({"board":"bad-plan","title":"existing"}),
    );
    let alias_collision = s.call_error(
        8,
        "create_cards",
        json!({
            "board":"bad-plan",
            "cards":[
                {"alias":"BP-1","title":"collides with existing key"}
            ]
        }),
    );
    assert!(alias_collision.contains("alias conflicts with existing card key 'BP-1'"));
    let after_alias_collision = s.call(9, "list_cards", json!({"board":"bad-plan"}));
    assert!(after_alias_collision.contains("BP-1"));
    assert!(!after_alias_collision.contains("BP-2"));

    let cycle = s.call_error(
        5,
        "create_cards",
        json!({
            "board":"bad-plan",
            "cards":[
                {"alias":"B","title":"B","depends_on":["C"]},
                {"alias":"C","title":"C","depends_on":["B"]}
            ]
        }),
    );
    assert!(cycle.contains("cycle"));
    let after_cycle = s.call(6, "list_cards", json!({"board":"bad-plan"}));
    assert!(after_cycle.contains("BP-1"));
    assert!(!after_cycle.contains("BP-2"));
    assert!(!after_cycle.contains("BP-3"));
}

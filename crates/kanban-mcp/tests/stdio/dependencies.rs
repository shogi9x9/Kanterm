use serde_json::json;

use crate::support::{response_field, Server};

#[test]
fn list_cards_queue_modes_select_next_work() {
    let mut s = Server::start();
    assert!(s
        .call(
            2,
            "manage_boards",
            json!({"action":"create","name":"Queue","template":"workflow"})
        )
        .contains("slug: queue"));
    s.call(
        3,
        "create_cards",
        json!({
            "board":"queue",
            "cards":[
                {
                    "title":"ready",
                    "column":"Todo",
                    "acceptance_criteria":"done means tested",
                    "next_action":"run test",
                    "agent_weight":2,
                    "expected_tokens":2000,
                    "human_intervention":"none"
                },
                {
                    "title":"blocked",
                    "column":"Todo",
                    "acceptance_criteria":"unblocked",
                    "next_action":"wait",
                    "human_intervention":"none"
                },
                {
                    "title":"review",
                    "column":"Todo",
                    "acceptance_criteria":"human signs off",
                    "next_action":"present diff",
                    "human_intervention":"review"
                },
                {
                    "title":"missing context",
                    "column":"Todo",
                    "human_intervention":"none"
                },
                {
                    "title":"claimed",
                    "column":"Todo",
                    "acceptance_criteria":"claim held",
                    "next_action":"work",
                    "human_intervention":"none"
                },
                {
                    "title":"dependency wait",
                    "column":"Todo",
                    "acceptance_criteria":"upstream complete",
                    "next_action":"resume",
                    "human_intervention":"none"
                },
                {
                    "title":"ready high priority",
                    "column":"Todo",
                    "acceptance_criteria":"done means tested",
                    "next_action":"run focused test",
                    "agent_weight":1,
                    "expected_tokens":500,
                    "human_intervention":"none"
                }
            ]
        }),
    );
    s.call(
        4,
        "update_card",
        json!({"board":"queue","key":"QUE-2","blocked_reason":"waiting for input"}),
    );
    let agent = s.call(5, "register_agent", json!({"requested_name":"queue"}));
    let identity = response_field(&agent, "assigned_identity:").to_string();
    let token = response_field(&agent, "claim_token:").to_string();
    s.call(
        6,
        "update_card",
        json!({"board":"queue","key":"QUE-5","claim":identity,"claim_token":token}),
    );
    s.call(
        7,
        "update_card",
        json!({"board":"queue","key":"QUE-6","depends_on":["QUE-1"]}),
    );
    s.call(
        8,
        "update_card",
        json!({"board":"queue","key":"QUE-7","priority":2}),
    );

    let executable = s.call(
        9,
        "list_cards",
        json!({"board":"queue","queue":"executable"}),
    );
    assert!(executable.contains("QUE-1"));
    assert!(executable.contains("QUE-7"));
    assert!(executable.contains("[queue:executable]"));
    assert!(!executable.contains("QUE-2"));
    assert!(!executable.contains("QUE-3"));
    assert!(!executable.contains("QUE-4"));
    assert!(!executable.contains("QUE-5"));
    assert!(!executable.contains("QUE-6"));

    let ranked = s.call(
        10,
        "list_cards",
        json!({"board":"queue","queue":"executable","ranked":true}),
    );
    let high = ranked.find("QUE-7").unwrap();
    let normal = ranked.find("QUE-1").unwrap();
    assert!(high < normal, "{ranked}");
    assert!(ranked.contains("[rank:priority=high weight=1 tokens=500]"));

    let blocked = s.call(11, "list_cards", json!({"board":"queue","queue":"blocked"}));
    assert!(blocked.contains("QUE-2"));
    assert!(blocked.contains("[queue:blocked]"));

    let review = s.call(12, "list_cards", json!({"board":"queue","queue":"review"}));
    assert!(review.contains("QUE-3"));
    assert!(review.contains("[queue:review]"));

    let missing = s.call(
        13,
        "list_cards",
        json!({"board":"queue","queue":"missing_context"}),
    );
    assert!(missing.contains("QUE-4"));
    assert!(missing.contains("[queue:missing-context]"));

    let claimed = s.call(14, "list_cards", json!({"board":"queue","queue":"claimed"}));
    assert!(claimed.contains("QUE-5"));
    assert!(claimed.contains("[queue:claimed]"));

    let dependency_blocked = s.call(
        15,
        "list_cards",
        json!({"board":"queue","queue":"dependency_blocked"}),
    );
    assert!(dependency_blocked.contains("QUE-6"));
    assert!(dependency_blocked.contains("[blocked_by:QUE-1]"));
    assert!(dependency_blocked.contains("[queue:dependency-blocked]"));
}

#[test]
fn dependency_graph_renders_stages_and_blockers() {
    let mut s = Server::start();
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"Graph","template":"workflow"}),
    );
    s.call(
        3,
        "create_cards",
        json!({
            "board":"graph",
            "cards":[
                {"title":"A","acceptance_criteria":"done","next_action":"do A"},
                {"title":"B","acceptance_criteria":"done","next_action":"do B"},
                {"title":"C","acceptance_criteria":"done","next_action":"do C"},
                {"title":"D","acceptance_criteria":"done","next_action":"do D"},
                {"title":"E","acceptance_criteria":"done","next_action":"do E"}
            ]
        }),
    );
    s.call(
        4,
        "update_card",
        json!({"board":"graph","key":"GRA-2","depends_on":["GRA-1"]}),
    );
    s.call(
        5,
        "update_card",
        json!({"board":"graph","key":"GRA-3","depends_on":["GRA-1"]}),
    );
    s.call(
        6,
        "update_card",
        json!({"board":"graph","key":"GRA-4","depends_on":["GRA-1"]}),
    );
    s.call(
        7,
        "update_card",
        json!({"board":"graph","key":"GRA-5","depends_on":["GRA-2","GRA-3","GRA-4"]}),
    );
    s.call(
        8,
        "update_card",
        json!({"board":"graph","key":"GRA-5","human_intervention":"decision"}),
    );

    let graph = s.call(9, "dependency_graph", json!({"board":"graph"}));
    assert!(graph.contains("- GRA-1 -> GRA-2"));
    assert!(graph.contains("- GRA-1 -> GRA-3"));
    assert!(graph.contains("- GRA-1 -> GRA-4"));
    assert!(graph.contains("stage 1: GRA-1"));
    assert!(graph.contains("stage 2: GRA-2(dep-blocked), GRA-3(dep-blocked), GRA-4(dep-blocked)"));
    assert!(graph.contains("stage 3: GRA-5(human:decision)"));

    let blocked = s.call(
        10,
        "list_cards",
        json!({"board":"graph","queue":"dependency_blocked"}),
    );
    assert!(blocked.contains("GRA-2"));
    assert!(blocked.contains("[blocked_by:GRA-1]"));

    let card = s.call(11, "get_card", json!({"board":"graph","key":"GRA-2"}));
    assert!(card.contains("dependencies:"));
    assert!(card.contains("upstream: GRA-1"));
    assert!(card.contains("readiness: dependency_blocked by GRA-1"));

    s.call(
        12,
        "update_card",
        json!({"board":"graph","key":"GRA-1","agent_state":"done","archived":true}),
    );
    let executable = s.call(
        13,
        "list_cards",
        json!({"board":"graph","queue":"executable"}),
    );
    assert!(executable.contains("GRA-2"));
    assert!(executable.contains("GRA-3"));
    assert!(executable.contains("GRA-4"));
    assert!(!executable.contains("GRA-5"));

    let active_graph = s.call(
        14,
        "dependency_graph",
        json!({"board":"graph","active_only":true}),
    );
    assert!(!active_graph.contains("- GRA-1 -> GRA-2"));
    assert!(!active_graph.contains("- GRA-1 -> GRA-3"));
    assert!(!active_graph.contains("- GRA-1 -> GRA-4"));
    assert!(active_graph.contains("- GRA-2 -> GRA-5"));

    let focused_graph = s.call(
        15,
        "dependency_graph",
        json!({"board":"graph","focus":"GRA-5","active_only":true}),
    );
    assert!(focused_graph.contains("- GRA-2 -> GRA-5"));
    assert!(focused_graph.contains("- GRA-3 -> GRA-5"));
    assert!(focused_graph.contains("- GRA-4 -> GRA-5"));
    assert!(!focused_graph.contains("- GRA-1 -> GRA-2"));

    let missing_focus = s.call_error(
        16,
        "dependency_graph",
        json!({"board":"graph","focus":"GRA-999"}),
    );
    assert!(missing_focus.contains("no card 'GRA-999'"));
}

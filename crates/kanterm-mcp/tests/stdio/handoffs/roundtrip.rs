use super::*;

#[test]
fn handoff_round_trip_claims_and_completes() {
    let mut s = Server::start();
    let registered = s.call(
        2,
        "register_agent",
        json!({"requested_name": "claude", "lease_minutes": 30}),
    );
    let identity = response_field(&registered, "assigned_identity:").to_string();
    let token = response_field(&registered, "claim_token:").to_string();

    let sent = s.call(
        3,
        "send_handoff",
        json!({
            "from_agent": "codex#sender",
            "to_agent": "claude",
            "subject": "Start next project",
            "body": "Please claim executable work on the target board."
        }),
    );
    let handoff_id = response_field(&sent, "handoff_sent:").to_string();

    let inbox = s.call(4, "list_handoffs", json!({"for_agent": identity}));
    assert!(inbox.contains(&handoff_id));
    assert!(inbox.contains("[pending]"));

    let claimed = s.call(
        5,
        "claim_handoff",
        json!({"id": handoff_id, "claimant": identity, "claim_token": token}),
    );
    assert!(claimed.contains("handoff_claimed:"));
    assert!(claimed.contains("body:\nPlease claim executable work"));

    let completed = s.call(
        6,
        "complete_handoff",
        json!({
            "id": response_field(&claimed, "handoff_claimed:"),
            "claimant": response_field(&claimed, "claimed_by:"),
            "claim_token": token,
            "status": "completed",
            "note": "Delivered the finished implementation."
        }),
    );
    assert!(completed.contains("status: completed"));

    let empty = s.call(7, "list_handoffs", json!({"for_agent": identity}));
    assert_eq!(empty, "no handoffs");

    let sent_items = s.call(
        8,
        "list_handoffs",
        json!({"from_agent": "codex#sender", "status": "completed"}),
    );
    assert!(sent_items.contains(&handoff_id));
    assert!(sent_items.contains("[completed]"));

    let detail = s.call(9, "get_handoff", json!({"id": handoff_id}));
    assert!(detail.contains("status: completed"));
    assert!(detail.contains("result:\nDelivered the finished implementation."));
}

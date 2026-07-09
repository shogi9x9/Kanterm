use super::*;

#[test]
fn create_move_and_list() {
    let mut s = Store::open_in_memory().unwrap();
    s.ensure_default_board().unwrap();
    let b = s
        .create_board("Work", BoardColumnTemplate::Planning)
        .unwrap();
    let cols = s.columns(&b.id).unwrap();
    assert_eq!(cols.len(), BoardColumnTemplate::Planning.columns().len());

    let c = s.create_card(&b.id, None, "first", "body", "test").unwrap();
    assert_eq!(c.key, "WOR-1");
    assert_eq!(c.agent_state, "open");

    let c2 = s
        .create_card(&b.id, Some("This week"), "second", "", "test")
        .unwrap();
    assert_eq!(c2.key, "WOR-2");

    let moved = s.move_card(&b.id, "WOR-1", "This month", "test").unwrap();
    let done = cols.iter().find(|c| c.name == "This month").unwrap();
    assert_eq!(moved.column_id, done.id);

    let all = s.cards(&b.id).unwrap();
    assert_eq!(all.len(), 2);

    let patched = s
        .update_card(
            &b.id,
            "WOR-2",
            &CardPatch {
                title: Some("renamed".into()),
                priority: Some(PRIORITY_HIGH),
                ..Default::default()
            },
            "test",
        )
        .unwrap();
    assert_eq!(patched.title, "renamed");
    assert_eq!(patched.priority, PRIORITY_HIGH);

    s.update_card(
        &b.id,
        "WOR-1",
        &CardPatch {
            archived: Some(true),
            ..Default::default()
        },
        "test",
    )
    .unwrap();
    assert_eq!(s.cards(&b.id).unwrap().len(), 1);
}

#[test]
fn labels_reorder_and_export() {
    let mut s = Store::open_in_memory().unwrap();
    s.ensure_default_board().unwrap();
    let b = s
        .create_board("Work", BoardColumnTemplate::Planning)
        .unwrap();

    s.create_card(&b.id, Some("Today"), "a", "", "t").unwrap();
    s.create_card(&b.id, Some("Today"), "b", "", "t").unwrap();
    s.create_card(&b.id, Some("Today"), "c", "", "t").unwrap();

    // Attach two labels to WOR-1, then drop one.
    s.update_card(
        &b.id,
        "WOR-1",
        &CardPatch {
            add_labels: Some(vec!["bug".into(), "urgent".into()]),
            ..Default::default()
        },
        "t",
    )
    .unwrap();
    s.update_card(
        &b.id,
        "WOR-1",
        &CardPatch {
            remove_labels: Some(vec!["urgent".into()]),
            ..Default::default()
        },
        "t",
    )
    .unwrap();
    let by_card = s.labels_by_card(&b.id).unwrap();
    let kb1 = s.card_by_key(&b.id, "WOR-1").unwrap().unwrap();
    assert_eq!(by_card.get(&kb1.id).unwrap().len(), 1);
    assert_eq!(by_card.get(&kb1.id).unwrap()[0].name, "bug");

    // Order in Todo is a, b, c. Move WOR-1 down -> b, a, c.
    s.reorder_card(&b.id, "WOR-1", 1).unwrap();
    let todo_col = s
        .columns(&b.id)
        .unwrap()
        .into_iter()
        .find(|c| c.name == "Today")
        .unwrap();
    let order: Vec<String> = s
        .cards(&b.id)
        .unwrap()
        .into_iter()
        .filter(|c| c.column_id == todo_col.id)
        .map(|c| c.key)
        .collect();
    assert_eq!(order, vec!["WOR-2", "WOR-1", "WOR-3"]);

    let json = s.export_json(&b.id).unwrap();
    assert!(json.contains("\"bug\""));
    let md = s.export_markdown(&b.id).unwrap();
    assert!(md.contains("## Today (3)"));
}

#[test]
fn dates() {
    // Known anchor: 2000-01-01 is 10957 days after the epoch.
    assert_eq!(parse_date("2000-01-01").unwrap(), 10957 * MS_PER_DAY);
    for s in ["1970-01-01", "2024-02-29", "2026-06-11", "1999-12-31"] {
        assert_eq!(format_date(parse_date(s).unwrap()), s);
    }
    assert!(parse_date("2026-13-01").is_err());
    assert!(parse_date("nope").is_err());
    // Calendar-aware day validation.
    assert!(parse_date("2026-02-31").is_err());
    assert!(parse_date("2025-02-29").is_err()); // 2025 is not a leap year
    assert!(parse_date("2024-02-29").is_ok()); // 2024 is
    assert!(parse_date("2026-04-31").is_err()); // April has 30 days
    assert!(parse_date("2026-00-10").is_err());

    let mut store = Store::open_in_memory().unwrap();
    let b = store.ensure_default_board().unwrap();
    store.create_card(&b.id, None, "task", "", "t").unwrap();
    store
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                due: Some("2026-06-20".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    let c = store.card_by_key(&b.id, "KB-1").unwrap().unwrap();
    assert_eq!(format_date(c.due_date.unwrap()), "2026-06-20");
    // Empty string clears it.
    store
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                due: Some("".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    assert!(store
        .card_by_key(&b.id, "KB-1")
        .unwrap()
        .unwrap()
        .due_date
        .is_none());
}

#[test]
fn agent_handoff_can_be_claimed_by_recipient_family() {
    let mut store = Store::open_in_memory().unwrap();
    let agent = store.register_agent("claude", None, None, None).unwrap();
    let handoff = store
        .create_handoff(&HandoffDraft {
            from_agent: "codex#sender".into(),
            to_agent: "claude".into(),
            board_id: None,
            card_key: None,
            subject: "continue work".into(),
            body: "pick up the next card".into(),
        })
        .unwrap();

    let claimed = store
        .claim_handoff(
            &handoff.id,
            &agent.registration.assigned_identity,
            Some(&agent.claim_token),
            Some(5),
        )
        .unwrap();

    assert_eq!(claimed.status, "claimed");
    assert_eq!(
        claimed.claimed_by.as_deref(),
        Some(agent.registration.assigned_identity.as_str())
    );

    let completed = store
        .update_handoff_status(
            &handoff.id,
            &agent.registration.assigned_identity,
            Some(&agent.claim_token),
            &HandoffStatusPatch {
                status: "completed".into(),
                note: None,
            },
        )
        .unwrap();
    assert_eq!(completed.status, "completed");
    assert!(completed.completed_at.is_some());
}

#[test]
fn agent_handoff_rejects_wrong_recipient() {
    let mut store = Store::open_in_memory().unwrap();
    let agent = store.register_agent("codex", None, None, None).unwrap();
    let handoff = store
        .create_handoff(&HandoffDraft {
            from_agent: "sender".into(),
            to_agent: "claude".into(),
            board_id: None,
            card_key: None,
            subject: "wrong inbox".into(),
            body: "nope".into(),
        })
        .unwrap();

    let err = store
        .claim_handoff(
            &handoff.id,
            &agent.registration.assigned_identity,
            Some(&agent.claim_token),
            None,
        )
        .unwrap_err();
    assert!(err.to_string().contains("is addressed to 'claude'"));
}

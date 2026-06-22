use crate::common::{temp_db, TempDb};
use kanban_core::{BoardColumnTemplate, CardPatch, Store};
use rusqlite::Connection;

#[test]
fn card_agent_fields_update_and_clear() {
    let db = TempDb(temp_db("agent-fields"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "agent task", "", "t").unwrap();

    let patched = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                next_action: Some("run cargo test".into()),
                blocked_reason: Some("waiting for schema decision".into()),
                acceptance_criteria: Some("MCP can update and read fields".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    assert_eq!(patched.next_action.as_deref(), Some("run cargo test"));
    assert_eq!(
        patched.blocked_reason.as_deref(),
        Some("waiting for schema decision")
    );
    assert_eq!(
        patched.acceptance_criteria.as_deref(),
        Some("MCP can update and read fields")
    );
    let json = s.export_json(&b.id).unwrap();
    assert!(json.contains("\"next_action\": \"run cargo test\""));
    assert!(json.contains("\"blocked_reason\": \"waiting for schema decision\""));
    assert!(json.contains("\"acceptance_criteria\": \"MCP can update and read fields\""));

    let cleared = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                blocked_reason: Some("".into()),
                next_action: Some("  write release note  ".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    assert!(cleared.blocked_reason.is_none());
    assert_eq!(cleared.next_action.as_deref(), Some("write release note"));
}

#[test]
fn board_agent_context_persists_and_exports() {
    let db = TempDb(temp_db("board-agent-context"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s
        .create_board("Repo Work", BoardColumnTemplate::Workflow)
        .unwrap();

    s.update_board_agent_context(
        &b.id,
        Some("Run cargo test -p kanban-core before closing cards."),
    )
    .unwrap();

    let json = s.export_json(&b.id).unwrap();
    assert!(
        json.contains("\"agent_context\": \"Run cargo test -p kanban-core before closing cards.\"")
    );
    let md = s.export_markdown(&b.id).unwrap();
    assert!(md.contains("## Board Agent Context"));
    assert!(md.contains("Run cargo test -p kanban-core before closing cards."));

    drop(s);
    let s = Store::open(&db.0).unwrap();
    let persisted = s.board_by_slug("repo-work").unwrap().unwrap();
    assert_eq!(
        persisted.agent_context.as_deref(),
        Some("Run cargo test -p kanban-core before closing cards.")
    );
}

#[test]
fn card_agent_execution_metadata_update_clear_validate_and_persist() {
    let db = TempDb(temp_db("agent-execution-metadata"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "agent execution", "", "t")
        .unwrap();

    let patched = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                agent_weight: Some(Some(3)),
                agent_effort: Some("high-reasoning".into()),
                suggested_model: Some("gpt-5".into()),
                expected_tokens: Some(Some(12_000)),
                human_intervention: Some("review".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    assert_eq!(patched.agent_weight, Some(3));
    assert_eq!(patched.agent_effort.as_deref(), Some("high-reasoning"));
    assert_eq!(patched.suggested_model.as_deref(), Some("gpt-5"));
    assert_eq!(patched.expected_tokens, Some(12_000));
    assert_eq!(patched.human_intervention.as_deref(), Some("review"));

    let json = s.export_json(&b.id).unwrap();
    assert!(json.contains("\"agent_weight\": 3"));
    assert!(json.contains("\"agent_effort\": \"high-reasoning\""));
    assert!(json.contains("\"suggested_model\": \"gpt-5\""));
    assert!(json.contains("\"expected_tokens\": 12000"));
    assert!(json.contains("\"human_intervention\": \"review\""));

    drop(s);
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    let persisted = s.card_by_key(&b.id, "KB-1").unwrap().unwrap();
    assert_eq!(persisted.agent_weight, Some(3));
    assert_eq!(persisted.expected_tokens, Some(12_000));

    assert!(s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                agent_weight: Some(Some(0)),
                ..Default::default()
            },
            "t",
        )
        .is_err());
    assert!(s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                expected_tokens: Some(Some(0)),
                ..Default::default()
            },
            "t",
        )
        .is_err());
    assert!(s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                human_intervention: Some("maybe".into()),
                ..Default::default()
            },
            "t",
        )
        .is_err());

    let cleared = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                agent_weight: Some(None),
                agent_effort: Some(" ".into()),
                suggested_model: Some("".into()),
                expected_tokens: Some(None),
                human_intervention: Some("".into()),
                ..Default::default()
            },
            "t",
        )
        .unwrap();
    assert_eq!(cleared.agent_weight, None);
    assert_eq!(cleared.agent_effort, None);
    assert_eq!(cleared.suggested_model, None);
    assert_eq!(cleared.expected_tokens, None);
    assert_eq!(cleared.human_intervention, None);
}

#[test]
fn agent_registration_token_lifetime_defaults_and_caps() {
    let db = TempDb(temp_db("agent-token-lifetime"));
    let mut s = Store::open(&db.0).unwrap();

    let defaulted = s.register_agent("codex", None, None, None).unwrap();
    let default_lifetime = defaulted.registration.expires_at - defaulted.registration.registered_at;
    assert!(
        (119 * 60_000..=120 * 60_000).contains(&default_lifetime),
        "default lifetime was {default_lifetime}"
    );

    let capped = s
        .register_agent("claude", None, None, Some(7 * 24 * 60))
        .unwrap();
    let capped_lifetime = capped.registration.expires_at - capped.registration.registered_at;
    assert!(
        (23 * 60 * 60_000..=24 * 60 * 60_000).contains(&capped_lifetime),
        "capped lifetime was {capped_lifetime}"
    );
}

#[test]
fn card_claims_conflict_expire_and_release() {
    let db = TempDb(temp_db("card-claims"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "claim task", "", "t").unwrap();
    let codex = s.register_agent("codex", None, None, None).unwrap();
    let claude = s.register_agent("claude", None, None, None).unwrap();

    let claimed = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                claim: Some(codex.registration.assigned_identity.clone()),
                claim_token: Some(codex.claim_token.clone()),
                lease_minutes: Some(30),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert_eq!(
        claimed.claimed_by.as_deref(),
        Some(codex.registration.assigned_identity.as_str())
    );
    assert!(claimed.claimed_at.is_some());
    assert!(claimed.lease_expires_at.unwrap() > claimed.claimed_at.unwrap());

    assert!(s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                claim: Some(claude.registration.assigned_identity.clone()),
                claim_token: Some(claude.claim_token.clone()),
                ..Default::default()
            },
            "agent",
        )
        .is_err());

    let renewed = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                claim: Some(codex.registration.assigned_identity.clone()),
                claim_token: Some(codex.claim_token.clone()),
                lease_minutes: Some(120),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert_eq!(
        renewed.claimed_by.as_deref(),
        Some(codex.registration.assigned_identity.as_str())
    );

    {
        let conn = Connection::open(&db.0).unwrap();
        conn.execute(
            "UPDATE cards SET lease_expires_at = 1 WHERE key_text = 'KB-1'",
            [],
        )
        .unwrap();
    }
    let taken_over = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                claim: Some(claude.registration.assigned_identity.clone()),
                claim_token: Some(claude.claim_token.clone()),
                lease_minutes: Some(10),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert_eq!(
        taken_over.claimed_by.as_deref(),
        Some(claude.registration.assigned_identity.as_str())
    );

    let released = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                release_claim: Some(true),
                claim_token: Some(claude.claim_token),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert!(released.claimed_by.is_none());
    assert!(released.claimed_at.is_none());
    assert!(released.lease_expires_at.is_none());
}

#[test]
fn archiving_card_clears_claim() {
    let db = TempDb(temp_db("archive-clears-claim"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    let agent = s.register_agent("codex", None, None, None).unwrap();
    s.create_card(&b.id, None, "finish task", "", "t").unwrap();

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            claim: Some(agent.registration.assigned_identity),
            claim_token: Some(agent.claim_token),
            ..Default::default()
        },
        "agent",
    )
    .unwrap();

    let archived = s
        .update_card(
            &b.id,
            "KB-1",
            &CardPatch {
                archived: Some(true),
                agent_state: Some("done".into()),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    assert_eq!(archived.agent_state, "done");
    assert!(archived.archived_at.is_some());
    assert!(archived.claimed_by.is_none());
    assert!(archived.claimed_at.is_none());
    assert!(archived.lease_expires_at.is_none());
}

#[test]
fn card_search_uses_fts_index_and_stays_in_sync() {
    let db = TempDb(temp_db("card-search"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "fix parser", "spreadsheet import", "t")
        .unwrap();
    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            add_labels: Some(vec!["regression".into()]),
            next_action: Some("write fts coverage".into()),
            ..Default::default()
        },
        "t",
    )
    .unwrap();

    assert_eq!(s.search_cards(&b.id, "parser").unwrap()[0].key, "KB-1");
    assert_eq!(s.search_cards(&b.id, "regression").unwrap()[0].key, "KB-1");
    assert_eq!(
        s.search_cards(&b.id, "fts coverage").unwrap()[0].key,
        "KB-1"
    );

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            remove_labels: Some(vec!["regression".into()]),
            ..Default::default()
        },
        "t",
    )
    .unwrap();
    assert!(s.search_cards(&b.id, "regression").unwrap().is_empty());

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            archived: Some(true),
            ..Default::default()
        },
        "t",
    )
    .unwrap();
    assert!(s.search_cards(&b.id, "parser").unwrap().is_empty());
}

#[test]
fn move_card_between_boards_with_update_card() {
    let db = TempDb(temp_db("move-boards"));
    let mut s = Store::open(&db.0).unwrap();
    let backlog = s.ensure_default_board().unwrap();
    let work = s
        .create_board("Work", BoardColumnTemplate::Planning)
        .unwrap();

    let card = s
        .create_card(&backlog.id, None, "handoff", "", "test")
        .unwrap();
    assert_eq!(card.key, "KB-1");

    let moved = s
        .update_card(
            &backlog.id,
            &card.key,
            &CardPatch {
                move_to_board: Some(work.id.clone()),
                ..Default::default()
            },
            "test",
        )
        .unwrap();
    assert_eq!(moved.board_id, work.id);
    assert_ne!(moved.key, card.key);
    assert!(s.card_by_key(&backlog.id, &card.key).unwrap().is_none());

    let work_cols = s.columns(&work.id).unwrap();
    let work_backlog = work_cols
        .into_iter()
        .find(|c| c.name == "Backlog")
        .expect("backlog column on work");
    let current = s.card_by_key(&work.id, &moved.key).unwrap().unwrap();
    assert_eq!(current.id, card.id);
    assert_eq!(current.column_id, work_backlog.id);
    assert_eq!(s.cards(&backlog.id).unwrap().len(), 0);
    assert_eq!(s.cards(&work.id).unwrap().len(), 1);

    let activity = s.card_activity(&current.id, 10).unwrap();
    let move_log = activity
        .iter()
        .find(|log| log.action == "move_board")
        .expect("move_board activity");
    let payload: serde_json::Value = serde_json::from_str(&move_log.payload_json).unwrap();
    assert_eq!(payload["old_key"], "KB-1");
    assert_eq!(payload["new_key"], moved.key);
    assert_eq!(payload["source_board"]["slug"], "backlog");
    assert_eq!(payload["destination_board"]["slug"], "work");
}

use crate::common::{temp_db, TempDb};
use kanterm_core::{BoardColumnTemplate, CardPatch, Store};

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
        Some("Run cargo test -p kanterm-core before closing cards."),
    )
    .unwrap();

    let json = s.export_json(&b.id).unwrap();
    assert!(json
        .contains("\"agent_context\": \"Run cargo test -p kanterm-core before closing cards.\""));
    let md = s.export_markdown(&b.id).unwrap();
    assert!(md.contains("## Board Agent Context"));
    assert!(md.contains("Run cargo test -p kanterm-core before closing cards."));

    drop(s);
    let s = Store::open(&db.0).unwrap();
    let persisted = s.board_by_slug("repo-work").unwrap().unwrap();
    assert_eq!(
        persisted.agent_context.as_deref(),
        Some("Run cargo test -p kanterm-core before closing cards.")
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

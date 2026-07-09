use crate::{CardPatch, Store};

#[test]
fn blank_claim_releases_existing_claim() {
    let mut store = Store::open_in_memory().unwrap();
    let board = store.ensure_default_board().unwrap();
    let card = store
        .create_card(&board.id, None, "claim me", "", "test")
        .unwrap();
    let agent = store.register_agent("agent-a", None, None, None).unwrap();

    store
        .update_card(
            &board.id,
            &card.key,
            &CardPatch {
                claim: Some(agent.registration.assigned_identity.clone()),
                claim_token: Some(agent.claim_token.clone()),
                ..Default::default()
            },
            "test",
        )
        .unwrap();
    let released = store
        .update_card(
            &board.id,
            &card.key,
            &CardPatch {
                claim: Some(" ".to_string()),
                claim_token: Some(agent.claim_token),
                ..Default::default()
            },
            "test",
        )
        .unwrap();

    assert_eq!(released.claimed_by, None);
    assert_eq!(released.claimed_at, None);
    assert_eq!(released.lease_expires_at, None);
}

#[test]
fn blank_workflow_fields_clear_optional_values() {
    let mut store = Store::open_in_memory().unwrap();
    let board = store.ensure_default_board().unwrap();
    let card = store
        .create_card(&board.id, None, "workflow", "", "test")
        .unwrap();

    store
        .update_card(
            &board.id,
            &card.key,
            &CardPatch {
                next_action: Some("Run tests".to_string()),
                blocked_reason: Some("Waiting".to_string()),
                acceptance_criteria: Some("All green".to_string()),
                ..Default::default()
            },
            "test",
        )
        .unwrap();
    let cleared = store
        .update_card(
            &board.id,
            &card.key,
            &CardPatch {
                next_action: Some(" ".to_string()),
                blocked_reason: Some("".to_string()),
                acceptance_criteria: Some(" ".to_string()),
                ..Default::default()
            },
            "test",
        )
        .unwrap();

    assert_eq!(cleared.next_action, None);
    assert_eq!(cleared.blocked_reason, None);
    assert_eq!(cleared.acceptance_criteria, None);
}

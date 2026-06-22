use crate::common::{temp_db, TempDb};
use kanban_core::{CardPatch, Store};
use rusqlite::Connection;

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

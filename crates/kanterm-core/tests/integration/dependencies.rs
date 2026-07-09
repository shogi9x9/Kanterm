use crate::common::{temp_db, TempDb};
use kanterm_core::{CardCreateDraft, CardPatch, Store};

#[test]
fn dependencies_validate_graph_and_drive_readiness() {
    let db = TempDb(temp_db("dependencies-readiness"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    for title in ["A", "B", "C", "D", "E"] {
        s.create_card(&b.id, None, title, "", "t").unwrap();
    }

    let b_deps = s
        .set_card_dependencies(&b.id, "KB-2", &["KB-1".to_string()], "agent")
        .unwrap();
    assert_eq!(b_deps[0].downstream_key, "KB-2");
    assert_eq!(b_deps[0].upstream_key, "KB-1");
    s.set_card_dependencies(&b.id, "KB-3", &["KB-1".to_string()], "agent")
        .unwrap();
    s.set_card_dependencies(&b.id, "KB-4", &["KB-1".to_string()], "agent")
        .unwrap();
    s.set_card_dependencies(
        &b.id,
        "KB-5",
        &["KB-2".to_string(), "KB-3".to_string(), "KB-4".to_string()],
        "agent",
    )
    .unwrap();

    let all = s.card_dependencies(&b.id).unwrap();
    assert_eq!(all.len(), 6);
    assert!(s.card_readiness(&b.id, "KB-1").unwrap().ready);
    let blocked_b = s.card_readiness(&b.id, "KB-2").unwrap();
    assert!(!blocked_b.ready);
    assert_eq!(blocked_b.blocked_by[0].key, "KB-1");

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            agent_state: Some("done".into()),
            archived: Some(true),
            ..Default::default()
        },
        "agent",
    )
    .unwrap();
    assert!(s.card_readiness(&b.id, "KB-2").unwrap().ready);
    assert!(s.card_readiness(&b.id, "KB-3").unwrap().ready);
    assert!(s.card_readiness(&b.id, "KB-4").unwrap().ready);
    assert!(!s.card_readiness(&b.id, "KB-5").unwrap().ready);

    for key in ["KB-2", "KB-3", "KB-4"] {
        s.update_card(
            &b.id,
            key,
            &CardPatch {
                agent_state: Some("done".into()),
                ..Default::default()
            },
            "agent",
        )
        .unwrap();
    }
    assert!(s.card_readiness(&b.id, "KB-5").unwrap().ready);

    let cycle = s.set_card_dependencies(&b.id, "KB-1", &["KB-5".to_string()], "agent");
    assert!(cycle.unwrap_err().to_string().contains("cycle"));

    let missing = s.set_card_dependencies(&b.id, "KB-5", &["KB-999".to_string()], "agent");
    assert!(missing.unwrap_err().to_string().contains("missing card"));
}

#[test]
fn create_cards_from_plan_rolls_back_dependency_failures() {
    let db = TempDb(temp_db("plan-import-rollback"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "existing upstream", "", "t")
        .unwrap();

    let err = s
        .create_cards_from_plan(
            &b.id,
            &[
                CardCreateDraft {
                    alias: Some("A".into()),
                    title: "A".into(),
                    depends_on: vec!["B".into()],
                    ..Default::default()
                },
                CardCreateDraft {
                    alias: Some("B".into()),
                    title: "B".into(),
                    depends_on: vec!["KB-1".into()],
                    ..Default::default()
                },
            ],
            "agent",
        )
        .unwrap();
    assert_eq!(err[0].key, "KB-2");
    assert_eq!(err[1].key, "KB-3");

    let cycle = s
        .create_cards_from_plan(
            &b.id,
            &[CardCreateDraft {
                alias: Some("C".into()),
                title: "C".into(),
                depends_on: vec!["KB-4".into()],
                ..Default::default()
            }],
            "agent",
        )
        .unwrap_err();
    assert!(cycle.to_string().contains("cannot depend on itself"));

    let cards = s.cards(&b.id).unwrap();
    assert_eq!(cards.len(), 3);
    assert!(s.card_by_key(&b.id, "KB-4").unwrap().is_none());
}

#[test]
fn create_cards_from_plan_rejects_alias_colliding_with_existing_key() {
    let db = TempDb(temp_db("plan-import-alias-key-collision"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "existing", "", "t").unwrap();

    let err = s
        .create_cards_from_plan(
            &b.id,
            &[CardCreateDraft {
                alias: Some("KB-1".into()),
                title: "new work".into(),
                ..Default::default()
            }],
            "agent",
        )
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("alias conflicts with existing card key 'KB-1'"));
    assert_eq!(s.cards(&b.id).unwrap().len(), 1);
    assert!(s.card_by_key(&b.id, "KB-2").unwrap().is_none());
}

#[test]
fn archived_unfinished_dependency_blocks_downstream() {
    let db = TempDb(temp_db("dependencies-archived-unfinished"));
    let mut s = Store::open(&db.0).unwrap();
    let b = s.ensure_default_board().unwrap();
    s.create_card(&b.id, None, "upstream", "", "t").unwrap();
    s.create_card(&b.id, None, "downstream", "", "t").unwrap();
    s.set_card_dependencies(&b.id, "KB-2", &["KB-1".to_string()], "agent")
        .unwrap();

    s.update_card(
        &b.id,
        "KB-1",
        &CardPatch {
            archived: Some(true),
            ..Default::default()
        },
        "agent",
    )
    .unwrap();

    let readiness = s.card_readiness(&b.id, "KB-2").unwrap();
    assert!(!readiness.ready);
    assert_eq!(readiness.blocked_by[0].key, "KB-1");
    assert!(readiness.blocked_by[0].archived_at.is_some());
}

use super::resume::delta_size;
use crate::{
    AgentWorkPacket, BoardColumnTemplate, CardPatch, HandoffDraft, Store,
    AGENT_WORK_PACKET_VERSION, MAX_RESUME_DELTA_CHARS,
};

#[test]
fn attempts_retain_exact_packet_and_stable_digest() {
    let mut store = Store::open_in_memory().unwrap();
    let handoff = store
        .create_handoff(&HandoffDraft {
            from_agent: "sender".into(),
            to_agent: "receiver".into(),
            board_id: None,
            card_key: None,
            subject: "work".into(),
            body: "do work".into(),
        })
        .unwrap();
    let packet = "kanterm-agent-work-packet/v1\nprofile: execute\n";

    let started = store
        .start_agent_task_attempt(
            &handoff.id,
            "command",
            AGENT_WORK_PACKET_VERSION,
            "execute",
            packet,
        )
        .unwrap();
    let finished = store
        .finish_agent_task_attempt(&started.id, "agent_succeeded", Some("ok"))
        .unwrap();

    assert_eq!(finished.packet_text, packet);
    assert_eq!(finished.packet_sha256.len(), 64);
    assert_eq!(finished.status, "agent_succeeded");
    assert_eq!(finished.agent_output.as_deref(), Some("ok"));
    assert_eq!(
        store.agent_task_attempts(&handoff.id).unwrap(),
        vec![finished]
    );
}

#[test]
fn retries_append_attempts_instead_of_overwriting_history() {
    let mut store = Store::open_in_memory().unwrap();
    let handoff = store
        .create_handoff(&HandoffDraft {
            from_agent: "sender".into(),
            to_agent: "receiver".into(),
            board_id: None,
            card_key: None,
            subject: "retry".into(),
            body: "retry work".into(),
        })
        .unwrap();

    let first = store
        .start_agent_task_attempt(
            &handoff.id,
            "command",
            AGENT_WORK_PACKET_VERSION,
            "execute",
            "first packet",
        )
        .unwrap();
    store
        .finish_agent_task_attempt(&first.id, "agent_failed", Some("failed"))
        .unwrap();
    let second = store
        .start_agent_task_attempt(
            &handoff.id,
            "command",
            AGENT_WORK_PACKET_VERSION,
            "resume",
            "second packet",
        )
        .unwrap();

    let attempts = store.agent_task_attempts(&handoff.id).unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].attempt_no, 1);
    assert_eq!(attempts[0].status, "agent_failed");
    assert_eq!(attempts[1].attempt_no, 2);
    assert_eq!(attempts[1].id, second.id);
    assert_eq!(attempts[1].packet_profile, "resume");
}

#[test]
fn resume_delta_keeps_latest_bounded_history_and_original_acceptance() {
    let mut store = Store::open_in_memory().unwrap();
    let board = store
        .create_board("Resume", BoardColumnTemplate::Workflow)
        .unwrap();
    let card = store
        .create_card(&board.id, None, "Resume work", "body", "test")
        .unwrap();
    store
        .update_card(
            &board.id,
            &card.key,
            &CardPatch {
                next_action: Some("Fix the remaining failure.".into()),
                acceptance_criteria: Some("The original acceptance stays visible.".into()),
                ..Default::default()
            },
            "test",
        )
        .unwrap();
    let handoff = store
        .create_handoff(&HandoffDraft {
            from_agent: "sender".into(),
            to_agent: "receiver".into(),
            board_id: Some(board.id.clone()),
            card_key: Some(card.key.clone()),
            subject: "resume".into(),
            body: "continue".into(),
        })
        .unwrap();
    for attempt_no in 1..=4 {
        let attempt = store
            .start_agent_task_attempt(
                &handoff.id,
                "command",
                AGENT_WORK_PACKET_VERSION,
                if attempt_no == 1 { "execute" } else { "resume" },
                &format!("packet {attempt_no}"),
            )
            .unwrap();
        store
            .finish_agent_task_attempt(
                &attempt.id,
                "agent_failed",
                Some(&format!("failure {attempt_no} {}", "x".repeat(2_000))),
            )
            .unwrap();
    }
    for note_no in 1..=6 {
        store
            .record_execution_note(
                &board.id,
                &card.key,
                &format!("note {note_no} {}", "y".repeat(2_000)),
                "test",
            )
            .unwrap();
    }

    let delta = store
        .agent_work_packet_resume_delta(&handoff.id, &board.slug, &card.key)
        .unwrap();
    let snapshot = store
        .execution_prompt_snapshot(&board.slug, &card.key)
        .unwrap();
    let rendered = AgentWorkPacket::resume_with_delta(&snapshot, &delta, &handoff.body)
        .render()
        .unwrap();

    assert_eq!(delta.prior_attempts.len(), 3);
    assert_eq!(delta.prior_attempts[0].attempt_no, 2);
    assert_eq!(delta.prior_attempts[2].attempt_no, 4);
    assert_eq!(delta.execution_notes.len(), 5);
    assert!(delta.execution_notes[0].starts_with("note 2"));
    assert!(delta.execution_notes[4].starts_with("note 6"));
    assert!(delta_size(&delta) <= MAX_RESUME_DELTA_CHARS);
    assert!(rendered.starts_with("kanterm-agent-work-packet/v1\nprofile: resume\n"));
    assert!(rendered.contains("The original acceptance stays visible."));
    assert!(rendered.contains("## Resume delta"));
    assert_eq!(
        rendered,
        AgentWorkPacket::resume_with_delta(&snapshot, &delta, &handoff.body)
            .render()
            .unwrap()
    );
}

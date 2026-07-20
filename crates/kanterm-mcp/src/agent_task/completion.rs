use anyhow::{anyhow, Result};
use kanterm_core::{now_ms, CardPatch, Store};

use super::args::AgentTaskArgs;
use super::command::VerificationProcess;
use crate::render::complete_note_body;

pub(super) struct CompletionOutcome {
    pub(super) state: &'static str,
    pub(super) completed: bool,
    pub(super) verification_summary: String,
}

pub(super) fn apply_completion_policy(
    store: &mut Store,
    args: &AgentTaskArgs,
    agent: &str,
    output: &str,
    verification: Option<&VerificationProcess>,
) -> Result<CompletionOutcome> {
    let board = store.board_by_id_or_slug(&args.board)?;
    let card = store
        .card_by_key(&board.id, &args.card)?
        .ok_or_else(|| anyhow!("no card '{}'", args.card))?;

    let Some(verification) = verification else {
        let summary = "agent process succeeded; no verification command was configured";
        store.update_card(
            &board.id,
            &args.card,
            &CardPatch {
                agent_state: Some("verification_pending".into()),
                handoff_note: Some(summary.into()),
                ..Default::default()
            },
            agent,
        )?;
        return Ok(CompletionOutcome {
            state: "verification_pending",
            completed: false,
            verification_summary: summary.into(),
        });
    };

    let status = if verification.passed {
        "passed"
    } else {
        "failed"
    };
    let verification_json = serde_json::json!({
        "command": verification.command,
        "status": status,
        "summary": verification.summary,
        "timestamp": now_ms(),
    })
    .to_string();

    if !verification.passed {
        let blocked_reason = format!("verification failed: {}", verification.summary);
        store.update_card(
            &board.id,
            &args.card,
            &CardPatch {
                agent_state: Some("verification_failed".into()),
                blocked_reason: Some(blocked_reason.clone()),
                handoff_note: Some("Agent process succeeded, but verification failed.".into()),
                last_verification: Some(verification_json),
                ..Default::default()
            },
            agent,
        )?;
        return Ok(CompletionOutcome {
            state: "verification_failed",
            completed: false,
            verification_summary: blocked_reason,
        });
    }

    let note = args.complete_note.clone().unwrap_or_else(|| {
        format!(
            "verified agent task completed by {agent}: {}",
            output.trim()
        )
    });
    let patch = CardPatch {
        body: Some(complete_note_body(&card.body, note.trim())),
        agent_state: Some("done".into()),
        archived: Some(true),
        next_action: Some(String::new()),
        blocked_reason: Some(String::new()),
        handoff_note: Some(String::new()),
        last_verification: Some(verification_json),
        ..Default::default()
    };
    store.update_card(&board.id, &args.card, &patch, agent)?;
    Ok(CompletionOutcome {
        state: "completed",
        completed: true,
        verification_summary: verification.summary.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn setup() -> (Store, AgentTaskArgs, String) {
        let mut store = Store::open_in_memory().unwrap();
        let board = store
            .create_board("Verification", kanterm_core::BoardColumnTemplate::Workflow)
            .unwrap();
        let card = store
            .create_card(&board.id, None, "Verify me", "body", "test")
            .unwrap();
        let args = AgentTaskArgs {
            for_agent: "agent".into(),
            claim_token: "token".into(),
            targets: PathBuf::from("targets.yaml"),
            target: "command".into(),
            board: board.slug,
            card: card.key.clone(),
            lease_minutes: None,
            complete_note: None,
            verification_command: None,
            verification_args: Vec::new(),
            workflow: None,
            workflow_step: None,
            workflow_targets: None,
            from_agent: None,
        };
        (store, args, card.key)
    }

    #[test]
    fn process_success_without_verification_stays_pending() {
        let (mut store, args, key) = setup();

        let outcome = apply_completion_policy(&mut store, &args, "agent", "output", None).unwrap();
        let board = store.board_by_slug(&args.board).unwrap().unwrap();
        let card = store.card_by_key(&board.id, &key).unwrap().unwrap();

        assert_eq!(outcome.state, "verification_pending");
        assert!(!outcome.completed);
        assert_eq!(card.agent_state, "verification_pending");
        assert!(card.archived_at.is_none());
    }

    #[test]
    fn failed_verification_keeps_card_resumable_with_evidence() {
        let (mut store, args, key) = setup();
        let verification = VerificationProcess {
            command: "cargo test".into(),
            passed: false,
            summary: "exited with 1".into(),
        };

        let outcome =
            apply_completion_policy(&mut store, &args, "agent", "output", Some(&verification))
                .unwrap();
        let board = store.board_by_slug(&args.board).unwrap().unwrap();
        let card = store.card_by_key(&board.id, &key).unwrap().unwrap();

        assert_eq!(outcome.state, "verification_failed");
        assert!(!outcome.completed);
        assert!(card.blocked_reason.is_some());
        assert!(card
            .last_verification
            .as_deref()
            .unwrap()
            .contains("failed"));
        assert!(card.archived_at.is_none());
    }

    #[test]
    fn passed_verification_completes_and_archives_card() {
        let (mut store, args, key) = setup();
        let verification = VerificationProcess {
            command: "cargo test".into(),
            passed: true,
            summary: "exited with 0".into(),
        };

        let outcome =
            apply_completion_policy(&mut store, &args, "agent", "output", Some(&verification))
                .unwrap();
        let board = store.board_by_slug(&args.board).unwrap().unwrap();
        let card = store.card_by_key(&board.id, &key).unwrap().unwrap();

        assert_eq!(outcome.state, "completed");
        assert!(outcome.completed);
        assert_eq!(card.agent_state, "done");
        assert!(card.archived_at.is_some());
        assert!(card
            .last_verification
            .as_deref()
            .unwrap()
            .contains("passed"));
    }
}

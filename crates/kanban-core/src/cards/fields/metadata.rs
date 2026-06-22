use anyhow::{anyhow, Result};
use rusqlite::{params, Transaction};

use crate::text::trimmed_optional;
use crate::{CardPatch, HumanIntervention};

pub(in crate::cards) fn apply_execution_metadata(
    tx: &Transaction<'_>,
    card_id: &str,
    patch: &CardPatch,
) -> Result<()> {
    if let Some(agent_weight) = patch.agent_weight {
        if let Some(weight) = agent_weight {
            if !(1..=5).contains(&weight) {
                return Err(anyhow!("agent_weight must be between 1 and 5"));
            }
        }
        tx.execute(
            "UPDATE cards SET agent_weight = ?1 WHERE id = ?2",
            params![agent_weight, card_id],
        )?;
    }
    if let Some(agent_effort) = &patch.agent_effort {
        tx.execute(
            "UPDATE cards SET agent_effort = ?1 WHERE id = ?2",
            params![trimmed_optional(agent_effort), card_id],
        )?;
    }
    if let Some(suggested_model) = &patch.suggested_model {
        tx.execute(
            "UPDATE cards SET suggested_model = ?1 WHERE id = ?2",
            params![trimmed_optional(suggested_model), card_id],
        )?;
    }
    if let Some(expected_tokens) = patch.expected_tokens {
        if let Some(tokens) = expected_tokens {
            if tokens <= 0 {
                return Err(anyhow!("expected_tokens must be positive"));
            }
        }
        tx.execute(
            "UPDATE cards SET expected_tokens = ?1 WHERE id = ?2",
            params![expected_tokens, card_id],
        )?;
    }
    if let Some(human_intervention) = &patch.human_intervention {
        HumanIntervention::parse(human_intervention).map_err(|message| anyhow!(message))?;
        tx.execute(
            "UPDATE cards SET human_intervention = ?1 WHERE id = ?2",
            params![trimmed_optional(human_intervention), card_id],
        )?;
    }
    Ok(())
}

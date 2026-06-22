use crate::{Card, CardReadiness, DependencyBlocker};

pub(super) fn readiness_for_card(card: &Card, upstream: Vec<Card>) -> CardReadiness {
    let blocked_by = upstream
        .into_iter()
        .filter(|c| c.agent_state != "done")
        .map(|c| DependencyBlocker {
            key: c.key,
            title: c.title,
            agent_state: c.agent_state,
            archived_at: c.archived_at,
        })
        .collect::<Vec<_>>();
    let closed = card.is_closed();
    CardReadiness {
        card_key: card.key.clone(),
        ready: !closed && blocked_by.is_empty(),
        closed,
        blocked_by,
    }
}

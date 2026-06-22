use anyhow::{anyhow, Result};
use rusqlite::Transaction;

use crate::{CardDependency, CardReadiness, DependencyStagePlan, Store};

mod cycle;
mod read;
mod readiness;
mod stage;
mod write;

impl Store {
    pub fn card_dependencies(&self, board_id: &str) -> Result<Vec<CardDependency>> {
        read::load_dependencies(&self.conn, board_id)
    }

    pub fn card_upstream_dependencies(
        &self,
        board_id: &str,
        key: &str,
    ) -> Result<Vec<CardDependency>> {
        let card = self
            .card_by_key(board_id, key)?
            .ok_or_else(|| anyhow!("no card with key '{key}'"))?;
        read::load_dependencies_for_downstream(&self.conn, board_id, &card.id)
    }

    pub fn set_card_dependencies(
        &mut self,
        board_id: &str,
        key: &str,
        upstream_keys: &[String],
        actor: &str,
    ) -> Result<Vec<CardDependency>> {
        self.assert_writable()?;
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let downstream_id =
            write::set_card_dependencies_tx(&tx, board_id, key, upstream_keys, actor)?;
        let dependencies = read::load_dependencies_for_downstream(&tx, board_id, &downstream_id)?;
        tx.commit()?;
        Ok(dependencies)
    }

    pub(crate) fn set_card_dependencies_in_tx(
        tx: &Transaction<'_>,
        board_id: &str,
        key: &str,
        upstream_keys: &[String],
        actor: &str,
    ) -> Result<()> {
        write::set_card_dependencies_tx(tx, board_id, key, upstream_keys, actor).map(|_| ())
    }

    pub fn card_readiness(&self, board_id: &str, key: &str) -> Result<CardReadiness> {
        let card = self
            .card_by_key(board_id, key)?
            .ok_or_else(|| anyhow!("no card with key '{key}'"))?;
        let upstream = read::load_upstream_cards_for_card(&self.conn, board_id, &card.id)?;
        Ok(readiness::readiness_for_card(&card, upstream))
    }

    pub fn dependency_stage_plan(&self, board_id: &str) -> Result<DependencyStagePlan> {
        let cards = self.cards(board_id)?;
        let dependencies = self.card_dependencies(board_id)?;
        stage::stage_plan_from_cards(self, board_id, &cards, &dependencies)
    }
}

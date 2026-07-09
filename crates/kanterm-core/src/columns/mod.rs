use anyhow::Result;

use crate::{Column, Store};

mod delete;
mod guard;
mod mutate;
mod read;

impl Store {
    pub fn columns(&self, board_id: &str) -> Result<Vec<Column>> {
        read::columns(&self.conn, board_id)
    }

    /// Append a new column to the end of a board. Names are unique per board.
    pub fn add_column(&mut self, board_id: &str, name: &str) -> Result<Column> {
        mutate::add_column(self, board_id, name)
    }

    /// Rename a column. Fails if the new name collides on the same board.
    pub fn rename_column(&mut self, column_id: &str, new_name: &str) -> Result<()> {
        mutate::rename_column(self, column_id, new_name)
    }

    /// Move a column left (-1) or right (+1) by swapping sort_order with its
    /// neighbour. No-op at the ends.
    pub fn reorder_column(&mut self, board_id: &str, column_id: &str, dir: i32) -> Result<()> {
        mutate::reorder_column(self, board_id, column_id, dir)
    }

    /// Delete a column, relocating its cards (including archived ones) to
    /// `dest_id`. Refuses to delete the last column or move into itself.
    pub fn delete_column(&mut self, board_id: &str, victim_id: &str, dest_id: &str) -> Result<()> {
        delete::delete_column(self, board_id, victim_id, dest_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_column_rejects_blank_name() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();

        let err = store.add_column(&board.id, "   ").unwrap_err().to_string();

        assert!(err.contains("column name must not be empty"));
    }

    #[test]
    fn backlog_board_columns_are_not_mutable() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let column = store.columns(&board.id).unwrap().remove(0);

        let add = store
            .add_column(&board.id, "Today")
            .unwrap_err()
            .to_string();
        assert!(add.contains("must keep exactly one Backlog column"));

        let rename = store
            .rename_column(&column.id, "Inbox")
            .unwrap_err()
            .to_string();
        assert!(rename.contains("must keep exactly one Backlog column"));
    }
}

use crate::app::App;
use kanban_core::Card;

impl App {
    pub(crate) fn column_cards(&self, col: usize) -> Vec<&Card> {
        let col_id = &self.columns[col].id;
        let needle = self.filter.as_deref().map(str::to_lowercase);
        self.cards
            .iter()
            .filter(|c| &c.column_id == col_id)
            .filter(|c| match &needle {
                None => true,
                Some(q) => {
                    c.title.to_lowercase().contains(q)
                        || c.body.to_lowercase().contains(q)
                        || self
                            .labels
                            .get(&c.id)
                            .map(|ls| ls.iter().any(|l| l.name.to_lowercase().contains(q)))
                            .unwrap_or(false)
                }
            })
            .collect()
    }

    pub(crate) fn selected_card(&self) -> Option<&Card> {
        self.column_cards(self.focus)
            .get(self.cursors[self.focus])
            .copied()
    }

    pub(crate) fn card_by_key(&self, key: &str) -> Option<&Card> {
        self.cards.iter().find(|c| c.key == key)
    }

    pub(crate) fn select_key(&mut self, key: &str) {
        for i in 0..self.columns.len() {
            if let Some(pos) = self.column_cards(i).iter().position(|c| c.key == key) {
                self.focus = i;
                self.cursors[i] = pos;
                return;
            }
        }
    }
}

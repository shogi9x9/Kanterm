use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::execution_dashboard::FLOW_GROUPS;
use crate::app::App;
use crate::mode::{ExecutionDashboardView, Mode};

impl App {
    pub(crate) fn on_execution_dashboard_key(&mut self, key: KeyEvent) -> Result<()> {
        let (view, cursor, focus) = match &self.mode {
            Mode::ExecutionDashboard {
                view,
                cursor,
                focus,
            } => (*view, *cursor, *focus),
            _ => return Ok(()),
        };

        match key.code {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('1') => self.open_kanban(),
            KeyCode::Tab => match view {
                ExecutionDashboardView::Flow => self.open_kanban(),
                _ => self.open_execution_dashboard(view.next()),
            },
            KeyCode::BackTab => match view {
                ExecutionDashboardView::List => self.open_kanban(),
                _ => self.open_execution_dashboard(view.previous()),
            },
            KeyCode::Char('2') => self.open_execution_dashboard(ExecutionDashboardView::List),
            KeyCode::Char('3') => self.open_execution_dashboard(ExecutionDashboardView::Timeline),
            KeyCode::Char('4') => self.open_execution_dashboard(ExecutionDashboardView::Flow),
            KeyCode::Char('j') | KeyCode::Down => {
                let len = self.dashboard_view_len(view, focus)?;
                if len > 0 {
                    self.set_dashboard_position(view, (cursor + 1) % len, focus);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let len = self.dashboard_view_len(view, focus)?;
                if len > 0 {
                    self.set_dashboard_position(view, (cursor + len - 1) % len, focus);
                }
            }
            KeyCode::Char('h') | KeyCode::Left => match view {
                ExecutionDashboardView::Timeline => {
                    self.set_dashboard_position(view, cursor, focus.saturating_sub(1));
                }
                ExecutionDashboardView::Flow => {
                    let next = (focus + FLOW_GROUPS.len() - 1) % FLOW_GROUPS.len();
                    self.set_dashboard_position(view, 0, next);
                }
                ExecutionDashboardView::List => {}
            },
            KeyCode::Char('l') | KeyCode::Right => match view {
                ExecutionDashboardView::Timeline => {
                    let (_, max_stages) = self.execution_timeline_items()?;
                    self.set_dashboard_position(
                        view,
                        cursor,
                        (focus + 1).min(max_stages.saturating_sub(1)),
                    );
                }
                ExecutionDashboardView::Flow => {
                    self.set_dashboard_position(view, 0, (focus + 1) % FLOW_GROUPS.len());
                }
                ExecutionDashboardView::List => {}
            },
            KeyCode::Home => self.set_dashboard_position(view, 0, focus),
            KeyCode::End => {
                let len = self.dashboard_view_len(view, focus)?;
                self.set_dashboard_position(view, len.saturating_sub(1), focus);
            }
            KeyCode::Enter => {
                if let Some((board, key)) = self.dashboard_target(view, cursor, focus)? {
                    self.switch_board(board)?;
                    self.select_key(&key);
                    self.detail_return_dashboard = Some((view, cursor, focus));
                    self.mode = Mode::Detail { key, scroll: 0 };
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn open_execution_dashboard(&mut self, view: ExecutionDashboardView) {
        let focus = if view == ExecutionDashboardView::Flow {
            FLOW_GROUPS
                .iter()
                .position(|group| *group == crate::app::execution_dashboard::DashboardGroup::Ready)
                .unwrap_or(0)
        } else {
            0
        };
        self.set_dashboard_position(view, 0, focus);
    }

    fn open_kanban(&mut self) {
        self.detail_return_dashboard = None;
        self.mode = Mode::Normal;
    }

    fn set_dashboard_position(
        &mut self,
        view: ExecutionDashboardView,
        cursor: usize,
        focus: usize,
    ) {
        self.mode = Mode::ExecutionDashboard {
            view,
            cursor,
            focus,
        };
    }

    fn dashboard_view_len(&self, view: ExecutionDashboardView, focus: usize) -> Result<usize> {
        match view {
            ExecutionDashboardView::List => Ok(self.execution_dashboard_items()?.len()),
            ExecutionDashboardView::Timeline => Ok(self.execution_timeline_items()?.0.len()),
            ExecutionDashboardView::Flow => {
                let group = FLOW_GROUPS[focus.min(FLOW_GROUPS.len() - 1)];
                Ok(self.execution_items_for_group(group)?.len())
            }
        }
    }

    fn dashboard_target(
        &self,
        view: ExecutionDashboardView,
        cursor: usize,
        focus: usize,
    ) -> Result<Option<(kanterm_core::Board, String)>> {
        let target = match view {
            ExecutionDashboardView::List => {
                let items = self.execution_dashboard_items()?;
                items
                    .get(cursor.min(items.len().saturating_sub(1)))
                    .map(|item| (item.board.clone(), item.card.key.clone()))
            }
            ExecutionDashboardView::Timeline => {
                let items = self.execution_timeline_items()?.0;
                items
                    .get(cursor.min(items.len().saturating_sub(1)))
                    .map(|entry| (entry.item.board.clone(), entry.item.card.key.clone()))
            }
            ExecutionDashboardView::Flow => {
                let group = FLOW_GROUPS[focus.min(FLOW_GROUPS.len() - 1)];
                let items = self.execution_items_for_group(group)?;
                items
                    .get(cursor.min(items.len().saturating_sub(1)))
                    .map(|item| (item.board.clone(), item.card.key.clone()))
            }
        };
        Ok(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use kanterm_core::Store;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn tabs_cycle_between_kanban_and_execution_views() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let mut app = App::new(store, board).unwrap();

        app.on_execution_dashboard_key(key(KeyCode::BackTab))
            .unwrap();
        assert!(matches!(app.mode, Mode::Normal));

        app.on_normal_key(key(KeyCode::Tab)).unwrap();
        assert!(matches!(
            app.mode,
            Mode::ExecutionDashboard {
                view: ExecutionDashboardView::List,
                ..
            }
        ));

        app.on_execution_dashboard_key(key(KeyCode::Char('3')))
            .unwrap();
        assert!(matches!(
            app.mode,
            Mode::ExecutionDashboard {
                view: ExecutionDashboardView::Timeline,
                ..
            }
        ));

        app.on_execution_dashboard_key(key(KeyCode::Char('1')))
            .unwrap();
        assert!(matches!(app.mode, Mode::Normal));

        app.open_execution_dashboard(ExecutionDashboardView::Flow);
        app.on_execution_dashboard_key(key(KeyCode::Tab)).unwrap();
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn card_detail_returns_to_the_execution_tab_that_opened_it() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let card = store
            .create_card(&board.id, None, "Inspect this card", "body", "test")
            .unwrap();
        let mut app = App::new(store, board).unwrap();

        app.on_execution_dashboard_key(key(KeyCode::Enter)).unwrap();
        assert!(matches!(app.mode, Mode::Detail { ref key, .. } if key == &card.key));

        app.on_detail_key(key(KeyCode::Esc)).unwrap();
        assert!(matches!(
            app.mode,
            Mode::ExecutionDashboard {
                view: ExecutionDashboardView::List,
                ..
            }
        ));
    }

    #[test]
    fn escape_exits_directly_from_an_execution_tab() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let mut app = App::new(store, board).unwrap();

        app.on_execution_dashboard_key(key(KeyCode::Esc)).unwrap();

        assert!(app.should_quit);
        assert!(matches!(app.mode, Mode::ExecutionDashboard { .. }));
    }
}

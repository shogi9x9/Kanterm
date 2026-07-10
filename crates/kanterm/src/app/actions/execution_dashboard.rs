use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::execution_dashboard::FLOW_GROUPS;
use crate::app::App;
use crate::mode::{ExecutionDashboardState, ExecutionDashboardView, Mode};

impl App {
    pub(crate) fn on_execution_dashboard_key(&mut self, key: KeyEvent) -> Result<()> {
        let state = match self.mode {
            Mode::ExecutionDashboard(state) => state,
            _ => return Ok(()),
        };
        let view = state.view;
        let focus = state.focus;

        match key.code {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('1') => self.open_kanban(),
            KeyCode::Tab => self.cycle_execution_tab(view, true),
            KeyCode::BackTab => self.cycle_execution_tab(view, false),
            KeyCode::Char('2') => self.open_execution_dashboard(ExecutionDashboardView::List),
            KeyCode::Char('3') => self.open_execution_dashboard(ExecutionDashboardView::Timeline),
            KeyCode::Char('4') => self.open_execution_dashboard(ExecutionDashboardView::Flow),
            KeyCode::Char('j') | KeyCode::Down => self.move_dashboard_cursor(state, true)?,
            KeyCode::Char('k') | KeyCode::Up => self.move_dashboard_cursor(state, false)?,
            KeyCode::Char('h') | KeyCode::Left => self.move_dashboard_focus(state, false)?,
            KeyCode::Char('l') | KeyCode::Right => self.move_dashboard_focus(state, true)?,
            KeyCode::Home => self.set_dashboard_position(view, 0, focus),
            KeyCode::End => {
                let len = self.dashboard_view_len(view, focus)?;
                self.set_dashboard_position(view, len.saturating_sub(1), focus);
            }
            KeyCode::Enter => self.open_dashboard_card(state)?,
            _ => {}
        }
        Ok(())
    }

    fn cycle_execution_tab(&mut self, view: ExecutionDashboardView, forward: bool) {
        match (view, forward) {
            (ExecutionDashboardView::Flow, true) | (ExecutionDashboardView::List, false) => {
                self.open_kanban()
            }
            (_, true) => self.open_execution_dashboard(view.next()),
            (_, false) => self.open_execution_dashboard(view.previous()),
        }
    }

    fn move_dashboard_cursor(
        &mut self,
        mut state: ExecutionDashboardState,
        forward: bool,
    ) -> Result<()> {
        let len = self.dashboard_view_len(state.view, state.focus)?;
        if len == 0 {
            return Ok(());
        }
        state.cursor = if forward {
            (state.cursor + 1) % len
        } else {
            (state.cursor + len - 1) % len
        };
        self.mode = Mode::ExecutionDashboard(state);
        Ok(())
    }

    fn move_dashboard_focus(
        &mut self,
        mut state: ExecutionDashboardState,
        forward: bool,
    ) -> Result<()> {
        match state.view {
            ExecutionDashboardView::List => return Ok(()),
            ExecutionDashboardView::Timeline => {
                let (_, max_stages) = self.execution_timeline_items()?;
                state.focus = if forward {
                    (state.focus + 1).min(max_stages.saturating_sub(1))
                } else {
                    state.focus.saturating_sub(1)
                };
            }
            ExecutionDashboardView::Flow => {
                state.focus = if forward {
                    (state.focus + 1) % FLOW_GROUPS.len()
                } else {
                    (state.focus + FLOW_GROUPS.len() - 1) % FLOW_GROUPS.len()
                };
                state.cursor = 0;
            }
        }
        self.mode = Mode::ExecutionDashboard(state);
        Ok(())
    }

    fn open_dashboard_card(&mut self, state: ExecutionDashboardState) -> Result<()> {
        if let Some(key) = self.dashboard_target(state.view, state.cursor, state.focus)? {
            self.select_key(&key);
            self.detail_return_dashboard = Some(state);
            self.mode = Mode::Detail { key, scroll: 0 };
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
        self.mode = Mode::ExecutionDashboard(ExecutionDashboardState::new(view, cursor, focus));
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
    ) -> Result<Option<String>> {
        let target = match view {
            ExecutionDashboardView::List => {
                let items = self.execution_dashboard_items()?;
                items
                    .get(cursor.min(items.len().saturating_sub(1)))
                    .map(|item| item.card.key.clone())
            }
            ExecutionDashboardView::Timeline => {
                let items = self.execution_timeline_items()?.0;
                items
                    .get(cursor.min(items.len().saturating_sub(1)))
                    .map(|entry| entry.item.card.key.clone())
            }
            ExecutionDashboardView::Flow => {
                let group = FLOW_GROUPS[focus.min(FLOW_GROUPS.len() - 1)];
                let items = self.execution_items_for_group(group)?;
                items
                    .get(cursor.min(items.len().saturating_sub(1)))
                    .map(|item| item.card.key.clone())
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
            Mode::ExecutionDashboard(ExecutionDashboardState {
                view: ExecutionDashboardView::List,
                ..
            })
        ));

        app.on_execution_dashboard_key(key(KeyCode::Char('3')))
            .unwrap();
        assert!(matches!(
            app.mode,
            Mode::ExecutionDashboard(ExecutionDashboardState {
                view: ExecutionDashboardView::Timeline,
                ..
            })
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
            Mode::ExecutionDashboard(ExecutionDashboardState {
                view: ExecutionDashboardView::List,
                ..
            })
        ));
    }

    #[test]
    fn escape_exits_directly_from_an_execution_tab() {
        let mut store = Store::open_in_memory().unwrap();
        let board = store.ensure_default_board().unwrap();
        let mut app = App::new(store, board).unwrap();

        app.on_execution_dashboard_key(key(KeyCode::Esc)).unwrap();

        assert!(app.should_quit);
        assert!(matches!(app.mode, Mode::ExecutionDashboard(_)));
    }
}

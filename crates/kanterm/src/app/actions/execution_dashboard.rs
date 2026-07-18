use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;
use crate::mode::{
    CardActionBack, ExecutionDashboardState, ExecutionDashboardView, Mode, ViewBack,
};
use kanterm_core::PROTECTED_BOARD_SLUG;

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
            KeyCode::Char('b') => self.open_board_switcher_from_dashboard(state),
            KeyCode::Char('C') => {
                self.open_board_execution_prompt(ViewBack::ExecutionDashboard(state))
            }
            KeyCode::Char('d') => self.prompt_archive_from_dashboard(state)?,
            KeyCode::Char('D') => self.prompt_board_archive_from_dashboard(state),
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
            (ExecutionDashboardView::Timeline, true) | (ExecutionDashboardView::List, false) => {
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
        self.set_dashboard_position(view, 0, 0);
    }

    fn open_board_switcher_from_dashboard(&mut self, state: ExecutionDashboardState) {
        let cursor = self
            .boards
            .iter()
            .position(|board| board.id == self.board.id)
            .unwrap_or(0);
        self.mode = Mode::BoardSwitcher {
            cursor,
            back: ViewBack::ExecutionDashboard(state),
        };
    }

    fn prompt_archive_from_dashboard(&mut self, state: ExecutionDashboardState) -> Result<()> {
        if let Some(key) = self.dashboard_target(state.view, state.cursor, state.focus)? {
            self.prompt_archive_key(
                key,
                CardActionBack::View(ViewBack::ExecutionDashboard(state)),
            );
        }
        Ok(())
    }

    fn prompt_board_archive_from_dashboard(&mut self, state: ExecutionDashboardState) {
        if self.board.slug == PROTECTED_BOARD_SLUG {
            self.status = "cannot archive the Backlog board".into();
        } else {
            self.mode = Mode::BoardArchive {
                board_id: self.board.id.clone(),
                board_name: self.board.name.clone(),
                back: ViewBack::ExecutionDashboard(state),
            };
        }
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

    fn dashboard_view_len(&self, view: ExecutionDashboardView, _focus: usize) -> Result<usize> {
        match view {
            ExecutionDashboardView::List => Ok(self.execution_dashboard_items()?.len()),
            ExecutionDashboardView::Timeline => Ok(self.execution_timeline_items()?.0.len()),
        }
    }

    fn dashboard_target(
        &self,
        view: ExecutionDashboardView,
        cursor: usize,
        _focus: usize,
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
        };
        Ok(target)
    }
}

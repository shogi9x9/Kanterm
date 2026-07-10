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
            KeyCode::Char('W') | KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Tab => self.open_dashboard_view(view.next()),
            KeyCode::BackTab => self.open_dashboard_view(view.previous()),
            KeyCode::Char('1') => self.open_dashboard_view(ExecutionDashboardView::List),
            KeyCode::Char('2') => self.open_dashboard_view(ExecutionDashboardView::Timeline),
            KeyCode::Char('3') => self.open_dashboard_view(ExecutionDashboardView::Flow),
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
                    self.mode = Mode::Detail { key, scroll: 0 };
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn open_dashboard_view(&mut self, view: ExecutionDashboardView) {
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

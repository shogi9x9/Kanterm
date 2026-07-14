use crate::app::App;
use crate::mode::{
    CardActionBack, ExecutionDashboardState, ExecutionDashboardView, Mode, ViewBack,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kanterm_core::{BoardColumnTemplate, Store};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn backlog_app() -> App {
    let mut store = Store::open_in_memory().unwrap();
    let board = store.ensure_default_board().unwrap();
    App::new(store, board).unwrap()
}

#[test]
fn tabs_cycle_between_kanban_and_execution_views() {
    let mut app = backlog_app();

    app.on_execution_dashboard_key(key(KeyCode::BackTab))
        .unwrap();
    assert!(matches!(app.mode, Mode::Normal));

    app.on_normal_key(key(KeyCode::Tab)).unwrap();
    assert_dashboard(&app, ExecutionDashboardView::List);

    app.on_execution_dashboard_key(key(KeyCode::Char('3')))
        .unwrap();
    assert_dashboard(&app, ExecutionDashboardView::Timeline);

    app.on_execution_dashboard_key(key(KeyCode::Char('1')))
        .unwrap();
    assert!(matches!(app.mode, Mode::Normal));

    app.open_execution_dashboard(ExecutionDashboardView::Timeline);
    app.on_execution_dashboard_key(key(KeyCode::Tab)).unwrap();
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn board_switcher_preserves_the_execution_position() {
    let mut app = backlog_app();
    let other = app
        .store
        .create_board("Other", BoardColumnTemplate::Workflow)
        .unwrap();
    let state = ExecutionDashboardState::new(ExecutionDashboardView::List, 3, 2);
    app.mode = Mode::ExecutionDashboard(state);

    app.on_execution_dashboard_key(key(KeyCode::Char('b')))
        .unwrap();
    assert!(matches!(
        app.mode,
        Mode::BoardSwitcher {
            back: ViewBack::ExecutionDashboard(found),
            ..
        } if found == state
    ));
    app.on_board_switcher_key(key(KeyCode::Down)).unwrap();
    app.on_board_switcher_key(key(KeyCode::Enter)).unwrap();

    assert_eq!(app.board.id, other.id);
    assert!(matches!(app.mode, Mode::ExecutionDashboard(found) if found == state));
}

#[test]
fn card_archive_returns_to_the_execution_view() {
    let mut app = backlog_app();
    let card = app
        .store
        .create_card(&app.board.id, None, "Archive from LIST", "body", "test")
        .unwrap();
    app.reload().unwrap();

    app.on_execution_dashboard_key(key(KeyCode::Char('d')))
        .unwrap();
    assert!(matches!(
        app.mode,
        Mode::ArchiveConfirm {
            back: CardActionBack::View(ViewBack::ExecutionDashboard(ExecutionDashboardState {
                view: ExecutionDashboardView::List,
                ..
            })),
            ..
        }
    ));
    app.on_archive_confirm_key(key(KeyCode::Char('y'))).unwrap();

    assert_dashboard(&app, ExecutionDashboardView::List);
    assert!(app.card_by_key(&card.key).is_none());
}

#[test]
fn board_archive_returns_to_the_execution_view_on_the_next_board() {
    let mut store = Store::open_in_memory().unwrap();
    store.ensure_default_board().unwrap();
    let project = store
        .create_board("Archive Project", BoardColumnTemplate::Workflow)
        .unwrap();
    let mut app = App::new(store, project.clone()).unwrap();

    app.on_execution_dashboard_key(key(KeyCode::Char('D')))
        .unwrap();
    assert!(matches!(
        app.mode,
        Mode::BoardArchive {
            back: ViewBack::ExecutionDashboard(ExecutionDashboardState {
                view: ExecutionDashboardView::List,
                ..
            }),
            ..
        }
    ));
    app.on_board_archive_key(key(KeyCode::Char('y'))).unwrap();

    assert_ne!(app.board.id, project.id);
    assert_dashboard(&app, ExecutionDashboardView::List);
}

#[test]
fn card_detail_returns_to_the_execution_tab_that_opened_it() {
    let mut app = backlog_app();
    let card = app
        .store
        .create_card(&app.board.id, None, "Inspect this card", "body", "test")
        .unwrap();
    app.reload().unwrap();

    app.on_execution_dashboard_key(key(KeyCode::Enter)).unwrap();
    assert!(matches!(app.mode, Mode::Detail { ref key, .. } if key == &card.key));

    app.on_detail_key(key(KeyCode::Esc)).unwrap();
    assert_dashboard(&app, ExecutionDashboardView::List);
}

#[test]
fn escape_exits_directly_from_an_execution_tab() {
    let mut app = backlog_app();

    app.on_execution_dashboard_key(key(KeyCode::Esc)).unwrap();

    assert!(app.should_quit);
    assert!(matches!(app.mode, Mode::ExecutionDashboard(_)));
}

fn assert_dashboard(app: &App, view: ExecutionDashboardView) {
    assert!(matches!(
        app.mode,
        Mode::ExecutionDashboard(ExecutionDashboardState { view: found, .. }) if found == view
    ));
}

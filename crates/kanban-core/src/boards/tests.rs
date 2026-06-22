use super::*;

#[test]
fn cannot_archive_backlog_board() {
    let mut store = Store::open_in_memory().unwrap();
    let backlog = store.ensure_default_board().unwrap();

    let err = store.archive_board(&backlog.id).unwrap_err().to_string();

    assert!(err.contains("Backlog board cannot be archived"));
}

#[test]
fn cannot_create_another_backlog_board() {
    let mut store = Store::open_in_memory().unwrap();
    store.ensure_default_board().unwrap();

    let err = store
        .create_board("  BACKLOG  ", BoardColumnTemplate::Planning)
        .unwrap_err()
        .to_string();

    assert!(err.contains("Backlog is the reserved default board"));
    assert!(store.board_by_slug("backlog-2").unwrap().is_none());
}

#[test]
fn board_reorder_swaps_active_neighbours() {
    let mut store = Store::open_in_memory().unwrap();
    let first = store.ensure_default_board().unwrap();
    let second = store
        .create_board("Second Board", BoardColumnTemplate::Planning)
        .unwrap();

    store.reorder_board(&second.id, -1).unwrap();

    let boards = store.list_boards().unwrap();
    assert_eq!(boards[0].id, second.id);
    assert_eq!(boards[1].id, first.id);
}

#[test]
fn create_board_uses_selected_column_template() {
    let mut store = Store::open_in_memory().unwrap();
    store.ensure_default_board().unwrap();

    let workflow = store
        .create_board("Release Work", BoardColumnTemplate::Workflow)
        .unwrap();
    let columns: Vec<String> = store
        .columns(&workflow.id)
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect();

    assert_eq!(
        columns,
        BoardColumnTemplate::Workflow
            .columns()
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn default_project_template_is_workflow() {
    assert_eq!(
        BoardColumnTemplate::DEFAULT_PROJECT,
        BoardColumnTemplate::Workflow
    );
    assert_eq!(
        BoardColumnTemplate::ALL[BoardColumnTemplate::default_index()],
        BoardColumnTemplate::Workflow
    );
}

#[test]
fn board_agent_context_trims_and_clears() {
    let mut store = Store::open_in_memory().unwrap();
    let board = store
        .create_board("Work", BoardColumnTemplate::Workflow)
        .unwrap();

    let updated = store
        .update_board_agent_context(&board.id, Some("  Run cargo test before closing.  "))
        .unwrap();
    assert_eq!(
        updated.agent_context.as_deref(),
        Some("Run cargo test before closing.")
    );

    let cleared = store
        .update_board_agent_context(&board.id, Some("  "))
        .unwrap();
    assert_eq!(cleared.agent_context, None);
}

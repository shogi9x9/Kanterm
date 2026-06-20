//! End-to-end test of the MCP server: spawn the real binary, speak
//! line-delimited JSON-RPC over stdio, and assert on tool behaviour.
//!
//! Cargo provides the built binary path via `CARGO_BIN_EXE_kanban-mcp`.

use kanban_core::BoardColumnTemplate;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn response_field<'a>(text: &'a str, name: &str) -> &'a str {
    text.lines()
        .find_map(|line| Some(line.strip_prefix(name)?.trim()))
        .unwrap_or_else(|| panic!("missing response field {name} in {text}"))
}

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    db: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", self.db));
        }
    }
}

impl Server {
    fn fresh_db() -> String {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("kanban-mcp-it-{}-{ts}-{n}.db", std::process::id()))
            .display()
            .to_string()
    }

    fn start() -> Server {
        Server::start_at(Self::fresh_db())
    }

    fn start_at(db: String) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_kanban-mcp"))
            .env("KANBAN_DB", &db)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn kanban-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut s = Server {
            child,
            stdin,
            stdout,
            db,
        };
        s.handshake();
        s
    }

    fn send(&mut self, v: &Value) {
        self.stdin.write_all(v.to_string().as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Read JSON-RPC lines until one carries the given id.
    fn recv_id(&mut self, id: i64) -> Value {
        loop {
            let mut line = String::new();
            let n = self.stdout.read_line(&mut line).expect("read");
            assert!(n > 0, "server closed stdout while awaiting id {id}");
            if line.trim().is_empty() {
                continue;
            }
            let v: Value = serde_json::from_str(&line).expect("valid json-rpc line");
            if v.get("id").and_then(Value::as_i64) == Some(id) {
                return v;
            }
        }
    }

    fn handshake(&mut self) {
        self.send(&json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":"2025-06-18","capabilities":{},
                      "clientInfo":{"name":"it","version":"0"}}
        }));
        let init = self.recv_id(1);
        assert_eq!(init["result"]["serverInfo"]["name"], "kanban-mcp");
        self.send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    }

    /// Call a tool and return its concatenated text content.
    fn call(&mut self, id: i64, name: &str, args: Value) -> String {
        self.send(&json!({
            "jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":name,"arguments":args}
        }));
        let resp = self.recv_id(id);
        resp["result"]["content"]
            .as_array()
            .map(|cs| {
                cs.iter()
                    .filter_map(|c| c["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default()
    }

    fn call_error(&mut self, id: i64, name: &str, args: Value) -> String {
        self.send(&json!({
            "jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":name,"arguments":args}
        }));
        let resp = self.recv_id(id);
        resp["error"]["message"].as_str().unwrap_or("").to_string()
    }

    fn tool_names(&mut self, id: i64) -> Vec<String> {
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":"tools/list","params":{}}));
        let resp = self.recv_id(id);
        resp["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect()
    }
}

#[test]
fn exposes_the_expected_tools() {
    let mut s = Server::start();
    let mut names = s.tool_names(2);
    names.sort();
    assert_eq!(
        names,
        vec![
            "create_card",
            "create_card_in_backlog",
            "create_cards",
            "dependency_graph",
            "get_board",
            "get_card",
            "list_cards",
            "manage_boards",
            "manage_columns",
            "recall_memories",
            "record_memory",
            "register_agent",
            "status",
            "update_card",
        ],
    );
}

#[test]
fn create_card_requires_board_and_does_not_create_card() {
    let mut s = Server::start();
    let err = s.call_error(2, "create_card", json!({"title":"missing board"}));
    assert!(err.contains("`board` is required"), "got: {err}");
    assert!(s.call(3, "list_cards", json!({})).contains("no matching"));
}

#[test]
fn create_cards_requires_board_and_does_not_create_card() {
    let mut s = Server::start();
    let err = s.call_error(
        2,
        "create_cards",
        json!({"cards":[{"title":"missing board"}]}),
    );
    assert!(err.contains("`board` is required"), "got: {err}");
    assert!(s.call(3, "list_cards", json!({})).contains("no matching"));
}

#[test]
fn create_card_existing_project_slug_reports_existing_board() {
    let mut s = Server::start();
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"Work","template":"workflow"}),
    );
    let created = s.call(
        3,
        "create_card",
        json!({"board":"work","title":"project task","column":"Todo"}),
    );
    assert!(created.contains("created WOR-1 in board 'work' (board: existing) column 'Todo'"));
}

#[test]
fn create_card_unknown_project_name_creates_board_and_card_there() {
    let mut s = Server::start();
    let created = s.call(
        2,
        "create_card",
        json!({"board":"New Project","title":"first project task"}),
    );
    assert!(created.contains("board 'new-project' (board: created)"));
    let board = s.call(3, "get_board", json!({"board":"new-project"}));
    assert!(board.contains("first project task"), "got: {board}");
    assert!(s.call(4, "list_cards", json!({})).contains("no matching"));
}

#[test]
fn create_card_rejects_backlog_board_with_guidance() {
    let mut s = Server::start();
    let err = s.call_error(
        2,
        "create_card",
        json!({"board":"backlog","title":"wrong inbox path"}),
    );
    assert!(
        err.contains(
            "create_card cannot target the Backlog board; use create_card_in_backlog instead."
        ),
        "got: {err}"
    );
}

#[test]
fn create_card_in_backlog_creates_card_in_backlog() {
    let mut s = Server::start();
    let created = s.call(2, "create_card_in_backlog", json!({"title":"inbox item"}));
    assert!(created.contains("created KB-1 in Backlog (board: backlog)"));
    let listed = s.call(3, "list_cards", json!({}));
    assert!(listed.contains("KB-1"), "got: {listed}");
    assert!(listed.contains("inbox item"), "got: {listed}");
}

#[test]
fn create_cards_unknown_project_name_creates_one_board_with_dependencies() {
    let mut s = Server::start();
    let created = s.call(
        2,
        "create_cards",
        json!({
            "board":"Launch Plan",
            "cards":[
                {"alias":"A","title":"A setup","column":"Todo"},
                {"alias":"B","title":"B ship","column":"Todo","depends_on":["A"]}
            ]
        }),
    );
    assert!(created.contains("created 2 cards in board 'launch-plan' (board: created)"));
    assert!(created.contains("1 LP-1 A setup"));
    assert!(created.contains("2 LP-2 B ship"));
    let board = s.call(3, "get_board", json!({"board":"launch-plan"}));
    assert!(board.contains("A setup"));
    assert!(board.contains("B ship"));
    let graph = s.call(4, "dependency_graph", json!({"board":"launch-plan"}));
    assert!(graph.contains("- LP-1 -> LP-2"), "got: {graph}");
}

#[test]
fn status_reports_runtime_identity() {
    let mut s = Server::start();
    let status = s.call(2, "status", json!({}));
    assert!(status.contains("kanban_mcp_status:"));
    assert!(status.contains("version:"));
    assert!(status.contains("schema_version:"));
    assert!(status.contains("db_path:"));
    assert!(status.contains("working_directory:"));
    assert!(status.contains("default_board: backlog (Backlog)"));
}

#[test]
fn memory_record_and_recall() {
    let mut s = Server::start();

    // record (linked to a card) -> key comes back
    let out = s.call(
        2,
        "record_memory",
        json!({"title":"pin rusqlite 0.37","body":"0.38 needs newer rustc\nsecond line","kind":"decision","card":"KB-1"}),
    );
    assert!(out.contains("recorded M-1 [decision]"), "got: {out}");
    s.call(3, "record_memory", json!({"title":"plain note"}));

    // recall: newest first, one line each with kind/date/card and a body snippet
    let list = s.call(4, "recall_memories", json!({}));
    let lines: Vec<&str> = list.lines().collect();
    assert_eq!(lines.len(), 2, "got: {list}");
    assert!(lines[0].starts_with("M-2 [note]"), "got: {list}");
    assert!(lines[1].contains("M-1 [decision]"), "got: {list}");
    assert!(lines[1].contains("card:KB-1"), "got: {list}");
    assert!(lines[1].contains("— 0.38 needs newer rustc"), "got: {list}");

    // filters
    assert!(!s
        .call(5, "recall_memories", json!({"query":"rusqlite"}))
        .contains("M-2"));
    assert!(!s
        .call(6, "recall_memories", json!({"card":"KB-1"}))
        .contains("M-2"));
    assert!(!s
        .call(7, "recall_memories", json!({"kind":"note"}))
        .contains("M-1"));
    assert_eq!(
        s.call(8, "recall_memories", json!({"query":"no such thing"})),
        "no memories found"
    );

    // key mode returns the full body
    let full = s.call(9, "recall_memories", json!({"key":"M-1"}));
    assert!(full.contains("# pin rusqlite 0.37"), "got: {full}");
    assert!(full.contains("second line"), "got: {full}");

    // unknown key is a clean error
    s.send(&json!({"jsonrpc":"2.0","id":10,"method":"tools/call",
        "params":{"name":"recall_memories","arguments":{"key":"M-99"}}}));
    let resp = s.recv_id(10);
    assert!(resp.get("error").is_some() || resp["result"]["isError"] == Value::Bool(true));
}

#[test]
fn manage_boards_create_and_delete() {
    let mut s = Server::start();

    // create -> appears in the board directory and is addressable
    assert!(s
        .call(2, "manage_boards", json!({"action":"create","name":"Work"}))
        .contains("template: workflow"));
    let work_board = s.call(19, "get_board", json!({"board":"work"}));
    assert!(
        work_board.contains("## In progress (0)"),
        "got: {work_board}"
    );
    assert!(s
        .call(
            15,
            "manage_boards",
            json!({
                "action":"create",
                "name":"Side",
                "template":"simple",
                "agent_context":"Run cargo test -p side before complete_note."
            })
        )
        .contains("with agent_context"));
    let side_board = s.call(20, "get_board", json!({"board":"side"}));
    assert!(side_board.contains("board_agent_context:"));
    assert!(side_board.contains("Run cargo test -p side before complete_note."));
    assert!(side_board.contains("side [context] (current)"));
    s.send(&json!({"jsonrpc":"2.0","id":18,"method":"tools/call",
        "params":{"name":"manage_boards","arguments":{"action":"create","name":"Backlog","template":"planning"}}}));
    let duplicate_backlog = s.recv_id(18);
    assert!(
        duplicate_backlog.get("error").is_some()
            || duplicate_backlog["result"]["isError"] == Value::Bool(true),
        "expected duplicate Backlog board to be rejected: {duplicate_backlog}"
    );
    let dir = s.call(3, "get_board", json!({}));
    assert!(
        dir.contains("boards: backlog (current), work, side [context]"),
        "got: {dir}"
    );
    assert!(
        dir.contains("side [context]"),
        "context marker should appear in board directory: {dir}"
    );
    assert!(s
        .call(
            16,
            "manage_boards",
            json!({"action":"reorder","board":"side","direction":"up"})
        )
        .contains("moved board 'side' up"));
    let dir = s.call(17, "get_board", json!({}));
    assert!(
        dir.contains("boards: backlog (current), side [context], work"),
        "got: {dir}"
    );
    assert!(s
        .call(
            21,
            "manage_boards",
            json!({
                "action":"set_context",
                "board":"work",
                "agent_context":"Use cargo test --workspace before release."
            })
        )
        .contains("updated board 'work' agent_context"));
    s.call(4, "create_card", json!({"board":"work","title":"on work"}));
    let card = s.call(22, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(card.contains("board_agent_context: Use cargo test --workspace before release."));
    assert!(s
        .call(5, "list_cards", json!({"board":"work"}))
        .contains("on work"));
    assert!(s
        .call(
            23,
            "manage_boards",
            json!({"action":"clear_context","board":"work"})
        )
        .contains("cleared board 'work' agent_context"));
    let work_board = s.call(24, "get_board", json!({"board":"work"}));
    assert!(!work_board.contains("Use cargo test --workspace before release."));

    // Backlog is protected; unknown slug errors; deleting a non-archived board errors
    for args in [
        json!({"action":"delete","board":"backlog"}),
        json!({"action":"delete","board":"ghost"}),
        json!({"action":"archive","board":"backlog"}),
        json!({"action":"delete","board":"work"}),
    ] {
        s.send(&json!({"jsonrpc":"2.0","id":6,"method":"tools/call",
            "params":{"name":"manage_boards","arguments":args}}));
        let resp = s.recv_id(6);
        assert!(
            resp.get("error").is_some() || resp["result"]["isError"] == Value::Bool(true),
            "expected error for {args}"
        );
    }

    // archive: leaves the active list, shows up on the archived line, cards kept
    assert!(s
        .call(
            7,
            "manage_boards",
            json!({"action":"archive","board":"work"})
        )
        .contains("archived"));
    let dir = s.call(8, "get_board", json!({}));
    assert!(dir.contains("boards: backlog (current)"), "got: {dir}");
    assert!(dir.contains("archived boards: work"), "got: {dir}");
    assert!(s
        .call(9, "list_cards", json!({"board":"work"}))
        .contains("on work"));

    // unarchive restores it to the active list
    s.call(
        10,
        "manage_boards",
        json!({"action":"unarchive","board":"work"}),
    );
    let dir = s.call(11, "get_board", json!({}));
    assert!(
        dir.contains("boards: backlog (current), side [context], work"),
        "got: {dir}"
    );
    assert!(!dir.contains("archived boards:"), "got: {dir}");

    // archive then delete the work board for good
    s.call(
        12,
        "manage_boards",
        json!({"action":"archive","board":"work"}),
    );
    assert!(s
        .call(
            13,
            "manage_boards",
            json!({"action":"delete","board":"work"})
        )
        .contains("deleted"));
    let dir = s.call(14, "get_board", json!({}));
    assert!(!dir.contains("work"), "got: {dir}");
}

#[test]
fn manage_columns_add_rename_reorder_delete() {
    let mut s = Server::start();
    assert!(s
        .call(
            20,
            "manage_boards",
            json!({"action":"create","name":"Work","template":"planning"})
        )
        .contains("slug: work"));
    s.send(&json!({"jsonrpc":"2.0","id":21,"method":"tools/call",
        "params":{"name":"manage_columns","arguments":{"action":"add","name":"Today"}}}));
    let backlog_column_change = s.recv_id(21);
    assert!(
        backlog_column_change.get("error").is_some()
            || backlog_column_change["result"]["isError"] == Value::Bool(true),
        "expected Backlog board columns to be immutable: {backlog_column_change}"
    );

    // add
    assert!(s
        .call(
            2,
            "manage_columns",
            json!({"board":"work","action":"add","name":"保留"})
        )
        .contains("added"));
    assert!(s
        .call(3, "get_board", json!({"board":"work"}))
        .contains("## 保留 (0)"));

    // rename
    s.call(
        4,
        "manage_columns",
        json!({"board":"work","action":"rename","column":"保留","new_name":"アイスボックス"}),
    );
    assert!(s
        .call(5, "get_board", json!({"board":"work"}))
        .contains("## アイスボックス (0)"));

    // put a card in This week, then delete that column moving cards to Today
    s.call(
        6,
        "create_card",
        json!({"board":"work","title":"wip","column":"This week"}),
    );
    s.call(
        7,
        "manage_columns",
        json!({"board":"work","action":"delete","column":"This week","to":"Today"}),
    );
    let board = s.call(8, "get_board", json!({"board":"work"}));
    assert!(!board.contains("## This week"));
    assert!(
        board.contains("## Today (1)"),
        "card relocated to Today: {board}"
    );

    // delete requires a destination
    s.send(&json!({"jsonrpc":"2.0","id":9,"method":"tools/call",
        "params":{"name":"manage_columns","arguments":{"board":"work","action":"delete","column":"Today"}}}));
    let resp = s.recv_id(9);
    assert!(resp.get("error").is_some() || resp["result"]["isError"] == Value::Bool(true));
}

#[test]
fn create_update_move_and_label_flow() {
    let mut s = Server::start();
    assert!(s
        .call(
            14,
            "manage_boards",
            json!({"action":"create","name":"Work","template":"planning"})
        )
        .contains("slug: work"));

    assert!(s
        .call(
            2,
            "create_card",
            json!({"board":"work","title":"fix bug","column":"Today"})
        )
        .contains("WOR-1"));
    s.call(
        3,
        "update_card",
        json!({
            "board":"work",
            "key":"WOR-1",
            "add_labels":["bug"],
            "priority":2,
            "column":"This week",
            "next_action":"write regression test",
            "blocked_reason":"needs reproduction",
            "acceptance_criteria":"get_card shows agent fields",
            "handoff_note":"resume from MCP get_card",
            "execution_note":"tried parser fixture approach; next resume from failing test",
            "agent_weight":3,
            "agent_effort":"high-reasoning",
            "suggested_model":"gpt-5",
            "expected_tokens":12000,
            "human_intervention":"review",
            "last_verification":{
                "command":"cargo test",
                "status":"passed",
                "summary":"stdio flow passed",
                "timestamp":12345
            }
        }),
    );
    s.call(
        13,
        "record_memory",
        json!({
            "title":"Remember card context",
            "body":"Memory is linked to this card",
            "kind":"decision",
            "card":"WOR-1"
        }),
    );

    let card = s.call(4, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(card.contains("column: This week"));
    assert!(card.contains("priority: [H]"));
    assert!(card.contains("labels: bug"));
    assert!(card.contains("\nagent_metadata:\n"));
    assert!(card.contains("agent_state: open"));
    assert!(card.contains("agent_weight: 3"));
    assert!(card.contains("agent_effort: high-reasoning"));
    assert!(card.contains("suggested_model: gpt-5"));
    assert!(card.contains("expected_tokens: 12000"));
    assert!(card.contains("human_intervention: review"));
    assert!(card.contains("\nbody:\n"));
    assert!(card.contains("next_action: write regression test"));
    assert!(card.contains("blocked_reason: needs reproduction"));
    assert!(card.contains("acceptance_criteria: get_card shows agent fields"));
    assert!(card.contains("handoff_note: resume from MCP get_card"));
    assert!(card.contains("last_verification: {\"command\":\"cargo test\""));
    assert!(card.contains("\nexecution_notes:\n- "));
    assert!(card.contains("tried parser fixture approach; next resume from failing test"));
    assert!(card.contains("\"status\":\"passed\""));
    assert!(card.contains("\"timestamp\":12345"));
    assert!(card.contains("activity:\n- "));
    assert!(card.contains("agent update WOR-1"));
    assert!(card.contains("related_memories:\n- M-1 [decision] Remember card context"));

    let board = s.call(5, "get_board", json!({"board":"work"}));
    assert!(board.contains("## This week (1)"));
    assert!(board.contains("WOR-1 fix bug"));
    assert!(board.contains("[w:3 human:review]"));

    // Filter by query.
    let listed = s.call(6, "list_cards", json!({"board":"work","query":"fix"}));
    assert!(listed.contains("WOR-1"));
    assert!(listed.contains("[blocked]"));
    assert!(listed.contains("[w:3 effort:high-reasoning model:gpt-5 tokens:12000 human:review]"));
    let by_next = s.call(
        7,
        "list_cards",
        json!({"board":"work","query":"regression"}),
    );
    assert!(by_next.contains("WOR-1"));
    let by_criteria = s.call(
        8,
        "list_cards",
        json!({"board":"work","query":"agent fields"}),
    );
    assert!(by_criteria.contains("WOR-1"));
    let by_execution_metadata = s.call(
        15,
        "list_cards",
        json!({
            "board":"work",
            "agent_weight_max":3,
            "agent_effort":"high-reasoning",
            "suggested_model":"gpt-5",
            "expected_tokens_max":15000,
            "human_intervention":"review"
        }),
    );
    assert!(by_execution_metadata.contains("WOR-1"));
    let too_small_budget = s.call(
        16,
        "list_cards",
        json!({"board":"work","expected_tokens_max":1000}),
    );
    assert!(too_small_budget.contains("no matching"));
    let by_metadata_query = s.call(
        17,
        "list_cards",
        json!({"board":"work","query":"high-reasoning"}),
    );
    assert!(by_metadata_query.contains("WOR-1"));
    s.call(
        9,
        "update_card",
        json!({"board":"work","key":"WOR-1","blocked_reason":""}),
    );
    let cleared = s.call(10, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(cleared.contains("blocked_reason: -"));
    let after_clear = s.call(
        11,
        "list_cards",
        json!({"board":"work","query":"regression"}),
    );
    assert!(after_clear.contains("[next]"));
    assert!(!after_clear.contains("[blocked]"));
    let empty = s.call(
        12,
        "list_cards",
        json!({"board":"work","query":"nonexistent"}),
    );
    assert!(empty.contains("no matching"));

    s.call(
        18,
        "update_card",
        json!({
            "board":"work",
            "key":"WOR-1",
            "agent_weight":null,
            "agent_effort":"",
            "suggested_model":"",
            "expected_tokens":null,
            "human_intervention":""
        }),
    );
    let cleared_metadata = s.call(19, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(cleared_metadata.contains("agent_weight: -"));
    assert!(cleared_metadata.contains("agent_effort: -"));
    assert!(cleared_metadata.contains("suggested_model: -"));
    assert!(cleared_metadata.contains("expected_tokens: -"));
    assert!(cleared_metadata.contains("human_intervention: -"));
}

#[test]
fn update_card_rejects_invalid_execution_metadata() {
    let mut s = Server::start();
    assert!(s
        .call(
            2,
            "create_card_in_backlog",
            json!({"title":"execution metadata"})
        )
        .contains("KB-1"));

    s.send(&json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","agent_weight":0}}}));
    let bad_weight = s.recv_id(3);
    assert!(
        bad_weight.get("error").is_some() || bad_weight["result"]["isError"] == Value::Bool(true),
        "got: {bad_weight}"
    );

    s.send(&json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","expected_tokens":0}}}));
    let bad_tokens = s.recv_id(4);
    assert!(
        bad_tokens.get("error").is_some() || bad_tokens["result"]["isError"] == Value::Bool(true),
        "got: {bad_tokens}"
    );

    s.send(&json!({"jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","human_intervention":"maybe"}}}));
    let bad_human = s.recv_id(5);
    assert!(
        bad_human.get("error").is_some() || bad_human["result"]["isError"] == Value::Bool(true),
        "got: {bad_human}"
    );
}

#[test]
fn create_cards_imports_ordered_execution_plan() {
    let mut s = Server::start();
    assert!(s
        .call(
            2,
            "manage_boards",
            json!({"action":"create","name":"Plan","template":"workflow"})
        )
        .contains("slug: plan"));

    let created = s.call(
        3,
        "create_cards",
        json!({
            "board":"plan",
            "cards":[
                {
                    "alias":"A",
                    "title":"A prepare fixtures",
                    "body":"Set up test fixtures",
                    "column":"Todo",
                    "acceptance_criteria":"fixtures exist",
                    "next_action":"write fixture files",
                    "agent_weight":1,
                    "agent_effort":"low",
                    "suggested_model":"gpt-5-mini",
                    "expected_tokens":1000,
                    "human_intervention":"none"
                },
                {
                    "alias":"B",
                    "title":"B implement parser",
                    "body":"Implement parser changes",
                    "column":"Todo",
                    "acceptance_criteria":"parser tests pass",
                    "next_action":"change parser",
                    "depends_on":["A"],
                    "agent_weight":4,
                    "agent_effort":"high",
                    "suggested_model":"gpt-5",
                    "expected_tokens":12000,
                    "human_intervention":"review"
                }
            ]
        }),
    );
    assert!(created.contains("1 PLA-1 A prepare fixtures"));
    assert!(created.contains("2 PLA-2 B implement parser"));

    let listed = s.call(4, "list_cards", json!({"board":"plan"}));
    let first = listed.find("PLA-1").unwrap();
    let second = listed.find("PLA-2").unwrap();
    assert!(first < second, "{listed}");
    assert!(listed.contains("[w:1 effort:low model:gpt-5-mini tokens:1000]"));
    assert!(listed.contains("[w:4 effort:high model:gpt-5 tokens:12000 human:review]"));

    let card = s.call(5, "get_card", json!({"board":"plan","key":"PLA-2"}));
    assert!(card.contains("acceptance_criteria: parser tests pass"));
    assert!(card.contains("next_action: change parser"));
    assert!(card.contains("agent_weight: 4"));
    assert!(card.contains("human_intervention: review"));
    assert!(card.contains("upstream: PLA-1"));
    assert!(card.contains("readiness: dependency_blocked by PLA-1"));

    let graph = s.call(6, "dependency_graph", json!({"board":"plan"}));
    assert!(graph.contains("- PLA-1 -> PLA-2"));
    assert!(graph.contains("stage 1: PLA-1"));
    assert!(graph.contains("stage 2: PLA-2"));

    let followup = s.call(
        7,
        "create_cards",
        json!({
            "board":"plan",
            "cards":[
                {
                    "alias":"C",
                    "title":"C existing-key follow-up",
                    "column":"Todo",
                    "acceptance_criteria":"existing dependency works",
                    "next_action":"depend on existing card",
                    "depends_on":["PLA-1"]
                }
            ]
        }),
    );
    assert!(followup.contains("1 PLA-3 C existing-key follow-up"));
    let followup_card = s.call(8, "get_card", json!({"board":"plan","key":"PLA-3"}));
    assert!(followup_card.contains("upstream: PLA-1"));
}

#[test]
fn create_cards_rejects_unknown_dependency_alias_and_cycles() {
    let mut s = Server::start();
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"Bad Plan","template":"workflow"}),
    );
    let unknown = s.call_error(
        3,
        "create_cards",
        json!({
            "board":"bad-plan",
            "cards":[
                {"alias":"A","title":"A","depends_on":["missing"]}
            ]
        }),
    );
    assert!(unknown.contains("unknown alias or key 'missing'"));
    let after_unknown = s.call(4, "list_cards", json!({"board":"bad-plan"}));
    assert_eq!(after_unknown, "(no matching cards)");

    s.call(
        7,
        "create_card",
        json!({"board":"bad-plan","title":"existing"}),
    );
    let alias_collision = s.call_error(
        8,
        "create_cards",
        json!({
            "board":"bad-plan",
            "cards":[
                {"alias":"BP-1","title":"collides with existing key"}
            ]
        }),
    );
    assert!(alias_collision.contains("alias conflicts with existing card key 'BP-1'"));
    let after_alias_collision = s.call(9, "list_cards", json!({"board":"bad-plan"}));
    assert!(after_alias_collision.contains("BP-1"));
    assert!(!after_alias_collision.contains("BP-2"));

    let cycle = s.call_error(
        5,
        "create_cards",
        json!({
            "board":"bad-plan",
            "cards":[
                {"alias":"B","title":"B","depends_on":["C"]},
                {"alias":"C","title":"C","depends_on":["B"]}
            ]
        }),
    );
    assert!(cycle.contains("cycle"));
    let after_cycle = s.call(6, "list_cards", json!({"board":"bad-plan"}));
    assert!(after_cycle.contains("BP-1"));
    assert!(!after_cycle.contains("BP-2"));
    assert!(!after_cycle.contains("BP-3"));
}

#[test]
fn list_cards_queue_modes_select_next_work() {
    let mut s = Server::start();
    assert!(s
        .call(
            2,
            "manage_boards",
            json!({"action":"create","name":"Queue","template":"workflow"})
        )
        .contains("slug: queue"));
    s.call(
        3,
        "create_cards",
        json!({
            "board":"queue",
            "cards":[
                {
                    "title":"ready",
                    "column":"Todo",
                    "acceptance_criteria":"done means tested",
                    "next_action":"run test",
                    "agent_weight":2,
                    "expected_tokens":2000,
                    "human_intervention":"none"
                },
                {
                    "title":"blocked",
                    "column":"Todo",
                    "acceptance_criteria":"unblocked",
                    "next_action":"wait",
                    "human_intervention":"none"
                },
                {
                    "title":"review",
                    "column":"Todo",
                    "acceptance_criteria":"human signs off",
                    "next_action":"present diff",
                    "human_intervention":"review"
                },
                {
                    "title":"missing context",
                    "column":"Todo",
                    "human_intervention":"none"
                },
                {
                    "title":"claimed",
                    "column":"Todo",
                    "acceptance_criteria":"claim held",
                    "next_action":"work",
                    "human_intervention":"none"
                },
                {
                    "title":"dependency wait",
                    "column":"Todo",
                    "acceptance_criteria":"upstream complete",
                    "next_action":"resume",
                    "human_intervention":"none"
                },
                {
                    "title":"ready high priority",
                    "column":"Todo",
                    "acceptance_criteria":"done means tested",
                    "next_action":"run focused test",
                    "agent_weight":1,
                    "expected_tokens":500,
                    "human_intervention":"none"
                }
            ]
        }),
    );
    s.call(
        4,
        "update_card",
        json!({"board":"queue","key":"QUE-2","blocked_reason":"waiting for input"}),
    );
    let agent = s.call(5, "register_agent", json!({"requested_name":"queue"}));
    let identity = response_field(&agent, "assigned_identity:").to_string();
    let token = response_field(&agent, "claim_token:").to_string();
    s.call(
        6,
        "update_card",
        json!({"board":"queue","key":"QUE-5","claim":identity,"claim_token":token}),
    );
    s.call(
        7,
        "update_card",
        json!({"board":"queue","key":"QUE-6","depends_on":["QUE-1"]}),
    );
    s.call(
        8,
        "update_card",
        json!({"board":"queue","key":"QUE-7","priority":2}),
    );

    let executable = s.call(
        9,
        "list_cards",
        json!({"board":"queue","queue":"executable"}),
    );
    assert!(executable.contains("QUE-1"));
    assert!(executable.contains("QUE-7"));
    assert!(executable.contains("[queue:executable]"));
    assert!(!executable.contains("QUE-2"));
    assert!(!executable.contains("QUE-3"));
    assert!(!executable.contains("QUE-4"));
    assert!(!executable.contains("QUE-5"));
    assert!(!executable.contains("QUE-6"));

    let ranked = s.call(
        10,
        "list_cards",
        json!({"board":"queue","queue":"executable","ranked":true}),
    );
    let high = ranked.find("QUE-7").unwrap();
    let normal = ranked.find("QUE-1").unwrap();
    assert!(high < normal, "{ranked}");
    assert!(ranked.contains("[rank:priority=high weight=1 tokens=500]"));

    let blocked = s.call(11, "list_cards", json!({"board":"queue","queue":"blocked"}));
    assert!(blocked.contains("QUE-2"));
    assert!(blocked.contains("[queue:blocked]"));

    let review = s.call(12, "list_cards", json!({"board":"queue","queue":"review"}));
    assert!(review.contains("QUE-3"));
    assert!(review.contains("[queue:review]"));

    let missing = s.call(
        13,
        "list_cards",
        json!({"board":"queue","queue":"missing_context"}),
    );
    assert!(missing.contains("QUE-4"));
    assert!(missing.contains("[queue:missing-context]"));

    let claimed = s.call(14, "list_cards", json!({"board":"queue","queue":"claimed"}));
    assert!(claimed.contains("QUE-5"));
    assert!(claimed.contains("[queue:claimed]"));

    let dependency_blocked = s.call(
        15,
        "list_cards",
        json!({"board":"queue","queue":"dependency_blocked"}),
    );
    assert!(dependency_blocked.contains("QUE-6"));
    assert!(dependency_blocked.contains("[blocked_by:QUE-1]"));
    assert!(dependency_blocked.contains("[queue:dependency-blocked]"));
}

#[test]
fn dependency_graph_renders_stages_and_blockers() {
    let mut s = Server::start();
    s.call(
        2,
        "manage_boards",
        json!({"action":"create","name":"Graph","template":"workflow"}),
    );
    s.call(
        3,
        "create_cards",
        json!({
            "board":"graph",
            "cards":[
                {"title":"A","acceptance_criteria":"done","next_action":"do A"},
                {"title":"B","acceptance_criteria":"done","next_action":"do B"},
                {"title":"C","acceptance_criteria":"done","next_action":"do C"},
                {"title":"D","acceptance_criteria":"done","next_action":"do D"},
                {"title":"E","acceptance_criteria":"done","next_action":"do E"}
            ]
        }),
    );
    s.call(
        4,
        "update_card",
        json!({"board":"graph","key":"GRA-2","depends_on":["GRA-1"]}),
    );
    s.call(
        5,
        "update_card",
        json!({"board":"graph","key":"GRA-3","depends_on":["GRA-1"]}),
    );
    s.call(
        6,
        "update_card",
        json!({"board":"graph","key":"GRA-4","depends_on":["GRA-1"]}),
    );
    s.call(
        7,
        "update_card",
        json!({"board":"graph","key":"GRA-5","depends_on":["GRA-2","GRA-3","GRA-4"]}),
    );
    s.call(
        8,
        "update_card",
        json!({"board":"graph","key":"GRA-5","human_intervention":"decision"}),
    );

    let graph = s.call(9, "dependency_graph", json!({"board":"graph"}));
    assert!(graph.contains("- GRA-1 -> GRA-2"));
    assert!(graph.contains("- GRA-1 -> GRA-3"));
    assert!(graph.contains("- GRA-1 -> GRA-4"));
    assert!(graph.contains("stage 1: GRA-1"));
    assert!(graph.contains("stage 2: GRA-2(dep-blocked), GRA-3(dep-blocked), GRA-4(dep-blocked)"));
    assert!(graph.contains("stage 3: GRA-5(human:decision)"));

    let blocked = s.call(
        10,
        "list_cards",
        json!({"board":"graph","queue":"dependency_blocked"}),
    );
    assert!(blocked.contains("GRA-2"));
    assert!(blocked.contains("[blocked_by:GRA-1]"));

    let card = s.call(11, "get_card", json!({"board":"graph","key":"GRA-2"}));
    assert!(card.contains("dependencies:"));
    assert!(card.contains("upstream: GRA-1"));
    assert!(card.contains("readiness: dependency_blocked by GRA-1"));

    s.call(
        12,
        "update_card",
        json!({"board":"graph","key":"GRA-1","agent_state":"done","archived":true}),
    );
    let executable = s.call(
        13,
        "list_cards",
        json!({"board":"graph","queue":"executable"}),
    );
    assert!(executable.contains("GRA-2"));
    assert!(executable.contains("GRA-3"));
    assert!(executable.contains("GRA-4"));
    assert!(!executable.contains("GRA-5"));

    let active_graph = s.call(
        14,
        "dependency_graph",
        json!({"board":"graph","active_only":true}),
    );
    assert!(!active_graph.contains("- GRA-1 -> GRA-2"));
    assert!(!active_graph.contains("- GRA-1 -> GRA-3"));
    assert!(!active_graph.contains("- GRA-1 -> GRA-4"));
    assert!(active_graph.contains("- GRA-2 -> GRA-5"));

    let focused_graph = s.call(
        15,
        "dependency_graph",
        json!({"board":"graph","focus":"GRA-5","active_only":true}),
    );
    assert!(focused_graph.contains("- GRA-2 -> GRA-5"));
    assert!(focused_graph.contains("- GRA-3 -> GRA-5"));
    assert!(focused_graph.contains("- GRA-4 -> GRA-5"));
    assert!(!focused_graph.contains("- GRA-1 -> GRA-2"));

    let missing_focus = s.call_error(
        16,
        "dependency_graph",
        json!({"board":"graph","focus":"GRA-999"}),
    );
    assert!(missing_focus.contains("no card 'GRA-999'"));
}

#[test]
fn execution_notes_are_append_only_resume_history() {
    let mut s = Server::start();
    assert!(s
        .call(2, "create_card_in_backlog", json!({"title":"resume work"}))
        .contains("KB-1"));
    s.call(
        3,
        "update_card",
        json!({
            "key":"KB-1",
            "next_action":"continue from failing test",
            "acceptance_criteria":"tests pass",
            "execution_note":"first attempt hit stale fixture"
        }),
    );
    s.call(
        4,
        "update_card",
        json!({
            "key":"KB-1",
            "execution_note":"second attempt narrowed failure to renderer"
        }),
    );
    let card = s.call(5, "get_card", json!({"key":"KB-1"}));
    assert!(card.contains("execution_notes:\n- "));
    assert!(card.contains("first attempt hit stale fixture"));
    assert!(card.contains("second attempt narrowed failure to renderer"));

    s.call(
        6,
        "update_card",
        json!({"key":"KB-1","complete_note":"done after renderer fix"}),
    );
    let completed = s.call(7, "get_card", json!({"key":"KB-1"}));
    assert!(completed.contains("agent_state: done"));
    assert!(completed.contains("next_action: -"));
    assert!(completed.contains("execution_notes:\n- "));
    assert!(completed.contains("first attempt hit stale fixture"));
    assert!(completed.contains("second attempt narrowed failure to renderer"));
}

#[test]
fn update_card_with_complete_note_appends_body_and_archives() {
    let mut s = Server::start();

    assert!(s
        .call(
            2,
            "create_card_in_backlog",
            json!({"title":"release","body":"実装内容"})
        )
        .contains("KB-1"));

    assert!(s
        .call(
            3,
            "update_card",
            json!({"key":"KB-1","complete_note":"CI 通過を確認"})
        )
        .contains("updated"));

    let card = s.call(4, "get_card", json!({"key":"KB-1"}));
    assert!(card.contains("実装内容"));
    assert!(card.contains("[completion note] CI 通過を確認"));
    assert!(card.contains("agent_state: done"));
    assert!(card.contains("next_action: -"));
    assert!(card.contains("blocked_reason: -"));
    assert!(card.contains("handoff_note: -"));
    assert!(card.contains("claim: -"));

    let board = s.call(5, "list_cards", json!({}));
    assert!(!board.contains("KB-1"));
}

#[test]
fn update_card_claims_and_releases_lease() {
    let mut s = Server::start();
    assert!(s
        .call(2, "create_card_in_backlog", json!({"title":"claimed work"}))
        .contains("KB-1"));
    let codex = s.call(3, "register_agent", json!({"requested_name":"codex"}));
    let codex_identity = response_field(&codex, "assigned_identity:").to_string();
    let codex_token = response_field(&codex, "claim_token:").to_string();
    let claude = s.call(4, "register_agent", json!({"requested_name":"claude"}));
    let claude_identity = response_field(&claude, "assigned_identity:").to_string();
    let claude_token = response_field(&claude, "claim_token:").to_string();

    assert!(s
        .call(
            5,
            "update_card",
            json!({"key":"KB-1","claim":codex_identity.clone(),"claim_token":codex_token.clone(),"lease_minutes":30})
        )
        .contains("updated KB-1"));
    let claimed = s.call(6, "get_card", json!({"key":"KB-1"}));
    assert!(claimed.contains(&format!("claim: {codex_identity} until lease_expires_at=")));
    let listed = s.call(7, "list_cards", json!({}));
    assert!(listed.contains(&format!("[claimed:{codex_identity}]")));

    s.send(&json!({"jsonrpc":"2.0","id":8,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","claim":claude_identity,"claim_token":claude_token}}}));
    let conflict = s.recv_id(8);
    assert!(
        conflict.get("error").is_some() || conflict["result"]["isError"] == Value::Bool(true),
        "got: {conflict}"
    );

    assert!(s
        .call(
            9,
            "update_card",
            json!({"key":"KB-1","release_claim":true,"claim_token":codex_token})
        )
        .contains("updated KB-1"));
    let released = s.call(10, "get_card", json!({"key":"KB-1"}));
    assert!(released.contains("claim: -"));
}

#[test]
fn due_dates_and_errors() {
    let mut s = Server::start();
    s.call(2, "create_card_in_backlog", json!({"title":"task"}));

    // A past date is flagged overdue (test data is well before any plausible run date).
    s.call(3, "update_card", json!({"key":"KB-1","due":"2000-01-01"}));
    assert!(s
        .call(4, "get_card", json!({"key":"KB-1"}))
        .contains("(overdue)"));
    assert!(s
        .call(5, "get_board", json!({}))
        .contains("!due:2000-01-01"));

    // Bad date -> JSON-RPC error, not a panic.
    s.send(&json!({
        "jsonrpc":"2.0","id":6,"method":"tools/call",
        "params":{"name":"update_card","arguments":{"key":"KB-1","due":"2026-99-99"}}
    }));
    let resp = s.recv_id(6);
    assert!(
        resp.get("error").is_some() || resp["result"]["isError"] == serde_json::Value::Bool(true),
        "invalid date should surface as an error: {resp}"
    );

    // Clearing works.
    s.call(7, "update_card", json!({"key":"KB-1","due":""}));
    assert!(s
        .call(8, "get_card", json!({"key":"KB-1"}))
        .contains("due: -"));
}

#[test]
fn update_card_detects_stale_expected_updated_at() {
    let mut s = Server::start();
    assert!(s
        .call(2, "create_card_in_backlog", json!({"title":"task"}))
        .contains("KB-1"));

    s.send(&json!({
        "jsonrpc":"2.0",
        "id":3,
        "method":"tools/call",
        "params":{
            "name":"update_card",
            "arguments": {"key":"KB-1","title":"nope","expected_updated_at":0}
        }
    }));
    let resp = s.recv_id(3);
    assert!(resp.get("error").is_some() || resp["result"]["isError"] == Value::Bool(true));
    assert!(resp.to_string().contains("stale update"), "got: {resp}");
}

#[test]
fn board_param_addresses_distinct_boards() {
    // Seed a second board directly via the core before starting the server
    // (the MCP surface deliberately has no create-board tool).
    let db = Server::fresh_db();
    {
        let mut store = kanban_core::Store::open(&db).unwrap();
        store.ensure_default_board().unwrap();
        let work = store
            .create_board("Work", BoardColumnTemplate::Planning)
            .unwrap();
        store
            .create_card(&work.id, Some("This week"), "ship it", "", "t")
            .unwrap();
    }
    let mut s = Server::start_at(db);

    // Default board (backlog) is empty; its directory lists both boards.
    let backlog = s.call(2, "get_board", json!({}));
    assert!(backlog.contains("boards: backlog (current), work"));
    assert!(!backlog.contains("ship it"));

    // Targeting the work board by slug shows its card and flips "current".
    let work = s.call(3, "get_board", json!({"board":"work"}));
    assert!(work.contains("WOR-1 ship it"), "got: {work}");
    assert!(work.contains("work (current)"));

    // Writes honour the board param too.
    assert!(s
        .call(4, "create_card", json!({"board":"work","title":"second"}))
        .contains("created"));
    assert!(s
        .call(5, "list_cards", json!({"board":"work"}))
        .contains("second"));
    // ...and don't leak onto backlog.
    assert!(s.call(6, "list_cards", json!({})).contains("no matching"));

    // Unknown board errors cleanly.
    s.send(&json!({"jsonrpc":"2.0","id":7,"method":"tools/call",
        "params":{"name":"get_board","arguments":{"board":"ghost"}}}));
    let resp = s.recv_id(7);
    assert!(resp.get("error").is_some() || resp["result"]["isError"] == Value::Bool(true));
}

#[test]
fn update_card_can_move_to_another_board() {
    let mut s = Server::start();

    s.call(
        2,
        "create_card_in_backlog",
        json!({"title":"migrate across boards"}),
    );
    assert!(s
        .call(
            3,
            "manage_boards",
            json!({"action":"create","name":"Work","template":"planning"})
        )
        .contains("slug: work"));
    assert!(s
        .call(
            4,
            "update_card",
            json!({
                "key":"KB-1","move_to_board":"work","column":"This week"
            })
        )
        .contains("updated"));

    assert!(!s
        .call(5, "list_cards", json!({}))
        .contains("migrate across boards"));
    let work = s.call(6, "list_cards", json!({"board":"work"}));
    assert!(work.contains("migrate across boards"));
    let detail = s.call(7, "get_card", json!({"board":"work","key":"WOR-1"}));
    assert!(detail.contains("move_board KB-1 -> WOR-1; Backlog (backlog) -> Work (work)"));
}

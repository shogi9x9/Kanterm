use serde_json::{json, Value};

use crate::support::Server;

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

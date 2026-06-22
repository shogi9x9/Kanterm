use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn response_field<'a>(text: &'a str, name: &str) -> &'a str {
    text.lines()
        .find_map(|line| Some(line.strip_prefix(name)?.trim()))
        .unwrap_or_else(|| panic!("missing response field {name} in {text}"))
}

pub(crate) struct Server {
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
    pub(crate) fn fresh_db() -> String {
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

    pub(crate) fn start() -> Server {
        Server::start_at(Self::fresh_db())
    }

    pub(crate) fn start_at(db: String) -> Server {
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

    pub(crate) fn send(&mut self, v: &Value) {
        self.stdin.write_all(v.to_string().as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Read JSON-RPC lines until one carries the given id.
    pub(crate) fn recv_id(&mut self, id: i64) -> Value {
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
    pub(crate) fn call(&mut self, id: i64, name: &str, args: Value) -> String {
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

    pub(crate) fn call_error(&mut self, id: i64, name: &str, args: Value) -> String {
        self.send(&json!({
            "jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":name,"arguments":args}
        }));
        let resp = self.recv_id(id);
        resp["error"]["message"].as_str().unwrap_or("").to_string()
    }

    pub(crate) fn tool_names(&mut self, id: i64) -> Vec<String> {
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

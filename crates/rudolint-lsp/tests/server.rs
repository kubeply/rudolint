use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn stdio_server_lints_and_handles_editor_requests() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    std::fs::write(&dockerfile, "FROM alpine:latest\n").expect("Dockerfile should be written");
    let root_uri = url::Url::from_directory_path(temp.path())
        .expect("workspace uri should be built")
        .to_string();
    let document_uri = url::Url::from_file_path(&dockerfile)
        .expect("document uri should be built")
        .to_string();
    let mut server = LspServer::spawn();

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": root_uri,
            "capabilities": {},
            "workspaceFolders": [{ "uri": root_uri, "name": "workspace" }]
        }
    }));
    let initialize = server.recv_response(1);
    assert_eq!(initialize["id"], 1);
    assert_eq!(initialize["result"]["serverInfo"]["name"], "rudolint-lsp");
    assert_eq!(initialize["result"]["capabilities"]["textDocumentSync"], 1);
    assert_eq!(
        initialize["result"]["capabilities"]["hoverProvider"],
        Value::Bool(true)
    );
    assert_eq!(
        initialize["result"]["capabilities"]["codeActionProvider"],
        Value::Bool(true)
    );

    server.send(json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }));
    server.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/hover",
        "params": { "textDocument": { "uri": document_uri } }
    }));
    let invalid_hover = server.recv_response(2);
    assert_eq!(invalid_hover["error"]["code"], -32602);

    let initial_text = "# DL3007\nFROM alpine:latest\nRUN --mount=type=cache,target=/var/cache/apt apt-get update\n";
    server.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": document_uri,
                "languageId": "dockerfile",
                "version": 1,
                "text": initial_text
            }
        }
    }));
    let diagnostics = server.recv_method("textDocument/publishDiagnostics");
    assert_eq!(diagnostics["params"]["uri"], document_uri);
    assert_eq!(diagnostics["params"]["version"], 1);
    assert!(diagnostic_codes(&diagnostics).contains(&"DL3007".to_string()));

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": document_uri },
            "position": { "line": 0, "character": 3 }
        }
    }));
    let hover = server.recv_response(3);
    assert!(
        hover["result"]["contents"]["value"]
            .as_str()
            .expect("hover should contain markdown")
            .contains("DL3007")
    );

    server.send(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": { "uri": document_uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 2, "character": 0 }
            },
            "context": { "diagnostics": [] }
        }
    }));
    let code_actions = server.recv_response(4);
    let action_titles = code_actions["result"]
        .as_array()
        .expect("code actions should be an array")
        .iter()
        .filter_map(|action| action["title"].as_str())
        .collect::<Vec<_>>();
    assert!(action_titles.contains(&"insert BuildKit syntax directive"));

    server.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": document_uri, "version": 2 },
            "contentChanges": [{ "text": "FROM alpine:3.20\n" }]
        }
    }));
    let changed = server.recv_method("textDocument/publishDiagnostics");
    assert_eq!(changed["params"]["version"], 2);
    assert!(!diagnostic_codes(&changed).contains(&"DL3007".to_string()));

    server.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": { "textDocument": { "uri": document_uri } }
    }));
    let closed = server.recv_method("textDocument/publishDiagnostics");
    assert!(
        closed["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    server.shutdown();
}

struct LspServer {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
    pending: VecDeque<Value>,
}

impl LspServer {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rudolint-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("rudolint-lsp should start");
        let stdin = child.stdin.take().expect("stdin should be piped");
        let stdout = child.stdout.take().expect("stdout should be piped");
        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Ok(Some(message)) = read_lsp_message(&mut reader) {
                if sender.send(message).is_err() {
                    break;
                }
            }
        });

        Self {
            child,
            stdin,
            messages,
            pending: VecDeque::new(),
        }
    }

    fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message).expect("message should serialize");
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("header should write");
        self.stdin
            .write_all(&body)
            .expect("message body should write");
        self.stdin.flush().expect("message should flush");
    }

    fn recv_timeout(&mut self, timeout: Duration) -> Value {
        self.pending.pop_front().unwrap_or_else(|| {
            self.messages
                .recv_timeout(timeout)
                .expect("server should send a message")
        })
    }

    fn recv_response(&mut self, id: i64) -> Value {
        let mut skipped = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let timeout = deadline.saturating_duration_since(Instant::now());
            let message = self.recv_timeout(timeout);
            if message["id"] == id {
                self.restore_skipped(skipped);
                return message;
            }
            skipped.push(message);
        }

        self.restore_skipped(skipped);
        panic!("server did not send response id {id}");
    }

    fn recv_method(&mut self, method: &str) -> Value {
        let mut skipped = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let timeout = deadline.saturating_duration_since(Instant::now());
            let message = self.recv_timeout(timeout);
            if message["method"] == method {
                self.restore_skipped(skipped);
                return message;
            }
            skipped.push(message);
        }

        self.restore_skipped(skipped);
        panic!("server did not send {method}");
    }

    fn restore_skipped(&mut self, skipped: Vec<Value>) {
        for message in skipped.into_iter().rev() {
            self.pending.push_front(message);
        }
    }

    fn shutdown(&mut self) {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "shutdown",
            "params": null
        }));
        self.recv_response(99);
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }));
        let status = self.child.wait().expect("server should exit");
        assert!(status.success(), "server exited with {status}");
    }
}

impl Drop for LspServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_lsp_message(reader: &mut BufReader<impl Read>) -> std::io::Result<Option<Value>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }

        if line == "\r\n" {
            break;
        }

        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .expect("content length should be numeric"),
            );
        }
    }

    let Some(content_length) = content_length else {
        return Ok(None);
    };
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;
    let message = serde_json::from_slice(&body).expect("LSP message should be JSON");
    Ok(Some(message))
}

fn diagnostic_codes(message: &Value) -> Vec<String> {
    message["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array")
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str().map(str::to_string))
        .collect()
}

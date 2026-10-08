//! C3: `taskhub mcp` over stdio, end to end against the mock server.

mod common;

use common::*;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Stdio};

struct Mcp {
    child: std::process::Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: u64,
}

impl Mcp {
    fn start(sandbox: &Sandbox) -> Mcp {
        let mut child = sandbox
            .command()
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Mcp { child, input, output, next_id: 0 }
    }

    fn send_line(&mut self, line: &str) {
        writeln!(self.input, "{line}").unwrap();
        self.input.flush().unwrap();
    }

    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("stdout line is not JSON ({e}): {line:?}"))
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let message = json!({ "jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params });
        self.send_line(&message.to_string());
        let reply = self.read();
        assert_eq!(reply["id"], self.next_id, "{reply}");
        reply
    }

    /// Calls a tool and returns (the CLI envelope, isError).
    fn call(&mut self, name: &str, arguments: Value) -> (Value, bool) {
        let reply = self.request("tools/call", json!({ "name": name, "arguments": arguments }));
        let result = &reply["result"];
        let text =
            result["content"][0]["text"].as_str().unwrap_or_else(|| panic!("no text content: {reply}"));
        (serde_json::from_str(text).unwrap(), result["isError"].as_bool().unwrap())
    }

    fn close(mut self) -> i32 {
        drop(self.input);
        self.child.wait().unwrap().code().unwrap_or(-1)
    }
}

fn server() -> MockServer {
    MockServer::start(|r| match (r.method.as_str(), r.url.split('?').next().unwrap()) {
        ("GET", "/api/v1/me") => Reply::json(200, fixture("success/getMe.json")),
        ("GET", "/api/v1/items/WEB-12/context") => Reply::json(200, fixture("success/getContext.json")),
        ("POST", "/api/v1/items/WEB-12/comments") => {
            let key = r.header("Idempotency-Key").unwrap_or_default();
            let body = json!({
                "data": { "key": "WEB-12", "version": 8, "status": "in_progress",
                          "commentId": "00000000-0000-4000-8000-000000000005", "mentioned": [], "unmatchedMentions": [] },
                "meta": { "apiVersion": 1, "requestId": "r", "operation": { "id": key, "replayed": false } },
            });
            Reply::json(200, body.to_string())
        }
        _ => Reply::problem(404, "NOT_FOUND"),
    })
}

#[test]
fn a_session_initializes_lists_tools_reads_and_writes() {
    let server = server();
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('M')).code, 0);
    let mut mcp = Mcp::start(&sandbox);

    let init = mcp.request("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }));
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "taskhub");
    mcp.send_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    assert_eq!(mcp.request("ping", json!({}))["result"], json!({}), "notifications get no reply");

    let tools = mcp.request("tools/list", json!({}));
    let names: Vec<&str> =
        tools["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names.len(), 25);
    assert!(!names.iter().any(|n| n.contains("delete")), "agents never delete");

    let (show, is_error) = mcp.call("show", json!({ "key": "WEB-12" }));
    assert!(!is_error, "{show}");
    assert_eq!(show["ok"], true);
    assert_eq!(show["data"]["item"]["key"], "WEB-12");

    let (comment, is_error) = mcp.call("comment", json!({ "key": "WEB-12", "body": "--looks like a flag" }));
    assert!(!is_error, "{comment}");
    assert_eq!(comment["meta"]["operation"]["outcome"], "committed");
    let posted: Vec<Recorded> = server.requests().into_iter().filter(|r| r.method == "POST").collect();
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0].json(), json!({ "body": "--looks like a flag" }));
    assert_eq!(
        posted[0].header("Idempotency-Key").unwrap(),
        comment["meta"]["operation"]["id"].as_str().unwrap()
    );

    assert_eq!(mcp.close(), 0);
    let operations = sandbox.home().join(".local/state/taskhub/operations");
    let journal: String = std::fs::read_dir(&operations)
        .unwrap_or_else(|e| panic!("{}: {e}", operations.display()))
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect();
    let op = comment["meta"]["operation"]["id"].as_str().unwrap();
    assert!(journal.contains(op), "the write is in the same journal the CLI uses");
    assert!(!journal.contains(&token('M')), "the token never reaches the journal");
}

#[test]
fn bad_input_is_a_tool_error_and_the_server_keeps_going() {
    let server = server();
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('N')).code, 0);
    let mut mcp = Mcp::start(&sandbox);

    let (bad_key, is_error) = mcp.call("show", json!({ "key": "not a key" }));
    assert!(is_error);
    assert_eq!(bad_key["error"]["code"], "INVALID_INPUT");

    let (relative, is_error) = mcp.call("attach", json!({ "key": "WEB-12", "files": ["shot.png"] }));
    assert!(is_error);
    assert_eq!(relative["error"]["code"], "INVALID_INPUT");

    let (wrong_status, is_error) =
        mcp.call("move", json!({ "key": "WEB-12", "to": "done", "from": "dev_done" }));
    assert!(is_error, "Done is not reachable through MCP: {wrong_status}");

    let (missing, is_error) = mcp.call("show", json!({ "key": "WEB-404" }));
    assert!(is_error);
    assert_eq!(missing["error"]["code"], "NOT_FOUND");
    assert!(missing["error"]["hint"].is_string());

    mcp.send_line("this is not json");
    assert_eq!(mcp.read()["error"]["code"], -32700);
    assert_eq!(mcp.request("resources/list", json!({}))["error"]["code"], -32601);
    assert_eq!(mcp.request("tools/call", json!({ "name": "delete_item" }))["error"]["code"], -32602);
    assert_eq!(mcp.request("ping", json!({}))["result"], json!({}));
    assert_eq!(server.count("POST", "/"), 0, "no write was sent");
    assert_eq!(mcp.close(), 0);
}

//! Test harness: a mock TaskHub on 127.0.0.1 and a sandboxed `taskhub` process (temporary HOME and XDG dirs).
#![allow(dead_code)]

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/api/v1/fixtures");

pub fn fixture(relative: &str) -> String {
    std::fs::read_to_string(Path::new(FIXTURES).join(relative)).unwrap()
}

/// A valid token for tests (checksum computed like TaskHub's).
pub fn token(c: char) -> String {
    let payload: String = std::iter::repeat_n(c, 43).collect();
    format!("thk_{payload}{}", taskhub_cli::token::token_checksum(&payload))
}

#[derive(Clone, Debug)]
pub struct Recorded {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
    /// Hold the response this long before answering (to trigger timeouts).
    pub delay_ms: u64,
}

impl Reply {
    pub fn json(status: u16, body: impl Into<String>) -> Reply {
        Reply {
            status,
            body: body.into().into_bytes(),
            headers: vec![("Content-Type".into(), "application/json".into())],
            delay_ms: 0,
        }
    }
    pub fn problem(status: u16, code: &str) -> Reply {
        Reply::json(status, fixture(&format!("errors/{code}.json")))
    }
    pub fn bytes(status: u16, body: Vec<u8>) -> Reply {
        Reply { status, body, headers: vec![], delay_ms: 0 }
    }
    pub fn header(mut self, name: &str, value: &str) -> Reply {
        self.headers.push((name.into(), value.into()));
        self
    }
    pub fn delayed(mut self, ms: u64) -> Reply {
        self.delay_ms = ms;
        self
    }
}

type Handler = dyn Fn(&Recorded) -> Reply + Send + Sync;

pub struct MockServer {
    pub origin: String,
    pub requests: Arc<Mutex<Vec<Recorded>>>,
    server: Arc<tiny_http::Server>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    pub fn start(handler: impl Fn(&Recorded) -> Reply + Send + Sync + 'static) -> MockServer {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler: Arc<Handler> = Arc::new(handler);
        let thread = {
            let server = Arc::clone(&server);
            let requests = Arc::clone(&requests);
            std::thread::spawn(move || {
                for mut request in server.incoming_requests() {
                    let mut body = Vec::new();
                    let _ = std::io::Read::read_to_end(request.as_reader(), &mut body);
                    let recorded = Recorded {
                        method: request.method().to_string(),
                        url: request.url().to_owned(),
                        headers: request
                            .headers()
                            .iter()
                            .map(|h| (h.field.to_string(), h.value.to_string()))
                            .collect(),
                        body,
                    };
                    requests.lock().unwrap().push(recorded.clone());
                    let handler = Arc::clone(&handler);
                    std::thread::spawn(move || {
                        let reply = handler(&recorded);
                        if reply.delay_ms > 0 {
                            std::thread::sleep(std::time::Duration::from_millis(reply.delay_ms));
                        }
                        let mut response =
                            tiny_http::Response::from_data(reply.body).with_status_code(reply.status);
                        for (name, value) in reply.headers {
                            response.add_header(
                                tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap(),
                            );
                        }
                        let _ = request.respond(response);
                    });
                }
            })
        };
        MockServer { origin: format!("http://127.0.0.1:{port}"), requests, server, thread: Some(thread) }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }

    pub fn count(&self, method: &str, path_prefix: &str) -> usize {
        self.requests().iter().filter(|r| r.method == method && r.url.starts_with(path_prefix)).count()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A sandbox: its own HOME, config and state directories, and a working directory.
pub struct Sandbox {
    pub dir: tempfile::TempDir,
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn json(&self) -> Value {
        serde_json::from_str(self.stdout.trim())
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {}", self.stdout))
    }
    pub fn error_code(&self) -> String {
        self.json()["error"]["code"].as_str().unwrap_or_default().to_owned()
    }
}

impl Sandbox {
    pub fn new() -> Sandbox {
        let base = std::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let dir = tempfile::Builder::new().prefix("taskhub-test-").tempdir_in(base).unwrap();
        std::fs::create_dir_all(dir.path().join("work")).unwrap();
        Sandbox { dir }
    }

    pub fn home(&self) -> &Path {
        self.dir.path()
    }

    pub fn work(&self) -> PathBuf {
        self.dir.path().join("work")
    }

    pub fn command(&self) -> std::process::Command {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_taskhub"));
        command
            .current_dir(self.work())
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", self.home())
            .env("XDG_CONFIG_HOME", self.home().join(".config"))
            .env("XDG_STATE_HOME", self.home().join(".local/state"))
            .env("TMPDIR", self.home())
            .env("GIT_CONFIG_NOSYSTEM", "1");
        command
    }

    pub fn run(&self, args: &[&str]) -> Run {
        self.run_with(args, None, &[])
    }

    pub fn run_with(&self, args: &[&str], stdin: Option<&str>, env: &[(&str, &str)]) -> Run {
        use std::io::Write;
        use std::process::Stdio;
        let mut command = self.command();
        command.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command.spawn().unwrap();
        {
            let mut input = child.stdin.take().unwrap();
            if let Some(text) = stdin {
                input.write_all(text.as_bytes()).unwrap();
            }
        }
        let output = child.wait_with_output().unwrap();
        Run {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Logs in against `origin` with a test token; the mock must answer `/me`.
    pub fn login(&self, origin: &str, token: &str) -> Run {
        self.run_with(&["auth", "login", "--with-token", "--origin", origin], Some(token), &[])
    }

    /// Every file the CLI wrote under HOME, as text, for leak checks.
    pub fn all_files_text(&self) -> String {
        let mut out = String::new();
        let mut stack = vec![self.home().to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(text) = std::fs::read_to_string(&path) {
                    out.push_str(&format!("== {}\n{text}\n", path.display()));
                }
            }
        }
        out
    }
}

/// A mock that answers the read endpoints from the contract fixtures.
pub fn fixture_server() -> MockServer {
    MockServer::start(|request| {
        let path = request.url.split('?').next().unwrap_or("").to_owned();
        match (request.method.as_str(), path.as_str()) {
            ("GET", "/api/v1/me") => Reply::json(200, fixture("success/getMe.json")),
            ("GET", "/api/v1/projects") => Reply::json(200, fixture("success/listProjects.json")),
            ("GET", "/api/v1/projects/WEB") => Reply::json(200, fixture("success/getProject.json")),
            ("GET", "/api/v1/items") => Reply::json(200, fixture("success/listItems.json")),
            ("GET", "/api/v1/items/next") => Reply::json(200, fixture("success/getNextItem.json")),
            ("GET", "/api/v1/items/WEB-12") => Reply::json(200, fixture("success/getItem.json")),
            ("GET", "/api/v1/items/WEB-12/context") => Reply::json(200, fixture("success/getContext.json")),
            ("GET", "/api/v1/items/WEB-12/activity") => {
                Reply::json(200, fixture("success/listActivity.json"))
            }
            ("GET", "/api/v1/items/WEB-12/comments") => {
                Reply::json(200, fixture("success/listComments.json"))
            }
            ("GET", "/api/v1/items/WEB-12/attachments") => {
                Reply::json(200, fixture("success/listAttachments.json"))
            }
            ("GET", "/api/v1/notifications") => Reply::json(200, fixture("success/listNotifications.json")),
            ("GET", p) if p.starts_with("/api/v1/attachments/") => {
                Reply::bytes(200, fixture("success/downloadAttachment.md").into_bytes())
            }
            _ => Reply::problem(404, "NOT_FOUND"),
        }
    })
}

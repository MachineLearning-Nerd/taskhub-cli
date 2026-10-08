//! C1: credentials, transport, envelope and every read command, through the real binary.

mod common;

use common::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn login_saves_a_private_credential_and_never_prints_the_token() {
    let server = fixture_server();
    let sandbox = Sandbox::new();
    let token = token('L');
    let run = sandbox.login(&server.origin, &format!("{token}\n"));
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let out = run.json();
    assert_eq!(out["ok"], true);
    assert_eq!(out["data"]["me"]["owner"]["username"], "dev.example");
    let path = sandbox.home().join(".config/taskhub/credentials.toml");
    assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(std::fs::metadata(path.parent().unwrap()).unwrap().permissions().mode() & 0o777, 0o700);
    assert!(!run.stdout.contains(&token) && !run.stderr.contains(&token));

    let me = server.requests().into_iter().find(|r| r.url == "/api/v1/me").unwrap();
    assert_eq!(me.header("Authorization"), Some(format!("Bearer {token}").as_str()));
    assert_eq!(me.header("TaskHub-Client"), Some(concat!("taskhub-cli/", env!("CARGO_PKG_VERSION"))));

    let status = sandbox.run(&["auth", "status"]);
    assert_eq!(status.code, 0);
    assert_eq!(status.json()["data"]["source"], "credentialsFile");
    assert!(!status.stdout.contains(&token));

    let logout = sandbox.run(&["auth", "logout"]);
    assert_eq!(logout.json()["data"]["removed"], true);
    let after = sandbox.run(&["auth", "status"]);
    assert_eq!(after.code, 3);
    assert_eq!(after.error_code(), "CREDENTIALS_MISSING");
}

#[test]
fn login_refuses_bad_tokens_and_origins_before_any_request() {
    let server = fixture_server();
    let sandbox = Sandbox::new();
    let bad = sandbox.login(&server.origin, "thk_not-a-real-token");
    assert_eq!((bad.code, bad.error_code().as_str()), (2, "INVALID_INPUT"));
    for origin in ["http://localhost:3000", "http://taskhub.example.com", "https://x.example/api"] {
        let run = sandbox.login(origin, &token('A'));
        assert_eq!((run.code, run.error_code().as_str()), (2, "ORIGIN_NOT_ALLOWED"), "{origin}");
    }
    assert!(server.requests().is_empty());
}

#[test]
fn credentials_with_wide_permissions_are_refused() {
    let server = fixture_server();
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('P')).code, 0);
    let path = sandbox.home().join(".config/taskhub/credentials.toml");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    let run = sandbox.run(&["projects", "list"]);
    assert_eq!((run.code, run.error_code().as_str()), (3, "CREDENTIALS_INSECURE"));
}

#[test]
fn environment_tokens_take_precedence_and_use_the_configured_origin() {
    let server = fixture_server();
    let sandbox = Sandbox::new();
    let config = sandbox.home().join(".config/taskhub");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("config.toml"), format!("origin = \"{}\"\n", server.origin)).unwrap();
    let env_token = token('E');
    let run = sandbox.run_with(&["auth", "status"], None, &[("TASKHUB_TOKEN", &env_token)]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(run.json()["data"]["source"], "environment");
    assert!(run.json()["data"]["warnings"][0].as_str().unwrap().contains("TASKHUB_TOKEN"));

    let file = sandbox.home().join("token");
    std::fs::write(&file, token('F')).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    let file_path = file.to_str().unwrap();
    let insecure = sandbox.run_with(
        &["auth", "status"],
        None,
        &[("TASKHUB_TOKEN_FILE", file_path), ("TASKHUB_TOKEN", &env_token)],
    );
    assert_eq!(insecure.error_code(), "CREDENTIALS_INSECURE");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    let ok = sandbox.run_with(
        &["auth", "status"],
        None,
        &[("TASKHUB_TOKEN_FILE", file_path), ("TASKHUB_TOKEN", &env_token)],
    );
    assert_eq!(ok.json()["data"]["source"], "tokenFile");
    let last = server.requests().into_iter().last().unwrap();
    assert_eq!(last.header("Authorization"), Some(format!("Bearer {}", token('F')).as_str()));
}

fn logged_in() -> (MockServer, Sandbox) {
    let server = fixture_server();
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('R')).code, 0);
    (server, sandbox)
}

#[test]
fn every_read_command_returns_the_envelope() {
    let (server, sandbox) = logged_in();
    for (args, check) in [
        (vec!["show", "web-12"], "/data/item/key"),
        (vec!["items", "get", "WEB-12"], "/data/key"),
        (vec!["items", "activity", "WEB-12"], "/data/0/event"),
        (vec!["items", "list", "--project", "web", "--status", "todo"], "/data/0/key"),
        (vec!["mine"], "/data/0/key"),
        (vec!["comments", "list", "WEB-12"], "/data/0/body"),
        (vec!["attachments", "list", "WEB-12"], "/data/0/filename"),
        (vec!["projects", "list"], "/data/0/key"),
        (vec!["projects", "show", "web"], "/data/limits/title"),
        (vec!["inbox", "--unread"], "/data/0/kind"),
    ] {
        let run = sandbox.run(&args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.stdout);
        let out = run.json();
        assert_eq!(out["ok"], true, "{args:?}");
        assert!(out.pointer(check).is_some(), "{args:?} has no {check}: {out}");
        assert_eq!(out["meta"]["apiVersion"], 1, "{args:?}");
    }
    let show = sandbox.run(&["show", "web-12"]).json();
    assert_eq!(show["meta"]["ref"], serde_json::json!({"key": "WEB-12", "source": "argument"}));
    let context =
        server.requests().into_iter().find(|r| r.url.starts_with("/api/v1/items/WEB-12/context")).unwrap();
    assert_eq!(context.url, "/api/v1/items/WEB-12/context?comments=5&attachments=10");
    let mine = server.requests().into_iter().find(|r| r.url.contains("assignee=me")).unwrap();
    assert!(mine.url.contains("status=todo&status=in_progress"), "{}", mine.url);
    // Reading the inbox is a GET only: nothing is marked read.
    assert!(server.requests().iter().all(|r| r.method == "GET"));
}

#[test]
fn human_output_on_request_and_json_when_piped() {
    let (_server, sandbox) = logged_in();
    let human = sandbox.run(&["show", "WEB-12", "--human"]);
    assert!(human.stdout.starts_with("WEB-12  Restore keyboard focus"), "{}", human.stdout);
    assert!(human.stdout.contains("dev.example (agent)"));
    assert!(human.stdout.contains("Allowed moves: To Do, Dev Done"));
    let piped = sandbox.run(&["show", "WEB-12"]);
    assert!(piped.stdout.starts_with("{\"ok\":true"));
}

#[test]
fn item_keys_come_from_the_branch_or_are_required() {
    let (_server, sandbox) = logged_in();
    let none = sandbox.run(&["show"]);
    assert_eq!((none.code, none.error_code().as_str()), (2, "REF_REQUIRED"));
    assert!(none.json()["error"]["hint"].as_str().unwrap().contains("taskhub show WEB-12"));

    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(sandbox.work())
            .env("HOME", sandbox.home())
            .output()
            .unwrap();
        assert!(status.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    let main = sandbox.run(&["show"]);
    assert_eq!(main.error_code(), "REF_REQUIRED");
    assert!(main.json()["error"]["message"].as_str().unwrap().contains("branch main has none"));
    git(&["switch", "-q", "-c", "feature/web-12-fix-focus"]);
    let run = sandbox.run(&["show"]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(run.json()["meta"]["ref"], serde_json::json!({"key": "WEB-12", "source": "branch"}));
    assert!(run.stderr.contains("Using WEB-12 from branch feature/web-12-fix-focus"));
}

#[test]
fn server_errors_map_to_exit_codes_and_hints() {
    let server = MockServer::start(|request| match request.url.as_str() {
        "/api/v1/me" => Reply::json(200, fixture("success/getMe.json")),
        u if u.starts_with("/api/v1/items/WEB-1/") => Reply::problem(401, "UNAUTHENTICATED"),
        u if u.starts_with("/api/v1/items/WEB-2/") => Reply::problem(404, "NOT_FOUND"),
        u if u.starts_with("/api/v1/items/WEB-3/") => Reply::problem(400, "CLIENT_UPGRADE_REQUIRED"),
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('X')).code, 0);
    let revoked = sandbox.run(&["show", "WEB-1"]);
    assert_eq!((revoked.code, revoked.error_code().as_str()), (3, "UNAUTHENTICATED"));
    let out = revoked.json();
    assert_eq!(out["error"]["httpStatus"], 401);
    assert_eq!(out["error"]["details"]["reason"], "revoked");
    assert!(out["error"]["hint"].as_str().unwrap().contains(&format!("{}/settings/tokens", server.origin)));
    let missing = sandbox.run(&["show", "WEB-2"]);
    assert_eq!((missing.code, missing.error_code().as_str()), (5, "NOT_FOUND"));
    let upgrade = sandbox.run(&["show", "WEB-3"]);
    assert_eq!((upgrade.code, upgrade.error_code().as_str()), (9, "CLIENT_UPGRADE_REQUIRED"));
}

#[test]
fn reads_retry_once_after_rate_limits_and_reject_oversized_bodies() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = std::sync::Arc::clone(&calls);
    let server = MockServer::start(move |request| match request.url.split('?').next().unwrap() {
        "/api/v1/me" => Reply::json(200, fixture("success/getMe.json")),
        "/api/v1/projects" => {
            if counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                Reply::problem(429, "RATE_LIMITED").header("Retry-After", "1")
            } else {
                Reply::json(200, fixture("success/listProjects.json"))
            }
        }
        "/api/v1/items" => Reply::json(200, format!("{{\"data\":\"{}\"}}", "x".repeat(2 * 1024 * 1024 + 10))),
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('Y')).code, 0);
    let run = sandbox.run(&["projects", "list"]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(server.count("GET", "/api/v1/projects"), 2);
    let big = sandbox.run(&["items", "list"]);
    assert_eq!((big.code, big.error_code().as_str()), (9, "PROTOCOL_ERROR"));
}

#[test]
fn downloads_never_replace_files_without_force_and_cap_the_size() {
    let big = vec![b'a'; 4 * 1024 * 1024 + 1];
    let server = MockServer::start(move |request| match request.url.as_str() {
        "/api/v1/me" => Reply::json(200, fixture("success/getMe.json")),
        "/api/v1/attachments/00000000-0000-4000-8000-000000000004" => {
            Reply::bytes(200, fixture("success/downloadAttachment.md").into_bytes())
        }
        "/api/v1/attachments/00000000-0000-4000-8000-000000000009" => Reply::bytes(200, big.clone()),
        _ => Reply::problem(404, "NOT_FOUND"),
    });
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.login(&server.origin, &token('D')).code, 0);
    let id = "00000000-0000-4000-8000-000000000004";
    let ok = sandbox.run(&["attachments", "download", id, "--output", "evidence.md"]);
    assert_eq!(ok.code, 0, "{}", ok.stdout);
    let saved = sandbox.work().join("evidence.md");
    assert_eq!(std::fs::read_to_string(&saved).unwrap(), fixture("success/downloadAttachment.md"));

    std::fs::write(&saved, "mine").unwrap();
    let refused = sandbox.run(&["attachments", "download", id, "--output", "evidence.md"]);
    assert_eq!((refused.code, refused.error_code().as_str()), (2, "OUTPUT_EXISTS"));
    assert_eq!(std::fs::read_to_string(&saved).unwrap(), "mine");
    let forced = sandbox.run(&["attachments", "download", id, "--output", "evidence.md", "--force"]);
    assert_eq!(forced.code, 0);
    assert_eq!(std::fs::read_to_string(&saved).unwrap(), fixture("success/downloadAttachment.md"));

    std::os::unix::fs::symlink("/etc/hostname", sandbox.work().join("link.md")).unwrap();
    let symlink = sandbox.run(&["attachments", "download", id, "--output", "link.md"]);
    assert_eq!(symlink.error_code(), "OUTPUT_EXISTS");

    let too_big = sandbox.run(&[
        "attachments",
        "download",
        "00000000-0000-4000-8000-000000000009",
        "--output",
        "big.bin",
    ]);
    assert_eq!((too_big.code, too_big.error_code().as_str()), (9, "PROTOCOL_ERROR"));
    assert!(!sandbox.work().join("big.bin").exists());
    let leftovers: Vec<_> = std::fs::read_dir(sandbox.work())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".part"))
        .collect();
    assert!(leftovers.is_empty(), "partial files left behind");
}

#[test]
fn usage_errors_are_json_when_piped_and_help_works_offline() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&["items", "list", "--status", "blocked"]);
    assert_eq!((run.code, run.error_code().as_str()), (2, "INVALID_INPUT"));
    let help = sandbox.run(&["--help"]);
    assert_eq!(help.code, 0);
    assert!(help.stdout.contains("next"));
    let version = sandbox.run(&["version"]);
    assert_eq!(version.json()["data"]["apiVersion"], 1);
}

#[test]
fn the_token_never_appears_outside_the_credentials_file() {
    let (_server, sandbox) = logged_in();
    let secret = token('R');
    for args in [vec!["show", "WEB-12"], vec!["auth", "status"], vec!["show", "WEB-99"], vec!["inbox"]] {
        let run = sandbox.run(&args);
        assert!(!run.stdout.contains(&secret) && !run.stderr.contains(&secret), "{args:?}");
    }
    let files = sandbox.all_files_text();
    let occurrences = files.matches(&secret).count();
    assert_eq!(occurrences, 1, "the token must be stored once, in credentials.toml only:\n{files}");
}

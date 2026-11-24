//! `agents owner-decision` against a loopback mock Slack Lists API.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const SCHEMA: &str = include_str!("data/owner-decision/list_schema.json");
const ITEMS: &str = include_str!("data/owner-decision/list_items.json");

struct MockServer {
    base: String,
    _handle: JoinHandle<()>,
}

impl MockServer {
    fn start(responses: HashMap<String, String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
        listener.set_nonblocking(true).ok();
        let port = listener.local_addr().unwrap().port();
        let map = Arc::new(Mutex::new(responses));
        let handle = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            while std::time::Instant::now() < deadline {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 4096];
                    loop {
                        match stream.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => {
                                buf.extend_from_slice(&chunk[..n]);
                                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let req = String::from_utf8_lossy(&buf);
                    let path = req
                        .lines()
                        .next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .unwrap_or("")
                        .trim_start_matches('/');
                    let body =
                        map.lock().unwrap().get(path).cloned().unwrap_or_else(|| {
                            r#"{"ok":false,"error":"unknown_method"}"#.to_string()
                        });
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes());
                } else {
                    thread::sleep(Duration::from_millis(5));
                }
            }
        });
        Self {
            base: format!("http://127.0.0.1:{port}/"),
            _handle: handle,
        }
    }
}

fn write_config(home: &Path, list_id: &str) {
    let dir = home.join(".config/agents");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("config.toml"),
        format!(
            r#"[owner_decision]
list_id = "{list_id}"
"#
        ),
    )
    .unwrap();
}

fn agents(home: &Path, api_base: &str, args: &[&str]) -> std::process::Output {
    agents_env(home, api_base, args, &[])
}

fn agents_env(
    home: &Path,
    api_base: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_agents"))
        .args(args)
        .env_remove("OWNER_DECISION_SESSION")
        .env_remove("AGENTS_OWNER_DECISION_SESSION")
        .envs(envs.iter().copied())
        .env("HOME", home)
        .env("AGENTS_CONFIG", home.join(".config/agents/config.toml"))
        .env(
            "OWNER_DECISION_SLACK_TOKEN",
            "example-bearer-token-for-tests",
        )
        .env("AGENTS_SLACK_API_BASE", api_base)
        .env("AGENTS_UPDATE_CHECK", "off")
        .env_remove("AGENTS_HOME")
        .output()
        .expect("agents")
}

fn ten_open_items() -> String {
    let mut items = serde_json::Value::Array(Vec::new());
    for i in 0..10 {
        items.as_array_mut().unwrap().push(serde_json::json!({
            "id": format!("RecOpen{i:03}"),
            "fields": [
                { "column_id": "ColStatus", "select": [{ "label": "Open" }] },
                { "column_id": "ColSession", "text": format!("other-{i}") }
            ]
        }));
    }
    let schema: serde_json::Value = serde_json::from_str(SCHEMA).unwrap();
    serde_json::json!({
        "ok": true,
        "schema": schema["schema"],
        "items": items
    })
    .to_string()
}

fn session_items(rows: &[(&str, &str, &str)]) -> String {
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|(id, status, session)| {
            serde_json::json!({
                "id": id,
                "fields": [
                    { "column_id": "ColDecision", "text": format!("Decision {id}") },
                    { "column_id": "ColStatus", "select": [{ "label": status }] },
                    { "column_id": "ColSession", "text": session },
                    { "column_id": "ColAnswer", "select": [{ "label": "Approve" }] },
                    { "column_id": "ColChecked", "text": "https://example.com/evidence" }
                ]
            })
        })
        .collect();
    let schema: serde_json::Value = serde_json::from_str(SCHEMA).unwrap();
    serde_json::json!({ "ok": true, "schema": schema["schema"], "items": items }).to_string()
}

fn mock_map(open_items: &str) -> HashMap<String, String> {
    let create = r#"{"ok":true,"item":{"id":"RecCreated999"}}"#.to_string();
    let info = r#"{"ok":true,"record":{"id":"RecExample001","fields":[{"column_id":"ColDecision","text":"Example decision"},{"column_id":"ColStatus","select":[{"label":"Open"}]},{"column_id":"ColSession","text":"sess-mock"},{"column_id":"ColAnswer","select":[{"label":"Approve"}]},{"column_id":"ColChecked","text":"https://example.com/evidence"}]}}"#.to_string();
    HashMap::from([
        ("slackLists.items.list".into(), open_items.into()),
        ("slackLists.items.create".into(), create),
        ("slackLists.items.info".into(), info),
        ("slackLists.items.update".into(), SCHEMA.to_string()),
    ])
}

#[test]
fn ask_refuses_missing_link_or_why() {
    let server = MockServer::start(mock_map(ITEMS));
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let out = agents(
        home.path(),
        &server.base,
        &[
            "owner-decision",
            "ask",
            "--decision",
            "Ship",
            "--checked",
            "no url",
            "--why-you",
            "owner policy",
        ],
    );
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("http"));
}

#[test]
fn ask_refuses_when_ten_open_rows() {
    let server = MockServer::start(mock_map(&ten_open_items()));
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let out = agents(
        home.path(),
        &server.base,
        &[
            "owner-decision",
            "ask",
            "--decision",
            "Ship",
            "--checked",
            "https://example.com/evidence",
            "--why-you",
            "owner policy",
        ],
    );
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("10 open rows"));
}

#[test]
fn ask_read_close_happy_path() {
    let server = MockServer::start(mock_map(ITEMS));
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let ask = agents(
        home.path(),
        &server.base,
        &[
            "owner-decision",
            "ask",
            "--decision",
            "Ship",
            "--checked",
            "https://example.com/evidence",
            "--why-you",
            "owner policy",
        ],
    );
    assert!(
        ask.status.success(),
        "{}",
        String::from_utf8_lossy(&ask.stderr)
    );
    assert!(String::from_utf8_lossy(&ask.stdout).contains("RecCreated999"));

    let read = agents(
        home.path(),
        &server.base,
        &[
            "owner-decision",
            "read",
            "--record-id",
            "RecExample001",
            "--json",
        ],
    );
    assert!(read.status.success());
    let doc: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&read.stdout)).unwrap();
    assert_eq!(doc["answer"], "Approve");
    assert!(doc.get("comments").is_none());

    let close = agents(
        home.path(),
        &server.base,
        &[
            "owner-decision",
            "close",
            "--record-id",
            "RecExample001",
            "--proof-link",
            "https://example.com/proof",
        ],
    );
    assert!(
        close.status.success(),
        "{}",
        String::from_utf8_lossy(&close.stderr)
    );
    assert!(String::from_utf8_lossy(&close.stdout).contains("Done"));
}

#[test]
fn read_and_close_refuse_a_session_shared_by_two_open_rows() {
    let items = session_items(&[
        ("RecDone001", "Done", "sess-shared"),
        ("RecOpenA", "Open", "sess-shared"),
        ("RecOpenB", "Open", "sess-shared"),
    ]);
    let server = MockServer::start(mock_map(&items));
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    for args in [
        &["owner-decision", "read", "--session", "sess-shared"][..],
        &[
            "owner-decision",
            "close",
            "--session",
            "sess-shared",
            "--proof-link",
            "https://example.com/proof",
        ][..],
    ] {
        let out = agents(home.path(), &server.base, args);
        assert!(
            !out.status.success(),
            "{args:?} picked a row: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("RecOpenA") && err.contains("RecOpenB"),
            "{err}"
        );
    }
}

#[test]
fn read_and_close_by_session_skip_done_rows() {
    let items = session_items(&[
        ("RecDone001", "Done", "sess-shared"),
        ("RecOpenA", "Open", "sess-shared"),
    ]);
    let server = MockServer::start(mock_map(&items));
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let read = agents(
        home.path(),
        &server.base,
        &[
            "owner-decision",
            "read",
            "--session",
            "sess-shared",
            "--json",
        ],
    );
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    let doc: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&read.stdout)).unwrap();
    assert_eq!(doc["record_id"], "RecOpenA");

    let close = agents(
        home.path(),
        &server.base,
        &[
            "owner-decision",
            "close",
            "--session",
            "sess-shared",
            "--proof-link",
            "https://example.com/proof",
        ],
    );
    assert!(
        close.status.success(),
        "{}",
        String::from_utf8_lossy(&close.stderr)
    );
    assert!(String::from_utf8_lossy(&close.stdout).contains("record_id: RecOpenA"));
}

#[test]
fn empty_session_flag_is_treated_as_absent() {
    let items = session_items(&[("RecEmpty", "Open", ""), ("RecEnv", "Open", "sess-env")]);
    let server = MockServer::start(mock_map(&items));
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let read = agents_env(
        home.path(),
        &server.base,
        &["owner-decision", "read", "--session", "", "--json"],
        &[("OWNER_DECISION_SESSION", "sess-env")],
    );
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    let doc: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&read.stdout)).unwrap();
    assert_eq!(doc["record_id"], "RecEnv");
}

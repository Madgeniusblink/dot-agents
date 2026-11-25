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

/// One request the mock received: the API method and the JSON body.
type Request = (String, serde_json::Value);

struct MockServer {
    base: String,
    requests: Arc<Mutex<Vec<Request>>>,
    _handle: JoinHandle<()>,
}

impl MockServer {
    fn start(responses: HashMap<String, String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
        listener.set_nonblocking(true).ok();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            while std::time::Instant::now() < deadline {
                if let Ok((mut stream, _)) = listener.accept() {
                    // An accepted socket inherits non-blocking mode on macOS and
                    // BSD; a read would then return before the request arrives.
                    stream.set_nonblocking(false).ok();
                    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
                    let (method, body) = read_request(&mut stream);
                    let reply = responses
                        .get(&method)
                        .cloned()
                        .unwrap_or_else(|| r#"{"ok":false,"error":"unknown_method"}"#.to_string());
                    seen.lock().unwrap().push((method, body));
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        reply.len(),
                        reply
                    );
                    let _ = stream.write_all(resp.as_bytes());
                } else {
                    thread::sleep(Duration::from_millis(5));
                }
            }
        });
        Self {
            base: format!("http://127.0.0.1:{port}/"),
            requests,
            _handle: handle,
        }
    }

    /// The bodies the CLI sent to one API method, in order.
    fn bodies(&self, method: &str) -> Vec<serde_json::Value> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, _)| m == method)
            .map(|(_, b)| b.clone())
            .collect()
    }
}

/// Read the whole request, body included. Closing a socket with unread
/// bytes can reset the connection before the client reads the reply.
fn read_request(stream: &mut std::net::TcpStream) -> (String, serde_json::Value) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return (String::new(), serde_json::Value::Null),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let length = head
        .lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while buf.len() < head_end + length {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let method = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("")
        .trim_start_matches('/')
        .to_string();
    let body = serde_json::from_slice(&buf[head_end..]).unwrap_or(serde_json::Value::Null);
    (method, body)
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

/// An `items.info` reply for `RecExample001` with no Answer set.
fn unanswered_info() -> String {
    r#"{"ok":true,"record":{"id":"RecExample001","fields":[{"column_id":"ColDecision","text":"Example decision"},{"column_id":"ColStatus","select":[{"label":"Open"}]},{"column_id":"ColSession","text":"sess-mock"},{"column_id":"ColChecked","text":"https://example.com/evidence"}]}}"#.to_string()
}

fn cell<'a>(cells: &'a serde_json::Value, column: &str) -> &'a serde_json::Value {
    cells
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["column_id"] == column)
        .unwrap_or_else(|| panic!("no {column} cell in {cells}"))
}

#[test]
fn ask_refuses_an_empty_why_you() {
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
            "https://example.com/evidence",
            "--why-you",
            "  ",
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("why you"));
    assert!(server.bodies("slackLists.items.create").is_empty());
}

#[test]
fn read_reports_a_row_with_no_answer_as_pending() {
    let mut map = mock_map(ITEMS);
    map.insert("slackLists.items.info".into(), unanswered_info());
    let server = MockServer::start(map);
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let args = ["owner-decision", "read", "--record-id", "RecExample001"];
    let text = agents(home.path(), &server.base, &args);
    assert!(text.status.success());
    assert!(String::from_utf8_lossy(&text.stdout).contains("answer: (pending)"));
    let json = agents(
        home.path(),
        &server.base,
        &[&args[..], &["--json"]].concat(),
    );
    assert!(json.status.success());
    let doc: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(doc["pending"], true);
    assert!(doc["answer"].is_null());
}

#[test]
fn close_refuses_a_row_with_an_empty_answer() {
    let mut map = mock_map(ITEMS);
    map.insert("slackLists.items.info".into(), unanswered_info());
    let server = MockServer::start(map);
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let out = agents(
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
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Answer is still empty"));
    assert!(server.bodies("slackLists.items.update").is_empty());
}

#[test]
fn ask_sends_every_field_to_create() {
    let server = MockServer::start(mock_map(ITEMS));
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let empty_path = tempfile::tempdir().unwrap();
    let out = agents_env(
        home.path(),
        &server.base,
        &[
            "owner-decision",
            "ask",
            "--decision",
            "Ship",
            "--recommended",
            "Approve",
            "--checked",
            "https://example.com/evidence",
            "--why-you",
            "owner policy",
            "--project",
            "EX-1",
            "--session",
            "sess-ask",
        ],
        // No `date` executable on PATH, as on Windows.
        &[("PATH", &empty_path.path().to_string_lossy())],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bodies = server.bodies("slackLists.items.create");
    assert_eq!(bodies.len(), 1);
    let body = &bodies[0];
    assert_eq!(body["list_id"], "F000EXAMPLE");
    let fields = &body["initial_fields"];
    assert_eq!(fields.as_array().unwrap().len(), 8, "{fields}");
    assert_eq!(cell(fields, "ColDecision")["text"], "Ship");
    assert_eq!(cell(fields, "ColRecommended")["text"], "Approve");
    assert_eq!(
        cell(fields, "ColChecked")["text"],
        "https://example.com/evidence"
    );
    assert_eq!(cell(fields, "ColWhy")["text"], "owner policy");
    assert_eq!(cell(fields, "ColProject")["text"], "EX-1");
    assert_eq!(cell(fields, "ColSession")["text"], "sess-ask");
    assert_eq!(
        cell(fields, "ColStatus")["select"],
        serde_json::json!([{ "label": "Open" }])
    );
    let asked = cell(fields, "ColAsked")["date"]
        .as_str()
        .unwrap()
        .to_string();
    let b = asked.as_bytes();
    assert!(
        b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()),
        "Asked {asked} is not YYYY-MM-DD"
    );
    assert!(asked.as_str() >= "2026-01-01", "Asked {asked} is not today");
}

#[test]
fn close_sends_done_and_merged_checked_to_update() {
    let server = MockServer::start(mock_map(ITEMS));
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let out = agents(
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
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bodies = server.bodies("slackLists.items.update");
    assert_eq!(bodies.len(), 1);
    let body = &bodies[0];
    assert_eq!(body["list_id"], "F000EXAMPLE");
    let cells = &body["cells"];
    assert_eq!(cells.as_array().unwrap().len(), 2, "{cells}");
    let status = cell(cells, "ColStatus");
    assert_eq!(status["row_id"], "RecExample001");
    assert_eq!(status["select"], serde_json::json!([{ "label": "Done" }]));
    let checked = cell(cells, "ColChecked");
    assert_eq!(checked["row_id"], "RecExample001");
    assert_eq!(
        checked["text"],
        "https://example.com/evidence\nproof: https://example.com/proof"
    );
}

#[test]
fn read_json_gives_null_for_an_empty_session_and_status() {
    let info = r#"{"ok":true,"record":{"id":"RecExample001","fields":[{"column_id":"ColDecision","text":"Example decision"},{"column_id":"ColSession","text":""}]}}"#;
    let mut map = mock_map(ITEMS);
    map.insert("slackLists.items.info".into(), info.to_string());
    let server = MockServer::start(map);
    let home = tempfile::tempdir().unwrap();
    write_config(home.path(), "F000EXAMPLE");
    let out = agents(
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
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(doc["session"].is_null(), "{doc}");
    assert!(doc["status"].is_null(), "{doc}");
    assert_eq!(doc["decision"], "Example decision");
}

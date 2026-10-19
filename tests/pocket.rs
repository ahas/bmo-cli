//! The tool host's catalogue from a shell (docs/boundary.md, 「bmo-pocket」): listed, described, called and
//! its site tools read on a stand-in control socket; the rest not served.

mod support;

use serde_json::{Value, json};
use support::stub::Stub;
use support::{Bench, errors, lines};

// The pocket's commands no route serves today.
const NOT_SERVED: [&str; 3] = ["tool probe", "tool traffic", "tool unread-pages"];

fn entry(name: &str, description: &str, placed: Option<&str>) -> Value {
  json!({
    "name": name,
    "readOnly": true,
    "reachedBy": "call",
    "capabilities": ["room.tools"],
    "grant": null,
    "system": false,
    "resident": true,
    "placed": placed,
    "description": description,
  })
}

// A bench whose control socket a stand-in host answers on.
fn host(answers: &[(&str, u16, Value)]) -> (Bench, Stub, (&'static str, String)) {
  let bench = Bench::new();
  let socket = bench.path().join("host.sock");
  let stub = Stub::answering_on(&socket, answers);
  let named = ("BMO_MCP_HOST_SOCKET", socket.to_str().unwrap().to_owned());

  (bench, stub, named)
}

#[test]
fn the_catalogue_is_listed_a_line_an_entry_narrowed_by_the_words_given() {
  let catalogue = json!([
    entry("bmo/todo_show", "One item whole. Read as a person reads it.", Some("todo")),
    entry("web/fetch", "A page read.", None),
  ]);
  let (bench, _stub, socket) = host(&[("/catalogue", 200, catalogue)]);

  let every = bench.bmo_given(&["tool", "list"], "", &[(socket.0, &socket.1)]);
  let narrowed = bench.bmo_given(&["tool", "list", "PAGE"], "", &[(socket.0, &socket.1)]);

  assert_eq!(
    lines(&every),
    [
      "bmo/todo_show · call · tools/todo: One item whole.",
      "web/fetch · call · linked: A page read."
    ]
  );
  assert_eq!(lines(&narrowed), ["web/fetch · call · linked: A page read."]);
}

#[test]
fn an_entry_is_described_or_refused_where_the_host_serves_none() {
  let mut described = entry("bmo/todo_show", "One item whole.", Some("todo"));
  described["described"] = json!("todo_show: one item whole, as a person reads it");
  let (bench, _stub, socket) = host(&[("/catalogue/bmo/todo_show", 200, described)]);

  let output = bench.bmo_given(&["tool", "describe", "bmo/todo_show"], "", &[(socket.0, &socket.1)]);
  let none = bench.bmo_given(&["tool", "describe", "bmo/none"], "", &[(socket.0, &socket.1)]);

  assert_eq!(
    lines(&output),
    [
      "todo_show: one item whole, as a person reads it",
      "Capabilities: [\"room.tools\"]",
      "Grant: none",
      "bmo/todo_show · call · tools/todo: One item whole.",
    ]
  );
  assert_eq!(
    (none.status.code(), errors(&none)),
    (Some(1), "the tool host serves no bmo/none\n".to_owned())
  );
}

#[test]
fn an_entry_is_called_under_a_record_holding_its_own_capabilities_as_the_owner() {
  let (bench, stub, socket) = host(&[
    ("/catalogue/bmo/todo_show", 200, entry("bmo/todo_show", "One item.", Some("todo"))),
    (
      "/calls",
      200,
      json!({ "content": [{ "type": "text", "text": "item 7" }], "isError": false }),
    ),
  ]);
  let data = bench.path().join("data");

  let output = bench.bmo_given(
    &["tool", "call", "bmo/todo_show", "{\"item\":7}"],
    "",
    &[(socket.0, &socket.1), ("BMO_DATA_DIR", data.to_str().unwrap())],
  );
  let asked = stub.asked();

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(lines(&output), ["item 7"]);
  assert_eq!((asked[1].method.as_str(), asked[1].path.as_str()), ("POST", "/calls"));
  assert_eq!(
    asked[1].body,
    json!({
      "session": {
        "line": "shell",
        "capabilities": ["room.tools"],
        "dataRoot": data,
        "runVariables": { "BMO_TURN_PRIVILEGED": "1" },
      },
      "tool": "bmo/todo_show",
      "arguments": { "item": 7 },
    })
  );
}

#[test]
fn a_call_the_tool_failed_ends_as_failed() {
  let (bench, _stub, socket) = host(&[
    ("/catalogue/bmo/todo_show", 200, entry("bmo/todo_show", "One item.", None)),
    (
      "/calls",
      200,
      json!({ "content": [{ "type": "text", "text": "no such item" }], "isError": true }),
    ),
  ]);

  let output = bench.bmo_given(&["tool", "call", "bmo/todo_show"], "", &[(socket.0, &socket.1)]);

  assert_eq!(output.status.code(), Some(1));
  assert_eq!(lines(&output), ["no such item"]);
}

#[test]
fn the_site_tools_are_read_off_the_web_namespaces_facts() {
  let facts = json!({
    "digest": "d",
    "sites": [{ "name": "web/site_a", "what": "its pages", "site": "a.example", "since": "2026-01-01", "arguments": ["url"] }],
  });
  let (bench, _stub, socket) = host(&[("/facts/web", 200, facts)]);

  let output = bench.bmo_given(&["tool", "sites"], "", &[(socket.0, &socket.1)]);

  assert_eq!(
    lines(&output),
    [
      "1 site tools",
      "",
      "web/site_a: its pages",
      "  site: a.example",
      "  since: 2026-01-01",
      "  arguments: url"
    ]
  );
}

#[test]
fn a_host_that_does_not_answer_is_said_to_and_ends_as_failed() {
  let bench = Bench::new();
  let socket = bench.path().join("absent.sock");

  let output = bench.bmo_given(&["tool", "list"], "", &[("BMO_MCP_HOST_SOCKET", socket.to_str().unwrap())]);

  assert_eq!(output.status.code(), Some(1));
  assert!(errors(&output).starts_with(&format!("mcp-host does not answer on {}: ", socket.display())));
}

#[test]
fn the_pockets_commands_no_route_serves_are_not_served() {
  let bench = Bench::new();

  for command in NOT_SERVED {
    let output = bench.bmo(&command.split(' ').collect::<Vec<_>>());

    assert_eq!(output.status.code(), Some(2), "{command}");
    assert_eq!(
      errors(&output),
      format!("bmo: `{command}` is not served yet: no installed unit carries it\n")
    );
  }
}

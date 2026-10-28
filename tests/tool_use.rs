//! The tool verb (docs/api.md, 「The tool verb」): a tool resolved through a stand-in toolbook and called with
//! `call` or `act`, its answer printed or refused, and an asynchronous call kept, started as a unit and ended
//! with a wake to a stand-in receiver.

mod support;

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::{Value, json};
use support::stand_ins::{self, INSTRUCTION, RECEIVER_TOKEN, SESSION_TOKEN, StandIn};
use support::{Bench, errors, lines};

// A bench whose `PATH` holds a `systemd-run` keeping the words it was given and a `systemctl` saying no unit
// is active, with a stand-in tool host and receiver, and a workspace the caller runs in.
struct Stage {
  bench: Bench,
  host: StandIn,
  receiver: StandIn,
  workspace: PathBuf,
}

impl Stage {
  // A host that serves no calls face, as a host older than the face does: a call asked of it is kept by the
  // caller itself, in its workspace.
  fn new() -> Self {
    Self::standing(stand_ins::tool_host())
  }

  // A host whose calls face answers, as the deployment's own does: the host keeps every call asked of it.
  fn with_calls() -> Self {
    Self::standing(stand_ins::tool_host_with_calls())
  }

  fn standing(host: StandIn) -> Self {
    let bench = Bench::new();
    let started = bench.path().join("systemd-run.words");
    let workspace = bench.path().join("workspace");

    std::fs::create_dir(&workspace).unwrap();
    bench.command("systemd-run", &format!("printf '%s\\n' \"$@\" > '{}'", started.display()));
    bench.command("systemctl", "exit 3");

    Self {
      bench,
      host,
      receiver: stand_ins::receiver(),
      workspace,
    }
  }

  fn root(&self) -> PathBuf {
    self.bench.path().join("data")
  }

  // The folder a caller's own records stand in, in its workspace.
  fn records(&self) -> PathBuf {
    self.workspace.join(".bmo/use")
  }

  // `bmo` run with `words` as a turn of agent-a in room -5 would run it.
  fn bmo(&self, words: &[&str]) -> Output {
    self.bmo_with(words, SESSION_TOKEN)
  }

  fn bmo_with(&self, words: &[&str], token: &str) -> Output {
    let root = self.root();
    let port = self.host.port.to_string();
    let receiver = self.receiver.address();

    let mut command = self.bench.bmo_command(words);

    command.current_dir(&self.workspace).envs([
      ("BMO_DATA_DIR", root.to_str().unwrap()),
      ("BMO_MCP_TOKEN", token),
      ("BMO_MCP_HOST_PORT", &port),
      ("RECEIVER_ADDRESS", &receiver),
      ("RECEIVER_TOKEN", RECEIVER_TOKEN),
      ("BMO_TURN_CHAT", "-5"),
      ("BMO_TURN_AGENT", "agent-a"),
    ]);

    support::run(command, None)
  }

  // `bmo` run with `words` and the variables `variables`, in the workspace.
  fn bmo_env(&self, words: &[&str], variables: &[(&str, &str)]) -> Output {
    let root = self.root();
    let port = self.host.port.to_string();
    let receiver = self.receiver.address();
    let mut command = self.bench.bmo_command(words);

    command
      .current_dir(&self.workspace)
      .envs([
        ("BMO_DATA_DIR", root.to_str().unwrap()),
        ("BMO_MCP_TOKEN", stand_ins::SESSION_TOKEN),
        ("BMO_MCP_HOST_PORT", &port),
        ("RECEIVER_ADDRESS", &receiver),
        ("RECEIVER_TOKEN", stand_ins::RECEIVER_TOKEN),
        ("BMO_TURN_CHAT", "-5"),
        ("BMO_TURN_AGENT", "agent-a"),
      ])
      .envs(variables.iter().copied());

    support::run(command, None)
  }

  // The toolbook tools the host was asked, with the inner tool each named.
  fn asked(&self) -> Vec<(String, String)> {
    self
      .host
      .sent()
      .iter()
      .filter(|sent| sent.path == "/mcp/toolbook")
      .map(|sent| {
        let params = &sent.body["params"];

        (
          params["name"].as_str().unwrap().to_owned(),
          params["arguments"]["tool"].as_str().unwrap().to_owned(),
        )
      })
      .collect()
  }

  // The words `systemd-run` was given.
  fn unit_words(&self) -> Vec<String> {
    std::fs::read_to_string(self.bench.path().join("systemd-run.words"))
      .unwrap()
      .lines()
      .map(str::to_owned)
      .collect()
  }

  // An asynchronous call started, its id answered.
  fn started(&self, words: &[&str]) -> String {
    let output = self.bmo(words);

    assert!(output.status.success(), "{}", errors(&output));

    let answer: Value = serde_json::from_slice(&output.stdout).unwrap();

    answer["id"].as_str().unwrap().to_owned()
  }
}

fn read(path: &Path) -> Value {
  serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn a_sync_call_of_a_read_reaches_it_with_call_and_prints_its_text() {
  let stage = Stage::new();

  let output = stage.bmo(&["use", "web/read", "--arg", "name=12", "--arg", "count=3"]);

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(lines(&output), [r#"read {"count":3,"name":"12"}"#, "second line"]);
  assert_eq!(
    stage.asked(),
    [
      ("describe".to_owned(), "web/read".to_owned()),
      ("call".to_owned(), "web/read".to_owned())
    ]
  );
  assert!(stage.host.sent().iter().all(|sent| sent.path == "/mcp/toolbook"));
}

#[test]
fn a_sync_call_of_an_act_reaches_it_with_act() {
  let stage = Stage::new();

  let output =
    stage.bmo(&["use", "bmo/say", "--sync", "--json", r#"{"text":"hi","to":1}"#, "--arg", "text=5"]);

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(lines(&output), [r#"acted {"text":"5","to":1}"#]);
  assert_eq!(stage.asked()[1], ("act".to_owned(), "bmo/say".to_owned()));
}

#[test]
fn structured_content_is_printed_as_json() {
  let stage = Stage::new();

  let output = stage.bmo(&["use", "web/shape"]);

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(serde_json::from_slice::<Value>(&output.stdout).unwrap(), json!({ "shape": "round" }));
}

#[test]
fn a_refused_call_ends_with_1_and_says_the_code() {
  let stage = Stage::new();

  let unknown = stage.bmo(&["use", "web/nope"]);

  assert_eq!(unknown.status.code(), Some(1));
  assert_eq!(
    errors(&unknown),
    "bmo: `web/nope` was refused: refused: ERROR: Tool web/nope not found\n"
  );
  assert!(unknown.stdout.is_empty());

  let wrong_token = stage.bmo_with(&["use", "web/read"], "not-a-session");

  assert_eq!(wrong_token.status.code(), Some(1));
  assert_eq!(
    errors(&wrong_token),
    "bmo: `web/read` was refused: unauthorized: the token names no session\n"
  );
}

#[test]
fn words_or_a_token_that_do_not_read_end_with_2() {
  let stage = Stage::new();

  let no_token = stage.bmo_with(&["use", "web/read"], "");

  assert_eq!(no_token.status.code(), Some(2));
  assert!(errors(&no_token).contains("BMO_MCP_TOKEN"), "{}", errors(&no_token));

  for words in [
    &["use", "web/read", "--arg", "name"][..],
    &["use", "web/read", "--json", "[1]"],
    &["use"],
  ] {
    assert_eq!(stage.bmo(words).status.code(), Some(2), "{words:?}");
  }

  assert_eq!(stage.bmo(&["use", "web/read", "--memory", "m"]).status.code(), Some(2));
}

#[test]
fn the_help_names_the_verb_and_use_help_its_words() {
  let stage = Stage::new();

  let help = String::from_utf8_lossy(&stage.bmo(&["--help"]).stdout).into_owned();

  assert!(help.contains("use <TOOL>"), "{help}");

  let output = stage.bmo(&["use", "--help"]);
  let usage = String::from_utf8_lossy(&output.stdout).into_owned();

  assert!(output.status.success());
  assert!(usage.contains("Usage: bmo use"), "{usage}");
  assert!(
    [
      "--async",
      "--sync",
      "--arg",
      "--json",
      "--memory",
      "--max-seconds",
      "--tools",
      "--query",
      "--describe",
      "--list",
      "--status"
    ]
    .iter()
    .all(|word| usage.contains(word))
  );
}

#[test]
fn use_is_bmos_own_word_whatever_a_unit_declares() {
  let stage = Stage::new();
  stage.bench.unit("bmo-use", &["use"]);

  let output = stage.bmo(&["use", "web/shape"]);

  assert!(output.status.success(), "{}", errors(&output));
  assert!(!lines(&output).contains(&"bmo-use".to_owned()));
}

#[test]
fn an_async_call_a_host_with_no_calls_face_keeps_in_the_workspace() {
  let stage = Stage::new();

  let output =
    stage.bmo(&["use", "web/read", "--async", "--memory", "tell the owner", "--arg", "name=page"]);

  assert!(output.status.success(), "{}", errors(&output));

  let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
  let id = answer["id"].as_str().unwrap();
  let (named, nonce) = id.rsplit_once('-').unwrap();

  assert_eq!(named, "web-read");
  assert!(nonce.len() == 8 && nonce.chars().all(|digit| digit.is_ascii_hexdigit()), "{id}");
  assert_eq!(answer["unit"], format!("bmo-use-{id}"));

  let place = stage.records().join(id);

  assert_eq!(answer["record"], place.display().to_string());

  let request = read(&place.join("request.json"));

  assert_eq!(request["tool"], "web/read");
  assert_eq!(request["reachedBy"], "call");
  assert_eq!(request["arguments"], json!({ "name": "page" }));
  assert_eq!(request["memory"], "tell the owner");
  assert_eq!(request["room"], -5);
  assert_eq!(request["agent"], "agent-a");
  assert_eq!(request["via"], Value::Null);
  assert_eq!(request["maxSeconds"], 86400);
  assert!(!place.join("end.json").exists());
  assert_eq!(stage.asked(), [("describe".to_owned(), "web/read".to_owned())]);

  let exe = env!("CARGO_BIN_EXE_bmo");

  assert_eq!(
    stage.unit_words(),
    [
      "--user".to_owned(),
      format!("--unit=bmo-use-{id}"),
      "--collect".to_owned(),
      "--quiet".to_owned(),
      "--property=RuntimeMaxSec=86400".to_owned(),
      format!("--property=ExecStopPost={exe} use --ended {id}"),
      format!("--setenv=BMO_DATA_DIR={}", stage.root().display()),
      format!("--setenv=BMO_USE_RECORD_DIR={}", stage.records().display()),
      "--setenv=BMO_MCP_TOKEN".to_owned(),
      "--setenv=BMO_MCP_HOST_PORT".to_owned(),
      "--setenv=RECEIVER_ADDRESS".to_owned(),
      "--setenv=RECEIVER_TOKEN".to_owned(),
      exe.to_owned(),
      "use".to_owned(),
      "--run".to_owned(),
      id.to_owned(),
    ]
  );

  let status: Value = serde_json::from_slice(&stage.bmo(&["use", "--status", id]).stdout).unwrap();

  assert_eq!(status["state"], "lost");
  assert_eq!(lines(&stage.bmo(&["use", "--list"])), [format!("{id}  lost  web/read")]);
}

#[test]
fn an_async_call_is_asked_of_the_host_and_kept_there() {
  let stage = Stage::with_calls();

  let output = stage.bmo(&[
    "use",
    "web/read",
    "--async",
    "--memory",
    "tell the owner",
    "--arg",
    "name=page",
    "--max-seconds",
    "90",
  ]);

  assert!(output.status.success(), "{}", errors(&output));

  let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
  let id = answer["id"].as_str().unwrap();

  assert_eq!(id, "web-read-00000007");
  assert_eq!(answer["state"], json!("running"));
  assert_eq!(answer["record"], json!(format!("/kept/use/{id}")));

  let asked: Vec<_> = stage
    .host
    .sent()
    .into_iter()
    .filter(|sent| sent.method == "POST" && sent.path == "/use")
    .collect();

  assert_eq!(asked.len(), 1, "{asked:?}");
  assert_eq!(asked[0].method, "POST");
  assert_eq!(asked[0].body["tool"], json!("web/read"));
  assert_eq!(asked[0].body["arguments"], json!({ "name": "page" }));
  assert_eq!(asked[0].body["memory"], json!("tell the owner"));
  assert_eq!(asked[0].body["room"], json!(-5));
  assert_eq!(asked[0].body["agent"], json!("agent-a"));
  assert_eq!(asked[0].body["maxSeconds"], json!(90));

  // The caller's shell kept nothing: no record under the data root, nothing in its workspace, no unit started.
  assert!(!stage.root().exists());
  assert!(!stage.records().exists());
  assert!(!stage.bench.path().join("systemd-run.words").exists());
}

#[test]
fn a_status_and_a_list_ask_the_host_for_its_calls() {
  let stage = Stage::with_calls();
  let id = stage.started(&["use", "web/read", "--async", "--arg", "name=page"]);

  let status: Value = serde_json::from_slice(&stage.bmo(&["use", "--status", &id]).stdout).unwrap();

  assert_eq!(status["id"], json!(id));
  assert_eq!(status["tool"], json!("web/read"));
  assert_eq!(status["state"], json!("running"));
  assert_eq!(lines(&stage.bmo(&["use", "--list"])), [format!("{id}  running  web/read")]);
  assert!(!stage.records().exists());
}

#[test]
fn a_refused_write_names_the_sandbox_and_the_workspace() {
  let stage = Stage::new();
  let refused = stage.bench.path().join("refused");
  let records = refused.join("use");

  std::fs::create_dir(&refused).unwrap();
  std::fs::set_permissions(&refused, std::fs::Permissions::from_mode(0o500)).unwrap();

  let output =
    stage.bmo_env(&["use", "web/read", "--async"], &[("BMO_USE_RECORD_DIR", records.to_str().unwrap())]);
  let said = errors(&output);

  assert_eq!(output.status.code(), Some(1), "{said}");
  assert!(said.contains("sandbox"), "{said}");
  assert!(said.contains(&stage.workspace.display().to_string()), "{said}");
  assert!(said.contains(&records.display().to_string()), "{said}");
}

#[test]
fn an_unknown_tool_is_refused_before_any_record_is_kept() {
  let stage = Stage::new();

  let output = stage.bmo(&["use", "web/nope", "--async"]);

  assert_eq!(output.status.code(), Some(1));
  assert!(!stage.records().exists());
  assert!(!stage.bench.path().join("systemd-run.words").exists());
}

#[test]
fn a_run_keeps_the_answer_and_wakes_the_agent_with_its_path_and_summary() {
  let stage = Stage::new();
  let id = stage.started(&["--via", "road-1", "use", "web/read", "--async", "--memory", "tell the owner"]);

  let run = stage.bmo(&["use", "--run", &id]);

  assert!(run.status.success(), "{}", errors(&run));

  let place = stage.records().join(&id);
  let answer = place.join("answer.json");

  assert_eq!(read(&answer)["content"][0]["text"], "read {}\nsecond line");

  let woken = stage.receiver.sent();

  assert_eq!(woken.len(), 1);
  assert_eq!((woken[0].method.as_str(), woken[0].path.as_str()), ("POST", "/instructions"));
  assert_eq!(woken[0].bearer, format!("Bearer {RECEIVER_TOKEN}"));
  assert_eq!(
    woken[0].body,
    json!({
      "agentName": "agent-a",
      "room": -5,
      "said": {
        "fact": {
          "use": id,
          "tool": "web/read",
          "state": "done",
          "answer": answer.display().to_string(),
          "summary": "web/read answered: read {}",
          "memory": "tell the owner",
        },
        "order": null,
      },
      "sort": "use-ended",
      "summary": "web/read answered: read {}",
      "kind": null,
      "expiresAt": null,
      "events": [],
      "ridesUntil": null,
      "via": "road-1",
      "run": null,
    })
  );

  let end = read(&place.join("end.json"));

  assert_eq!(end["state"], "done");
  assert_eq!(end["wake"], json!({ "id": INSTRUCTION }));

  let status: Value = serde_json::from_slice(&stage.bmo(&["use", "--status", &id]).stdout).unwrap();

  assert_eq!(status["state"], "done");
  assert_eq!(status["answer"], answer.display().to_string());
  assert_eq!(status["summary"], "web/read answered: read {}");
  assert_eq!(lines(&stage.bmo(&["use", "--list"])), [format!("{id}  done  web/read")]);
}

#[test]
fn a_refused_run_wakes_the_agent_with_the_refusal() {
  let stage = Stage::new();
  let id = stage.started(&["use", "bmo/say", "--async", "--room", "42", "--agent", "agent-b"]);
  let place = stage.records().join(&id);
  let mut request = read(&place.join("request.json"));

  request["tool"] = json!("bmo/gone");
  std::fs::write(place.join("request.json"), request.to_string()).unwrap();

  let run = stage.bmo(&["use", "--run", &id]);

  assert!(run.status.success(), "{}", errors(&run));
  assert_eq!(
    read(&place.join("answer.json")),
    json!({ "code": "refused", "error": "ERROR: Tool bmo/gone not found" })
  );

  let fact = &stage.receiver.sent()[0].body;

  assert_eq!((fact["room"].clone(), fact["agentName"].clone()), (json!(42), json!("agent-b")));
  assert_eq!(fact["said"]["fact"]["state"], "refused");
  assert_eq!(fact["summary"], "bmo/gone was refused: refused: ERROR: Tool bmo/gone not found");
  assert_eq!(read(&place.join("end.json"))["state"], "refused");
}

#[test]
fn a_wake_the_receiver_refuses_is_kept_and_ends_the_run_with_1() {
  let stage = Stage::new();
  let id = stage.started(&["use", "web/read", "--async"]);
  let mut command = stage.bench.bmo_command(&["use", "--run", &id]);

  command
    .current_dir(&stage.workspace)
    .env("BMO_DATA_DIR", stage.root())
    .env("BMO_MCP_TOKEN", SESSION_TOKEN)
    .env("BMO_MCP_HOST_PORT", stage.host.port.to_string())
    .env("RECEIVER_ADDRESS", stage.receiver.address())
    .env("RECEIVER_TOKEN", "wrong");

  let run = support::run(command, None);

  assert_eq!(run.status.code(), Some(1));

  let end = read(&stage.records().join(&id).join("end.json"));

  assert_eq!(end["state"], "done");
  assert!(end["wake"]["error"].as_str().unwrap().contains("unauthorized"), "{end}");
}

#[test]
fn the_receiver_is_read_from_the_data_roots_env_file_where_the_environment_names_none() {
  let stage = Stage::new();
  let id = stage.started(&["use", "web/read", "--async"]);
  let env = stage.root().join("env");

  std::fs::create_dir_all(&env).unwrap();
  std::fs::write(
    env.join("receiver.env"),
    format!("RECEIVER_ADDRESS={}\nRECEIVER_TOKEN={RECEIVER_TOKEN}\n", stage.receiver.address()),
  )
  .unwrap();
  std::fs::write(env.join("pocket.env"), format!("BMO_MCP_HOST_PORT={}\n", stage.host.port)).unwrap();

  let mut command = stage.bench.bmo_command(&["use", "--run", &id]);

  command
    .current_dir(&stage.workspace)
    .env("BMO_DATA_DIR", stage.root())
    .env("BMO_MCP_TOKEN", SESSION_TOKEN);

  let run = support::run(command, None);

  assert!(run.status.success(), "{}", errors(&run));
  assert_eq!(stage.receiver.sent().len(), 1);
}

#[test]
fn a_status_of_no_record_ends_with_2() {
  let stage = Stage::new();

  for id in ["web-read-00000000", "../escape"] {
    assert_eq!(stage.bmo(&["use", "--status", id]).status.code(), Some(2), "{id}");
  }
}

#[test]
fn the_tools_are_listed_one_a_line_and_narrowed_by_a_query() {
  let stage = Stage::new();

  let output = stage.bmo(&["use", "--tools"]);

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(
    lines(&output),
    [
      "bmo/say  act  Says a line.",
      "web/read  call  Reads a page.",
      "web/shape  call  Shapes."
    ]
  );
  assert_eq!(stage.host.sent()[0].body["params"], json!({ "name": "list", "arguments": {} }));

  let narrowed = stage.bmo(&["use", "--tools", "--query", "say"]);

  assert_eq!(lines(&narrowed), ["bmo/say  act  Says a line."]);
  assert_eq!(stage.host.sent()[1].body["params"]["arguments"], json!({ "query": "say" }));

  let none = stage.bmo(&["use", "--tools", "--query", "nothing"]);

  assert!(none.status.success(), "{}", errors(&none));
  assert!(lines(&none).is_empty());
  assert!(errors(&none).contains("no tool is listed for `nothing`"), "{}", errors(&none));
}

#[test]
fn a_tool_described_is_printed_as_the_hosts_object_and_not_called() {
  let stage = Stage::new();

  let output = stage.bmo(&["use", "bmo/say", "--describe"]);

  assert!(output.status.success(), "{}", errors(&output));

  let shape: Value = serde_json::from_slice(&output.stdout).unwrap();

  assert_eq!(shape["reachedBy"], "act");
  assert_eq!(shape["manual"], "Call bmo/say well.");
  assert_eq!(shape["input"]["properties"]["text"]["type"], "string");
  assert_eq!(stage.asked(), [("describe".to_owned(), "bmo/say".to_owned())]);
  assert_eq!(stage.bmo(&["use", "web/nope", "--describe"]).status.code(), Some(1));
  assert_eq!(stage.bmo(&["use", "bmo/say", "--describe", "--async"]).status.code(), Some(2));
}

#[test]
fn a_longest_run_given_is_the_units_and_the_records() {
  let stage = Stage::new();
  let id = stage.started(&["use", "web/read", "--async", "--max-seconds", "90"]);

  assert!(stage.unit_words().contains(&"--property=RuntimeMaxSec=90".to_owned()));
  assert_eq!(read(&stage.records().join(&id).join("request.json"))["maxSeconds"], 90);
  assert_eq!(stage.bmo(&["use", "web/read", "--async", "--max-seconds", "0"]).status.code(), Some(2));
  assert_eq!(stage.bmo(&["use", "web/read", "--max-seconds", "90"]).status.code(), Some(2));
}

#[test]
fn a_unit_that_stopped_before_its_end_wakes_the_agent_with_the_failure() {
  let stage = Stage::new();
  let id = stage.started(&["use", "web/read", "--async", "--memory", "tell the owner"]);
  let place = stage.records().join(&id);
  let mut command = stage.bench.bmo_command(&["use", "--ended", &id]);

  command
    .current_dir(&stage.workspace)
    .env("BMO_DATA_DIR", stage.root())
    .env("RECEIVER_ADDRESS", stage.receiver.address())
    .env("RECEIVER_TOKEN", RECEIVER_TOKEN)
    .env("SERVICE_RESULT", "timeout")
    .env("EXIT_CODE", "killed")
    .env("EXIT_STATUS", "TERM");

  let last_word = support::run(command, None);

  assert!(last_word.status.success(), "{}", errors(&last_word));

  let why = "its unit stopped before the call ended (result timeout, exit killed TERM)";

  assert_eq!(read(&place.join("answer.json")), json!({ "code": "failed", "error": why }));

  let fact = &stage.receiver.sent()[0].body;

  assert_eq!(fact["said"]["fact"]["state"], "failed");
  assert_eq!(fact["said"]["fact"]["memory"], "tell the owner");
  assert_eq!(fact["summary"], format!("web/read failed: {why}"));
  assert_eq!(read(&place.join("end.json"))["state"], "failed");
  assert_eq!(lines(&stage.bmo(&["use", "--list"])), [format!("{id}  failed  web/read")]);
}

#[test]
fn a_units_last_word_after_its_end_does_nothing() {
  let stage = Stage::new();
  let id = stage.started(&["use", "web/read", "--async"]);

  assert!(stage.bmo(&["use", "--run", &id]).status.success());

  let end = read(&stage.records().join(&id).join("end.json"));
  let last_word = stage.bmo(&["use", "--ended", &id]);

  assert!(last_word.status.success(), "{}", errors(&last_word));
  assert_eq!(stage.receiver.sent().len(), 1);
  assert_eq!(read(&stage.records().join(&id).join("end.json")), end);
}

//! The tool verb, `bmo use`: one hosted tool reached through the tool host's toolbook, its answer printed at
//! once or, asked of the tool host to run beyond the request, handed to the agent by a wake (docs/api.md, 「The tool
//! verb」).

pub mod calls;
pub mod ground;
pub mod record;
pub mod toolbook;
pub mod wake;

use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use clap::Parser;
use serde_json::{Value, json};

use self::record::{End, Request};
use self::toolbook::{Refusal, Toolbook};
use self::wake::Pace;

/// The words `bmo use` reads.
#[derive(Parser, Debug)]
#[command(
  name = "bmo use",
  about = "Calls one hosted tool through the tool host's toolbook",
  disable_version_flag = true
)]
pub struct UseLine {
  /// The tool's full name, `<namespace>/<name>`.
  #[arg(value_name = "TOOL", required_unless_present_any = ["list", "status", "run", "tools", "ended", "wake"])]
  pub tool: Option<String>,
  /// Print the tool as the host describes it, as JSON, instead of calling it.
  #[arg(long, conflicts_with_all = ["asynchronous", "sync", "args", "json"])]
  pub describe: bool,
  /// Call the tool and wait for its answer (the default).
  #[arg(long, conflicts_with = "asynchronous")]
  pub sync: bool,
  /// Start the call as a unit of its own and wake the agent once it is done.
  #[arg(long = "async", id = "asynchronous")]
  pub asynchronous: bool,
  /// One argument; a value is typed by the tool's input schema.
  #[arg(long = "arg", value_name = "KEY=VALUE")]
  pub args: Vec<String>,
  /// The arguments as one JSON object, each `--arg` laid over it.
  #[arg(long, value_name = "OBJECT")]
  pub json: Option<String>,
  /// What to hand back with the answer when it comes.
  #[arg(long, value_name = "WORDS", requires = "asynchronous")]
  pub memory: Option<String>,
  /// The room the wake is for, else BMO_TURN_CHAT.
  #[arg(long, value_name = "ROOM", requires = "asynchronous", allow_negative_numbers = true)]
  pub room: Option<i64>,
  /// The agent the wake is for, else BMO_TURN_AGENT.
  #[arg(long, value_name = "NAME", requires = "asynchronous")]
  pub agent: Option<String>,
  /// Read and given no effect: a call runs until its tool answers.
  #[arg(long, value_name = "SECONDS", hide = true)]
  pub max_seconds: Option<u64>,
  /// The tools use reaches, one a line.
  #[arg(long, conflicts_with_all = ["tool", "status", "run", "list", "ended"])]
  pub tools: bool,
  /// Words a listed tool's full name or description holds.
  #[arg(long, value_name = "WORDS", requires = "tools")]
  pub query: Option<String>,
  /// Every record kept, one a line.
  #[arg(long, conflicts_with_all = ["tool", "status", "run"])]
  pub list: bool,
  /// One record, as JSON.
  #[arg(long, value_name = "ID", conflicts_with_all = ["tool", "run"])]
  pub status: Option<String>,
  /// Runs a kept call; the unit's own word.
  #[arg(long, value_name = "ID", hide = true, conflicts_with = "tool")]
  pub run: Option<String>,
  /// Ends a kept call whose unit stopped; the unit's own last word.
  #[arg(long, value_name = "ID", hide = true, conflicts_with_all = ["tool", "run"])]
  pub ended: Option<String>,
  /// Queues the owed wake of a kept call; a wake's own unit's word.
  #[arg(long, value_name = "ID", hide = true, conflicts_with_all = ["tool", "run", "ended"])]
  pub wake: Option<String>,
}

/// Answers `bmo use` given `words`, the road the caller named before the first word with it.
pub fn run(words: &[OsString], via: Option<&str>) -> ExitCode {
  let line = match UseLine::try_parse_from(
    std::iter::once(OsString::from("bmo use")).chain(words.iter().cloned()),
  ) {
    Ok(line) => line,
    Err(why) => why.exit(),
  };
  let root = ground::data_root();
  let records = record::records(&root);

  if line.list {
    return listed(&root, &records);
  }

  if let Some(id) = &line.status {
    return status(&root, &records, id);
  }

  if let Some(id) = &line.run {
    return ran(&root, &records, id);
  }

  if let Some(id) = &line.ended {
    return ended(&root, &records, id);
  }

  if let Some(id) = &line.wake {
    return woken(&root, &records, id);
  }

  if line.tools {
    return tools(&root, line.query.as_deref());
  }

  let tool = line.tool.clone().unwrap_or_default();
  let book = match Toolbook::found(&root) {
    Ok(book) => book,
    Err(why) => return unread(&why),
  };
  let described = match book.describe(&tool) {
    Ok(described) => described,
    Err(refusal) => return refused(&tool, &refusal),
  };

  if line.describe {
    println!("{}", serde_json::to_string_pretty(&described.shape).unwrap_or_default());

    return ExitCode::SUCCESS;
  }

  let arguments = match toolbook::arguments(line.json.as_deref(), &line.args, &described.input) {
    Ok(arguments) => arguments,
    Err(why) => return unread(&why),
  };

  if !line.asynchronous {
    return match book.call(&tool, &described.reached_by, arguments) {
      Ok(result) => match toolbook::refusal_of(&result) {
        Some(refusal) => refused(&tool, &refusal),
        None => {
          print!("{}", toolbook::printed(&result));

          ExitCode::SUCCESS
        }
      },
      Err(refusal) => refused(&tool, &refusal),
    };
  }

  let id = record::new_id(&tool);
  let request = Request {
    unit: format!("bmo-use-{id}"),
    id,
    tool,
    reached_by: described.reached_by,
    arguments,
    memory: line.memory,
    room: line
      .room
      .or_else(|| ground::variable("BMO_TURN_CHAT").and_then(|room| room.trim().parse().ok())),
    agent: line.agent.or_else(|| ground::variable("BMO_TURN_AGENT")),
    via: via.map(str::to_owned).or_else(|| ground::variable("BMO_TURN_VIA")),
    started_at: record::now(),
  };

  asked_of(&request, &root)
}

// A call asked: the tool host keeps and runs it where its calls face answers; a host that serves none, or cannot be
// asked, leaves the caller to keep it in its own workspace.
fn asked_of(request: &Request, root: &Path) -> ExitCode {
  let asked = json!({
    "tool": request.tool,
    "arguments": &request.arguments,
    "memory": &request.memory,
    "room": request.room,
    "agent": &request.agent,
    "via": &request.via,
  });

  if let Some(calls) = calls::Calls::found(root) {
    match calls.served() {
      Ok(true) => {
        return match calls.start(&asked) {
          Ok(answer) => {
            println!("{}", serde_json::to_string_pretty(&answer).unwrap_or_default());

            ExitCode::SUCCESS
          }
          Err(why) => not_started(&request.tool, &why),
        };
      }
      Ok(false) => {}
      Err(why) => return not_started(&request.tool, &why),
    }
  }

  let records = record::workspace_records();

  settle(root, &records);
  started(root, &records, request)
}

// A kept request started as its own unit, its id, unit and folder answered: the road of a shell whose host serves
// no calls face, the record standing in the caller's workspace and the wake naming it there.
fn started(root: &Path, records: &Path, request: &Request) -> ExitCode {
  let (place, _held) = match record::create(records, request) {
    Ok(made) => made,
    Err(why) => {
      eprintln!(
        "bmo: the record of `{}` could not be kept: {}",
        request.tool,
        refused_write(&why, records)
      );

      return ExitCode::from(1);
    }
  };

  match systemd_run(&unit_line(request, root, records, &this_binary())) {
    Ok(()) => {
      let answer = json!({ "id": request.id, "unit": request.unit, "record": place.display().to_string() });

      println!("{answer}");

      ExitCode::SUCCESS
    }
    Err(why) => {
      let _ = std::fs::remove_dir_all(&place);

      eprintln!("bmo: the unit `{}` could not be started: {why}", request.unit);

      ExitCode::from(1)
    }
  }
}

// The binary running now, which a unit runs again.
fn this_binary() -> std::path::PathBuf {
  std::env::current_exe().unwrap_or_else(|_| "bmo".into())
}

// A unit started by `systemd-run` given `words`, or why it was not.
fn systemd_run(words: &[OsString]) -> Result<(), String> {
  match Command::new("systemd-run").args(words).stdin(Stdio::null()).status() {
    Ok(status) if status.success() => Ok(()),
    Ok(status) => Err(status.to_string()),
    Err(why) => Err(why.to_string()),
  }
}

/// The words `systemd-run` is given to start a kept call: the unit, its last word, each variable the run reads,
/// and the same binary as `use --run <id>`. The unit runs until the call ends, however long that takes.
pub fn unit_line(request: &Request, root: &Path, records: &Path, exe: &Path) -> Vec<OsString> {
  let mut last_word = OsString::from("--property=ExecStopPost=");

  last_word.push(exe);
  last_word.push(format!(" use --ended {}", request.id));

  unit_words(&request.unit, Some(last_word), root, records, exe, ["--run", &request.id])
}

/// The words `systemd-run` is given to start the wake's own unit of the kept call `id`: the unit, each variable a
/// wake reads, and the same binary as `use --wake <id>`, with no last word. It runs until the wake is queued.
pub fn wake_line(id: &str, root: &Path, records: &Path, exe: &Path) -> Vec<OsString> {
  unit_words(&wake_unit(id), None, root, records, exe, ["--wake", id])
}

/// The name of the wake's own unit of the kept call `id`.
pub fn wake_unit(id: &str) -> String {
  format!("bmo-use-{id}-wake")
}

// The words starting `unit`: its last word where it has one, the variables a kept call's units read, then `exe`
// as `use <said>`.
fn unit_words(
  unit: &str,
  last_word: Option<OsString>,
  root: &Path,
  records: &Path,
  exe: &Path,
  said: [&str; 2],
) -> Vec<OsString> {
  let mut words: Vec<OsString> =
    vec!["--user".into(), format!("--unit={unit}").into(), "--collect".into(), "--quiet".into()];

  words.extend(last_word);

  let mut data_root = OsString::from("--setenv=BMO_DATA_DIR=");

  data_root.push(root);
  words.push(data_root);

  let mut record_dir = OsString::from("--setenv=BMO_USE_RECORD_DIR=");

  record_dir.push(records);
  words.push(record_dir);

  for name in [
    "BMO_MCP_TOKEN",
    "BMO_MCP_HOST_PORT",
    "RECEIVER_ADDRESS",
    "RECEIVER_TOKEN",
    "BMO_USE_WAKE_TRY_MS",
    "BMO_USE_WAKE_RETRY_MS",
  ] {
    if ground::variable(name).is_some() {
      words.push(format!("--setenv={name}").into());
    }
  }

  words.extend([exe.as_os_str().to_owned(), "use".into()]);
  words.extend(said.map(OsString::from));
  words
}

// A kept call run: the answer and the end kept, then the agent woken.
fn ran(root: &Path, records: &Path, id: &str) -> ExitCode {
  let Some((request, _)) = record::of(records, id) else {
    return unread(&format!("no record of `{id}` stands"));
  };
  let place = record::place(records, id);
  let outcome = Toolbook::found(root)
    .map_err(|why| Refusal {
      code: "unauthorized".to_owned(),
      why,
    })
    .and_then(|book| book.call(&request.tool, &request.reached_by, request.arguments.clone()))
    .and_then(|result| toolbook::refusal_of(&result).map_or(Ok(result), Err));
  let answer = match &outcome {
    Ok(result) => result.clone(),
    Err(refusal) => json!({ "code": refusal.code, "error": refusal.why }),
  };
  let state = if outcome.is_ok() { "done" } else { "refused" };
  let summary = wake::summary(&request.tool, &outcome);
  let answer_path = place.join(record::ANSWER);
  let _held = held_or_said(&place);

  if let Err(why) = record::keep(&answer_path, &answer) {
    eprintln!("bmo: the answer of `{id}` could not be kept: {why}");
  }

  finish(root, records, &request, state, summary)
}

// A kept call whose unit stopped: nothing where its run kept its end and queued its wake. Where no end stands, the
// failure is kept, as the answer where none stands, and as the end. An owed wake is then tried once, and handed to
// a wake's own unit where it is still owed.
fn ended(root: &Path, records: &Path, id: &str) -> ExitCode {
  if record::of(records, id).is_none() {
    return unread(&format!("no record of `{id}` stands"));
  }

  let held = held_or_said(&record::place(records, id));
  let Some((request, end)) = record::of(records, id) else {
    return unread(&format!("no record of `{id}` stands"));
  };
  let mut end = match end {
    Some(end) if !end.owed() => return ExitCode::SUCCESS,
    Some(end) => end,
    None => {
      let why = format!("its unit stopped before the call ended ({})", how_stopped());

      ended_as(records, &request, "failed", &why, wake::failure_summary(&request.tool, &why))
    }
  };

  match wake::send(root, &wake_body(records, &request, &end), Pace::read().try_for) {
    Ok(instruction) => {
      end.wake = wake::queued(instruction);
      kept_end(records, id, &end);

      ExitCode::SUCCESS
    }
    Err(why) => {
      end.wake = wake::owed(tries(&end) + 1, Some(&why));
      kept_end(records, id, &end);
      drop(held);

      match hand_off(root, records, id) {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::from(1),
      }
    }
  }
}

// A wake's own unit: the owed wake of the kept call `id` tried until the receiver queues it, once the record's
// lock is held; nothing where the wake was queued or no end stands.
fn woken(root: &Path, records: &Path, id: &str) -> ExitCode {
  if record::of(records, id).is_none() {
    return unread(&format!("no record of `{id}` stands"));
  }

  let _held = held_or_said(&record::place(records, id));

  match record::of(records, id) {
    Some((request, Some(end))) if end.owed() => deliver(root, records, &request, end),
    _ => ExitCode::SUCCESS,
  }
}

// The records at `records` settled before they are read or added to (docs/api.md, 「Settling the records」): a
// record with no end whose unit stands nowhere ended `lost` with its wake owed, and every owed wake handed to a
// wake's own unit. A record another process holds is passed over, and so is the data root's `use/`, the host's.
fn settle(root: &Path, records: &Path) {
  if records == record::folder(root) {
    return;
  }

  for (request, end) in record::every(records) {
    if end.is_some_and(|end| !end.owed()) {
      continue;
    }

    let place = record::place(records, &request.id);
    let Some(held) = record::try_hold(&place) else {
      continue;
    };

    match record::read::<End>(&place.join(record::END)) {
      Some(end) if !end.owed() => continue,
      Some(_) => {}
      None if record::standing(&request.unit) => continue,
      None => {
        let why = "its unit stood nowhere and kept no end, as where the machine restarted under it";

        ended_as(records, &request, "lost", why, wake::lost_summary(&request.tool, why));
      }
    }

    drop(held);

    let _ = hand_off(root, records, &request.id);
  }
}

// The lock of the record at `place`, waited for; where it cannot be had, said, and the record kept without it.
fn held_or_said(place: &Path) -> Option<std::fs::File> {
  record::hold(place)
    .inspect_err(|why| eprintln!("bmo: the record's lock could not be held: {why}"))
    .ok()
}

// The owed wake of the kept call `id` handed to a wake's own unit; where that unit cannot be started, said.
fn hand_off(root: &Path, records: &Path, id: &str) -> Result<(), ()> {
  systemd_run(&wake_line(id, root, records, &this_binary())).map_err(|why| {
    eprintln!(
      "bmo: the wake of `{id}` is owed and its unit `{}` could not be started: {why}",
      wake_unit(id)
    );
  })
}

// A kept call ended as `state` without its run's answer: `{code: state, error: why}` kept as the answer where none
// stands, then the end with its wake owed.
fn ended_as(records: &Path, request: &Request, state: &str, why: &str, summary: String) -> End {
  let id = &request.id;
  let answer_path = record::place(records, id).join(record::ANSWER);

  if !answer_path.exists()
    && let Err(kept) = record::keep(&answer_path, &json!({ "code": state, "error": why }))
  {
    eprintln!("bmo: the answer of `{id}` could not be kept: {kept}");
  }

  let end = owed_end(state, summary);

  kept_end(records, id, &end);
  end
}

// How the unit stopped, as systemd hands its last word: the service result, the exit code and the exit status.
fn how_stopped() -> String {
  let said = |name: &str| ground::variable(name).unwrap_or_else(|| "unknown".to_owned());

  format!("result {}, exit {} {}", said("SERVICE_RESULT"), said("EXIT_CODE"), said("EXIT_STATUS"))
}

// A kept call ended as `state`: the end kept with its wake owed, then the agent woken.
fn finish(root: &Path, records: &Path, request: &Request, state: &str, summary: String) -> ExitCode {
  let end = owed_end(state, summary);

  kept_end(records, &request.id, &end);
  deliver(root, records, request, end)
}

// The owed wake of `request` tried until the receiver queues it, the end kept after every try.
fn deliver(root: &Path, records: &Path, request: &Request, mut end: End) -> ExitCode {
  let id = &request.id;
  let body = wake_body(records, request, &end);
  let instruction = wake::until_queued(root, &body, Pace::read(), tries(&end), |owed| {
    end.wake = owed;
    kept_end(records, id, &end);
  });

  end.wake = wake::queued(instruction);
  kept_end(records, id, &end);

  ExitCode::SUCCESS
}

// An end as `state` now, its wake owed and not yet tried.
fn owed_end(state: &str, summary: String) -> End {
  End {
    ended_at: record::now(),
    state: state.to_owned(),
    summary,
    wake: wake::owed(0, None),
  }
}

// The wake a kept call's end is queued as.
fn wake_body(records: &Path, request: &Request, end: &End) -> Value {
  let answer = record::place(records, &request.id).join(record::ANSWER);

  wake::body(request, &end.state, &answer, &end.summary)
}

// How many times an end's owed wake was tried.
fn tries(end: &End) -> u64 {
  end.wake["tries"].as_u64().unwrap_or(0)
}

// The end of the kept call `id` kept; where it cannot be, said.
fn kept_end(records: &Path, id: &str, end: &End) {
  if let Err(why) = record::keep(&record::place(records, id).join(record::END), end) {
    eprintln!("bmo: the end of `{id}` could not be kept: {why}");
  }
}

// The tools the toolbook lists, one a line: the full name, the proxy reaching it and its first sentence.
fn tools(root: &Path, query: Option<&str>) -> ExitCode {
  let book = match Toolbook::found(root) {
    Ok(book) => book,
    Err(why) => return unread(&why),
  };

  match book.list(query) {
    Ok(listed) if listed.is_empty() => {
      eprintln!(
        "bmo: use: no tool is listed{}",
        query.map(|query| format!(" for `{query}`")).unwrap_or_default()
      );

      ExitCode::SUCCESS
    }
    Ok(listed) => {
      for item in listed {
        println!("{}  {}  {}", item.tool, item.reached_by, item.summary);
      }

      ExitCode::SUCCESS
    }
    Err(refusal) => refused("list", &refusal),
  }
}

// Every record, one a line: its id, its state and its tool. The host's records are asked for where its calls face
// answers; the shell's own are read where it does not.
fn listed(root: &Path, records: &Path) -> ExitCode {
  if let Some(calls) = calls::Calls::found(root) {
    match calls.served() {
      Ok(true) => {
        return match calls.list() {
          Ok(answer) => {
            for call in answer["items"].as_array().into_iter().flatten() {
              println!(
                "{}  {}  {}",
                call["id"].as_str().unwrap_or_default(),
                call["state"].as_str().unwrap_or_default(),
                call["tool"].as_str().unwrap_or_default()
              );
            }

            ExitCode::SUCCESS
          }
          Err(why) => unread(&why),
        };
      }
      Ok(false) => {}
      Err(why) => return unread(&why),
    }
  }

  settle(root, records);

  for (request, end) in record::every(records) {
    println!("{}  {}  {}", request.id, record::state(&request, end.as_ref()), request.tool);
  }

  ExitCode::SUCCESS
}

// One record as JSON: the host's where its calls face answers, else the one this shell kept.
fn status(root: &Path, records: &Path, id: &str) -> ExitCode {
  if let Some(calls) = calls::Calls::found(root) {
    match calls.served() {
      Ok(true) => {
        return match calls.one(id) {
          Ok(record) => {
            println!("{}", serde_json::to_string_pretty(&record).unwrap_or_default());

            ExitCode::SUCCESS
          }
          Err(why) => unread(&why),
        };
      }
      Ok(false) => {}
      Err(why) => return unread(&why),
    }
  }

  settle(root, records);

  let Some((request, end)) = record::of(records, id) else {
    return unread(&format!("no record of `{id}` stands"));
  };
  let state = record::state(&request, end.as_ref());
  let mut answer = serde_json::to_value(&request).unwrap_or(Value::Null);

  answer["state"] = json!(state);

  if let Some(end) = end {
    answer["endedAt"] = json!(end.ended_at);
    answer["summary"] = json!(end.summary);
    answer["answer"] = json!(record::place(records, id).join(record::ANSWER).display().to_string());
    answer["wake"] = end.wake;
  }

  println!("{}", serde_json::to_string_pretty(&answer).unwrap_or_default());

  ExitCode::SUCCESS
}

// A call that could not be started, and why.
fn not_started(tool: &str, why: &str) -> ExitCode {
  eprintln!("bmo: `{tool}` could not be started: {why}");

  ExitCode::from(1)
}

// A write the shell refused, said with what refused it: a shell sandboxed to its workspace allows no path outside
// it, so the error names the sandbox and the workspace a call's record must stand in.
fn refused_write(why: &str, records: &Path) -> String {
  let sandboxed = ["Permission denied", "Read-only file system", "Operation not permitted"]
    .iter()
    .any(|said| why.contains(said));

  if !sandboxed {
    return why.to_owned();
  }

  let workspace = std::env::current_dir().unwrap_or_default();

  format!(
    "{why}; the shell's file sandbox allows its workspace only ({}), and the record was to stand in {}",
    workspace.display(),
    records.display()
  )
}

// A call refused, by its code and why.
fn refused(tool: &str, refusal: &Refusal) -> ExitCode {
  eprintln!("bmo: `{tool}` was refused: {}: {}", refusal.code, refusal.why);

  ExitCode::from(1)
}

// Words or a configuration that do not read.
fn unread(why: &str) -> ExitCode {
  eprintln!("bmo: use: {why}");

  ExitCode::from(2)
}

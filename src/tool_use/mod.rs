//! The tool verb, `bmo use`: one hosted tool reached through the tool host's toolbook, its answer printed at
//! once or, started as a unit of its own, kept and handed to the agent by a wake (docs/api.md, 「The tool
//! verb」).

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

/// The words `bmo use` reads.
#[derive(Parser, Debug)]
#[command(
  name = "bmo use",
  about = "Calls one hosted tool through the tool host's toolbook",
  disable_version_flag = true
)]
pub struct UseLine {
  /// The tool's full name, `<namespace>/<name>`.
  #[arg(value_name = "TOOL", required_unless_present_any = ["list", "status", "run"])]
  pub tool: Option<String>,
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
  /// Every record kept, one a line.
  #[arg(long, conflicts_with_all = ["tool", "status", "run"])]
  pub list: bool,
  /// One record, as JSON.
  #[arg(long, value_name = "ID", conflicts_with_all = ["tool", "run"])]
  pub status: Option<String>,
  /// Runs a kept call; the unit's own word.
  #[arg(long, value_name = "ID", hide = true, conflicts_with = "tool")]
  pub run: Option<String>,
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

  if line.list {
    return listed(&root);
  }

  if let Some(id) = &line.status {
    return status(&root, id);
  }

  if let Some(id) = &line.run {
    return ran(&root, id);
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

  started(&root, &request)
}

// A kept request started as its own unit, its id, unit and folder answered.
fn started(root: &Path, request: &Request) -> ExitCode {
  let place = match record::create(root, request) {
    Ok(place) => place,
    Err(why) => {
      eprintln!("bmo: the record of `{}` could not be kept: {why}", request.tool);

      return ExitCode::from(1);
    }
  };
  let exe = std::env::current_exe().unwrap_or_else(|_| "bmo".into());
  let started = Command::new("systemd-run")
    .args(unit_line(&request.unit, &request.id, root, &exe))
    .stdin(Stdio::null())
    .status();

  match started {
    Ok(status) if status.success() => {
      let answer = json!({ "id": request.id, "unit": request.unit, "record": place.display().to_string() });

      println!("{answer}");

      ExitCode::SUCCESS
    }
    failed => {
      let why = failed.map_or_else(|why| why.to_string(), |status| status.to_string());
      let _ = std::fs::remove_dir_all(&place);

      eprintln!("bmo: the unit `{}` could not be started: {why}", request.unit);

      ExitCode::from(1)
    }
  }
}

/// The words `systemd-run` is given to start a kept call: the unit, each variable the run reads, and the same
/// binary as `use --run <id>`.
pub fn unit_line(unit: &str, id: &str, root: &Path, exe: &Path) -> Vec<OsString> {
  let mut words: Vec<OsString> =
    vec!["--user".into(), format!("--unit={unit}").into(), "--collect".into(), "--quiet".into()];
  let mut data_root = OsString::from("--setenv=BMO_DATA_DIR=");

  data_root.push(root);
  words.push(data_root);

  for name in ["BMO_MCP_TOKEN", "BMO_MCP_HOST_PORT", "RECEIVER_ADDRESS", "RECEIVER_TOKEN"] {
    if ground::variable(name).is_some() {
      words.push(format!("--setenv={name}").into());
    }
  }

  words.extend([exe.as_os_str().to_owned(), "use".into(), "--run".into(), id.into()]);
  words
}

// A kept call run: the answer and the end kept, then the agent woken.
fn ran(root: &Path, id: &str) -> ExitCode {
  let Some((request, _)) = record::of(root, id) else {
    return unread(&format!("no record of `{id}` stands"));
  };
  let place = record::place(root, id);
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

  if let Err(why) = record::keep(&answer_path, &answer) {
    eprintln!("bmo: the answer of `{id}` could not be kept: {why}");
  }

  let woken = wake::send(root, &wake::body(&request, state, &answer_path, &summary));
  let end = End {
    ended_at: record::now(),
    state: state.to_owned(),
    summary,
    wake: match &woken {
      Ok(instruction) => json!({ "id": instruction }),
      Err(why) => json!({ "error": why }),
    },
  };

  if let Err(why) = record::keep(&place.join(record::END), &end) {
    eprintln!("bmo: the end of `{id}` could not be kept: {why}");
  }

  match woken {
    Ok(_) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("bmo: the agent was not woken for `{id}`: {why}");

      ExitCode::from(1)
    }
  }
}

// Every record, one a line: its id, its state and its tool.
fn listed(root: &Path) -> ExitCode {
  for (request, end) in record::every(root) {
    println!("{}  {}  {}", request.id, record::state(&request, end.as_ref()), request.tool);
  }

  ExitCode::SUCCESS
}

// One record as JSON.
fn status(root: &Path, id: &str) -> ExitCode {
  let Some((request, end)) = record::of(root, id) else {
    return unread(&format!("no record of `{id}` stands"));
  };
  let state = record::state(&request, end.as_ref());
  let mut answer = serde_json::to_value(&request).unwrap_or(Value::Null);

  answer["state"] = json!(state);

  if let Some(end) = end {
    answer["endedAt"] = json!(end.ended_at);
    answer["summary"] = json!(end.summary);
    answer["answer"] = json!(record::place(root, id).join(record::ANSWER).display().to_string());
    answer["wake"] = end.wake;
  }

  println!("{}", serde_json::to_string_pretty(&answer).unwrap_or_default());

  ExitCode::SUCCESS
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

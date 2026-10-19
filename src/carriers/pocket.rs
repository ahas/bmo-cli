//! The tool host's catalogue read and called from a shell (docs/boundary.md, 「bmo-pocket」): `tool list`,
//! `tool describe` and `tool call` on the routes of its control socket, and `tool sites` on the web
//! namespace's facts there.

use std::ffi::OsString;
use std::os::unix::fs::MetadataExt as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use hyper::Method;
use serde_json::{Value, json};

use crate::carried::{Call, Carried, Carrier};
use crate::unix_http;

const UNIT: &str = "bmo-pocket";

/// Every entry the tool host serves, by full name; words narrow it to those whose name or description holds
/// them.
#[derive(Parser)]
#[command(name = "bmo tool list", no_binary_name = true)]
struct List {
  words: Vec<String>,
}

/// One entry by its full name, namespace/tool: what it does, what it asks and its schemas.
#[derive(Parser)]
#[command(name = "bmo tool describe", no_binary_name = true)]
struct Describe {
  tool: String,
}

/// One entry called by its full name, its arguments one JSON object.
#[derive(Parser)]
#[command(name = "bmo tool call", no_binary_name = true)]
struct CallTool {
  tool: String,
  #[arg(default_value = "{}")]
  arguments: String,
}

/// The site tools as the web namespace's facts carry them.
#[derive(Parser)]
#[command(name = "bmo tool sites", no_binary_name = true)]
struct Sites {
  #[arg(long)]
  detail: bool,
}

struct Pocket;

impl Carried for Pocket {
  fn unit(&self) -> &'static str {
    UNIT
  }

  fn commands(&self) -> &'static [&'static str] {
    &["tool list", "tool describe", "tool call", "tool sites"]
  }

  fn run(&self, call: &Call<'_>) -> ExitCode {
    let answered = match call.command {
      "tool list" => list(read(call.rest)),
      "tool describe" => describe(read(call.rest)),
      "tool call" => call_tool(read(call.rest)),
      _ => sites(read(call.rest)),
    };

    match answered {
      Ok((lines, failed)) => {
        for line in lines {
          println!("{line}");
        }

        if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS }
      }
      Err(why) => {
        eprintln!("{why}");

        ExitCode::FAILURE
      }
    }
  }
}

inventory::submit! { Carrier(&Pocket) }

// What a command printed, and whether it ends as failed.
type Answered = Result<(Vec<String>, bool), String>;

fn read<T: Parser>(rest: &[OsString]) -> T {
  T::try_parse_from(rest).unwrap_or_else(|why| why.exit())
}

// The control socket: the one `BMO_MCP_HOST_SOCKET` names, else `bmo/mcp-host.sock` under the user's runtime
// directory.
fn socket() -> PathBuf {
  if let Some(named) = std::env::var_os("BMO_MCP_HOST_SOCKET").filter(|named| !named.is_empty()) {
    return PathBuf::from(named);
  }

  let runtime = std::env::var_os("XDG_RUNTIME_DIR").filter(|named| !named.is_empty()).map_or_else(
    || {
      let user = std::fs::metadata("/proc/self").map(|meta| meta.uid()).unwrap_or_default();

      PathBuf::from(format!("/run/user/{user}"))
    },
    PathBuf::from,
  );

  runtime.join("bmo").join("mcp-host.sock")
}

fn ask(method: Method, path: &str, body: Option<&Value>) -> Result<(u16, Value), String> {
  let socket = socket();

  unix_http::request(&socket, method, path, body)
    .map_err(|why| format!("mcp-host does not answer on {}: {why}", socket.display()))
}

// A description's first sentence.
fn first_sentence(text: &str) -> &str {
  let text = text.trim();

  text.find(". ").map_or(text, |end| &text[..=end])
}

// One line of the catalogue: the entry's full name, how it is reached, where it stands and its first sentence.
fn line(entry: &Value) -> String {
  let at = match entry["placed"].as_str() {
    Some(folder) => format!("tools/{folder}"),
    None => "linked".to_owned(),
  };
  let reached = if entry["system"] == true {
    "system"
  } else {
    entry["reachedBy"].as_str().unwrap_or("call")
  };

  format!(
    "{} · {reached} · {at}: {}",
    entry["name"].as_str().unwrap_or_default(),
    first_sentence(entry["description"].as_str().unwrap_or_default())
  )
}

fn list(asked: List) -> Answered {
  let (_, every) = ask(Method::GET, "/catalogue", None)?;
  let wanted: Vec<String> = asked.words.iter().map(|word| word.to_lowercase()).collect();
  let lines = every
    .as_array()
    .into_iter()
    .flatten()
    .filter(|entry| {
      let said = format!("{} {}", entry["name"], entry["description"]).to_lowercase();

      wanted.iter().all(|word| said.contains(word))
    })
    .map(line)
    .collect();

  Ok((lines, false))
}

fn entry(tool: &str) -> Result<Value, String> {
  match ask(Method::GET, &format!("/catalogue/{tool}"), None)? {
    (200, entry) => Ok(entry),
    _ => Err(format!("the tool host serves no {tool}")),
  }
}

fn describe(asked: Describe) -> Answered {
  let entry = entry(&asked.tool)?;

  Ok((
    vec![
      entry["described"].as_str().unwrap_or_default().to_owned(),
      format!("Capabilities: {}", entry["capabilities"]),
      format!("Grant: {}", entry["grant"].as_str().unwrap_or("none")),
      line(&entry),
    ],
    false,
  ))
}

// The record a call from the shell is made under: the entry's own capabilities, the data root this process
// stands on, the archive its environment names, and a turn whose every message is the owner's, since only the
// host's own user reaches the socket.
fn shell_record(capabilities: &Value) -> Value {
  let data_root = std::env::var_os("BMO_DATA_DIR")
    .filter(|named| !named.is_empty())
    .map(PathBuf::from)
    .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share/bmo/data")))
    .unwrap_or_default();
  let mut record = json!({
    "line": "shell",
    "capabilities": capabilities,
    "dataRoot": data_root,
    "runVariables": { "BMO_TURN_PRIVILEGED": "1" },
  });

  if let Some(archive) = std::env::var("BMO_ARCHIVE_URL").ok().filter(|archive| !archive.is_empty()) {
    record["archive"] = Value::from(archive);
  }

  record
}

fn call_tool(asked: CallTool) -> Answered {
  let arguments: Value = serde_json::from_str(&asked.arguments)
    .map_err(|why| format!("the arguments are not one JSON object: {why}"))?;
  let entry = entry(&asked.tool)?;
  let record = shell_record(&entry["capabilities"]);
  let (_, answer) = ask(
    Method::POST,
    "/calls",
    Some(&json!({ "session": record, "tool": asked.tool, "arguments": arguments })),
  )?;
  let lines = answer["content"]
    .as_array()
    .into_iter()
    .flatten()
    .map(|piece| piece["text"].as_str().map_or_else(|| piece.to_string(), str::to_owned))
    .collect();

  Ok((lines, answer["isError"] == true))
}

fn sites(asked: Sites) -> Answered {
  let (status, facts) = ask(Method::GET, "/facts/web", None)?;
  let catalogue = match facts["sites"].as_array() {
    Some(sites) if status == 200 => sites.clone(),
    _ => return Err("the tool host keeps no site catalogue".to_owned()),
  };
  let text = |tool: &Value, field: &str| tool[field].as_str().unwrap_or_default().to_owned();
  let mut lines = vec![format!("{} site tools", catalogue.len())];

  for tool in &catalogue {
    let arguments: Vec<&str> =
      tool["arguments"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();

    lines.push(String::new());
    lines.push(format!("{}: {}", text(tool, "name"), text(tool, "what")));
    lines.push(format!("  site: {}", text(tool, "site")));
    lines.push(format!("  since: {}", text(tool, "since")));
    lines.push(format!(
      "  arguments: {}",
      if arguments.is_empty() { "none".to_owned() } else { arguments.join(", ") }
    ));
    lines.extend(tool["answers"].as_str().map(|forms| format!("  answers: {forms}")));

    if asked.detail {
      lines.push(format!("  description: {}", text(tool, "description")));
    }
  }

  Ok((lines, false))
}

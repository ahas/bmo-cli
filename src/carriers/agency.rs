//! bmo-agency's commands (docs/boundary.md, 「bmo-agency」): each handed to the `bmo-agency` command under the
//! words its documents and its own command give, and a worker's keepalive paused at the cache warmer, which
//! keeps the switch it moves.

use std::ffi::OsString;
use std::process::ExitCode;
use std::time::Duration;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::{Value, json};

use crate::carried::{Call, Carried, Carrier};
use crate::{answer, http, settings};

const UNIT: &str = "bmo-agency";
const WARMER: &str = "bmo-cache-warmer";
// The consumer agency registers its workers' sessions under, where the warmer's table names none.
const CONSUMER: &str = "agency";
const WITHIN: Duration = Duration::from_secs(30);
// The kind of call `agent status` answers that runs where it is called: the shell guard.
const SHELL_KIND: &str = "shell";

// Each command beside the words `bmo-agency` takes it under, where they are not the same.
const RENAMED: [(&str, &str); 1] = [("agent status", "agent-status")];

// The commands that run on the operations machine, handed over the home bridge on a work node.
const HOME: [&str; 7] = [
  "agent",
  "agents",
  "coworker hook",
  "node claude-token",
  "node work",
  "work-node",
  "run list",
];

struct Agency;

impl Carried for Agency {
  fn unit(&self) -> &'static str {
    UNIT
  }

  fn commands(&self) -> &'static [&'static str] {
    &[
      "agent",
      "agents",
      "agent status",
      "agent-status",
      "model",
      "models",
      "coworker pool",
      "coworker hook",
      "worker fold",
      "run list",
      "run stop",
      "machine memory",
      "reload",
      "node work",
      "work-node",
      "node claude-token",
      "node pane",
      "node-pane",
      "dev-server",
      "dev-pane",
      "companion follow",
      "companion reset",
      "claude batch",
      "live-claude",
    ]
  }

  fn run(&self, call: &Call<'_>) -> ExitCode {
    if matches!(call.command, "agent" | "agents")
      && let Some(worker) = paused(call.rest)
    {
      return pause(call, &worker);
    }

    let words = RENAMED
      .iter()
      .find(|(command, _)| *command == call.command)
      .map_or(call.command, |(_, words)| words);

    call.forward(UNIT, &words.split(' ').collect::<Vec<_>>())
  }

  fn home(&self, call: &Call<'_>) -> bool {
    match call.command {
      "agent status" | "agent-status" => {
        call.rest.first().and_then(|kind| kind.to_str()) != Some(SHELL_KIND)
      }
      command => HOME.contains(&command),
    }
  }
}

inventory::submit! { Carrier(&Agency) }

// The worker `agent --pause-keepalive <worker>` names, where those are the command's only words.
fn paused(rest: &[OsString]) -> Option<String> {
  let words: Vec<&str> = rest.iter().map(|word| word.to_str()).collect::<Option<_>>()?;

  match words.as_slice() {
    ["--pause-keepalive", worker] => Some((*worker).to_owned()),
    [word] => word.strip_prefix("--pause-keepalive=").map(str::to_owned),
    _ => None,
  }
  .filter(|worker| !worker.is_empty())
}

// A worker's sessions left unwarmed until its next job: its subject paused at the cache warmer.
fn pause(call: &Call<'_>, worker: &str) -> ExitCode {
  let table = match settings::unit(WARMER) {
    Ok(table) => table,
    Err(why) => return answer::unsettled(&call.words(), WARMER, &why),
  };
  let Some(address) =
    table.get("address").and_then(toml::Value::as_str).filter(|address| !address.is_empty())
  else {
    return answer::unsettled(
      &call.words(),
      WARMER,
      &format!("the settings name no `unit.{WARMER}.address`"),
    );
  };
  let consumer = table.get("consumer").and_then(toml::Value::as_str).unwrap_or(CONSUMER);
  let token = table.get("token").and_then(toml::Value::as_str).unwrap_or_default();
  let path = format!("/consumers/{}/subjects/{}/pause", segment(consumer), segment(worker));

  match http::post(address, token, &path, &json!({}), WITHIN) {
    Ok((200..=299, _)) => {
      println!("worker {worker}: keepalive off until its next job");

      ExitCode::SUCCESS
    }
    Ok((status, said)) => {
      let why = said.get("error").and_then(Value::as_str).map_or_else(
        || format!("the cache warmer turned it away ({status}) and gave no reason"),
        str::to_owned,
      );

      eprintln!("{why}");
      ExitCode::FAILURE
    }
    Err(why) => {
      eprintln!("{why}");
      ExitCode::FAILURE
    }
  }
}

// The bytes a path segment keeps as they are: the unreserved set.
const KEPT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

// A name as one percent-encoded path segment.
fn segment(name: &str) -> String {
  utf8_percent_encode(name, KEPT).to_string()
}

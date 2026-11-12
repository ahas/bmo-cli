//! The wake: once an asynchronous call is done, the receiver is asked to queue a fact for the agent in its
//! room, carrying the answer's path and a one-line summary, and asked again until it queues it (docs/api.md,
//! 「The wake」).

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use super::ground;
use super::record::Request;
use super::toolbook::{Refusal, texts};

/// The sort every wake is queued under.
pub const SORT: &str = "use-ended";

// The longest summary, in characters.
const SUMMARY_CHARS: usize = 160;

// The longest wait between two tries.
const LONGEST_WAIT: Duration = Duration::from_secs(60);

/// How a wake is tried: how long one try waits for the receiver's answer, and the first wait between two tries,
/// which doubles after every try that failed, up to a minute.
#[derive(Debug, Clone, Copy)]
pub struct Pace {
  pub try_for: Duration,
  pub first_wait: Duration,
}

impl Pace {
  /// The pace `BMO_USE_WAKE_TRY_MS` and `BMO_USE_WAKE_RETRY_MS` name, else thirty seconds and one second.
  pub fn read() -> Self {
    let millis = |name: &str, otherwise: u64| {
      Duration::from_millis(
        ground::variable(name).and_then(|value| value.trim().parse().ok()).unwrap_or(otherwise),
      )
    };

    Self {
      try_for: millis("BMO_USE_WAKE_TRY_MS", 30_000),
      first_wait: millis("BMO_USE_WAKE_RETRY_MS", 1_000),
    }
  }
}

/// A wake still owed after `tries` tries, the last of which failed for `why`.
pub fn owed(tries: u64, why: Option<&str>) -> Value {
  match why {
    Some(why) => json!({ "pending": true, "tries": tries, "error": why }),
    None => json!({ "pending": true, "tries": tries }),
  }
}

/// A wake the receiver queued as `instruction`.
pub fn queued(instruction: i64) -> Value {
  json!({ "id": instruction })
}

/// `body` queued, tried again at `pace` until the receiver queues it: the wake as it stands after each failed try,
/// counted on from `tries`, handed to `kept`; the instruction's id once queued.
pub fn until_queued(
  root: &Path,
  body: &Value,
  pace: Pace,
  mut tries: u64,
  mut kept: impl FnMut(Value),
) -> i64 {
  let mut wait = pace.first_wait;

  loop {
    match send(root, body, pace.try_for) {
      Ok(instruction) => return instruction,
      Err(why) => {
        tries += 1;
        kept(owed(tries, Some(&why)));
        std::thread::sleep(wait);
        wait = (wait * 2).min(LONGEST_WAIT);
      }
    }
  }
}

/// One line saying how a call of `tool` ended.
pub fn summary(tool: &str, outcome: &Result<Value, Refusal>) -> String {
  match outcome {
    Ok(result) => {
      let first = texts(result).unwrap_or_default().into_iter().next().unwrap_or_default();

      cut(&format!("{tool} answered: {}", first_line(&first)))
    }
    Err(refusal) => cut(&format!("{tool} was refused: {}: {}", refusal.code, first_line(&refusal.why))),
  }
}

/// One line saying that a call of `tool` failed, and why.
pub fn failure_summary(tool: &str, why: &str) -> String {
  cut(&format!("{tool} failed: {}", first_line(why)))
}

/// One line saying that a call of `tool` was lost, and why.
pub fn lost_summary(tool: &str, why: &str) -> String {
  cut(&format!("{tool} was lost: {}", first_line(why)))
}

// The first line of `text`.
fn first_line(text: &str) -> &str {
  text.lines().next().unwrap_or_default()
}

// `line` cut to the longest summary.
fn cut(line: &str) -> String {
  line.chars().take(SUMMARY_CHARS).collect()
}

/// The instruction a call's end is queued as.
pub fn body(request: &Request, state: &str, answer: &Path, summary: &str) -> Value {
  json!({
    "agentName": request.agent,
    "room": request.room,
    "said": {
      "fact": {
        "use": request.id,
        "tool": request.tool,
        "state": state,
        "answer": answer.display().to_string(),
        "summary": summary,
        "memory": request.memory,
      },
      "order": null,
    },
    "sort": SORT,
    "summary": summary,
    "kind": null,
    "expiresAt": null,
    "events": [],
    "ridesUntil": null,
    "via": request.via,
    "run": null,
  })
}

/// `body` queued with the receiver the environment or the data root's `env/receiver.env` names, its answer waited
/// for `try_for` at most: the instruction's id, or why it was not queued.
pub fn send(root: &Path, body: &Value, try_for: Duration) -> Result<i64, String> {
  let address = ground::published("RECEIVER_ADDRESS", root, "env/receiver.env")
    .ok_or("no receiver address: RECEIVER_ADDRESS is set nowhere")?;
  let token = ground::published("RECEIVER_TOKEN", root, "env/receiver.env")
    .ok_or("no receiver token: RECEIVER_TOKEN is set nowhere")?;
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .timeout_global(Some(try_for))
    .build()
    .into();
  let url = format!("{}/instructions", address.trim_end_matches('/'));
  let mut response = agent
    .post(&url)
    .header("Authorization", format!("Bearer {token}"))
    .send_json(body)
    .map_err(|why| format!("the receiver at {address} is out of reach: {why}"))?;
  let status = response.status();
  let answer: Value = response.body_mut().read_json().unwrap_or(Value::Null);

  if !status.is_success() {
    return Err(format!(
      "the receiver answered {status}: {}: {}",
      answer["code"].as_str().unwrap_or("refused"),
      answer["error"].as_str().unwrap_or_default()
    ));
  }

  answer["id"]
    .as_i64()
    .ok_or_else(|| format!("the receiver answered {status} with no instruction id"))
}

//! The wake: once an asynchronous call is done, the receiver is asked to queue a fact for the agent in its
//! room, carrying the answer's path and a one-line summary (docs/api.md, 「The wake」).

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

// How long the receiver is given to answer.
const ANSWER_WITHIN: Duration = Duration::from_secs(30);

/// One line saying how a call of `tool` ended.
pub fn summary(tool: &str, outcome: &Result<Value, Refusal>) -> String {
  let line = match outcome {
    Ok(result) => {
      let first = texts(result).unwrap_or_default().into_iter().next().unwrap_or_default();

      format!("{tool} answered: {}", first.lines().next().unwrap_or_default())
    }
    Err(refusal) => {
      format!(
        "{tool} was refused: {}: {}",
        refusal.code,
        refusal.why.lines().next().unwrap_or_default()
      )
    }
  };

  line.chars().take(SUMMARY_CHARS).collect()
}

/// One line saying that a call of `tool` failed, and why.
pub fn failure_summary(tool: &str, why: &str) -> String {
  format!("{tool} failed: {}", why.lines().next().unwrap_or_default())
    .chars()
    .take(SUMMARY_CHARS)
    .collect()
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

/// `body` queued with the receiver the environment or the data root's `env/receiver.env` names: the
/// instruction's id, or why it was not queued.
pub fn send(root: &Path, body: &Value) -> Result<i64, String> {
  let address = ground::published("RECEIVER_ADDRESS", root, "env/receiver.env")
    .ok_or("no receiver address: RECEIVER_ADDRESS is set nowhere")?;
  let token = ground::published("RECEIVER_TOKEN", root, "env/receiver.env")
    .ok_or("no receiver token: RECEIVER_TOKEN is set nowhere")?;
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .timeout_global(Some(ANSWER_WITHIN))
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

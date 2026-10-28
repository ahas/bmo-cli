//! The tool host's calls face (`docs/api.md`, 「Asynchronous calls」): a call asked of the host, which keeps its
//! record and runs it, and the records it answers back. Nothing of a call is written in the caller's file system.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use super::ground;
use super::toolbook::DEFAULT_PORT;

// How long the host is given to answer, in seconds.
const ANSWER_WITHIN: Duration = Duration::from_secs(30);

/// The tool host's calls face: the port and the token the toolbook is asked with.
pub struct Calls {
  base: String,
  token: String,
  agent: ureq::Agent,
}

impl Calls {
  /// The calls face the caller reaches, or none where it holds no tool session token.
  pub fn found(root: &Path) -> Option<Self> {
    let token = ground::variable("BMO_MCP_TOKEN")?;
    let port = ground::published("BMO_MCP_HOST_PORT", root, "env/pocket.env")
      .unwrap_or_else(|| DEFAULT_PORT.to_owned());
    let agent = ureq::Agent::config_builder()
      .http_status_as_error(false)
      .timeout_global(Some(ANSWER_WITHIN))
      .build()
      .into();

    Some(Self {
      base: format!("http://127.0.0.1:{port}"),
      token,
      agent,
    })
  }

  /// Whether the host serves the calls face: a `GET /use` is answered where it does, and a host older than the
  /// face answers no route at all.
  pub fn served(&self) -> Result<bool, String> {
    let status = self
      .agent
      .get(&format!("{}/use", self.base))
      .header("Authorization", format!("Bearer {}", self.token))
      .call()
      .map_err(|why| format!("the tool host at {} is out of reach: {why}", self.base))?
      .status();

    Ok(status.is_success())
  }

  /// The call `asked` started by the host: its id and state.
  pub fn start(&self, asked: &Value) -> Result<Value, String> {
    let response = self
      .agent
      .post(&format!("{}/use", self.base))
      .header("Authorization", format!("Bearer {}", self.token))
      .send_json(asked)
      .map_err(|why| format!("the tool host at {} is out of reach: {why}", self.base))?;

    answer(response)
  }

  /// Every call the caller's session keeps, oldest first.
  pub fn list(&self) -> Result<Value, String> {
    let response = self
      .agent
      .get(&format!("{}/use", self.base))
      .header("Authorization", format!("Bearer {}", self.token))
      .call()
      .map_err(|why| format!("the tool host at {} is out of reach: {why}", self.base))?;

    answer(response)
  }

  /// One call the caller's session keeps, as it stands.
  pub fn one(&self, id: &str) -> Result<Value, String> {
    if !readable(id) {
      return Err(format!("`{id}` names no call"));
    }

    let response = self
      .agent
      .get(&format!("{}/use/{id}", self.base))
      .header("Authorization", format!("Bearer {}", self.token))
      .call()
      .map_err(|why| format!("the tool host at {} is out of reach: {why}", self.base))?;

    answer(response)
  }
}

// One answer of the calls face: its body, or its refusal by the code and error it answers.
fn answer(mut response: ureq::http::Response<ureq::Body>) -> Result<Value, String> {
  let status = response.status();
  let body: Value = response.body_mut().read_json().unwrap_or(Value::Null);

  if !status.is_success() {
    let code = body["code"].as_str().unwrap_or("refused");
    let why = body["error"].as_str().unwrap_or_default();

    return Err(format!("the tool host answered {code}: {why}"));
  }

  Ok(body)
}

// Whether `id` can name a call: letters, digits, `_` and `-` alone.
fn readable(id: &str) -> bool {
  !id.is_empty()
    && id
      .chars()
      .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

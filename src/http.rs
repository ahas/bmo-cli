//! One JSON request to a unit's HTTP service, with the token the settings name for it.

use std::time::Duration;

use serde_json::Value;

/// `POST`s `body` to `path` under `address`, with the token where one is given, and answers the status and
/// the JSON it answered with, `null` for an answer that is no JSON; a service that cannot be reached is the
/// error, saying why.
pub fn post(
  address: &str,
  token: &str,
  path: &str,
  body: &Value,
  within: Duration,
) -> Result<(u16, Value), String> {
  let url = format!("{}{path}", address.trim_end_matches('/'));
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .timeout_global(Some(within))
    .http_status_as_error(false)
    .build()
    .into();
  let mut request = agent.post(&url);

  if !token.is_empty() {
    request = request.header("Authorization", &format!("Bearer {token}"));
  }

  let mut answer = request.send_json(body).map_err(|why| format!("{url} did not answer: {why}"))?;
  let status = answer.status().as_u16();
  let said = answer.body_mut().read_json::<Value>().unwrap_or(Value::Null);

  Ok((status, said))
}

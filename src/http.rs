//! One JSON request to a unit's HTTP service, with the token the settings name for it.

use std::time::Duration;

use serde_json::Value;

fn agent(within: Duration) -> ureq::Agent {
  ureq::Agent::config_builder()
    .timeout_global(Some(within))
    .http_status_as_error(false)
    .build()
    .into()
}

fn url(address: &str, path: &str) -> String {
  format!("{}{path}", address.trim_end_matches('/'))
}

fn bearer(token: &str) -> Option<String> {
  (!token.is_empty()).then(|| format!("Bearer {token}"))
}

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
  let url = url(address, path);
  let mut request = agent(within).post(&url);

  if let Some(bearer) = bearer(token) {
    request = request.header("Authorization", &bearer);
  }

  let mut answer = request.send_json(body).map_err(|why| format!("{url} did not answer: {why}"))?;
  let status = answer.status().as_u16();

  Ok((status, answer.body_mut().read_json::<Value>().unwrap_or(Value::Null)))
}

/// `GET`s `path` under `address` with the query `pairs`, as `post` does.
pub fn get(
  address: &str,
  token: &str,
  path: &str,
  pairs: &[(&str, String)],
  within: Duration,
) -> Result<(u16, Value), String> {
  let url = url(address, path);
  let mut request = agent(within).get(&url);

  for (name, value) in pairs {
    request = request.query(*name, value);
  }

  if let Some(bearer) = bearer(token) {
    request = request.header("Authorization", &bearer);
  }

  let mut answer = request.call().map_err(|why| format!("{url} did not answer: {why}"))?;
  let status = answer.status().as_u16();

  Ok((status, answer.body_mut().read_json::<Value>().unwrap_or(Value::Null)))
}

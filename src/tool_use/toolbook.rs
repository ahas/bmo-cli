//! The tool host's toolbook, asked over the tools face: a tool described, then called with `call` or `act`,
//! under the caller's own tool session (docs/api.md, 「The tool verb」).

use std::path::Path;

use serde_json::{Map, Value, json};

use super::ground;

/// The tools face's port where neither the environment nor the data root names one.
const DEFAULT_PORT: &str = "3290";

/// Why a call did not answer: a stable code and the sentence that goes with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
  pub code: String,
  pub why: String,
}

impl Refusal {
  fn new(code: &str, why: impl Into<String>) -> Self {
    Self {
      code: code.to_owned(),
      why: why.into(),
    }
  }
}

/// A tool as `describe` answers it: which proxy reaches it and its input schema.
#[derive(Debug, Clone)]
pub struct Described {
  pub tool: String,
  pub reached_by: String,
  pub input: Value,
}

/// The toolbook of the caller's tool session.
pub struct Toolbook {
  url: String,
  token: String,
  agent: ureq::Agent,
}

impl Toolbook {
  /// The toolbook the caller reaches: its token from `BMO_MCP_TOKEN`, its port from `BMO_MCP_HOST_PORT`, else
  /// from the data root's `env/pocket.env`, else the default; or why the caller has no token.
  pub fn found(root: &Path) -> Result<Self, String> {
    let token = ground::variable("BMO_MCP_TOKEN")
      .ok_or("the caller's tool session token is needed in BMO_MCP_TOKEN, and none is set")?;
    let port = ground::published("BMO_MCP_HOST_PORT", root, "env/pocket.env")
      .unwrap_or_else(|| DEFAULT_PORT.to_owned());
    let agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();

    Ok(Self {
      url: format!("http://127.0.0.1:{port}/mcp/toolbook"),
      token,
      agent,
    })
  }

  /// `tool` as `describe` answers it.
  pub fn describe(&self, tool: &str) -> Result<Described, Refusal> {
    let result = self.ask("describe", json!({ "tool": tool }))?;

    if let Some(refusal) = refusal_of(&result) {
      return Err(refusal);
    }

    let text = texts(&result).unwrap_or_default().join("\n");
    let reached_by = text
      .lines()
      .next()
      .and_then(|first| first.rsplit_once("reached with "))
      .map(|(_, proxy)| proxy.trim())
      .filter(|proxy| matches!(*proxy, "call" | "act"))
      .ok_or_else(|| Refusal::new("unread", format!("describe named neither call nor act for {tool}")))?;
    let input = text
      .lines()
      .find_map(|line| line.strip_prefix("Input schema: "))
      .and_then(|schema| serde_json::from_str(schema).ok())
      .unwrap_or(Value::Null);

    Ok(Described {
      tool: tool.to_owned(),
      reached_by: reached_by.to_owned(),
      input,
    })
  }

  /// `tool` called through `reached_by` with `arguments`: the tool's result as the host answered it, a result
  /// the tool marked failed among them.
  pub fn call(
    &self,
    tool: &str,
    reached_by: &str,
    arguments: Map<String, Value>,
  ) -> Result<Value, Refusal> {
    self.ask(reached_by, json!({ "tool": tool, "arguments": arguments }))
  }

  // One `tools/call` of a toolbook tool, standing alone: its result, or the host's refusal of the request.
  fn ask(&self, name: &str, arguments: Value) -> Result<Value, Refusal> {
    let request = json!({
      "jsonrpc": "2.0",
      "id": 1,
      "method": "tools/call",
      "params": { "name": name, "arguments": arguments },
    });
    let mut response = self
      .agent
      .post(&self.url)
      .header("Authorization", format!("Bearer {}", self.token))
      .header("Accept", "application/json, text/event-stream")
      .send_json(&request)
      .map_err(|why| {
        Refusal::new("unreachable", format!("the tool host at {} is out of reach: {why}", self.url))
      })?;
    let status = response.status();
    let body: Value = response.body_mut().with_config().limit(u64::MAX).read_json().map_err(|why| {
      Refusal::new("unread", format!("the tool host answered {status} with no JSON: {why}"))
    })?;

    if !status.is_success() {
      let code = body["code"].as_str().unwrap_or("refused");
      let why = body["error"]
        .as_str()
        .map_or_else(|| format!("the tool host answered {status}"), str::to_owned);

      return Err(Refusal::new(code, why));
    }

    if let Some(error) = body.get("error") {
      return Err(Refusal::new(
        "refused",
        error["message"].as_str().unwrap_or("the toolbook refused the call"),
      ));
    }

    Ok(body.get("result").cloned().unwrap_or(Value::Null))
  }
}

/// The refusal a result carries where the tool or the toolbook marked it failed.
pub fn refusal_of(result: &Value) -> Option<Refusal> {
  result["isError"].as_bool().filter(|failed| *failed).map(|_| {
    let said = texts(result).unwrap_or_default().join("\n");

    Refusal::new(
      "refused",
      if said.is_empty() { "the tool answered that it failed".to_owned() } else { said },
    )
  })
}

/// Each content piece's text, where every piece is text.
pub fn texts(result: &Value) -> Option<Vec<String>> {
  result["content"]
    .as_array()?
    .iter()
    .map(|piece| (piece["type"] == "text").then(|| piece["text"].as_str().map(str::to_owned)).flatten())
    .collect()
}

/// The answer as printed: its structured content as JSON where it has some, else each text piece where every
/// piece is text, else its content pieces as one JSON array.
pub fn printed(result: &Value) -> String {
  let structured = &result["structuredContent"];

  if !structured.is_null() {
    return format!("{}\n", serde_json::to_string_pretty(structured).unwrap_or_default());
  }

  match texts(result) {
    Some(texts) => texts.iter().map(|text| format!("{text}\n")).collect(),
    None => format!("{}\n", serde_json::to_string_pretty(&result["content"]).unwrap_or_default()),
  }
}

/// The arguments the words give: the `--json` object, each `--arg` laid over it, a value typed by the tool's
/// input schema; or why they do not read.
pub fn arguments(json: Option<&str>, args: &[String], input: &Value) -> Result<Map<String, Value>, String> {
  let mut arguments = match json {
    Some(json) => match serde_json::from_str(json) {
      Ok(Value::Object(object)) => object,
      Ok(_) => return Err("--json takes one JSON object".to_owned()),
      Err(why) => return Err(format!("--json does not read as JSON: {why}")),
    },
    None => Map::new(),
  };

  for arg in args {
    let (key, value) =
      arg.split_once('=').ok_or_else(|| format!("--arg takes <key>=<value>, not `{arg}`"))?;
    let typed = if takes_string(&input["properties"][key]["type"]) {
      Value::String(value.to_owned())
    } else {
      serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_owned()))
    };

    arguments.insert(key.to_owned(), typed);
  }

  Ok(arguments)
}

// Whether a schema's `type` names a string.
fn takes_string(kind: &Value) -> bool {
  match kind {
    Value::String(kind) => kind == "string",
    Value::Array(kinds) => kinds.iter().any(|kind| kind == "string"),
    _ => false,
  }
}

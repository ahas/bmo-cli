//! A stand-in tool host and a stand-in receiver on the loopback address, each answering the requests the tool
//! verb makes and keeping every request it was sent.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::{Value, json};
use tiny_http::{Header, Response, Server};

/// The token the stand-in tool host takes.
pub const SESSION_TOKEN: &str = "session-token";
/// The token the stand-in receiver takes.
pub const RECEIVER_TOKEN: &str = "receiver-token";
/// The instruction id the stand-in receiver answers.
pub const INSTRUCTION: i64 = 7;

/// A request a stand-in was sent: its method, path, bearer header and JSON body.
#[derive(Debug, Clone)]
pub struct Sent {
  pub method: String,
  pub path: String,
  pub bearer: String,
  pub body: Value,
}

/// A stand-in serving on a port of its own until the test ends.
pub struct StandIn {
  pub port: u16,
  sent: Arc<Mutex<Vec<Sent>>>,
}

impl StandIn {
  /// Every request sent so far.
  pub fn sent(&self) -> Vec<Sent> {
    self.sent.lock().unwrap().clone()
  }

  /// The address it is reached at.
  pub fn address(&self) -> String {
    format!("http://127.0.0.1:{}", self.port)
  }
}

/// A tool host whose toolbook holds `web/read` (a read answering text), `web/shape` (a read answering
/// structured content) and `bmo/say` (an act), lists them, narrowed by a query naming `say`, and refuses every
/// other tool and every other token.
pub fn tool_host() -> StandIn {
  serve(|sent| match (sent.method.as_str(), sent.path.as_str()) {
    // A host older than the calls face answers no route there.
    ("POST" | "GET", path) if path.starts_with("/use") => {
      (404, json!({ "code": "no-route", "error": "no route answers this" }))
    }
    _ => toolbook_answer(sent),
  })
}

// A tool host whose calls face answers: a call asked is kept and answered, its record readable back.
pub fn tool_host_with_calls() -> StandIn {
  let kept: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
  let held = Arc::clone(&kept);

  serve(move |sent| match (sent.method.as_str(), sent.path.as_str()) {
    ("POST", "/use") if sent.bearer == format!("Bearer {SESSION_TOKEN}") => {
      let named = sent.body["tool"].as_str().unwrap_or("tool").replace('/', "-");
      let id = format!("{named}-00000007");
      let mut record = sent.body.clone();

      record["id"] = json!(id);
      record["state"] = json!("running");
      held.lock().unwrap().push(record);

      (200, json!({ "id": id, "state": "running", "record": format!("/kept/use/{id}") }))
    }
    ("GET", "/use") if sent.bearer == format!("Bearer {SESSION_TOKEN}") => {
      (200, json!({ "items": held.lock().unwrap().clone() }))
    }
    _ if sent.path.starts_with("/use/") && sent.bearer == format!("Bearer {SESSION_TOKEN}") => {
      let id = sent.path.trim_start_matches("/use/");

      match held.lock().unwrap().iter().find(|call| call["id"] == json!(id)) {
        Some(call) => (200, call.clone()),
        None => (404, json!({ "code": "not-found", "error": format!("call {id} not found") })),
      }
    }
    _ => toolbook_answer(sent),
  })
}

// The toolbook's own answer to a request on the tools face.
fn toolbook_answer(sent: &Sent) -> (u16, Value) {
  if sent.bearer != format!("Bearer {SESSION_TOKEN}") {
    return (401, json!({ "code": "unauthorized", "error": "the token names no session" }));
  }

  let params = &sent.body["params"];
  let tool = params["arguments"]["tool"].as_str().unwrap_or_default().to_owned();
  let arguments = &params["arguments"]["arguments"];
  let result = match (params["name"].as_str().unwrap_or_default(), tool.as_str()) {
    ("describe", "web/read") => described(
      "web/read",
      "call",
      "Reads a page.",
      json!({ "properties": { "name": { "type": "string" }, "count": { "type": "integer" } }, "type": "object" }),
    ),
    ("describe", "web/shape") => described("web/shape", "call", "Shapes.", json!({})),
    ("describe", "bmo/say") => described(
      "bmo/say",
      "act",
      "Says a line.",
      json!({ "properties": { "text": { "type": "string" } }, "type": "object" }),
    ),
    ("list", _) => {
      let items: Vec<Value> = [
        ("bmo/say", "act", "Says a line."),
        ("web/read", "call", "Reads a page."),
        ("web/shape", "call", "Shapes."),
      ]
      .into_iter()
      .filter(|(tool, ..)| params["arguments"]["query"].as_str().is_none_or(|query| tool.contains(query)))
      .map(
        |(tool, reached_by, summary)| json!({ "tool": tool, "reachedBy": reached_by, "summary": summary }),
      )
      .collect();

      json!({
        "content": [{ "type": "text", "text": "the list's lines" }],
        "structuredContent": { "items": items },
        "isError": false,
      })
    }
    ("call", "web/read") => text(&format!("read {arguments}\nsecond line")),
    ("call", "web/shape") => json!({
      "content": [{ "type": "text", "text": "{\"shape\":\"round\"}" }],
      "structuredContent": { "shape": "round" },
      "isError": false,
    }),
    ("call", "bmo/say") => failed("bmo/say acts, so call does not reach it: call it with act."),
    ("act", "bmo/say" | "web/read") => text(&format!("acted {arguments}")),
    _ => failed(&format!("ERROR: Tool {tool} not found")),
  };

  (200, json!({ "jsonrpc": "2.0", "id": sent.body["id"], "result": result }))
}

/// A receiver that queues every instruction it is sent with its token as instruction 7.
pub fn receiver() -> StandIn {
  serve(receiver_answer)
}

/// A receiver that refuses the first `refusals` requests it is sent, as one restarting does, then answers as
/// `receiver` does.
pub fn receiver_refusing_first(refusals: usize) -> StandIn {
  let answered = AtomicUsize::new(0);

  serve(move |sent| {
    if answered.fetch_add(1, Ordering::SeqCst) < refusals {
      return (503, json!({ "code": "unavailable", "error": "the receiver is restarting" }));
    }

    receiver_answer(sent)
  })
}

// The receiver's own answer to a request.
fn receiver_answer(sent: &Sent) -> (u16, Value) {
  match (sent.method.as_str(), sent.path.as_str()) {
    _ if sent.bearer != format!("Bearer {RECEIVER_TOKEN}") => {
      (401, json!({ "code": "unauthorized", "error": "the token is missing or wrong" }))
    }
    ("POST", "/instructions") => (201, json!({ "id": INSTRUCTION })),
    _ => (404, json!({ "code": "no-route", "error": "no route answers this" })),
  }
}

// A tool described as the toolbook describes one: its text, and the tool's shape as structured content.
fn described(tool: &str, reached_by: &str, description: &str, input: Value) -> Value {
  let (namespace, name) = tool.split_once('/').unwrap();

  json!({
    "content": [{ "type": "text", "text": format!("{tool} — reached with {reached_by}\n{description}") }],
    "structuredContent": {
      "namespace": namespace,
      "name": name,
      "description": description,
      "manual": format!("Call {tool} well."),
      "input": input,
      "hints": { "readOnly": reached_by == "call" },
      "reachedBy": reached_by,
    },
    "isError": false,
  })
}

fn text(text: &str) -> Value {
  json!({ "content": [{ "type": "text", "text": text }], "isError": false })
}

fn failed(text: &str) -> Value {
  json!({ "content": [{ "type": "text", "text": text }], "isError": true })
}

// A server on a free loopback port answering each request with `answer`, keeping each request.
fn serve(answer: impl Fn(&Sent) -> (u16, Value) + Send + 'static) -> StandIn {
  let server = Server::http("127.0.0.1:0").unwrap();
  let port = server.server_addr().to_ip().unwrap().port();
  let sent = Arc::new(Mutex::new(Vec::new()));
  let kept = Arc::clone(&sent);

  thread::spawn(move || {
    for mut request in server.incoming_requests() {
      let mut body = String::new();
      let _ = request.as_reader().read_to_string(&mut body);
      let bearer = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Authorization"))
        .map(|header| header.value.to_string())
        .unwrap_or_default();
      let one = Sent {
        method: request.method().to_string(),
        path: request.url().to_owned(),
        bearer,
        body: serde_json::from_str(&body).unwrap_or(Value::Null),
      };
      let (status, answered) = answer(&one);

      kept.lock().unwrap().push(one);

      let response = Response::from_string(answered.to_string())
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap());
      let _ = request.respond(response);
    }
  });

  StandIn { port, sent }
}

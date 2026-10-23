//! The records of asynchronous calls: one folder each under the data root's `use/`, holding the request, the
//! answer and the end (docs/api.md, 「The tool verb」).

use std::fs::DirBuilder;
use std::hash::{BuildHasher as _, RandomState};
use std::io::Write as _;
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The file the request is kept in.
pub const REQUEST: &str = "request.json";
/// The file the answer is kept in.
pub const ANSWER: &str = "answer.json";
/// The file the end is kept in.
pub const END: &str = "end.json";

/// A call as it was asked.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
  pub id: String,
  pub tool: String,
  pub reached_by: String,
  pub arguments: Map<String, Value>,
  pub memory: Option<String>,
  pub room: Option<i64>,
  pub agent: Option<String>,
  pub via: Option<String>,
  pub unit: String,
  pub started_at: i64,
}

/// How a call ended, and whether its wake was queued.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct End {
  pub ended_at: i64,
  pub state: String,
  pub summary: String,
  pub wake: Value,
}

/// The folder every record stands in.
pub fn folder(root: &Path) -> PathBuf {
  root.join("use")
}

/// One record's folder.
pub fn place(root: &Path, id: &str) -> PathBuf {
  folder(root).join(id)
}

/// A fresh id for a call of `tool`: its namespace and name joined by `-`, then eight hexadecimal digits.
pub fn new_id(tool: &str) -> String {
  let named: String = tool
    .chars()
    .map(|character| if well_formed(character) { character } else { '-' })
    .collect();
  let nonce = RandomState::new().hash_one(SystemTime::now()) as u32;

  format!("{named}-{nonce:08x}")
}

/// Whether `id` can name a record: letters, digits, `_` and `-` alone.
pub fn readable_id(id: &str) -> bool {
  !id.is_empty() && id.chars().all(well_formed)
}

fn well_formed(character: char) -> bool {
  character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
}

/// Now, in epoch milliseconds.
pub fn now() -> i64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as i64)
}

/// The record of `request` made, readable by the user alone, its request kept.
pub fn create(root: &Path, request: &Request) -> Result<PathBuf, String> {
  let place = place(root, &request.id);

  DirBuilder::new()
    .recursive(true)
    .mode(0o700)
    .create(folder(root))
    .and_then(|()| DirBuilder::new().mode(0o700).create(&place))
    .map_err(|why| format!("{}: {why}", place.display()))?;
  keep(&place.join(REQUEST), request)?;

  Ok(place)
}

/// `value` kept as JSON at `path`, readable by the user alone, written whole or not at all.
pub fn keep(path: &Path, value: &impl Serialize) -> Result<(), String> {
  let text = serde_json::to_string_pretty(value).map_err(|why| why.to_string())?;
  let written = path.with_extension("json.part");

  std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(&written)
    .and_then(|mut file| file.write_all(format!("{text}\n").as_bytes()))
    .and_then(|()| std::fs::rename(&written, path))
    .map_err(|why| format!("{}: {why}", path.display()))
}

/// The JSON kept at `path`, where it stands and reads.
pub fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
  serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// A record's request and its end, where it has one.
pub fn of(root: &Path, id: &str) -> Option<(Request, Option<End>)> {
  if !readable_id(id) {
    return None;
  }

  let place = place(root, id);

  Some((read(&place.join(REQUEST))?, read(&place.join(END))))
}

/// A record's state: its end's, else `running` while its unit is active, else `lost`.
pub fn state(request: &Request, end: Option<&End>) -> String {
  match end {
    Some(end) => end.state.clone(),
    None if active(&request.unit) => "running".to_owned(),
    None => "lost".to_owned(),
  }
}

// Whether the user unit `unit` is active.
fn active(unit: &str) -> bool {
  Command::new("systemctl")
    .args(["--user", "is-active", "--quiet", unit])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()
    .is_ok_and(|status| status.success())
}

/// Every record kept, oldest first.
pub fn every(root: &Path) -> Vec<(Request, Option<End>)> {
  let mut records: Vec<(Request, Option<End>)> = std::fs::read_dir(folder(root))
    .into_iter()
    .flatten()
    .filter_map(Result::ok)
    .filter_map(|entry| of(root, entry.file_name().to_str()?))
    .collect();

  records.sort_by(|(one, _), (other, _)| (one.started_at, &one.id).cmp(&(other.started_at, &other.id)));
  records
}

//! The records of asynchronous calls a shell keeps itself, and the folders the tool host keeps its own under: one
//! folder each, holding the request, the answer, the end and the lock its keeper holds (docs/api.md, 「The tool
//! verb」).

use std::fs::{DirBuilder, File};
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
/// The file whichever process keeps the record at a moment holds locked.
pub const LOCK: &str = "lock";

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

impl End {
  /// Whether the wake is still owed: the receiver has queued it only where the end names its instruction's id.
  pub fn owed(&self) -> bool {
    self.wake["id"].as_i64().is_none()
  }
}

/// The folder every record stands in, under the data root: what an older caller keeps, and what the host keeps.
pub fn folder(root: &Path) -> PathBuf {
  root.join("use")
}

/// The folder the calls this shell keeps stand in: `BMO_USE_RECORD_DIR` where it names one, else the workspace's
/// own `.bmo/use` where it stands, else the data root's `use`. A shell sandboxed to its workspace writes only the
/// second.
pub fn records(root: &Path) -> PathBuf {
  if let Some(named) = super::ground::variable("BMO_USE_RECORD_DIR") {
    return PathBuf::from(named);
  }

  let workspace = workspace();

  if workspace.is_dir() { workspace } else { folder(root) }
}

/// The folder a call this shell keeps itself stands in: `BMO_USE_RECORD_DIR` where it names one, else `.bmo/use` in
/// the workspace the command runs in, the one place a shell sandboxed to it may write.
pub fn workspace_records() -> PathBuf {
  super::ground::variable("BMO_USE_RECORD_DIR").map_or_else(workspace, PathBuf::from)
}

// `.bmo/use` in the directory the command runs in.
fn workspace() -> PathBuf {
  std::env::current_dir().unwrap_or_default().join(".bmo/use")
}

/// One record's folder, inside `records`.
pub fn place(records: &Path, id: &str) -> PathBuf {
  records.join(id)
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

/// The record of `request` made, readable by the user alone, its request kept: its folder, and its lock held
/// until the hold is dropped.
pub fn create(records: &Path, request: &Request) -> Result<(PathBuf, File), String> {
  let place = place(records, &request.id);

  DirBuilder::new()
    .recursive(true)
    .mode(0o700)
    .create(records)
    .and_then(|()| DirBuilder::new().mode(0o700).create(&place))
    .map_err(|why| format!("{}: {why}", place.display()))?;

  let held = hold(&place)?;

  keep(&place.join(REQUEST), request)?;

  Ok((place, held))
}

/// The lock of the record at `place`, waited for: held until the file answered is dropped.
pub fn hold(place: &Path) -> Result<File, String> {
  let lock = lock_file(place)?;

  lock.lock().map_err(|why| format!("{}: {why}", place.join(LOCK).display()))?;

  Ok(lock)
}

/// The lock of the record at `place` where no other process holds it.
pub fn try_hold(place: &Path) -> Option<File> {
  let lock = lock_file(place).ok()?;

  lock.try_lock().ok()?;

  Some(lock)
}

// The record's lock file, made where it does not stand, readable by the user alone.
fn lock_file(place: &Path) -> Result<File, String> {
  std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(false)
    .mode(0o600)
    .open(place.join(LOCK))
    .map_err(|why| format!("{}: {why}", place.join(LOCK).display()))
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
pub fn of(records: &Path, id: &str) -> Option<(Request, Option<End>)> {
  if !readable_id(id) {
    return None;
  }

  let place = place(records, id);

  Some((read(&place.join(REQUEST))?, read(&place.join(END))))
}

/// A record's state: its end's, else `running` while its unit stands, else `lost`.
pub fn state(request: &Request, end: Option<&End>) -> String {
  match end {
    Some(end) => end.state.clone(),
    None if standing(&request.unit) => "running".to_owned(),
    None => "lost".to_owned(),
  }
}

/// Whether the user unit `unit` stands: active, or starting or stopping, its last word running among them.
pub fn standing(unit: &str) -> bool {
  Command::new("systemctl")
    .args(["--user", "is-active", unit])
    .stdin(Stdio::null())
    .stderr(Stdio::null())
    .output()
    .is_ok_and(|said| {
      matches!(
        String::from_utf8_lossy(&said.stdout).trim(),
        "active" | "activating" | "deactivating" | "reloading" | "refreshing"
      )
    })
}

/// Every record kept, oldest first.
pub fn every(records: &Path) -> Vec<(Request, Option<End>)> {
  let mut found: Vec<(Request, Option<End>)> = std::fs::read_dir(records)
    .into_iter()
    .flatten()
    .filter_map(Result::ok)
    .filter_map(|entry| of(records, entry.file_name().to_str()?))
    .collect();

  found.sort_by(|(one, _), (other, _)| (one.started_at, &one.id).cmp(&(other.started_at, &other.id)));
  found
}

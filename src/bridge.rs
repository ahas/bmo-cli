//! The node's end of the home bridge (docs/api.md, 「The home bridge」): run on a work node, where
//! `BMO_HOME_BRIDGE` names the bridge's socket, a home-bound command is handed whole to the operations
//! machine: the binary's path under the deploy tree, every word, the run's own variables and the directory it
//! was asked from, then its standing input. What comes back, output, errors and exit code, is this run's. A
//! bridge whose socket refuses the connection, or stands nowhere, refuses the run, never passes it.

use std::ffi::OsString;
use std::io::{Read as _, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::json;

// What the asker sends after its request: a piece of its standing input, and the empty piece that ends it.
const IN: u8 = 1;
const END: u8 = 0;
// What the bridge answers in: output, errors, and the exit code that ends the answer.
const OUT: u8 = 1;
const ERR: u8 = 2;
const EXIT: u8 = 3;

/// The binary as the bridge names it under the deploy tree.
pub const BINARY_UNDER_TREE: &str = "bin/bmo";

/// The bridge's socket, where this run stands on a node.
pub fn socket() -> Option<PathBuf> {
  std::env::var_os("BMO_HOME_BRIDGE").filter(|named| !named.is_empty()).map(PathBuf::from)
}

fn reach(socket: &Path) -> Result<UnixStream, String> {
  UnixStream::connect(socket).map_err(|why| why.to_string())
}

fn frame(channel: u8, bytes: &[u8]) -> Vec<u8> {
  let mut framed = Vec::with_capacity(bytes.len() + 5);

  framed.push(channel);
  framed.extend_from_slice(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_be_bytes());
  framed.extend_from_slice(bytes);
  framed
}

// The variables a run hands to the operations machine: its own, and the agent program's.
fn handed_variables() -> serde_json::Map<String, serde_json::Value> {
  std::env::vars()
    .filter(|(name, _)| name.starts_with("BMO_") || name.starts_with("CLAUDE"))
    .map(|(name, value)| (name, value.into()))
    .collect()
}

fn dropped() -> ExitCode {
  eprintln!("home bridge: the operations machine dropped {BINARY_UNDER_TREE} before it ended");

  ExitCode::from(2)
}

/// The run handed to the operations machine over the bridge at `socket` with every word `bmo` was given,
/// answering the code it ended with.
pub fn run_home(socket: &Path, words: &[OsString]) -> ExitCode {
  let node = std::env::var("BMO_NODE").unwrap_or_else(|_| "this node".to_owned());
  let mut stream = match reach(socket) {
    Ok(stream) => stream,
    Err(why) => {
      eprintln!(
        "home bridge: the operations machine is out of reach from {node} ({why}); {BINARY_UNDER_TREE} was not run"
      );

      return ExitCode::from(2);
    }
  };
  let args: Vec<String> = words.iter().map(|word| word.to_string_lossy().into_owned()).collect();
  let request = json!({
    "script": BINARY_UNDER_TREE,
    "args": args,
    "env": handed_variables(),
    "cwd": std::env::current_dir().unwrap_or_default(),
  });

  if stream.write_all(format!("{request}\n").as_bytes()).is_err() {
    return dropped();
  }

  // The standing input goes as frames and an empty frame says it ended; the socket stays open both ways,
  // since one closed for writing here could no longer carry the answer back.
  if let Ok(mut input_side) = stream.try_clone() {
    std::thread::spawn(move || {
      let mut buffer = vec![0; 64 * 1024];
      let mut input = std::io::stdin();

      while let Ok(read) = input.read(&mut buffer) {
        if read == 0 || input_side.write_all(&frame(IN, &buffer[..read])).is_err() {
          break;
        }
      }

      let _ = input_side.write_all(&frame(END, &[]));
    });
  }

  loop {
    let mut head = [0u8; 5];

    if stream.read_exact(&mut head).is_err() {
      return dropped();
    }

    let mut body = vec![0; u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize];

    if stream.read_exact(&mut body).is_err() {
      return dropped();
    }

    match head[0] {
      OUT => {
        let mut output = std::io::stdout();
        let _ = output.write_all(&body);
        let _ = output.flush();
      }
      ERR => {
        let _ = std::io::stderr().write_all(&body);
      }
      EXIT => {
        let code = body
          .get(..4)
          .map_or(1, |bytes| i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));

        return ExitCode::from(u8::try_from(code).unwrap_or(1));
      }
      _ => {}
    }
  }
}

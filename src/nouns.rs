//! The nouns a unit command declares when asked `nouns` (docs/api.md, 「The nouns a unit declares」). The
//! answer is this platform's own protocol, one noun a line with its marks after a tab, so it is read here.

use std::io::Read as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// How long a unit command is given to answer `nouns`.
pub const ANSWER_WITHIN: Duration = Duration::from_secs(2);

// How often a command still answering is looked at again.
const LOOK_AGAIN: Duration = Duration::from_millis(5);

/// One declared noun: its words, and the marks its line carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Noun {
  pub words: Vec<String>,
  pub marks: Vec<String>,
}

impl Noun {
  /// The noun as a person writes it: its words with one space between them.
  pub fn spelled(&self) -> String {
    self.words.join(" ")
  }

  /// Whether its line carries `mark`.
  pub fn marked(&self, mark: &str) -> bool {
    self.marks.iter().any(|given| given == mark)
  }
}

/// The nouns an answer declares; the protocol's own word `nouns` and empty lines declare none.
pub fn read(answer: &str) -> Vec<Noun> {
  answer
    .lines()
    .filter_map(|line| {
      let (noun, marks) = line.split_once('\t').unwrap_or((line, ""));
      let words: Vec<String> = noun.split_whitespace().map(str::to_owned).collect();

      (!words.is_empty() && words != ["nouns"]).then(|| Noun {
        words,
        marks: marks.split_whitespace().map(str::to_owned).collect(),
      })
    })
    .collect()
}

/// The nouns the command at `path` declares: none where it cannot be run, ends with any code but 0, or does
/// not end within `within`, when it is stopped.
pub fn ask(path: &Path, within: Duration) -> Vec<Noun> {
  let Ok(mut child) = Command::new(path)
    .arg("nouns")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
  else {
    return Vec::new();
  };
  let Some(mut output) = child.stdout.take() else { return Vec::new() };
  let (sent, answered) = mpsc::channel();

  // The answer is read beside the wait, so a long answer never stalls the command on a full pipe.
  std::thread::spawn(move || {
    let mut answer = Vec::new();
    let _ = output.read_to_end(&mut answer);
    let _ = sent.send(answer);
  });

  let deadline = Instant::now() + within;
  let ended = loop {
    match child.try_wait() {
      Ok(Some(status)) => break status.success(),
      Ok(None) if Instant::now() < deadline => std::thread::sleep(LOOK_AGAIN),
      _ => {
        let _ = child.kill();
        let _ = child.wait();

        break false;
      }
    }
  };

  if !ended {
    return Vec::new();
  }

  // A process the command left behind may still hold the pipe; the answer is waited for only until the deadline.
  let left = deadline.saturating_duration_since(Instant::now()).max(LOOK_AGAIN);

  match answered.recv_timeout(left) {
    Ok(answer) => read(&String::from_utf8_lossy(&answer)),
    Err(_) => Vec::new(),
  }
}

//! The nouns a unit command declares when asked `nouns` (docs/api.md, 「The nouns a unit declares」). The
//! answer is this platform's own protocol, one noun a line with its marks after a tab, so it is read here.

use std::io::Read as _;
use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

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

/// How long a unit command is given to answer `nouns`: the whole number of milliseconds `BMO_NOUNS_WAIT_MS`
/// names, else five seconds.
pub fn bound() -> Duration {
  std::env::var("BMO_NOUNS_WAIT_MS")
    .ok()
    .and_then(|named| named.trim().parse().ok())
    .map_or(DEFAULT_BOUND, Duration::from_millis)
}

const DEFAULT_BOUND: Duration = Duration::from_secs(5);

/// The nouns the command at `path` declares, once it has ended and its output has closed: none where it cannot be
/// run or ends with any code but 0. Where it has not ended and closed its output within `bound`, it is killed
/// with its process group and the answer is `None`.
pub fn ask(path: &Path, bound: Duration) -> Option<Vec<Noun>> {
  let Ok(mut child) = Command::new(path)
    .arg("nouns")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .process_group(0)
    .spawn()
  else {
    return Some(Vec::new());
  };
  let leader = child.id();
  let Some(mut output) = child.stdout.take() else { return Some(Vec::new()) };
  let (sent, answered) = mpsc::channel();

  // The answer is read to its end before the wait, so a long answer never stalls the command on a full pipe.
  std::thread::spawn(move || {
    let mut answer = Vec::new();
    let whole = output.read_to_end(&mut answer).is_ok();
    let ended = child.wait().is_ok_and(|status| status.success());
    let _ = sent.send((whole && ended).then_some(answer));
  });

  match answered.recv_timeout(bound) {
    Ok(answer) => Some(answer.map(|answer| read(&String::from_utf8_lossy(&answer))).unwrap_or_default()),
    Err(_) => {
      kill_group(leader);
      // The killed command is reaped before `bmo` goes on, so none is left behind it.
      let _ = answered.recv_timeout(bound);

      None
    }
  }
}

// Every process in the group `leader` leads, killed.
fn kill_group(leader: u32) {
  let Ok(group) = libc::pid_t::try_from(leader) else { return };

  // SAFETY: `kill` touches no memory of this process; a negative id names a process group.
  unsafe {
    libc::kill(-group, libc::SIGKILL);
  }
}

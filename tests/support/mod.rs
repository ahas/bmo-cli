//! A bench for running `bmo` as a caller does: a directory of stand-in unit commands on `PATH`, a cache
//! directory and a home of its own, all removed with the bench.

#![allow(dead_code)]

pub mod stand_ins;

use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

/// The nouns `bmo-job` declares, as its command line document lists them.
pub const JOB_NOUNS: [&str; 15] = [
  "serve",
  "job-run",
  "job",
  "node-land",
  "todo",
  "work-order",
  "worker",
  "overtime",
  "checkout setup",
  "room jobs",
  "research admit",
  "research run",
  "research close",
  "research branch",
  "nouns",
];

pub struct Bench {
  root: TempDir,
}

impl Bench {
  pub fn new() -> Self {
    let bench = Self {
      root: tempfile::tempdir().unwrap(),
    };

    for directory in ["bin", "cache", "home"] {
      std::fs::create_dir(bench.root.path().join(directory)).unwrap();
    }

    bench
  }

  /// The bench's own directory, for files a test keeps beside the commands.
  pub fn path(&self) -> &Path {
    self.root.path()
  }

  /// The directory on `PATH` the stand-in commands stand in.
  pub fn bin(&self) -> PathBuf {
    self.root.path().join("bin")
  }

  /// A stand-in command named `name` in `directory`, running `body` under `sh`.
  pub fn command_in(&self, directory: &Path, name: &str, body: &str) -> PathBuf {
    let path = directory.join(name);

    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

    path
  }

  /// A stand-in command on `PATH`.
  pub fn command(&self, name: &str, body: &str) -> PathBuf {
    self.command_in(&self.bin(), name, body)
  }

  /// A stand-in unit command that declares `nouns` and, asked anything else, prints its name and then each
  /// word it was handed on a line of its own.
  pub fn unit(&self, name: &str, nouns: &[&str]) -> PathBuf {
    self.command(name, &unit_body(name, nouns))
  }

  /// `bmo` run with `words`, nothing on its standard input.
  pub fn bmo(&self, words: &[&str]) -> Output {
    run(self.bmo_command(words), None)
  }

  /// `bmo` run with `words`, `input` on its standard input and `variables` in its environment beside the
  /// bench's own.
  pub fn bmo_given(&self, words: &[&str], input: &str, variables: &[(&str, &str)]) -> Output {
    let mut command = self.bmo_command(words);

    command.envs(variables.iter().copied());
    run(command, Some(input))
  }

  /// The `bmo` this package built, run with the bench's environment alone.
  pub fn bmo_command(&self, words: &[&str]) -> Command {
    self.bmo_at(Path::new(env!("CARGO_BIN_EXE_bmo")), words)
  }

  /// A `bmo` binary at `binary`, run with the bench's environment alone.
  pub fn bmo_at(&self, binary: &Path, words: &[&str]) -> Command {
    let mut command = Command::new(binary);

    command
      .args(words)
      .env_clear()
      .env("PATH", format!("{}:/usr/bin:/bin", self.bin().display()))
      .env("XDG_CACHE_HOME", self.root.path().join("cache"))
      .env("HOME", self.root.path().join("home"));
    command
  }
}

/// The body of a stand-in unit command: its nouns when asked, else its name and the words it was handed.
pub fn unit_body(name: &str, nouns: &[&str]) -> String {
  let declared: Vec<String> = nouns.iter().map(|noun| format!("'{noun}'")).collect();

  format!(
    "if [ \"$1\" = nouns ]; then printf '%s\\n' {}; exit 0; fi\necho {name}\nfor word in \"$@\"; do echo \"$word\"; done",
    declared.join(" ")
  )
}

/// Standard output as lines.
pub fn lines(output: &Output) -> Vec<String> {
  String::from_utf8_lossy(&output.stdout).lines().map(str::to_owned).collect()
}

/// Standard error as text.
pub fn errors(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Runs `command`, writing `input` to it where given, and waits for it.
pub fn run(mut command: Command, input: Option<&str>) -> Output {
  command.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
  command.stdout(Stdio::piped()).stderr(Stdio::piped());

  let mut child = command.spawn().unwrap();

  if let Some(input) = input {
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
  }

  child.wait_with_output().unwrap()
}

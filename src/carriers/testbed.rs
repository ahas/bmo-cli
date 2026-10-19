//! bmo-testbed's commands under the platform's words: each handed to the `bmo-testbed` verb its boundary
//! documents (docs/boundary.md, 「bmo-testbed」). None of them reads the caller's road, so it is not handed on.

use std::process::ExitCode;

use crate::carried::{Call, Carried, Carrier};

const UNIT: &str = "bmo-testbed";

// Each command beside the verb it is handed to.
const VERBS: [(&str, &str); 11] = [
  ("test suite", "suite"),
  ("test count", "count"),
  ("test log", "log"),
  ("test template", "template"),
  ("test full", "full"),
  ("check", "check"),
  ("imports", "imports"),
  ("cargo", "cargo"),
  ("cargo prune", "prune"),
  ("deploy commit", "deploy"),
  ("deploy backups", "backups"),
];

const COMMANDS: [&str; VERBS.len()] = {
  let mut commands = [""; VERBS.len()];
  let mut at = 0;

  while at < VERBS.len() {
    commands[at] = VERBS[at].0;
    at += 1;
  }

  commands
};

struct Testbed;

impl Carried for Testbed {
  fn unit(&self) -> &'static str {
    UNIT
  }

  fn commands(&self) -> &'static [&'static str] {
    &COMMANDS
  }

  fn run(&self, call: &Call<'_>) -> ExitCode {
    let verb = VERBS
      .iter()
      .find(|(command, _)| *command == call.command)
      .map_or(call.command, |(_, verb)| verb);

    call.forward(UNIT, &[verb])
  }
}

inventory::submit! { Carrier(&Testbed) }

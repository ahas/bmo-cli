//! bmo-artifact's commands: its command `bmo-artifact-cli` runs each under the same words the platform's
//! shell gives them, its documented second names among them (docs/boundary.md, 「bmo-artifact」). The search
//! and its runner read the caller's road; the rest do not, and it is not handed to them.

use std::process::ExitCode;

use crate::carried::{Call, Carried, Carrier};

const UNIT: &str = "bmo-artifact-cli";

// The commands whose words name the road a result answers on.
const READ_THE_ROAD: [&str; 3] = ["search", "runner search", "search-runner"];

struct Artifact;

impl Carried for Artifact {
  fn unit(&self) -> &'static str {
    UNIT
  }

  fn commands(&self) -> &'static [&'static str] {
    &[
      "archive doc",
      "archive-doc",
      "archive share",
      "archive-share",
      "archive html-read",
      "search",
      "search branch",
      "research-branch",
      "runner search",
      "search-runner",
    ]
  }

  fn run(&self, call: &Call<'_>) -> ExitCode {
    let words: Vec<&str> = call.command.split(' ').collect();

    if READ_THE_ROAD.contains(&call.command) {
      call.forward_with_road(UNIT, &words)
    } else {
      call.forward(UNIT, &words)
    }
  }
}

inventory::submit! { Carrier(&Artifact) }

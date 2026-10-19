//! bmo-node's commands (docs/boundary.md, 「bmo-node」): the node list and the Windows sensor setup, each
//! handed to the `bmo-node` command under the words its documents give, with the configuration file the
//! command reads.

use std::process::ExitCode;

use crate::carried::{Call, Carried, Carrier};
use crate::{answer, settings};

const UNIT: &str = "bmo-node";

// Each command beside the words `bmo-node` takes it under after its configuration.
const VERBS: [(&str, &str); 2] = [("node list", "node"), ("node hardware", "hardware-setup")];

struct Node;

impl Carried for Node {
  fn unit(&self) -> &'static str {
    UNIT
  }

  fn commands(&self) -> &'static [&'static str] {
    &["node list", "node hardware"]
  }

  fn run(&self, call: &Call<'_>) -> ExitCode {
    let config = match settings::unit(UNIT) {
      Ok(table) => table
        .get("config")
        .and_then(toml::Value::as_str)
        .filter(|config| !config.is_empty())
        .map(str::to_owned),
      Err(why) => return answer::unsettled(&call.words(), UNIT, &why),
    };
    let Some(config) = config else {
      return answer::unsettled(&call.words(), UNIT, &format!("the settings name no `unit.{UNIT}.config`"));
    };
    let verb = VERBS
      .iter()
      .find(|(command, _)| *command == call.command)
      .map_or(call.command, |(_, verb)| verb);

    call.forward(UNIT, &["--config", &config, verb])
  }

  fn home(&self, call: &Call<'_>) -> bool {
    call.command == "node list"
  }
}

inventory::submit! { Carrier(&Node) }

//! bmo-node's commands (docs/boundary.md, 「bmo-node」): the node list and the Windows sensor setup handed to
//! `bmo-node` with its configuration file, the node list home-bound.

mod support;

use support::{Bench, errors, lines};

// The node's command answers no `nouns`: asked it, it refuses the word for want of its configuration.
fn node(bench: &Bench) {
  bench.command(
    "bmo-node",
    "if [ \"$1\" = nouns ]; then exit 2; fi\necho bmo-node\nfor word in \"$@\"; do echo \"$word\"; done",
  );
  bench.settings("[unit.bmo-node]\nconfig = \"conf/node.toml\"\n");
}

#[test]
fn the_node_list_and_the_sensor_setup_reach_the_node_with_its_configuration() {
  let bench = Bench::new();
  node(&bench);

  assert_eq!(
    lines(&bench.bmo(&["node", "list", "--set", "n2", "--ssh", "host"])),
    ["bmo-node", "--config", "conf/node.toml", "node", "--set", "n2", "--ssh", "host"]
  );
  assert_eq!(
    lines(&bench.bmo(&["node", "hardware", "n2", "n3"])),
    ["bmo-node", "--config", "conf/node.toml", "hardware-setup", "n2", "n3"]
  );
}

#[test]
fn a_node_command_without_its_configuration_in_the_settings_is_refused_naming_it() {
  let bench = Bench::new();
  bench.command("bmo-node", "exit 0");

  let output = bench.bmo(&["node", "list"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `node list` cannot be asked of bmo-node: the settings name no `unit.bmo-node.config`\n"
  );
}

#[test]
fn a_node_command_without_the_node_installed_names_it() {
  let bench = Bench::new();
  bench.settings("[unit.bmo-node]\nconfig = \"conf/node.toml\"\n");

  let output = bench.bmo(&["node", "hardware"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(errors(&output), "bmo: `node hardware` is not served yet: `bmo-node` is not installed\n");
}

#[test]
fn the_node_list_goes_home_on_a_node_and_the_sensor_setup_stays() {
  let bench = Bench::new();
  node(&bench);
  let absent = bench.path().join("absent.sock");
  let on_a_node = [("BMO_HOME_BRIDGE", absent.to_str().unwrap()), ("BMO_HOME_BRIDGE_WAIT_MS", "1")];

  let listed = bench.bmo_given(&["node", "list"], "", &on_a_node);
  let set_up = bench.bmo_given(&["node", "hardware"], "", &on_a_node);

  assert_eq!(listed.status.code(), Some(2));
  assert!(errors(&listed).starts_with("home bridge: the operations machine is out of reach"));
  assert!(set_up.status.success(), "{}", errors(&set_up));
}

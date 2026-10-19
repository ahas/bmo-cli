//! bmo-agency's commands (docs/boundary.md, 「bmo-agency」): handed to `bmo-agency` under its own words, a
//! worker's keepalive paused at a stand-in cache warmer, the home-bound ones handed home on a work node, and
//! the rest not served.

mod support;

use serde_json::json;
use support::stub::Stub;
use support::{Bench, errors, lines};

// Each command carried, as the platform's words, beside the words `bmo-agency` is handed.
const HANDED: [(&str, &str); 24] = [
  ("agent", "agent"),
  ("agents", "agents"),
  ("agent status", "agent-status"),
  ("agent-status", "agent-status"),
  ("model", "model"),
  ("models", "models"),
  ("coworker pool", "coworker pool"),
  ("coworker hook", "coworker hook"),
  ("worker fold", "worker fold"),
  ("run list", "run list"),
  ("run stop", "run stop"),
  ("machine memory", "machine memory"),
  ("reload", "reload"),
  ("node work", "node work"),
  ("work-node", "work-node"),
  ("node claude-token", "node claude-token"),
  ("node pane", "node pane"),
  ("node-pane", "node-pane"),
  ("dev-server", "dev-server"),
  ("dev-pane", "dev-pane"),
  ("companion follow", "companion follow"),
  ("companion reset", "companion reset"),
  ("claude batch", "claude batch"),
  ("live-claude", "live-claude"),
];

// The agency's commands neither its documents nor its command carry.
const NOT_SERVED: [&str; 4] = ["usage", "fasttrack", "companion say", "companion settings"];

// The agency's command declares no nouns: asked `nouns`, it refuses the word.
fn agency(bench: &Bench) {
  bench.command(
    "bmo-agency",
    "if [ \"$1\" = nouns ]; then exit 2; fi\necho bmo-agency\nfor word in \"$@\"; do echo \"$word\"; done",
  );
}

#[test]
fn every_carried_command_reaches_the_agency_under_its_words() {
  let bench = Bench::new();
  agency(&bench);

  for (command, words) in HANDED {
    let mut asked: Vec<&str> = command.split(' ').collect();
    asked.extend(["--flag", "w1"]);
    let mut handed = vec!["bmo-agency"];
    handed.extend(words.split(' '));
    handed.extend(["--flag", "w1"]);

    let output = bench.bmo(&asked);

    assert!(output.status.success(), "{command}: {}", errors(&output));
    assert_eq!(lines(&output), handed, "{command}");
  }
}

#[test]
fn a_dev_servers_action_and_a_job_s_live_claude_reach_the_agency_whole() {
  let bench = Bench::new();
  agency(&bench);

  assert_eq!(
    lines(&bench.bmo(&["dev-server", "restart", "w1"])),
    ["bmo-agency", "dev-server", "restart", "w1"]
  );
  assert_eq!(
    lines(&bench.bmo(&["live-claude", "--purpose", "p"])),
    ["bmo-agency", "live-claude", "--purpose", "p"]
  );
}

#[test]
fn a_carried_command_without_the_agency_installed_names_it() {
  let bench = Bench::new();

  let output = bench.bmo(&["worker", "fold", "w1"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `worker fold w1` is not served yet: `bmo-agency` is not installed\n"
  );
}

#[test]
fn a_workers_keepalive_is_paused_at_the_cache_warmer() {
  let stub = Stub::answering(&[(
    "/consumers/agency/subjects/w%201/pause",
    200,
    json!({ "subject": "w 1", "switch": null, "paused_at": 1, "targets": 0 }),
  )]);
  let bench = Bench::new();
  agency(&bench);
  bench.settings(&format!("[unit.bmo-cache-warmer]\naddress = \"{}\"\n", stub.address));

  let output = bench.bmo(&["agent", "--pause-keepalive", "w 1"]);
  let asked = stub.asked();

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(lines(&output), ["worker w 1: keepalive off until its next job"]);
  assert_eq!(asked.len(), 1);
  assert_eq!(asked[0].path, "/consumers/agency/subjects/w%201/pause");
  assert_eq!(asked[0].authorization, None);
}

#[test]
fn the_warmers_consumer_and_its_refusal_are_its_own() {
  let stub = Stub::answering(&[(
    "/consumers/agents/subjects/w1/pause",
    404,
    json!({ "error": "no consumer is registered under that name", "code": "unknown_consumer" }),
  )]);
  let bench = Bench::new();
  bench.settings(&format!(
    "[unit.bmo-cache-warmer]\naddress = \"{}\"\nconsumer = \"agents\"\ntoken = \"w\"\n",
    stub.address
  ));

  let output = bench.bmo(&["agent", "--pause-keepalive=w1"]);

  assert_eq!(output.status.code(), Some(1));
  assert_eq!(errors(&output), "no consumer is registered under that name\n");
  assert_eq!(stub.asked()[0].authorization.as_deref(), Some("Bearer w"));
}

#[test]
fn a_pause_without_the_warmer_in_the_settings_is_refused_naming_the_setting() {
  let bench = Bench::new();

  let output = bench.bmo(&["agent", "--pause-keepalive", "w1"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `agent --pause-keepalive w1` cannot be asked of bmo-cache-warmer: the settings name no `unit.bmo-cache-warmer.address`\n"
  );
}

#[test]
fn any_other_agent_words_go_to_the_agency() {
  let bench = Bench::new();
  agency(&bench);

  assert_eq!(
    lines(&bench.bmo(&["agent", "--set", "w1", "--keepalive", "off"])),
    ["bmo-agency", "agent", "--set", "w1", "--keepalive", "off"]
  );
}

#[test]
fn the_home_bound_commands_go_home_on_a_node_and_the_shell_guard_stays() {
  let bench = Bench::new();
  agency(&bench);
  let absent = bench.path().join("absent.sock");
  let node = [("BMO_HOME_BRIDGE", absent.to_str().unwrap()), ("BMO_HOME_BRIDGE_WAIT_MS", "1")];

  for command in ["agent", "agent status", "coworker hook", "node claude-token", "node work", "run list"] {
    let output = bench.bmo_given(&command.split(' ').collect::<Vec<_>>(), "", &node);

    assert_eq!(output.status.code(), Some(2), "{command}");
    assert!(
      errors(&output).starts_with("home bridge: the operations machine is out of reach"),
      "{command}"
    );
  }

  for local in [&["agent", "status", "shell"][..], &["model"], &["run", "stop"]] {
    let output = bench.bmo_given(local, "", &node);

    assert!(output.status.success(), "{local:?}: {}", errors(&output));
    assert_eq!(lines(&output)[0], "bmo-agency", "{local:?}");
  }
}

#[test]
fn the_agencys_commands_nothing_carries_are_not_served() {
  let bench = Bench::new();
  agency(&bench);

  for command in NOT_SERVED {
    let output = bench.bmo(&command.split(' ').collect::<Vec<_>>());

    assert_eq!(output.status.code(), Some(2), "{command}");
    assert_eq!(
      errors(&output),
      format!("bmo: `{command}` is not served yet: no installed unit carries it\n")
    );
  }
}

//! bmo-testbed's commands (docs/boundary.md, 「bmo-testbed」): each handed to the verb the testbed's boundary
//! documents, the rest not served.

mod support;

use support::{Bench, errors, lines};

// Each command, as the platform's words, beside the `bmo-testbed` verb it is handed to.
const HANDED: [(&str, &str); 11] = [
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

// The testbed's commands its boundary documents no verb for, and the two retired ones.
const NOT_SERVED: [&str; 9] = [
  "edit",
  "deploy tool",
  "deploy web",
  "deploy backlog",
  "deploy main",
  "deploy down-alert",
  "machine stand-in",
  "backup snapshot",
  "backup drill",
];

// The testbed's command declares no nouns: asked `nouns`, it refuses the word as it refuses any it does not
// carry.
fn testbed(bench: &Bench) {
  bench.command(
    "bmo-testbed",
    "if [ \"$1\" = nouns ]; then exit 2; fi\necho bmo-testbed\nfor word in \"$@\"; do echo \"$word\"; done",
  );
}

#[test]
fn every_carried_command_reaches_its_testbed_verb_with_the_rest_of_its_words() {
  let bench = Bench::new();
  testbed(&bench);

  for (command, verb) in HANDED {
    let mut words: Vec<&str> = command.split(' ').collect();
    words.extend(["--since", "abc"]);

    let output = bench.bmo(&words);

    assert!(output.status.success(), "{command}: {}", errors(&output));
    assert_eq!(lines(&output), ["bmo-testbed", verb, "--since", "abc"], "{command}");
  }
}

#[test]
fn the_callers_road_is_not_handed_to_the_testbed() {
  let bench = Bench::new();
  testbed(&bench);

  assert_eq!(
    lines(&bench.bmo(&["--via", "r", "test", "suite", "tests/a"])),
    ["bmo-testbed", "suite", "tests/a"]
  );
}

#[test]
fn the_testbed_s_own_words_reach_it_by_the_git_rule() {
  let bench = Bench::new();
  testbed(&bench);

  assert_eq!(
    lines(&bench.bmo(&["testbed", "run", "--", "cargo", "check"])),
    ["bmo-testbed", "run", "--", "cargo", "check"]
  );
  assert_eq!(lines(&bench.bmo(&["testbed", "status", "--json"])), ["bmo-testbed", "status", "--json"]);
}

#[test]
fn a_carried_command_without_the_testbed_installed_names_it() {
  let bench = Bench::new();

  let output = bench.bmo(&["test", "suite", "--since", "abc"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `test suite --since abc` is not served yet: `bmo-testbed` is not installed\n"
  );
}

#[test]
fn the_commands_the_testbed_documents_no_verb_for_are_not_served() {
  let bench = Bench::new();
  testbed(&bench);

  for command in NOT_SERVED {
    let output = bench.bmo(&command.split(' ').collect::<Vec<_>>());

    assert_eq!(output.status.code(), Some(2), "{command}");
    assert_eq!(
      errors(&output),
      format!("bmo: `{command}` is not served yet: no installed unit carries it\n")
    );
  }
}

#[test]
fn the_help_lists_the_carried_commands_beside_the_testbed() {
  let bench = Bench::new();

  let help = String::from_utf8_lossy(&bench.bmo(&["--help"]).stdout).into_owned();

  for (command, _) in HANDED {
    assert!(
      help
        .lines()
        .any(|line| line.trim_start().starts_with(&format!("{command} ")) && line.ends_with("bmo-testbed")),
      "{command}: {help}"
    );
  }
}

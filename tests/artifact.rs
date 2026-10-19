//! bmo-artifact's commands (docs/boundary.md, 「bmo-artifact」): handed to `bmo-artifact-cli` under the same
//! words, the road only to the commands that read it.

mod support;

use support::{Bench, errors, lines};

// Every command carried for bmo-artifact, its documented second names among them.
const CARRIED: [&str; 10] = [
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
];

// The artifact's command declares no nouns: asked `nouns`, it refuses the word.
fn artifact(bench: &Bench) {
  bench.command(
    "bmo-artifact-cli",
    "if [ \"$1\" = nouns ]; then exit 1; fi\necho bmo-artifact-cli\nfor word in \"$@\"; do echo \"$word\"; done",
  );
}

#[test]
fn every_carried_command_reaches_the_artifact_under_its_own_words() {
  let bench = Bench::new();
  artifact(&bench);

  for command in CARRIED {
    let mut words: Vec<&str> = command.split(' ').collect();
    words.extend(["--chat", "7"]);

    let output = bench.bmo(&words);

    assert!(output.status.success(), "{command}: {}", errors(&output));
    assert_eq!(lines(&output), [vec!["bmo-artifact-cli"], words].concat(), "{command}");
  }
}

#[test]
fn the_road_goes_only_to_the_search_and_its_runner() {
  let bench = Bench::new();
  artifact(&bench);

  assert_eq!(
    lines(&bench.bmo(&["--via", "r", "search", "--chat", "7"])),
    ["bmo-artifact-cli", "search", "--via", "r", "--chat", "7"]
  );
  assert_eq!(
    lines(&bench.bmo(&["--via", "r", "runner", "search", "7", "q"])),
    ["bmo-artifact-cli", "runner", "search", "--via", "r", "7", "q"]
  );
  assert_eq!(
    lines(&bench.bmo(&["--via", "r", "archive", "doc", "--title", "t"])),
    ["bmo-artifact-cli", "archive", "doc", "--title", "t"]
  );
}

#[test]
fn a_carried_command_without_the_artifact_installed_names_it() {
  let bench = Bench::new();

  let output = bench.bmo(&["archive", "share", "12", "--chat", "7"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `archive share 12 --chat 7` is not served yet: `bmo-artifact-cli` is not installed\n"
  );
}

#[test]
fn the_news_brief_is_not_the_artifacts_and_is_not_served() {
  let bench = Bench::new();
  artifact(&bench);

  for command in ["news-brief", "runner news-brief"] {
    let output = bench.bmo(&command.split(' ').collect::<Vec<_>>());

    assert_eq!(output.status.code(), Some(2), "{command}");
    assert_eq!(
      errors(&output),
      format!("bmo: `{command}` is not served yet: no installed unit carries it\n")
    );
  }
}

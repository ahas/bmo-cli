//! The dispatch contract (docs/api.md): unit commands found, their nouns read and kept, a command resolved
//! and handed on, and what `bmo` answers itself.

use crate::support;

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use support::{Bench, JOB_NOUNS, errors, lines};

// The commands bmo-job's nouns serve, each under its own words.
const JOB_COMMANDS: [&str; 18] = [
  "job ask",
  "job borrow",
  "job branches",
  "job close",
  "job cowork-timing",
  "job figures",
  "job guard",
  "job land",
  "job live-claude",
  "job start",
  "job tool-calls",
  "job wait",
  "todo",
  "todo korean-records",
  "work-order",
  "overtime",
  "checkout setup",
  "room jobs",
];

// Commands whose first word begins no noun an installed unit declares and names no unit command.
const NOT_SERVED: [&str; 18] = [
  "gate eval",
  "browse",
  "voice say",
  "song",
  "file share",
  "query",
  "graph",
  "memory",
  "note",
  "watch set",
  "digest run",
  "test suite",
  "check",
  "cargo",
  "deploy commit",
  "archive doc",
  "tool list",
  "agent",
];

fn split(command: &str) -> Vec<&str> {
  command.split(' ').collect()
}

#[test]
fn every_command_of_bmo_job_reaches_it_with_all_its_words() {
  let bench = Bench::new();
  bench.unit("bmo-job", &JOB_NOUNS);

  for command in JOB_COMMANDS.iter().copied().chain(["node-land"]) {
    let mut words = split(command);
    words.extend(["--flag", "value"]);

    let output = bench.bmo(&words);

    assert!(output.status.success(), "{command}: {}", errors(&output));
    assert_eq!(lines(&output), [vec!["bmo-job"], words].concat(), "{command}");
  }
}

#[test]
fn the_old_words_of_node_land_are_not_bmo_jobs_noun() {
  let bench = Bench::new();
  bench.unit("bmo-job", &JOB_NOUNS);

  let output = bench.bmo(&["node", "land", "--branch", "b"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `node land --branch b` is not served yet: no installed unit carries it\n"
  );
}

#[test]
fn every_command_no_unit_declares_is_not_served() {
  let bench = Bench::new();
  bench.unit("bmo-job", &JOB_NOUNS);

  for command in NOT_SERVED {
    let output = bench.bmo(&split(command));

    assert_eq!(output.status.code(), Some(2), "{command}");
    assert!(output.stdout.is_empty(), "{command}");
    assert_eq!(
      errors(&output),
      format!("bmo: `{command}` is not served yet: no installed unit carries it\n")
    );
  }
}

#[test]
fn leading_words_that_begin_declared_nouns_list_them() {
  let bench = Bench::new();
  bench.unit("bmo-testbed", &["deploy commit", "deploy backups", "test suite"]);
  bench.unit("bmo-job", &JOB_NOUNS);

  for asked in [
    &["deploy"][..],
    &["deploy", "--help"],
    &["deploy", "foo"],
    &["help", "deploy"],
    &["--via", "r", "deploy"],
  ] {
    let output = bench.bmo(asked);

    assert_eq!(output.status.code(), Some(2), "{asked:?}");
    assert!(output.stdout.is_empty(), "{asked:?}");
    assert_eq!(
      errors(&output),
      "bmo: `deploy` is not a command of its own; the nouns beginning with it are:\n  \
       deploy backups  bmo-testbed\n  \
       deploy commit   bmo-testbed\n",
      "{asked:?}"
    );
  }

  let output = bench.bmo(&["research"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `research` is not a command of its own; the nouns beginning with it are:\n  \
     research admit   bmo-job\n  \
     research branch  bmo-job\n  \
     research close   bmo-job\n  \
     research run     bmo-job\n"
  );

  let output = bench.bmo(&["room", "say"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `room` is not a command of its own; the nouns beginning with it are:\n  room jobs  bmo-job\n"
  );
}

#[test]
fn the_longest_run_of_words_beginning_nouns_is_named() {
  let bench = Bench::new();
  bench.unit("bmo-one", &["backup run now", "backup list"]);
  bench.unit("bmo-two", &["backup run later"]);

  let output = bench.bmo(&["backup", "run", "--quiet"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `backup run` is not a command of its own; the nouns beginning with it are:\n  \
     backup run later  bmo-two\n  \
     backup run now    bmo-one\n"
  );
}

#[test]
fn the_longest_declared_noun_wins() {
  let bench = Bench::new();
  bench.unit("bmo-wide", &["room"]);
  bench.unit("bmo-narrow", &["room say"]);

  assert_eq!(
    lines(&bench.bmo(&["room", "say", "--text", "t"])),
    ["bmo-narrow", "room", "say", "--text", "t"]
  );
  assert_eq!(lines(&bench.bmo(&["room", "mute"])), ["bmo-wide", "room", "mute"]);
}

#[test]
fn a_noun_two_unit_commands_declare_is_refused_naming_both() {
  let bench = Bench::new();
  bench.unit("bmo-one", &["room say"]);
  bench.unit("bmo-two", &["room say", "room"]);

  let output = bench.bmo(&["room", "say"]);

  assert_eq!(output.status.code(), Some(2));
  assert!(output.stdout.is_empty());
  assert_eq!(errors(&output), "bmo: `room say` is declared by both `bmo-one` and `bmo-two`\n");
}

#[test]
fn the_git_rule_hands_the_words_after_the_first_to_the_command_it_names() {
  let bench = Bench::new();
  // The word game's command declares no nouns: asked `nouns`, it refuses the word.
  bench.command(
    "bmo-word-game",
    "if [ \"$1\" = nouns ]; then exit 2; fi\necho bmo-word-game\nfor word in \"$@\"; do echo \"$word\"; done",
  );

  let output = bench.bmo(&["word-game", "install", "--from", "book"]);

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(lines(&output), ["bmo-word-game", "install", "--from", "book"]);
}

#[test]
fn a_unit_that_declares_no_nouns_is_reached_by_the_git_rule() {
  let bench = Bench::new();
  bench.command(
    "bmo-note",
    "if [ \"$1\" = nouns ]; then exit 2; fi\necho bmo-note\nfor word in \"$@\"; do echo \"$word\"; done",
  );

  assert_eq!(lines(&bench.bmo(&["note", "list", "mine"])), ["bmo-note", "list", "mine"]);
  assert_eq!(lines(&bench.bmo(&["note"])), ["bmo-note"]);
}

#[test]
fn a_road_named_before_the_command_follows_the_noun_words() {
  let bench = Bench::new();
  bench.unit("bmo-job", &JOB_NOUNS);
  bench.unit("bmo-pocket", &[]);

  assert_eq!(
    lines(&bench.bmo(&["--via", "r1", "todo", "--add", "x"])),
    ["bmo-job", "todo", "--via", "r1", "--add", "x"]
  );
  assert_eq!(
    lines(&bench.bmo(&["--via=r2", "checkout", "setup", "--quiet"])),
    ["bmo-job", "checkout", "setup", "--via", "r2", "--quiet"]
  );
  assert_eq!(
    lines(&bench.bmo(&["--via", "r3", "pocket", "put", "f"])),
    ["bmo-pocket", "--via", "r3", "put", "f"]
  );
  // A road named after the first word is one of the command's words.
  assert_eq!(lines(&bench.bmo(&["todo", "--via", "r4"])), ["bmo-job", "todo", "--via", "r4"]);
}

#[test]
fn the_exit_code_the_environment_and_the_input_are_the_callers() {
  let bench = Bench::new();
  bench.command(
    "bmo-job",
    "if [ \"$1\" = nouns ]; then echo job; exit 0; fi\necho \"$BENCH_WORD\"\ncat\nexit 7",
  );

  let output = bench.bmo_given(&["job", "close"], "from the caller\n", &[("BENCH_WORD", "handed")]);

  assert_eq!(output.status.code(), Some(7));
  assert_eq!(lines(&output), ["handed", "from the caller"]);
}

#[test]
fn the_help_lists_every_noun_and_unit_command() {
  let bench = Bench::new();
  bench.unit("bmo-job", &JOB_NOUNS);
  bench.unit("bmo-pocket", &[]);

  for asked in [&["--help"][..], &["-h"], &["help"]] {
    let output = bench.bmo(asked);
    let help = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "{asked:?}");
    assert!(help.contains("Usage: bmo [--via <ROAD>] <WORDS>..."), "{help}");

    for noun in JOB_NOUNS.iter().filter(|noun| **noun != "nouns") {
      assert!(
        help
          .lines()
          .any(|line| line.trim_start().starts_with(noun) && line.ends_with("bmo-job")),
        "{noun}: {help}"
      );
    }

    assert!(help.lines().any(|line| line.trim() == "pocket  bmo-pocket"), "{help}");
    assert!(!help.contains("Carried:"), "{help}");
    assert!(!help.lines().any(|line| line.trim_start().starts_with("nouns ")), "{help}");
  }
}

#[test]
fn no_words_print_the_help_as_a_refusal() {
  let bench = Bench::new();

  let output = bench.bmo(&[]);

  assert_eq!(output.status.code(), Some(2));
  assert!(output.stdout.is_empty());
  assert!(errors(&output).contains("Usage: bmo [--via <ROAD>] <WORDS>..."));
}

#[test]
fn help_with_words_asks_the_unit_command_for_its_own_help() {
  let bench = Bench::new();
  bench.unit("bmo-job", &JOB_NOUNS);

  assert_eq!(lines(&bench.bmo(&["help", "work-order"])), ["bmo-job", "work-order", "--help"]);
}

#[test]
fn the_version_is_the_packages() {
  let bench = Bench::new();

  let output = bench.bmo(&["--version"]);

  assert!(output.status.success());
  assert_eq!(lines(&output), [format!("bmo {}", env!("CARGO_PKG_VERSION"))]);
}

#[test]
fn an_unchanged_command_is_asked_its_nouns_once_and_a_changed_one_again() {
  let bench = Bench::new();
  let asked = bench.path().join("asked");
  let body = |noun: &str| {
    format!("if [ \"$1\" = nouns ]; then echo x >> \"$ASKED\"; echo {noun}; exit 0; fi\necho \"$1\"")
  };
  let variables = [("ASKED", asked.to_str().unwrap())];
  let times_asked = || std::fs::read_to_string(&asked).unwrap_or_default().lines().count();

  bench.command("bmo-unit", &body("first"));

  assert_eq!(lines(&bench.bmo_given(&["first"], "", &variables)), ["first"]);
  assert_eq!(lines(&bench.bmo_given(&["first"], "", &variables)), ["first"]);
  assert_eq!(times_asked(), 1);

  bench.command("bmo-unit", &body("second-noun"));

  assert_eq!(lines(&bench.bmo_given(&["second-noun"], "", &variables)), ["second-noun"]);
  assert_eq!(times_asked(), 2);
  assert_eq!(bench.bmo_given(&["first"], "", &variables).status.code(), Some(2));
  assert_eq!(times_asked(), 2);
}

#[test]
fn a_unit_command_that_never_answers_its_nouns_is_left_out_killed_and_asked_again() {
  let bench = Bench::new();
  let asked = bench.path().join("asked");
  bench.unit("bmo-job", &["job"]);
  bench.command(
    "bmo-stuck",
    "if [ \"$1\" = nouns ]; then echo $$ >> \"$ASKED\"; exec sleep 600; fi\necho bmo-stuck",
  );
  let variables = [("ASKED", asked.to_str().unwrap()), ("BMO_NOUNS_WAIT_MS", "500")];
  let started = || -> Vec<String> {
    std::fs::read_to_string(&asked).unwrap_or_default().lines().map(str::to_owned).collect()
  };

  for call in 1..=2 {
    let output = bench.bmo_given(&["job", "now"], "", &variables);
    let pids = started();
    // `bmo` reaps what it kills before it goes on, so a stand-in still listed under /proc was left running;
    // it is taken down here before anything is asserted.
    let left: Vec<&String> =
      pids.iter().filter(|pid| Path::new("/proc").join(pid.as_str()).exists()).collect();

    for pid in &left {
      let _ = std::process::Command::new("kill").args(["-KILL", pid.as_str()]).status();
    }

    assert_eq!(lines(&output), ["bmo-job", "job", "now"]);
    assert_eq!(
      errors(&output),
      "bmo: `bmo-stuck` did not answer `nouns` within 500 ms; its nouns are left out of this call\n"
    );
    assert_eq!(pids.len(), call, "asked again on every call");
    assert!(left.is_empty(), "left running: {left:?}");
  }
}

#[test]
fn a_call_with_a_narrower_path_keeps_the_answers_of_the_commands_it_does_not_find() {
  let bench = Bench::new();
  let asked = bench.path().join("asked");
  let other = bench.path().join("other");
  std::fs::create_dir(&other).unwrap();
  bench.unit("bmo-job", &["job"]);
  bench.command_in(
    &other,
    "bmo-far",
    "if [ \"$1\" = nouns ]; then echo x >> \"$ASKED\"; echo far; exit 0; fi\necho \"$1\"",
  );
  let wide_path = format!("{}:{}:/usr/bin:/bin", bench.bin().display(), other.display());
  let wide = [("ASKED", asked.to_str().unwrap()), ("PATH", wide_path.as_str())];
  let narrow = [("ASKED", asked.to_str().unwrap())];
  let times_asked = || std::fs::read_to_string(&asked).unwrap_or_default().lines().count();

  assert_eq!(lines(&bench.bmo_given(&["far"], "", &wide)), ["far"]);
  assert_eq!(lines(&bench.bmo_given(&["job"], "", &narrow)), ["bmo-job", "job"]);
  assert_eq!(lines(&bench.bmo_given(&["far"], "", &wide)), ["far"]);
  assert_eq!(times_asked(), 1);
}

#[test]
fn a_noun_line_with_marks_declares_its_noun() {
  let bench = Bench::new();
  bench.command(
    "bmo-brain",
    "if [ \"$1\" = nouns ]; then printf 'brain hold\\thome other\\n'; exit 0; fi\necho bmo-brain\necho \"$@\"",
  );

  assert_eq!(lines(&bench.bmo(&["brain", "hold"])), ["bmo-brain", "brain hold"]);
}

#[test]
fn the_directory_holding_bmo_is_searched_before_the_path() {
  let bench = Bench::new();
  let own = bench.path().join("release");
  std::fs::create_dir(&own).unwrap();
  let binary = own.join("bmo");
  std::fs::copy(env!("CARGO_BIN_EXE_bmo"), &binary).unwrap();
  std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
  bench.command_in(&own, "bmo-job", &support::unit_body("beside", &["todo"]));
  bench.unit("bmo-job", &["todo"]);

  let output = support::run(bench.bmo_at(&binary, &["todo"]), None);

  assert_eq!(lines(&output), ["beside", "todo"]);
}

#[test]
fn a_unit_command_that_cannot_start_is_answered_with_one() {
  let bench = Bench::new();
  let broken = bench.bin().join("bmo-broken");
  std::fs::write(&broken, "#!/nonexistent/interpreter\n").unwrap();
  std::fs::set_permissions(&broken, std::fs::Permissions::from_mode(0o755)).unwrap();

  let output = bench.bmo(&["broken", "now"]);

  assert_eq!(output.status.code(), Some(1));
  assert!(
    errors(&output).starts_with("bmo: `bmo-broken` could not be started: "),
    "{}",
    errors(&output)
  );
}

#[test]
fn an_option_before_the_first_word_that_bmo_does_not_read_is_refused() {
  let bench = Bench::new();
  bench.unit("bmo-job", &JOB_NOUNS);

  let output = bench.bmo(&["--quiet", "todo"]);

  assert_eq!(output.status.code(), Some(2));
  assert!(output.stdout.is_empty());
}

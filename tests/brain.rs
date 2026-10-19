//! bmo-brain's commands (docs/boundary.md, 「bmo-brain」): the memory rows, a note, the fragment table, the
//! digests and the brain's holds, each asked of a stand-in brain on the route it documents.

mod support;

use serde_json::{Value, json};
use support::stub::Stub;
use support::{Bench, errors, lines};

const AGENT: (&str, &str) = ("BMO_TURN_AGENT", "agent-a");

fn reading(lines: &[&str]) -> Value {
  json!({ "answered": true, "lines": lines })
}

// A bench whose settings reach `stub` as the brain.
fn reaching(stub: &Stub) -> Bench {
  let bench = Bench::new();

  bench.settings(&format!("[unit.bmo-brain]\naddress = \"{}\"\ntoken = \"b\"\n", stub.address));
  bench
}

#[test]
fn a_memory_row_is_filed_as_the_turns_agent_in_its_room() {
  let stub = Stub::answering(&[("/memory/add", 200, reading(&["filed #12 stretch"]))]);
  let bench = reaching(&stub);

  let output = bench.bmo_given(
    &[
      "memory", "--chat", "5", "--add", "--type", "errand", "--action", "-", "--on", "deployed", "--for",
      "9", "--every", "60", "--within", "30",
    ],
    "stretch",
    &[AGENT],
  );
  let asked = stub.asked();

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(lines(&output), ["filed #12 stretch"]);
  assert_eq!(asked[0].authorization.as_deref(), Some("Bearer b"));
  assert_eq!(
    asked[0].body,
    json!({
      "turn": { "agent": "agent-a", "chatId": 5, "messageId": null },
      "type": "errand",
      "action": "stretch",
      "on": "deployed",
      "every": 60,
      "within": 30,
      "for": 9,
    })
  );
}

#[test]
fn a_watch_is_set_as_its_agent_for_the_chat_setting_it() {
  let stub = Stub::answering(&[("/watches/set", 200, reading(&["watch #4 set"]))]);
  let bench = reaching(&stub);

  let output = bench.bmo_given(
    &[
      "watch",
      "set",
      "--chat",
      "6",
      "--by",
      "chat:5",
      "--until",
      "test -f done",
      "--name",
      "build",
      "--note",
      "tell 9",
      "--every",
      "60",
      "--within",
      "30",
      "--item",
      "12",
    ],
    "",
    &[AGENT],
  );

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(lines(&output), ["watch #4 set"]);
  assert_eq!(
    stub.asked()[0].body,
    json!({
      "turn": { "agent": "agent-a", "chatId": 5, "messageId": null },
      "note": "tell 9",
      "until": "test -f done",
      "name": "build",
      "every": 60,
      "within": 30,
      "item": 12,
      "chat": 6,
    })
  );
}

#[test]
fn a_watch_that_does_not_read_is_refused_with_its_usage() {
  let stub = Stub::answering(&[(
    "/watches/set",
    200,
    json!({ "answered": false, "refusal": { "code": "usage", "message": "a watch takes --until or --at" } }),
  )]);
  let bench = reaching(&stub);

  let unread = bench.bmo_given(&["watch", "set", "--by", "5", "--note", "x"], "", &[AGENT]);
  let refused = bench.bmo_given(&["watch", "set", "--by", "chat:5", "--note", "x"], "", &[AGENT]);
  let nobody = bench.bmo(&["watch", "set", "--by", "chat:5", "--note", "x"]);

  assert_eq!(unread.status.code(), Some(1));
  assert!(
    errors(&unread).starts_with("watch: --by takes chat:<chatId>, the chat setting it\nusage: bmo watch")
  );
  assert!(
    errors(&refused).starts_with("watch: a watch takes --until or --at\nusage: bmo watch"),
    "{}",
    errors(&refused)
  );
  assert!(errors(&nobody).starts_with("watch: no agent is named"));
  assert_eq!(stub.asked().len(), 1);
}

#[test]
fn a_chats_watches_are_listed_as_a_session_reads_them_and_one_is_called_off() {
  let stub = Stub::answering(&[
    ("/watches/list", 200, json!({ "watches": [], "lines": [] })),
    ("/watches/cancel", 200, reading(&["cancelled #4"])),
  ]);
  let bench = reaching(&stub);

  let listed = bench.bmo(&["watch", "list", "--by", "chat:5", "--all"]);
  let cancelled = bench.bmo(&["watch", "list", "--cancel", "4", "--by", "chat:5"]);
  let unread = bench.bmo(&["watch", "list", "--by", "5"]);

  assert_eq!(lines(&listed), ["(no watches)"]);
  assert_eq!(lines(&cancelled), ["cancelled #4"]);
  assert_eq!(
    (unread.status.code(), errors(&unread)),
    (
      Some(1),
      "usage: bmo watch list --by chat:<chatId> [--all] | --cancel <id> --by chat:<chatId>\n".to_owned()
    )
  );
  assert_eq!(
    stub.asked().into_iter().map(|asked| asked.body).collect::<Vec<_>>(),
    [json!({ "by": 5, "agent": null, "all": true }), json!({ "id": 4, "by": 5 })]
  );
}

#[test]
fn every_memory_verb_is_asked_of_its_route() {
  let stub = Stub::answering(&[
    ("/memory/update", 200, reading(&["moved"])),
    ("/memory/done", 200, reading(&["done"])),
    ("/memory/retire", 200, reading(&["down"])),
    ("/memory/revive", 200, reading(&["up"])),
    ("/memory/candidate", 200, reading(&["raised"])),
    ("/memory/list", 200, reading(&["#1", "#2"])),
  ]);
  let bench = reaching(&stub);

  for words in [
    &["memory", "--chat", "5", "--update", "7", "--action", "next"][..],
    &["memory", "--chat", "5", "--done", "stretch", "--text", "ok"],
    &["memory", "--global", "--retire", "7", "--why", "stale"],
    &["memory", "--chat", "5", "--revive", "7"],
    &["memory", "--global", "--candidate", "7", "--why", "common"],
    &["memory", "--global", "--list", "--all"],
  ] {
    assert!(bench.bmo_given(words, "", &[AGENT]).status.success(), "{words:?}");
  }

  let asked: Vec<(String, Value)> = stub
    .asked()
    .into_iter()
    .map(|asked| {
      let mut body = asked.body;
      body.as_object_mut().unwrap().remove("turn");
      (asked.path, body)
    })
    .collect();

  assert_eq!(
    asked,
    [
      ("/memory/update".to_owned(), json!({ "memory": 7, "action": "next" })),
      ("/memory/done".to_owned(), json!({ "memory": "stretch", "text": "ok" })),
      ("/memory/retire".to_owned(), json!({ "memory": 7, "why": "stale", "global": true })),
      ("/memory/revive".to_owned(), json!({ "memory": 7 })),
      ("/memory/candidate".to_owned(), json!({ "memory": 7, "why": "common", "global": true })),
      ("/memory/list".to_owned(), json!({ "global": true, "all": true })),
    ]
  );
}

#[test]
fn a_memory_call_that_does_not_read_is_refused_in_its_own_words_before_anything_is_asked() {
  let stub = Stub::answering(&[]);
  let bench = reaching(&stub);

  for (words, reason) in [
    (
      &["memory", "--chat", "5"][..],
      "give one of --add, --update, --done, --retire, --revive, --candidate or --list",
    ),
    (&["memory", "--chat", "5", "--add", "--list"], "give only one of --add, --list"),
    (
      &["memory", "--add"],
      "--chat <chatId> names the room, or --global for a row tied to none",
    ),
    (
      &["memory", "--chat", "5", "--global", "--list"],
      "--global and --chat name two different scopes; give one",
    ),
    (&["memory", "--chat", "5", "--add", "--why", "x"], "--why does not go with --add"),
    (
      &["memory", "--global", "--update", "7"],
      "--update moves a room's own state on, so it takes --chat rather than --global",
    ),
    (&["memory", "--chat", "five", "--list"], "--chat takes a whole number, not 'five'"),
  ] {
    let output = bench.bmo_given(words, "", &[AGENT]);

    assert_eq!(output.status.code(), Some(1), "{words:?}");
    assert!(
      errors(&output).starts_with(&format!("memory: {reason}\nusage: bmo memory")),
      "{}",
      errors(&output)
    );
  }

  let nobody = bench.bmo(&["memory", "--chat", "5", "--list"]);

  assert!(errors(&nobody).starts_with("memory: no agent is named"), "{}", errors(&nobody));
  assert!(stub.asked().is_empty());
}

#[test]
fn the_brains_refusal_is_the_memory_calls() {
  let stub = Stub::answering(&[(
    "/memory/done",
    200,
    json!({ "answered": false, "refusal": { "code": "no-row", "message": "no row 7 stands in chat 5" } }),
  )]);
  let bench = reaching(&stub);

  let output = bench.bmo_given(&["memory", "--chat", "5", "--done", "7"], "", &[AGENT]);

  assert_eq!(output.status.code(), Some(1));
  assert!(
    errors(&output).starts_with("memory: no row 7 stands in chat 5\nusage:"),
    "{}",
    errors(&output)
  );
}

#[test]
fn a_note_is_kept_for_its_room_as_the_agent_named() {
  let stub = Stub::answering(&[("/notes/add", 200, reading(&["noted"]))]);
  let bench = reaching(&stub);

  let output = bench.bmo_given(
    &["note", "--chat", "5", "--author", "9", "--global", "--agent", "agent-b"],
    "the words",
    &[],
  );
  let nobody = bench.bmo_given(&["note", "--chat", "5", "--text", "x"], "", &[]);

  assert_eq!(lines(&output), ["noted"]);
  assert_eq!(
    stub.asked()[0].body,
    json!({
      "turn": { "agent": "agent-b", "chatId": 5, "messageId": null },
      "text": "the words",
      "author": 9,
      "global": true,
    })
  );
  assert_eq!(nobody.status.code(), Some(1));
  assert!(errors(&nobody).starts_with("note: no agent is named"));
}

#[test]
fn the_fragment_table_is_listed_a_row_a_line_narrowed_as_asked() {
  let row = json!({
    "kind": "room-rule", "chatId": 5, "owner": null, "slug": "quiet", "title": "Quiet hours", "path": "rules/quiet.md",
    "scope": null, "active": true, "lastHitAt": 0, "hits": 3, "handAt": null, "keywords": ["night"], "items": [4],
    "writtenOn": "2026-10-01", "confirmedOn": null, "status": "fact",
  });
  let stub = Stub::answering(&[("/fragments", 200, json!({ "fragments": [row] }))]);
  let bench = reaching(&stub);

  let output = bench.bmo(&["fragment", "--kind", "room-rule", "--chat", "5", "--word", "night", "--all"]);

  assert_eq!(
    lines(&output),
    [
      "  room-rule · chat 5 · quiet: Quiet hours [rules/quiet.md] · keywords night · items #4 · written 2026-10-01 · fact · 3 hits, last 1970-01-01 00:00 UTC"
    ]
  );
  assert_eq!(stub.asked()[0].query.as_deref(), Some("kind=room-rule&chat=5&text=night&down=true"));
}

#[test]
fn an_empty_listing_says_what_was_asked_for() {
  let stub = Stub::answering(&[("/fragments", 200, json!({ "fragments": [] }))]);
  let bench = reaching(&stub);

  assert_eq!(lines(&bench.bmo(&["fragment", "--item", "4"])), ["no active fragments tied to #4"]);
}

#[test]
fn a_fragment_is_written_removed_synced_and_checked_on_its_routes() {
  let stub = Stub::answering(&[
    ("/fragments/write", 200, reading(&["quiet: Quiet hours"])),
    ("/fragments/remove", 200, reading(&["taken down: rules/old.md"])),
    ("/fragments/sync", 200, reading(&["room rules: 1 changed"])),
    (
      "/fragments/check",
      200,
      json!({
        "drift": [{ "trouble": "bodiless", "kind": "room-rule", "chatId": 5, "owner": null, "slug": "gone", "said": "no file" }],
        "titles": { "refused": [], "counted": [{ "kind": "room-rule", "refused": 0, "standing": 2 }] },
      }),
    ),
  ]);
  let bench = reaching(&stub);

  let wrote = bench.bmo(&[
    "fragment",
    "--write",
    "rules/quiet.md",
    "--title",
    "Quiet hours",
    "--keywords",
    "night, sleep",
    "--items",
    "4,5",
    "--chat",
    "5",
  ]);
  let removed = bench.bmo(&["fragment", "--remove", "rules/old.md"]);
  let synced = bench.bmo(&["fragment", "--sync"]);
  let checked = bench.bmo(&["fragment", "--check", "--kind", "room-rule"]);

  assert_eq!(lines(&wrote), ["quiet: Quiet hours"]);
  assert_eq!(lines(&removed), ["taken down: rules/old.md"]);
  assert_eq!(lines(&synced), ["room rules: 1 changed"]);
  assert_eq!(
    lines(&checked),
    [
      "bodiless: room-rule · chat 5 · gone: no file",
      "titles the shape check refuses: room-rule 0 of 2"
    ]
  );
  assert_eq!(
    stub.asked()[0].body,
    json!({
      "path": "rules/quiet.md", "title": "Quiet hours", "keywords": ["night", "sleep"], "items": [4, 5],
      "shared": null, "chatId": 5, "forEveryAgent": null,
    })
  );
  assert_eq!(stub.asked()[3].body, json!({ "kind": "room-rule" }));
}

#[test]
fn a_digests_launch_is_said_as_it_came_out() {
  let stub = Stub::answering(&[
    ("/digest/run", 200, json!({ "outcome": "nothing" })),
    ("/digest/now", 200, json!({ "outcome": "launched", "unit": "digest-5", "notes": 3 })),
    ("/digest/people", 200, json!({ "outcome": "refused", "why": "one-to-one" })),
    ("/digest/pause", 200, json!({ "pausedUntil": null, "lines": ["the digests run"] })),
  ]);
  let bench = reaching(&stub);

  let run = bench.bmo(&["digest", "run", "--chat", "5"]);
  let now = bench.bmo_given(&["digest", "now", "--chat", "5"], "", &[AGENT]);
  let people = bench.bmo(&["digest", "people", "--chat", "5", "--split", "--spans", "2", "--since"]);
  let pause = bench.bmo(&["digest", "pause"]);

  assert_eq!(lines(&run), ["no pending notes for chat 5; nothing to digest."]);
  assert_eq!(
    lines(&now),
    [
      "digest launched as digest-5 (3 notes); it updates the library by itself and sends nothing back to the chat."
    ]
  );
  assert_eq!(
    (people.status.code(), errors(&people)),
    (Some(1), "people digest: failed to launch the digest: one-to-one\n".to_owned())
  );
  assert_eq!(lines(&pause), ["the digests run"]);

  let bodies: Vec<Value> = stub.asked().into_iter().map(|asked| asked.body).collect();

  assert_eq!(
    bodies,
    [
      json!({ "chatId": 5 }),
      json!({ "agent": "agent-a", "chatId": 5 }),
      json!({ "chatId": 5, "split": true, "spans": 2, "since": null }),
      json!({ "set": "status" }),
    ]
  );
}

#[test]
fn the_brains_holds_are_read_and_waited_on() {
  let stub = Stub::answering(&[
    ("/brain/hold", 200, json!({ "holders": ["run-1"], "taken": [] })),
    (
      "/brain/wait",
      200,
      json!({ "free": false, "lines": ["still taken after nine minutes"] }),
    ),
  ]);
  let bench = reaching(&stub);

  let hold = bench.bmo(&["brain", "hold"]);
  let wait = bench.bmo(&["brain", "wait"]);

  assert_eq!(lines(&hold), ["held by: run-1", "every editing lock is free"]);
  assert_eq!(stub.asked()[0].method, "GET");
  assert_eq!(
    (wait.status.code(), lines(&wait)),
    (Some(1), vec!["still taken after nine minutes".to_owned()])
  );
}

#[test]
fn the_brains_holds_and_its_pause_go_home_on_a_node() {
  let stub = Stub::answering(&[]);
  let bench = reaching(&stub);
  let absent = bench.path().join("absent.sock");

  for words in [&["brain", "hold"][..], &["brain", "wait"], &["digest", "pause", "2h"]] {
    let output = bench.bmo_given(
      words,
      "",
      &[("BMO_HOME_BRIDGE", absent.to_str().unwrap()), ("BMO_HOME_BRIDGE_WAIT_MS", "1")],
    );

    assert_eq!(output.status.code(), Some(2), "{words:?}");
    assert!(
      errors(&output).starts_with("home bridge: the operations machine is out of reach"),
      "{words:?}"
    );
  }

  assert!(stub.asked().is_empty());
}

#[test]
fn a_brain_the_settings_do_not_reach_is_refused_naming_the_setting() {
  let output = Bench::new().bmo(&["brain", "hold"]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `brain hold` cannot be asked of bmo-brain: the settings name no `unit.bmo-brain.address`\n"
  );
}

#[test]
fn a_rooms_reminders_are_listed_as_a_session_reads_them_and_one_is_called_off() {
  let stub = Stub::answering(&[
    (
      "/reminders/list",
      200,
      json!({ "reminders": [{ "id": 3 }], "lines": ["#3 2026-10-20 09:00 stretch"] }),
    ),
    ("/reminders/cancel", 200, reading(&["cancelled #3"])),
  ]);
  let bench = reaching(&stub);

  let listed = bench.bmo(&["reminder", "list", "--chat", "5", "--all"]);
  let cancelled = bench.bmo(&["reminder", "list", "--cancel", "3"]);

  assert_eq!(lines(&listed), ["#3 2026-10-20 09:00 stretch"]);
  assert_eq!(lines(&cancelled), ["cancelled #3"]);
  assert_eq!(
    stub.asked().into_iter().map(|asked| (asked.path, asked.body)).collect::<Vec<_>>(),
    [
      ("/reminders/list".to_owned(), json!({ "chatId": 5, "agent": null, "all": true })),
      ("/reminders/cancel".to_owned(), json!({ "id": 3 })),
    ]
  );
}

#[test]
fn no_reminders_and_one_the_brain_does_not_hold_are_said_so() {
  let stub = Stub::answering(&[
    ("/reminders/list", 200, json!({ "reminders": [], "lines": [] })),
    (
      "/reminders/cancel",
      200,
      json!({ "answered": false, "refusal": { "code": "no-reminder", "message": "no reminder #9" } }),
    ),
  ]);
  let bench = reaching(&stub);

  let none = bench.bmo(&["reminder", "list", "--chat", "5"]);
  let unknown = bench.bmo(&["reminder", "list", "--cancel", "9"]);
  let unread = bench.bmo(&["reminder", "list"]);

  assert_eq!(lines(&none), ["(no reminders)"]);
  assert_eq!((unknown.status.code(), errors(&unknown)), (Some(1), "no reminder #9\n".to_owned()));
  assert_eq!(unread.status.code(), Some(2));
}

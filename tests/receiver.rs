//! bmo-receiver's commands (docs/boundary.md, 「bmo-receiver」): the sends, a fair draw and a reminder asked of
//! `POST /sends`, a consult line and block of `POST /consult/say` and `POST /consult/block`, against a
//! stand-in receiver; the rest not served.

mod support;

use serde_json::{Value, json};
use support::stub::Stub;
use support::{Bench, errors, lines};

// The receiver's commands no route serves today.
const NOT_SERVED: [&str; 17] = [
  "room mute",
  "room notice",
  "room photos",
  "room rules",
  "room file",
  "room upload",
  "room mp3",
  "runner say",
  "runner mp3",
  "owner tell",
  "archive scan",
  "archive photo-repair",
  "runner watch",
  "procedure show",
  "draw verify",
  "consult room",
  "room sticker --list",
];

fn said(lines: &[&str]) -> Value {
  json!({ "lines": lines })
}

// A bench whose settings reach `stub` as the receiver, with `owner` as the owner where one is given.
fn reaching(stub: &Stub, owner: Option<i64>) -> Bench {
  let bench = Bench::new();
  let owner = owner.map(|owner| format!("owner = {owner}\n")).unwrap_or_default();

  bench.settings(&format!(
    "[unit.bmo-receiver]\naddress = \"{}\"\ntoken = \"t0k\"\n{owner}",
    stub.address
  ));
  bench
}

const AGENT: (&str, &str) = ("BMO_TURN_AGENT", "agent-a");

#[test]
fn a_line_is_asked_of_the_sends_as_the_turns_agent_on_the_road_named() {
  let stub = Stub::answering(&[("/sends", 200, said(&["said into 5"]))]);
  let bench = reaching(&stub, None);

  let output = bench.bmo_given(
    &["--via", "r", "room", "say", "--chat", "5", "--by", "chat:9", "--reply", "--text", "hi"],
    "",
    &[AGENT],
  );
  let asked = stub.asked();

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(lines(&output), ["said into 5"]);
  assert_eq!(asked.len(), 1);
  assert_eq!(asked[0].path, "/sends");
  assert_eq!(asked[0].authorization.as_deref(), Some("Bearer t0k"));
  assert_eq!(
    asked[0].body,
    json!({
      "asker": { "chat": 5, "via": "r", "agent": "agent-a", "by": "chat:9" },
      "tool": "say",
      "input": { "text": "hi", "reply": true },
    })
  );
}

#[test]
fn a_line_without_a_room_goes_to_the_owner_and_its_words_come_from_the_input() {
  let stub = Stub::answering(&[("/sends", 200, said(&["ok"]))]);
  let bench = reaching(&stub, Some(42));

  let output = bench.bmo_given(&["room", "say"], "from the input", &[]);

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(stub.asked()[0].body["asker"], json!({ "chat": 42 }));
  assert_eq!(stub.asked()[0].body["input"], json!({ "text": "from the input" }));
}

#[test]
fn a_line_with_nothing_to_say_or_an_answer_nowhere_to_go_is_refused_before_it_is_asked() {
  let stub = Stub::answering(&[("/sends", 200, said(&[]))]);
  let bench = reaching(&stub, Some(42));

  let empty = bench.bmo_given(&["room", "say", "--text", "  "], "", &[]);
  let reply = bench.bmo_given(&["room", "say", "--chat", "5", "--reply", "--text", "x"], "", &[]);

  assert_eq!(empty.status.code(), Some(1));
  assert!(
    errors(&empty).starts_with("say: there are no words to send.\nusage: bmo room say"),
    "{}",
    errors(&empty)
  );
  assert_eq!(reply.status.code(), Some(1));
  assert!(
    errors(&reply).starts_with("say: an answer comes back to the chat that asked for it"),
    "{}",
    errors(&reply)
  );
  assert!(stub.asked().is_empty());
}

#[test]
fn each_picture_carries_the_label_named_right_after_it() {
  let stub = Stub::answering(&[("/sends", 200, said(&["album sent"]))]);
  let bench = reaching(&stub, None);

  let output = bench.bmo_given(
    &[
      "room",
      "photo",
      "--chat",
      "5",
      "--url",
      "a.png",
      "--label",
      "one",
      "--url",
      "b.png",
      "--caption",
      "c",
    ],
    "",
    &[],
  );

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(
    stub.asked()[0].body["input"],
    json!({ "pictures": [{ "url": "a.png", "label": "one" }, { "url": "b.png" }], "caption": "c" })
  );
}

#[test]
fn a_reaction_a_pin_and_a_sticker_are_each_asked_as_their_act() {
  let stub = Stub::answering(&[("/sends", 200, said(&["done"]))]);
  let bench = reaching(&stub, None);

  for words in [
    &["room", "react", "--chat", "5", "--message", "7", "--emoji", "+1"][..],
    &["room", "pin", "--chat", "5", "--message", "7", "--unpin"],
    &["room", "sticker", "--chat", "5", "--emoji", "x"],
  ] {
    assert!(bench.bmo_given(words, "", &[]).status.success(), "{words:?}");
  }

  let asked: Vec<(Value, Value)> = stub
    .asked()
    .into_iter()
    .map(|asked| (asked.body["tool"].clone(), asked.body["input"].clone()))
    .collect();

  assert_eq!(
    asked,
    [
      (json!("react"), json!({ "message": 7, "emoji": "+1" })),
      (json!("pin"), json!({ "message": 7, "unpin": true })),
      (json!("sticker"), json!({ "emoji": "x" })),
    ]
  );
}

#[test]
fn a_fair_draw_is_asked_with_its_spec_and_its_mode() {
  let stub = Stub::answering(&[("/sends", 200, said(&["drawn"]))]);
  let bench = reaching(&stub, None);

  let output = bench.bmo_given(
    &[
      "draw",
      "draw",
      "--chat",
      "5",
      "--mode",
      "sync",
      "--spec",
      "{\"shape\":\"dice\",\"sides\":6}",
      "--via",
      "r",
    ],
    "",
    &[],
  );
  let refused = bench.bmo_given(&["draw", "draw", "--chat", "5", "--mode", "now", "--spec", "{}"], "", &[]);

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(
    stub.asked()[0].body,
    json!({
      "asker": { "chat": 5, "via": "r" },
      "tool": "random_draw",
      "input": { "shape": "dice", "sides": 6, "mode": "sync" },
    })
  );
  assert_eq!(refused.status.code(), Some(1));
  assert_eq!(errors(&refused), "mode must be one of sync, async\n");
}

#[test]
fn a_consult_line_and_block_are_asked_as_the_agent() {
  let stub = Stub::answering(&[
    (
      "/consult/say",
      200,
      json!({ "ok": true, "chatId": 3, "messageId": 4, "lines": ["window"] }),
    ),
    ("/consult/block", 200, json!({ "ok": true, "block": "the others are at work" })),
  ]);
  let bench = reaching(&stub, None);

  // The line's own flags stand before its words, which take every word after them, as they always have.
  let line = bench.bmo_given(&["consult", "say", "--reply", "11", "on it"], "", &[AGENT]);
  let block = bench.bmo_given(&["consult", "state", "--agent", "agent-b"], "", &[]);
  let nobody = bench.bmo_given(&["consult", "state"], "", &[]);

  assert_eq!(lines(&line), ["window"]);
  assert_eq!(lines(&block), ["the others are at work"]);
  assert_eq!(nobody.status.code(), Some(1));
  assert_eq!(
    stub.asked().into_iter().map(|asked| (asked.path, asked.body)).collect::<Vec<_>>(),
    [
      ("/consult/say".to_owned(), json!({ "agent": "agent-a", "text": "on it", "replyTo": 11 })),
      ("/consult/block".to_owned(), json!({ "agent": "agent-b" })),
    ]
  );
}

#[test]
fn a_reminder_is_asked_of_the_sends_as_its_agent_in_its_room() {
  let stub = Stub::answering(&[("/sends", 200, said(&["set for 09:00"]))]);
  let bench = reaching(&stub, None);

  let output = bench.bmo_given(
    &[
      "--via",
      "r",
      "reminder",
      "set",
      "--chat",
      "5",
      "--every",
      "daily 09:00",
      "--reason",
      "stretch",
    ],
    "",
    &[AGENT],
  );

  assert_eq!(lines(&output), ["set for 09:00"]);
  assert_eq!(
    stub.asked()[0].body,
    json!({
      "asker": { "chat": 5, "agent": "agent-a" },
      "tool": "remind",
      "input": { "every": "daily 09:00", "reason": "stretch" },
    })
  );
}

#[test]
fn what_went_before_a_refusal_is_printed_and_the_refusal_ends_it() {
  let stub =
    Stub::answering(&[("/sends", 200, json!({ "lines": ["one went"], "refusal": "the other did not" }))]);
  let bench = reaching(&stub, None);

  let output =
    bench.bmo_given(&["room", "react", "--chat", "5", "--message", "7", "--emoji", "x"], "", &[]);

  assert_eq!(output.status.code(), Some(1));
  assert_eq!(lines(&output), ["one went"]);
  assert_eq!(errors(&output), "the other did not\n");
}

#[test]
fn a_refused_status_says_the_receivers_error_or_that_it_gave_none() {
  let words = ["room", "pin", "--chat", "5", "--message", "7"];
  let named =
    Stub::answering(&[("/sends", 409, json!({ "code": "conflict", "error": "nobody answers there" }))]);
  let bare = Stub::answering(&[("/sends", 502, Value::Null)]);

  let named = reaching(&named, None).bmo_given(&words, "", &[]);
  let bare = reaching(&bare, None).bmo_given(&words, "", &[]);

  assert_eq!((named.status.code(), errors(&named)), (Some(1), "nobody answers there\n".to_owned()));
  assert_eq!(
    (bare.status.code(), errors(&bare)),
    (Some(1), "the receiver turned it away (502) and gave no reason\n".to_owned())
  );
}

#[test]
fn a_receiver_the_settings_do_not_reach_is_refused_naming_the_setting() {
  let bench = Bench::new();

  let output = bench.bmo_given(&["room", "pin", "--chat", "5", "--message", "7"], "", &[]);

  assert_eq!(output.status.code(), Some(2));
  assert_eq!(
    errors(&output),
    "bmo: `room pin --chat 5 --message 7` cannot be asked of bmo-receiver: the settings name no `unit.bmo-receiver.address`\n"
  );
}

#[test]
fn the_agent_is_read_from_the_run_the_agents_place_names_and_a_jobs_run_is_marked() {
  let stub = Stub::answering(&[("/sends", 200, said(&["ok"]))]);
  let bench = reaching(&stub, None);
  let place = bench.path().join("place");
  std::fs::create_dir(&place).unwrap();
  std::fs::write(
    place.join("run.json"),
    r#"{"person":false,"env":{"BMO_TURN_AGENT":"agent-placed","BMO_TASK_LOCKED":"1"}}"#,
  )
  .unwrap();

  let output = bench.bmo_given(
    &["room", "sticker", "--chat", "5", "--emoji", "x"],
    "",
    &[
      ("BMO_AGENT_PLACE", place.to_str().unwrap()),
      ("BMO_TURN_AGENT", "stale"),
      ("CLAUDECODE", "1"),
    ],
  );

  assert!(output.status.success(), "{}", errors(&output));
  assert_eq!(
    stub.asked()[0].body["asker"],
    json!({ "chat": 5, "agent": "agent-placed", "job": true })
  );
}

#[test]
fn the_receivers_commands_no_route_serves_are_not_served() {
  let stub = Stub::answering(&[]);
  let bench = reaching(&stub, None);

  for command in NOT_SERVED {
    let output = bench.bmo(&command.split(' ').collect::<Vec<_>>());

    assert_eq!(output.status.code(), Some(2), "{command}");
    assert_eq!(
      errors(&output),
      format!("bmo: `{command}` is not served yet: no installed unit carries it\n")
    );
  }

  assert!(stub.asked().is_empty());
}

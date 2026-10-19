//! bmo-receiver's commands that its boundary documents a route for (docs/boundary.md, 「bmo-receiver」): the
//! sends, a fair draw and a reminder asked of `POST /sends`, and a consult line and block asked of
//! `POST /consult/say` and `POST /consult/block`, each as the agent whose turn the shell works in.

use std::ffi::OsString;
use std::io::Read as _;
use std::process::ExitCode;
use std::time::Duration;

use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use serde_json::{Map, Value, json};

use crate::carried::{Call, Carried, Carrier};
use crate::{answer, http, settings, turn};

const UNIT: &str = "bmo-receiver";

// How long a send is waited for: a fair draw made in sync is the one wait inside one.
const WITHIN: Duration = Duration::from_secs(170);

const SAY_USAGE: &str = "usage: bmo room say [--chat <chatId>] [--by chat:<chatId>] [--reply] --text <text>  (the words are read from stdin without --text)";
const DRAW_USAGE: &str =
  "usage: bmo draw draw --chat <chatId> --mode sync|async --spec '<json>' [--via <road>]";
const CONSULT_USAGE: &str = "usage: bmo consult state [--agent <name>] | bmo consult say '<text>' [--reply <messageId>] [--agent <name>]";
const MODES: [&str; 2] = ["sync", "async"];
// The largest whole number a request's field may be, as the draw tool reads one.
const SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// Puts a line in a room from a run of its own.
#[derive(Parser)]
#[command(name = "bmo room say", no_binary_name = true, allow_negative_numbers = true)]
struct Say {
  /// The room; the owner's own chat where none is named.
  #[arg(long)]
  chat: Option<i64>,
  /// Who set this off: chat:<chatId> for a chat session, told there what became of the line.
  #[arg(long)]
  by: Option<String>,
  /// Asks the room for an answer back, which comes to the chat named by --by.
  #[arg(long)]
  reply: bool,
  /// The words; read from standard input where none are given.
  #[arg(long)]
  text: Option<String>,
  #[arg(long)]
  via: Option<String>,
}

/// Sends a picture, or several as one album.
#[derive(Parser)]
#[command(name = "bmo room photo", no_binary_name = true, allow_negative_numbers = true)]
struct Photo {
  /// The room; the owner's own chat where none is named.
  #[arg(long)]
  chat: Option<i64>,
  #[arg(long = "url", required = true)]
  urls: Vec<String>,
  /// The word drawn into the picture named by the --url just before it.
  #[arg(long = "label")]
  labels: Vec<String>,
  #[arg(long)]
  caption: Option<String>,
  #[arg(long)]
  via: Option<String>,
}

/// Puts an emoji reaction on a message, or clears the agent's with --emoji none.
#[derive(Parser)]
#[command(name = "bmo room react", no_binary_name = true, allow_negative_numbers = true)]
struct React {
  #[arg(long)]
  chat: i64,
  #[arg(long)]
  message: i64,
  #[arg(long)]
  emoji: String,
  #[arg(long)]
  via: Option<String>,
}

/// Pins a message in a chat, or unpins one with --unpin.
#[derive(Parser)]
#[command(name = "bmo room pin", no_binary_name = true, allow_negative_numbers = true)]
struct Pin {
  #[arg(long)]
  chat: i64,
  #[arg(long)]
  message: i64,
  #[arg(long)]
  unpin: bool,
  #[arg(long)]
  via: Option<String>,
}

/// Sends one of the agent's stickers by its emoji, or lists the set with --list.
#[derive(Parser)]
#[command(name = "bmo room sticker", no_binary_name = true, allow_negative_numbers = true)]
struct Sticker {
  #[arg(long, required_unless_present = "list")]
  chat: Option<i64>,
  #[arg(long, required_unless_present = "list")]
  emoji: Option<String>,
  #[arg(long, conflicts_with_all = ["chat", "emoji"])]
  list: bool,
  #[arg(long)]
  via: Option<String>,
}

/// A fair draw made in a room, as the send tool makes it.
#[derive(Parser)]
#[command(name = "bmo draw draw", no_binary_name = true, allow_negative_numbers = true)]
struct Draw {
  #[arg(long)]
  chat: Option<String>,
  #[arg(long)]
  mode: Option<String>,
  /// The request, as the tool's own fields in one JSON object.
  #[arg(long)]
  spec: Option<String>,
  #[arg(long)]
  via: Option<String>,
}

/// A line said in the consult room, or the block an agent is shown there.
#[derive(Parser)]
#[command(name = "bmo consult", no_binary_name = true)]
struct Consult {
  #[arg(allow_hyphen_values = true)]
  words: Vec<String>,
  /// The agent the line or the block is; the one the calling run names where none is.
  #[arg(long, allow_hyphen_values = true)]
  agent: Option<String>,
  /// The message the line answers.
  #[arg(long, allow_hyphen_values = true)]
  reply: Option<String>,
  /// Another archive's address, which a line said through the running service never reads.
  #[arg(long, hide = true)]
  db: Option<String>,
  #[arg(long)]
  via: Option<String>,
}

/// A reminder set for an agent in its room.
#[derive(Parser)]
#[command(name = "bmo reminder set", no_binary_name = true, allow_negative_numbers = true)]
struct Remind {
  #[arg(long)]
  chat: i64,
  /// An ISO moment, or `YYYY-MM-DD HH:MM` on the deployment's clock.
  #[arg(long, conflicts_with = "every", required_unless_present = "every")]
  at: Option<String>,
  /// `daily HH:MM`, `weekly mon HH:MM` or `monthly 1 HH:MM`.
  #[arg(long)]
  every: Option<String>,
  /// Why it wakes the agent.
  #[arg(long)]
  reason: String,
  /// The memory it serves, by id or slug.
  #[arg(long)]
  memory: Option<String>,
  #[arg(long)]
  user: Option<i64>,
  /// Whose reminder it is, where the run calling it names none.
  #[arg(long)]
  agent: Option<String>,
  #[arg(long)]
  via: Option<String>,
}

struct Receiver;

impl Carried for Receiver {
  fn unit(&self) -> &'static str {
    UNIT
  }

  fn commands(&self) -> &'static [&'static str] {
    &[
      "room say",
      "room photo",
      "room react",
      "room pin",
      "room sticker",
      "draw draw",
      "consult say",
      "consult state",
      "reminder set",
    ]
  }

  fn run(&self, call: &Call<'_>) -> ExitCode {
    match call.command {
      "room say" => say(call),
      "room photo" => photo(call),
      "room react" => {
        let asked: React = read(call.rest);
        let input = json!({ "message": asked.message, "emoji": asked.emoji });

        send(call, asker(Some(asked.chat), road(call, asked.via), None), "react", input)
      }
      "room pin" => {
        let asked: Pin = read(call.rest);
        let input =
          fields([("message", Some(json!(asked.message))), ("unpin", asked.unpin.then_some(json!(true)))]);

        send(call, asker(Some(asked.chat), road(call, asked.via), None), "pin", input)
      }
      "room sticker" => {
        let asked: Sticker = read(call.rest);

        match (asked.list, asked.chat, asked.emoji) {
          (false, Some(chat), Some(emoji)) => send(
            call,
            asker(Some(chat), road(call, asked.via), None),
            "sticker",
            json!({ "emoji": emoji }),
          ),
          _ => answer::not_served(&call.words()),
        }
      }
      "draw draw" => draw(call),
      "consult say" | "consult state" => consult(call),
      _ => remind(call),
    }
  }
}

inventory::submit! { Carrier(&Receiver) }

// A command's words read by its own parser; words it does not read end the run as a parser ends one.
fn read<T: Parser>(rest: &[OsString]) -> T {
  T::try_parse_from(rest).unwrap_or_else(|why| why.exit())
}

// The road a send goes out on: the one the command names, else the one named before the command.
fn road(call: &Call<'_>, named: Option<String>) -> Option<String> {
  named.or_else(|| call.via.map(str::to_owned))
}

// An object of the fields that are given.
fn fields<const N: usize>(pairs: [(&str, Option<Value>); N]) -> Value {
  Value::Object(
    pairs
      .into_iter()
      .filter_map(|(name, value)| Some((name.to_owned(), value?)))
      .collect::<Map<_, _>>(),
  )
}

// Who a shell's send is asked by: the room, the road it names, the chat that set it off, the agent whose turn
// the shell works in, and whether a job runs it.
fn asker(chat: Option<i64>, via: Option<String>, by: Option<String>) -> Value {
  asker_as(chat, via, by, turn::agent())
}

fn asker_as(chat: Option<i64>, via: Option<String>, by: Option<String>, agent: Option<String>) -> Value {
  fields([
    ("chat", chat.map(Value::from)),
    ("via", via.map(Value::from)),
    ("agent", agent.map(Value::from)),
    ("by", by.map(Value::from)),
    ("job", turn::inside_job().then_some(Value::Bool(true))),
  ])
}

// The owner's own chat: `BMO_OWNER_ID`, else the owner the settings name for the receiver.
fn owner() -> Result<i64, String> {
  if let Some(named) = std::env::var("BMO_OWNER_ID").ok().filter(|named| !named.trim().is_empty()) {
    return named.trim().parse().map_err(|_| format!("BMO_OWNER_ID is not a chat number: {named}"));
  }

  settings::unit(UNIT)?.get("owner").and_then(toml::Value::as_integer).ok_or_else(|| {
    format!("the owner is not known: BMO_OWNER_ID is not set and the settings name no `unit.{UNIT}.owner`")
  })
}

fn refused(why: &str) -> ExitCode {
  eprintln!("{why}");

  ExitCode::FAILURE
}

// The words from --text, else from standard input.
fn words(text: Option<String>) -> Result<String, String> {
  match text {
    Some(text) => Ok(text),
    None => {
      let mut read = String::new();

      std::io::stdin()
        .read_to_string(&mut read)
        .map_err(|why| format!("the words could not be read from standard input: {why}"))?;

      Ok(read)
    }
  }
}

fn say(call: &Call<'_>) -> ExitCode {
  let asked: Say = read(call.rest);
  let chat = match asked.chat.map_or_else(owner, Ok) {
    Ok(chat) => chat,
    Err(why) => return refused(&why),
  };
  let text = match words(asked.text) {
    Ok(text) => text,
    Err(why) => return refused(&why),
  };
  let hands_back = asked
    .by
    .as_deref()
    .and_then(|by| by.trim().strip_prefix("chat:")?.parse::<i64>().ok())
    .is_some_and(|caller| caller != chat);

  if text.trim().is_empty() {
    return refused(&format!("say: there are no words to send.\n{SAY_USAGE}"));
  }

  if asked.reply && !hands_back {
    return refused(&format!(
      "say: an answer comes back to the chat that asked for it, so --reply needs --by chat:<your own chatId>, and that chat cannot be the room being written to.\n{SAY_USAGE}"
    ));
  }

  let input =
    fields([("text", Some(Value::from(text))), ("reply", asked.reply.then_some(Value::Bool(true)))]);

  send(call, asker(Some(chat), road(call, asked.via), asked.by), "say", input)
}

fn photo(call: &Call<'_>) -> ExitCode {
  let matches = Photo::command().try_get_matches_from(call.rest).unwrap_or_else(|why| why.exit());
  let asked = Photo::from_arg_matches(&matches).unwrap_or_else(|why| why.exit());
  let chat = match asked.chat.map_or_else(owner, Ok) {
    Ok(chat) => chat,
    Err(why) => return refused(&why),
  };
  let labels = labels_by_url(&matches);
  let pictures: Vec<Value> = asked
    .urls
    .into_iter()
    .enumerate()
    .map(|(at, url)| {
      fields([
        ("url", Some(Value::from(url))),
        ("label", labels.get(at).cloned().flatten().map(Value::from)),
      ])
    })
    .collect();
  let input =
    fields([("pictures", Some(Value::Array(pictures))), ("caption", asked.caption.map(Value::from))]);

  send(call, asker(Some(chat), road(call, asked.via), None), "photo", input)
}

// The label each --url carries, by the url's place among them: the --label standing after it and before the
// next --url, where one does.
fn labels_by_url(matches: &ArgMatches) -> Vec<Option<String>> {
  let urls: Vec<usize> = matches.indices_of("urls").map(Iterator::collect).unwrap_or_default();
  let labels: Vec<(usize, String)> = matches
    .indices_of("labels")
    .into_iter()
    .flatten()
    .zip(matches.get_many::<String>("labels").into_iter().flatten().cloned())
    .collect();

  urls
    .iter()
    .enumerate()
    .map(|(at, url)| {
      let next = urls.get(at + 1).copied().unwrap_or(usize::MAX);

      labels
        .iter()
        .find(|(place, _)| place > url && *place < next)
        .map(|(_, label)| label.clone())
    })
    .collect()
}

fn draw(call: &Call<'_>) -> ExitCode {
  let asked: Draw = read(call.rest);
  let chat = asked.chat.as_deref().and_then(whole);
  let Some(chat) = chat else { return refused(DRAW_USAGE) };
  let Some(spec) = asked.spec.as_deref() else { return refused(DRAW_USAGE) };
  let Ok(Value::Object(mut input)) = serde_json::from_str::<Value>(spec) else {
    return refused("--spec must be one JSON object, such as '{\"shape\":\"dice\",\"sides\":6}'");
  };
  let Some(mode) = asked.mode.filter(|mode| MODES.contains(&mode.as_str())) else {
    return refused(&format!("mode must be one of {}", MODES.join(", ")));
  };

  input.insert("mode".to_owned(), Value::from(mode));
  send(
    call,
    asker(Some(chat), road(call, asked.via), None),
    "random_draw",
    Value::Object(input),
  )
}

// A whole number as the draw tool reads one: digits, a minus before them, within the safe range.
fn whole(text: &str) -> Option<i64> {
  let digits = text.strip_prefix('-').unwrap_or(text);

  if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
    return None;
  }

  text.parse().ok().filter(|number: &i64| number.abs() <= SAFE_INTEGER)
}

fn consult(call: &Call<'_>) -> ExitCode {
  let asked: Consult = read(call.rest);
  let Some(agent) = asked.agent.or_else(turn::agent) else {
    return refused("no agent is named: run this inside an agent's turn, or name one with --agent <name>");
  };

  match (call.command, asked.words.as_slice()) {
    ("consult state", []) => block(call, &agent),
    ("consult say", [text]) => {
      let reply = match asked.reply.as_deref().map(|reply| reply.trim().parse::<i64>()) {
        None => None,
        Some(Ok(reply)) => Some(reply),
        Some(Err(_)) => return refused(CONSULT_USAGE),
      };

      ask(call, "/consult/say", &json!({ "agent": agent, "text": text, "replyTo": reply }))
    }
    _ => refused(&format!("{}: not a verb with these words\n{CONSULT_USAGE}", call.command)),
  }
}

// The block an agent is shown of the other agents' work in the consult room.
fn block(call: &Call<'_>, agent: &str) -> ExitCode {
  let said = match reached(call, "/consult/block", &json!({ "agent": agent })) {
    Ok(Ok((status, said))) if (200..300).contains(&status) => said,
    Ok(Ok((status, said))) => {
      return refused(&said.get("error").and_then(Value::as_str).map_or_else(
        || format!("the receiver turned it away ({status}) and gave no reason"),
        str::to_owned,
      ));
    }
    Ok(Err(why)) => return refused(&why),
    Err(code) => return code,
  };

  match said.get("block").and_then(Value::as_str) {
    Some(block) => println!("{block}"),
    None => println!("nothing of the other agents' work is shown to {agent} now"),
  }

  ExitCode::SUCCESS
}

fn remind(call: &Call<'_>) -> ExitCode {
  let asked: Remind = read(call.rest);
  let Some(agent) = asked.agent.or_else(turn::agent) else {
    return refused(
      "remind: no agent is named: run this inside an agent's turn, or name one with --agent <name>",
    );
  };
  let input = fields([
    ("at", asked.at.map(Value::from)),
    ("every", asked.every.map(Value::from)),
    ("reason", Some(Value::from(asked.reason))),
    ("memory", asked.memory.map(Value::from)),
    ("user", asked.user.map(Value::from)),
  ]);

  send(call, asker_as(Some(asked.chat), None, None, Some(agent)), "remind", input)
}

// One act asked of `POST /sends`.
fn send(call: &Call<'_>, asker: Value, tool: &str, input: Value) -> ExitCode {
  ask(call, "/sends", &json!({ "asker": asker, "tool": tool, "input": input }))
}

// The receiver asked at `path`: the settings it needs refused with 2, the answer or why none came.
fn reached(call: &Call<'_>, path: &str, body: &Value) -> Result<Result<(u16, Value), String>, ExitCode> {
  let (address, token) =
    settings::reach(UNIT).map_err(|why| answer::unsettled(&call.words(), UNIT, &why))?;

  Ok(http::post(&address, &token, path, body, WITHIN))
}

// What the receiver said of an act: its lines printed, and the refusal it ended in on the error stream.
fn ask(call: &Call<'_>, path: &str, body: &Value) -> ExitCode {
  let (status, said) = match reached(call, path, body) {
    Ok(Ok(answered)) => answered,
    Ok(Err(why)) => return refused(&why),
    Err(code) => return code,
  };
  let lines: Vec<&str> = said
    .get("lines")
    .and_then(Value::as_array)
    .map(|lines| lines.iter().filter_map(Value::as_str).collect())
    .unwrap_or_default();
  let refusal =
    if (200..300).contains(&status) {
      match said.get("refusal").and_then(Value::as_str) {
        Some(refusal) => Some(refusal.to_owned()),
        None if said.get("lines").is_none() => {
          return refused(&format!("the receiver's answer to {path} says nothing of what it did"));
        }
        None => None,
      }
    } else {
      Some(said.get("error").and_then(Value::as_str).map_or_else(
        || format!("the receiver turned it away ({status}) and gave no reason"),
        str::to_owned,
      ))
    };

  for line in &lines {
    println!("{line}");
  }

  match refusal {
    Some(refusal) => refused(&refusal),
    None => ExitCode::SUCCESS,
  }
}

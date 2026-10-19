//! bmo-brain's commands (docs/boundary.md, 「bmo-brain」): the memory rows, a note, the fragment table, the
//! digests and the brain's holds, each asked of the route bmo-brain documents for it, as the agent the command
//! names or the turn's.

use std::ffi::OsString;
use std::io::Read as _;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use serde_json::{Map, Value, json};

use crate::carried::{Call, Carried, Carrier};
use crate::{answer, http, settings, turn};

const UNIT: &str = "bmo-brain";
const WITHIN: Duration = Duration::from_secs(60);
// The brain waits for its editing locks nine minutes at most; its caller waits longer.
const WAIT_WITHIN: Duration = Duration::from_secs(600);

const MEMORY_USAGE: &str = "usage: bmo memory (--chat <chatId> | --global) [--agent <name>] and one of:
  --add --type <type> --action '<text>' [--condition <text>] [--on <event>] [--until <moment> [--name <n>] [--every <span>] [--within <span>]] [--slug <slug>] [--for <userId>] [--by <userId>] [--message <messageId>] [--todo <item>] [--reach <reach>] [--replaces <row>]
  --update <row> --action '<text>'
  --done <row> [--text '<text>']
  --retire <row> --why '<text>'
  --revive <row>
  --candidate <row> --why '<text>'
  --list [--type <type>] [--all]
--action - reads the action from standard input";
const FRAGMENT_USAGE: &str = "usage: bmo fragment [--list] [--kind <kind>] [--chat <chatId>] [--person <userId>] [--item <n>] [--word <word>] [--all]
       bmo fragment --check [--kind <kind>]
       bmo fragment --sync
       bmo fragment --write <brain path> --title '<what it is and when a turn needs it>' [--keywords '<a, b>'] [--items <n,n>] [--shared | --chat <chatId>]
       bmo fragment --remove <brain path>";
const DIGEST_USAGE: &str = "usage: bmo digest run [--chat <chatId>]";
const PEOPLE_USAGE: &str = "usage: bmo digest people [--chat <chatId>] [--agent <name>] [--all] [--split [--person <userId>] [--spans <n>]] [--sheets] [--reform --batches <n>] [--sort [--person <userId> | --all | --status] [--groups <n>]] [--since [<day> | none]] [--forget-read]";

/// The memory rows an agent holds in a room or for no room.
#[derive(Parser, Default)]
#[command(name = "bmo memory", no_binary_name = true, allow_negative_numbers = true)]
struct Memory {
  #[arg(long, allow_hyphen_values = true)]
  chat: Option<String>,
  #[arg(long)]
  global: bool,
  #[arg(long)]
  add: bool,
  #[arg(long)]
  list: bool,
  #[arg(long)]
  all: bool,
  #[arg(long = "type", allow_hyphen_values = true)]
  kind: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  condition: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  on: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  until: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  name: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  every: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  within: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  action: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  slug: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  text: Option<String>,
  #[arg(long = "for", allow_hyphen_values = true)]
  for_user: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  by: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  message: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  todo: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  reach: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  replaces: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  update: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  done: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  retire: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  revive: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  candidate: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  why: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  agent: Option<String>,
}

impl Memory {
  // Whether the flag of this name was given.
  fn given(&self, name: &str) -> bool {
    match name {
      "--type" => self.kind.is_some(),
      "--condition" => self.condition.is_some(),
      "--on" => self.on.is_some(),
      "--until" => self.until.is_some(),
      "--name" => self.name.is_some(),
      "--every" => self.every.is_some(),
      "--within" => self.within.is_some(),
      "--action" => self.action.is_some(),
      "--slug" => self.slug.is_some(),
      "--text" => self.text.is_some(),
      "--for" => self.for_user.is_some(),
      "--by" => self.by.is_some(),
      "--message" => self.message.is_some(),
      "--todo" => self.todo.is_some(),
      "--reach" => self.reach.is_some(),
      "--replaces" => self.replaces.is_some(),
      "--why" => self.why.is_some(),
      "--all" => self.all,
      "--update" => self.update.is_some(),
      "--done" => self.done.is_some(),
      "--retire" => self.retire.is_some(),
      "--revive" => self.revive.is_some(),
      "--candidate" => self.candidate.is_some(),
      _ => false,
    }
  }

  fn outside(&self, verb: &str, names: &[&str]) -> Result<(), String> {
    match names.iter().find(|name| self.given(name)) {
      Some(name) => Err(format!("{name} does not go with {verb}")),
      None => Ok(()),
    }
  }
}

// The flags no verb but the one that files a row takes.
const CHANGE_FLAGS: [&str; 17] = [
  "--type",
  "--condition",
  "--on",
  "--until",
  "--name",
  "--every",
  "--within",
  "--action",
  "--slug",
  "--text",
  "--for",
  "--by",
  "--message",
  "--todo",
  "--reach",
  "--replaces",
  "--all",
];

/// A note kept tied to a room, for the digest to carry into the room's rulebook and the library.
#[derive(Parser)]
#[command(name = "bmo note", no_binary_name = true, allow_negative_numbers = true)]
struct Note {
  #[arg(long)]
  chat: i64,
  #[arg(long, allow_hyphen_values = true)]
  text: Option<String>,
  #[arg(long)]
  author: Option<i64>,
  /// Kept for every room rather than the one it was said in.
  #[arg(long)]
  global: bool,
  /// Whose note it is, where the run calling it names none.
  #[arg(long)]
  agent: Option<String>,
}

/// The fragment table: listed, checked, brought up to date, or one piece's row written or taken down.
#[derive(Parser)]
#[command(name = "bmo fragment", no_binary_name = true, allow_negative_numbers = true)]
struct Fragment {
  #[arg(long)]
  list: bool,
  #[arg(long)]
  kind: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  chat: Option<String>,
  #[arg(long, allow_hyphen_values = true)]
  person: Option<String>,
  #[arg(long)]
  item: Option<String>,
  #[arg(long)]
  word: Option<String>,
  #[arg(long)]
  all: bool,
  #[arg(long)]
  check: bool,
  #[arg(long)]
  sync: bool,
  #[arg(long)]
  write: Option<String>,
  #[arg(long)]
  title: Option<String>,
  #[arg(long)]
  keywords: Option<String>,
  #[arg(long)]
  items: Option<String>,
  #[arg(long)]
  shared: bool,
  #[arg(long)]
  remove: Option<String>,
}

/// The memo digest launched, for one room or every room with notes waiting.
#[derive(Parser)]
#[command(name = "bmo digest run", no_binary_name = true, allow_negative_numbers = true)]
struct DigestRun {
  #[arg(long)]
  chat: Option<String>,
}

/// The digests' pause: told, switched, or set for a length.
#[derive(Parser)]
#[command(name = "bmo digest pause", no_binary_name = true)]
struct DigestPause {
  what: Option<String>,
}

/// One room's memo digest launched now.
#[derive(Parser)]
#[command(name = "bmo digest now", no_binary_name = true, allow_negative_numbers = true)]
struct DigestNow {
  #[arg(long)]
  chat: i64,
  /// Whose digest it is, where the run calling it names none.
  #[arg(long)]
  agent: Option<String>,
}

/// The person book's digest, and the work on a room's book beside it.
#[derive(Parser)]
#[command(name = "bmo digest people", no_binary_name = true, allow_negative_numbers = true)]
struct DigestPeople {
  #[arg(long)]
  chat: Option<String>,
  #[arg(long)]
  agent: Option<String>,
  #[arg(long)]
  all: bool,
  #[arg(long)]
  split: bool,
  #[arg(long)]
  sheets: bool,
  #[arg(long)]
  reform: bool,
  #[arg(long)]
  sort: bool,
  #[arg(long)]
  status: bool,
  #[arg(long)]
  groups: Option<String>,
  #[arg(long)]
  batches: Option<String>,
  #[arg(long)]
  person: Option<String>,
  #[arg(long)]
  spans: Option<String>,
  #[arg(long, num_args = 0..=1)]
  since: Option<Option<String>>,
  #[arg(long = "forget-read")]
  forget_read: bool,
}

/// A room's reminders still to come, or every one with --all; or one called off with --cancel.
#[derive(Parser)]
#[command(name = "bmo reminder list", no_binary_name = true, allow_negative_numbers = true)]
struct Reminders {
  #[arg(long, required_unless_present = "cancel")]
  chat: Option<i64>,
  #[arg(long)]
  all: bool,
  #[arg(long)]
  cancel: Option<i64>,
}

struct Brain;

impl Carried for Brain {
  fn unit(&self) -> &'static str {
    UNIT
  }

  fn commands(&self) -> &'static [&'static str] {
    &[
      "memory",
      "note",
      "fragment",
      "digest run",
      "digest pause",
      "digest people",
      "digest now",
      "brain hold",
      "brain wait",
      "reminder list",
    ]
  }

  fn run(&self, call: &Call<'_>) -> ExitCode {
    let service = match Service::of(call) {
      Ok(service) => service,
      Err(code) => return code,
    };

    match call.command {
      "memory" => memory(&service, read(call.rest)),
      "note" => note(&service, read(call.rest)),
      "fragment" => fragment(&service, read(call.rest)),
      "digest run" => digest_run(&service, read(call.rest)),
      "digest pause" => digest_pause(&service, read(call.rest)),
      "digest people" => digest_people(&service, read(call.rest)),
      "digest now" => digest_now(&service, read(call.rest)),
      "brain hold" => brain_hold(&service),
      "reminder list" => reminders(&service, read(call.rest)),
      _ => brain_wait(&service),
    }
  }

  fn home(&self, call: &Call<'_>) -> bool {
    matches!(call.command, "brain hold" | "brain wait" | "digest pause")
  }
}

inventory::submit! { Carrier(&Brain) }

fn read<T: Parser>(rest: &[OsString]) -> T {
  T::try_parse_from(rest).unwrap_or_else(|why| why.exit())
}

// The brain as the settings reach it.
struct Service {
  address: String,
  token: String,
}

impl Service {
  fn of(call: &Call<'_>) -> Result<Self, ExitCode> {
    settings::reach(UNIT)
      .map(|(address, token)| Self { address, token })
      .map_err(|why| answer::unsettled(&call.words(), UNIT, &why))
  }

  // What the brain answered, or why it did not: a status outside 200 to 299 says its sentence.
  fn answered(said: Result<(u16, Value), String>) -> Result<Value, String> {
    let (status, said) = said?;

    if (200..300).contains(&status) {
      return Ok(said);
    }

    Err(
      said
        .get("error")
        .and_then(Value::as_str)
        .map_or_else(|| format!("the brain turned it away ({status}) and gave no reason"), str::to_owned),
    )
  }

  fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
    Self::answered(http::post(&self.address, &self.token, path, body, WITHIN))
  }

  fn post_within(&self, path: &str, body: &Value, within: Duration) -> Result<Value, String> {
    Self::answered(http::post(&self.address, &self.token, path, body, within))
  }

  fn get(&self, path: &str, pairs: &[(&str, String)]) -> Result<Value, String> {
    Self::answered(http::get(&self.address, &self.token, path, pairs, WITHIN))
  }
}

// A reading's lines, or the refusal the brain answered with in their place.
fn reading(said: &Value) -> Result<Vec<String>, String> {
  if said["answered"] == false {
    let refusal = &said["refusal"];

    return Err(
      refusal["message"]
        .as_str()
        .or_else(|| refusal["code"].as_str())
        .unwrap_or("refused")
        .to_owned(),
    );
  }

  Ok(lines_of(said))
}

fn lines_of(said: &Value) -> Vec<String> {
  said["lines"]
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .map(str::to_owned)
    .collect()
}

// The lines printed, or why not on the error stream with 1.
fn printed(answered: Result<Vec<String>, String>) -> ExitCode {
  match answered {
    Ok(lines) => {
      for line in lines {
        println!("{line}");
      }

      ExitCode::SUCCESS
    }
    Err(why) => refused(&why),
  }
}

fn refused(why: &str) -> ExitCode {
  eprintln!("{why}");

  ExitCode::FAILURE
}

// A whole number a flag names, or the refusal naming the flag.
fn whole(flag: &str, given: Option<&str>) -> Result<Option<i64>, String> {
  given
    .map(|given| {
      given
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("{flag} takes a whole number, not '{given}'"))
    })
    .transpose()
}

// The words from a flag, or from standard input.
fn stdin_text() -> String {
  let mut read = String::new();
  let _ = std::io::stdin().read_to_string(&mut read);

  read
}

// An object of the fields that are given.
fn fields(pairs: Vec<(&str, Option<Value>)>) -> Map<String, Value> {
  pairs.into_iter().filter_map(|(name, value)| Some((name.to_owned(), value?))).collect()
}

// A row named by its number, or by its slug.
fn row(handle: &str) -> Value {
  handle.trim().parse::<i64>().map_or_else(|_| Value::from(handle.trim()), Value::from)
}

fn memory(service: &Service, asked: Memory) -> ExitCode {
  match memory_call(service, &asked) {
    Ok(lines) => printed(Ok(lines)),
    Err(reason) => refused(&format!("memory: {reason}\n{MEMORY_USAGE}")),
  }
}

fn memory_call(service: &Service, asked: &Memory) -> Result<Vec<String>, String> {
  let chat = whole("--chat", asked.chat.as_deref())?;

  if asked.global == chat.is_some() {
    return Err(
      if asked.global {
        "--global and --chat name two different scopes; give one"
      } else {
        "--chat <chatId> names the room, or --global for a row tied to none"
      }
      .to_owned(),
    );
  }

  let agent = asked.agent.clone().or_else(turn::agent).ok_or_else(|| {
    "no agent is named: run this inside an agent's turn, or name one with --agent <name>".to_owned()
  })?;
  let verbs: Vec<&str> = [
    ("--add", asked.add),
    ("--update", asked.update.is_some()),
    ("--done", asked.done.is_some()),
    ("--retire", asked.retire.is_some()),
    ("--revive", asked.revive.is_some()),
    ("--candidate", asked.candidate.is_some()),
    ("--list", asked.list),
  ]
  .into_iter()
  .filter_map(|(verb, given)| given.then_some(verb))
  .collect();

  match verbs.as_slice() {
    [] => {
      return Err(
        "give one of --add, --update, --done, --retire, --revive, --candidate or --list".to_owned(),
      );
    }
    [_] => {}
    many => return Err(format!("give only one of {}", many.join(", "))),
  }

  let message = whole("--message", asked.message.as_deref())?;
  let action =
    |given: Option<&str>| given.map(|given| if given == "-" { stdin_text() } else { given.to_owned() });
  let text = |given: &Option<String>| given.clone().map(Value::from);
  let global = asked.global.then_some(Value::Bool(true));
  let (path, arguments) = match verbs[0] {
    "--add" => {
      asked.outside("--add", &["--why", "--text", "--all"])?;

      (
        "/memory/add",
        fields(vec![
          ("type", text(&asked.kind)),
          ("action", Some(Value::from(action(asked.action.as_deref()).unwrap_or_default()))),
          ("condition", text(&asked.condition)),
          ("on", text(&asked.on)),
          ("until", text(&asked.until)),
          ("name", text(&asked.name)),
          ("every", text(&asked.every)),
          ("within", text(&asked.within)),
          ("slug", text(&asked.slug)),
          ("for", whole("--for", asked.for_user.as_deref())?.map(Value::from)),
          ("by", whole("--by", asked.by.as_deref())?.map(Value::from)),
          ("message", message.map(Value::from)),
          ("todo", whole("--todo", asked.todo.as_deref())?.map(Value::from)),
          ("reach", text(&asked.reach)),
          ("replaces", text(&asked.replaces)),
          ("global", global),
        ]),
      )
    }
    "--update" => {
      asked.outside(
        "--update",
        &[
          "--type",
          "--condition",
          "--on",
          "--until",
          "--name",
          "--every",
          "--within",
          "--slug",
          "--text",
          "--for",
          "--by",
          "--message",
          "--todo",
          "--reach",
          "--replaces",
          "--why",
          "--all",
        ],
      )?;

      if asked.global {
        return Err(
          "--update moves a room's own state on, so it takes --chat rather than --global".to_owned(),
        );
      }

      (
        "/memory/update",
        fields(vec![
          ("memory", asked.update.as_deref().map(row)),
          ("action", action(asked.action.as_deref()).map(Value::from)),
        ]),
      )
    }
    "--done" => {
      asked.outside(
        "--done",
        &[
          "--type",
          "--condition",
          "--on",
          "--until",
          "--name",
          "--every",
          "--within",
          "--action",
          "--slug",
          "--for",
          "--by",
          "--message",
          "--todo",
          "--reach",
          "--replaces",
          "--why",
          "--all",
        ],
      )?;

      if asked.global {
        return Err(
          "--done checks off a room's own errand, so it takes --chat rather than --global".to_owned(),
        );
      }

      (
        "/memory/done",
        fields(vec![("memory", asked.done.as_deref().map(row)), ("text", text(&asked.text))]),
      )
    }
    "--retire" => {
      asked.outside("--retire", &CHANGE_FLAGS)?;

      (
        "/memory/retire",
        fields(vec![
          ("memory", asked.retire.as_deref().map(row)),
          ("why", text(&asked.why)),
          ("global", global),
        ]),
      )
    }
    "--revive" => {
      asked.outside("--revive", &CHANGE_FLAGS)?;
      asked.outside("--revive", &["--why"])?;

      (
        "/memory/revive",
        fields(vec![("memory", asked.revive.as_deref().map(row)), ("global", global)]),
      )
    }
    "--candidate" => {
      asked.outside("--candidate", &CHANGE_FLAGS)?;

      (
        "/memory/candidate",
        fields(vec![
          ("memory", asked.candidate.as_deref().map(row)),
          ("why", text(&asked.why)),
          ("global", global),
        ]),
      )
    }
    _ => {
      asked.outside(
        "--list",
        &[
          "--condition",
          "--on",
          "--until",
          "--name",
          "--every",
          "--within",
          "--action",
          "--slug",
          "--text",
          "--for",
          "--by",
          "--message",
          "--todo",
          "--reach",
          "--replaces",
          "--why",
        ],
      )?;

      (
        "/memory/list",
        fields(vec![
          ("global", global),
          ("all", asked.all.then_some(Value::Bool(true))),
          ("type", text(&asked.kind)),
        ]),
      )
    }
  };
  let mut body = json!({ "turn": { "agent": agent, "chatId": chat, "messageId": message } });

  body.as_object_mut().into_iter().for_each(|object| object.extend(arguments.clone()));

  reading(&service.post(path, &body)?)
}

fn note(service: &Service, asked: Note) -> ExitCode {
  let Some(agent) = asked.agent.or_else(turn::agent) else {
    return refused(
      "note: no agent is named: run this inside an agent's turn, or name one with --agent <name>",
    );
  };
  let text = asked.text.unwrap_or_else(stdin_text);
  let mut body =
    json!({ "turn": { "agent": agent, "chatId": asked.chat, "messageId": null }, "text": text });

  if let Some(author) = asked.author {
    body["author"] = Value::from(author);
  }

  if asked.global {
    body["global"] = Value::Bool(true);
  }

  printed(
    service
      .post("/notes/add", &body)
      .and_then(|said| reading(&said))
      .map_err(|why| format!("note: {why}")),
  )
}

fn fragment(service: &Service, asked: Fragment) -> ExitCode {
  match fragment_call(service, &asked) {
    Ok(lines) => printed(Ok(lines)),
    Err(reason) => refused(&format!("fragment: {reason}\n{FRAGMENT_USAGE}")),
  }
}

fn fragment_call(service: &Service, asked: &Fragment) -> Result<Vec<String>, String> {
  let chat = whole("--chat", asked.chat.as_deref())?;
  let person = whole("--person", asked.person.as_deref())?;
  let item = whole("--item", asked.item.as_deref())?;
  let verbs: Vec<&str> = [
    ("--list", asked.list),
    ("--check", asked.check),
    ("--sync", asked.sync),
    ("--write", asked.write.is_some()),
    ("--remove", asked.remove.is_some()),
  ]
  .into_iter()
  .filter_map(|(verb, given)| given.then_some(verb))
  .collect();

  if verbs.len() > 1 {
    return Err(format!("give only one of {}", verbs.join(", ")));
  }

  match verbs.first().copied().unwrap_or("--list") {
    "--check" => Ok(checked(&service.post("/fragments/check", &json!({ "kind": asked.kind }))?)),
    "--sync" => reading(&service.post("/fragments/sync", &json!({}))?),
    "--remove" => reading(&service.post("/fragments/remove", &json!({ "path": asked.remove }))?),
    "--write" => {
      let Some(title) = asked.title.as_deref() else {
        return Err("--write takes --title".to_owned());
      };
      let keywords: Vec<&str> = asked
        .keywords
        .as_deref()
        .into_iter()
        .flat_map(|words| words.split(','))
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .collect();
      let items = asked
        .items
        .as_deref()
        .into_iter()
        .flat_map(|items| items.split(','))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| whole("--items", Some(item)).map(Option::unwrap_or_default))
        .collect::<Result<Vec<i64>, String>>()?;
      let body = json!({
        "path": asked.write,
        "title": title,
        "keywords": keywords,
        "items": items,
        "shared": asked.shared.then_some(true),
        "chatId": chat,
        "forEveryAgent": null,
      });
      let said = service.post("/fragments/write", &body)?;

      reading(&said).map_err(|why| {
        let reasons: Vec<&str> = said["refusal"]["reasons"]
          .as_array()
          .into_iter()
          .flatten()
          .filter_map(Value::as_str)
          .collect();

        if reasons.is_empty() { why } else { format!("{why}: {}", reasons.join("; ")) }
      })
    }
    _ => {
      let pairs: Vec<(&str, String)> = [
        ("kind", asked.kind.clone()),
        ("chat", chat.map(|chat| chat.to_string())),
        ("person", person.map(|person| person.to_string())),
        ("item", item.map(|item| item.to_string())),
        ("text", asked.word.clone()),
        ("down", asked.all.then(|| "true".to_owned())),
      ]
      .into_iter()
      .filter_map(|(name, value)| Some((name, value?)))
      .collect();
      let said = service.get("/fragments", &pairs)?;
      let rows = said["fragments"].as_array().cloned().unwrap_or_default();

      if rows.is_empty() {
        return Ok(vec![format!(
          "no {}fragments{}{}{}{}{}",
          if asked.all { "" } else { "active " },
          asked.kind.as_ref().map_or_else(String::new, |kind| format!(" of kind {kind}")),
          chat.map_or_else(String::new, |chat| format!(" in chat {chat}")),
          person.map_or_else(String::new, |person| format!(" about {person}")),
          item.map_or_else(String::new, |item| format!(" tied to #{item}")),
          asked.word.as_ref().map_or_else(String::new, |word| format!(" asked for by {word}"))
        )]);
      }

      Ok(rows.iter().map(fragment_line).collect())
    }
  }
}

// Whose a row or a drift is: its room, its person, or every room.
fn where_of(row: &Value) -> String {
  match (row["chatId"].as_i64(), row["owner"].as_i64()) {
    (Some(chat), _) => format!("chat {chat}"),
    (None, Some(owner)) => format!("person {owner}"),
    (None, None) => "every room".to_owned(),
  }
}

// A moment in Unix seconds, to the minute in UTC.
fn moment(seconds: i64) -> String {
  chrono::DateTime::from_timestamp(seconds, 0)
    .map_or_else(|| seconds.to_string(), |at| at.format("%Y-%m-%d %H:%M UTC").to_string())
}

fn fragment_line(row: &Value) -> String {
  let text = |field: &str| row[field].as_str().unwrap_or_default().to_owned();
  let hits = match row["lastHitAt"].as_i64() {
    Some(at) => format!("{} hits, last {}", row["hits"].as_i64().unwrap_or_default(), moment(at)),
    None => "never attached".to_owned(),
  };
  let hand = row["handAt"]
    .as_i64()
    .map_or_else(String::new, |at| format!(" · set by hand {}", moment(at)));
  let keywords: Vec<&str> =
    row["keywords"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
  let items: Vec<String> = row["items"]
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(Value::as_i64)
    .map(|item| format!("#{item}"))
    .collect();
  let period = [("written", "writtenOn"), ("confirmed", "confirmedOn")]
    .iter()
    .filter_map(|(word, field)| row[*field].as_str().map(|day| format!("{word} {day}")))
    .collect::<Vec<_>>();
  let known = [
    (!keywords.is_empty()).then(|| format!("keywords {}", keywords.join(", "))),
    (!items.is_empty()).then(|| format!("items {}", items.join(", "))),
    (!period.is_empty()).then(|| period.join(", ")),
    row["status"].as_str().map(str::to_owned),
  ]
  .into_iter()
  .flatten()
  .collect::<Vec<_>>()
  .join(" · ");
  let scope = row["scope"].as_str().map_or_else(String::new, |scope| format!(" {scope}"));

  format!(
    "{} {} · {} · {}{scope}: {} [{}] · {known} · {hits}{hand}",
    if row["active"] == true { " " } else { "-" },
    text("kind"),
    where_of(row),
    text("slug"),
    text("title"),
    text("path"),
  )
}

// The drift check's answer: each drift, each title the shape check refuses, and the counts by kind.
fn checked(said: &Value) -> Vec<String> {
  let mut lines: Vec<String> = said["drift"]
    .as_array()
    .into_iter()
    .flatten()
    .map(|drift| {
      format!(
        "{}: {} · {} · {}: {}",
        drift["trouble"].as_str().unwrap_or_default(),
        drift["kind"].as_str().unwrap_or_default(),
        where_of(drift),
        drift["slug"].as_str().unwrap_or_default(),
        drift["said"].as_str().unwrap_or_default()
      )
    })
    .collect();

  if lines.is_empty() {
    lines.push("no drift between the table and the brain".to_owned());
  }

  let titles = &said["titles"];

  lines.extend(titles["refused"].as_array().into_iter().flatten().map(|title| {
    format!(
      "title refused: {} · {} · {} [{}]: {}",
      title["kind"].as_str().unwrap_or_default(),
      where_of(title),
      title["slug"].as_str().unwrap_or_default(),
      title["path"].as_str().unwrap_or_default(),
      title["problem"].as_str().unwrap_or_default()
    )
  }));

  let counts: Vec<String> = titles["counted"]
    .as_array()
    .into_iter()
    .flatten()
    .map(|counted| {
      format!(
        "{} {} of {}",
        counted["kind"].as_str().unwrap_or_default(),
        counted["refused"].as_i64().unwrap_or_default(),
        counted["standing"].as_i64().unwrap_or_default()
      )
    })
    .collect();

  if !counts.is_empty() {
    lines.push(format!("titles the shape check refuses: {}", counts.join(" · ")));
  }

  lines
}

// What a digest's launch came to, as lines, and whether it ends as failed.
fn outcome(said: &Value, chat: Option<i64>) -> Result<Vec<String>, String> {
  let unit = said["unit"].as_str().unwrap_or_default();
  let room = chat.map_or_else(String::new, |chat| format!(" for chat {chat}"));

  match said["outcome"].as_str().unwrap_or_default() {
    "paused" => Ok(vec!["the digests are paused; nothing was launched".to_owned()]),
    "resting" => Ok(vec!["a rest is on; nothing was launched".to_owned()]),
    "nothing" => Ok(vec![format!("no pending notes{room}; nothing to digest.")]),
    "running" => Ok(vec![format!("a digest{room} is already running ({unit}); not starting another.")]),
    "launched" => Ok(vec![match said["notes"].as_i64() {
      Some(notes) => format!(
        "digest launched as {unit} ({notes} notes); it updates the library by itself and sends nothing back to the chat."
      ),
      None => format!(
        "digest launched as {unit}; it updates the library by itself and sends nothing back to the chat."
      ),
    }]),
    "answered" => reading(said),
    "skipped" => Ok(vec!["skipped: the book is not written from this room".to_owned()]),
    "refused" => Err(format!("failed to launch the digest: {}", said["why"].as_str().unwrap_or_default())),
    other => Err(format!("the brain answered an outcome this command does not know: {other}")),
  }
}

fn digest_run(service: &Service, asked: DigestRun) -> ExitCode {
  let chat = match whole("--chat", asked.chat.as_deref()) {
    Ok(chat) => chat,
    Err(_) => return refused(DIGEST_USAGE),
  };

  printed(
    service
      .post("/digest/run", &json!({ "chatId": chat }))
      .and_then(|said| outcome(&said, chat)),
  )
}

fn digest_now(service: &Service, asked: DigestNow) -> ExitCode {
  let Some(agent) = asked.agent.or_else(turn::agent) else {
    return refused(
      "digest now: no agent is named: run this inside an agent's turn, or name one with --agent <name>",
    );
  };
  let body = json!({ "agent": agent, "chatId": asked.chat });

  printed(service.post("/digest/now", &body).and_then(|said| outcome(&said, Some(asked.chat))))
}

fn digest_pause(service: &Service, asked: DigestPause) -> ExitCode {
  let set = asked.what.unwrap_or_else(|| "status".to_owned());

  printed(
    service
      .post("/digest/pause", &json!({ "set": set }))
      .map(|said| lines_of(&said))
      .map_err(|why| {
        format!("{why}\nusage: bmo digest pause <duration such as 2h|90m> | on | off | status")
      }),
  )
}

fn digest_people(service: &Service, asked: DigestPeople) -> ExitCode {
  let numbers = || -> Result<_, String> {
    Ok((
      whole("--chat", asked.chat.as_deref())?,
      whole("--person", asked.person.as_deref())?,
      whole("--spans", asked.spans.as_deref())?,
      whole("--batches", asked.batches.as_deref())?,
      whole("--groups", asked.groups.as_deref())?,
    ))
  };
  let (chat, person, spans, batches, groups) = match numbers() {
    Ok(numbers) => numbers,
    Err(why) => return refused(&format!("people digest: {why}\n{PEOPLE_USAGE}")),
  };
  let flag = |given: bool| given.then_some(Value::Bool(true));
  let mut body = Value::Object(fields(vec![
    ("chatId", Some(chat.map_or(Value::Null, Value::from))),
    ("agent", asked.agent.clone().map(Value::from)),
    ("all", flag(asked.all)),
    ("split", flag(asked.split)),
    ("person", person.map(Value::from)),
    ("spans", spans.map(Value::from)),
    ("sheets", flag(asked.sheets)),
    ("reform", flag(asked.reform)),
    ("batches", batches.map(Value::from)),
    ("sort", flag(asked.sort)),
    ("status", flag(asked.status)),
    ("groups", groups.map(Value::from)),
    ("forgetRead", flag(asked.forget_read)),
  ]));

  // The day the book reads from: asked with no day, it is read; a day or `none` sets it.
  if let Some(since) = &asked.since {
    body["since"] = since.clone().map_or(Value::Null, Value::from);
  }

  printed(
    service
      .post("/digest/people", &body)
      .and_then(|said| outcome(&said, chat))
      .map_err(|why| format!("people digest: {why}")),
  )
}

fn brain_hold(service: &Service) -> ExitCode {
  let names = |said: &Value, field: &str| -> Vec<String> {
    said[field]
      .as_array()
      .into_iter()
      .flatten()
      .filter_map(Value::as_str)
      .map(str::to_owned)
      .collect()
  };

  printed(service.get("/brain/hold", &[]).map(|said| {
    let holders = names(&said, "holders");
    let taken = names(&said, "taken");

    vec![
      if holders.is_empty() {
        "no job holds the brain".to_owned()
      } else {
        format!("held by: {}", holders.join(", "))
      },
      if taken.is_empty() {
        "every editing lock is free".to_owned()
      } else {
        format!("taken by a brain editor: {}", taken.join(", "))
      },
    ]
  }))
}

fn brain_wait(service: &Service) -> ExitCode {
  match service.post_within("/brain/wait", &json!({}), WAIT_WITHIN) {
    Ok(said) => {
      for line in lines_of(&said) {
        println!("{line}");
      }

      if said["free"] == true { ExitCode::SUCCESS } else { ExitCode::FAILURE }
    }
    Err(why) => refused(&why),
  }
}

// A room's reminders listed as a session reads them back, every room's agent among them, or one called off.
fn reminders(service: &Service, asked: Reminders) -> ExitCode {
  let answered = match (asked.cancel, asked.chat) {
    (Some(id), _) => {
      service.post("/reminders/cancel", &json!({ "id": id })).and_then(|said| reading(&said))
    }
    (None, Some(chat)) => service
      .post("/reminders/list", &json!({ "chatId": chat, "agent": null, "all": asked.all }))
      .map(|said| {
        let lines = lines_of(&said);

        if lines.is_empty() { vec!["(no reminders)".to_owned()] } else { lines }
      }),
    (None, None) => Err("usage: bmo reminder list --chat <chatId> [--all] | --cancel <id>".to_owned()),
  };

  printed(answered)
}

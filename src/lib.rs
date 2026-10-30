//! The thin entry point's parts (docs/api.md): the words read, the unit commands found and the nouns they
//! declare, a command resolved, and what is handed on or answered, the tool verb among it.

pub mod answer;
pub mod bridge;
pub mod cache;
pub mod hand;
pub mod help;
pub mod line;
pub mod nouns;
pub mod resolve;
pub mod tool_use;
pub mod units;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::Parser;

use crate::line::Line;
use crate::resolve::Resolution;

/// Reads the command line `bmo` was given and hands the command to whoever carries it, or answers it.
pub fn run(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
  let arguments: Vec<OsString> = arguments.into_iter().collect();
  let line = match Line::try_parse_from(&arguments) {
    Ok(line) => line,
    Err(why) => why.exit(),
  };

  if line.version {
    println!("bmo {}", env!("CARGO_PKG_VERSION"));

    return ExitCode::SUCCESS;
  }

  let units = units::found(units::own_directory().as_deref(), std::env::var_os("PATH").as_deref());
  let declared = cache::declared(&units);

  if line.help {
    print!("{}", help::text(&declared));

    return ExitCode::SUCCESS;
  }

  let mut words = line.words;

  if let Some(first) = words.first().and_then(|word| word.to_str()) {
    if first.starts_with('-') {
      return answer::not_an_option(first);
    }

    if first == "help" {
      if words.len() == 1 {
        print!("{}", help::text(&declared));

        return ExitCode::SUCCESS;
      }

      words.remove(0);
      words.push("--help".into());
    }
  }

  if words.is_empty() {
    eprint!("{}", help::text(&declared));

    return ExitCode::from(2);
  }

  let via = line.via.as_deref();

  if words[0] == "use" {
    return tool_use::run(&words[1..], via);
  }

  // A home-bound command asked on a work node is handed to the operations machine with every word given.
  let given = arguments.get(1..).unwrap_or_default();
  let home =
    |bound: bool| bound.then(bridge::socket).flatten().map(|socket| bridge::run_home(&socket, given));

  match resolve::resolve(&words, &declared) {
    Resolution::Noun { unit, noun } => home(noun.marked("home"))
      .unwrap_or_else(|| hand::hand(unit, hand::with_via(&words, noun.words.len(), via))),
    Resolution::Conflict { noun, one, other } => answer::conflict(&noun, one, other),
    Resolution::Git { unit } => hand::hand(unit, hand::with_via(&words[1..], 0, via)),
    Resolution::Begun { words, nouns } => answer::begun(&words, &nouns),
    Resolution::NotServed => answer::not_served(&words),
  }
}

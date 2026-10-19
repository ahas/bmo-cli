//! The thin entry point's parts (docs/boundary.md): the words read, the unit commands found and the nouns
//! they declare, a command resolved, and what is handed on or answered.

pub mod answer;
pub mod cache;
pub mod carried;
pub mod hand;
pub mod help;
pub mod line;
pub mod nouns;
pub mod resolve;
pub mod units;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::Parser;

use crate::carried::Call;
use crate::line::Line;
use crate::resolve::Resolution;

/// Reads the command line `bmo` was given and hands the command to whoever carries it, or answers it.
pub fn run(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
  let line = match Line::try_parse_from(arguments) {
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

  match resolve::resolve(&words, &declared) {
    Resolution::Noun { unit, noun } => hand::hand(unit, hand::with_via(&words, noun.words.len(), via)),
    Resolution::Conflict { noun, one, other } => answer::conflict(&noun, one, other),
    Resolution::Carried { carrier, command } => {
      let rest = &words[command.split(' ').count()..];

      carrier.run(&Call {
        command,
        rest,
        via,
        units: &declared.units,
      })
    }
    Resolution::Git { unit } => hand::hand(unit, hand::with_via(&words[1..], 0, via)),
    Resolution::NotServed => answer::not_served(&words),
  }
}

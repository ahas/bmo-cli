//! What `bmo` answers itself, each answer a line on standard error and the exit code it ends with
//! (docs/api.md, 「What `bmo` answers itself」).

use std::ffi::OsString;
use std::process::ExitCode;

use crate::units::Unit;

/// Words nothing serves.
pub fn not_served(words: &[OsString]) -> ExitCode {
  eprintln!("bmo: `{}` is not served yet: no installed unit carries it", spelled(words));

  ExitCode::from(2)
}

/// A noun two unit commands declare.
pub fn conflict(noun: &str, one: &Unit, other: &Unit) -> ExitCode {
  eprintln!("bmo: `{noun}` is declared by both `{}` and `{}`", one.name, other.name);

  ExitCode::from(2)
}

/// A unit command that could not be started.
pub fn not_started(name: &str, why: &std::io::Error) -> ExitCode {
  eprintln!("bmo: `{name}` could not be started: {why}");

  ExitCode::from(1)
}

/// An option before the first word that `bmo` does not read.
pub fn not_an_option(option: &str) -> ExitCode {
  eprintln!("bmo: `{option}` is not an option of bmo; options of a command follow its words");

  ExitCode::from(2)
}

// The words as given, one space between them.
fn spelled(words: &[OsString]) -> String {
  words.iter().map(|word| word.to_string_lossy()).collect::<Vec<_>>().join(" ")
}

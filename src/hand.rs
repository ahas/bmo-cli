//! A command handed to the unit command that carries it: `bmo` becomes that command, so its words, its
//! environment, its standard streams and its exit code are the caller's (docs/api.md, 「Resolving a
//! command」).

use std::ffi::OsString;
use std::os::unix::process::CommandExt as _;
use std::process::{Command, ExitCode};

use crate::answer;
use crate::units::Unit;

/// The words a unit command is handed: the first `kept` of `words`, the road where the caller named one,
/// then the rest.
pub fn with_via(words: &[OsString], kept: usize, via: Option<&str>) -> Vec<OsString> {
  let mut handed: Vec<OsString> = words[..kept].to_vec();

  if let Some(road) = via {
    handed.push("--via".into());
    handed.push(road.into());
  }

  handed.extend_from_slice(&words[kept..]);
  handed
}

/// Becomes `unit` run with `words`; answers only where it could not be started.
pub fn hand(unit: &Unit, words: Vec<OsString>) -> ExitCode {
  let why = Command::new(&unit.path).args(words).exec();

  answer::not_started(&unit.name, &why)
}

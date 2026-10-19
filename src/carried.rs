//! The commands this repository carries on a unit's behalf, where the unit carries them under other words or
//! documents a route for them and carries no command (docs/boundary.md, 「Resolving a command」). Each module
//! carrying one unit's commands registers itself here, so a module added changes no other file.

use std::ffi::OsString;
use std::process::ExitCode;

use crate::units::Unit;

/// A module carrying one unit's commands.
pub trait Carried: Sync {
  /// The unit command the module carries commands for: `bmo-agency`.
  fn unit(&self) -> &'static str;

  /// The commands it carries, each its words with one space between them.
  fn commands(&self) -> &'static [&'static str];

  /// Runs `call.command`, one of its commands, and answers with the exit code `bmo` ends with.
  fn run(&self, call: &Call<'_>) -> ExitCode;
}

/// One carried command as it was asked.
pub struct Call<'a> {
  /// The command, as the module names it among its commands.
  pub command: &'static str,
  /// The words after the command's own.
  pub rest: &'a [OsString],
  /// The road the caller named before the command, where it named one.
  pub via: Option<&'a str>,
  /// Every unit command found.
  pub units: &'a [Unit],
}

/// A registered carrier, as a module submits it.
pub struct Carrier(pub &'static dyn Carried);

inventory::collect!(Carrier);

/// Every registered carrier.
pub fn all() -> impl Iterator<Item = &'static dyn Carried> {
  inventory::iter::<Carrier>.into_iter().map(|carrier| carrier.0)
}

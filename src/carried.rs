//! The commands this repository carries on a unit's behalf, where the unit carries them under other words or
//! documents a route for them and carries no command (docs/boundary.md, 「Resolving a command」). Each module
//! carrying one unit's commands registers itself here, so a module added changes no other file.

use std::ffi::OsString;
use std::process::ExitCode;

use crate::units::Unit;
use crate::{answer, hand};

/// A module carrying one unit's commands.
pub trait Carried: Sync {
  /// The unit command the module carries commands for: `bmo-agency`.
  fn unit(&self) -> &'static str;

  /// The commands it carries, each its words with one space between them.
  fn commands(&self) -> &'static [&'static str];

  /// Runs `call.command`, one of its commands, and answers with the exit code `bmo` ends with.
  fn run(&self, call: &Call<'_>) -> ExitCode;

  /// Whether `call` runs on the operations machine, handed over the home bridge on a work node
  /// (docs/boundary.md, 「The home bridge」); none of a module's commands does unless it says so.
  fn home(&self, _call: &Call<'_>) -> bool {
    false
  }
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

impl Call<'_> {
  /// The words as they were asked, the command's own and the rest.
  pub fn words(&self) -> Vec<OsString> {
    self.command.split(' ').map(OsString::from).chain(self.rest.iter().cloned()).collect()
  }

  /// Hands the call to the unit command `unit` as `words` followed by the rest of the call's words; where
  /// that command does not stand on this machine, the call is not served. The caller's road is left out.
  pub fn forward(&self, unit: &str, words: &[&str]) -> ExitCode {
    self.hand_to(unit, words, None)
  }

  /// Hands the call as `forward` does, with the caller's road after `words` where it named one, for a
  /// command that reads it.
  pub fn forward_with_road(&self, unit: &str, words: &[&str]) -> ExitCode {
    self.hand_to(unit, words, self.via)
  }

  fn hand_to(&self, unit: &str, words: &[&str], via: Option<&str>) -> ExitCode {
    let Some(found) = self.units.iter().find(|found| found.name == unit) else {
      return answer::not_installed(&self.words(), unit);
    };
    let road = via.into_iter().flat_map(|road| ["--via", road]);
    let handed = words
      .iter()
      .copied()
      .chain(road)
      .map(OsString::from)
      .chain(self.rest.iter().cloned())
      .collect();

    hand::hand(found, handed)
  }
}

/// A registered carrier, as a module submits it.
pub struct Carrier(pub &'static dyn Carried);

inventory::collect!(Carrier);

/// Every registered carrier.
pub fn all() -> impl Iterator<Item = &'static dyn Carried> {
  inventory::iter::<Carrier>.into_iter().map(|carrier| carrier.0)
}

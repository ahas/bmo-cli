//! The help: the options of `bmo`, then every noun with the unit command declaring it, every carried command
//! with its unit, and every unit command the git rule reaches (docs/boundary.md, 「What `bmo` answers itself」).

use std::fmt::Write as _;

use crate::cache::Declared;
use crate::carried;

/// The help as printed.
pub fn text(declared: &Declared) -> String {
  let mut text = String::from(
    "The platform's thin entry point: hands each command to the unit that carries it\n\n\
     Usage: bmo [--via <ROAD>] <WORDS>...\n\n\
     Options:\n      \
     --via <ROAD>  The road the caller was told its turn is read on, handed on with the command\n  \
     -h, --help        This help\n      \
     --version     The version of bmo\n",
  );

  let mut nouns: Vec<(String, &str)> =
    declared.each().map(|(unit, noun)| (noun.spelled(), unit.name.as_str())).collect();
  nouns.sort();
  listed(&mut text, "Nouns", &nouns);

  let mut carried: Vec<(String, &str)> = carried::all()
    .flat_map(|carrier| carrier.commands().iter().map(move |command| (command.to_string(), carrier.unit())))
    .collect();
  carried.sort();
  listed(&mut text, "Carried", &carried);

  let git: Vec<(String, &str)> = declared
    .units
    .iter()
    .map(|unit| (unit.name.trim_start_matches("bmo-").to_owned(), unit.name.as_str()))
    .collect();
  listed(&mut text, "Unit commands", &git);

  text
}

// One section of commands, each beside the unit command that carries it; a section with none is left out.
fn listed(text: &mut String, title: &str, rows: &[(String, &str)]) {
  if rows.is_empty() {
    return;
  }

  let width = rows.iter().map(|(command, _)| command.len()).max().unwrap_or(0);
  let _ = write!(text, "\n{title}:\n");

  for (command, unit) in rows {
    let _ = writeln!(text, "  {command:width$}  {unit}");
  }
}

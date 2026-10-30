//! The help: the options of `bmo` and its own verb, then every noun with the unit command declaring it, and
//! every unit command the git rule reaches (docs/api.md, 「What `bmo` answers itself」).

use std::fmt::Write as _;

use crate::cache::Declared;

/// The help as printed.
pub fn text(declared: &Declared) -> String {
  let mut text = String::from(
    "The platform's thin entry point: hands each command to the unit that carries it\n\n\
     Usage: bmo [--via <ROAD>] <WORDS>...\n\n\
     Options:\n      \
     --via <ROAD>  The road the caller was told its turn is read on, handed on with the command\n  \
     -h, --help        This help\n      \
     --version     The version of bmo\n\n\
     Its own verb:\n  \
     use <TOOL>        Calls a hosted tool through the tool host's toolbook (bmo use --help)\n",
  );

  let mut nouns: Vec<(String, &str)> =
    declared.each().map(|(unit, noun)| (noun.spelled(), unit.name.as_str())).collect();
  nouns.sort();
  listed(&mut text, "Nouns", &nouns);

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

  let _ = write!(text, "\n{title}:\n");
  aligned(text, rows);
}

/// Commands one a line, each indented and beside the unit command that carries it, those aligned in a column.
pub fn aligned(text: &mut String, rows: &[(String, &str)]) {
  let width = rows.iter().map(|(command, _)| command.len()).max().unwrap_or(0);

  for (command, unit) in rows {
    let _ = writeln!(text, "  {command:width$}  {unit}");
  }
}

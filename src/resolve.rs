//! A command resolved in the order the contract gives: a declared noun, the git rule, or nothing
//! (docs/api.md, 「Resolving a command」).

use std::ffi::OsString;

use crate::cache::Declared;
use crate::nouns::Noun;
use crate::units::Unit;

/// Who carries a command.
pub enum Resolution<'a> {
  /// The unit command declaring the longest noun the words start with.
  Noun { unit: &'a Unit, noun: &'a Noun },
  /// Two unit commands declare the longest noun the words start with.
  Conflict {
    noun: String,
    one: &'a Unit,
    other: &'a Unit,
  },
  /// The unit command named by the first word.
  Git { unit: &'a Unit },
  /// Nobody.
  NotServed,
}

/// Who carries `words`.
pub fn resolve<'a>(words: &[OsString], declared: &'a Declared) -> Resolution<'a> {
  let starts_with = |noun: &[&str]| {
    noun.len() <= words.len() && noun.iter().zip(words).all(|(word, given)| given.to_str() == Some(*word))
  };

  let matching: Vec<(&Unit, &Noun)> = declared
    .each()
    .filter(|(_, noun)| starts_with(&noun.words.iter().map(String::as_str).collect::<Vec<_>>()))
    .collect();
  let longest = matching.iter().map(|(_, noun)| noun.words.len()).max().unwrap_or(0);
  let mut claiming = matching.iter().filter(|(_, noun)| noun.words.len() == longest);

  if let Some(&(unit, noun)) = claiming.next() {
    return match claiming.find(|(other, _)| *other != unit) {
      Some(&(other, _)) => Resolution::Conflict {
        noun: noun.spelled(),
        one: unit,
        other,
      },
      None => Resolution::Noun { unit, noun },
    };
  }

  words
    .first()
    .and_then(|first| first.to_str())
    .and_then(|first| declared.unit(&format!("bmo-{first}")))
    .map_or(Resolution::NotServed, |unit| Resolution::Git { unit })
}

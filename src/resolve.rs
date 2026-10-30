//! A command resolved in the order the contract gives: a declared noun, the git rule, or nothing, said with
//! the nouns the words begin where they begin any (docs/api.md, 「Resolving a command」).

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
  /// Nobody, but the leading words begin declared nouns: the longest run of them that does, and every noun
  /// it begins with the unit command declaring it.
  Begun {
    words: String,
    nouns: Vec<(&'a Unit, &'a Noun)>,
  },
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

  if let Some(unit) = words
    .first()
    .and_then(|first| first.to_str())
    .and_then(|first| declared.unit(&format!("bmo-{first}")))
  {
    return Resolution::Git { unit };
  }

  begun(words, declared)
}

// The longest run of leading words that begins declared nouns, and those nouns; no noun matched the words, so
// every noun such a run begins is longer than it.
fn begun<'a>(words: &[OsString], declared: &'a Declared) -> Resolution<'a> {
  for length in (1..=words.len()).rev() {
    let Some(given) = words[..length].iter().map(|word| word.to_str()).collect::<Option<Vec<_>>>() else {
      continue;
    };
    let nouns: Vec<(&Unit, &Noun)> = declared
      .each()
      .filter(|(_, noun)| {
        noun.words.len() > length && noun.words.iter().zip(&given).all(|(word, given)| word == given)
      })
      .collect();

    if !nouns.is_empty() {
      return Resolution::Begun {
        words: given.join(" "),
        nouns,
      };
    }
  }

  Resolution::NotServed
}

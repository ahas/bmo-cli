//! The nouns every unit command declares, kept between runs so a command is asked only when it has changed
//! (docs/api.md, 「The unit command」).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::nouns::{self, Noun};
use crate::units::Unit;

/// The unit commands found and the nouns each declares, in the order the commands were found.
#[derive(Debug, Clone, Default)]
pub struct Declared {
  pub units: Vec<Unit>,
  pub nouns: Vec<Vec<Noun>>,
}

impl Declared {
  /// Every declared noun beside the unit command declaring it.
  pub fn each(&self) -> impl Iterator<Item = (&Unit, &Noun)> {
    self
      .units
      .iter()
      .zip(&self.nouns)
      .flat_map(|(unit, nouns)| nouns.iter().map(move |noun| (unit, noun)))
  }

  /// The unit command of a name, where one was found.
  pub fn unit(&self, name: &str) -> Option<&Unit> {
    self.units.iter().find(|unit| unit.name == name)
  }
}

// One unit command's answer as kept: the file it was read from, by its size and modification time, the
// time in epoch milliseconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Kept {
  size: u64,
  modified_ms: u64,
  nouns: Vec<Noun>,
}

// What the cache file holds: each unit command's path to its kept answer.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct Cache {
  units: BTreeMap<PathBuf, Kept>,
}

/// The nouns of every unit command in `units`, from the cache where the command is unchanged and asked
/// afresh where it is not, every command asked at once; the cache is then written again where it changed.
pub fn declared(units: &[Unit]) -> Declared {
  let file = cache_file();
  let before: Cache = file
    .as_deref()
    .and_then(|file| std::fs::read(file).ok())
    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    .unwrap_or_default();
  let stamps: Vec<Option<(u64, u64)>> = units.iter().map(|unit| stamp(&unit.path)).collect();
  let mut nouns: Vec<Option<Vec<Noun>>> = units
    .iter()
    .zip(&stamps)
    .map(|(unit, stamp)| {
      let kept = before.units.get(&unit.path)?;

      (Some((kept.size, kept.modified_ms)) == *stamp).then(|| kept.nouns.clone())
    })
    .collect();

  std::thread::scope(|scope| {
    let asked: Vec<_> = units
      .iter()
      .zip(&nouns)
      .enumerate()
      .filter(|(_, (_, kept))| kept.is_none())
      .map(|(at, (unit, _))| (at, scope.spawn(move || nouns::ask(&unit.path))))
      .collect();

    for (at, asking) in asked {
      nouns[at] = Some(asking.join().unwrap_or_default());
    }
  });

  let nouns: Vec<Vec<Noun>> = nouns.into_iter().map(Option::unwrap_or_default).collect();
  let after = Cache {
    units: units
      .iter()
      .zip(&stamps)
      .zip(&nouns)
      .filter_map(|((unit, stamp), nouns)| {
        let (size, modified_ms) = (*stamp)?;

        Some((
          unit.path.clone(),
          Kept {
            size,
            modified_ms,
            nouns: nouns.clone(),
          },
        ))
      })
      .collect(),
  };

  if after != before
    && let Some(file) = file.as_deref()
  {
    keep(file, &after);
  }

  Declared {
    units: units.to_vec(),
    nouns,
  }
}

// The cache file under the user's cache directory, where one can be named.
fn cache_file() -> Option<PathBuf> {
  let base = std::env::var_os("XDG_CACHE_HOME")
    .filter(|named| !named.is_empty())
    .map(PathBuf::from)
    .or_else(|| {
      std::env::var_os("HOME")
        .filter(|named| !named.is_empty())
        .map(|home| PathBuf::from(home).join(".cache"))
    })?;

  Some(base.join("bmo-cli").join("nouns.json"))
}

// A file's size and modification time in epoch milliseconds, which change whenever the file is replaced or
// written.
fn stamp(path: &Path) -> Option<(u64, u64)> {
  let meta = std::fs::metadata(path).ok()?;
  let since = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;

  Some((meta.len(), u64::try_from(since.as_millis()).ok()?))
}

// The cache written whole beside itself and moved into place, so a reader never sees half of it; a cache that
// cannot be written is passed over.
fn keep(file: &Path, cache: &Cache) {
  let Some(directory) = file.parent() else { return };
  let Ok(bytes) = serde_json::to_vec(cache) else { return };
  let beside = directory.join(format!("nouns.json.{}", std::process::id()));

  if std::fs::create_dir_all(directory).is_ok()
    && std::fs::write(&beside, bytes).is_ok()
    && std::fs::rename(&beside, file).is_err()
  {
    let _ = std::fs::remove_file(&beside);
  }
}

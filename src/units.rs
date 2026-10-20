//! The unit commands on this machine: executables named `bmo-<name>`, in the directory holding `bmo`, then
//! on `PATH` (docs/api.md, 「The unit command」).

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// One unit command: its name, `bmo-job`, and the file that answers to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
  pub name: String,
  pub path: PathBuf,
}

/// The directory holding the running `bmo`, where it can be told.
pub fn own_directory() -> Option<PathBuf> {
  std::env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

/// Every unit command found, the first file of each name in the order the directories are searched.
pub fn found(own: Option<&Path>, search_path: Option<&OsStr>) -> Vec<Unit> {
  let directories = own
    .map(Path::to_path_buf)
    .into_iter()
    .chain(search_path.into_iter().flat_map(std::env::split_paths));
  let mut seen = BTreeSet::new();
  let mut units = Vec::new();

  for directory in directories {
    let Ok(entries) = std::fs::read_dir(&directory) else { continue };
    let mut here: Vec<Unit> = entries
      .filter_map(Result::ok)
      .filter_map(|entry| {
        let name = entry.file_name().into_string().ok()?;
        let path = entry.path();

        (name.len() > "bmo-".len() && name.starts_with("bmo-") && executable(&path))
          .then_some(Unit { name, path })
      })
      .collect();

    here.sort_by(|one, other| one.name.cmp(&other.name));

    for unit in here {
      if seen.insert(unit.name.clone()) {
        units.push(unit);
      }
    }
  }

  units
}

// A regular file, followed through links, that someone may run.
fn executable(path: &Path) -> bool {
  std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

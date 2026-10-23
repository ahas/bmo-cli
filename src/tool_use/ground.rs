//! Where the tool verb stands: the data root, and a variable read from the environment, else from the env file
//! a unit publishes under the data root (docs/api.md, 「The tool verb」).

use std::path::{Path, PathBuf};

/// The data root: `BMO_DATA_DIR`, else `.local/share/bmo/data` in the home directory.
pub fn data_root() -> PathBuf {
  match variable("BMO_DATA_DIR") {
    Some(root) => PathBuf::from(root),
    None => PathBuf::from(variable("HOME").unwrap_or_default()).join(".local/share/bmo/data"),
  }
}

/// A variable of the environment, where it is set and not empty.
pub fn variable(name: &str) -> Option<String> {
  std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// A variable of the environment, else the value the env file `file` under `root` gives it.
pub fn published(name: &str, root: &Path, file: &str) -> Option<String> {
  variable(name).or_else(|| {
    dotenvy::from_path_iter(root.join(file))
      .ok()?
      .filter_map(Result::ok)
      .find(|(key, _)| key == name)
      .map(|(_, value)| value)
      .filter(|value| !value.is_empty())
  })
}

//! The cli's own settings (docs/boundary.md, 「The settings」): one TOML file, each unit's table read by the
//! module carrying that unit's commands alone.

use std::path::PathBuf;

use serde::Deserialize;

/// The table a unit's module reads, as written; empty where the file or the table does not stand.
pub type UnitTable = toml::Table;

#[derive(Deserialize, Default)]
struct Settings {
  #[serde(default)]
  unit: toml::Table,
}

/// The settings file: the one `BMO_CLI_SETTINGS` names, else `settings.toml` in the release `bmo` stands in.
pub fn file() -> Option<PathBuf> {
  if let Some(named) = std::env::var_os("BMO_CLI_SETTINGS").filter(|named| !named.is_empty()) {
    return Some(PathBuf::from(named));
  }

  crate::units::own_directory()?.parent().map(|release| release.join("settings.toml"))
}

/// The table of the unit command `unit`; a file that does not read is refused, naming it and why.
pub fn unit(unit: &str) -> Result<UnitTable, String> {
  let Some(file) = file() else { return Ok(UnitTable::new()) };
  let Ok(text) = std::fs::read_to_string(&file) else { return Ok(UnitTable::new()) };
  let settings: Settings = toml::from_str(&text)
    .map_err(|why| format!("the settings at {} do not read: {why}", file.display()))?;

  match settings.unit.get(unit) {
    None => Ok(UnitTable::new()),
    Some(toml::Value::Table(table)) => Ok(table.clone()),
    Some(_) => Err(format!("the settings at {} hold `unit.{unit}` as no table", file.display())),
  }
}

/// A unit's base address and token, as its table names them, or the setting that is missing.
pub fn reach(unit: &str) -> Result<(String, String), String> {
  let table = self::unit(unit)?;
  let named = |key: &str| {
    table
      .get(key)
      .and_then(toml::Value::as_str)
      .filter(|value| !value.is_empty())
      .map(str::to_owned)
      .ok_or_else(|| format!("the settings name no `unit.{unit}.{key}`"))
  };

  Ok((named("address")?, named("token")?))
}

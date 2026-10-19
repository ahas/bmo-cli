//! The turn a command is run in, read as bmo-agency's tools read it (docs/boundary.md, 「The turn」): the
//! process's environment, where `BMO_AGENT_PLACE` names an agent's place overlaid by the variables of the run
//! that place's `run.json` names, none for a person's turn.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

const PLACE: &str = "BMO_AGENT_PLACE";
const AGENT: &str = "BMO_TURN_AGENT";
const JOB_MARK: &str = "BMO_TASK_LOCKED";
// The run's own variables, which a place's run gives where a place is named.
const RUN_VARIABLES: [&str; 3] = ["BMO_RUN_NAME", "BMO_TASK_LOCKED", "BMO_WORK_ORDER_STEP"];
const RUN_PREFIX: &str = "BMO_TURN_";

#[derive(Deserialize)]
struct PlaceRun {
  #[serde(default)]
  person: bool,
  #[serde(default)]
  env: BTreeMap<String, String>,
}

/// The turn's variables: the process's own, or with a place named, the process's without the run's own and
/// the place's run over them.
pub fn variables() -> BTreeMap<String, String> {
  let own: BTreeMap<String, String> = std::env::vars().collect();
  let Some(place) = own.get(PLACE).filter(|place| !place.is_empty()) else { return own };
  let run = std::fs::read_to_string(Path::new(place).join("run.json"))
    .ok()
    .and_then(|text| serde_json::from_str::<PlaceRun>(&text).ok());
  let mut turn: BTreeMap<String, String> = own
    .iter()
    .filter(|(name, _)| !(RUN_VARIABLES.contains(&name.as_str()) || name.starts_with(RUN_PREFIX)))
    .map(|(name, value)| (name.clone(), value.clone()))
    .collect();

  if let Some(run) = run.filter(|run| !run.person) {
    turn.extend(run.env);
  }

  turn
}

/// The agent whose turn this is, where one is.
pub fn agent() -> Option<String> {
  variables()
    .get(AGENT)
    .map(|named| named.trim().to_owned())
    .filter(|named| !named.is_empty())
}

/// Whether a job runs this: an agent program's session that the job runner holds.
pub fn inside_job() -> bool {
  let set = |value: Option<&String>| value.is_some_and(|value| !value.is_empty());

  set(std::env::var("CLAUDECODE").ok().as_ref()) && set(variables().get(JOB_MARK))
}

//! A described tool as text to read (docs/api.md, 「`use`」): what it does, its manual as written, each argument on
//! a line of its own, and how a call of it is written, so a reader reads the manual's lines rather than a JSON string
//! holding them.

use serde_json::Value;

// The fields of a JSON Schema a property's line says beside its type.
fn property_line(name: &str, schema: &Value, required: bool) -> String {
  let kind = match schema.get("type") {
    Some(Value::String(kind)) => kind.clone(),
    Some(Value::Array(kinds)) => kinds.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" or "),
    _ if schema.get("enum").is_some() => "one of the values below".to_owned(),
    _ => "any".to_owned(),
  };
  let kind = match (kind.as_str(), schema.pointer("/items/type").and_then(Value::as_str)) {
    ("array", Some(item)) => format!("array of {item}"),
    _ => kind,
  };
  let mut line = format!("- {name} ({kind}, {})", if required { "required" } else { "optional" });

  if let Some(description) = schema.get("description").and_then(Value::as_str) {
    line.push_str(": ");
    line.push_str(description.trim());
  }

  if let Some(values) = schema.get("enum").and_then(Value::as_array) {
    let values: Vec<String> = values.iter().map(|value| value.to_string()).collect();

    line.push_str(&format!(" One of: {}.", values.join(", ")));
  }

  if let Some(default) = schema.get("default") {
    line.push_str(&format!(" Default: {default}."));
  }

  line
}

// The arguments an input schema names, required ones first, each on a line.
fn argument_lines(input: &Value) -> Vec<String> {
  let Some(properties) = input.get("properties").and_then(Value::as_object) else {
    return vec![];
  };
  let required: Vec<&str> = input
    .get("required")
    .and_then(Value::as_array)
    .map(|names| names.iter().filter_map(Value::as_str).collect())
    .unwrap_or_default();
  let mut names: Vec<&String> = properties.keys().collect();

  names.sort_by_key(|name| !required.contains(&name.as_str()));

  names
    .into_iter()
    .map(|name| property_line(name, &properties[name], required.contains(&name.as_str())))
    .collect()
}

/// The tool the host described, as text: its name and how it is reached, its description, its manual, its
/// arguments, the shape of its answer where it names one, and a call of it written out.
pub fn text(tool: &str, shape: &Value) -> String {
  let field =
    |name: &str| shape.get(name).and_then(Value::as_str).map(str::trim).filter(|text| !text.is_empty());
  let mut parts = vec![match field("reachedBy") {
    Some(reached_by) => format!("{tool} (reached by {reached_by})"),
    None => tool.to_owned(),
  }];

  if let Some(description) = field("description") {
    parts.push(description.to_owned());
  }

  if let Some(manual) = field("manual") {
    parts.push(format!("Manual:\n{manual}"));
  }

  let input = shape.get("input").cloned().unwrap_or(Value::Null);
  let arguments = argument_lines(&input);

  parts.push(if arguments.is_empty() {
    "Arguments: none.".to_owned()
  } else {
    format!("Arguments:\n{}", arguments.join("\n"))
  });

  if let Some(output) = shape.get("output").filter(|output| !output.is_null()) {
    parts.push(format!("Answer shape:\n{}", serde_json::to_string_pretty(output).unwrap_or_default()));
  }

  parts.push(format!(
    "Call it: bmo use {tool} --json '{{...}}' (or --arg <key>=<value> per argument). An answer with isError true is a refusal; read its words for what to do next."
  ));

  parts.join("\n\n") + "\n"
}

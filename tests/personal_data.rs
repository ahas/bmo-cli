//! No file of the checkout holds personal data (docs/PERSONAL-DATA.md).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;

/// The repository root, where the package stands.
fn root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).to_owned()
}

/// One form the checkout may not hold, and what a failure calls it.
struct PersonalData {
  kind: &'static str,
  pattern: Regex,
  allowed: fn(&str) -> bool,
}

/// The owner's own address is the one mail address the repository allows.
fn the_owner_s_mail(found: &str) -> bool {
  found.eq_ignore_ascii_case("dev@ahas.io")
}

/// A home path under the owner's account is allowed; the form stops at the account name.
fn the_owner_s_home(found: &str) -> bool {
  found.rsplit(['/', '\\']).next() == Some("ahas")
}

fn nothing_allowed(_: &str) -> bool {
  false
}

/// The forms no file of the checkout may hold, each carrying what it allows (docs/PERSONAL-DATA.md).
fn personal_data() -> &'static [PersonalData] {
  static RULES: LazyLock<Vec<PersonalData>> = LazyLock::new(|| {
    let rule = |kind, source: &str, allowed: fn(&str) -> bool| PersonalData {
      kind,
      pattern: Regex::new(source).expect("a personal-data form"),
      allowed,
    };

    vec![
      rule("a mail address", r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}", the_owner_s_mail),
      rule("a home path", r"(?:/home|/Users|/root)/[A-Za-z0-9._-]+", the_owner_s_home),
      rule("a home path", r"[A-Za-z]:\\Users\\[A-Za-z0-9._-]+", the_owner_s_home),
      rule("a telephone number", r"\b01[016789][-. ]?[0-9]{3,4}[-. ][0-9]{4}\b", nothing_allowed),
      rule("a telephone number", r"\b0[2-9][0-9]?[-. ][0-9]{3,4}[-. ][0-9]{4}\b", nothing_allowed),
      rule("a telephone number", r"\+\d{1,3}[-. ]\d{2,4}[-. ]\d{3,4}", nothing_allowed),
      rule("a token", r"\bgh[pousr]_[A-Za-z0-9]{20,}", nothing_allowed),
      rule("a token", r"\bgithub_pat_[A-Za-z0-9_]{20,}", nothing_allowed),
      rule("a token", r"\bsk-[A-Za-z0-9_-]{20,}", nothing_allowed),
      rule("a token", r"\bxox[baprs]-[A-Za-z0-9-]{10,}", nothing_allowed),
      rule("a token", r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b", nothing_allowed),
      rule("a token", r"\bAIza[0-9A-Za-z_-]{35}", nothing_allowed),
      rule("a token", r"\bglpat-[A-Za-z0-9_-]{20,}", nothing_allowed),
      rule("a token", r"\bnpm_[A-Za-z0-9]{36}\b", nothing_allowed),
      rule(
        "a token",
        r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
        nothing_allowed,
      ),
      rule("a token", r"-----BEGIN (?:[A-Z]+ )?PRIVATE KEY-----", nothing_allowed),
      rule(
        "a private network address",
        r"\b10\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\b",
        nothing_allowed,
      ),
      rule(
        "a private network address",
        r"\b172\.(?:1[6-9]|2[0-9]|3[01])\.[0-9]{1,3}\.[0-9]{1,3}\b",
        nothing_allowed,
      ),
      rule("a private network address", r"\b192\.168\.[0-9]{1,3}\.[0-9]{1,3}\b", nothing_allowed),
      rule("a private network address", r"\b169\.254\.[0-9]{1,3}\.[0-9]{1,3}\b", nothing_allowed),
      rule("a private network address", r"\bf[cd][0-9a-f]{2}:", nothing_allowed),
    ]
  });

  RULES.as_slice()
}

/// Every file of the checkout: the ones git tracks and the new ones it does not ignore.
fn checkout_files() -> Vec<String> {
  let output = Command::new("git")
    .args(["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
    .current_dir(root())
    .output()
    .expect("git ls-files runs");

  assert!(
    output.status.success(),
    "git ls-files failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  String::from_utf8(output.stdout)
    .expect("paths are UTF-8")
    .split('\0')
    .filter(|path| !path.is_empty() && root().join(path).is_file())
    .map(str::to_owned)
    .collect()
}

/// The kinds a line holds, each found by its form and not allowed by its rule.
fn personal_data_in(line: &str) -> BTreeSet<&'static str> {
  personal_data()
    .iter()
    .filter(|rule| rule.pattern.find_iter(line).any(|found| !(rule.allowed)(found.as_str())))
    .map(|rule| rule.kind)
    .collect()
}

#[test]
fn no_file_of_the_checkout_holds_personal_data() {
  let mut found = Vec::new();

  for path in checkout_files() {
    let bytes = fs::read(root().join(&path)).expect("a file is readable");
    for (index, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
      for kind in personal_data_in(line) {
        found.push(format!("{path}:{} holds {kind}", index + 1));
      }
    }
  }

  assert!(found.is_empty(), "these files hold personal data (docs/PERSONAL-DATA.md): {found:#?}");
}

#[test]
fn the_check_finds_each_form_and_admits_the_owner_s_values() {
  // The forms are put together here so that this file does not itself hold one.
  let (at, user, folder) = ("@", "tester", "notes");
  let (network, unit) = ("10", "0");
  let (prefix, middle, last) = ("010", "1234", "5678");
  let body = "a".repeat(36);

  let examples = [
    ("a mail address", format!("{user}{at}tester.test")),
    ("a home path", format!("/home/{user}/{folder}")),
    ("a telephone number", format!("{prefix}-{middle}-{last}")),
    ("a token", format!("ghp_{body}")),
    ("a private network address", format!("{network}.{unit}.{unit}.1")),
  ];

  for (kind, text) in examples {
    assert!(personal_data_in(&text).contains(kind), "{kind} is not found in its own form");
  }

  let tilde = format!("~/{folder}/key");

  for text in ["a neutral line", "dev@ahas.io", "/home/ahas/notes", tilde.as_str()] {
    assert!(personal_data_in(text).is_empty(), "the check flags the allowed value {text}");
  }
}

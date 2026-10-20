//! The command line as `bmo` reads it: its own options before the first word, and the words after.

use std::ffi::OsString;

use clap::Parser;

/// The words `bmo` is given, its own options taken out (docs/api.md, 「Resolving a command」).
#[derive(Parser, Debug)]
#[command(
  name = "bmo",
  disable_help_flag = true,
  disable_version_flag = true,
  disable_help_subcommand = true
)]
pub struct Line {
  /// The road the caller was told its turn is read on, handed on with the command.
  #[arg(long, value_name = "ROAD")]
  pub via: Option<String>,
  /// The help, listing every command found.
  #[arg(short = 'h', long)]
  pub help: bool,
  /// The version of `bmo`.
  #[arg(long)]
  pub version: bool,
  /// The command and its words, handed on unchanged.
  #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "WORDS")]
  pub words: Vec<OsString>,
}

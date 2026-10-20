//! `bmo`, the platform's thin entry point: it finds the unit that carries a command and hands the command
//! to it (docs/api.md).

use std::process::ExitCode;

fn main() -> ExitCode {
  bmo_cli::run(std::env::args_os())
}

use crate::cli::{Common, ValidateArgs};
use std::process::ExitCode;

pub fn run(_common: Common, _args: ValidateArgs) -> ExitCode {
    eprintln!("validate: not yet implemented (Phase 1b)");
    ExitCode::from(2)
}

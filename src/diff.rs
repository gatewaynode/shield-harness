use crate::cli::{Common, DiffArgs};
use std::process::ExitCode;

pub fn run(_common: Common, _args: DiffArgs) -> ExitCode {
    eprintln!("diff: not yet implemented (Phase 4)");
    ExitCode::from(2)
}

use crate::cli::{Common, ImportArgs};
use std::process::ExitCode;

pub fn run(_common: Common, _args: ImportArgs) -> ExitCode {
    eprintln!("import: not yet implemented (Phase 5)");
    ExitCode::from(2)
}

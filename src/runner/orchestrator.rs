use crate::cli::{Common, InspectArgs, RunArgs};
use std::process::ExitCode;

pub fn run(_common: Common, _args: RunArgs) -> ExitCode {
    eprintln!("run: not yet implemented (Phase 2)");
    ExitCode::from(2)
}

pub fn inspect(_common: Common, _args: InspectArgs) -> ExitCode {
    eprintln!("inspect: not yet implemented (Phase 7)");
    ExitCode::from(2)
}

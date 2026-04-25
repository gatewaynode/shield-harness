use crate::cli::{Common, SynthArgs};
use std::process::ExitCode;

pub fn run(_common: Common, _args: SynthArgs) -> ExitCode {
    eprintln!("synth: not yet implemented (Phase 6)");
    ExitCode::from(2)
}

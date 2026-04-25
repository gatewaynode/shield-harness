use std::process::ExitCode;

mod cli;
mod corpus;
mod diff;
mod import;
mod metrics;
mod report;
mod runner;
mod synth;
mod util;

fn main() -> ExitCode {
    cli::run()
}

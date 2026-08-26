use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;
use sift_cli::{Cli, execute};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let stdout = io::stdout();
    let mut lock = stdout.lock();

    match execute(cli.command, &mut lock) {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            let _ = writeln!(io::stderr(), "error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

//! Launcher for the `matra` command line.
//!
//! Everything the command does lives in [`matra::cli`], which is compiled
//! into the library so the Rust binary and the Python entry point run the
//! same program rather than two that agree by inspection. This file does
//! the three things a launcher does: collect the arguments, lock the
//! streams, and turn the returned code into the process's own.

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();

    let code = matra::cli::run(std::env::args_os(), &mut out, &mut err);

    // A flush failure here is a broken pipe in almost every case, which
    // the CLI already treats as success; there is nowhere left to report
    // it to in any case.
    let _ = out.flush();
    let _ = err.flush();
    ExitCode::from(code)
}

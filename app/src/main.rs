//! The binary: turn the library's typed startup error into a shell exit status.
//!
//! Everything the app does lives in `allesbuien::run`; this file exists only so
//! that failing to start is reported the way a terminal expects — one line on
//! stderr and a nonzero status — rather than as a panic.

use std::process::ExitCode;

fn main() -> ExitCode {
    match allesbuien::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("allesbuien: {error}");
            ExitCode::FAILURE
        }
    }
}

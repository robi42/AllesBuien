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

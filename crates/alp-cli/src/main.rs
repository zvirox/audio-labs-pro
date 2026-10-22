use std::process::ExitCode;

fn main() -> ExitCode {
    match alp_cli::run_from_env() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("alp-cli: {error:#}");
            ExitCode::FAILURE
        }
    }
}

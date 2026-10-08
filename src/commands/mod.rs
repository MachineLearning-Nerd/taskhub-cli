//! Command implementations. Each returns a `Success` or a `CliError`; none prints or exits.

use crate::cli::{Cli, Command};
use crate::error::{CliError, Result};
use crate::output::Success;
use clap::CommandFactory;
use serde_json::json;

pub fn run(cli: Cli) -> Result<Success> {
    match cli.command {
        Command::Version => Ok(version()),
        Command::Completions(args) => {
            let mut command = Cli::command();
            let mut buffer = Vec::new();
            clap_complete::generate(args.shell, &mut command, "taskhub", &mut buffer);
            Ok(raw(buffer))
        }
        Command::Man => {
            let mut buffer = Vec::new();
            clap_mangen::Man::new(Cli::command())
                .render(&mut buffer)
                .map_err(|e| CliError::internal(format!("Could not render the man page: {e}")))?;
            Ok(raw(buffer))
        }
        _ => Err(CliError::internal("This command is not implemented yet.")),
    }
}

fn version() -> Success {
    let cli = env!("CARGO_PKG_VERSION");
    let commit = crate::CONTRACT_COMMIT;
    Success::new(
        json!({ "cli": cli, "apiVersion": crate::API_VERSION, "contract": commit }),
        format!("taskhub {cli}\nAPI version {}\nContract from TaskHub {commit}", crate::API_VERSION),
    )
}

/// Output that is not an envelope (completions, the man page): printed as-is in both modes.
fn raw(bytes: Vec<u8>) -> Success {
    let mut success = Success::new(serde_json::Value::Null, String::from_utf8_lossy(&bytes).into_owned());
    success.meta.insert("raw".into(), json!(true));
    success
}

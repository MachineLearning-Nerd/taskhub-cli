#![forbid(unsafe_code)]

use clap::Parser;
use std::process::ExitCode;
use taskhub_cli::cli::Cli;
use taskhub_cli::error::{CliError, Code};
use taskhub_cli::output::{self, Mode};

fn main() -> ExitCode {
    // Decide the format before parsing, so even a usage error comes out as JSON when piped.
    let args: Vec<String> = std::env::args().collect();
    let early_mode = Mode::choose(args.iter().any(|a| a == "--json"), args.iter().any(|a| a == "--human"));
    std::panic::set_hook(Box::new(move |info| {
        let error = CliError::internal(format!("Unexpected failure: {info}"));
        output::print_error(early_mode, &error);
    }));

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return usage_error(early_mode, error),
    };
    let mode = Mode::choose(cli.json, cli.human);
    match taskhub_cli::commands::run(cli) {
        Ok(success) if success.meta.contains_key("raw") => {
            print!("{}", success.human);
            ExitCode::SUCCESS
        }
        Ok(success) => {
            output::print_success(mode, &success);
            ExitCode::SUCCESS
        }
        Err(error) => {
            output::print_error(mode, &error);
            ExitCode::from(error.exit_code())
        }
    }
}

/// Help and version are printed by clap as usual; real usage errors become an INVALID_INPUT envelope.
fn usage_error(mode: Mode, error: clap::Error) -> ExitCode {
    use clap::error::ErrorKind;
    if matches!(
        error.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    ) {
        let _ = error.print();
        return if error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        };
    }
    if mode == Mode::Human {
        let _ = error.print();
        return ExitCode::from(2);
    }
    let message = error.render().to_string();
    let first =
        message.lines().next().unwrap_or("Invalid arguments.").trim_start_matches("error: ").to_owned();
    let cli_error =
        CliError::new(Code::InvalidInput, first).with_hint("Run the command with --help to see its usage.");
    output::print_error(mode, &cli_error);
    ExitCode::from(2)
}

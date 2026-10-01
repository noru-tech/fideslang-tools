use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = fideslang_cli::cli::Cli::parse();
    let command = cli.command.name();
    match fideslang_cli::cli::run(cli) {
        Ok(exit) => exit.into(),
        // The reader went away (`fl … | head -1`): stop quietly, as a pipeline expects.
        Err(err) if fideslang_cli::cli::is_broken_pipe(&err) => fideslang_cli::Exit::Ok.into(),
        Err(err) => {
            // anstream::eprintln honors --color / NO_COLOR like the rest of the output.
            let style = anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::AnsiColor::Red.into()));
            anstream::eprintln!("{style}error{style:#}: {err:#}");
            if let Some(hint) = fideslang_cli::cli::hint(&err, command) {
                let style = anstyle::Style::new().bold();
                anstream::eprintln!("  {style}hint{style:#}: {hint}");
            }
            fideslang_cli::Exit::Error.into()
        }
    }
}

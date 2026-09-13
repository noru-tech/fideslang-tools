use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = fideslang_cli::cli::Cli::parse();
    match fideslang_cli::cli::run(cli) {
        Ok(exit) => exit.into(),
        Err(err) => {
            // anstream::eprintln honors --color / NO_COLOR like the rest of the output.
            let style = anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::AnsiColor::Red.into()));
            anstream::eprintln!("{style}error{style:#}: {err:#}");
            fideslang_cli::Exit::Error.into()
        }
    }
}

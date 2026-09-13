//! `fideslang-cli` — the library half of the `fl` command-line tool.
//!
//! Everything lives here so it is testable; `src/main.rs` only parses arguments and maps the
//! result to an exit code.
//!
//! Design in one paragraph: every input file (YAML, JSON, CSV) is parsed into a
//! [`serde_json::Value`] with key order preserved. The vendored IAB Tech Lab taxonomy snapshot is compiled
//! into the binary and parsed lazily. Commands operate on `Value`s and on the typed
//! [`taxonomy::Taxonomy`]; renderers turn results into terminal trees, tables, Graphviz DOT or
//! Mermaid. Nothing here touches the network.

pub mod cli;
pub mod format;
pub mod graph;
pub mod manifest;
pub mod render;
pub mod stats;
pub mod taxonomy;
pub mod validate;

/// Crate version, as compiled in.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Process exit codes used by `fl`.
///
/// * `0` — success
/// * `1` — the command ran but reported findings (validation errors, non-empty diff with
///   `--exit-code`)
/// * `2` — usage, I/O or parse error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Ok,
    Findings,
    Error,
}

impl From<Exit> for std::process::ExitCode {
    fn from(e: Exit) -> Self {
        match e {
            Exit::Ok => std::process::ExitCode::SUCCESS,
            Exit::Findings => std::process::ExitCode::from(1),
            Exit::Error => std::process::ExitCode::from(2),
        }
    }
}

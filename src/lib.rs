//! `fideslang-cli` — library half of the `fl` command-line tool.
//!
//! Everything lives here so it is testable; `src/main.rs` only parses arguments and maps
//! the result to an exit code.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

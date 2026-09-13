//! Command-line surface. Each subcommand lives in its own module and gets a [`Ctx`].

pub mod cat;
pub mod completions;
pub mod convert;
pub mod graph;
pub mod merge;
pub mod split;
pub mod stats;
pub mod taxonomy;
pub mod validate;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use crate::Exit;
use crate::render::{ColorChoice, Output, Theme};
use crate::taxonomy::{Snapshot, Taxonomy, embedded};

const ABOUT: &str = "fl — work with Fideslang taxonomies and Fides manifests";
const LONG_ABOUT: &str = "\
fl — work with Fideslang taxonomies and Fides manifests.

Browse and visualize the Fideslang privacy taxonomy (data categories, data uses, data subjects),
and cat, convert, merge, split, validate, summarize and graph Fides manifests (datasets, systems,
policies, organizations). Two taxonomy snapshots are compiled in: ethyca/fideslang 3.1.4 (default)
and IABTechLab/fideslang 3.0.0; pick one with --taxonomy. Nothing touches the network.

Exit codes: 0 ok · 1 findings (validate errors, non-empty diff with --exit-code) · 2 usage/IO error.";

#[derive(Debug, Parser)]
#[command(name = "fl", version, about = ABOUT, long_about = LONG_ABOUT, propagate_version = true)]
pub struct Cli {
    #[command(flatten)]
    pub global: Global,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Args)]
pub struct Global {
    /// Which vendored taxonomy snapshot to use.
    #[arg(
        long,
        global = true,
        env = "FL_TAXONOMY",
        default_value = "ethyca",
        value_enum
    )]
    pub taxonomy: Snapshot,

    /// When to use colors.
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "auto",
        env = "FL_COLOR"
    )]
    pub color: ColorChoice,

    /// Write output to FILE instead of stdout.
    #[arg(short = 'o', long, global = true, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Suppress summary lines on stderr.
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Browse, search, visualize and diff the bundled taxonomy.
    #[command(subcommand, visible_alias = "tax")]
    Taxonomy(taxonomy::Cmd),
    /// Print manifests (optionally filtered) as YAML, JSON or a tree.
    Cat(cat::Args),
    /// Convert a taxonomy file or manifest between YAML, JSON and CSV.
    Convert(convert::Args),
    /// Union many manifest files into one document.
    Merge(merge::Args),
    /// Split a manifest into one file per resource type (or per resource).
    Split(split::Args),
    /// Check manifests against the taxonomy and for structural problems.
    Validate(validate::Args),
    /// Summarize a manifest set: counts, category usage, uncategorized fields.
    Stats(stats::Args),
    /// Render systems, datasets and data flows as Graphviz DOT or Mermaid.
    Graph(graph::Args),
    /// Generate shell completions.
    Completions(completions::Args),
    /// Generate man pages.
    Manpage(completions::ManArgs),
}

/// Everything a subcommand needs.
pub struct Ctx {
    pub global: Global,
    pub theme: Theme,
    pub tax: &'static Taxonomy,
    pub out: Output,
}

impl Ctx {
    /// Print a summary line to stderr unless `--quiet`.
    pub fn note(&self, msg: impl AsRef<str>) {
        if !self.global.quiet {
            anstream::eprintln!("{}", msg.as_ref());
        }
    }
}

/// Run a parsed command line.
pub fn run(cli: Cli) -> Result<Exit> {
    cli.global.color.apply();
    let mut ctx = Ctx {
        theme: Theme::default(),
        tax: embedded::load(cli.global.taxonomy),
        out: Output::open(cli.global.output.as_deref())?,
        global: cli.global,
    };
    let exit = match cli.command {
        Command::Taxonomy(cmd) => taxonomy::run(&mut ctx, cmd),
        Command::Cat(args) => cat::run(&mut ctx, args),
        Command::Convert(args) => convert::run(&mut ctx, args),
        Command::Merge(args) => merge::run(&mut ctx, args),
        Command::Split(args) => split::run(&mut ctx, args),
        Command::Validate(args) => validate::run(&mut ctx, args),
        Command::Stats(args) => stats::run(&mut ctx, args),
        Command::Graph(args) => graph::run(&mut ctx, args),
        Command::Completions(args) => completions::run(&mut ctx, args),
        Command::Manpage(args) => completions::run_man(&mut ctx, args),
    }?;
    std::io::Write::flush(&mut ctx.out)?;
    Ok(exit)
}

/// Shared `--type` / `--key` filter flags.
#[derive(Debug, Clone, Args, Default)]
pub struct FilterArgs {
    /// Only these resource types (comma-separated or repeated), e.g. `system,dataset`.
    #[arg(short = 't', long = "type", value_name = "TYPE", value_delimiter = ',')]
    pub types: Vec<String>,
    /// Only resources whose fides_key matches this glob (comma-separated or repeated).
    #[arg(short = 'k', long = "key", value_name = "GLOB", value_delimiter = ',')]
    pub keys: Vec<String>,
}

impl FilterArgs {
    pub fn build(&self) -> Result<crate::manifest::filter::Filter> {
        crate::manifest::filter::Filter::new(&self.types, &self.keys)
    }
}

/// Load manifests from `paths` (defaulting to `.fides/`) and apply the filter.
pub fn load_manifests(
    paths: Vec<PathBuf>,
    format: Option<crate::format::Format>,
    filter: &FilterArgs,
) -> Result<crate::manifest::Manifest> {
    let paths = crate::manifest::load::default_paths(paths)?;
    let m = crate::manifest::load::load(&paths, format)?;
    Ok(filter.build()?.apply(&m))
}

/// Parse a taxonomy kind argument (`categories`, `uses`, `subjects`, many aliases).
pub fn parse_kind(s: &str) -> Result<crate::taxonomy::Kind, String> {
    crate::taxonomy::Kind::parse(s)
        .ok_or_else(|| format!("expected one of: categories, uses, subjects (got `{s}`)"))
}

/// `all` or a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindSel {
    All,
    One(crate::taxonomy::Kind),
}

impl KindSel {
    pub fn kinds(self) -> Vec<crate::taxonomy::Kind> {
        match self {
            KindSel::All => crate::taxonomy::Kind::ALL.to_vec(),
            KindSel::One(k) => vec![k],
        }
    }
}

pub fn parse_kind_sel(s: &str) -> Result<KindSel, String> {
    if s.eq_ignore_ascii_case("all") {
        return Ok(KindSel::All);
    }
    parse_kind(s)
        .map(KindSel::One)
        .map_err(|e| format!("{e}; or `all`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_definition_is_consistent() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}

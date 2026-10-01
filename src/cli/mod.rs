//! Command-line surface. Each subcommand lives in its own module and gets a [`Ctx`].

pub mod cat;
pub mod completions;
pub mod convert;
pub mod doctor;
pub mod graph;
pub mod merge;
pub mod split;
pub mod stats;
pub mod taxonomy;
pub mod validate;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use crate::Exit;
use crate::render::{ColorChoice, Output, Theme};
use crate::taxonomy::{Kind, Taxonomy, embedded};

const ABOUT: &str = "fl — work with Fideslang taxonomies and Fides manifests";
const LONG_ABOUT: &str = "\
fl — work with Fideslang taxonomies and Fides manifests.

Browse and visualize the Fideslang privacy taxonomy (data categories, data uses, data subjects),
and cat, convert, merge, split, validate, summarize and graph Fides manifests (datasets, systems,
policies, organizations). The IAB Tech Lab Privacy Taxonomy (fideslang 3.0.0) is compiled in, so
nothing touches the network.

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
    /// When to use colors.
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "auto",
        env = "FL_COLOR"
    )]
    pub color: ColorChoice,

    /// Never use colors; the same as `--color never` (and wins over it and over FL_COLOR).
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Write output to FILE instead of stdout.
    #[arg(short = 'o', long, global = true, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Suppress summary lines on stderr.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Print extra diagnostics to stderr (files loaded, counts, timings). Never changes stdout.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Where `--color` came from (filled in by [`parse`], for `fl doctor`).
    #[arg(skip)]
    pub color_source: ColorSource,
}

/// Where the `--color` setting came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSource {
    /// Not given: the default, `auto`.
    #[default]
    Default,
    /// `--color` on the command line.
    Flag,
    /// The `FL_COLOR` environment variable.
    Env,
}

/// Parse the process's command line (exits on a usage error, like `Cli::parse`), recording where
/// `--color` came from.
pub fn parse() -> Cli {
    use clap::{CommandFactory, FromArgMatches, parser::ValueSource};
    let matches = Cli::command().get_matches();
    let mut cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    cli.global.color_source = match matches.value_source("color") {
        Some(ValueSource::CommandLine) => ColorSource::Flag,
        Some(ValueSource::EnvVariable) => ColorSource::Env,
        _ => ColorSource::Default,
    };
    cli
}

impl Global {
    /// The color setting in effect: `--no-color` wins over `--color` and `FL_COLOR`.
    pub fn color_choice(&self) -> ColorChoice {
        if self.no_color {
            ColorChoice::Never
        } else {
            self.color
        }
    }
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
    /// Report version, bundled taxonomy, color decision, ./.fides and installed completions.
    Doctor(doctor::Args),
}

impl Command {
    /// The subcommand as typed after `fl`, for messages (`validate`, `taxonomy diff`).
    pub fn name(&self) -> &'static str {
        match self {
            Command::Taxonomy(cmd) => cmd.name(),
            Command::Cat(_) => "cat",
            Command::Convert(_) => "convert",
            Command::Merge(_) => "merge",
            Command::Split(_) => "split",
            Command::Validate(_) => "validate",
            Command::Stats(_) => "stats",
            Command::Graph(_) => "graph",
            Command::Completions(_) => "completions",
            Command::Manpage(_) => "manpage",
            Command::Doctor(_) => "doctor",
        }
    }
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

    /// Print a diagnostic line to stderr with `--verbose`. Never writes to stdout.
    pub fn debug(&self, msg: impl AsRef<str>) {
        verbose(self.global.verbose, msg);
    }
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

fn verbose(on: bool, msg: impl AsRef<str>) {
    if on {
        let style = anstyle::Style::new().dimmed();
        anstream::eprintln!("{style}verbose:{style:#} {}", msg.as_ref());
    }
}

/// Run a parsed command line.
pub fn run(cli: Cli) -> Result<Exit> {
    let started = Instant::now();
    cli.global.color_choice().apply();
    let v = cli.global.verbose;
    verbose(
        v,
        format!("fl {} · {}", crate::version(), cli.command.name()),
    );
    let tax = embedded::load();
    verbose(
        v,
        format!(
            "taxonomy {}: {} data categories, {} data uses, {} data subjects ({:.1?})",
            tax.label(),
            tax.table(Kind::Category).len(),
            tax.table(Kind::Use).len(),
            tax.table(Kind::Subject).len(),
            started.elapsed()
        ),
    );
    let mut ctx = Ctx {
        theme: Theme::default(),
        tax,
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
        Command::Doctor(args) => doctor::run(&mut ctx, args),
    }?;
    std::io::Write::flush(&mut ctx.out)?;
    ctx.debug(format!(
        "exit {} after {:.1?}",
        exit.code(),
        started.elapsed()
    ));
    Ok(exit)
}

/// True when `err` comes from writing to a closed pipe (`fl … | head -1`). `fl` then stops quietly
/// with exit 0, like other Unix tools.
pub fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain().any(|e| {
        e.downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe)
    })
}

/// A one-line fix suggestion for errors the user can fix on the command line.
pub fn hint(err: &anyhow::Error, command: &str) -> Option<String> {
    err.chain()
        .find_map(|e| e.downcast_ref::<crate::manifest::load::PathError>())
        .map(|_| format!("pass a file or directory, e.g. fl {command} path/to/manifests"))
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
    ctx: &Ctx,
    paths: Vec<PathBuf>,
    format: Option<crate::format::Format>,
    filter: &FilterArgs,
) -> Result<crate::manifest::Manifest> {
    let m = load_unfiltered(ctx, paths, format)?;
    let filter = filter.build()?;
    if filter.is_noop() {
        return Ok(m);
    }
    let filtered = filter.apply(&m);
    ctx.debug(format!(
        "filter kept {} of {} resources",
        filtered.len(),
        m.len()
    ));
    Ok(filtered)
}

/// Load manifests from `paths` (defaulting to `.fides/`), logging each file with `--verbose`.
pub fn load_unfiltered(
    ctx: &Ctx,
    paths: Vec<PathBuf>,
    format: Option<crate::format::Format>,
) -> Result<crate::manifest::Manifest> {
    let started = Instant::now();
    let paths = crate::manifest::load::default_paths(paths)?;
    let m = crate::manifest::load::load_each(&paths, format, |p, added| {
        ctx.debug(format!(
            "loaded {} ({})",
            p.display(),
            plural(added, "resource")
        ));
    })?;
    ctx.debug(format!(
        "{}, {} loaded in {:.1?}",
        plural(m.files().len(), "file"),
        plural(m.len(), "resource"),
        started.elapsed()
    ));
    Ok(m)
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

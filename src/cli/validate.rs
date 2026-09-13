//! `fl validate`

use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args as ClapArgs, ValueEnum};

use super::{Ctx, FilterArgs, load_manifests};
use crate::Exit;
use crate::format::{self, Format};
use crate::validate::{self, Options, Report, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutFormat {
    Text,
    Json,
    /// GitHub Actions `::error`/`::warning` annotations.
    Github,
}

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Manifest files or directories (default: ./.fides/).
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub filter: FilterArgs,
    #[arg(long, value_enum)]
    pub from: Option<Format>,
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: OutFormat,
    /// Also warn about fields the upstream models do not define (W005).
    #[arg(long)]
    pub strict: bool,
    /// Treat all warnings as errors.
    #[arg(short = 'W', long)]
    pub warnings_as_errors: bool,
    /// Promote these codes to errors (comma-separated, repeatable).
    #[arg(long, value_name = "CODE", value_delimiter = ',')]
    pub deny: Vec<String>,
    /// Silence these codes (comma-separated, repeatable).
    #[arg(long, value_name = "CODE", value_delimiter = ',')]
    pub allow: Vec<String>,
    /// Ignore data_category / data_use / data_subject resources declared in the manifests.
    #[arg(long)]
    pub no_custom_taxonomy: bool,
}

fn write_text(ctx: &mut Ctx, report: &Report) -> Result<()> {
    let t = ctx.theme;
    for d in &report.diagnostics {
        let sev_style = if d.severity == Severity::Error {
            t.error
        } else {
            t.warning
        };
        let file = d.file.as_deref().unwrap_or("<stdin>");
        writeln!(ctx.out, "{}  {}", t.paint(t.dim, file), d.location())?;
        let mut line = format!("  {} {}", t.paint(sev_style, d.code), d.message);
        if let Some(s) = &d.suggestion {
            line.push_str(&format!(" — {s}"));
        }
        writeln!(ctx.out, "{line}")?;
    }
    let summary = format!(
        "{} files, {} resources checked against {}: {}, {}",
        report.files,
        report.resources,
        report.taxonomy,
        pluralize(report.errors(), "error", &t.error, &t),
        pluralize(report.warnings(), "warning", &t.warning, &t),
    );
    if report.diagnostics.is_empty() {
        writeln!(ctx.out, "{} {summary}", t.paint(t.ok, "OK"))?;
    } else {
        writeln!(ctx.out, "{summary}")?;
    }
    Ok(())
}

fn pluralize(n: usize, word: &str, style: &anstyle::Style, t: &crate::render::Theme) -> String {
    let s = format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    if n > 0 { t.paint(*style, &s) } else { s }
}

fn write_github(ctx: &mut Ctx, report: &Report) -> Result<()> {
    for d in &report.diagnostics {
        let level = if d.severity == Severity::Error {
            "error"
        } else {
            "warning"
        };
        let file = d
            .file
            .as_deref()
            .map(|f| format!(" file={f}"))
            .unwrap_or_default();
        let title = format!("{} {}", d.code, d.location());
        let mut msg = d.message.clone();
        if let Some(s) = &d.suggestion {
            msg.push_str(&format!(" ({s})"));
        }
        writeln!(
            ctx.out,
            "::{level}{file},title={}::{}",
            gh_escape(&title),
            gh_escape(&msg)
        )?;
    }
    Ok(())
}

fn gh_escape(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

pub fn run(ctx: &mut Ctx, a: Args) -> Result<Exit> {
    let m = load_manifests(a.paths, a.from, &a.filter)?;
    let mut opts = Options {
        strict: a.strict,
        no_custom_taxonomy: a.no_custom_taxonomy,
        deny: a
            .deny
            .iter()
            .map(|c| c.to_ascii_uppercase())
            .collect::<BTreeSet<_>>(),
        allow: a
            .allow
            .iter()
            .map(|c| c.to_ascii_uppercase())
            .collect::<BTreeSet<_>>(),
    };
    if a.warnings_as_errors {
        for code in ["W001", "W002", "W003", "W004", "W005"] {
            opts.deny.insert(code.to_string());
        }
    }
    let report = validate::validate(&m, ctx.tax, &opts);
    match a.format {
        OutFormat::Text => write_text(ctx, &report)?,
        OutFormat::Json => {
            let v = serde_json::to_value(&report)?;
            format::write_value(&mut ctx.out, &v, Format::Json)?;
            ctx.note(format!(
                "{} errors, {} warnings",
                report.errors(),
                report.warnings()
            ));
        }
        OutFormat::Github => {
            write_github(ctx, &report)?;
            ctx.note(format!(
                "{} errors, {} warnings",
                report.errors(),
                report.warnings()
            ));
        }
    }
    Ok(if report.is_clean() {
        Exit::Ok
    } else {
        Exit::Findings
    })
}

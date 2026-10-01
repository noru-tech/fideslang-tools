//! `fl doctor`: what this `fl` is and how it sees its environment. Read-only, offline, and always
//! exits 0; every check is best-effort.

use std::env;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Args as ClapArgs;
use serde::Serialize;

use super::taxonomy::TextFormat;
use super::{ColorSource, Ctx};
use crate::Exit;
use crate::format::{self, Format};
use crate::manifest::load::{self, DEFAULT_DIR};
use crate::render::ColorChoice;
use crate::taxonomy::Kind;

#[derive(Debug, ClapArgs)]
pub struct Args {
    #[arg(long, value_enum, default_value = "text")]
    pub format: TextFormat,
}

/// How `fl` gets updates: it never checks by itself.
pub const UPDATE_POLICY: &str = "fl never contacts the network and never checks for updates; \
update with `brew upgrade fl`, `cargo binstall fideslang-cli` or a download from GitHub Releases";

#[derive(Debug, Serialize)]
pub struct Report {
    pub version: String,
    pub taxonomy: TaxonomyInfo,
    pub color: ColorInfo,
    pub fides_dir: FidesDir,
    pub completions: Completions,
    pub man_page: Option<String>,
    pub updates: &'static str,
}

#[derive(Debug, Serialize)]
pub struct TaxonomyInfo {
    pub label: String,
    pub upstream: String,
    pub tag: String,
    pub commit: String,
    pub snapshot_date: String,
    pub data_categories: usize,
    pub data_uses: usize,
    pub data_subjects: usize,
}

#[derive(Debug, Serialize)]
pub struct ColorInfo {
    /// Whether this run's stdout gets colors.
    pub enabled: bool,
    /// `auto`, `always` or `never`, after `--no-color`.
    pub setting: &'static str,
    /// Why: the flag, the environment variable or the terminal check that decided.
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct FidesDir {
    pub path: String,
    pub exists: bool,
    /// Manifest files (`*.yml`, `*.yaml`, `*.json`) under it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifests: Option<usize>,
    /// Resources they hold, when they load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<usize>,
    /// Why they could not be loaded or listed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Where a completion script was found, per shell (`null` when none was found).
#[derive(Debug, Serialize)]
pub struct Completions {
    pub bash: Option<String>,
    pub zsh: Option<String>,
    pub fish: Option<String>,
}

pub fn run(ctx: &mut Ctx, a: Args) -> Result<Exit> {
    let report = collect(ctx);
    match a.format {
        TextFormat::Text => write_text(ctx, &report)?,
        TextFormat::Json | TextFormat::Yaml => {
            let f = if a.format == TextFormat::Json {
                Format::Json
            } else {
                Format::Yaml
            };
            format::write_value(&mut ctx.out, &serde_json::to_value(&report)?, f)?;
        }
    }
    Ok(Exit::Ok)
}

fn collect(ctx: &Ctx) -> Report {
    let p = ctx.tax.provenance();
    Report {
        version: crate::version().to_string(),
        taxonomy: TaxonomyInfo {
            label: ctx.tax.label(),
            upstream: p.upstream.clone(),
            tag: p.tag.clone(),
            commit: p.commit.clone(),
            snapshot_date: p.snapshot_date.clone(),
            data_categories: ctx.tax.table(Kind::Category).len(),
            data_uses: ctx.tax.table(Kind::Use).len(),
            data_subjects: ctx.tax.table(Kind::Subject).len(),
        },
        color: color(ctx),
        fides_dir: fides_dir(Path::new(DEFAULT_DIR)),
        completions: Completions {
            bash: find(&bash_candidates()),
            zsh: find(&zsh_candidates()),
            fish: find(&fish_candidates()),
        },
        man_page: find(&man_candidates()),
        updates: UPDATE_POLICY,
    }
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.is_empty())
}

/// The color decision for this run's output and the reason, in the order `anstream` applies them.
fn color(ctx: &Ctx) -> ColorInfo {
    let g = &ctx.global;
    let choice = g.color_choice();
    let setting = match choice {
        ColorChoice::Auto => "auto",
        ColorChoice::Always => "always",
        ColorChoice::Never => "never",
    };
    let enabled = !ctx.out.is_file()
        && anstream::AutoStream::choice(&std::io::stdout()) != anstream::ColorChoice::Never;
    let from = |what: &str| match g.color_source {
        ColorSource::Flag => format!("--color {what}"),
        ColorSource::Env => format!("FL_COLOR={what}"),
        ColorSource::Default => format!("--color {what} (default)"),
    };
    let reason = if ctx.out.is_file() {
        "output goes to a file (-o), which never gets colors".to_string()
    } else if g.no_color {
        "--no-color".to_string()
    } else if choice != ColorChoice::Auto {
        from(setting)
    } else {
        let auto = from("auto");
        let clicolor = env::var_os("CLICOLOR");
        let tty = std::io::stdout().is_terminal();
        let term_ok = env::var_os("TERM").is_some_and(|t| t != "dumb");
        let why = if non_empty_env("NO_COLOR").is_some() {
            "NO_COLOR is set".to_string()
        } else if non_empty_env("CLICOLOR_FORCE").is_some() {
            "CLICOLOR_FORCE is set".to_string()
        } else if clicolor.as_deref().is_some_and(|c| c == "0") {
            "CLICOLOR=0".to_string()
        } else if !tty {
            "stdout is not a terminal".to_string()
        } else if !term_ok && clicolor.is_none() && env::var_os("CI").is_none() {
            "TERM is unset or dumb".to_string()
        } else {
            "stdout is a terminal".to_string()
        };
        format!("{auto}: {why}")
    };
    ColorInfo {
        enabled,
        setting,
        reason,
    }
}

fn fides_dir(dir: &Path) -> FidesDir {
    let mut out = FidesDir {
        path: format!("./{}/", dir.display()),
        exists: dir.is_dir(),
        manifests: None,
        resources: None,
        error: None,
    };
    if !out.exists {
        return out;
    }
    match load::expand_paths(&[dir.to_path_buf()]) {
        Ok(files) => {
            out.manifests = Some(files.len());
            match load::load(&files, None) {
                Ok(m) => out.resources = Some(m.len()),
                Err(e) => out.error = Some(format!("{e:#}")),
            }
        }
        Err(e) => {
            out.manifests = Some(0);
            out.error = Some(format!("{e:#}"));
        }
    }
    out
}

fn find(candidates: &[PathBuf]) -> Option<String> {
    candidates
        .iter()
        .find(|p| p.is_file())
        .map(|p| p.display().to_string())
}

fn home() -> Option<PathBuf> {
    env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

fn xdg(var: &str, fallback: &str) -> Option<PathBuf> {
    env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| home().map(|h| h.join(fallback)))
}

/// Install prefixes to look under: the one the running binary lives in (`<prefix>/bin/fl`, e.g.
/// Homebrew's), `HOMEBREW_PREFIX`, and the usual system ones.
fn prefixes() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(prefix) = env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .and_then(|p| p.parent()?.parent().map(Path::to_path_buf))
    {
        out.push(prefix);
    }
    if let Some(p) = env::var_os("HOMEBREW_PREFIX").filter(|p| !p.is_empty()) {
        out.push(PathBuf::from(p));
    }
    for p in [
        "/opt/homebrew",
        "/usr/local",
        "/usr",
        "/home/linuxbrew/.linuxbrew",
    ] {
        out.push(PathBuf::from(p));
    }
    out.dedup();
    out
}

fn under_prefixes(rel: &str) -> impl Iterator<Item = PathBuf> {
    prefixes().into_iter().map(move |p| p.join(rel))
}

fn bash_candidates() -> Vec<PathBuf> {
    let mut c = Vec::new();
    if let Some(d) = xdg("XDG_DATA_HOME", ".local/share") {
        c.push(d.join("bash-completion/completions/fl"));
    }
    if let Some(h) = home() {
        c.push(h.join(".bash_completion.d/fl"));
    }
    c.extend(under_prefixes("etc/bash_completion.d/fl"));
    c.extend(under_prefixes("share/bash-completion/completions/fl"));
    c.push(PathBuf::from("/etc/bash_completion.d/fl"));
    c
}

fn zsh_candidates() -> Vec<PathBuf> {
    let mut c: Vec<PathBuf> = env::var_os("FPATH")
        .map(|f| env::split_paths(&f).map(|d| d.join("_fl")).collect())
        .unwrap_or_default();
    if let Some(h) = home() {
        c.push(h.join(".zfunc/_fl"));
        c.push(h.join(".zsh/completions/_fl"));
    }
    if let Some(d) = xdg("XDG_DATA_HOME", ".local/share") {
        c.push(d.join("zsh/site-functions/_fl"));
    }
    c.extend(under_prefixes("share/zsh/site-functions/_fl"));
    c.extend(under_prefixes("share/zsh/vendor-completions/_fl"));
    c
}

fn fish_candidates() -> Vec<PathBuf> {
    let mut c = Vec::new();
    if let Some(d) = xdg("XDG_CONFIG_HOME", ".config") {
        c.push(d.join("fish/completions/fl.fish"));
    }
    if let Some(d) = xdg("XDG_DATA_HOME", ".local/share") {
        c.push(d.join("fish/vendor_completions.d/fl.fish"));
    }
    c.extend(under_prefixes("share/fish/vendor_completions.d/fl.fish"));
    c.extend(under_prefixes("share/fish/completions/fl.fish"));
    c
}

fn man_candidates() -> Vec<PathBuf> {
    let mut c: Vec<PathBuf> = env::var_os("MANPATH")
        .map(|m| {
            env::split_paths(&m)
                .filter(|d| !d.as_os_str().is_empty())
                .map(|d| d.join("man1/fl.1"))
                .collect()
        })
        .unwrap_or_default();
    if let Some(d) = xdg("XDG_DATA_HOME", ".local/share") {
        c.push(d.join("man/man1/fl.1"));
    }
    c.extend(under_prefixes("share/man/man1/fl.1"));
    c.extend(under_prefixes("man/man1/fl.1"));
    c
}

fn write_text(ctx: &mut Ctx, r: &Report) -> Result<()> {
    let t = ctx.theme;
    let label = |s: &str| t.paint(t.key, &format!("{s:<12}"));
    let missing = |s: &str| t.paint(t.dim, s);
    writeln!(ctx.out, "{} {}", label("fl"), r.version)?;
    let tx = &r.taxonomy;
    writeln!(
        ctx.out,
        "{} {} ({} data categories, {} data uses, {} data subjects; commit {}, {})",
        label("taxonomy"),
        tx.label,
        tx.data_categories,
        tx.data_uses,
        tx.data_subjects,
        tx.commit.get(..12).unwrap_or(&tx.commit),
        tx.snapshot_date
    )?;
    writeln!(
        ctx.out,
        "{} {} ({})",
        label("color"),
        if r.color.enabled { "on" } else { "off" },
        r.color.reason
    )?;
    let f = &r.fides_dir;
    let fides = if !f.exists {
        missing("not found (manifest commands then need a path)")
    } else {
        let mut s = format!(
            "{} manifest file{}",
            f.manifests.unwrap_or(0),
            if f.manifests == Some(1) { "" } else { "s" }
        );
        if let Some(n) = f.resources {
            s.push_str(&format!(", {n} resource{}", if n == 1 { "" } else { "s" }));
        }
        if let Some(e) = &f.error {
            s.push_str(&format!(" ({})", t.paint(t.warning, e)));
        }
        s
    };
    writeln!(ctx.out, "{} {} {fides}", label(".fides"), f.path)?;
    for (shell, found) in [
        ("bash", &r.completions.bash),
        ("zsh", &r.completions.zsh),
        ("fish", &r.completions.fish),
    ] {
        let value = match found {
            Some(p) => p.clone(),
            None => missing(&format!(
                "not found (generate with `fl completions {shell}`)"
            )),
        };
        writeln!(ctx.out, "{} {value}", label(&format!("{shell} compl.")))?;
    }
    let man = match &r.man_page {
        Some(p) => p.clone(),
        None => missing("not found (generate with `fl manpage --out-dir DIR`)"),
    };
    writeln!(ctx.out, "{} {man}", label("man page"))?;
    writeln!(ctx.out, "{} {}", label("updates"), r.updates)?;
    Ok(())
}

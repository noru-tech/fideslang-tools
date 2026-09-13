//! `fl merge` — union manifest files into one document.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Args as ClapArgs;

use super::{Ctx, FilterArgs, load_manifests};
use crate::Exit;
use crate::format::{self, Format};
use crate::manifest::Manifest;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Manifest files or directories (default: ./.fides/).
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub filter: FilterArgs,
    #[arg(long, value_enum)]
    pub from: Option<Format>,
    /// Output format (default: from the `-o` extension, else YAML).
    #[arg(short = 'f', long, value_enum)]
    pub format: Option<Format>,
    /// Fail if the same fides_key appears twice within a resource type.
    #[arg(long)]
    pub fail_on_duplicate: bool,
    /// Keep only the last occurrence of a duplicated fides_key.
    #[arg(long, conflicts_with = "fail_on_duplicate")]
    pub dedupe: bool,
}

pub fn run(ctx: &mut Ctx, a: Args) -> Result<Exit> {
    let m = load_manifests(a.paths, a.from, &a.filter)?;
    let mut dups = Vec::new();
    for t in m.resource_types() {
        let mut seen = HashSet::new();
        for r in m.of_type(t) {
            if let Some(k) = r.fides_key()
                && !seen.insert(k)
            {
                dups.push(format!("{t}[{k}]"));
            }
        }
    }
    if a.fail_on_duplicate && !dups.is_empty() {
        bail!("duplicate fides_keys: {}", dups.join(", "));
    }
    let merged = if a.dedupe && !dups.is_empty() {
        let mut out = Manifest::new();
        for f in m.files() {
            out.add_file(f);
        }
        // Keep the last occurrence: walk in reverse, then restore order.
        let mut keep: Vec<&crate::manifest::Resource> = Vec::new();
        let mut seen = HashSet::new();
        for r in m.iter().collect::<Vec<_>>().into_iter().rev() {
            let id = (r.resource_type.clone(), r.fides_key().map(str::to_string));
            if r.fides_key().is_none() || seen.insert(id) {
                keep.push(r);
            }
        }
        keep.reverse();
        for r in keep {
            out.push(r.clone());
        }
        out
    } else {
        m.clone()
    };
    let to = a.format.unwrap_or_else(|| {
        ctx.global
            .output
            .as_deref()
            .filter(|o| o.extension().is_some())
            .map(Format::from_path)
            .unwrap_or(Format::Yaml)
    });
    format::write_value(&mut ctx.out, &merged.to_value(), to)?;
    let summary = merged
        .counts()
        .iter()
        .map(|(t, n)| format!("{t} {n}"))
        .collect::<Vec<_>>()
        .join(", ");
    let dest = ctx
        .global
        .output
        .as_deref()
        .map(|p| format!(" → {}", p.display()))
        .unwrap_or_default();
    let dup_note = if dups.is_empty() {
        String::new()
    } else if a.dedupe {
        format!("; deduplicated {}", dups.len())
    } else {
        format!("; {} duplicate keys kept ({})", dups.len(), dups.join(", "))
    };
    ctx.note(format!(
        "merged {} files{dest}: {summary}{dup_note}",
        m.files().len()
    ));
    Ok(Exit::Ok)
}

//! `fl split` — one file per resource type, or per resource.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args as ClapArgs, ValueEnum};
use serde_json::{Map, Value};

use super::{Ctx, FilterArgs, load_manifests};
use crate::Exit;
use crate::format::{self, Format};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum By {
    /// `DIR/<type>.yml`
    Type,
    /// `DIR/<type>/<fides_key>.yml`
    Resource,
}

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Manifest files or directories (default: ./.fides/).
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub filter: FilterArgs,
    #[arg(long, value_enum)]
    pub from: Option<Format>,
    /// Directory to write into (created if missing).
    #[arg(short = 'd', long, value_name = "DIR", default_value = ".")]
    pub out_dir: PathBuf,
    #[arg(long, value_enum, default_value = "type")]
    pub by: By,
    /// Output format.
    #[arg(short = 'f', long, value_enum, default_value = "yaml")]
    pub format: Format,
    /// Overwrite existing files.
    #[arg(long)]
    pub force: bool,
}

pub fn run(ctx: &mut Ctx, a: Args) -> Result<Exit> {
    if a.format == Format::Csv {
        bail!("split writes manifests; use --format yaml or json (convert can produce CSV)");
    }
    let m = load_manifests(a.paths, a.from, &a.filter)?;
    fs::create_dir_all(&a.out_dir)
        .with_context(|| format!("cannot create {}", a.out_dir.display()))?;
    let mut written = Vec::new();
    let mut write = |path: PathBuf, value: &Value| -> Result<()> {
        if path.exists() && !a.force {
            bail!("{} exists (use --force to overwrite)", path.display());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, format::to_string(value, a.format)?)
            .with_context(|| format!("writing {}", path.display()))?;
        written.push(path);
        Ok(())
    };
    match a.by {
        By::Type => {
            for (t, doc) in m.split_by_type() {
                write(
                    a.out_dir.join(format!("{t}.{}", a.format.extension())),
                    &doc,
                )?;
            }
        }
        By::Resource => {
            for r in m.iter() {
                let name = r
                    .fides_key()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("item-{}", r.index));
                let mut doc = Map::new();
                doc.insert(r.resource_type.clone(), Value::Array(vec![r.value.clone()]));
                write(
                    a.out_dir
                        .join(&r.resource_type)
                        .join(format!("{name}.{}", a.format.extension())),
                    &Value::Object(doc),
                )?;
            }
        }
    }
    for p in &written {
        ctx.note(format!("wrote {}", p.display()));
    }
    ctx.note(format!(
        "{} files written to {}",
        written.len(),
        a.out_dir.display()
    ));
    Ok(Exit::Ok)
}

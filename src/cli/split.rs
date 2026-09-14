//! `fl split` — one file per resource type, or per resource.

use std::fs;
use std::path::{Component, Path, PathBuf};

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

/// Resource types and `fides_key`s come straight out of the manifest, so before they become a
/// file or directory name make sure each one is a single plain path component: no separators,
/// no `.`/`..`, no NUL, not empty. Otherwise a crafted manifest could write outside `--out-dir`.
fn plain_file_name<'a>(what: &str, s: &'a str) -> Result<&'a str> {
    let mut parts = Path::new(s).components();
    let single =
        matches!(parts.next(), Some(Component::Normal(c)) if c == s) && parts.next().is_none();
    if !single || s.contains(['\\', '\0']) {
        bail!(
            "{what} `{s}` is not a plain file name (path separators, `.` and `..` are not allowed)"
        );
    }
    Ok(s)
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
                let t = plain_file_name("resource type", &t)?;
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
                let rtype = plain_file_name("resource type", &r.resource_type)
                    .with_context(|| format!("cannot split {}", r.locator()))?;
                let name = plain_file_name("fides_key", &name)
                    .with_context(|| format!("cannot split {}", r.locator()))?;
                let mut doc = Map::new();
                doc.insert(r.resource_type.clone(), Value::Array(vec![r.value.clone()]));
                write(
                    a.out_dir
                        .join(rtype)
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

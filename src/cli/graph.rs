//! `fl graph`

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args as ClapArgs, ValueEnum};

use super::{Ctx, FilterArgs, load_manifests};
use crate::Exit;
use crate::format::{self, Format};
use crate::graph::manifest::{Include, Options};
use crate::render::dot::RankDir;
use crate::render::{dot, mermaid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GraphFormat {
    Dot,
    Mermaid,
    Json,
}

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Manifest files or directories (default: ./.fides/).
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub filter: FilterArgs,
    #[arg(long, value_enum)]
    pub from: Option<Format>,
    #[arg(short, long, value_enum, default_value = "dot")]
    pub format: GraphFormat,
    /// Extra node families to include (comma-separated, repeatable). Default: uses.
    #[arg(
        short = 'i',
        long = "include",
        value_enum,
        value_delimiter = ',',
        value_name = "WHAT"
    )]
    pub include: Vec<Include>,
    /// Only systems, datasets and flows (no taxonomy nodes).
    #[arg(long, conflicts_with = "include")]
    pub bare: bool,
    /// Keep only nodes within --depth hops of this fides_key (or `system:<key>` / `dataset:<key>`).
    #[arg(long, value_name = "KEY")]
    pub focus: Option<String>,
    #[arg(long, value_name = "N", default_value = "2", requires = "focus")]
    pub depth: usize,
    /// Graph direction.
    #[arg(long, value_enum, default_value = "lr")]
    pub rankdir: RankDir,
}

pub fn run(ctx: &mut Ctx, a: Args) -> Result<Exit> {
    let m = load_manifests(a.paths, a.from, &a.filter)?;
    let opts = if a.bare {
        Options::default()
    } else if a.include.is_empty() {
        Options::from_includes(&[Include::Uses])
    } else {
        Options::from_includes(&a.include)
    };
    let mut g = crate::graph::manifest::build(&m, &opts);
    if let Some(focus) = &a.focus {
        let id = if g.get(focus).is_some() {
            focus.clone()
        } else if let Some(n) = [
            "system", "dataset", "use", "cat", "subj", "org", "policy", "ext",
        ]
        .iter()
        .map(|p| format!("{p}:{focus}"))
        .find(|id| g.get(id).is_some())
        {
            n
        } else {
            bail!("`{focus}` is not a node in the graph");
        };
        g = g.focus(&id, a.depth);
    }
    match a.format {
        GraphFormat::Dot => ctx.out.write_all(dot::render(&g, a.rankdir).as_bytes())?,
        GraphFormat::Mermaid => ctx
            .out
            .write_all(mermaid::render(&g, a.rankdir).as_bytes())?,
        GraphFormat::Json => {
            format::write_value(&mut ctx.out, &serde_json::to_value(&g)?, Format::Json)?
        }
    }
    ctx.note(format!(
        "{} nodes, {} edges",
        g.node_count(),
        g.edge_count()
    ));
    Ok(Exit::Ok)
}

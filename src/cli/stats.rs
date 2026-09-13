//! `fl stats`

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args as ClapArgs;

use super::taxonomy::TextFormat;
use super::{Ctx, FilterArgs, load_manifests};
use crate::Exit;
use crate::format::{self, Format};
use crate::render::table::Table;
use crate::stats;
use crate::taxonomy::Kind;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Manifest files or directories (default: ./.fides/).
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub filter: FilterArgs,
    #[arg(long, value_enum)]
    pub from: Option<Format>,
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: TextFormat,
    /// Show the N most used keys per kind.
    #[arg(long, value_name = "N", default_value = "10")]
    pub top: usize,
    /// Also credit every ancestor of a used key (e.g. `user.contact.email` counts for `user.contact` and `user`).
    #[arg(long)]
    pub rollup: bool,
}

pub fn run(ctx: &mut Ctx, a: Args) -> Result<Exit> {
    let m = load_manifests(a.paths, a.from, &a.filter)?;
    let s = stats::compute(&m, ctx.tax, a.rollup);
    match a.format {
        TextFormat::Json | TextFormat::Yaml => {
            let v = serde_json::to_value(&s)?;
            let f = if a.format == TextFormat::Json {
                Format::Json
            } else {
                Format::Yaml
            };
            format::write_value(&mut ctx.out, &v, f)?;
            return Ok(Exit::Ok);
        }
        TextFormat::Text => {}
    }
    let t = ctx.theme;
    let head = |s: &str| t.paint(t.heading, s);

    writeln!(ctx.out, "{}  {} files", head("resources"), s.files)?;
    let mut table = Table::new(&["type", "count"]).style_column(0, t.key);
    for (k, v) in &s.resources {
        table.push(vec![k.clone(), v.to_string()]);
    }
    table.render(&mut ctx.out, None)?;

    let d = &s.datasets;
    writeln!(ctx.out)?;
    writeln!(ctx.out, "{}", head("datasets"))?;
    let pct = (d.categorized_fields * 100)
        .checked_div(d.fields)
        .unwrap_or(0);
    writeln!(
        ctx.out,
        "  {} datasets, {} collections, {} fields, {} categorized ({pct}%)",
        d.datasets, d.collections, d.fields, d.categorized_fields
    )?;
    if !d.uncategorized.is_empty() {
        writeln!(
            ctx.out,
            "  {} uncategorized:",
            t.paint(t.warning, &d.uncategorized.len().to_string())
        )?;
        for u in &d.uncategorized {
            writeln!(ctx.out, "    {u}")?;
        }
    }

    let sy = &s.systems;
    writeln!(ctx.out)?;
    writeln!(ctx.out, "{}", head("systems"))?;
    writeln!(
        ctx.out,
        "  {} systems, {} privacy declarations, {} datasets referenced",
        sy.systems, sy.declarations, sy.referenced_datasets
    )?;
    if !sy.systems_without_declarations.is_empty() {
        writeln!(
            ctx.out,
            "  {} without declarations: {}",
            t.paint(
                t.warning,
                &sy.systems_without_declarations.len().to_string()
            ),
            sy.systems_without_declarations.join(", ")
        )?;
    }
    if !sy.orphan_datasets.is_empty() {
        writeln!(
            ctx.out,
            "  {} datasets not referenced by any system: {}",
            t.paint(t.warning, &sy.orphan_datasets.len().to_string()),
            sy.orphan_datasets.join(", ")
        )?;
    }

    let usage = s.rollup.as_ref().unwrap_or(&s.usage);
    for kind in Kind::ALL {
        let Some(counts) = usage.get(kind.resource_type()) else {
            continue;
        };
        writeln!(ctx.out)?;
        let title = format!(
            "{} used{}",
            kind.label(),
            if a.rollup { " (rolled up)" } else { "" }
        );
        writeln!(ctx.out, "{}  {} distinct", head(&title), counts.len())?;
        let mut rows: Vec<(&String, &usize)> = counts.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let mut table = Table::new(&["key", "uses"]).style_column(0, t.for_kind(kind));
        for (k, n) in rows.iter().take(a.top) {
            table.push(vec![(*k).clone(), n.to_string()]);
        }
        if !table.is_empty() {
            table.render(&mut ctx.out, None)?;
        }
        if rows.len() > a.top {
            writeln!(
                ctx.out,
                "  {}",
                t.paint(t.dim, &format!("… {} more (--top N)", rows.len() - a.top))
            )?;
        }
    }
    Ok(Exit::Ok)
}

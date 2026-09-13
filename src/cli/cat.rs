//! `fl cat` — print manifests, filtered, as YAML / JSON / a resource tree.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args as ClapArgs, ValueEnum};
use serde_json::Value;

use super::{Ctx, FilterArgs, load_manifests};
use crate::Exit;
use crate::format::{self, Format};
use crate::manifest::{Manifest, Resource};
use crate::render::Theme;
use crate::render::tree::{Guides, TreeNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CatFormat {
    Yaml,
    Json,
    Tree,
}

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Manifest files or directories (default: ./.fides/). `-` reads stdin.
    pub paths: Vec<PathBuf>,
    #[command(flatten)]
    pub filter: FilterArgs,
    /// Input format when it cannot be inferred from the extension.
    #[arg(long, value_enum)]
    pub from: Option<Format>,
    #[arg(short, long, value_enum, default_value = "yaml")]
    pub format: CatFormat,
    /// ASCII tree guides.
    #[arg(long)]
    pub ascii: bool,
}

fn strs(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn field_nodes(theme: &Theme, fields: &[Value]) -> Vec<TreeNode> {
    fields
        .iter()
        .filter_map(|f| {
            let name = f.get("name").and_then(Value::as_str)?;
            let cats = strs(f.get("data_categories"));
            let mut label = name.to_string();
            if cats.is_empty() {
                if f.get("fields").is_none() {
                    label.push_str(&theme.paint(theme.dim, "  (uncategorized)"));
                }
            } else {
                label.push_str("  ");
                label.push_str(&theme.paint(theme.category, &cats.join(", ")));
            }
            let children = f
                .get("fields")
                .and_then(Value::as_array)
                .map(|n| field_nodes(theme, n))
                .unwrap_or_default();
            Some(TreeNode::new(label).with_children(children))
        })
        .collect()
}

/// A resource as a tree: datasets show collections → fields → categories; systems show
/// declarations → use / categories / subjects and flows.
pub fn resource_node(theme: &Theme, r: &Resource) -> TreeNode {
    let key = r.fides_key().unwrap_or("?");
    let style = match r.resource_type.as_str() {
        "system" => theme.system,
        "dataset" => theme.dataset,
        "data_category" => theme.category,
        "data_use" => theme.data_use,
        "data_subject" => theme.subject,
        _ => theme.key,
    };
    let mut label = theme.paint(style, key);
    if let Some(n) = r.name() {
        label.push_str(&format!("  {n}"));
    }
    let mut children = Vec::new();
    match r.resource_type.as_str() {
        "dataset" => {
            for c in r
                .value
                .get("collections")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                let cname = c.get("name").and_then(Value::as_str).unwrap_or("?");
                let fields = c
                    .get("fields")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                children.push(
                    TreeNode::new(theme.paint(theme.key, cname))
                        .with_children(field_nodes(theme, &fields)),
                );
            }
        }
        "system" => {
            if let Some(t) = r.value.get("system_type").and_then(Value::as_str) {
                children.push(TreeNode::new(theme.paint(theme.dim, &format!("type: {t}"))));
            }
            for (field, arrow) in [("ingress", "←"), ("egress", "→")] {
                for flow in r
                    .value
                    .get(field)
                    .and_then(Value::as_array)
                    .unwrap_or(&Vec::new())
                {
                    let k = flow
                        .get("fides_key")
                        .and_then(Value::as_str)
                        .or_else(|| flow.as_str())
                        .unwrap_or("?");
                    children.push(TreeNode::new(format!(
                        "{arrow} {field} {}",
                        theme.paint(theme.dataset, k)
                    )));
                }
            }
            for d in r
                .value
                .get("privacy_declarations")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                let name = d
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("(unnamed declaration)");
                let mut kids = Vec::new();
                if let Some(u) = d.get("data_use").and_then(Value::as_str) {
                    kids.push(TreeNode::new(format!(
                        "use: {}",
                        theme.paint(theme.data_use, u)
                    )));
                }
                let cats = strs(d.get("data_categories"));
                if !cats.is_empty() {
                    kids.push(TreeNode::new(format!(
                        "categories: {}",
                        theme.paint(theme.category, &cats.join(", "))
                    )));
                }
                let subs = strs(d.get("data_subjects"));
                if !subs.is_empty() {
                    kids.push(TreeNode::new(format!(
                        "subjects: {}",
                        theme.paint(theme.subject, &subs.join(", "))
                    )));
                }
                for f in ["ingress", "egress", "dataset_references"] {
                    let v = strs(d.get(f));
                    if !v.is_empty() {
                        kids.push(TreeNode::new(format!("{f}: {}", v.join(", "))));
                    }
                }
                children.push(TreeNode::new(name.to_string()).with_children(kids));
            }
        }
        "policy" => {
            for rule in r
                .value
                .get("rules")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                let name = rule.get("name").and_then(Value::as_str).unwrap_or("(rule)");
                let mut kids = Vec::new();
                for (f, style) in [
                    ("data_categories", theme.category),
                    ("data_uses", theme.data_use),
                    ("data_subjects", theme.subject),
                ] {
                    if let Some(pr) = rule.get(f) {
                        let m = pr.get("matches").and_then(Value::as_str).unwrap_or("?");
                        let vals = strs(pr.get("values"));
                        kids.push(TreeNode::new(format!(
                            "{f} {} {}",
                            theme.paint(theme.dim, m),
                            theme.paint(style, &vals.join(", "))
                        )));
                    }
                }
                children.push(TreeNode::new(name.to_string()).with_children(kids));
            }
        }
        "data_category" | "data_use" | "data_subject" => {
            if let Some(p) = r.value.get("parent_key").and_then(Value::as_str) {
                children.push(TreeNode::new(format!("parent: {p}")));
            }
        }
        _ => {}
    }
    TreeNode::new(label).with_children(children)
}

/// Whole manifest set as a forest, one root per resource type.
pub fn manifest_tree(theme: &Theme, m: &Manifest) -> Vec<TreeNode> {
    let mut types: Vec<&str> = m.resource_types().collect();
    types.sort_by_key(|t| crate::manifest::type_order(t));
    types
        .into_iter()
        .map(|t| {
            let items = m.of_type(t);
            let label = format!(
                "{} {}",
                theme.paint(theme.heading, t),
                theme.paint(theme.dim, &format!("({})", items.len()))
            );
            TreeNode::new(label)
                .with_children(items.iter().map(|r| resource_node(theme, r)).collect())
        })
        .collect()
}

pub fn run(ctx: &mut Ctx, a: Args) -> Result<Exit> {
    let m = load_manifests(a.paths, a.from, &a.filter)?;
    match a.format {
        CatFormat::Yaml => format::write_value(&mut ctx.out, &m.to_value(), Format::Yaml)?,
        CatFormat::Json => format::write_value(&mut ctx.out, &m.to_value(), Format::Json)?,
        CatFormat::Tree => {
            let forest = manifest_tree(&ctx.theme, &m);
            crate::render::tree::render(
                &mut ctx.out,
                &forest,
                if a.ascii {
                    Guides::Ascii
                } else {
                    Guides::Unicode
                },
            )?;
        }
    }
    if a.format != CatFormat::Tree {
        let summary = m
            .counts()
            .iter()
            .map(|(t, n)| format!("{t} {n}"))
            .collect::<Vec<_>>()
            .join(", ");
        ctx.note(format!(
            "{} files, {} resources: {summary}",
            m.files().len(),
            m.len()
        ));
    }
    ctx.out.flush()?;
    Ok(Exit::Ok)
}

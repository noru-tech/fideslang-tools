//! `fl taxonomy …`

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::{Value, json};

use super::{Ctx, KindSel, parse_kind, parse_kind_sel};
use crate::Exit;
use crate::format::{self, Format};
use crate::graph::{EdgeKind, Graph, NodeKind};
use crate::render::dot::RankDir;
use crate::render::table::Table;
use crate::render::tree::{Guides, TreeNode};
use crate::render::{dot, mermaid, tree};
use crate::taxonomy::search::Matcher;
use crate::taxonomy::{Kind, Snapshot, Taxonomy, TaxonomyRecord, diff, embedded, search};

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List keys of one kind (or `all`) as a table, plain keys, JSON, YAML or CSV.
    List(ListArgs),
    /// Print the vendored taxonomy file for one kind, as YAML (byte-exact), JSON or CSV.
    Cat(CatArgs),
    /// Draw the hierarchy as a terminal tree, Graphviz DOT, Mermaid or nested JSON.
    Tree(TreeArgs),
    /// Show one key with its ancestors, children and metadata.
    Show(ShowArgs),
    /// Search keys, names and descriptions.
    Search(SearchArgs),
    /// Compare two snapshots, or a snapshot against a manifest's custom taxonomy.
    Diff(DiffArgs),
    /// Print snapshot provenance (upstream, tag, commit, date, counts).
    Info(InfoArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ListFormat {
    Table,
    Plain,
    Json,
    Yaml,
    Csv,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// `categories`, `uses`, `subjects` or `all`.
    #[arg(value_parser = parse_kind_sel, default_value = "all")]
    pub kind: KindSel,
    #[arg(long, value_enum, default_value = "table")]
    pub format: ListFormat,
    /// Only keys under this prefix (inclusive), e.g. `user.contact`.
    #[arg(long, value_name = "KEY")]
    pub prefix: Option<String>,
    /// Only deprecated keys.
    #[arg(long)]
    pub deprecated: bool,
}

#[derive(Debug, Args)]
pub struct CatArgs {
    #[arg(value_parser = parse_kind)]
    pub kind: Kind,
    #[arg(long, value_enum, default_value = "yaml")]
    pub format: Format,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TreeFormat {
    Tree,
    Dot,
    Mermaid,
    Json,
}

#[derive(Debug, Args)]
pub struct TreeArgs {
    /// `categories`, `uses`, `subjects` or `all`.
    #[arg(value_parser = parse_kind_sel, default_value = "all")]
    pub kind: KindSel,
    /// Maximum depth to show (1 = top-level keys only).
    #[arg(short, long, value_name = "N")]
    pub depth: Option<usize>,
    /// Start at this key instead of the top level.
    #[arg(long, value_name = "KEY")]
    pub root: Option<String>,
    /// Show descriptions next to names.
    #[arg(long)]
    pub descriptions: bool,
    /// Show full dotted keys instead of the last segment.
    #[arg(long)]
    pub full_keys: bool,
    /// Use ASCII guides instead of box-drawing characters.
    #[arg(long)]
    pub ascii: bool,
    #[arg(long, value_enum, default_value = "tree")]
    pub format: TreeFormat,
    /// Graph direction (dot/mermaid only).
    #[arg(long, value_enum, default_value = "lr")]
    pub rankdir: RankDir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TextFormat {
    Text,
    Json,
    Yaml,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    pub key: String,
    #[arg(long, value_enum, default_value = "text")]
    pub format: TextFormat,
    /// List every descendant, not just direct children.
    #[arg(long)]
    pub all_descendants: bool,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    pub pattern: String,
    /// Restrict to a kind (repeatable).
    #[arg(long = "kind", value_parser = parse_kind, value_name = "KIND")]
    pub kinds: Vec<Kind>,
    /// Treat PATTERN as a regular expression.
    #[arg(short = 'e', long)]
    pub regex: bool,
    /// Match only against keys, not names or descriptions.
    #[arg(long)]
    pub keys_only: bool,
    #[arg(short = 's', long)]
    pub case_sensitive: bool,
    #[arg(long, value_enum, default_value = "text")]
    pub format: TextFormat,
}

#[derive(Debug, Args)]
pub struct DiffArgs {
    /// Baseline snapshot.
    #[arg(long, value_enum, default_value = "iab")]
    pub from: Snapshot,
    /// Target snapshot (ignored with --against).
    #[arg(long, value_enum, default_value = "ethyca")]
    pub to: Snapshot,
    /// Compare --taxonomy against the custom taxonomy declared in these manifest paths.
    #[arg(long, value_name = "PATH", num_args = 1.., conflicts_with_all = ["from", "to"])]
    pub against: Vec<PathBuf>,
    /// Restrict to a kind (repeatable).
    #[arg(long = "kind", value_parser = parse_kind, value_name = "KIND")]
    pub kinds: Vec<Kind>,
    #[arg(long, value_enum, default_value = "text")]
    pub format: TextFormat,
    /// Exit with status 1 when there are differences.
    #[arg(long)]
    pub exit_code: bool,
}

#[derive(Debug, Args)]
pub struct InfoArgs {
    #[arg(long, value_enum, default_value = "text")]
    pub format: TextFormat,
    /// Show every bundled snapshot, not just the selected one.
    #[arg(long)]
    pub all: bool,
}

pub fn run(ctx: &mut Ctx, cmd: Cmd) -> Result<Exit> {
    match cmd {
        Cmd::List(a) => list(ctx, a),
        Cmd::Cat(a) => cat(ctx, a),
        Cmd::Tree(a) => tree_cmd(ctx, a),
        Cmd::Show(a) => show(ctx, a),
        Cmd::Search(a) => search_cmd(ctx, a),
        Cmd::Diff(a) => diff_cmd(ctx, a),
        Cmd::Info(a) => info(ctx, a),
    }
}

fn record_json(kind: Kind, r: &TaxonomyRecord) -> Value {
    let mut v = serde_json::to_value(r).expect("record serializes");
    if let Some(m) = v.as_object_mut() {
        m.insert("kind".into(), json!(kind.resource_type()));
    }
    v
}

fn list(ctx: &mut Ctx, a: ListArgs) -> Result<Exit> {
    let tax = ctx.tax;
    let kinds = a.kind.kinds();
    let selected: Vec<(Kind, &TaxonomyRecord)> = kinds
        .iter()
        .flat_map(|&k| tax.table(k).records().map(move |r| (k, r)))
        .filter(|(k, r)| {
            a.prefix
                .as_deref()
                .is_none_or(|p| tax.table(*k).hierarchy().is_within(&r.fides_key, p))
        })
        .filter(|(_, r)| !a.deprecated || r.is_deprecated())
        .collect();

    match a.format {
        ListFormat::Table => {
            let multi = kinds.len() > 1;
            let mut t = if multi {
                Table::new(&["kind", "key", "name", "added", "deprecated"])
            } else {
                Table::new(&["key", "name", "added", "deprecated"])
            };
            t = t
                .style_column(if multi { 1 } else { 0 }, ctx.theme.key)
                .flexible_column(if multi { 2 } else { 1 });
            for (k, r) in &selected {
                let mut row = Vec::new();
                if multi {
                    row.push(k.resource_type().to_string());
                }
                row.push(r.fides_key.clone());
                row.push(r.name.clone().unwrap_or_default());
                row.push(r.version_added.clone().unwrap_or_default());
                row.push(r.version_deprecated.clone().unwrap_or_default());
                t.push(row);
            }
            t.render(&mut ctx.out, None)?;
            ctx.note(format!("{} keys ({})", selected.len(), tax.label()));
        }
        ListFormat::Plain => {
            for (_, r) in &selected {
                writeln!(ctx.out, "{}", r.fides_key)?;
            }
        }
        ListFormat::Json => {
            let items: Vec<Value> = selected.iter().map(|(k, r)| record_json(*k, r)).collect();
            format::write_value(&mut ctx.out, &Value::Array(items), Format::Json)?;
        }
        ListFormat::Yaml | ListFormat::Csv => {
            // Manifest shape so the output is itself a valid Fides document.
            let mut doc = serde_json::Map::new();
            for k in &kinds {
                let items: Vec<Value> = selected
                    .iter()
                    .filter(|(kk, _)| kk == k)
                    .map(|(_, r)| serde_json::to_value(r).unwrap())
                    .collect();
                doc.insert(k.resource_type().into(), Value::Array(items));
            }
            let fmt = if a.format == ListFormat::Yaml {
                Format::Yaml
            } else {
                Format::Csv
            };
            if fmt == Format::Csv && kinds.len() > 1 {
                bail!("CSV output needs a single kind (categories, uses or subjects)");
            }
            format::write_value(&mut ctx.out, &Value::Object(doc), fmt)?;
        }
    }
    Ok(Exit::Ok)
}

fn cat(ctx: &mut Ctx, a: CatArgs) -> Result<Exit> {
    let raw = embedded::raw(ctx.global.taxonomy, a.kind);
    match a.format {
        Format::Yaml => ctx.out.write_all(raw.as_bytes())?,
        other => {
            let v = format::parse_yaml(raw)?;
            format::write_value(&mut ctx.out, &v, other)?;
        }
    }
    Ok(Exit::Ok)
}

/// Build the display tree for one kind.
fn kind_tree(ctx: &Ctx, tax: &Taxonomy, kind: Kind, a: &TreeArgs) -> Result<(TreeNode, usize)> {
    let table = tax.table(kind);
    let h = table.hierarchy();
    let style = ctx.theme.for_kind(kind);
    let label = |key: &str| -> String {
        let r = table.get(key).expect("key in table");
        let shown = if a.full_keys { key } else { r.leaf() };
        let mut s = ctx.theme.paint(style.bold(), shown);
        if let Some(name) = &r.name {
            s.push_str("  ");
            s.push_str(name);
        }
        if r.is_deprecated() {
            s.push_str(&ctx.theme.paint(ctx.theme.warning, "  (deprecated)"));
        }
        if a.descriptions
            && let Some(d) = &r.description
        {
            s.push_str(&ctx.theme.paint(ctx.theme.dim, &format!("  — {d}")));
        }
        s
    };
    fn build(
        h: &crate::taxonomy::Hierarchy,
        key: &str,
        depth_left: Option<usize>,
        label: &dyn Fn(&str) -> String,
        count: &mut usize,
    ) -> TreeNode {
        *count += 1;
        let mut node = TreeNode::new(label(key));
        if depth_left != Some(0) {
            node.children = h
                .children(key)
                .iter()
                .map(|c| build(h, c, depth_left.map(|d| d - 1), label, count))
                .collect();
        }
        node
    }
    let mut count = 0;
    let (root_label, roots): (String, Vec<&str>) = match &a.root {
        Some(r) => {
            if !table.contains(r) {
                bail!("`{r}` is not a {} in {}", kind.human(), tax.label());
            }
            (
                format!(
                    "{} ({})",
                    ctx.theme.paint(ctx.theme.heading, kind.resource_type()),
                    tax.label()
                ),
                vec![r.as_str()],
            )
        }
        None => (
            format!(
                "{} ({})",
                ctx.theme.paint(ctx.theme.heading, kind.resource_type()),
                tax.label()
            ),
            h.roots().iter().map(String::as_str).collect(),
        ),
    };
    let depth = a.depth.map(|d| d.max(1));
    let children = roots
        .iter()
        .map(|r| build(h, r, depth.map(|d| d - 1), &label, &mut count))
        .collect();
    Ok((TreeNode::new(root_label).with_children(children), count))
}

fn kind_graph(g: &mut Graph, tax: &Taxonomy, kind: Kind, a: &TreeArgs) {
    let table = tax.table(kind);
    let h = table.hierarchy();
    let nk = NodeKind::from_taxonomy(kind);
    let roots: Vec<String> = match &a.root {
        Some(r) => vec![r.clone()],
        None => h.roots().to_vec(),
    };
    let depth = a.depth.map(|d| d.max(1));
    fn add(
        g: &mut Graph,
        h: &crate::taxonomy::Hierarchy,
        table: &crate::taxonomy::KindTable,
        nk: NodeKind,
        key: &str,
        depth_left: Option<usize>,
    ) {
        let r = table.get(key).expect("key");
        g.node(key, key, nk).detail = r.name.clone();
        if depth_left == Some(0) {
            return;
        }
        for c in h.children(key) {
            add(g, h, table, nk, c, depth_left.map(|d| d - 1));
            g.edge(key, c, EdgeKind::Child, None);
        }
    }
    for r in roots {
        if table.contains(&r) {
            add(g, h, table, nk, &r, depth.map(|d| d - 1));
        }
    }
}

fn kind_json(tax: &Taxonomy, kind: Kind, a: &TreeArgs) -> Value {
    let table = tax.table(kind);
    let h = table.hierarchy();
    fn build(
        h: &crate::taxonomy::Hierarchy,
        table: &crate::taxonomy::KindTable,
        key: &str,
        depth_left: Option<usize>,
    ) -> Value {
        let r = table.get(key).expect("key");
        let children: Vec<Value> = if depth_left == Some(0) {
            Vec::new()
        } else {
            h.children(key)
                .iter()
                .map(|c| build(h, table, c, depth_left.map(|d| d - 1)))
                .collect()
        };
        json!({
            "fides_key": r.fides_key,
            "name": r.name,
            "description": r.description,
            "deprecated": r.is_deprecated(),
            "children": children,
        })
    }
    let roots: Vec<String> = match &a.root {
        Some(r) => vec![r.clone()],
        None => h.roots().to_vec(),
    };
    let depth = a.depth.map(|d| d.max(1));
    let items: Vec<Value> = roots
        .iter()
        .filter(|r| table.contains(r))
        .map(|r| build(h, table, r, depth.map(|d| d - 1)))
        .collect();
    json!({ "kind": kind.resource_type(), "taxonomy": tax.label(), "roots": items })
}

fn tree_cmd(ctx: &mut Ctx, a: TreeArgs) -> Result<Exit> {
    let tax = ctx.tax;
    let kinds = a.kind.kinds();
    if let Some(r) = &a.root
        && !kinds.iter().any(|k| tax.contains(*k, r))
    {
        bail!("`{r}` is not a known key in {}", tax.label());
    }
    match a.format {
        TreeFormat::Tree => {
            let guides = if a.ascii {
                Guides::Ascii
            } else {
                Guides::Unicode
            };
            let mut total = 0;
            for &k in &kinds {
                if a.root.as_deref().is_some_and(|r| !tax.contains(k, r)) {
                    continue;
                }
                let (node, n) = kind_tree(ctx, tax, k, &a)?;
                total += n;
                tree::render(&mut ctx.out, &[node], guides)?;
            }
            let depth_note = a
                .depth
                .map(|d| format!(", depth ≤ {d}"))
                .unwrap_or_default();
            ctx.note(format!("{total} keys shown{depth_note} ({})", tax.label()));
        }
        TreeFormat::Dot | TreeFormat::Mermaid => {
            let mut g = Graph::new(format!("fideslang {}", tax.label()));
            for &k in &kinds {
                kind_graph(&mut g, tax, k, &a);
            }
            let text = if a.format == TreeFormat::Dot {
                dot::render(&g, a.rankdir)
            } else {
                mermaid::render(&g, a.rankdir)
            };
            ctx.out.write_all(text.as_bytes())?;
        }
        TreeFormat::Json => {
            let items: Vec<Value> = kinds.iter().map(|&k| kind_json(tax, k, &a)).collect();
            let v = if items.len() == 1 {
                items.into_iter().next().unwrap()
            } else {
                Value::Array(items)
            };
            format::write_value(&mut ctx.out, &v, Format::Json)?;
        }
    }
    Ok(Exit::Ok)
}

fn show(ctx: &mut Ctx, a: ShowArgs) -> Result<Exit> {
    let tax = ctx.tax;
    let hits = tax.lookup(&a.key);
    if hits.is_empty() {
        let mut msg = format!("`{}` is not a key in {}", a.key, tax.label());
        let suggestions: Vec<String> = Kind::ALL
            .into_iter()
            .flat_map(|k| crate::validate::suggest::suggest(&a.key, tax.table(k), 2))
            .collect();
        if let Some(s) = crate::validate::suggest::format(&suggestions) {
            msg.push_str(&format!("; {s}"));
        }
        bail!(msg);
    }
    for (kind, r) in hits {
        let h = tax.table(kind).hierarchy();
        let ancestors = h.ancestors(&r.fides_key);
        let children: Vec<String> = if a.all_descendants {
            h.descendants(&r.fides_key)
        } else {
            h.children(&r.fides_key).to_vec()
        };
        match a.format {
            TextFormat::Text => {
                let t = &ctx.theme;
                let style = t.for_kind(kind).bold();
                let mut head = t.paint(style, &r.fides_key);
                if let Some(n) = &r.name {
                    head.push_str(&format!("  —  {n}"));
                }
                let mut meta = vec![kind.resource_type().to_string(), tax.label()];
                if let Some(v) = &r.version_added {
                    meta.push(format!("added {v}"));
                }
                if let Some(v) = &r.version_deprecated {
                    meta.push(t.paint(t.warning, &format!("deprecated {v}")));
                }
                writeln!(
                    ctx.out,
                    "{head}  {}",
                    t.paint(t.dim, &format!("[{}]", meta.join(" · ")))
                )?;
                if let Some(d) = &r.description {
                    writeln!(ctx.out, "  {d}")?;
                }
                if let Some(rb) = &r.replaced_by {
                    writeln!(ctx.out, "  {} {}", t.paint(t.warning, "replaced by:"), rb)?;
                }
                let chain = if ancestors.is_empty() {
                    "(top level)".to_string()
                } else {
                    ancestors.join(" › ") + " › " + &r.fides_key
                };
                writeln!(ctx.out, "{} {chain}", t.paint(t.dim, "ancestors:  "))?;
                let label = if a.all_descendants {
                    "descendants:"
                } else {
                    "children:   "
                };
                if children.is_empty() {
                    writeln!(ctx.out, "{} (none)", t.paint(t.dim, label))?;
                } else {
                    writeln!(ctx.out, "{} {}", t.paint(t.dim, label), children.len())?;
                    for c in &children {
                        let cr = tax.get(kind, c).expect("child exists");
                        writeln!(
                            ctx.out,
                            "  {}  {}",
                            t.paint(t.for_kind(kind), c),
                            t.paint(t.dim, cr.name.as_deref().unwrap_or(""))
                        )?;
                    }
                }
                if let Some(rights) = &r.rights {
                    writeln!(
                        ctx.out,
                        "{} {}",
                        t.paint(t.dim, "rights:     "),
                        serde_json::to_string(rights)?
                    )?;
                }
                if let Some(adp) = r.automated_decisions_or_profiling {
                    writeln!(
                        ctx.out,
                        "{} {adp}",
                        t.paint(t.dim, "automated decisions or profiling:")
                    )?;
                }
            }
            TextFormat::Json | TextFormat::Yaml => {
                let mut v = record_json(kind, r);
                let m = v.as_object_mut().unwrap();
                m.insert("taxonomy".into(), json!(tax.label()));
                m.insert("ancestors".into(), json!(ancestors));
                m.insert(
                    if a.all_descendants {
                        "descendants"
                    } else {
                        "children"
                    }
                    .into(),
                    json!(children),
                );
                let f = if a.format == TextFormat::Json {
                    Format::Json
                } else {
                    Format::Yaml
                };
                format::write_value(&mut ctx.out, &v, f)?;
            }
        }
    }
    Ok(Exit::Ok)
}

fn highlight(theme: &crate::render::Theme, text: &str, spans: &[(usize, usize)]) -> String {
    let mut out = String::new();
    let mut pos = 0;
    for &(s, e) in spans {
        if s < pos || e > text.len() {
            continue;
        }
        out.push_str(&text[pos..s]);
        out.push_str(&theme.paint(theme.highlight, &text[s..e]));
        pos = e;
    }
    out.push_str(&text[pos..]);
    out
}

fn search_cmd(ctx: &mut Ctx, a: SearchArgs) -> Result<Exit> {
    let tax = ctx.tax;
    let kinds = if a.kinds.is_empty() {
        Kind::ALL.to_vec()
    } else {
        a.kinds.clone()
    };
    let m = Matcher::new(&a.pattern, a.regex, a.case_sensitive)?;
    let hits = search::search(tax, &kinds, &m, a.keys_only);
    match a.format {
        TextFormat::Text => {
            for h in &hits {
                let t = &ctx.theme;
                let key_spans = h
                    .fields
                    .iter()
                    .find(|f| f.field == "fides_key")
                    .map(|f| f.spans.as_slice())
                    .unwrap_or(&[]);
                let key = highlight(t, &h.record.fides_key, key_spans);
                let styled_key = format!("{}{key}{}", t.for_kind(h.kind).bold(), anstyle::Reset);
                let name = h.record.name.as_deref().unwrap_or("");
                let name_spans = h
                    .fields
                    .iter()
                    .find(|f| f.field == "name")
                    .map(|f| f.spans.as_slice())
                    .unwrap_or(&[]);
                writeln!(
                    ctx.out,
                    "{}  {styled_key}  {}",
                    t.paint(t.dim, kind_short(h.kind)),
                    highlight(t, name, name_spans)
                )?;
                if let Some(f) = h.fields.iter().find(|f| f.field == "description") {
                    let d = h.record.description.as_deref().unwrap_or("");
                    writeln!(ctx.out, "      {}", highlight(t, d, &f.spans))?;
                }
            }
            ctx.note(format!("{} matches ({})", hits.len(), tax.label()));
        }
        TextFormat::Json | TextFormat::Yaml => {
            let items: Vec<Value> = hits
                .iter()
                .map(|h| {
                    let mut v = record_json(h.kind, h.record);
                    v.as_object_mut().unwrap().insert(
                        "matched".into(),
                        json!(h.fields.iter().map(|f| f.field).collect::<Vec<_>>()),
                    );
                    v
                })
                .collect();
            let f = if a.format == TextFormat::Json {
                Format::Json
            } else {
                Format::Yaml
            };
            format::write_value(&mut ctx.out, &Value::Array(items), f)?;
        }
    }
    Ok(Exit::Ok)
}

fn kind_short(kind: Kind) -> &'static str {
    match kind {
        Kind::Category => "cat ",
        Kind::Use => "use ",
        Kind::Subject => "subj",
    }
}

fn diff_cmd(ctx: &mut Ctx, a: DiffArgs) -> Result<Exit> {
    let kinds = if a.kinds.is_empty() {
        Kind::ALL.to_vec()
    } else {
        a.kinds.clone()
    };
    let extended;
    let (from, to): (&Taxonomy, &Taxonomy) = if a.against.is_empty() {
        (embedded::load(a.from), embedded::load(a.to))
    } else {
        let m = crate::manifest::load::load(&a.against, None)
            .context("loading manifests for --against")?;
        let custom = m.custom_taxonomy();
        let mut ext = ctx.tax.extended_with(custom);
        let label = a
            .against
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        ext = {
            let mut t = ext.clone();
            let mut prov = t.provenance().clone();
            prov.snapshot = format!("{} + {label}", ctx.tax.provenance().snapshot);
            t = Taxonomy::new(prov);
            for k in Kind::ALL {
                t.set_kind(k, ext.table(k).clone());
            }
            t
        };
        extended = ext;
        (ctx.tax, &extended)
    };
    let d = diff::diff(from, to, &kinds);
    match a.format {
        TextFormat::Text => {
            let t = &ctx.theme;
            writeln!(
                ctx.out,
                "{} → {}",
                t.paint(t.heading, &d.from),
                t.paint(t.heading, &d.to)
            )?;
            for k in &d.kinds {
                if k.is_empty() {
                    writeln!(
                        ctx.out,
                        "{}: no changes ({})",
                        k.kind.resource_type(),
                        k.to_count
                    )?;
                    continue;
                }
                writeln!(
                    ctx.out,
                    "{}: {} → {}",
                    k.kind.resource_type(),
                    k.from_count,
                    k.to_count
                )?;
                for r in &k.added {
                    let parent = r
                        .parent_key
                        .as_deref()
                        .map(|p| format!("  (parent {p})"))
                        .unwrap_or_default();
                    writeln!(
                        ctx.out,
                        "  {} {}  {}{}",
                        t.paint(t.added, "+"),
                        t.paint(t.added, &r.fides_key),
                        r.name.as_deref().unwrap_or(""),
                        t.paint(t.dim, &parent)
                    )?;
                }
                for r in &k.removed {
                    writeln!(
                        ctx.out,
                        "  {} {}  {}",
                        t.paint(t.removed, "-"),
                        t.paint(t.removed, &r.fides_key),
                        r.name.as_deref().unwrap_or("")
                    )?;
                }
                for c in &k.changed {
                    writeln!(
                        ctx.out,
                        "  {} {}",
                        t.paint(t.warning, "~"),
                        t.paint(t.key, &c.fides_key)
                    )?;
                    for ch in &c.changes {
                        writeln!(
                            ctx.out,
                            "      {}: {} → {}",
                            ch.field,
                            t.paint(t.removed, ch.from.as_deref().unwrap_or("null")),
                            t.paint(t.added, ch.to.as_deref().unwrap_or("null"))
                        )?;
                    }
                }
            }
            let (added, removed, changed) = d.totals();
            writeln!(
                ctx.out,
                "{added} added, {removed} removed, {changed} changed"
            )?;
        }
        TextFormat::Json | TextFormat::Yaml => {
            let v = serde_json::to_value(&d)?;
            let f = if a.format == TextFormat::Json {
                Format::Json
            } else {
                Format::Yaml
            };
            format::write_value(&mut ctx.out, &v, f)?;
        }
    }
    Ok(if a.exit_code && !d.is_empty() {
        Exit::Findings
    } else {
        Exit::Ok
    })
}

fn info(ctx: &mut Ctx, a: InfoArgs) -> Result<Exit> {
    let snapshots: Vec<Snapshot> = if a.all {
        Snapshot::ALL.to_vec()
    } else {
        vec![ctx.global.taxonomy]
    };
    match a.format {
        TextFormat::Text => {
            for s in snapshots {
                let tax = embedded::load(s);
                let p = tax.provenance();
                let t = &ctx.theme;
                let default = if s == Snapshot::default() {
                    t.paint(t.dim, "  (default)")
                } else {
                    String::new()
                };
                writeln!(
                    ctx.out,
                    "{}{default}",
                    t.paint(t.heading, &format!("{} {}", p.snapshot, p.tag))
                )?;
                writeln!(ctx.out, "  upstream:  {}", p.upstream)?;
                writeln!(ctx.out, "  commit:    {}", p.commit)?;
                writeln!(ctx.out, "  snapshot:  {}", p.snapshot_date)?;
                writeln!(ctx.out, "  license:   {}", p.license)?;
                let counts = Kind::ALL
                    .iter()
                    .map(|k| format!("{} {}", tax.table(*k).len(), k.label()))
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(ctx.out, "  contents:  {counts}")?;
            }
        }
        TextFormat::Json | TextFormat::Yaml => {
            let items: Vec<Value> = snapshots
                .iter()
                .map(|s| serde_json::to_value(embedded::load(*s).provenance()).unwrap())
                .collect();
            let v = if items.len() == 1 {
                items.into_iter().next().unwrap()
            } else {
                Value::Array(items)
            };
            let f = if a.format == TextFormat::Json {
                Format::Json
            } else {
                Format::Yaml
            };
            format::write_value(&mut ctx.out, &v, f)?;
        }
    }
    Ok(Exit::Ok)
}

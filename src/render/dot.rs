//! Graphviz DOT output.

use std::fmt::Write as _;

use crate::graph::{EdgeKind, Graph, NodeKind, ident};

/// `--rankdir` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum RankDir {
    #[default]
    LR,
    TB,
    RL,
    BT,
}

impl RankDir {
    pub fn as_str(self) -> &'static str {
        match self {
            RankDir::LR => "LR",
            RankDir::TB => "TB",
            RankDir::RL => "RL",
            RankDir::BT => "BT",
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn node_attrs(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::System => "shape=box, style=\"rounded,filled\", fillcolor=\"#dbeafe\"",
        NodeKind::Dataset => "shape=cylinder, style=filled, fillcolor=\"#dcfce7\"",
        NodeKind::Collection => "shape=box, style=filled, fillcolor=\"#f0fdf4\"",
        NodeKind::Field => "shape=plaintext",
        NodeKind::Organization => "shape=house, style=filled, fillcolor=\"#f3e8ff\"",
        NodeKind::Policy => "shape=note, style=filled, fillcolor=\"#fee2e2\"",
        NodeKind::DataCategory => "shape=box, style=\"rounded,filled\", fillcolor=\"#cffafe\"",
        NodeKind::DataUse => "shape=ellipse, style=filled, fillcolor=\"#fef9c3\"",
        NodeKind::DataSubject => "shape=hexagon, style=filled, fillcolor=\"#e0e7ff\"",
        NodeKind::External => "shape=box, style=dashed",
    }
}

fn edge_attrs(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::Child | EdgeKind::Contains => "arrowhead=none",
        EdgeKind::Ingress | EdgeKind::Egress => "penwidth=1.5",
        EdgeKind::References => "style=dashed",
        EdgeKind::Uses => "style=dotted, color=\"#a16207\"",
        EdgeKind::Categorizes => "style=dotted, color=\"#0e7490\"",
        EdgeKind::Concerns => "style=dotted, color=\"#4338ca\"",
    }
}

/// Render the graph as a `digraph`.
pub fn render(g: &Graph, rankdir: RankDir) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "digraph \"{}\" {{", escape(&g.title));
    let _ = writeln!(out, "  rankdir={};", rankdir.as_str());
    let _ = writeln!(
        out,
        "  node [fontname=\"Helvetica,Arial,sans-serif\", fontsize=10];"
    );
    let _ = writeln!(
        out,
        "  edge [fontname=\"Helvetica,Arial,sans-serif\", fontsize=8, color=\"#64748b\"];"
    );
    for n in g.nodes() {
        let label = match &n.detail {
            Some(d) => format!("{}\\n{}", escape(&n.label), escape(d)),
            None => escape(&n.label),
        };
        let _ = writeln!(
            out,
            "  {} [label=\"{}\", {}];",
            ident(&n.id),
            label,
            node_attrs(n.kind)
        );
    }
    for e in g.edges() {
        let mut attrs = vec![edge_attrs(e.kind).to_string()];
        let label = e
            .label
            .clone()
            .unwrap_or_else(|| e.kind.label().to_string());
        if !label.is_empty() {
            attrs.push(format!("label=\"{}\"", escape(&label)));
        }
        let _ = writeln!(
            out,
            "  {} -> {} [{}];",
            ident(&e.from),
            ident(&e.to),
            attrs.join(", ")
        );
    }
    out.push_str("}\n");
    out
}

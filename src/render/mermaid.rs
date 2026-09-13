//! Mermaid `flowchart` output (pastes straight into GitHub markdown).

use std::fmt::Write as _;

use crate::graph::{EdgeKind, Graph, NodeKind, ident};
use crate::render::dot::RankDir;

fn escape(s: &str) -> String {
    // Mermaid labels are safest inside double quotes; `"` must become the HTML entity.
    s.replace('"', "#quot;").replace('\n', "<br/>")
}

fn shape(kind: NodeKind, label: &str) -> String {
    let l = escape(label);
    match kind {
        NodeKind::System => format!("[\"{l}\"]"),
        NodeKind::Dataset => format!("[(\"{l}\")]"),
        NodeKind::Collection => format!("[[\"{l}\"]]"),
        NodeKind::Field => format!("(\"{l}\")"),
        NodeKind::Organization => format!("[/\"{l}\"/]"),
        NodeKind::Policy => format!(">\"{l}\"]"),
        NodeKind::DataCategory => format!("([\"{l}\"])"),
        NodeKind::DataUse => format!("{{{{\"{l}\"}}}}"),
        NodeKind::DataSubject => format!("{{\"{l}\"}}"),
        NodeKind::External => format!("[\"{l}\"]"),
    }
}

fn arrow(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::Child | EdgeKind::Contains => "---",
        EdgeKind::Ingress | EdgeKind::Egress => "-->",
        EdgeKind::References => "-.->",
        EdgeKind::Uses | EdgeKind::Categorizes | EdgeKind::Concerns => "-.->",
    }
}

pub fn render(g: &Graph, rankdir: RankDir) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "flowchart {}", rankdir.as_str());
    let _ = writeln!(out, "  %% {}", g.title);
    let classes = [
        ("system", "fill:#dbeafe,stroke:#1d4ed8"),
        ("dataset", "fill:#dcfce7,stroke:#15803d"),
        ("collection", "fill:#f0fdf4,stroke:#15803d"),
        ("field", "fill:#ffffff,stroke:#94a3b8"),
        ("organization", "fill:#f3e8ff,stroke:#7e22ce"),
        ("policy", "fill:#fee2e2,stroke:#b91c1c"),
        ("category", "fill:#cffafe,stroke:#0e7490"),
        ("use", "fill:#fef9c3,stroke:#a16207"),
        ("subject", "fill:#e0e7ff,stroke:#4338ca"),
        (
            "external",
            "fill:#f8fafc,stroke:#64748b,stroke-dasharray: 4 2",
        ),
    ];
    for (name, style) in classes {
        let _ = writeln!(out, "  classDef {name} {style}");
    }
    for n in g.nodes() {
        let label = match &n.detail {
            Some(d) => format!("{}\n{}", n.label, d),
            None => n.label.clone(),
        };
        let _ = writeln!(
            out,
            "  {}{}:::{}",
            ident(&n.id),
            shape(n.kind, &label),
            n.kind.css_class()
        );
    }
    for e in g.edges() {
        let label = e
            .label
            .clone()
            .unwrap_or_else(|| e.kind.label().to_string());
        if label.is_empty() {
            let _ = writeln!(
                out,
                "  {} {} {}",
                ident(&e.from),
                arrow(e.kind),
                ident(&e.to)
            );
        } else {
            let _ = writeln!(
                out,
                "  {} {}|\"{}\"| {}",
                ident(&e.from),
                arrow(e.kind),
                escape(&label),
                ident(&e.to)
            );
        }
    }
    out
}

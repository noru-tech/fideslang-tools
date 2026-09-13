//! A small directed graph model shared by the taxonomy tree and manifest relationship renderers.

pub mod manifest;

use std::collections::{HashMap, HashSet, VecDeque};

use indexmap::IndexMap;
use serde::Serialize;

use crate::taxonomy::Kind;

/// What a node represents; drives shape and color in DOT/Mermaid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    System,
    Dataset,
    Collection,
    Field,
    Organization,
    Policy,
    DataCategory,
    DataUse,
    DataSubject,
    /// `user` / other external endpoints in ingress/egress.
    External,
}

impl NodeKind {
    pub fn from_taxonomy(kind: Kind) -> Self {
        match kind {
            Kind::Category => NodeKind::DataCategory,
            Kind::Use => NodeKind::DataUse,
            Kind::Subject => NodeKind::DataSubject,
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            NodeKind::System => "system",
            NodeKind::Dataset => "dataset",
            NodeKind::Collection => "collection",
            NodeKind::Field => "field",
            NodeKind::Organization => "organization",
            NodeKind::Policy => "policy",
            NodeKind::DataCategory => "category",
            NodeKind::DataUse => "use",
            NodeKind::DataSubject => "subject",
            NodeKind::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// Taxonomy parent → child.
    Child,
    /// Dataset/system → system (data flows in).
    Ingress,
    /// System → dataset/system (data flows out).
    Egress,
    /// System → dataset (`dataset_references`).
    References,
    /// Privacy declaration → data use.
    Uses,
    /// Declaration / field → data category.
    Categorizes,
    /// Declaration → data subject.
    Concerns,
    /// Dataset → collection, collection → field, field → nested field.
    Contains,
}

impl EdgeKind {
    pub fn label(self) -> &'static str {
        match self {
            EdgeKind::Child => "",
            EdgeKind::Ingress => "ingress",
            EdgeKind::Egress => "egress",
            EdgeKind::References => "references",
            EdgeKind::Uses => "data_use",
            EdgeKind::Categorizes => "data_categories",
            EdgeKind::Concerns => "data_subjects",
            EdgeKind::Contains => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    /// Optional secondary line (name, description).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
    /// Optional edge label (e.g. the categories flowing along an ingress).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Graph {
    pub title: String,
    #[serde(serialize_with = "serialize_nodes")]
    nodes: IndexMap<String, Node>,
    edges: Vec<Edge>,
    #[serde(skip)]
    edge_set: HashSet<(String, String, EdgeKind)>,
}

impl Graph {
    pub fn new(title: impl Into<String>) -> Self {
        Graph {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Add (or keep) a node. The first definition of an id wins.
    pub fn node(&mut self, id: &str, label: &str, kind: NodeKind) -> &mut Node {
        self.nodes.entry(id.to_string()).or_insert_with(|| Node {
            id: id.to_string(),
            label: label.to_string(),
            kind,
            detail: None,
        })
    }

    /// Add an edge, deduplicated by (from, to, kind); the first label wins.
    pub fn edge(&mut self, from: &str, to: &str, kind: EdgeKind, label: Option<String>) {
        if self
            .edge_set
            .insert((from.to_string(), to.to_string(), kind))
        {
            self.edges.push(Edge {
                from: from.to_string(),
                to: to.to_string(),
                kind,
                label,
            });
        }
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn get(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Keep only nodes within `depth` undirected hops of `focus` (and the edges between them).
    pub fn focus(&self, focus: &str, depth: usize) -> Graph {
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for e in &self.edges {
            adj.entry(&e.from).or_default().push(&e.to);
            adj.entry(&e.to).or_default().push(&e.from);
        }
        let mut keep: HashSet<&str> = HashSet::new();
        let mut queue = VecDeque::from([(focus, 0usize)]);
        while let Some((id, d)) = queue.pop_front() {
            if !keep.insert(id) || d == depth {
                continue;
            }
            for &n in adj.get(id).map(Vec::as_slice).unwrap_or(&[]) {
                if !keep.contains(n) {
                    queue.push_back((n, d + 1));
                }
            }
        }
        let mut g = Graph::new(self.title.clone());
        for n in self.nodes.values().filter(|n| keep.contains(n.id.as_str())) {
            g.nodes.insert(n.id.clone(), n.clone());
        }
        for e in self
            .edges
            .iter()
            .filter(|e| keep.contains(e.from.as_str()) && keep.contains(e.to.as_str()))
        {
            g.edge(&e.from, &e.to, e.kind, e.label.clone());
        }
        g
    }
}

fn serialize_nodes<S: serde::Serializer>(
    nodes: &IndexMap<String, Node>,
    s: S,
) -> Result<S::Ok, S::Error> {
    s.collect_seq(nodes.values())
}

/// Make an arbitrary string safe as a DOT / Mermaid identifier.
pub fn ident(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 1);
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        out.insert(0, 'n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_limits_by_hops() {
        let mut g = Graph::new("t");
        for id in ["a", "b", "c", "d"] {
            g.node(id, id, NodeKind::System);
        }
        g.edge("a", "b", EdgeKind::Egress, None);
        g.edge("b", "c", EdgeKind::Egress, None);
        g.edge("c", "d", EdgeKind::Egress, None);
        let f = g.focus("a", 1);
        assert_eq!(f.node_count(), 2);
        assert_eq!(f.edge_count(), 1);
        let f = g.focus("b", 1);
        assert_eq!(f.node_count(), 3);
        assert_eq!(ident("user.contact-email 1"), "user_contact_email_1");
        assert_eq!(ident("1abc"), "n1abc");
    }
}

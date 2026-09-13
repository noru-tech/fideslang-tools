//! Build a relationship graph from a manifest set: systems, datasets, data flows, and (optionally)
//! the data uses / categories / subjects / collections / fields they touch.

use serde_json::Value;

use super::{EdgeKind, Graph, NodeKind};
use crate::manifest::Manifest;

/// Which optional node families to include.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum Include {
    Uses,
    Categories,
    Subjects,
    Collections,
    Fields,
    Organizations,
    Policies,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub uses: bool,
    pub categories: bool,
    pub subjects: bool,
    pub collections: bool,
    pub fields: bool,
    pub organizations: bool,
    pub policies: bool,
}

impl Options {
    pub fn from_includes(includes: &[Include]) -> Self {
        let mut o = Options::default();
        for i in includes {
            match i {
                Include::Uses => o.uses = true,
                Include::Categories => o.categories = true,
                Include::Subjects => o.subjects = true,
                Include::Collections => o.collections = true,
                Include::Fields => {
                    o.fields = true;
                    o.collections = true;
                }
                Include::Organizations => o.organizations = true,
                Include::Policies => o.policies = true,
            }
        }
        o
    }
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

fn use_node(g: &mut Graph, key: &str) -> String {
    let id = format!("use:{key}");
    g.node(&id, key, NodeKind::DataUse);
    id
}

fn category_node(g: &mut Graph, key: &str) -> String {
    let id = format!("cat:{key}");
    g.node(&id, key, NodeKind::DataCategory);
    id
}

fn subject_node(g: &mut Graph, key: &str) -> String {
    let id = format!("subj:{key}");
    g.node(&id, key, NodeKind::DataSubject);
    id
}

/// Node id for an ingress/egress endpoint: a known dataset or system, else an external box.
fn endpoint(g: &mut Graph, m: &Manifest, key: &str, typ: Option<&str>) -> String {
    let is_dataset = typ == Some("dataset") || (typ.is_none() && m.get("dataset", key).is_some());
    let is_system = typ == Some("system") || (typ.is_none() && m.get("system", key).is_some());
    if is_dataset {
        let id = format!("dataset:{key}");
        g.node(&id, key, NodeKind::Dataset);
        id
    } else if is_system {
        let id = format!("system:{key}");
        g.node(&id, key, NodeKind::System);
        id
    } else {
        let id = format!("ext:{key}");
        g.node(&id, key, NodeKind::External);
        id
    }
}

fn add_fields(g: &mut Graph, parent_id: &str, fields: &[Value], opts: &Options) {
    for f in fields {
        let Some(name) = f.get("name").and_then(Value::as_str) else {
            continue;
        };
        let id = format!("{parent_id}/{name}");
        g.node(&id, name, NodeKind::Field);
        g.edge(parent_id, &id, EdgeKind::Contains, None);
        if opts.categories {
            for c in strs(f.get("data_categories")) {
                let cid = category_node(g, &c);
                g.edge(&id, &cid, EdgeKind::Categorizes, Some(String::new()));
            }
        }
        if let Some(nested) = f.get("fields").and_then(Value::as_array) {
            add_fields(g, &id, nested, opts);
        }
    }
}

/// Build the graph.
pub fn build(m: &Manifest, opts: &Options) -> Graph {
    let mut g = Graph::new("fides manifest");

    for ds in m.of_type("dataset") {
        let Some(key) = ds.fides_key() else { continue };
        let id = format!("dataset:{key}");
        let n = g.node(&id, key, NodeKind::Dataset);
        n.detail = ds.name().map(str::to_string);
        if opts.categories {
            for c in strs(ds.value.get("data_categories")) {
                let cid = category_node(&mut g, &c);
                g.edge(&id, &cid, EdgeKind::Categorizes, Some(String::new()));
            }
        }
        if opts.collections {
            for c in ds
                .value
                .get("collections")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                let Some(cname) = c.get("name").and_then(Value::as_str) else {
                    continue;
                };
                let cid = format!("{id}/{cname}");
                g.node(&cid, cname, NodeKind::Collection);
                g.edge(&id, &cid, EdgeKind::Contains, None);
                if opts.fields {
                    add_fields(
                        &mut g,
                        &cid,
                        c.get("fields")
                            .and_then(Value::as_array)
                            .map(Vec::as_slice)
                            .unwrap_or(&[]),
                        opts,
                    );
                } else if opts.categories {
                    // roll field categories up to the collection
                    let mut cats = Vec::new();
                    collect_field_categories(c.get("fields"), &mut cats);
                    for cat in cats {
                        let catid = category_node(&mut g, &cat);
                        g.edge(&cid, &catid, EdgeKind::Categorizes, Some(String::new()));
                    }
                }
            }
        }
    }

    for sys in m.of_type("system") {
        let Some(key) = sys.fides_key() else { continue };
        let id = format!("system:{key}");
        let n = g.node(&id, key, NodeKind::System);
        n.detail = sys.name().map(str::to_string);

        for r in strs(sys.value.get("dataset_references")) {
            let did = endpoint(&mut g, m, &r, Some("dataset"));
            g.edge(&id, &did, EdgeKind::References, None);
        }
        for (field, kind) in [("ingress", EdgeKind::Ingress), ("egress", EdgeKind::Egress)] {
            for flow in sys
                .value
                .get(field)
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                let (fk, typ, cats) = match flow {
                    Value::String(s) => (s.as_str(), None, Vec::new()),
                    Value::Object(o) => (
                        o.get("fides_key").and_then(Value::as_str).unwrap_or(""),
                        o.get("type").and_then(Value::as_str),
                        strs(o.get("data_categories")),
                    ),
                    _ => continue,
                };
                if fk.is_empty() {
                    continue;
                }
                let eid = endpoint(&mut g, m, fk, typ);
                let label = (!cats.is_empty()).then(|| cats.join(", "));
                match kind {
                    EdgeKind::Ingress => g.edge(&eid, &id, kind, label),
                    _ => g.edge(&id, &eid, kind, label),
                }
            }
        }

        for (i, d) in sys
            .value
            .get("privacy_declarations")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
            .iter()
            .enumerate()
        {
            let cats = strs(d.get("data_categories"));
            let subjects = strs(d.get("data_subjects"));
            let data_use = d.get("data_use").and_then(Value::as_str);
            // declaration-level flows
            for (field, kind) in [("ingress", EdgeKind::Ingress), ("egress", EdgeKind::Egress)] {
                for fk in strs(d.get(field)) {
                    let eid = endpoint(&mut g, m, &fk, None);
                    let label = (!cats.is_empty()).then(|| cats.join(", "));
                    match kind {
                        EdgeKind::Ingress => g.edge(&eid, &id, kind, label),
                        _ => g.edge(&id, &eid, kind, label),
                    }
                }
            }
            for r in strs(d.get("dataset_references")) {
                let did = endpoint(&mut g, m, &r, Some("dataset"));
                g.edge(&id, &did, EdgeKind::References, None);
            }
            if opts.uses
                && let Some(u) = data_use
            {
                let uid = use_node(&mut g, u);
                let label = if cats.is_empty() {
                    d.get("name").and_then(Value::as_str).map(str::to_string)
                } else {
                    Some(cats.join(", "))
                };
                g.edge(
                    &id,
                    &uid,
                    EdgeKind::Uses,
                    label.or_else(|| Some(format!("declaration {i}"))),
                );
            }
            if opts.categories {
                for c in &cats {
                    let cid = category_node(&mut g, c);
                    g.edge(&id, &cid, EdgeKind::Categorizes, Some(String::new()));
                }
            }
            if opts.subjects {
                for s in &subjects {
                    let sid = subject_node(&mut g, s);
                    g.edge(&id, &sid, EdgeKind::Concerns, Some(String::new()));
                }
            }
        }
    }

    if opts.organizations {
        for org in m.of_type("organization") {
            let Some(key) = org.fides_key() else { continue };
            let id = format!("org:{key}");
            g.node(&id, key, NodeKind::Organization).detail = org.name().map(str::to_string);
            for r in m
                .iter()
                .filter(|r| r.resource_type == "system" || r.resource_type == "dataset")
            {
                if r.value
                    .get("organization_fides_key")
                    .and_then(Value::as_str)
                    == Some(key)
                    && let Some(rk) = r.fides_key()
                {
                    g.edge(
                        &id,
                        &format!("{}:{rk}", r.resource_type),
                        EdgeKind::Contains,
                        None,
                    );
                }
            }
        }
    }
    if opts.policies {
        for p in m.of_type("policy") {
            let Some(key) = p.fides_key() else { continue };
            let id = format!("policy:{key}");
            g.node(&id, key, NodeKind::Policy).detail = p.name().map(str::to_string);
            for rule in p
                .value
                .get("rules")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new())
            {
                if opts.uses {
                    for u in strs(rule.get("data_uses").and_then(|v| v.get("values"))) {
                        let uid = use_node(&mut g, &u);
                        g.edge(
                            &id,
                            &uid,
                            EdgeKind::Uses,
                            rule.get("name").and_then(Value::as_str).map(str::to_string),
                        );
                    }
                }
                if opts.categories {
                    for c in strs(rule.get("data_categories").and_then(|v| v.get("values"))) {
                        let cid = category_node(&mut g, &c);
                        g.edge(&id, &cid, EdgeKind::Categorizes, Some(String::new()));
                    }
                }
                if opts.subjects {
                    for s in strs(rule.get("data_subjects").and_then(|v| v.get("values"))) {
                        let sid = subject_node(&mut g, &s);
                        g.edge(&id, &sid, EdgeKind::Concerns, Some(String::new()));
                    }
                }
            }
        }
    }
    g
}

fn collect_field_categories(fields: Option<&Value>, out: &mut Vec<String>) {
    for f in fields.and_then(Value::as_array).unwrap_or(&Vec::new()) {
        for c in strs(f.get("data_categories")) {
            if !out.contains(&c) {
                out.push(c);
            }
        }
        collect_field_categories(f.get("fields"), out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn demo_resources_graph_has_expected_edges() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_resources");
        let m = crate::manifest::load::load(&[dir], None).unwrap();
        let g = build(&m, &Options::from_includes(&[Include::Uses]));
        assert!(g.get("system:demo_analytics_system").is_some());
        assert!(g.get("dataset:demo_users_dataset").is_some());
        assert!(
            g.edges()
                .iter()
                .any(|e| e.from == "dataset:demo_users_dataset"
                    && e.to == "system:demo_analytics_system"
                    && e.kind == EdgeKind::Ingress)
        );
        assert!(g.edges().iter().any(|e| e.to == "use:improve.system"));
    }
}

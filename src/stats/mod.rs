//! Counts and usage summaries over a manifest set.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::manifest::Manifest;
use crate::manifest::keys::taxonomy_uses;
use crate::taxonomy::{Kind, Taxonomy};

#[derive(Debug, Clone, Default, Serialize)]
pub struct DatasetStats {
    pub datasets: usize,
    pub collections: usize,
    pub fields: usize,
    pub categorized_fields: usize,
    pub uncategorized: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemStats {
    pub systems: usize,
    pub declarations: usize,
    pub systems_without_declarations: Vec<String>,
    pub referenced_datasets: usize,
    pub orphan_datasets: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub files: usize,
    pub resources: BTreeMap<String, usize>,
    pub datasets: DatasetStats,
    pub systems: SystemStats,
    /// Usage counts per taxonomy key, per kind (`data_category`, `data_use`, `data_subject`).
    pub usage: BTreeMap<String, BTreeMap<String, usize>>,
    /// Same, rolled up to every ancestor (only when requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollup: Option<BTreeMap<String, BTreeMap<String, usize>>>,
}

fn count_fields(fields: &[Value], prefix: &str, ds: &mut DatasetStats, locator: &str) {
    for f in fields {
        let Some(m) = f.as_object() else { continue };
        let name = m.get("name").and_then(Value::as_str).unwrap_or("?");
        let path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}.{name}")
        };
        let nested = m.get("fields").and_then(Value::as_array);
        if nested.is_none_or(Vec::is_empty) {
            ds.fields += 1;
            let cats = m.get("data_categories").and_then(Value::as_array);
            if cats.is_some_and(|c| !c.is_empty()) {
                ds.categorized_fields += 1;
            } else {
                ds.uncategorized.push(format!("{locator}.{path}"));
            }
        }
        if let Some(n) = nested {
            count_fields(n, &path, ds, locator);
        }
    }
}

/// Compute stats. `rollup` also credits every ancestor of each used key.
pub fn compute(m: &Manifest, tax: &Taxonomy, rollup: bool) -> Stats {
    let mut s = Stats {
        files: m.files().len(),
        ..Default::default()
    };
    for (t, n) in m.counts() {
        s.resources.insert(t, n);
    }

    for ds in m.of_type("dataset") {
        s.datasets.datasets += 1;
        let key = ds.fides_key().unwrap_or("?");
        for c in ds
            .value
            .get("collections")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
        {
            s.datasets.collections += 1;
            let cname = c.get("name").and_then(Value::as_str).unwrap_or("?");
            let fields = c
                .get("fields")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            count_fields(&fields, "", &mut s.datasets, &format!("{key}.{cname}"));
        }
    }

    let mut referenced: Vec<String> = Vec::new();
    for sys in m.of_type("system") {
        s.systems.systems += 1;
        let decls = sys
            .value
            .get("privacy_declarations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        s.systems.declarations += decls.len();
        if decls.is_empty() {
            s.systems
                .systems_without_declarations
                .push(sys.fides_key().unwrap_or("?").to_string());
        }
        for r in crate::manifest::keys::resource_refs(sys) {
            if m.get("dataset", &r.key).is_some() && !referenced.contains(&r.key) {
                referenced.push(r.key.clone());
            }
        }
    }
    s.systems.referenced_datasets = referenced.len();
    s.systems.orphan_datasets = m
        .of_type("dataset")
        .iter()
        .filter_map(|d| d.fides_key())
        .filter(|k| !referenced.iter().any(|r| r == k))
        .map(str::to_string)
        .collect();

    for kind in Kind::ALL {
        s.usage
            .insert(kind.resource_type().to_string(), BTreeMap::new());
    }
    let mut roll: BTreeMap<String, BTreeMap<String, usize>> = s.usage.clone();
    for r in m.iter() {
        for u in taxonomy_uses(r) {
            if Kind::from_resource_type(&r.resource_type).is_some() {
                continue; // parent_key of custom records is not a "use"
            }
            *s.usage
                .get_mut(u.kind.resource_type())
                .unwrap()
                .entry(u.key.clone())
                .or_default() += 1;
            if rollup {
                let table = roll.get_mut(u.kind.resource_type()).unwrap();
                *table.entry(u.key.clone()).or_default() += 1;
                for a in tax.table(u.kind).hierarchy().ancestors(&u.key) {
                    *table.entry(a).or_default() += 1;
                }
            }
        }
    }
    if rollup {
        s.rollup = Some(roll);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::embedded;
    use std::path::Path;

    #[test]
    fn demo_resources_stats() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_resources");
        let m = crate::manifest::load::load(&[dir], None).unwrap();
        let s = compute(&m, embedded::load(), true);
        assert_eq!(s.datasets.fields, 6);
        assert_eq!(s.datasets.categorized_fields, 5);
        assert_eq!(
            s.datasets.uncategorized,
            vec!["demo_users_dataset.users.food_preference"]
        );
        assert_eq!(s.systems.systems, 2);
        assert_eq!(s.systems.referenced_datasets, 1);
        assert_eq!(s.usage["data_use"]["improve.system"], 1);
        // Direct uses under `user` whose keys exist in 3.x (unknown keys have no ancestors):
        // email, name, unique_id, contact (x2: system + policy), device.cookie_id → 6.
        let roll = &s.rollup.as_ref().unwrap()["data_category"];
        assert_eq!(roll["user"], 6);
        assert_eq!(roll["user.contact"], 3);
    }
}

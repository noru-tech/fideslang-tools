//! Walk resources and collect every taxonomy key and every cross-resource reference they use,
//! each with a JSON-pointer-like path so diagnostics can point at the exact spot.

use serde_json::Value;

use super::Resource;
use crate::taxonomy::Kind;

/// A taxonomy key used somewhere in a resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyUse {
    pub kind: Kind,
    pub key: String,
    /// Path within the resource, e.g. `privacy_declarations[0].data_categories[1]`.
    pub path: String,
    /// The field name the key appeared under (`data_categories`, `data_use`, `data_purposes`…).
    pub field: String,
}

/// What a fides_key reference may point at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefTarget {
    Dataset,
    System,
    /// `dataset` or `system` (ingress/egress flows without a `type`).
    DatasetOrSystem,
    Organization,
    /// A `dataset.collection` pair (`fides_meta.after`, `erase_after`).
    Collection,
}

/// A reference to another resource by fides_key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRef {
    pub key: String,
    pub path: String,
    pub target: RefTarget,
}

fn join(path: &str, seg: &str) -> String {
    if path.is_empty() {
        seg.to_string()
    } else {
        format!("{path}.{seg}")
    }
}

fn push_list(out: &mut Vec<KeyUse>, kind: Kind, field: &str, path: &str, v: &Value) {
    match v {
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                if let Some(s) = item.as_str() {
                    out.push(KeyUse {
                        kind,
                        key: s.to_string(),
                        path: format!("{path}[{i}]"),
                        field: field.to_string(),
                    });
                }
            }
        }
        Value::String(s) => out.push(KeyUse {
            kind,
            key: s.clone(),
            path: path.to_string(),
            field: field.to_string(),
        }),
        // Policy rules: `{ matches: ANY, values: [...] }`
        Value::Object(m) => {
            if let Some(values) = m.get("values") {
                push_list(out, kind, field, &join(path, "values"), values);
            }
        }
        _ => {}
    }
}

fn walk_uses(out: &mut Vec<KeyUse>, path: &str, v: &Value) {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let p = join(path, k);
                match k.as_str() {
                    "data_categories" | "shared_categories" => {
                        push_list(out, Kind::Category, k, &p, val)
                    }
                    "data_use" | "data_uses" | "data_purposes" => {
                        push_list(out, Kind::Use, k, &p, val)
                    }
                    "data_subjects" => push_list(out, Kind::Subject, k, &p, val),
                    _ => walk_uses(out, &p, val),
                }
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                walk_uses(out, &format!("{path}[{i}]"), item);
            }
        }
        _ => {}
    }
}

/// Every taxonomy key referenced by the resource, with paths.
pub fn taxonomy_uses(resource: &Resource) -> Vec<KeyUse> {
    let mut out = Vec::new();
    // Custom taxonomy records: parent_key / replaced_by point at the same kind.
    if let Some(kind) = Kind::from_resource_type(&resource.resource_type) {
        for field in ["parent_key", "replaced_by"] {
            if let Some(s) = resource.value.get(field).and_then(Value::as_str) {
                out.push(KeyUse {
                    kind,
                    key: s.to_string(),
                    path: field.to_string(),
                    field: field.to_string(),
                });
            }
        }
        return out;
    }
    walk_uses(&mut out, "", &resource.value);
    out
}

fn push_refs(out: &mut Vec<KeyRef>, path: &str, v: &Value, target: RefTarget) {
    match v {
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                let p = format!("{path}[{i}]");
                match item {
                    Value::String(s) => out.push(KeyRef {
                        key: s.clone(),
                        path: p,
                        target,
                    }),
                    // DataFlow { fides_key, type, data_categories }
                    Value::Object(m) => {
                        if let Some(s) = m.get("fides_key").and_then(Value::as_str) {
                            let t = match m.get("type").and_then(Value::as_str) {
                                Some("dataset") => RefTarget::Dataset,
                                Some("system") => RefTarget::System,
                                _ => target,
                            };
                            out.push(KeyRef {
                                key: s.to_string(),
                                path: join(&p, "fides_key"),
                                target: t,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        Value::String(s) => out.push(KeyRef {
            key: s.clone(),
            path: path.to_string(),
            target,
        }),
        _ => {}
    }
}

fn walk_refs(out: &mut Vec<KeyRef>, path: &str, v: &Value, rtype: &str) {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let p = join(path, k);
                match k.as_str() {
                    "dataset_references" => push_refs(out, &p, val, RefTarget::Dataset),
                    "ingress" | "egress" => push_refs(out, &p, val, RefTarget::DatasetOrSystem),
                    "organization_fides_key" => push_refs(out, &p, val, RefTarget::Organization),
                    "after" | "erase_after" if rtype == "dataset" => {
                        // dataset-level `fides_meta.after` lists datasets; collection-level lists
                        // `dataset.collection` pairs. Distinguish by whether we are under `collections`.
                        let target = if path.contains("collections[") {
                            RefTarget::Collection
                        } else {
                            RefTarget::Dataset
                        };
                        push_refs(out, &p, val, target);
                    }
                    "references" if path.ends_with("fides_meta") => {
                        if let Some(items) = val.as_array() {
                            for (i, item) in items.iter().enumerate() {
                                if let Some(ds) = item.get("dataset").and_then(Value::as_str) {
                                    out.push(KeyRef {
                                        key: ds.to_string(),
                                        path: format!("{p}[{i}].dataset"),
                                        target: RefTarget::Dataset,
                                    });
                                }
                            }
                        }
                    }
                    _ => walk_refs(out, &p, val, rtype),
                }
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                walk_refs(out, &format!("{path}[{i}]"), item, rtype);
            }
        }
        _ => {}
    }
}

/// Every cross-resource reference in the resource, with paths.
pub fn resource_refs(resource: &Resource) -> Vec<KeyRef> {
    let mut out = Vec::new();
    walk_refs(&mut out, "", &resource.value, &resource.resource_type);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn res(rtype: &str, v: Value) -> Resource {
        Resource {
            resource_type: rtype.into(),
            value: v,
            source: None,
            index: 0,
        }
    }

    #[test]
    fn collects_taxonomy_uses_with_paths() {
        let r = res(
            "system",
            json!({
                "fides_key": "s",
                "privacy_declarations": [{
                    "data_use": "analytics", "data_categories": ["user.contact", "user.name"], "data_subjects": ["customer"]
                }]
            }),
        );
        let uses = taxonomy_uses(&r);
        assert_eq!(uses.len(), 4);
        assert_eq!(uses[0].path, "privacy_declarations[0].data_use");
        assert_eq!(uses[1].path, "privacy_declarations[0].data_categories[0]");
        assert_eq!(uses[3].kind, Kind::Subject);

        let p = res(
            "policy",
            json!({"rules": [{"data_uses": {"matches": "ANY", "values": ["advertising"]}}]}),
        );
        let uses = taxonomy_uses(&p);
        assert_eq!(uses[0].path, "rules[0].data_uses.values[0]");
        assert_eq!(uses[0].kind, Kind::Use);
    }

    #[test]
    fn collects_references() {
        let r = res(
            "system",
            json!({
                "fides_key": "s",
                "dataset_references": ["ds1"],
                "ingress": [{"fides_key": "ds2", "type": "dataset"}, {"fides_key": "other_sys"}],
                "privacy_declarations": [{"egress": ["down"]}]
            }),
        );
        let refs = resource_refs(&r);
        let keys: Vec<_> = refs.iter().map(|r| (r.key.as_str(), r.target)).collect();
        assert_eq!(
            keys,
            vec![
                ("ds1", RefTarget::Dataset),
                ("ds2", RefTarget::Dataset),
                ("other_sys", RefTarget::DatasetOrSystem),
                ("down", RefTarget::DatasetOrSystem),
            ]
        );
        assert_eq!(refs[1].path, "ingress[0].fides_key");
    }
}

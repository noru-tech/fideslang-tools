//! W002 uncategorized dataset field · W003 empty privacy declaration.

use serde_json::Value;

use super::diag;
use crate::manifest::{Manifest, Resource};
use crate::validate::{Diagnostic, Severity};

fn is_empty_list(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(Value::Array(a)) => a.is_empty(),
        _ => false,
    }
}

fn walk_fields(r: &Resource, fields: &[Value], path: &str, out: &mut Vec<Diagnostic>) {
    for (i, f) in fields.iter().enumerate() {
        let Some(m) = f.as_object() else { continue };
        let p = format!("{path}[{i}]");
        let nested = m.get("fields").and_then(Value::as_array);
        // A field with sub-fields is a container; only leaves need categories.
        if nested.is_none_or(Vec::is_empty) && is_empty_list(m.get("data_categories")) {
            let name = m.get("name").and_then(Value::as_str).unwrap_or("?");
            out.push(diag(
                r,
                "W002",
                Severity::Warning,
                format!("{p}.data_categories"),
                format!("field `{name}` has no data_categories"),
            ));
        }
        if let Some(n) = nested {
            walk_fields(r, n, &format!("{p}.fields"), out);
        }
    }
}

pub fn run(manifest: &Manifest, out: &mut Vec<Diagnostic>) {
    for r in manifest.of_type("dataset") {
        for (i, c) in r
            .value
            .get("collections")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
            .iter()
            .enumerate()
        {
            if let Some(fields) = c.get("fields").and_then(Value::as_array) {
                walk_fields(r, fields, &format!("collections[{i}].fields"), out);
            }
        }
    }
    for r in manifest.of_type("system") {
        for (i, d) in r
            .value
            .get("privacy_declarations")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
            .iter()
            .enumerate()
        {
            let Some(m) = d.as_object() else { continue };
            let p = format!("privacy_declarations[{i}]");
            if is_empty_list(m.get("data_categories")) {
                out.push(diag(
                    r,
                    "W003",
                    Severity::Warning,
                    format!("{p}.data_categories"),
                    "declaration has no data_categories",
                ));
            }
            if is_empty_list(m.get("data_subjects")) {
                out.push(diag(
                    r,
                    "W003",
                    Severity::Warning,
                    format!("{p}.data_subjects"),
                    "declaration has no data_subjects",
                ));
            }
        }
    }
}

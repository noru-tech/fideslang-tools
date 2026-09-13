//! E004 parent_key mismatch · E005 self-reference, for `data_category` / `data_use` /
//! `data_subject` resources declared in a manifest set.

use serde_json::Value;

use super::diag;
use crate::manifest::Manifest;
use crate::taxonomy::{Kind, Taxonomy, record::dotted_parent};
use crate::validate::{Diagnostic, Severity};

pub fn run(manifest: &Manifest, tax: &Taxonomy, out: &mut Vec<Diagnostic>) {
    for kind in [Kind::Category, Kind::Use, Kind::Subject] {
        for r in manifest.of_type(kind.resource_type()) {
            let Some(key) = r.fides_key() else { continue };
            let parent = r.value.get("parent_key").and_then(Value::as_str);
            if parent == Some(key) {
                out.push(diag(
                    r,
                    "E005",
                    Severity::Error,
                    "parent_key",
                    format!("`{key}` cannot be its own parent"),
                ));
                continue;
            }
            if r.value.get("replaced_by").and_then(Value::as_str) == Some(key) {
                out.push(diag(
                    r,
                    "E005",
                    Severity::Error,
                    "replaced_by",
                    format!("`{key}` cannot replace itself"),
                ));
            }
            if kind == Kind::Subject {
                continue; // subjects are flat upstream
            }
            let expected = dotted_parent(key);
            match (parent, &expected) {
                (Some(p), Some(e)) if p != e => {
                    out.push(diag(
                        r,
                        "E004",
                        Severity::Error,
                        "parent_key",
                        format!(
                            "parent_key `{p}` does not match the dotted prefix `{e}` of `{key}`"
                        ),
                    ));
                }
                (None, Some(e)) => {
                    out.push(diag(
                        r,
                        "E004",
                        Severity::Error,
                        "parent_key",
                        format!("`{key}` has no parent_key; expected `{e}`"),
                    ));
                }
                (Some(p), None) => {
                    out.push(diag(
                        r,
                        "E004",
                        Severity::Error,
                        "parent_key",
                        format!("`{key}` is a top-level key but declares parent_key `{p}`"),
                    ));
                }
                _ => {}
            }
            // An unknown parent is reported as E001 by the keys rule (with suggestions).
            let _ = tax;
        }
    }
}

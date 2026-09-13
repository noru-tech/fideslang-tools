//! E002 duplicate `fides_key` within a resource type (across all loaded files).

use std::collections::HashMap;

use super::diag;
use crate::manifest::Manifest;
use crate::validate::{Diagnostic, Severity};

pub fn run(manifest: &Manifest, out: &mut Vec<Diagnostic>) {
    for rtype in manifest.resource_types() {
        let mut seen: HashMap<&str, &crate::manifest::Resource> = HashMap::new();
        for r in manifest.of_type(rtype) {
            let Some(key) = r.fides_key() else { continue };
            match seen.get(key) {
                Some(first) => {
                    out.push(diag(
                        r,
                        "E002",
                        Severity::Error,
                        "fides_key",
                        format!(
                            "duplicate {rtype} `{key}` (first defined in {})",
                            first.source_display()
                        ),
                    ));
                }
                None => {
                    seen.insert(key, r);
                }
            }
        }
    }
}

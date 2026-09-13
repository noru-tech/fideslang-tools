//! E003 dangling references between resources.

use super::diag;
use crate::manifest::Manifest;
use crate::manifest::keys::{RefTarget, resource_refs};
use crate::validate::{Diagnostic, Severity};

/// Built-in endpoint names Fides accepts in data flows without a matching resource.
const BUILTIN_ENDPOINTS: [&str; 1] = ["user"];

pub fn run(manifest: &Manifest, out: &mut Vec<Diagnostic>) {
    for r in manifest.iter() {
        for reference in resource_refs(r) {
            let ok = match reference.target {
                RefTarget::Dataset => manifest.get("dataset", &reference.key).is_some(),
                RefTarget::System => manifest.get("system", &reference.key).is_some(),
                RefTarget::DatasetOrSystem => {
                    manifest.get("dataset", &reference.key).is_some()
                        || manifest.get("system", &reference.key).is_some()
                        || BUILTIN_ENDPOINTS.contains(&reference.key.as_str())
                }
                RefTarget::Organization => {
                    // `default_organization` is implicit upstream; only flag explicit others.
                    reference.key == "default_organization"
                        || manifest.get("organization", &reference.key).is_some()
                        || manifest.of_type("organization").is_empty()
                }
                RefTarget::Collection => match reference.key.split_once('.') {
                    Some((ds, coll)) => manifest.get("dataset", ds).is_some_and(|d| {
                        d.value
                            .get("collections")
                            .and_then(|c| c.as_array())
                            .is_some_and(|cols| {
                                cols.iter()
                                    .any(|c| c.get("name").and_then(|n| n.as_str()) == Some(coll))
                            })
                    }),
                    None => manifest.get("dataset", &reference.key).is_some(),
                },
            };
            if !ok {
                let what = match reference.target {
                    RefTarget::Dataset => "dataset",
                    RefTarget::System => "system",
                    RefTarget::DatasetOrSystem => "dataset or system",
                    RefTarget::Organization => "organization",
                    RefTarget::Collection => "dataset.collection",
                };
                out.push(diag(
                    r,
                    "E003",
                    Severity::Error,
                    reference.path,
                    format!("reference to unknown {what} `{}`", reference.key),
                ));
            }
        }
    }
}

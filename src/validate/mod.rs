//! Manifest validation: stable diagnostic codes, rules, and reports.
//!
//! | Code | Severity | Check |
//! |------|----------|-------|
//! | E001 | error    | unknown taxonomy key (with "did you mean" suggestions) |
//! | E002 | error    | duplicate `fides_key` within a resource type |
//! | E003 | error    | dangling reference (`dataset_references`, ingress/egress, `fides_meta.references`, …) |
//! | E004 | error    | custom taxonomy `parent_key` missing or not the dotted prefix |
//! | E005 | error    | custom taxonomy record references itself |
//! | E006 | error    | invalid `fides_key` syntax |
//! | E007 | error    | schema: missing required field, wrong type, unknown resource type |
//! | W001 | warning  | deprecated taxonomy key |
//! | W002 | warning  | dataset field without `data_categories` |
//! | W003 | warning  | privacy declaration with no categories or no subjects |
//! | W004 | warning  | `data_purposes` (deprecated alias of `data_uses`) |
//! | W005 | warning  | unknown field on a resource (only with `--strict`) |

pub mod rules;
pub mod suggest;

use std::collections::BTreeSet;
use std::fmt;

use serde::Serialize;

use crate::manifest::Manifest;
use crate::taxonomy::Taxonomy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        })
    }
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    /// Source file, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// `system[demo_marketing_system]`
    pub resource: String,
    /// Path within the resource (`privacy_declarations[0].data_use`), empty for resource-level.
    pub path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn location(&self) -> String {
        if self.path.is_empty() {
            self.resource.clone()
        } else {
            format!("{}.{}", self.resource, self.path)
        }
    }
}

/// Knobs for a validation run.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Also report W005 (unknown fields).
    pub strict: bool,
    /// Ignore custom taxonomy records declared in the manifest set.
    pub no_custom_taxonomy: bool,
    /// Promote these codes to errors.
    pub deny: BTreeSet<String>,
    /// Silence these codes.
    pub allow: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Report {
    pub taxonomy: String,
    pub files: usize,
    pub resources: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    pub fn warnings(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    pub fn is_clean(&self) -> bool {
        self.errors() == 0
    }
}

/// Run every rule against `manifest` using `taxonomy` (extended with the manifest's own custom
/// records unless `no_custom_taxonomy`).
pub fn validate(manifest: &Manifest, taxonomy: &Taxonomy, opts: &Options) -> Report {
    let extended;
    let tax: &Taxonomy = if opts.no_custom_taxonomy {
        taxonomy
    } else {
        extended = taxonomy.extended_with(manifest.custom_taxonomy());
        &extended
    };

    let mut diagnostics = Vec::new();
    rules::schema::run(manifest, &mut diagnostics, opts.strict);
    rules::keys::run(manifest, tax, &mut diagnostics);
    rules::duplicates::run(manifest, &mut diagnostics);
    rules::references::run(manifest, &mut diagnostics);
    rules::custom_taxonomy::run(manifest, tax, &mut diagnostics);
    rules::hygiene::run(manifest, &mut diagnostics);

    for d in &mut diagnostics {
        if opts.deny.contains(d.code) {
            d.severity = Severity::Error;
        }
    }
    diagnostics.retain(|d| !opts.allow.contains(d.code));

    // Stable order: by file, resource, path, code.
    diagnostics.sort_by(|a, b| {
        (a.file.as_deref(), &a.resource, &a.path, a.code).cmp(&(
            b.file.as_deref(),
            &b.resource,
            &b.path,
            b.code,
        ))
    });
    diagnostics.dedup();

    Report {
        taxonomy: tax.label(),
        files: manifest.files().len(),
        resources: manifest.len(),
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::embedded;
    use std::path::Path;

    /// Upstream's demo manifests predate Fideslang 3.0: several keys they use were renamed.
    /// That makes them a realistic fixture for E001.
    #[test]
    fn demo_resources_report_the_stale_keys() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_resources");
        let m = crate::manifest::load::load(&[dir], None).unwrap();
        let report = validate(&m, embedded::load(), &Options::default());
        let e001: Vec<&Diagnostic> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == "E001")
            .collect();
        let keys: Vec<&str> = e001
            .iter()
            .map(|d| d.message.split('`').nth(1).unwrap())
            .collect();
        assert_eq!(
            keys,
            vec![
                "user.contact.state",
                "third_party_sharing.personalized_advertising",
                "advertising",
                "improve.system",
                "user.cookie_id",
                "advertising",
            ]
        );
        assert_eq!(report.errors(), 6, "{:#?}", report.diagnostics);
        assert_eq!(report.warnings(), 1); // food_preference has no categories
        let by_key = |k: &str| {
            e001.iter()
                .find(|d| d.message.contains(&format!("`{k}`")))
                .unwrap()
        };
        assert!(
            by_key("user.cookie_id")
                .suggestion
                .as_deref()
                .unwrap()
                .contains("`user.device.cookie_id`")
        );
        assert!(
            by_key("advertising")
                .suggestion
                .as_deref()
                .unwrap()
                .contains("`marketing.advertising`")
        );
        assert!(
            by_key("user.contact.state")
                .suggestion
                .as_deref()
                .unwrap()
                .contains("`user.contact.address.state`")
        );
    }
}

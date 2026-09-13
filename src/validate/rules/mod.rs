//! One module per rule family. Each exposes `run(...)` pushing into a `Vec<Diagnostic>`.

pub mod custom_taxonomy;
pub mod duplicates;
pub mod hygiene;
pub mod keys;
pub mod references;
pub mod schema;

use super::{Diagnostic, Severity};
use crate::manifest::Resource;

/// Upstream `FIDES_KEY_PATTERN`.
pub const FIDES_KEY_PATTERN: &str = r"^[a-zA-Z0-9_.<>-]+$";

pub fn is_valid_fides_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '<' | '>' | '-'))
}

pub(crate) fn diag(
    resource: &Resource,
    code: &'static str,
    severity: Severity,
    path: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code,
        severity,
        file: resource.source.as_deref().map(|p| p.display().to_string()),
        resource: resource.locator(),
        path: path.into(),
        message: message.into(),
        suggestion: None,
    }
}

//! One entry of a Fideslang taxonomy (a data category, data use or data subject).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A taxonomy record, mirroring upstream `DataCategory` / `DataUse` / `DataSubject`.
///
/// Field order matches the upstream export so that re-serialising a record reproduces the
/// vendored layout. Unknown fields are kept in `extra` so custom extensions survive round-trips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaxonomyRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_added: Option<String>,
    #[serde(default)]
    pub version_deprecated: Option<String>,
    #[serde(default)]
    pub replaced_by: Option<String>,
    #[serde(default)]
    pub is_default: bool,
    pub fides_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_fides_key: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parent_key: Option<String>,
    /// Data subjects only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rights: Option<Value>,
    /// Data subjects only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automated_decisions_or_profiling: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl TaxonomyRecord {
    /// A minimal record, used for custom taxonomy entries and tests.
    pub fn new(fides_key: impl Into<String>) -> Self {
        let fides_key = fides_key.into();
        Self {
            version_added: None,
            version_deprecated: None,
            replaced_by: None,
            is_default: false,
            parent_key: dotted_parent(&fides_key),
            fides_key,
            organization_fides_key: None,
            tags: None,
            name: None,
            description: None,
            rights: None,
            automated_decisions_or_profiling: None,
            extra: Map::new(),
        }
    }

    /// Human-readable name, falling back to the key.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.fides_key)
    }

    pub fn is_deprecated(&self) -> bool {
        self.version_deprecated.is_some()
    }

    /// The parent implied by the dotted key (`a.b.c` → `a.b`), regardless of `parent_key`.
    pub fn dotted_parent(&self) -> Option<String> {
        dotted_parent(&self.fides_key)
    }

    /// The last dotted segment of the key (`user.contact.email` → `email`).
    pub fn leaf(&self) -> &str {
        self.fides_key.rsplit('.').next().unwrap_or(&self.fides_key)
    }
}

/// `a.b.c` → `Some("a.b")`; `a` → `None`.
pub fn dotted_parent(key: &str) -> Option<String> {
    key.rsplit_once('.').map(|(parent, _)| parent.to_string())
}

/// Number of dotted segments (`user.contact.email` → 3).
pub fn depth_of(key: &str) -> usize {
    key.split('.').count()
}

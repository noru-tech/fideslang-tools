//! Fides manifests: YAML/JSON documents whose top-level keys are resource types
//! (`dataset`, `system`, `organization`, `policy`, `data_category`, `data_use`, `data_subject`),
//! each holding a list of resources identified by `fides_key`.

pub mod filter;
pub mod keys;
pub mod load;

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::taxonomy::{Kind, TaxonomyRecord};

/// Resource types in the order `fl` emits them (taxonomy extensions first, then the things that
/// reference them).
pub const RESOURCE_TYPES: [&str; 7] = [
    "organization",
    "data_category",
    "data_use",
    "data_subject",
    "dataset",
    "system",
    "policy",
];

/// Normalize the many spellings of a resource type (`systems`, `System`, `data-uses`) to the
/// canonical singular key. Unknown names are returned lower-cased so that filtering by an
/// arbitrary custom type still works.
pub fn canonical_type(name: &str) -> String {
    let n = name.trim().to_ascii_lowercase().replace('-', "_");
    match n.as_str() {
        "datasets" => "dataset".into(),
        "systems" => "system".into(),
        "organizations" | "org" | "orgs" => "organization".into(),
        "policies" => "policy".into(),
        other => match Kind::parse(other) {
            Some(k) => k.resource_type().into(),
            None => other.to_string(),
        },
    }
}

/// Sort key so output follows [`RESOURCE_TYPES`] order, then unknown types alphabetically.
pub fn type_order(rtype: &str) -> (usize, String) {
    let idx = RESOURCE_TYPES
        .iter()
        .position(|t| *t == rtype)
        .unwrap_or(RESOURCE_TYPES.len());
    (idx, rtype.to_string())
}

/// One resource plus where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Resource {
    pub resource_type: String,
    pub value: Value,
    /// File the resource was read from (`None` for stdin / synthetic).
    pub source: Option<PathBuf>,
    /// Index within its list in the source file.
    pub index: usize,
}

impl Resource {
    pub fn fides_key(&self) -> Option<&str> {
        self.value.get("fides_key").and_then(Value::as_str)
    }

    pub fn name(&self) -> Option<&str> {
        self.value.get("name").and_then(Value::as_str)
    }

    pub fn object(&self) -> Option<&Map<String, Value>> {
        self.value.as_object()
    }

    /// `system[demo_analytics_system]` (or `system[#2]` without a key).
    pub fn locator(&self) -> String {
        match self.fides_key() {
            Some(k) => format!("{}[{}]", self.resource_type, k),
            None => format!("{}[#{}]", self.resource_type, self.index),
        }
    }

    pub fn source_display(&self) -> String {
        self.source
            .as_deref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<stdin>".into())
    }
}

/// A set of resources, grouped by type, in load order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Manifest {
    resources: IndexMap<String, Vec<Resource>>,
    files: Vec<PathBuf>,
}

impl Manifest {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add every resource in a manifest-shaped `Value`. Non-list values and non-object items are
    /// kept as-is so that `validate` can report them; `cat`/`convert` pass them through.
    pub fn add_document(&mut self, doc: Value, source: Option<&Path>) {
        let Value::Object(map) = doc else { return };
        for (rtype, list) in map {
            let items = match list {
                Value::Array(items) => items,
                Value::Null => Vec::new(),
                other => vec![other],
            };
            let bucket = self.resources.entry(rtype.clone()).or_default();
            for (index, value) in items.into_iter().enumerate() {
                bucket.push(Resource {
                    resource_type: rtype.clone(),
                    value,
                    source: source.map(Path::to_path_buf),
                    index,
                });
            }
        }
    }

    pub fn push(&mut self, resource: Resource) {
        self.resources
            .entry(resource.resource_type.clone())
            .or_default()
            .push(resource);
    }

    pub fn add_file(&mut self, path: &Path) {
        self.files.push(path.to_path_buf());
    }

    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub fn resource_types(&self) -> impl Iterator<Item = &str> {
        self.resources.keys().map(String::as_str)
    }

    pub fn of_type(&self, rtype: &str) -> &[Resource] {
        self.resources.get(rtype).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn iter(&self) -> impl Iterator<Item = &Resource> {
        self.resources.values().flatten()
    }

    pub fn len(&self) -> usize {
        self.resources.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Count per type, in canonical order.
    pub fn counts(&self) -> Vec<(String, usize)> {
        let mut v: Vec<(String, usize)> = self
            .resources
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();
        v.sort_by_key(|(k, _)| type_order(k));
        v
    }

    /// Find a resource by type and key.
    pub fn get(&self, rtype: &str, fides_key: &str) -> Option<&Resource> {
        self.of_type(rtype)
            .iter()
            .find(|r| r.fides_key() == Some(fides_key))
    }

    /// Find a resource with this key in any type.
    pub fn find_key(&self, fides_key: &str) -> Vec<&Resource> {
        self.iter()
            .filter(|r| r.fides_key() == Some(fides_key))
            .collect()
    }

    /// Custom taxonomy records declared in this manifest set (`data_category:` etc.).
    pub fn custom_taxonomy(&self) -> Vec<(Kind, TaxonomyRecord)> {
        Kind::ALL
            .into_iter()
            .flat_map(|kind| {
                self.of_type(kind.resource_type())
                    .iter()
                    .filter_map(move |r| {
                        serde_json::from_value::<TaxonomyRecord>(r.value.clone())
                            .ok()
                            .map(|rec| (kind, rec))
                    })
            })
            .collect()
    }

    /// Back to a single manifest-shaped document, types in canonical order.
    pub fn to_value(&self) -> Value {
        let mut types: Vec<&String> = self.resources.keys().collect();
        types.sort_by_key(|k| type_order(k));
        let mut map = Map::new();
        for t in types {
            let items: Vec<Value> = self.resources[t].iter().map(|r| r.value.clone()).collect();
            map.insert(t.clone(), Value::Array(items));
        }
        Value::Object(map)
    }

    /// Split into one document per resource type.
    pub fn split_by_type(&self) -> Vec<(String, Value)> {
        let mut types: Vec<&String> = self.resources.keys().collect();
        types.sort_by_key(|k| type_order(k));
        types
            .into_iter()
            .map(|t| {
                let items: Vec<Value> = self.resources[t].iter().map(|r| r.value.clone()).collect();
                let mut map = Map::new();
                map.insert(t.clone(), Value::Array(items));
                (t.clone(), Value::Object(map))
            })
            .collect()
    }
}

//! Fideslang taxonomies: the three classification groups (data categories, data uses, data
//! subjects), their hierarchy, and the vendored IAB Tech Lab snapshot.

pub mod diff;
pub mod embedded;
pub mod record;
pub mod search;
pub mod tree;

use std::collections::BTreeMap;
use std::fmt;

use anyhow::{Context, Result, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use record::TaxonomyRecord;
pub use tree::Hierarchy;

/// The three classification groups of the taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Category,
    Use,
    Subject,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Category, Kind::Use, Kind::Subject];

    /// Manifest / snapshot top-level key (`data_category`).
    pub fn resource_type(self) -> &'static str {
        match self {
            Kind::Category => "data_category",
            Kind::Use => "data_use",
            Kind::Subject => "data_subject",
        }
    }

    /// Plural snapshot file stem (`data_categories`).
    pub fn file_stem(self) -> &'static str {
        match self {
            Kind::Category => "data_categories",
            Kind::Use => "data_uses",
            Kind::Subject => "data_subjects",
        }
    }

    /// Short human label (`categories`).
    pub fn label(self) -> &'static str {
        match self {
            Kind::Category => "categories",
            Kind::Use => "uses",
            Kind::Subject => "subjects",
        }
    }

    /// Singular human label (`data category`).
    pub fn human(self) -> &'static str {
        match self {
            Kind::Category => "data category",
            Kind::Use => "data use",
            Kind::Subject => "data subject",
        }
    }

    /// Accepts the many spellings people use on the command line.
    pub fn parse(s: &str) -> Option<Kind> {
        match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "c" | "cat" | "cats" | "category" | "categories" | "data_category"
            | "data_categories" => Some(Kind::Category),
            "u" | "use" | "uses" | "data_use" | "data_uses" | "purpose" | "purposes" => {
                Some(Kind::Use)
            }
            "s" | "subject" | "subjects" | "data_subject" | "data_subjects" => Some(Kind::Subject),
            _ => None,
        }
    }

    /// Parse a resource type key exactly (`data_category`), as found in manifests.
    pub fn from_resource_type(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.resource_type() == s)
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.resource_type())
    }
}

/// Contents of a snapshot's `snapshot.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub snapshot: String,
    pub upstream: String,
    pub tag: String,
    pub commit: String,
    pub snapshot_date: String,
    pub license: String,
    pub counts: BTreeMap<String, usize>,
}

impl Provenance {
    /// A label like `iab 3.0.0`.
    pub fn label(&self) -> String {
        if self.tag.is_empty() {
            self.snapshot.clone()
        } else {
            format!("{} {}", self.snapshot, self.tag)
        }
    }

    /// For custom-only taxonomies built from manifests.
    pub fn custom(label: &str) -> Self {
        Provenance {
            snapshot: label.to_string(),
            upstream: String::new(),
            tag: String::new(),
            commit: String::new(),
            snapshot_date: String::new(),
            license: String::new(),
            counts: BTreeMap::new(),
        }
    }
}

/// All records of one kind, in declaration order, plus the derived hierarchy.
#[derive(Debug, Clone, Default)]
pub struct KindTable {
    records: IndexMap<String, TaxonomyRecord>,
    hierarchy: Hierarchy,
}

impl KindTable {
    pub fn from_records(records: impl IntoIterator<Item = TaxonomyRecord>) -> Self {
        let mut table = KindTable::default();
        for r in records {
            table.records.insert(r.fides_key.clone(), r);
        }
        table.rebuild();
        table
    }

    fn rebuild(&mut self) {
        self.hierarchy = Hierarchy::build(self.records.values());
    }

    pub fn insert(&mut self, record: TaxonomyRecord) -> Option<TaxonomyRecord> {
        let prev = self.records.insert(record.fides_key.clone(), record);
        self.rebuild();
        prev
    }

    pub fn get(&self, key: &str) -> Option<&TaxonomyRecord> {
        self.records.get(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.records.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.records.keys().map(String::as_str)
    }

    pub fn records(&self) -> impl Iterator<Item = &TaxonomyRecord> {
        self.records.values()
    }

    pub fn hierarchy(&self) -> &Hierarchy {
        &self.hierarchy
    }
}

/// A complete taxonomy: three kind tables plus provenance.
#[derive(Debug, Clone)]
pub struct Taxonomy {
    provenance: Provenance,
    tables: [KindTable; 3],
}

impl Taxonomy {
    pub fn new(provenance: Provenance) -> Self {
        Taxonomy {
            provenance,
            tables: [
                KindTable::default(),
                KindTable::default(),
                KindTable::default(),
            ],
        }
    }

    /// Parse one kind from the YAML text of a snapshot file (`data_category: [...]`).
    pub fn parse_kind_yaml(kind: Kind, yaml: &str) -> Result<Vec<TaxonomyRecord>> {
        let doc: Value = crate::format::parse_yaml(yaml)?;
        records_from_value(kind, &doc)
    }

    pub fn set_kind(&mut self, kind: Kind, table: KindTable) {
        self.tables[kind_index(kind)] = table;
    }

    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// `iab 3.0.0`
    pub fn label(&self) -> String {
        self.provenance.label()
    }

    pub fn table(&self, kind: Kind) -> &KindTable {
        &self.tables[kind_index(kind)]
    }

    pub fn table_mut(&mut self, kind: Kind) -> &mut KindTable {
        &mut self.tables[kind_index(kind)]
    }

    pub fn get(&self, kind: Kind, key: &str) -> Option<&TaxonomyRecord> {
        self.table(kind).get(key)
    }

    pub fn contains(&self, kind: Kind, key: &str) -> bool {
        self.table(kind).contains(key)
    }

    /// Find a key in any kind (keys are unique across kinds in practice, but report all hits).
    pub fn lookup(&self, key: &str) -> Vec<(Kind, &TaxonomyRecord)> {
        Kind::ALL
            .into_iter()
            .filter_map(|k| self.get(k, key).map(|r| (k, r)))
            .collect()
    }

    pub fn counts(&self) -> BTreeMap<Kind, usize> {
        Kind::ALL
            .into_iter()
            .map(|k| (k, self.table(k).len()))
            .collect()
    }

    /// Extend a copy of this taxonomy with custom records (e.g. the `data_category:` /
    /// `data_use:` / `data_subject:` resources declared in a manifest set). Later records win.
    pub fn extended_with(
        &self,
        custom: impl IntoIterator<Item = (Kind, TaxonomyRecord)>,
    ) -> Taxonomy {
        let mut out = self.clone();
        for (kind, record) in custom {
            out.table_mut(kind).insert(record);
        }
        out
    }
}

fn kind_index(kind: Kind) -> usize {
    match kind {
        Kind::Category => 0,
        Kind::Use => 1,
        Kind::Subject => 2,
    }
}

/// Extract records of `kind` from a manifest-shaped value (`{ data_category: [ {...}, ... ] }`).
pub fn records_from_value(kind: Kind, doc: &Value) -> Result<Vec<TaxonomyRecord>> {
    let Some(map) = doc.as_object() else {
        bail!(
            "expected a mapping with a `{}` key at the top level",
            kind.resource_type()
        );
    };
    let Some(list) = map.get(kind.resource_type()) else {
        bail!("no `{}` key at the top level", kind.resource_type());
    };
    let Some(items) = list.as_array() else {
        bail!("`{}` must be a list", kind.resource_type());
    };
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            serde_json::from_value::<TaxonomyRecord>(item.clone()).with_context(|| {
                format!(
                    "{}[{}] is not a valid taxonomy record",
                    kind.resource_type(),
                    i
                )
            })
        })
        .collect()
}

//! Compare two taxonomies key by key.

use serde::Serialize;

use super::{Kind, Taxonomy, TaxonomyRecord};

/// A field-level change on a record present in both taxonomies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldChange {
    pub field: &'static str,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Changed {
    pub fides_key: String,
    pub changes: Vec<FieldChange>,
}

/// Differences for one kind.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KindDiff {
    pub kind: Kind,
    pub added: Vec<TaxonomyRecord>,
    pub removed: Vec<TaxonomyRecord>,
    pub changed: Vec<Changed>,
    pub from_count: usize,
    pub to_count: usize,
}

impl KindDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaxonomyDiff {
    pub from: String,
    pub to: String,
    pub kinds: Vec<KindDiff>,
}

impl TaxonomyDiff {
    pub fn is_empty(&self) -> bool {
        self.kinds.iter().all(KindDiff::is_empty)
    }

    pub fn totals(&self) -> (usize, usize, usize) {
        self.kinds.iter().fold((0, 0, 0), |(a, r, c), k| {
            (a + k.added.len(), r + k.removed.len(), c + k.changed.len())
        })
    }
}

impl serde::Serialize for Kind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.resource_type())
    }
}

fn field_changes(a: &TaxonomyRecord, b: &TaxonomyRecord) -> Vec<FieldChange> {
    let mut out = Vec::new();
    let mut cmp = |field: &'static str, x: &Option<String>, y: &Option<String>| {
        if x != y {
            out.push(FieldChange {
                field,
                from: x.clone(),
                to: y.clone(),
            });
        }
    };
    cmp("name", &a.name, &b.name);
    cmp("description", &a.description, &b.description);
    cmp("parent_key", &a.parent_key, &b.parent_key);
    cmp("version_added", &a.version_added, &b.version_added);
    cmp(
        "version_deprecated",
        &a.version_deprecated,
        &b.version_deprecated,
    );
    cmp("replaced_by", &a.replaced_by, &b.replaced_by);
    out
}

/// Diff `from` → `to` for the given kinds (all three by default).
pub fn diff(from: &Taxonomy, to: &Taxonomy, kinds: &[Kind]) -> TaxonomyDiff {
    let kinds = kinds
        .iter()
        .map(|&kind| {
            let a = from.table(kind);
            let b = to.table(kind);
            let added = b
                .records()
                .filter(|r| !a.contains(&r.fides_key))
                .cloned()
                .collect();
            let removed = a
                .records()
                .filter(|r| !b.contains(&r.fides_key))
                .cloned()
                .collect();
            let changed = a
                .records()
                .filter_map(|ra| {
                    let rb = b.get(&ra.fides_key)?;
                    let changes = field_changes(ra, rb);
                    (!changes.is_empty()).then(|| Changed {
                        fides_key: ra.fides_key.clone(),
                        changes,
                    })
                })
                .collect();
            KindDiff {
                kind,
                added,
                removed,
                changed,
                from_count: a.len(),
                to_count: b.len(),
            }
        })
        .collect();
    TaxonomyDiff {
        from: from.label(),
        to: to.label(),
        kinds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::{Snapshot, embedded};

    #[test]
    fn iab_to_ethyca_adds_exactly_one_data_use() {
        let d = diff(
            embedded::load(Snapshot::Iab),
            embedded::load(Snapshot::Ethyca),
            &Kind::ALL,
        );
        assert_eq!(d.totals(), (1, 0, 0));
        let uses = d.kinds.iter().find(|k| k.kind == Kind::Use).unwrap();
        assert_eq!(
            uses.added[0].fides_key,
            "functional.storage.privacy_preferences"
        );
    }
}

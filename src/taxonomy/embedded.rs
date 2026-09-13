//! The vendored IAB Tech Lab taxonomy snapshot, compiled into the binary and parsed on first use.

use std::sync::OnceLock;

use super::{Kind, KindTable, Provenance, Taxonomy};

const CATEGORIES: &str = include_str!("../../taxonomy/data_categories.yml");
const USES: &str = include_str!("../../taxonomy/data_uses.yml");
const SUBJECTS: &str = include_str!("../../taxonomy/data_subjects.yml");
const PROVENANCE: &str = include_str!("../../taxonomy/snapshot.json");

/// Raw YAML text of one vendored file, byte-for-byte.
pub fn raw(kind: Kind) -> &'static str {
    match kind {
        Kind::Category => CATEGORIES,
        Kind::Use => USES,
        Kind::Subject => SUBJECTS,
    }
}

/// Raw `snapshot.json` text.
pub fn raw_provenance() -> &'static str {
    PROVENANCE
}

fn parse() -> Taxonomy {
    let provenance: Provenance =
        serde_json::from_str(PROVENANCE).expect("vendored snapshot.json is valid");
    let mut tax = Taxonomy::new(provenance);
    for kind in Kind::ALL {
        let records = Taxonomy::parse_kind_yaml(kind, raw(kind))
            .unwrap_or_else(|e| panic!("vendored {kind} snapshot is valid: {e:#}"));
        tax.set_kind(kind, KindTable::from_records(records));
    }
    tax
}

/// The parsed snapshot. Parsing happens once per process, on first use.
pub fn load() -> &'static Taxonomy {
    static TAX: OnceLock<Taxonomy> = OnceLock::new();
    TAX.get_or_init(parse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_loads_and_matches_recorded_counts() {
        let tax = load();
        for kind in Kind::ALL {
            let recorded = tax.provenance().counts[kind.resource_type()];
            assert_eq!(tax.table(kind).len(), recorded, "{kind} count");
        }
        assert_eq!(tax.label(), "iab 3.0.0");
    }

    #[test]
    fn every_parent_key_exists_and_matches_dotted_prefix() {
        let tax = load();
        for kind in [Kind::Category, Kind::Use] {
            for r in tax.table(kind).records() {
                assert_eq!(r.parent_key, r.dotted_parent(), "{kind} {}", r.fides_key);
                if let Some(p) = &r.parent_key {
                    assert!(
                        tax.contains(kind, p),
                        "{kind} {} parent {p} missing",
                        r.fides_key
                    );
                }
            }
        }
    }
}

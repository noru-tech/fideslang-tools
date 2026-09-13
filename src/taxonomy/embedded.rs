//! The vendored snapshots, compiled into the binary and parsed on first use.

use std::sync::OnceLock;

use super::{Kind, KindTable, Provenance, Snapshot, Taxonomy};

macro_rules! snapshot_files {
    ($dir:literal) => {
        Files {
            categories: include_str!(concat!("../../taxonomy/", $dir, "/data_categories.yml")),
            uses: include_str!(concat!("../../taxonomy/", $dir, "/data_uses.yml")),
            subjects: include_str!(concat!("../../taxonomy/", $dir, "/data_subjects.yml")),
            provenance: include_str!(concat!("../../taxonomy/", $dir, "/snapshot.json")),
        }
    };
}

struct Files {
    categories: &'static str,
    uses: &'static str,
    subjects: &'static str,
    provenance: &'static str,
}

const ETHYCA: Files = snapshot_files!("ethyca");
const IAB: Files = snapshot_files!("iab");

fn files(snapshot: Snapshot) -> &'static Files {
    match snapshot {
        Snapshot::Ethyca => &ETHYCA,
        Snapshot::Iab => &IAB,
    }
}

/// Raw YAML text of one vendored file, byte-for-byte.
pub fn raw(snapshot: Snapshot, kind: Kind) -> &'static str {
    let f = files(snapshot);
    match kind {
        Kind::Category => f.categories,
        Kind::Use => f.uses,
        Kind::Subject => f.subjects,
    }
}

/// Raw `snapshot.json` text.
pub fn raw_provenance(snapshot: Snapshot) -> &'static str {
    files(snapshot).provenance
}

fn parse(snapshot: Snapshot) -> Taxonomy {
    let f = files(snapshot);
    let provenance: Provenance =
        serde_json::from_str(f.provenance).expect("vendored snapshot.json is valid");
    let mut tax = Taxonomy::new(provenance);
    for kind in Kind::ALL {
        let records = Taxonomy::parse_kind_yaml(kind, raw(snapshot, kind))
            .unwrap_or_else(|e| panic!("vendored {} {} snapshot is valid: {e:#}", snapshot, kind));
        tax.set_kind(kind, KindTable::from_records(records));
    }
    tax
}

/// The parsed snapshot. Parsing happens once per process, on first use.
pub fn load(snapshot: Snapshot) -> &'static Taxonomy {
    static ETHYCA_TAX: OnceLock<Taxonomy> = OnceLock::new();
    static IAB_TAX: OnceLock<Taxonomy> = OnceLock::new();
    match snapshot {
        Snapshot::Ethyca => ETHYCA_TAX.get_or_init(|| parse(Snapshot::Ethyca)),
        Snapshot::Iab => IAB_TAX.get_or_init(|| parse(Snapshot::Iab)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_load_and_match_recorded_counts() {
        for snapshot in Snapshot::ALL {
            let tax = load(snapshot);
            for kind in Kind::ALL {
                let recorded = tax.provenance().counts[kind.resource_type()];
                assert_eq!(tax.table(kind).len(), recorded, "{snapshot} {kind} count");
            }
        }
    }

    #[test]
    fn every_parent_key_exists_and_matches_dotted_prefix() {
        for snapshot in Snapshot::ALL {
            let tax = load(snapshot);
            for kind in [Kind::Category, Kind::Use] {
                for r in tax.table(kind).records() {
                    assert_eq!(
                        r.parent_key,
                        r.dotted_parent(),
                        "{snapshot} {kind} {}",
                        r.fides_key
                    );
                    if let Some(p) = &r.parent_key {
                        assert!(
                            tax.contains(kind, p),
                            "{snapshot} {kind} {} parent {p} missing",
                            r.fides_key
                        );
                    }
                }
            }
        }
    }
}

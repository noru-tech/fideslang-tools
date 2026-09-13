//! Search keys, names and descriptions.

use anyhow::{Context, Result};
use regex::{Regex, RegexBuilder};

use super::{Kind, Taxonomy, TaxonomyRecord};

/// What to match against.
#[derive(Debug, Clone)]
pub enum Matcher {
    /// Case-insensitive substring.
    Substring(String),
    Regex(Regex),
}

impl Matcher {
    pub fn new(pattern: &str, regex: bool, case_sensitive: bool) -> Result<Matcher> {
        if regex {
            let re = RegexBuilder::new(pattern)
                .case_insensitive(!case_sensitive)
                .build()
                .with_context(|| format!("invalid regex `{pattern}`"))?;
            Ok(Matcher::Regex(re))
        } else if case_sensitive {
            Ok(Matcher::Regex(
                RegexBuilder::new(&regex::escape(pattern))
                    .build()
                    .expect("escaped pattern"),
            ))
        } else {
            Ok(Matcher::Substring(pattern.to_lowercase()))
        }
    }

    /// Byte ranges of matches in `text`.
    pub fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        match self {
            Matcher::Substring(needle) => {
                if needle.is_empty() {
                    return Vec::new();
                }
                // Match on the lower-cased text; ASCII lowercasing keeps offsets stable for ASCII,
                // which is all taxonomy keys use. Names/descriptions are re-checked below.
                let hay = text.to_lowercase();
                if hay.len() != text.len() {
                    // Non-ASCII text: fall back to a whole-field hit, no spans.
                    return if hay.contains(needle.as_str()) {
                        vec![(0, text.len())]
                    } else {
                        vec![]
                    };
                }
                hay.match_indices(needle.as_str())
                    .map(|(i, m)| (i, i + m.len()))
                    .collect()
            }
            Matcher::Regex(re) => re.find_iter(text).map(|m| (m.start(), m.end())).collect(),
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        !self.find_all(text).is_empty()
    }
}

/// Which field matched, with byte spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldHit {
    pub field: &'static str,
    pub spans: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct Hit<'a> {
    pub kind: Kind,
    pub record: &'a TaxonomyRecord,
    pub fields: Vec<FieldHit>,
}

/// Search `kinds` of `tax`. With `keys_only`, only `fides_key` is searched.
pub fn search<'a>(tax: &'a Taxonomy, kinds: &[Kind], m: &Matcher, keys_only: bool) -> Vec<Hit<'a>> {
    let mut hits = Vec::new();
    for &kind in kinds {
        for record in tax.table(kind).records() {
            let mut fields = Vec::new();
            let spans = m.find_all(&record.fides_key);
            if !spans.is_empty() {
                fields.push(FieldHit {
                    field: "fides_key",
                    spans,
                });
            }
            if !keys_only {
                for (name, value) in [("name", &record.name), ("description", &record.description)]
                {
                    if let Some(v) = value {
                        let spans = m.find_all(v);
                        if !spans.is_empty() {
                            fields.push(FieldHit { field: name, spans });
                        }
                    }
                }
            }
            if !fields.is_empty() {
                hits.push(Hit {
                    kind,
                    record,
                    fields,
                });
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::{Snapshot, embedded};

    #[test]
    fn substring_and_regex_search() {
        let tax = embedded::load(Snapshot::Ethyca);
        let m = Matcher::new("cookie", false, false).unwrap();
        let hits = search(tax, &[Kind::Category], &m, true);
        assert!(
            hits.iter()
                .any(|h| h.record.fides_key == "user.device.cookie_id")
        );

        let m = Matcher::new(r"^user\.contact\.[a-z]+$", true, false).unwrap();
        let hits = search(tax, &[Kind::Category], &m, true);
        assert!(
            hits.iter()
                .all(|h| h.record.fides_key.starts_with("user.contact."))
        );
        assert!(!hits.is_empty());
    }
}

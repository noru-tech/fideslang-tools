//! "Did you mean …?" suggestions for unknown keys.

use crate::taxonomy::{KindTable, record::dotted_parent};

/// Up to `limit` candidate keys similar to `unknown`, best first.
///
/// Scores combine Jaro-Winkler similarity on the whole key with a bonus when the last dotted
/// segment matches (people usually get the leaf right and the path wrong, e.g.
/// `user.cookie_id` → `user.device.cookie_id`).
pub fn suggest(unknown: &str, table: &KindTable, limit: usize) -> Vec<String> {
    let leaf = unknown.rsplit('.').next().unwrap_or(unknown);
    let parent = dotted_parent(unknown);
    let mut scored: Vec<(f64, &str)> = table
        .keys()
        .map(|k| {
            let mut score = strsim::jaro_winkler(unknown, k);
            let k_leaf = k.rsplit('.').next().unwrap_or(k);
            if k_leaf == leaf {
                // Same leaf, different path (`advertising` → `marketing.advertising`): always a
                // candidate, ranked by how similar the rest is.
                score = (score + 0.15).max(0.9);
            } else if strsim::jaro_winkler(leaf, k_leaf) > 0.9 {
                score += 0.08;
            }
            if parent
                .as_ref()
                .is_some_and(|p| k.starts_with(&format!("{p}.")))
            {
                score += 0.05;
            }
            (score, k)
        })
        .filter(|(s, _)| *s >= 0.85)
        .collect();
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(b.1))
    });
    scored
        .into_iter()
        .take(limit)
        .map(|(_, k)| k.to_string())
        .collect()
}

/// Format as `did you mean `a`, `b`?`.
pub fn format(suggestions: &[String]) -> Option<String> {
    if suggestions.is_empty() {
        return None;
    }
    let list = suggestions
        .iter()
        .map(|s| format!("`{s}`"))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("did you mean {list}?"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::{Kind, embedded};

    #[test]
    fn suggests_plausible_keys() {
        let tax = embedded::load();
        let s = suggest("user.cookie_id", tax.table(Kind::Category), 3);
        assert_eq!(s.first().map(String::as_str), Some("user.device.cookie_id"));
        let s = suggest("advertising", tax.table(Kind::Use), 3);
        assert!(s.iter().any(|k| k == "marketing.advertising"), "{s:?}");
        let s = suggest("user.contact.emial", tax.table(Kind::Category), 3);
        assert_eq!(s.first().map(String::as_str), Some("user.contact.email"));
        assert!(suggest("zzzzzz", tax.table(Kind::Subject), 3).is_empty());
    }
}

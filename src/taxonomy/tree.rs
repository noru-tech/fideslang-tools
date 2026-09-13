//! Parent/child structure derived from `parent_key` (falling back to the dotted prefix).

use std::collections::{HashMap, HashSet};

use super::record::{TaxonomyRecord, dotted_parent};

/// Hierarchy over a set of keys. Children are kept in alphabetical order for stable output.
#[derive(Debug, Clone, Default)]
pub struct Hierarchy {
    parent: HashMap<String, Option<String>>,
    children: HashMap<String, Vec<String>>,
    roots: Vec<String>,
}

impl Hierarchy {
    pub fn build<'a>(records: impl IntoIterator<Item = &'a TaxonomyRecord>) -> Self {
        let records: Vec<&TaxonomyRecord> = records.into_iter().collect();
        let known: HashSet<&str> = records.iter().map(|r| r.fides_key.as_str()).collect();
        let mut h = Hierarchy::default();

        for r in &records {
            // Prefer the declared parent; fall back to the dotted prefix when it is missing or
            // points at an unknown key (so orphans still hang somewhere sensible).
            let declared = r.parent_key.clone().filter(|p| known.contains(p.as_str()));
            let parent = declared.or_else(|| {
                let mut cur = dotted_parent(&r.fides_key);
                while let Some(p) = cur {
                    if known.contains(p.as_str()) {
                        return Some(p);
                    }
                    cur = dotted_parent(&p);
                }
                None
            });
            match &parent {
                Some(p) => h
                    .children
                    .entry(p.clone())
                    .or_default()
                    .push(r.fides_key.clone()),
                None => h.roots.push(r.fides_key.clone()),
            }
            h.parent.insert(r.fides_key.clone(), parent);
        }
        for kids in h.children.values_mut() {
            kids.sort();
        }
        h.roots.sort();
        h
    }

    pub fn roots(&self) -> &[String] {
        &self.roots
    }

    pub fn children(&self, key: &str) -> &[String] {
        self.children.get(key).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn parent(&self, key: &str) -> Option<&str> {
        self.parent.get(key).and_then(|p| p.as_deref())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.parent.contains_key(key)
    }

    /// Ancestors from the root down to (excluding) `key`.
    pub fn ancestors(&self, key: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut cur = self.parent(key);
        while let Some(p) = cur {
            chain.push(p.to_string());
            cur = self.parent(p);
        }
        chain.reverse();
        chain
    }

    /// All descendants of `key`, depth-first, excluding `key`.
    pub fn descendants(&self, key: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack: Vec<&str> = self
            .children(key)
            .iter()
            .rev()
            .map(String::as_str)
            .collect();
        while let Some(k) = stack.pop() {
            out.push(k.to_string());
            stack.extend(self.children(k).iter().rev().map(String::as_str));
        }
        out
    }

    /// Depth of `key` in the hierarchy (roots are 0).
    pub fn depth(&self, key: &str) -> usize {
        self.ancestors(key).len()
    }

    /// Is `ancestor` equal to or an ancestor of `key`?
    pub fn is_within(&self, key: &str, ancestor: &str) -> bool {
        key == ancestor || self.ancestors(key).iter().any(|a| a == ancestor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(key: &str, parent: Option<&str>) -> TaxonomyRecord {
        let mut r = TaxonomyRecord::new(key);
        r.parent_key = parent.map(str::to_string);
        r
    }

    #[test]
    fn builds_from_parent_key_with_dotted_fallback() {
        let records = [
            rec("user", None),
            rec("user.contact", Some("user")),
            rec("user.contact.email", Some("user.contact")),
            rec("user.device.cookie_id", None), // parent missing → falls back to `user`
            rec("system", None),
        ];
        let h = Hierarchy::build(records.iter());
        assert_eq!(h.roots(), &["system", "user"]);
        assert_eq!(
            h.children("user"),
            &["user.contact", "user.device.cookie_id"]
        );
        assert_eq!(
            h.ancestors("user.contact.email"),
            vec!["user", "user.contact"]
        );
        assert_eq!(
            h.descendants("user"),
            vec![
                "user.contact",
                "user.contact.email",
                "user.device.cookie_id"
            ]
        );
        assert_eq!(h.depth("user.contact.email"), 2);
        assert!(h.is_within("user.contact.email", "user"));
        assert!(!h.is_within("system", "user"));
    }
}

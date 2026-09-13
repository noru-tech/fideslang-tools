//! `--type` and `--key` filtering.

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};

use super::{Manifest, canonical_type};

#[derive(Debug, Default)]
pub struct Filter {
    types: Option<Vec<String>>,
    keys: Option<GlobSet>,
}

impl Filter {
    /// `types`: comma-separated or repeated `--type`; `keys`: globs matched against `fides_key`.
    pub fn new(types: &[String], keys: &[String]) -> Result<Filter> {
        let types = (!types.is_empty()).then(|| {
            types
                .iter()
                .flat_map(|t| t.split(','))
                .filter(|t| !t.trim().is_empty())
                .map(canonical_type)
                .collect()
        });
        let keys = if keys.is_empty() {
            None
        } else {
            let mut b = GlobSetBuilder::new();
            for k in keys
                .iter()
                .flat_map(|k| k.split(','))
                .filter(|k| !k.trim().is_empty())
            {
                b.add(Glob::new(k.trim()).with_context(|| format!("invalid glob `{k}`"))?);
            }
            Some(b.build()?)
        };
        Ok(Filter { types, keys })
    }

    pub fn is_noop(&self) -> bool {
        self.types.is_none() && self.keys.is_none()
    }

    pub fn matches_type(&self, rtype: &str) -> bool {
        self.types
            .as_ref()
            .is_none_or(|ts| ts.iter().any(|t| t == rtype))
    }

    pub fn matches_key(&self, key: Option<&str>) -> bool {
        match (&self.keys, key) {
            (None, _) => true,
            (Some(set), Some(k)) => set.is_match(k),
            (Some(_), None) => false,
        }
    }

    pub fn apply(&self, manifest: &Manifest) -> Manifest {
        if self.is_noop() {
            return manifest.clone();
        }
        let mut out = Manifest::new();
        for f in manifest.files() {
            out.add_file(f);
        }
        for r in manifest.iter() {
            if self.matches_type(&r.resource_type) && self.matches_key(r.fides_key()) {
                out.push(r.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn filters_by_type_alias_and_key_glob() {
        let mut m = Manifest::new();
        m.add_document(
            json!({"system": [{"fides_key": "demo_a"}, {"fides_key": "prod_b"}], "dataset": [{"fides_key": "demo_ds"}]}),
            None,
        );
        let f = Filter::new(&["Systems".into()], &["demo_*".into()]).unwrap();
        let out = f.apply(&m);
        assert_eq!(out.len(), 1);
        assert_eq!(out.of_type("system")[0].fides_key(), Some("demo_a"));
    }
}

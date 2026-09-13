//! Loading manifests from files, directories and stdin — mirroring upstream `ingest_manifests`:
//! a directory is searched recursively for `*.yml` / `*.yaml` (and `*.json`), and all documents
//! are unioned per resource type.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;
use walkdir::WalkDir;

use super::Manifest;
use crate::format::{self, Format};

/// The conventional manifest directory used by the Fides CLI.
pub const DEFAULT_DIR: &str = ".fides";

fn is_manifest_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("yml" | "yaml" | "json")
    )
}

/// Expand paths: directories become their sorted manifest files; files and `-` pass through.
pub fn expand_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for p in paths {
        if p == Path::new("-") {
            out.push(p.clone());
        } else if p.is_dir() {
            let mut files: Vec<PathBuf> = WalkDir::new(p)
                .follow_links(true)
                .sort_by_file_name()
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file() && is_manifest_file(e.path()))
                .map(|e| e.into_path())
                .collect();
            files.sort();
            if files.is_empty() {
                bail!("{}: no .yml/.yaml/.json files found", p.display());
            }
            out.extend(files);
        } else if p.exists() {
            out.push(p.clone());
        } else {
            bail!("{}: no such file or directory", p.display());
        }
    }
    Ok(out)
}

/// Resolve the default when no paths were given: `.fides/` if it exists.
pub fn default_paths(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    if !paths.is_empty() {
        return Ok(paths);
    }
    let d = Path::new(DEFAULT_DIR);
    if d.is_dir() {
        Ok(vec![d.to_path_buf()])
    } else {
        bail!("no manifest path given and no ./{DEFAULT_DIR}/ directory found")
    }
}

/// Load one document (file or `-`) into `manifest`.
pub fn load_into(manifest: &mut Manifest, path: &Path, format: Option<Format>) -> Result<()> {
    let value = format::read_value(path, format)?;
    let source = (path != Path::new("-")).then_some(path);
    match &value {
        Value::Object(_) | Value::Null => {}
        _ => bail!(
            "{}: expected a mapping of resource types to lists at the top level",
            path.display()
        ),
    }
    manifest.add_document(value, source);
    manifest.add_file(path);
    Ok(())
}

/// Load and union every manifest under `paths`.
pub fn load(paths: &[PathBuf], format: Option<Format>) -> Result<Manifest> {
    let mut manifest = Manifest::new();
    for p in expand_paths(paths)? {
        load_into(&mut manifest, &p, format).with_context(|| format!("loading {}", p.display()))?;
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo_resources")
    }

    #[test]
    fn loads_demo_resources_directory() {
        let m = load(&[fixtures()], None).unwrap();
        assert_eq!(m.files().len(), 5);
        assert_eq!(m.of_type("system").len(), 2);
        assert_eq!(m.of_type("dataset").len(), 1);
        assert_eq!(
            m.get("system", "demo_marketing_system").unwrap().locator(),
            "system[demo_marketing_system]"
        );
        assert_eq!(m.custom_taxonomy().len(), 2);
        let counts = m.counts();
        assert_eq!(counts[0].0, "organization");
        assert_eq!(counts.last().unwrap().0, "policy");
    }
}

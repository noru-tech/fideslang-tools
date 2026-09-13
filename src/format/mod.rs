//! Reading and writing YAML / JSON / CSV as `serde_json::Value`.

pub mod csv;

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// Serialization formats `fl` understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Yaml,
    Json,
    Csv,
}

impl Format {
    /// Guess from a file extension. Unknown extensions default to YAML (the Fides convention).
    pub fn from_path(path: &Path) -> Format {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref()
        {
            Some("json") => Format::Json,
            Some("csv") => Format::Csv,
            _ => Format::Yaml,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Format::Yaml => "yml",
            Format::Json => "json",
            Format::Csv => "csv",
        }
    }
}

/// Parse YAML text into a `Value`. Uses YAML 1.2 booleans (`yes`/`no` stay strings), which is
/// what Fides manifests need (`version_added: 2.0.0` stays a string too).
pub fn parse_yaml(text: &str) -> Result<Value> {
    let mut opts = serde_saphyr::Options::default();
    opts.strict_booleans = true;
    let v: Value =
        serde_saphyr::from_str_with_options(text, opts).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(v)
}

pub fn parse_json(text: &str) -> Result<Value> {
    Ok(serde_json::from_str(text)?)
}

/// Parse text in `format`. CSV is parsed with the taxonomy layout (see [`csv`]).
pub fn parse(text: &str, format: Format) -> Result<Value> {
    match format {
        Format::Yaml => parse_yaml(text),
        Format::Json => parse_json(text),
        Format::Csv => csv::parse_taxonomy_csv(text),
    }
}

/// Read a whole file, or stdin for `-`.
pub fn read_text(path: &Path) -> Result<String> {
    if path == Path::new("-") {
        let mut s = String::new();
        io::stdin()
            .read_to_string(&mut s)
            .context("reading stdin")?;
        Ok(s)
    } else {
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))
    }
}

/// Read and parse a file (format from extension unless overridden).
pub fn read_value(path: &Path, format: Option<Format>) -> Result<Value> {
    let text = read_text(path)?;
    let format = format.unwrap_or_else(|| Format::from_path(path));
    parse(&text, format).with_context(|| format!("{}: invalid {:?}", path.display(), format))
}

/// Serialize a `Value` in `format`. CSV requires a manifest/taxonomy-shaped value.
pub fn to_string(value: &Value, format: Format) -> Result<String> {
    match format {
        Format::Yaml => to_yaml(value),
        Format::Json => Ok(format!("{}\n", serde_json::to_string_pretty(value)?)),
        Format::Csv => csv::to_csv(value),
    }
}

/// YAML in the upstream style: 2-space indent, block sequences flush with their parent key,
/// empty containers as `[]` / `{}`.
pub fn to_yaml(value: &Value) -> Result<String> {
    if value.is_null() {
        return Ok(String::new());
    }
    serde_saphyr::to_string(value).map_err(|e| anyhow::anyhow!("serializing YAML: {e}"))
}

pub fn write_value(w: &mut dyn Write, value: &Value, format: Format) -> Result<()> {
    w.write_all(to_string(value, format)?.as_bytes())?;
    Ok(())
}

/// A manifest-shaped document is a mapping whose values are lists.
pub fn expect_manifest_shape(value: &Value, what: &str) -> Result<()> {
    match value {
        Value::Object(map) if map.values().all(Value::is_array) => Ok(()),
        Value::Null => Ok(()),
        _ => bail!("{what}: expected a mapping of resource types to lists (e.g. `dataset: [...]`)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_keeps_versions_and_yes_no_as_strings() {
        let v = parse_yaml("a: yes\nb: 2.0.0\nc: true\nd: null\ne: []\n").unwrap();
        assert_eq!(v["a"], "yes");
        assert_eq!(v["b"], "2.0.0");
        assert_eq!(v["c"], true);
        assert!(v["d"].is_null());
        assert_eq!(v["e"], serde_json::json!([]));
    }

    #[test]
    fn yaml_round_trip_is_value_equal() {
        let text = "system:\n- fides_key: a\n  name: A\n  tags: null\n  privacy_declarations:\n  - data_use: x\n    data_categories: []\n";
        let v = parse_yaml(text).unwrap();
        let out = to_yaml(&v).unwrap();
        assert_eq!(parse_yaml(&out).unwrap(), v);
    }
}

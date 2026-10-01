//! `fl validate --format sarif`: the report as a SARIF 2.1.0 log, for GitHub code scanning and other
//! SARIF consumers.
//!
//! Every code in [`codes::CODES`] is a rule (with its `helpUri`), so a consumer can show rule help
//! even for codes that did not fire. Findings carry the file as a physical location and the
//! resource path (`system[x].privacy_declarations[0].data_use`) as a logical location. Manifests
//! are parsed into values without positions, so the line is best-effort: the line of the
//! resource's `fides_key` when it can be found in the file, else line 1. Findings read from stdin
//! get the artifact URI `stdin`, line 1.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{Value, json};

use super::{Diagnostic, Report, Severity, codes};

pub const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
pub const VERSION: &str = "2.1.0";

fn level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

/// Build the SARIF log. Files named in findings are re-read (best-effort) to find line numbers.
pub fn to_sarif(report: &Report) -> Value {
    let rules: Vec<Value> = codes::CODES
        .iter()
        .map(|c| {
            json!({
                "id": c.code,
                "shortDescription": { "text": c.title },
                "helpUri": codes::help_uri(c.code),
                "defaultConfiguration": { "level": level(c.severity) },
            })
        })
        .collect();
    let mut files: HashMap<String, Option<String>> = HashMap::new();
    let results: Vec<Value> = report
        .diagnostics
        .iter()
        .map(|d| {
            let text = d.file.as_ref().and_then(|f| {
                files
                    .entry(f.clone())
                    .or_insert_with(|| std::fs::read_to_string(f).ok())
                    .as_deref()
            });
            result(d, text)
        })
        .collect();
    json!({
        "$schema": SCHEMA,
        "version": VERSION,
        "runs": [{
            "tool": {
                "driver": {
                    "name": "fl",
                    "version": crate::version(),
                    "informationUri": env!("CARGO_PKG_REPOSITORY"),
                    "rules": rules,
                }
            },
            "properties": { "taxonomy": report.taxonomy },
            "results": results,
        }]
    })
}

fn result(d: &Diagnostic, file_text: Option<&str>) -> Value {
    let mut message = d.message.clone();
    if let Some(s) = &d.suggestion {
        message.push_str(&format!(" — {s}"));
    }
    let mut location = json!({
        "logicalLocations": [{ "fullyQualifiedName": d.location(), "kind": "object" }],
    });
    // Code scanning wants a physical location with a line on every result: use the resource's
    // line when it can be found, else line 1; findings read from stdin point at `stdin`.
    let (uri, line) = match &d.file {
        Some(file) => (
            file_uri(file),
            file_text
                .and_then(|t| resource_line(t, &d.resource))
                .unwrap_or(1),
        ),
        None => ("stdin".to_string(), 1),
    };
    location["physicalLocation"] = json!({
        "artifactLocation": { "uri": uri },
        "region": { "startLine": line },
    });
    let rule_index = codes::CODES
        .iter()
        .position(|c| c.code == d.code)
        .expect("every reported code is in the code table");
    json!({
        "ruleId": d.code,
        "ruleIndex": rule_index,
        "level": level(d.severity),
        "message": { "text": message },
        "locations": [location],
    })
}

/// A path as a URI reference: relative paths stay relative (resolved against the repository root
/// by GitHub code scanning), absolute paths become `file://` URIs. Characters outside the URI
/// unreserved set are percent-encoded.
pub fn file_uri(path: &str) -> String {
    let slashed = path.replace('\\', "/");
    let mut out = String::new();
    let absolute = Path::new(path).is_absolute() || slashed.starts_with('/');
    if absolute {
        out.push_str("file://");
        if !slashed.starts_with('/') {
            out.push('/'); // Windows drive letter: file:///C:/...
        }
    }
    let relative = slashed.strip_prefix("./").unwrap_or(&slashed);
    for b in relative.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(b as char)
            }
            b':' if absolute => out.push(':'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// 1-based line of the `fides_key` of `resource` (`type[key]`) in a YAML or JSON manifest: the
/// first match after the line that opens the resource type's list, else the first match anywhere.
pub fn resource_line(text: &str, resource: &str) -> Option<usize> {
    let (rtype, rest) = resource.split_once('[')?;
    let key = rest.strip_suffix(']')?;
    let lines: Vec<&str> = text.lines().collect();
    let is_key_line = |line: &str| {
        let t = line.trim_start().trim_start_matches("- ").trim_start();
        let Some(v) = t
            .strip_prefix("fides_key:")
            .or_else(|| t.strip_prefix("\"fides_key\":"))
        else {
            return false;
        };
        let v = v.trim().trim_end_matches(',').trim();
        let v = v
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| v.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(v);
        v == key
    };
    let is_type_line = |line: &str| {
        let t = line.trim_start();
        t.starts_with(&format!("{rtype}:")) || t.starts_with(&format!("\"{rtype}\":"))
    };
    let start = lines.iter().position(|l| is_type_line(l)).unwrap_or(0);
    lines[start..]
        .iter()
        .position(|l| is_key_line(l))
        .map(|i| start + i + 1)
        .or_else(|| lines.iter().position(|l| is_key_line(l)).map(|i| i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_are_encoded() {
        assert_eq!(file_uri("a/b c.yml"), "a/b%20c.yml");
        assert_eq!(file_uri("./x.yml"), "x.yml");
        assert_eq!(file_uri("/tmp/m#1.yml"), "file:///tmp/m%231.yml");
        assert_eq!(file_uri("dir\\f.yml"), "dir/f.yml");
    }

    #[test]
    fn resource_lines_are_found_in_yaml_and_json() {
        let yaml = "system:\n  - fides_key: shared\n    name: s\ndataset:\n- fides_key: other\n- fides_key: \"shared\"\n";
        assert_eq!(resource_line(yaml, "system[shared]"), Some(2));
        assert_eq!(resource_line(yaml, "dataset[shared]"), Some(6));
        assert_eq!(resource_line(yaml, "dataset[other]"), Some(5));
        assert_eq!(resource_line(yaml, "dataset[missing]"), None);
        assert_eq!(resource_line(yaml, "dataset[#0]"), None);
        let json = "{\n  \"system\": [\n    {\n      \"fides_key\": \"s1\",\n";
        assert_eq!(resource_line(json, "system[s1]"), Some(4));
    }
}

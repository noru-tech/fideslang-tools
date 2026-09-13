//! CSV support.
//!
//! * Taxonomy files use upstream's export layout: one row per record, `description` last, plus a
//!   synthetic root row (`data_category`, `Data Category`) that top-level entries hang from, so the
//!   file can be loaded straight into hierarchy visualizers.
//! * Manifests are flattened to an export-only table: datasets give one row per field, systems one
//!   row per privacy declaration, everything else one row per resource.

use anyhow::{Context, Result, bail};
use indexmap::IndexSet;
use serde_json::{Map, Value};

use crate::taxonomy::Kind;

fn cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => {
            if *b {
                "True".into()
            } else {
                "False".into()
            }
        } // matches upstream's Python export
        Value::Number(n) => n.to_string(),
        Value::Array(items) => items.iter().map(cell).collect::<Vec<_>>().join(", "),
        Value::Object(_) => v.to_string(),
    }
}

fn root_name(resource_type: &str) -> String {
    resource_type
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Serialize a single-kind taxonomy document (`{ data_use: [...] }`) the way upstream does.
fn taxonomy_to_csv(resource_type: &str, items: &[Value]) -> Result<String> {
    let mut keys: IndexSet<String> = IndexSet::new();
    for item in items {
        if let Some(m) = item.as_object() {
            keys.extend(m.keys().cloned());
        }
    }
    let mut headers: Vec<String> = keys.into_iter().collect();
    headers.sort();
    if !headers.iter().any(|h| h == "parent_key") {
        headers.push("parent_key".into());
    }
    if let Some(pos) = headers.iter().position(|h| h == "description") {
        let d = headers.remove(pos);
        headers.push(d);
    }
    let mut w = ::csv::Writer::from_writer(Vec::new());
    w.write_record(&headers)?;
    // synthetic root
    let root: Vec<String> = headers
        .iter()
        .map(|h| match h.as_str() {
            "fides_key" => resource_type.to_string(),
            "name" => root_name(resource_type),
            _ => String::new(),
        })
        .collect();
    w.write_record(&root)?;
    for item in items {
        let m = item.as_object().cloned().unwrap_or_default();
        let row: Vec<String> = headers
            .iter()
            .map(|h| {
                let v = m.get(h).unwrap_or(&Value::Null);
                if h == "parent_key" && v.is_null() {
                    resource_type.to_string()
                } else {
                    cell(v)
                }
            })
            .collect();
        w.write_record(&row)?;
    }
    Ok(String::from_utf8(w.into_inner()?)?)
}

/// Flatten a nested field list into (path, field) pairs.
fn walk_fields<'a>(
    fields: &'a [Value],
    prefix: &str,
    out: &mut Vec<(String, &'a Map<String, Value>)>,
) {
    for f in fields {
        let Some(m) = f.as_object() else { continue };
        let name = m.get("name").and_then(Value::as_str).unwrap_or("");
        let path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}.{name}")
        };
        if let Some(nested) = m.get("fields").and_then(Value::as_array) {
            walk_fields(nested, &path, out);
        }
        out.push((path, m));
    }
}

fn manifest_to_csv(map: &Map<String, Value>) -> Result<String> {
    let mut w = ::csv::Writer::from_writer(Vec::new());
    w.write_record([
        "resource_type",
        "fides_key",
        "name",
        "collection",
        "field",
        "data_categories",
        "data_use",
        "data_uses",
        "data_subjects",
        "description",
    ])?;
    let s = |m: &Map<String, Value>, k: &str| m.get(k).map(cell).unwrap_or_default();
    for (rtype, list) in map {
        let Some(items) = list.as_array() else {
            continue;
        };
        for item in items {
            let Some(r) = item.as_object() else { continue };
            let key = s(r, "fides_key");
            let name = s(r, "name");
            match rtype.as_str() {
                "dataset" => {
                    let mut any = false;
                    let empty = Vec::new();
                    for c in r
                        .get("collections")
                        .and_then(Value::as_array)
                        .unwrap_or(&empty)
                    {
                        let Some(cm) = c.as_object() else { continue };
                        let mut fields = Vec::new();
                        walk_fields(
                            cm.get("fields").and_then(Value::as_array).unwrap_or(&empty),
                            "",
                            &mut fields,
                        );
                        for (path, fm) in fields {
                            any = true;
                            w.write_record([
                                rtype.clone(),
                                key.clone(),
                                name.clone(),
                                s(cm, "name"),
                                path,
                                s(fm, "data_categories"),
                                String::new(),
                                s(fm, "data_uses"),
                                s(fm, "data_subjects"),
                                s(fm, "description"),
                            ])?;
                        }
                    }
                    if !any {
                        w.write_record([
                            rtype.clone(),
                            key,
                            name,
                            String::new(),
                            String::new(),
                            s(r, "data_categories"),
                            String::new(),
                            s(r, "data_uses"),
                            s(r, "data_subjects"),
                            s(r, "description"),
                        ])?;
                    }
                }
                "system" => {
                    let decls = r
                        .get("privacy_declarations")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    if decls.is_empty() {
                        w.write_record([
                            rtype.clone(),
                            key.clone(),
                            name,
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                            s(r, "description"),
                        ])?;
                    }
                    for d in &decls {
                        let Some(dm) = d.as_object() else { continue };
                        w.write_record([
                            rtype.clone(),
                            key.clone(),
                            s(dm, "name"),
                            String::new(),
                            String::new(),
                            s(dm, "data_categories"),
                            s(dm, "data_use"),
                            String::new(),
                            s(dm, "data_subjects"),
                            String::new(),
                        ])?;
                    }
                }
                _ => {
                    w.write_record([
                        rtype.clone(),
                        key,
                        name,
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        s(r, "description"),
                    ])?;
                }
            }
        }
    }
    Ok(String::from_utf8(w.into_inner()?)?)
}

/// Serialize a manifest- or taxonomy-shaped value to CSV.
pub fn to_csv(value: &Value) -> Result<String> {
    let Some(map) = value.as_object() else {
        bail!("CSV output needs a mapping of resource types to lists");
    };
    // A single taxonomy kind → upstream taxonomy layout; anything else → flat manifest table.
    if map.len() == 1 {
        let (rtype, list) = map.iter().next().expect("one entry");
        if Kind::from_resource_type(rtype).is_some() {
            let items = list.as_array().cloned().unwrap_or_default();
            return taxonomy_to_csv(rtype, &items);
        }
    }
    manifest_to_csv(map)
}

/// Parse a taxonomy CSV (upstream layout) back into `{ <resource_type>: [...] }`.
///
/// The synthetic root row is dropped and `parent_key` values pointing at it become `null`.
pub fn parse_taxonomy_csv(text: &str) -> Result<Value> {
    let mut rdr = ::csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());
    let headers: Vec<String> = rdr
        .headers()
        .context("reading CSV header")?
        .iter()
        .map(str::to_string)
        .collect();
    if !headers.iter().any(|h| h == "fides_key") {
        bail!("CSV has no `fides_key` column; only taxonomy CSV files can be read");
    }
    let mut rows: Vec<Map<String, Value>> = Vec::new();
    for rec in rdr.records() {
        let rec = rec.context("reading CSV row")?;
        let mut m = Map::new();
        for (h, v) in headers.iter().zip(rec.iter()) {
            let value = match (h.as_str(), v) {
                (_, "") => Value::Null,
                ("is_default", "True" | "true") => Value::Bool(true),
                ("is_default", "False" | "false") => Value::Bool(false),
                ("tags", v) => Value::Array(
                    v.split(',')
                        .map(|t| Value::String(t.trim().to_string()))
                        .collect(),
                ),
                (_, v) => Value::String(v.to_string()),
            };
            m.insert(h.clone(), value);
        }
        rows.push(m);
    }
    // Identify the root row: a key with no parent whose name is the title-cased resource type.
    let root_key = rows
        .iter()
        .find(|m| {
            m.get("parent_key").is_none_or(Value::is_null)
                && Kind::from_resource_type(
                    m.get("fides_key").and_then(Value::as_str).unwrap_or(""),
                )
                .is_some()
        })
        .and_then(|m| {
            m.get("fides_key")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    let resource_type = root_key
        .clone()
        .unwrap_or_else(|| "data_category".to_string());
    let items: Vec<Value> = rows
        .into_iter()
        .filter(|m| m.get("fides_key").and_then(Value::as_str) != root_key.as_deref())
        .map(|mut m| {
            if m.get("parent_key").and_then(Value::as_str) == root_key.as_deref() {
                m.insert("parent_key".into(), Value::Null);
            }
            Value::Object(m)
        })
        .collect();
    let mut doc = Map::new();
    doc.insert(resource_type, Value::Array(items));
    Ok(Value::Object(doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn taxonomy_csv_round_trips() {
        let doc = json!({"data_use": [
            {"fides_key": "analytics", "is_default": true, "name": "Analytics", "parent_key": null, "description": "A, b"},
            {"fides_key": "analytics.reporting", "is_default": true, "name": "Reporting", "parent_key": "analytics", "description": "R"},
        ]});
        let csv = to_csv(&doc).unwrap();
        let mut lines = csv.lines();
        assert_eq!(
            lines.next().unwrap(),
            "fides_key,is_default,name,parent_key,description"
        );
        assert_eq!(lines.next().unwrap(), "data_use,,Data Use,,");
        assert_eq!(
            lines.next().unwrap(),
            "analytics,True,Analytics,data_use,\"A, b\""
        );
        let back = parse_taxonomy_csv(&csv).unwrap();
        assert_eq!(back["data_use"][0]["fides_key"], "analytics");
        assert!(back["data_use"][0]["parent_key"].is_null());
        assert_eq!(back["data_use"][1]["parent_key"], "analytics");
        assert_eq!(back["data_use"][0]["is_default"], true);
    }

    #[test]
    fn manifest_csv_flattens_fields_and_declarations() {
        let doc = json!({
            "dataset": [{"fides_key": "ds", "collections": [{"name": "users", "fields": [
                {"name": "email", "data_categories": ["user.contact.email"]},
                {"name": "address", "fields": [{"name": "city", "data_categories": ["user.contact.city"]}]}
            ]}]}],
            "system": [{"fides_key": "sys", "privacy_declarations": [{"name": "d", "data_use": "analytics", "data_categories": ["user.contact"], "data_subjects": ["customer"]}]}]
        });
        let csv = to_csv(&doc).unwrap();
        assert!(csv.contains("dataset,ds,,users,email,user.contact.email,,,,"));
        assert!(csv.contains("dataset,ds,,users,address.city,user.contact.city,,,,"));
        assert!(csv.contains("system,sys,d,,,user.contact,analytics,,customer,"));
    }
}

//! E006 invalid fides_key · E007 schema (required fields, types, unknown resource types) ·
//! W005 unknown fields (strict only).

use serde_json::Value;

use super::{diag, is_valid_fides_key};
use crate::manifest::{Manifest, RESOURCE_TYPES, Resource};
use crate::validate::{Diagnostic, Severity};

/// Known top-level fields per resource type (upstream models, both forks), used for W005.
fn known_fields(rtype: &str) -> Option<&'static [&'static str]> {
    const TAXONOMY: &[&str] = &[
        "fides_key",
        "organization_fides_key",
        "tags",
        "name",
        "description",
        "parent_key",
        "version_added",
        "version_deprecated",
        "replaced_by",
        "is_default",
        "rights",
        "automated_decisions_or_profiling",
    ];
    const DATASET: &[&str] = &[
        "fides_key",
        "organization_fides_key",
        "tags",
        "name",
        "description",
        "meta",
        "data_categories",
        "data_uses",
        "data_purposes",
        "data_subjects",
        "fides_meta",
        "fidesops_meta",
        "collections",
    ];
    const SYSTEM: &[&str] = &[
        "fides_key",
        "organization_fides_key",
        "tags",
        "name",
        "description",
        "meta",
        "fidesctl_meta",
        "system_type",
        "egress",
        "ingress",
        "privacy_declarations",
        "administrating_department",
        "vendor_id",
        "previous_vendor_id",
        "vendor_deleted_date",
        "dataset_references",
        "processes_personal_data",
        "exempt_from_privacy_regulations",
        "reason_for_exemption",
        "uses_profiling",
        "legal_basis_for_profiling",
        "does_international_transfers",
        "legal_basis_for_transfers",
        "requires_data_protection_assessments",
        "dpa_location",
        "dpa_progress",
        "privacy_policy",
        "legal_name",
        "legal_address",
        "responsibility",
        "dpo",
        "joint_controller_info",
        "data_security_practices",
        "cookie_max_age_seconds",
        "uses_cookies",
        "cookie_refresh",
        "uses_non_cookie_access",
        "legitimate_interest_disclosure_url",
        "cookies",
    ];
    const ORGANIZATION: &[&str] = &[
        "fides_key",
        "organization_fides_key",
        "tags",
        "name",
        "description",
        "organization_parent_key",
        "controller",
        "data_protection_officer",
        "fidesctl_meta",
        "representative",
        "security_policy",
    ];
    const POLICY: &[&str] = &[
        "fides_key",
        "organization_fides_key",
        "tags",
        "name",
        "description",
        "rules",
    ];
    Some(match rtype {
        "data_category" | "data_use" | "data_subject" => TAXONOMY,
        "dataset" => DATASET,
        "system" => SYSTEM,
        "organization" => ORGANIZATION,
        "policy" => POLICY,
        _ => return None,
    })
}

fn required(rtype: &str) -> &'static [(&'static str, &'static str)] {
    // (field, expected JSON type)
    match rtype {
        "dataset" => &[("collections", "array")],
        "system" => &[("system_type", "string"), ("privacy_declarations", "array")],
        "policy" => &[("rules", "array")],
        _ => &[],
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn check_fides_key(r: &Resource, out: &mut Vec<Diagnostic>) {
    match r.value.get("fides_key") {
        None => out.push(diag(r, "E007", Severity::Error, "", "missing required field `fides_key`")),
        Some(Value::String(k)) if !is_valid_fides_key(k) => out.push(diag(r, "E006", Severity::Error, "fides_key",
            format!("invalid fides_key `{k}`: only letters, digits, `.`, `_`, `<`, `>` and `-` are allowed"))),
        Some(Value::String(_)) => {}
        Some(other) => out.push(diag(r, "E007", Severity::Error, "fides_key",
            format!("`fides_key` must be a string, found {}", type_name(other)))),
    }
}

fn check_declarations(r: &Resource, out: &mut Vec<Diagnostic>) {
    let Some(decls) = r
        .value
        .get("privacy_declarations")
        .and_then(Value::as_array)
    else {
        return;
    };
    for (i, d) in decls.iter().enumerate() {
        let p = format!("privacy_declarations[{i}]");
        let Some(m) = d.as_object() else {
            out.push(diag(
                r,
                "E007",
                Severity::Error,
                p,
                "privacy declaration must be a mapping",
            ));
            continue;
        };
        match m.get("data_use") {
            Some(Value::String(_)) => {}
            Some(other) => out.push(diag(
                r,
                "E007",
                Severity::Error,
                format!("{p}.data_use"),
                format!(
                    "`data_use` must be a single string, found {}",
                    type_name(other)
                ),
            )),
            None => out.push(diag(
                r,
                "E007",
                Severity::Error,
                format!("{p}.data_use"),
                "missing required field `data_use`",
            )),
        }
        for field in ["data_categories", "data_subjects"] {
            if let Some(v) = m.get(field)
                && !v.is_array()
            {
                out.push(diag(
                    r,
                    "E007",
                    Severity::Error,
                    format!("{p}.{field}"),
                    format!("`{field}` must be a list, found {}", type_name(v)),
                ));
            }
        }
    }
}

fn check_collections(r: &Resource, out: &mut Vec<Diagnostic>) {
    let Some(cols) = r.value.get("collections").and_then(Value::as_array) else {
        return;
    };
    for (i, c) in cols.iter().enumerate() {
        let p = format!("collections[{i}]");
        let Some(m) = c.as_object() else {
            out.push(diag(
                r,
                "E007",
                Severity::Error,
                p,
                "collection must be a mapping",
            ));
            continue;
        };
        if !m.get("name").is_some_and(Value::is_string) {
            out.push(diag(
                r,
                "E007",
                Severity::Error,
                format!("{p}.name"),
                "collection needs a string `name`",
            ));
        }
        match m.get("fields") {
            Some(Value::Array(fields)) => check_fields(r, fields, &format!("{p}.fields"), out),
            Some(other) => out.push(diag(
                r,
                "E007",
                Severity::Error,
                format!("{p}.fields"),
                format!("`fields` must be a list, found {}", type_name(other)),
            )),
            None => out.push(diag(
                r,
                "E007",
                Severity::Error,
                format!("{p}.fields"),
                "collection is missing `fields`",
            )),
        }
    }
}

fn check_fields(r: &Resource, fields: &[Value], path: &str, out: &mut Vec<Diagnostic>) {
    for (i, f) in fields.iter().enumerate() {
        let p = format!("{path}[{i}]");
        let Some(m) = f.as_object() else {
            out.push(diag(
                r,
                "E007",
                Severity::Error,
                p,
                "field must be a mapping",
            ));
            continue;
        };
        if !m.get("name").is_some_and(Value::is_string) {
            out.push(diag(
                r,
                "E007",
                Severity::Error,
                format!("{p}.name"),
                "field needs a string `name`",
            ));
        }
        if let Some(dc) = m.get("data_categories")
            && !dc.is_null()
            && !dc.is_array()
        {
            out.push(diag(
                r,
                "E007",
                Severity::Error,
                format!("{p}.data_categories"),
                format!("`data_categories` must be a list, found {}", type_name(dc)),
            ));
        }
        if let Some(Value::Array(nested)) = m.get("fields") {
            check_fields(r, nested, &format!("{p}.fields"), out);
        }
    }
}

pub fn run(manifest: &Manifest, out: &mut Vec<Diagnostic>, strict: bool) {
    for rtype in manifest.resource_types() {
        let known_type = RESOURCE_TYPES.contains(&rtype);
        for r in manifest.of_type(rtype) {
            if !known_type {
                out.push(diag(
                    r,
                    "E007",
                    Severity::Error,
                    "",
                    format!(
                        "unknown resource type `{rtype}` (expected one of {})",
                        RESOURCE_TYPES.join(", ")
                    ),
                ));
                continue;
            }
            let Some(obj) = r.value.as_object() else {
                out.push(diag(
                    r,
                    "E007",
                    Severity::Error,
                    "",
                    format!(
                        "{rtype} entry must be a mapping, found {}",
                        type_name(&r.value)
                    ),
                ));
                continue;
            };
            check_fides_key(r, out);
            for (field, expected) in required(rtype) {
                match obj.get(*field) {
                    None => out.push(diag(
                        r,
                        "E007",
                        Severity::Error,
                        *field,
                        format!("missing required field `{field}`"),
                    )),
                    Some(v) if type_name(v) != *expected => out.push(diag(
                        r,
                        "E007",
                        Severity::Error,
                        *field,
                        format!("`{field}` must be {expected}, found {}", type_name(v)),
                    )),
                    _ => {}
                }
            }
            match rtype {
                "system" => check_declarations(r, out),
                "dataset" => check_collections(r, out),
                _ => {}
            }
            if strict && let Some(known) = known_fields(rtype) {
                for k in obj.keys() {
                    if !known.contains(&k.as_str()) {
                        out.push(diag(
                            r,
                            "W005",
                            Severity::Warning,
                            k.clone(),
                            format!("unknown field `{k}` on {rtype}"),
                        ));
                    }
                }
            }
        }
    }
}

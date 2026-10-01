//! The one table of validation codes. Everything that needs the list of codes reads it from here:
//! `-W`, `--deny` / `--allow` checking, and the test that every code has a documentation page.
//!
//! Adding a rule means adding its code here; `cargo test` fails if a rule emits a code that is not
//! in this table, if the severity a rule emits differs from the one listed, or if a code has no
//! page under `docs/rules/`.

use super::Severity;

/// One stable validation code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code {
    /// `E001`
    pub code: &'static str,
    /// The severity the rule reports it with (before `--deny` / `-W`).
    pub severity: Severity,
    /// One-line title, as in `docs/rules/README.md`.
    pub title: &'static str,
}

const fn code(code: &'static str, severity: Severity, title: &'static str) -> Code {
    Code {
        code,
        severity,
        title,
    }
}

/// Every code `fl validate` can report, in order.
pub const CODES: &[Code] = &[
    code(
        "E001",
        Severity::Error,
        "A data category, data use or data subject key is not in the taxonomy",
    ),
    code(
        "E002",
        Severity::Error,
        "Two resources of the same type share a fides_key",
    ),
    code(
        "E003",
        Severity::Error,
        "A reference points at a dataset, system, collection or organization that is not loaded",
    ),
    code(
        "E004",
        Severity::Error,
        "A custom taxonomy record's parent_key is missing or is not the dotted prefix of its key",
    ),
    code(
        "E005",
        Severity::Error,
        "A custom taxonomy record names itself as its parent or replacement",
    ),
    code(
        "E006",
        Severity::Error,
        "A fides_key contains characters other than letters, digits, ., _, <, > and -",
    ),
    code(
        "E007",
        Severity::Error,
        "Structure: a required field is missing, a value has the wrong type, or the resource type is unknown",
    ),
    code("W001", Severity::Warning, "A taxonomy key is deprecated"),
    code(
        "W002",
        Severity::Warning,
        "A dataset field has no data_categories",
    ),
    code(
        "W003",
        Severity::Warning,
        "A privacy declaration has no data_categories or no data_subjects",
    ),
    code(
        "W004",
        Severity::Warning,
        "A resource uses data_purposes instead of data_uses",
    ),
    code(
        "W005",
        Severity::Warning,
        "A resource has a field the upstream models do not define (only with --strict)",
    ),
];

/// The documentation page for a code: `<DOCS_BASE>/docs/rules/<CODE>.md`.
pub fn help_uri(code: &str) -> String {
    format!("{}/docs/rules/{code}.md", crate::DOCS_BASE)
}

/// Look up a code, case-insensitively.
pub fn lookup(code: &str) -> Option<&'static Code> {
    CODES.iter().find(|c| c.code.eq_ignore_ascii_case(code))
}

/// The codes reported as warnings by default (what `-W` promotes).
pub fn warnings() -> impl Iterator<Item = &'static Code> {
    CODES.iter().filter(|c| c.severity == Severity::Warning)
}

/// `E001, E002, …` for error messages.
pub fn list() -> String {
    CODES.iter().map(|c| c.code).collect::<Vec<_>>().join(", ")
}

/// Parse a `--deny` / `--allow` value: a known code, any case, returned upper-case.
pub fn parse(s: &str) -> Result<String, String> {
    let s = s.trim();
    lookup(s)
        .map(|c| c.code.to_string())
        .ok_or_else(|| format!("unknown validation code `{s}`; valid codes: {}", list()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_sorted_unique_and_well_formed() {
        let names: Vec<&str> = CODES.iter().map(|c| c.code).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names, sorted);
        for c in CODES {
            let (prefix, digits) = c.code.split_at(1);
            assert!(digits.len() == 3 && digits.bytes().all(|b| b.is_ascii_digit()));
            let expected = if prefix == "E" {
                Severity::Error
            } else {
                assert_eq!(prefix, "W", "{}", c.code);
                Severity::Warning
            };
            assert_eq!(c.severity, expected, "{}", c.code);
            assert!(!c.title.is_empty());
        }
    }

    #[test]
    fn parse_accepts_any_case_and_rejects_unknown_codes() {
        assert_eq!(parse("w002").unwrap(), "W002");
        assert_eq!(parse("E001").unwrap(), "E001");
        let err = parse("W999").unwrap_err();
        assert!(err.contains("unknown validation code `W999`"), "{err}");
        assert!(err.contains("E001, E002"), "{err}");
        assert!(err.contains("W005"), "{err}");
    }

    #[test]
    fn help_uri_points_into_this_repository() {
        assert!(crate::DOCS_BASE.starts_with(env!("CARGO_PKG_REPOSITORY")));
        assert_eq!(
            help_uri("E001"),
            "https://github.com/noru-tech/fideslang-tools/blob/main/docs/rules/E001.md"
        );
    }

    #[test]
    fn warnings_are_the_w_codes() {
        let w: Vec<&str> = warnings().map(|c| c.code).collect();
        assert_eq!(w, ["W001", "W002", "W003", "W004", "W005"]);
    }
}

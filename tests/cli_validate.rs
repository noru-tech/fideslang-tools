mod common;

use common::{demo, fl, invalid, stdout};
use predicates::prelude::*;

fn codes(fixture: &str, extra: &[&str]) -> Vec<String> {
    let out = stdout(
        fl().args(["-q", "validate", "--format", "json"])
            .arg(invalid(fixture))
            .args(extra),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn demo_resources_text_report_snapshot() {
    let out = stdout(fl().args(["validate"]).arg(demo()));
    // Paths differ per machine; normalise the fixture directory.
    let out = out.replace(&demo().display().to_string(), "<demo>");
    insta::assert_snapshot!(out);
    fl().args(["validate"]).arg(demo()).assert().code(1);
}

#[test]
fn each_rule_fires_on_its_fixture() {
    assert_eq!(
        codes("e001_unknown_key.yml", &[]),
        vec!["E001", "E001", "E001"]
    );
    assert!(codes("e002_duplicate_key.yml", &[]).contains(&"E002".to_string()));
    assert_eq!(
        codes("e003_dangling_reference.yml", &[]),
        vec!["E003", "E003", "E003"]
    );
    assert_eq!(
        codes("e004_parent_key_mismatch.yml", &[]),
        vec!["E004", "E004"]
    );
    assert_eq!(codes("e005_self_reference.yml", &[]), vec!["E005"]);
    assert_eq!(codes("e006_invalid_fides_key.yml", &[]), vec!["E006"]);
    let e007 = codes("e007_schema.yml", &[]);
    assert!(e007.iter().all(|c| c == "E007"), "{e007:?}");
    assert!(e007.len() >= 4, "{e007:?}"); // missing system_type, data_use, categories not a list, no collections, unknown type
    let hygiene = codes("w002_w003_hygiene.yml", &[]);
    assert_eq!(hygiene, vec!["W002", "W003", "W003"]);
    assert_eq!(codes("w001_deprecated_key.yml", &[]), vec!["W001"]);
    assert_eq!(codes("w004_data_purposes.yml", &[]), vec!["W004"]);
    assert_eq!(codes("w005_unknown_field.yml", &[]), Vec::<String>::new());
    assert_eq!(codes("w005_unknown_field.yml", &["--strict"]), vec!["W005"]);
}

/// The passing examples in `docs/rules/` come from `tests/fixtures/valid/`; keep them clean.
#[test]
fn valid_fixtures_are_clean() {
    let dir = common::fixtures().join("valid");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    assert!(files.len() >= 11, "{files:?}");
    for f in files {
        let out = stdout(
            fl().args(["-q", "validate", "--strict", "--format", "json"])
                .arg(&f),
        );
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["diagnostics"], serde_json::json!([]), "{}", f.display());
        fl().args(["validate", "--strict"])
            .arg(&f)
            .assert()
            .success();
    }
}

/// The central code table (`validate::codes::CODES`) lists exactly the codes the rules emit, and
/// every code in it has a page under `docs/rules/`, listed in the index with its severity and
/// linked from the README table.
#[test]
fn every_code_has_a_docs_page() {
    use fideslang_cli::validate::codes::CODES;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Codes that appear as string literals in the rules.
    let mut emitted = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(root.join("src/validate/rules")).unwrap() {
        let src = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        for part in src.split('"').skip(1).step_by(2) {
            let b = part.as_bytes();
            if b.len() == 4
                && (b[0] == b'E' || b[0] == b'W')
                && b[1..].iter().all(u8::is_ascii_digit)
            {
                emitted.insert(part.to_string());
            }
        }
    }
    let table: std::collections::BTreeSet<String> =
        CODES.iter().map(|c| c.code.to_string()).collect();
    assert_eq!(
        emitted, table,
        "src/validate/rules and validate::codes::CODES disagree"
    );
    assert!(table.len() >= 12, "{table:?}");
    let index = std::fs::read_to_string(root.join("docs/rules/README.md")).unwrap();
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    for c in CODES {
        let code = c.code;
        assert!(
            root.join(format!("docs/rules/{code}.md")).is_file(),
            "missing docs/rules/{code}.md"
        );
        assert!(
            index.contains(&format!("| [{code}]({code}.md) | {} |", c.severity)),
            "{code} not in docs/rules/README.md as {}",
            c.severity
        );
        assert!(
            readme.contains(&format!(
                "| [{code}](docs/rules/{code}.md) | {} |",
                c.severity
            )),
            "{code} not linked from README.md as {}",
            c.severity
        );
    }
}

#[test]
fn unknown_codes_in_deny_and_allow_are_usage_errors() {
    for flag in ["--deny", "--allow"] {
        fl().args(["validate", flag, "W002,X123"])
            .arg(invalid("w004_data_purposes.yml"))
            .assert()
            .code(2)
            .stdout("")
            .stderr(predicate::str::contains("unknown validation code `X123`"))
            .stderr(predicate::str::contains(
                "valid codes: E001, E002, E003, E004, E005, E006, E007, W001, W002, W003, W004, W005",
            ));
    }
    // Known codes still work in any case.
    fl().args(["validate", "--allow", "w004"])
        .arg(invalid("w004_data_purposes.yml"))
        .assert()
        .success()
        .stdout(predicate::str::contains("0 warnings"));
}

/// `-W` promotes every warning in the code table.
#[test]
fn warnings_as_errors_covers_every_warning_code() {
    let out = stdout(
        fl().args(["-q", "validate", "-W", "--strict", "--format", "json"])
            .arg(invalid("w005_unknown_field.yml")),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["diagnostics"][0]["code"], "W005");
    assert_eq!(v["diagnostics"][0]["severity"], "error");
}

#[test]
fn suggestions_and_exit_codes() {
    fl().args(["validate"])
        .arg(invalid("e001_unknown_key.yml"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "did you mean `marketing.advertising`",
        ))
        .stdout(predicate::str::contains(
            "did you mean `user.contact.email`",
        ))
        .stdout(predicate::str::contains("did you mean `customer`"))
        .stdout(predicate::str::contains("3 errors, 0 warnings"));
    // warnings alone → exit 0; -W promotes them
    fl().args(["validate"])
        .arg(invalid("w004_data_purposes.yml"))
        .assert()
        .success();
    fl().args(["validate", "-W"])
        .arg(invalid("w004_data_purposes.yml"))
        .assert()
        .code(1);
    fl().args(["validate", "--deny", "W004"])
        .arg(invalid("w004_data_purposes.yml"))
        .assert()
        .code(1);
    fl().args(["validate", "--allow", "E003"])
        .arg(invalid("e003_dangling_reference.yml"))
        .assert()
        .success();
}

#[test]
fn github_format_emits_annotations() {
    fl().args(["-q", "validate", "--format", "github"])
        .arg(invalid("e005_self_reference.yml"))
        .assert()
        .code(1)
        .stdout(predicate::str::starts_with("::error file="))
        .stdout(predicate::str::contains(
            "title=E005 data_category[user.loop].parent_key::",
        ));
}

#[test]
fn custom_taxonomy_is_honoured_unless_disabled() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ext.yml"),
        "data_use:\n- fides_key: analytics.custom\n  name: Custom\n  parent_key: analytics\nsystem:\n- fides_key: s\n  system_type: Service\n  privacy_declarations:\n  - name: d\n    data_use: analytics.custom\n    data_categories: [user.name]\n    data_subjects: [customer]\n",
    )
    .unwrap();
    fl().args(["validate"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("OK"));
    fl().args(["validate", "--no-custom-taxonomy"])
        .arg(dir.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "unknown data use `analytics.custom`",
        ));
}

#[test]
fn keys_outside_the_iab_taxonomy_are_rejected() {
    // `functional.storage.privacy_preferences` exists only in a downstream fork, not in the
    // IAB Tech Lab 3.0.0 taxonomy that fl bundles.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("s.yml"),
        "system:\n- fides_key: s\n  system_type: Service\n  privacy_declarations:\n  - name: d\n    data_use: functional.storage.privacy_preferences\n    data_categories: [user.name]\n    data_subjects: [customer]\n",
    )
    .unwrap();
    fl().args(["validate"])
        .arg(dir.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "unknown data use `functional.storage.privacy_preferences`",
        ))
        .stdout(predicate::str::contains(
            "did you mean `functional.storage`",
        ));
}

#[test]
fn stats_text_and_json() {
    let out = stdout(fl().args(["stats"]).arg(demo()).args(["--top", "3"]));
    insta::assert_snapshot!(out);
    let json = stdout(
        fl().args(["stats", "--format", "json", "--rollup"])
            .arg(demo()),
    );
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["datasets"]["fields"], 6);
    assert_eq!(
        v["datasets"]["uncategorized"][0],
        "demo_users_dataset.users.food_preference"
    );
    assert_eq!(v["rollup"]["data_category"]["user"], 6);
}

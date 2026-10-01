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

const DOCS: &str = "https://github.com/noru-tech/fideslang-tools/blob/main/docs/rules";

#[test]
fn json_has_schema_version_and_help_uris() {
    let out = stdout(
        fl().args(["-q", "validate", "--format", "json"])
            .arg(invalid("e001_unknown_key.yml")),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["schema_version"], 1);
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "schema_version",
            "taxonomy",
            "files",
            "resources",
            "diagnostics"
        ]
    );
    for d in v["diagnostics"].as_array().unwrap() {
        assert_eq!(d["help_uri"], format!("{DOCS}/E001.md"));
        // The existing fields are unchanged.
        for f in ["code", "severity", "file", "resource", "path", "message"] {
            assert!(d.get(f).is_some(), "{f} missing in {d}");
        }
    }
}

#[test]
fn yaml_format_matches_json() {
    let fixture = invalid("w002_w003_hygiene.yml");
    let json = stdout(
        fl().args(["-q", "validate", "--format", "json"])
            .arg(&fixture),
    );
    let yaml = stdout(
        fl().args(["-q", "validate", "--format", "yaml"])
            .arg(&fixture),
    );
    assert!(yaml.starts_with("schema_version: 1\n"), "{yaml}");
    let json = fideslang_cli::format::to_string(
        &serde_json::from_str(&json).unwrap(),
        fideslang_cli::format::Format::Yaml,
    )
    .unwrap();
    assert_eq!(yaml, json);
    fl().args(["validate", "--format", "yaml"])
        .arg(&fixture)
        .assert()
        .success();
    fl().args(["validate", "--format", "yaml"])
        .arg(invalid("e005_self_reference.yml"))
        .assert()
        .code(1);
}

#[test]
fn text_links_each_code_once() {
    let out = stdout(fl().args(["validate"]).arg(demo()));
    let see: Vec<&str> = out.lines().filter(|l| l.starts_with("see ")).collect();
    assert_eq!(
        see,
        [format!("see {DOCS}/E001.md"), format!("see {DOCS}/W002.md")]
    );
    // The summary stays the last line; a clean run prints no links.
    assert!(out.lines().last().unwrap().contains("6 errors, 1 warning"));
    let clean = stdout(
        fl().args(["validate"])
            .arg(common::fixtures().join("valid/e001_known_keys.yml")),
    );
    assert!(!clean.contains("see "), "{clean}");
}

#[test]
fn github_annotations_link_the_rule_page() {
    let out = stdout(
        fl().args(["-q", "validate", "--format", "github"])
            .arg(invalid("e001_unknown_key.yml")),
    );
    let first = out.lines().next().unwrap();
    // Properties escape `:` and `,`; the message keeps them readable.
    assert!(first.starts_with("::error file="), "{first}");
    assert!(
        first.contains(
            "e001_unknown_key.yml,title=E001 system[bad_keys_system].privacy_declarations[0].data_categories[0]::"
        ),
        "{first}"
    );
    assert!(
        first.ends_with(&format!(
            "(did you mean `user.contact.email`, `user.contact.url`, `user.contact.fax_number`?) — see {DOCS}/E001.md"
        )),
        "{first}"
    );
    assert_eq!(out.lines().count(), 3);
}

/// Structural check of `--format sarif` against the parts of the SARIF 2.1.0 schema that consumers
/// such as GitHub code scanning rely on. (The output was also checked against the official JSON
/// schema with ajv; see the PR.)
#[test]
fn sarif_output_is_a_valid_sarif_log() {
    use fideslang_cli::validate::codes::CODES;
    let out = fl()
        .args(["-q", "validate", "--format", "sarif"])
        .arg(demo())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["version"], "2.1.0");
    assert_eq!(
        v["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    let runs = v["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    let driver = &runs[0]["tool"]["driver"];
    assert_eq!(driver["name"], "fl");
    assert_eq!(driver["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        driver["informationUri"],
        "https://github.com/noru-tech/fideslang-tools"
    );
    let rules = driver["rules"].as_array().unwrap();
    assert_eq!(rules.len(), CODES.len());
    for (rule, code) in rules.iter().zip(CODES) {
        assert_eq!(rule["id"], code.code);
        assert_eq!(rule["helpUri"], format!("{DOCS}/{}.md", code.code));
        assert_eq!(rule["shortDescription"]["text"], code.title);
        assert_eq!(
            rule["defaultConfiguration"]["level"],
            code.severity.to_string()
        );
    }
    let results = runs[0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 7);
    for r in results {
        let id = r["ruleId"].as_str().unwrap();
        let idx = r["ruleIndex"].as_u64().unwrap() as usize;
        assert_eq!(rules[idx]["id"], id);
        assert!(matches!(r["level"].as_str().unwrap(), "error" | "warning"));
        assert!(!r["message"]["text"].as_str().unwrap().is_empty());
        let loc = &r["locations"][0];
        let uri = loc["physicalLocation"]["artifactLocation"]["uri"]
            .as_str()
            .unwrap();
        assert!(!uri.contains(' ') && !uri.contains('\\'), "{uri}");
        assert!(
            loc["physicalLocation"]["region"]["startLine"]
                .as_u64()
                .unwrap()
                >= 1,
            "{r}"
        );
        assert!(
            loc["logicalLocations"][0]["fullyQualifiedName"]
                .as_str()
                .unwrap()
                .contains('[')
        );
        // No property outside what SARIF defines for a result.
        for k in r.as_object().unwrap().keys() {
            assert!(
                ["ruleId", "ruleIndex", "level", "message", "locations"].contains(&k.as_str()),
                "{k}"
            );
        }
    }
    // The line points at the resource's fides_key.
    let w002 = results.iter().find(|r| r["ruleId"] == "W002").unwrap();
    let uri = w002["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
        .as_str()
        .unwrap();
    assert!(uri.ends_with("demo_resources/demo_dataset.yml"), "{uri}");
    let line = w002["locations"][0]["physicalLocation"]["region"]["startLine"]
        .as_u64()
        .unwrap() as usize;
    let text = std::fs::read_to_string(demo().join("demo_dataset.yml")).unwrap();
    assert!(
        text.lines()
            .nth(line - 1)
            .unwrap()
            .contains("fides_key: demo_users_dataset"),
        "line {line}"
    );
    // A resource whose fides_key line cannot be found (no fides_key) falls back to line 1.
    let dir = tempfile::tempdir().unwrap();
    let nokey = dir.path().join("nokey.yml");
    std::fs::write(
        &nokey,
        "# comment\nsystem:\n- name: no key\n  system_type: Service\n",
    )
    .unwrap();
    let out = stdout(
        fl().args(["-q", "validate", "--format", "sarif"])
            .arg(&nokey),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let r = &v["runs"][0]["results"][0];
    assert_eq!(r["ruleId"], "E007");
    assert_eq!(
        r["locations"][0]["physicalLocation"]["region"]["startLine"],
        1
    );
    assert!(
        r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
            .as_str()
            .unwrap()
            .ends_with("/nokey.yml")
    );
    // Promoted warnings are reported at their effective level; stdin points at `stdin`, line 1.
    let out = fl()
        .args(["-q", "validate", "--format", "sarif", "-W", "-"])
        .write_stdin(std::fs::read(invalid("w004_data_purposes.yml")).unwrap())
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let r = &v["runs"][0]["results"][0];
    assert_eq!(r["level"], "error");
    let physical = &r["locations"][0]["physicalLocation"];
    assert_eq!(physical["artifactLocation"]["uri"], "stdin");
    assert_eq!(physical["region"]["startLine"], 1);
    assert!(
        r["locations"][0]["logicalLocations"][0]["fullyQualifiedName"]
            .as_str()
            .unwrap()
            .starts_with("dataset[legacy_purposes]")
    );
    assert_eq!(out.status.code(), Some(1));
}

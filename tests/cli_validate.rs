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
    assert_eq!(codes("w004_data_purposes.yml", &[]), vec!["W004"]);
    assert_eq!(codes("w005_unknown_field.yml", &[]), Vec::<String>::new());
    assert_eq!(codes("w005_unknown_field.yml", &["--strict"]), vec!["W005"]);
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
fn iab_snapshot_rejects_ethyca_only_key() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("s.yml"),
        "system:\n- fides_key: s\n  system_type: Service\n  privacy_declarations:\n  - name: d\n    data_use: functional.storage.privacy_preferences\n    data_categories: [user.name]\n    data_subjects: [customer]\n",
    )
    .unwrap();
    fl().args(["validate"]).arg(dir.path()).assert().success();
    fl().args(["--taxonomy", "iab", "validate"])
        .arg(dir.path())
        .assert()
        .code(1);
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

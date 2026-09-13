mod common;

use common::{fl, stdout};
use predicates::prelude::*;

#[test]
fn info_describes_the_bundled_snapshot() {
    fl().args(["taxonomy", "info"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "IAB Tech Lab Privacy Taxonomy (fideslang 3.0.0)",
        ))
        .stdout(predicate::str::contains(
            "https://github.com/IABTechLab/fideslang",
        ))
        .stdout(predicate::str::contains(
            "85 categories, 55 uses, 15 subjects",
        ));
}

#[test]
fn info_json_matches_snapshot_file() {
    let out = stdout(fl().args(["taxonomy", "info", "--format", "json"]));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let recorded: serde_json::Value =
        serde_json::from_str(include_str!("../taxonomy/snapshot.json")).unwrap();
    assert_eq!(v, recorded);
}

#[test]
fn tree_depth_two_snapshot() {
    let out = stdout(fl().args(["-q", "taxonomy", "tree", "categories", "--depth", "2"]));
    insta::assert_snapshot!(out);
}

#[test]
fn tree_ascii_with_root_and_descriptions() {
    let out = stdout(fl().args([
        "-q",
        "taxonomy",
        "tree",
        "uses",
        "--root",
        "analytics",
        "--ascii",
        "--descriptions",
    ]));
    insta::assert_snapshot!(out);
}

#[test]
fn tree_dot_and_mermaid_and_json() {
    let dot = stdout(fl().args(["-q", "taxonomy", "tree", "subjects", "--format", "dot"]));
    insta::assert_snapshot!("tree_subjects_dot", dot);
    let mmd = stdout(fl().args([
        "-q",
        "taxonomy",
        "tree",
        "categories",
        "--root",
        "system",
        "--format",
        "mermaid",
    ]));
    insta::assert_snapshot!("tree_system_mermaid", mmd);
    let json = stdout(fl().args([
        "-q",
        "taxonomy",
        "tree",
        "categories",
        "--root",
        "user.contact",
        "--depth",
        "2",
        "--format",
        "json",
    ]));
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["roots"][0]["fides_key"], "user.contact");
    assert!(v["roots"][0]["children"].as_array().unwrap().len() > 3);
    assert!(
        v["roots"][0]["children"][0]["children"]
            .as_array()
            .unwrap()
            .is_empty(),
        "depth 2 from the root cuts grandchildren"
    );
}

#[test]
fn show_text_and_json() {
    let out = stdout(fl().args(["taxonomy", "show", "user.contact.email"]));
    insta::assert_snapshot!(out);
    let json = stdout(fl().args(["taxonomy", "show", "user.contact", "--format", "json"]));
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["ancestors"], serde_json::json!(["user"]));
    assert!(
        v["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "user.contact.email")
    );
}

#[test]
fn show_unknown_key_suggests() {
    fl().args(["taxonomy", "show", "user.contact.emial"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "did you mean `user.contact.email`",
        ));
}

#[test]
fn search_substring_and_regex() {
    fl().args(["-q", "taxonomy", "search", "cookie", "--keys-only"])
        .assert()
        .success()
        .stdout(predicate::str::contains("user.device.cookie_id"))
        .stdout(predicate::str::contains(
            "user.device.cookie  Device Cookie",
        ));
    let out = stdout(fl().args([
        "-q",
        "taxonomy",
        "search",
        "^functional\\.storage",
        "--regex",
        "--format",
        "json",
    ]));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.as_array().unwrap().iter().all(|h| {
        h["fides_key"]
            .as_str()
            .unwrap()
            .starts_with("functional.storage")
    }));
}

#[test]
fn diff_against_manifest_custom_taxonomy() {
    let out = stdout(fl().args(["taxonomy", "diff"]).arg(common::demo()));
    // Paths differ per machine; normalise the fixture directory.
    insta::assert_snapshot!(out.replace(&common::demo().display().to_string(), "<demo>"));
    assert!(out.contains("+ third_party_sharing.personalized_advertising.direct_marketing"));
    assert!(out.contains("+ potential_customer"));
    assert!(out.contains("2 added, 0 removed, 0 changed"));
    fl().args(["taxonomy", "diff", "--exit-code"])
        .arg(common::demo())
        .assert()
        .code(1);
    // A manifest set with no custom taxonomy has no differences.
    fl().args(["taxonomy", "diff", "--exit-code"])
        .arg(common::demo().join("demo_system.yml"))
        .assert()
        .success()
        .stdout(predicate::str::contains("0 added, 0 removed, 0 changed"));
}

#[test]
fn list_formats() {
    let plain = stdout(fl().args(["-q", "taxonomy", "list", "subjects", "--format", "plain"]));
    assert_eq!(plain.lines().count(), 15);
    let csv = stdout(fl().args(["-q", "taxonomy", "list", "uses", "--format", "csv"]));
    assert!(csv.contains("fides_key"));
    let yaml = stdout(fl().args([
        "-q",
        "taxonomy",
        "list",
        "categories",
        "--prefix",
        "user.contact",
        "--format",
        "yaml",
    ]));
    assert!(yaml.starts_with("data_category:\n"));
    assert!(yaml.contains("fides_key: user.contact.email"));
    assert!(!yaml.contains("fides_key: user.name"));
}

#[test]
fn cat_yaml_is_byte_exact_and_csv_has_root_row() {
    let yaml = stdout(fl().args(["taxonomy", "cat", "subjects"]));
    assert_eq!(yaml, include_str!("../taxonomy/data_subjects.yml"));
    let csv = stdout(fl().args(["taxonomy", "cat", "uses", "--format", "csv"]));
    let mut lines = csv.lines();
    let header = lines.next().unwrap();
    assert!(header.starts_with("fides_key,"));
    assert!(header.ends_with(",description"));
    assert!(lines.next().unwrap().starts_with("data_use,,Data Use,"));
}

#[test]
fn completions_and_manpage() {
    fl().args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_fl"));
    fl().args(["manpage"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".TH fl"));
    let dir = tempfile::tempdir().unwrap();
    fl().args(["manpage", "--out-dir"])
        .arg(dir.path())
        .assert()
        .success();
    assert!(dir.path().join("fl-taxonomy-tree.1").exists());
}

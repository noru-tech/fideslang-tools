mod common;

use std::fs;

use common::{demo, fl, stdout};
use predicates::prelude::*;

#[test]
fn cat_filters_and_formats() {
    let out = stdout(
        fl().args(["-q", "cat"])
            .arg(demo())
            .args(["--type", "system", "--format", "json"]),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["system"].as_array().unwrap().len(), 2);
    assert!(v.get("dataset").is_none());

    let out = stdout(
        fl().args(["-q", "cat"])
            .arg(demo())
            .args(["--key", "demo_marketing_*"]),
    );
    assert!(out.contains("fides_key: demo_marketing_system"));
    assert!(!out.contains("demo_analytics_system"));

    let tree = stdout(fl().args(["cat"]).arg(demo()).args(["--format", "tree"]));
    insta::assert_snapshot!(tree);
}

#[test]
fn cat_reads_stdin_and_reports_bad_yaml() {
    fl().args(["-q", "cat", "-", "--format", "json"])
        .write_stdin("system:\n- fides_key: from_stdin\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"fides_key\": \"from_stdin\""));
    fl().args(["cat", "-"])
        .write_stdin("a: [1, 2\nb: 3\n")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error"));
    fl().args(["cat", "/nonexistent/path"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no such file"));
}

#[test]
fn convert_round_trips_yaml_json_yaml() {
    let json = stdout(
        fl().args(["convert"])
            .arg(demo().join("demo_system.yml"))
            .args(["--to", "json"]),
    );
    let via_json: serde_json::Value = serde_json::from_str(&json).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("system.json");
    fs::write(&json_path, &json).unwrap();
    let yaml = stdout(
        fl().args(["convert"])
            .arg(&json_path)
            .args(["--to", "yaml"]),
    );
    let back = stdout(
        fl().args(["-q", "cat", "-", "--format", "json"])
            .write_stdin(yaml),
    );
    let via_yaml: serde_json::Value = serde_json::from_str(&back).unwrap();
    assert_eq!(via_json, via_yaml);
}

#[test]
fn convert_infers_target_from_output_extension() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("policy.json");
    fl().args(["convert"])
        .arg(demo().join("demo_policy.yml"))
        .arg("-o")
        .arg(&out)
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(v["policy"][0]["fides_key"], "demo_privacy_policy");
}

#[test]
fn convert_taxonomy_to_csv_and_back() {
    let csv = stdout(
        fl().args(["convert"])
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/taxonomy/data_uses.yml"
            ))
            .args(["--to", "csv"]),
    );
    let mut lines = csv.lines();
    assert_eq!(
        lines.next().unwrap(),
        "fides_key,is_default,name,organization_fides_key,parent_key,replaced_by,tags,version_added,version_deprecated,description"
    );
    assert_eq!(lines.next().unwrap(), "data_use,,Data Use,,,,,,,");
    assert!(
        lines
            .next()
            .unwrap()
            .starts_with("analytics,True,Analytics,default_organization,data_use,,,2.0.0,,")
    );
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("uses.csv");
    fs::write(&p, &csv).unwrap();
    let yaml = stdout(fl().args(["convert"]).arg(&p).args(["--to", "yaml"]));
    assert!(yaml.starts_with("data_use:\n- fides_key: analytics\n"));
    assert!(yaml.contains("parent_key: analytics\n"));
}

#[test]
fn convert_manifest_to_csv_flattens() {
    let csv = stdout(
        fl().args(["convert"])
            .arg(demo().join("demo_dataset.yml"))
            .args(["--to", "csv"]),
    );
    assert!(
        csv.lines()
            .next()
            .unwrap()
            .starts_with("resource_type,fides_key,name,collection,field,data_categories")
    );
    assert!(csv.contains("dataset,demo_users_dataset,Demo Users Dataset,users,email,user.contact.email,,,,User's Email"));
}

#[test]
fn merge_and_split_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let all = dir.path().join("all.yml");
    fl().args(["merge"])
        .arg(demo())
        .arg("-o")
        .arg(&all)
        .assert()
        .success()
        .stderr(predicate::str::contains("merged 5 files"))
        .stderr(predicate::str::contains("dataset 1, system 2, policy 1"));
    let merged: serde_json::Value = serde_json::from_str(&stdout(
        fl().args(["-q", "cat", "--format", "json"]).arg(&all),
    ))
    .unwrap();
    let keys: Vec<&str> = merged
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec![
            "organization",
            "data_use",
            "data_subject",
            "dataset",
            "system",
            "policy"
        ]
    );

    let split_dir = dir.path().join("split");
    fl().args(["split"])
        .arg(&all)
        .arg("--out-dir")
        .arg(&split_dir)
        .assert()
        .success();
    let mut files: Vec<String> = fs::read_dir(&split_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(
        files,
        vec![
            "data_subject.yml",
            "data_use.yml",
            "dataset.yml",
            "organization.yml",
            "policy.yml",
            "system.yml"
        ]
    );

    let re_merged: serde_json::Value = serde_json::from_str(&stdout(
        fl().args(["-q", "merge", "--format", "json"])
            .arg(&split_dir),
    ))
    .unwrap();
    assert_eq!(re_merged, merged);

    let per_resource = dir.path().join("per");
    fl().args(["split"])
        .arg(&all)
        .args(["--by", "resource", "--format", "json", "--out-dir"])
        .arg(&per_resource)
        .assert()
        .success();
    assert!(
        per_resource
            .join("system/demo_marketing_system.json")
            .exists()
    );
    fl().args(["split"])
        .arg(&all)
        .arg("--out-dir")
        .arg(&split_dir)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--force"));
}

#[test]
fn merge_duplicate_handling() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.yml"), "system:\n- fides_key: s\n  name: first\n  system_type: Service\n  privacy_declarations: []\n").unwrap();
    fs::write(dir.path().join("b.yml"), "system:\n- fides_key: s\n  name: second\n  system_type: Service\n  privacy_declarations: []\n").unwrap();
    fl().args(["merge", "--fail-on-duplicate"])
        .arg(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("duplicate fides_keys: system[s]"));
    let out = stdout(
        fl().args(["-q", "merge", "--dedupe", "--format", "json"])
            .arg(dir.path()),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["system"].as_array().unwrap().len(), 1);
    assert_eq!(v["system"][0]["name"], "second");
}

#[test]
fn default_path_is_dot_fides() {
    let dir = tempfile::tempdir().unwrap();
    fl().current_dir(dir.path())
        .args(["cat"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(".fides/"));
    fs::create_dir(dir.path().join(".fides")).unwrap();
    fs::write(
        dir.path().join(".fides/s.yml"),
        "system:\n- fides_key: here\n",
    )
    .unwrap();
    fl().current_dir(dir.path())
        .args(["-q", "cat", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"here\""));
}

mod common;

use common::{demo, fl, stdout};
use predicates::prelude::*;

#[test]
fn dot_snapshot_and_validity() {
    let dot = stdout(
        fl().args(["-q", "graph"])
            .arg(demo())
            .args(["--include", "uses,categories,subjects"]),
    );
    insta::assert_snapshot!(dot);
    assert!(dot.starts_with("digraph"));
    // Every referenced edge endpoint is a declared node.
    let ids: Vec<&str> = dot
        .lines()
        .filter(|l| l.contains("[label="))
        .map(|l| l.trim().split(' ').next().unwrap())
        .collect();
    for line in dot.lines().filter(|l| l.contains(" -> ")) {
        let (from, rest) = line.trim().split_once(" -> ").unwrap();
        let to = rest.split(' ').next().unwrap();
        assert!(ids.contains(&from), "{from} undeclared");
        assert!(ids.contains(&to), "{to} undeclared");
    }
    // If graphviz is installed, make sure it parses.
    if let Ok(mut child) = std::process::Command::new("dot")
        .args(["-Tsvg", "-o", "/dev/null"])
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(dot.as_bytes())
            .unwrap();
        let status = child.wait().unwrap();
        assert!(status.success(), "graphviz rejected the DOT output");
    }
}

#[test]
fn mermaid_snapshot() {
    let mmd = stdout(fl().args(["-q", "graph"]).arg(demo()).args([
        "--format",
        "mermaid",
        "--include",
        "fields,uses",
        "--rankdir",
        "tb",
    ]));
    insta::assert_snapshot!(mmd);
    assert!(mmd.starts_with("flowchart TB\n"));
}

#[test]
fn bare_focus_and_json() {
    let json = stdout(
        fl().args(["-q", "graph", "--bare", "--format", "json"])
            .arg(demo()),
    );
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let kinds: Vec<&str> = v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["kind"].as_str().unwrap())
        .collect();
    assert!(
        kinds.iter().all(|k| *k == "system" || *k == "dataset"),
        "{kinds:?}"
    );

    fl().args([
        "graph",
        "--focus",
        "demo_marketing_system",
        "--depth",
        "1",
        "--format",
        "json",
    ])
    .arg(demo())
    .assert()
    .success()
    .stdout(predicate::str::contains("demo_marketing_system"))
    .stdout(predicate::str::contains("advertising"))
    .stdout(predicate::str::contains("demo_users_dataset").not());
    fl().args(["graph", "--focus", "nope"])
        .arg(demo())
        .assert()
        .code(2);
}

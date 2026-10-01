mod common;

use assert_cmd::Command;
use common::{demo, fl};
use predicates::prelude::*;
use serde_json::Value;

/// `fl` with a clean color environment and no color flag.
fn fl_raw() -> Command {
    let mut cmd = Command::cargo_bin("fl").expect("fl binary");
    for v in ["NO_COLOR", "FL_COLOR", "CLICOLOR_FORCE", "CLICOLOR"] {
        cmd.env_remove(v);
    }
    cmd
}

fn json(cmd: &mut Command) -> Value {
    let out = cmd.args(["doctor", "--format", "json"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn doctor_text_reports_everything_and_exits_0() {
    let empty = tempfile::tempdir().unwrap();
    fl().current_dir(empty.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(format!(
            "fl           {}\n",
            env!("CARGO_PKG_VERSION")
        )))
        .stdout(predicate::str::contains(
            "taxonomy     iab 3.0.0 (85 data categories, 55 data uses, 15 data subjects; commit c53726c9d9ee, 2026-09-13)",
        ))
        .stdout(predicate::str::contains("color        off (--color never)"))
        .stdout(predicate::str::contains(
            ".fides       ./.fides/ not found (manifest commands then need a path)",
        ))
        .stdout(predicate::str::contains("bash compl.  "))
        .stdout(predicate::str::contains("man page     "))
        .stdout(predicate::str::contains(
            "updates      fl never contacts the network and never checks for updates",
        ));
}

#[test]
fn doctor_counts_manifests_in_dot_fides() {
    let dir = tempfile::tempdir().unwrap();
    let fides = dir.path().join(".fides");
    std::fs::create_dir(&fides).unwrap();
    for e in std::fs::read_dir(demo()).unwrap() {
        let p = e.unwrap().path();
        std::fs::copy(&p, fides.join(p.file_name().unwrap())).unwrap();
    }
    let v = json(fl().current_dir(dir.path()));
    assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(v["taxonomy"]["label"], "iab 3.0.0");
    assert_eq!(v["taxonomy"]["data_categories"], 85);
    assert_eq!(v["taxonomy"]["data_uses"], 55);
    assert_eq!(v["taxonomy"]["data_subjects"], 15);
    assert_eq!(v["fides_dir"]["exists"], true);
    assert_eq!(v["fides_dir"]["manifests"], 5);
    assert_eq!(v["fides_dir"]["resources"], 7);
    assert!(v["fides_dir"].get("error").is_none());
    assert!(v["updates"].as_str().unwrap().contains("brew upgrade fl"));

    // A broken manifest is reported, not fatal.
    std::fs::write(fides.join("zz_broken.yml"), "system: [unclosed\n").unwrap();
    let v = json(fl().current_dir(dir.path()));
    assert_eq!(v["fides_dir"]["manifests"], 6);
    assert!(v["fides_dir"].get("resources").is_none());
    assert!(
        v["fides_dir"]["error"]
            .as_str()
            .unwrap()
            .contains("zz_broken.yml"),
        "{v}"
    );

    let empty = tempfile::tempdir().unwrap();
    let v = json(fl().current_dir(empty.path()));
    assert_eq!(v["fides_dir"]["exists"], false);
    assert!(v["fides_dir"].get("manifests").is_none());
}

#[test]
fn doctor_finds_installed_completions_and_man_page() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let put = |rel: &str| {
        let p = h.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "x").unwrap();
        p.display().to_string()
    };
    let bash = put(".local/share/bash-completion/completions/fl");
    let zsh = put(".zfunc/_fl");
    let fish = put(".config/fish/completions/fl.fish");
    let man = put("man/man1/fl.1");
    let v = json(
        fl().env("HOME", h)
            .env_remove("XDG_DATA_HOME")
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("FPATH")
            .env("MANPATH", h.join("man")),
    );
    assert_eq!(v["completions"]["bash"], bash);
    assert_eq!(v["completions"]["zsh"], zsh);
    assert_eq!(v["completions"]["fish"], fish);
    assert_eq!(v["man_page"], man);
}

#[test]
fn doctor_explains_the_color_decision() {
    let reason = |cmd: &mut Command| {
        let v = json(cmd);
        (
            v["color"]["enabled"].as_bool().unwrap(),
            v["color"]["reason"].as_str().unwrap().to_string(),
        )
    };
    // Tests run with stdout captured, so `auto` sees no terminal.
    assert_eq!(
        reason(&mut fl_raw()),
        (
            false,
            "--color auto (default): stdout is not a terminal".into()
        )
    );
    assert_eq!(
        reason(fl_raw().args(["--color", "always"])),
        (true, "--color always".into())
    );
    assert_eq!(
        reason(fl_raw().env("FL_COLOR", "always")),
        (true, "FL_COLOR=always".into())
    );
    assert_eq!(
        reason(fl_raw().env("FL_COLOR", "always").arg("--no-color")),
        (false, "--no-color".into())
    );
    assert_eq!(
        reason(fl_raw().env("NO_COLOR", "1").env("CLICOLOR_FORCE", "1")),
        (false, "--color auto (default): NO_COLOR is set".into())
    );
    assert_eq!(
        reason(fl_raw().env("CLICOLOR_FORCE", "1")),
        (true, "--color auto (default): CLICOLOR_FORCE is set".into())
    );
    assert_eq!(
        reason(fl_raw().args(["--color", "auto"]).env("CLICOLOR", "0")),
        (false, "--color auto: CLICOLOR=0".into())
    );
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("doctor.json");
    fl_raw()
        .args(["--color", "always", "doctor", "--format", "json", "-o"])
        .arg(&out)
        .assert()
        .success()
        .stdout("");
    let v: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(v["color"]["enabled"], false);
    assert_eq!(
        v["color"]["reason"],
        "output goes to a file (-o), which never gets colors"
    );
}

#[test]
fn doctor_yaml_output() {
    let empty = tempfile::tempdir().unwrap();
    let out = fl()
        .current_dir(empty.path())
        .args(["doctor", "--format", "yaml"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let yaml = String::from_utf8(out.stdout).unwrap();
    let first = yaml.lines().next().unwrap();
    assert!(first.starts_with("version: "), "{first}");
    assert!(first.contains(env!("CARGO_PKG_VERSION")), "{first}");
    assert!(yaml.contains("fides_dir:\n  path: ./.fides/\n  exists: false\n"));
}

//! Global flags and process behaviour shared by every command.

mod common;

use std::io::Read;
use std::process::{Command as StdCommand, Stdio};

use assert_cmd::Command;
use common::{demo, fl, invalid};
use predicates::prelude::*;

/// `fl` with no color flags and a clean color environment.
fn fl_raw() -> Command {
    let mut cmd = Command::cargo_bin("fl").expect("fl binary");
    cmd.env_remove("NO_COLOR")
        .env_remove("FL_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("CLICOLOR");
    cmd
}

fn has_ansi(cmd: &mut Command) -> bool {
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "{out:?}");
    out.stdout.contains(&0x1b)
}

#[test]
fn no_color_flag_turns_color_off() {
    let show = ["taxonomy", "show", "user"];
    // Sanity: these do color.
    assert!(has_ansi(fl_raw().args(["--color", "always"]).args(show)));
    assert!(has_ansi(fl_raw().env("FL_COLOR", "always").args(show)));
    assert!(has_ansi(fl_raw().env("CLICOLOR_FORCE", "1").args(show)));
    // --no-color wins over all of them, in any position.
    assert!(!has_ansi(fl_raw().args(["--no-color"]).args(show)));
    assert!(!has_ansi(
        fl_raw()
            .args(["--no-color", "--color", "always"])
            .args(show)
    ));
    assert!(!has_ansi(
        fl_raw()
            .env("FL_COLOR", "always")
            .args(show)
            .arg("--no-color")
    ));
    assert!(!has_ansi(
        fl_raw()
            .env("CLICOLOR_FORCE", "1")
            .arg("--no-color")
            .args(show)
    ));
    // NO_COLOR keeps working without the flag.
    assert!(!has_ansi(
        fl_raw()
            .env("NO_COLOR", "1")
            .env("CLICOLOR_FORCE", "1")
            .args(show)
    ));
}

#[test]
fn verbose_writes_to_stderr_only() {
    let cases: Vec<Vec<std::ffi::OsString>> = vec![
        vec!["validate".into(), demo().into()],
        vec!["stats".into(), demo().into()],
        vec![
            "cat".into(),
            demo().into(),
            "--type".into(),
            "system".into(),
        ],
        vec!["graph".into(), demo().into()],
        vec!["taxonomy".into(), "diff".into(), demo().into()],
        vec!["taxonomy".into(), "list".into(), "uses".into()],
    ];
    for args in cases {
        let plain = fl().args(&args).output().unwrap();
        let verbose = fl().arg("-v").args(&args).output().unwrap();
        let long = fl().args(&args).arg("--verbose").output().unwrap();
        assert_eq!(plain.stdout, verbose.stdout, "{args:?}");
        assert_eq!(plain.stdout, long.stdout, "{args:?}");
        assert_eq!(plain.status.code(), verbose.status.code(), "{args:?}");
        let plain_err = String::from_utf8(plain.stderr).unwrap();
        let verbose_err = String::from_utf8(verbose.stderr).unwrap();
        assert!(!plain_err.contains("verbose:"), "{args:?}: {plain_err}");
        assert!(
            verbose_err.contains("verbose: taxonomy iab 3.0.0: 85 data categories"),
            "{args:?}: {verbose_err}"
        );
        assert!(verbose_err.contains("verbose: exit "), "{args:?}");
        if args[0] != "taxonomy" || args[1] == "diff" {
            assert!(
                verbose_err.contains("demo_system.yml (2 resources)"),
                "{args:?}: {verbose_err}"
            );
            assert!(
                verbose_err.contains("5 files, 7 resources loaded in"),
                "{args:?}: {verbose_err}"
            );
        }
    }
    // -q silences summaries but not -v diagnostics.
    fl().args(["-q", "-v", "validate"])
        .arg(demo())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("verbose: loaded"));
}

#[test]
fn missing_path_explains_how_to_fix_it() {
    fl().args(["validate", "nope.yml"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "error: nope.yml: no such file or directory",
        ))
        .stderr(predicate::str::contains(
            "hint: pass a file or directory, e.g. fl validate path/to/manifests",
        ));
    let empty = tempfile::tempdir().unwrap();
    fl().current_dir(empty.path())
        .args(["stats"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "no manifest path given and no ./.fides/ directory found",
        ))
        .stderr(predicate::str::contains(
            "hint: pass a file or directory, e.g. fl stats path/to/manifests",
        ));
    fl().current_dir(empty.path())
        .args(["taxonomy", "diff"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "e.g. fl taxonomy diff path/to/manifests",
        ));
    // Other errors get no hint.
    fl().args(["validate", "--from", "json"])
        .arg(invalid("e001_unknown_key.yml"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("hint:").not());
}

/// `fl … | head -1`: when the reader closes the pipe, fl stops quietly with exit 0.
#[test]
fn closed_stdout_pipe_exits_quietly() {
    for args in [
        &["taxonomy", "cat", "categories", "--format", "json"][..],
        &["taxonomy", "list", "all", "--format", "plain"][..],
    ] {
        let mut child = StdCommand::new(assert_cmd::cargo::cargo_bin("fl"))
            .args(["--color", "never"])
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // Read one byte, then close our end like `head -c1` would.
        let mut stdout = child.stdout.take().unwrap();
        let mut first = [0u8; 1];
        stdout.read_exact(&mut first).unwrap();
        drop(stdout);
        let out = child.wait_with_output().unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(out.status.code(), Some(0), "{args:?}: {stderr}");
        assert_eq!(stderr, "", "{args:?}");
    }
}

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;

pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn demo() -> PathBuf {
    fixtures().join("demo_resources")
}

pub fn invalid(name: &str) -> PathBuf {
    fixtures().join("invalid").join(name)
}

/// `fl` with colors off and summaries silenced, so output is byte-stable.
pub fn fl() -> Command {
    let mut cmd = Command::cargo_bin("fl").expect("fl binary");
    cmd.env("NO_COLOR", "1")
        .env_remove("FL_COLOR")
        .arg("--color")
        .arg("never");
    cmd
}

pub fn stdout(cmd: &mut Command) -> String {
    let out = cmd.output().expect("run fl");
    String::from_utf8(out.stdout).expect("utf-8 stdout")
}

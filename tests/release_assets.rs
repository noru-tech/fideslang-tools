//! The shell completions and man pages that go into release archives (scripts/release-assets.sh)
//! and the Homebrew formula patch that installs them (scripts/homebrew-formula-extras.sh).
#![cfg(unix)]

use std::path::Path;
use std::process::Command;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn release_assets_are_generated_and_non_empty() {
    let out = tempfile::tempdir().unwrap();
    let status = Command::new(root().join("scripts/release-assets.sh"))
        .arg(out.path())
        .env("FL", assert_cmd::cargo::cargo_bin("fl"))
        .current_dir(root())
        .status()
        .unwrap();
    assert!(status.success());
    let read = |p: &str| std::fs::read_to_string(out.path().join(p)).unwrap();
    assert!(read("completions/fl.bash").contains("complete -F _fl"));
    assert!(read("completions/_fl").starts_with("#compdef fl"));
    assert!(read("completions/fl.fish").contains("complete -c fl"));
    assert!(read("man/fl.1").contains(".TH fl 1"));
    assert!(read("man/fl-validate.1").contains("validate"));
    let pages = std::fs::read_dir(out.path().join("man")).unwrap().count();
    assert!(pages >= 18, "{pages} man pages");
    for entry in std::fs::read_dir(out.path().join("man")).unwrap() {
        let p = entry.unwrap().path();
        assert!(std::fs::metadata(&p).unwrap().len() > 0, "{}", p.display());
    }
}

fn patch(formula: &Path) -> std::process::Output {
    Command::new(root().join("scripts/homebrew-formula-extras.sh"))
        .arg(formula)
        .output()
        .unwrap()
}

/// `tests/fixtures/homebrew/fl.rb` is the formula dist 0.33.0 generates for this project.
#[test]
fn homebrew_formula_patch_installs_completions_and_man_pages() {
    let dir = tempfile::tempdir().unwrap();
    let formula = dir.path().join("fl.rb");
    let original = std::fs::read_to_string(root().join("tests/fixtures/homebrew/fl.rb")).unwrap();
    std::fs::write(&formula, &original).unwrap();
    let out = patch(&formula);
    assert!(out.status.success(), "{out:?}");
    let patched = std::fs::read_to_string(&formula).unwrap();
    let install = &patched[patched.find("  def install\n").unwrap()..];
    let lines = [
        "    bash_completion.install \"completions/fl.bash\" => \"fl\"\n",
        "    zsh_completion.install \"completions/_fl\"\n",
        "    fish_completion.install \"completions/fl.fish\"\n",
        "    man1.install Dir[\"man/*.1\"]\n",
        "    rm_r %w[completions man]\n",
    ];
    let leftovers = install.find("leftover_contents = Dir").unwrap();
    for line in lines {
        let at = install
            .find(line)
            .unwrap_or_else(|| panic!("missing {line}"));
        assert!(
            at < leftovers,
            "{line} must come before the pkgshare leftovers"
        );
    }
    // Only lines were added.
    let removed: Vec<&str> = original
        .lines()
        .filter(|l| !patched.lines().any(|p| p == *l))
        .collect();
    assert!(removed.is_empty(), "{removed:?}");
    assert!(
        !patched.lines().any(|l| l.ends_with(' ')),
        "trailing spaces"
    );

    // Idempotent.
    assert!(patch(&formula).status.success());
    assert_eq!(std::fs::read_to_string(&formula).unwrap(), patched);

    // Valid Ruby, when Ruby is available.
    if let Ok(out) = Command::new("ruby").arg("-c").arg(&formula).output() {
        assert!(out.status.success(), "{out:?}");
    }
}

#[test]
fn homebrew_formula_patch_fails_when_the_template_changes() {
    let dir = tempfile::tempdir().unwrap();
    let formula = dir.path().join("fl.rb");
    std::fs::write(
        &formula,
        "class Fl < Formula\n  def install\n    bin.install \"fl\"\n  end\nend\n",
    )
    .unwrap();
    let out = patch(&formula);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("anchor line not found"));
}

/// Every archive ships NOTICE (attribution for the CC BY 4.0 taxonomy) and the generated
/// completions and man pages; the generated paths are the ones scripts/release-assets.sh writes.
#[test]
fn dist_includes_notice_and_release_assets() {
    let config = std::fs::read_to_string(root().join("dist-workspace.toml")).unwrap();
    let line = config
        .lines()
        .find(|l| l.starts_with("include = "))
        .expect("include in dist-workspace.toml");
    for entry in [
        "\"NOTICE\"",
        "\"target/release-assets/completions\"",
        "\"target/release-assets/man\"",
    ] {
        assert!(line.contains(entry), "{entry} missing from {line}");
    }
    let notice = std::fs::read_to_string(root().join("NOTICE")).unwrap();
    assert!(notice.contains("CC BY 4.0") || notice.contains("Creative Commons"));
    let script = std::fs::read_to_string(root().join("scripts/release-assets.sh")).unwrap();
    assert!(script.contains("out=\"${1:-target/release-assets}\""));
}

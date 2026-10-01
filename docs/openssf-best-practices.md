# OpenSSF Best Practices: "passing" self-assessment

Prepared answers for the [OpenSSF Best Practices badge](https://www.bestpractices.dev/) at the
**passing** level, so the form can be filled in from this page. The criteria and their order follow
`criteria/criteria.yml` (level `0`) in
[coreinfrastructure/best-practices-badge](https://github.com/coreinfrastructure/best-practices-badge);
retired and future criteria are left out. Answers describe the repository as of 2026-10-01 and are a
self-assessment: the project has **not** been submitted yet, so there is no badge.

**Status: 53 Met, 1 Unmet, 13 N/A of 67.** Every MUST is Met or N/A, so the project qualifies for
"passing"; the one Unmet criterion (`dynamic_analysis`) is SUGGESTED. Three answers depend on live
data a reviewer should confirm at submission time and are marked "Met (confirm)":
`report_responses`, `enhancement_responses` and `static_analysis_fixed`. The cryptography group is
N/A because `fl` implements and calls no cryptography.

## Basics

| Criterion | Level | Answer | Evidence |
| --- | --- | --- | --- |
| `description_good` | MUST | Met | The first line of the README says what `fl` does: "Rust CLI for Fideslang privacy taxonomies and Fides manifests. Browse, validate, merge, convert and graph data maps offline." [README #readme](https://github.com/noru-tech/fideslang-tools#readme) |
| `interact` | MUST | Met | Obtain: [README #how-do-i-install-fl](https://github.com/noru-tech/fideslang-tools#how-do-i-install-fl). Feedback: issue forms at [issues/new/choose](https://github.com/noru-tech/fideslang-tools/issues/new/choose). Contribute: [CONTRIBUTING.md](https://github.com/noru-tech/fideslang-tools/blob/main/CONTRIBUTING.md) |
| `contribution` | MUST | Met | [CONTRIBUTING.md](https://github.com/noru-tech/fideslang-tools/blob/main/CONTRIBUTING.md) describes the pull-request process and the checks to run; [.github/PULL_REQUEST_TEMPLATE.md](https://github.com/noru-tech/fideslang-tools/blob/main/.github/PULL_REQUEST_TEMPLATE.md) |
| `contribution_requirements` | SHOULD | Met | CONTRIBUTING.md: `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, snapshot review, ground rules (offline, pure Rust, no hand edits to `taxonomy/`). [CONTRIBUTING.md#ground-rules](https://github.com/noru-tech/fideslang-tools/blob/main/CONTRIBUTING.md#ground-rules) |
| `floss_license` | MUST | Met | MIT for the code; the vendored taxonomy is CC BY 4.0. [LICENSE](https://github.com/noru-tech/fideslang-tools/blob/main/LICENSE), [NOTICE](https://github.com/noru-tech/fideslang-tools/blob/main/NOTICE) |
| `floss_license_osi` | SUGGESTED | Met | MIT is OSI-approved. |
| `license_location` | MUST | Met | [LICENSE](https://github.com/noru-tech/fideslang-tools/blob/main/LICENSE) |
| `documentation_basics` | MUST | Met | README (install, quick start, command reference, limitations), `fl --help`, man pages via `fl manpage`. [README #readme](https://github.com/noru-tech/fideslang-tools#readme) |
| `documentation_interface` | MUST | Met | Command and `--format` reference: [README #what-it-does](https://github.com/noru-tech/fideslang-tools#what-it-does) and [README #output-formats-and-exit-codes](https://github.com/noru-tech/fideslang-tools#output-formats-and-exit-codes); validation codes: [docs/rules/README.md](https://github.com/noru-tech/fideslang-tools/blob/main/docs/rules/README.md); exit codes: [docs/exit-codes.md](https://github.com/noru-tech/fideslang-tools/blob/main/docs/exit-codes.md) |
| `sites_https` | MUST | Met | Repository, releases, crates.io and the Homebrew tap are all served over HTTPS by GitHub and crates.io. |
| `discussion` | MUST | Met | GitHub issues and pull requests: searchable, addressable by URL, open to anyone, no proprietary client needed. [issues](https://github.com/noru-tech/fideslang-tools/issues) |
| `english` | SHOULD | Met | All documentation is in English; reports are accepted in English. |
| `maintained` | MUST | Met | Releases 0.1.0 to 0.1.3 between 2026-09-13 and 2026-10-01; Dependabot and CI are active. [releases](https://github.com/noru-tech/fideslang-tools/releases) |

## Change control

| Criterion | Level | Answer | Evidence |
| --- | --- | --- | --- |
| `repo_public` | MUST | Met | [repository](https://github.com/noru-tech/fideslang-tools) |
| `repo_track` | MUST | Met | Git history records author, date and change. [commits/main](https://github.com/noru-tech/fideslang-tools/commits/main) |
| `repo_interim` | MUST | Met | Every change lands on `main` through pull requests between releases. [pulls?q=is%3Apr](https://github.com/noru-tech/fideslang-tools/pulls?q=is%3Apr) |
| `repo_distributed` | SUGGESTED | Met | Git. |
| `version_unique` | MUST | Met | Each release has a unique version in `Cargo.toml` and a tag. [tags](https://github.com/noru-tech/fideslang-tools/tags) |
| `version_semver` | SUGGESTED | Met | Semantic Versioning, stated in the changelog header. [CHANGELOG.md](https://github.com/noru-tech/fideslang-tools/blob/main/CHANGELOG.md) |
| `version_tags` | SUGGESTED | Met | `v0.1.0` … `v0.1.3`; releases are cut by tagging. [tags](https://github.com/noru-tech/fideslang-tools/tags) |
| `release_notes` | MUST | Met | Keep a Changelog file with a human-written section per release. [CHANGELOG.md](https://github.com/noru-tech/fideslang-tools/blob/main/CHANGELOG.md) |
| `release_notes_vulns` | MUST | N/A | No publicly known vulnerability with a CVE or similar ID has been fixed. The 0.1.1 `fl split` path-traversal fix was found internally, had no ID, and is still listed under `Security` in the changelog. [CHANGELOG.md#011---2026-09-14](https://github.com/noru-tech/fideslang-tools/blob/main/CHANGELOG.md#011---2026-09-14) |

## Reporting

| Criterion | Level | Answer | Evidence |
| --- | --- | --- | --- |
| `report_process` | MUST | Met | GitHub issues with bug-report and feature-request forms. [issues/new/choose](https://github.com/noru-tech/fideslang-tools/issues/new/choose) |
| `report_tracker` | SHOULD | Met | GitHub issues. [issues](https://github.com/noru-tech/fideslang-tools/issues) |
| `report_responses` | MUST | Met (confirm) | The maintainer responds to reports; at the time of writing there are few or no external bug reports in the 2–12 month window. Check [issues?q=is%3Aissue](https://github.com/noru-tech/fideslang-tools/issues?q=is%3Aissue) before submitting. |
| `enhancement_responses` | SHOULD | Met (confirm) | As above, for enhancement requests. |
| `report_archive` | MUST | Met | Closed and open issues stay public and searchable. [issues?q=is%3Aissue](https://github.com/noru-tech/fideslang-tools/issues?q=is%3Aissue) |
| `vulnerability_report_process` | MUST | Met | [SECURITY.md](https://github.com/noru-tech/fideslang-tools/blob/main/SECURITY.md) |
| `vulnerability_report_private` | MUST | Met | GitHub private vulnerability reporting ([security/advisories/new](https://github.com/noru-tech/fideslang-tools/security/advisories/new)) or security@noru.tech, both in SECURITY.md. |
| `vulnerability_report_response` | MUST | N/A | No vulnerability reports were received in the last 6 months (the 0.1.1 fix was found internally). SECURITY.md commits to acknowledging within 5 business days, inside the 14-day limit. |

## Quality

| Criterion | Level | Answer | Evidence |
| --- | --- | --- | --- |
| `build` | MUST | Met | `cargo build`; release builds by cargo-dist in GitHub Actions. [CONTRIBUTING.md#development](https://github.com/noru-tech/fideslang-tools/blob/main/CONTRIBUTING.md#development) |
| `build_common_tools` | SUGGESTED | Met | Cargo. |
| `build_floss_tools` | SHOULD | Met | Rust toolchain, Cargo and cargo-dist are FLOSS. |
| `test` | MUST | Met | Unit tests, `assert_cmd` integration tests and `insta` snapshots, run with `cargo test`; documented in README and CONTRIBUTING. [tests](https://github.com/noru-tech/fideslang-tools/tree/main/tests) |
| `test_invocation` | SHOULD | Met | `cargo test`, the standard Rust invocation. |
| `test_most` | SUGGESTED | Met | Every subcommand has integration tests; every validation code has a failing and a passing fixture checked by `cargo test`. Coverage is not measured. |
| `test_continuous_integration` | SUGGESTED | Met | `ci.yml` runs fmt, clippy, tests on Linux and macOS, MSRV, cargo-deny and typos on every pull request and push to `main`. [.github/workflows/ci.yml](https://github.com/noru-tech/fideslang-tools/blob/main/.github/workflows/ci.yml) |
| `test_policy` | MUST | Met | CONTRIBUTING.md: a new validation rule needs failing and passing fixtures; output changes need reviewed snapshots; the PR template checklist asks for both. [CONTRIBUTING.md#development](https://github.com/noru-tech/fideslang-tools/blob/main/CONTRIBUTING.md#development) |
| `tests_are_added` | MUST | Met | The 0.1.1 `fl split` fix added a regression test in `tests/cli_manifest.rs`; the rule documentation added fixtures and tests in `tests/cli_validate.rs`. [commits/main](https://github.com/noru-tech/fideslang-tools/commits/main) |
| `tests_documented_added` | SUGGESTED | Met | CONTRIBUTING.md and [.github/PULL_REQUEST_TEMPLATE.md](https://github.com/noru-tech/fideslang-tools/blob/main/.github/PULL_REQUEST_TEMPLATE.md) |
| `warnings` | MUST | Met | CI builds with `RUSTFLAGS=-D warnings` and runs `cargo clippy --all-targets --all-features -- -D warnings`. |
| `warnings_fixed` | MUST | Met | Warnings fail CI, so `main` has none. |
| `warnings_strict` | SUGGESTED | Met | All warnings are denied in CI. |

## Security

| Criterion | Level | Answer | Evidence |
| --- | --- | --- | --- |
| `know_secure_design` | MUST | Met | The maintainer applies secure design in practice: no network access, no `unsafe` code, writes only where told, path checks in `fl split`, least-privilege workflow tokens, pinned actions. See SECURITY.md "Scope and threat model". |
| `know_common_errors` | MUST | Met | Relevant error classes for a file-processing CLI and their mitigations: path traversal (file names validated in `fl split`), memory safety (safe Rust only), untrusted input parsing (pure-Rust parsers, errors instead of panics), supply chain (cargo-deny, pinned actions, attestations). |
| `crypto_published` | MUST | N/A | `fl` uses no cryptography. |
| `crypto_call` | SHOULD | N/A | No cryptography. |
| `crypto_floss` | MUST | N/A | No cryptography. |
| `crypto_keylength` | MUST | N/A | No cryptography. |
| `crypto_working` | MUST | N/A | No cryptography. |
| `crypto_weaknesses` | SHOULD | N/A | No cryptography. |
| `crypto_pfs` | SHOULD | N/A | No cryptography. |
| `crypto_password_storage` | MUST | N/A | `fl` stores no passwords. |
| `crypto_random` | MUST | N/A | `fl` generates no keys or nonces. |
| `delivery_mitm` | MUST | Met | Releases, crates.io and the Homebrew tap are delivered over HTTPS; archives carry GitHub artifact attestations. [README #how-do-i-verify-a-release-before-running-it](https://github.com/noru-tech/fideslang-tools#how-do-i-verify-a-release-before-running-it) |
| `delivery_unsigned` | MUST | Met | Checksums are fetched over HTTPS from GitHub Releases, and the attestation (a signed statement) can be checked with `gh attestation verify`. |
| `vulnerabilities_fixed_60_days` | MUST | Met | No known unpatched vulnerabilities; `cargo-deny` checks RustSec advisories in CI. |
| `vulnerabilities_critical_fixed` | SHOULD | Met | The one security fix so far (0.1.1) shipped one day after 0.1.0. |
| `no_leaked_credentials` | MUST | Met | No credentials in the repository; crates.io publishing uses Trusted Publishing (OIDC), and the only secret (`HOMEBREW_TAP_TOKEN`) lives in GitHub repository secrets. |

## Analysis

| Criterion | Level | Answer | Evidence |
| --- | --- | --- | --- |
| `static_analysis` | MUST | Met | Clippy with `-D warnings` on every pull request, and CodeQL (Rust, Python, workflows) on every pull request, on `main` and weekly. [.github/workflows/codeql.yml](https://github.com/noru-tech/fideslang-tools/blob/main/.github/workflows/codeql.yml) |
| `static_analysis_common_vulnerabilities` | SUGGESTED | Met | CodeQL's default security queries. |
| `static_analysis_fixed` | MUST | Met (confirm) | No open medium or higher CodeQL findings; findings appear in code scanning. [security/code-scanning](https://github.com/noru-tech/fideslang-tools/security/code-scanning) |
| `static_analysis_often` | SUGGESTED | Met | Every pull request and push to `main`. |
| `dynamic_analysis` | SUGGESTED | Unmet | No fuzzing or other dynamic analysis beyond the test suite. A `cargo fuzz` target for the manifest loader would satisfy it. |
| `dynamic_analysis_unsafe` | SUGGESTED | N/A | `fl` is written in safe Rust (no `unsafe` blocks). |
| `dynamic_analysis_enable_assertions` | SUGGESTED | Met | `cargo test` runs debug builds with debug assertions and overflow checks on, including clap's `debug_assert` of the CLI definition. |
| `dynamic_analysis_fixed` | MUST | N/A | No dynamic analysis beyond the test suite is run, so there are no findings to fix (see `dynamic_analysis`). |

## Gaps worth closing

- `dynamic_analysis` (SUGGESTED): add a fuzz target for manifest parsing (`cargo fuzz`) and run it in
  a scheduled workflow.
- Dependabot covers GitHub Actions only; Cargo dependencies are refreshed by hand. Adding a `cargo`
  ecosystem entry to `.github/dependabot.yml` would make `vulnerabilities_fixed_60_days` less
  dependent on manual checks.
- Before submitting, confirm `report_responses` and `enhancement_responses` against the live issue
  tracker.

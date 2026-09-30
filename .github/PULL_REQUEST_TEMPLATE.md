<!--
Thanks for contributing. See CONTRIBUTING.md for the project layout and how to add a validation rule
or refresh the taxonomy snapshot.
-->

## What

<!-- One or two sentences on the change. -->

## Why

<!-- The problem, the issue it closes, or the upstream Fideslang behaviour it matches. -->

## Checks

- [ ] `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test`
- [ ] Output changes: snapshots reviewed with `cargo insta review`
- [ ] New or changed validation rule: stable code, fixture under `tests/fixtures/invalid/`, README table updated
- [ ] Taxonomy refresh: generated with `scripts/refresh-taxonomy.sh`, `taxonomy/SOURCE.md` updated, removed keys called out
- [ ] `CHANGELOG.md` updated under Unreleased for user-visible changes
- [ ] No real manifests or personal data in the diff

# Taxonomy snapshot provenance

The YAML files under `taxonomy/ethyca/` and `taxonomy/iab/` are **vendored snapshots** of the
Fideslang default taxonomy. They are compiled into the `fl` binary (`include_str!`) so the tool
works offline and never needs the `fideslang` Python package at runtime.

## License and attribution

The Fideslang taxonomy is © Ethyca, Inc. / IAB Tech Lab and licensed under **Creative Commons
Attribution 4.0 International (CC BY 4.0)** — https://creativecommons.org/licenses/by/4.0/. These
files are a *modified* redistribution: the upstream Python definitions were exported to YAML using
the upstream data model. See the repository `NOTICE` file. The Rust code in this repository is
MIT-licensed; this directory remains CC BY 4.0.

## Snapshots

| Snapshot | Upstream | Tag | Commit | Categories | Uses | Subjects |
| --- | --- | --- | --- | ---: | ---: | ---: |
| `ethyca` (default) | https://github.com/ethyca/fideslang | `3.1.4` | `7c27b5a5cfc8a46a299506fdf116383432813518` | 85 | 56 | 15 |
| `iab` | https://github.com/IABTechLab/fideslang | `3.0.0` | `c53726c9d9eeebb6b43bf56d53c29b53fc07249e` | 85 | 55 | 15 |

Snapshot date: 2026-09-13 (machine-readable copy in each directory's `snapshot.json`).

The only content difference between the two: `ethyca` adds the data use
`functional.storage.privacy_preferences` (parent `functional.storage`). Data categories and data
subjects are identical. `fl taxonomy diff` shows this.

## Why the Python source, not `data_files/`

Both upstream repositories ship `data_files/{data_categories,data_uses,data_subjects}.{yml,json,csv}`,
but that directory is a **stale export**: it was last regenerated on 2023-12-15 (commit `ffe60ac1`,
"Fideslang 3.0") and contains 54 data uses, while the Python source at the same tags contains 55
(IAB 3.0.0) and 56 (Ethyca 3.1.4). The source of truth is
`src/fideslang/default_taxonomy/{data_categories,data_uses,data_subjects}.py`, so the refresh script
installs the package at the pinned tag and exports from it.

## Record shape

Each entry carries the full upstream model, in upstream declaration order:

```yaml
- version_added: 2.0.0
  version_deprecated: null      # set when a key is deprecated
  replaced_by: null             # successor key, if any
  is_default: true
  fides_key: user.contact.email
  organization_fides_key: default_organization
  tags: null
  name: User Contact Email
  description: User's contact email address.
  parent_key: user.contact      # null for roots; always equals the dotted prefix of fides_key
```

Top-level keys are singular (`data_category`, `data_use`, `data_subject`), matching Fides manifests.

## How to refresh

```bash
scripts/refresh-taxonomy.sh ethyca <tag>
scripts/refresh-taxonomy.sh iab    <tag>
```

Then update the table above, `CHANGELOG.md`, and run `cargo test` (the counts in `snapshot.json`
are asserted against the parsed files).

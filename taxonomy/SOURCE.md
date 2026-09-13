# Taxonomy snapshot provenance

The YAML files in this directory are a **vendored snapshot** of the IAB Tech Lab Privacy Taxonomy
(Fideslang). They are compiled into the `fl` binary (`include_str!`) so the tool works offline and
never needs the `fideslang` Python package at runtime.

## Snapshot

| Field | Value |
| --- | --- |
| Upstream | https://github.com/IABTechLab/fideslang |
| Release tag | `3.0.0` |
| Commit | `c53726c9d9eeebb6b43bf56d53c29b53fc07249e` |
| Snapshot date | 2026-09-13 |
| Data categories | 85 |
| Data uses | 55 |
| Data subjects | 15 |

A machine-readable copy lives in `snapshot.json` and is what `fl taxonomy info` prints. `cargo test`
asserts that the parsed files match these counts.

## License and attribution

The taxonomy is licensed under **Creative Commons Attribution 4.0 International (CC BY 4.0)** —
https://creativecommons.org/licenses/by/4.0/. Copyright holders per upstream: Ethyca, Inc. (original
author) and IAB Tech Lab (current steward). These files are a *modified* redistribution: the
upstream Python definitions were exported to YAML using the upstream data model. See the repository
`NOTICE` file. The Rust code in this repository is MIT-licensed; this directory remains CC BY 4.0.

## Why the Python source, not `data_files/`

The upstream repository ships `data_files/{data_categories,data_uses,data_subjects}.{yml,json,csv}`,
but that directory is a **stale export**: it was last regenerated on 2023-12-15 (commit `ffe60ac1`,
"Fideslang 3.0") and contains 54 data uses, while the Python source at tag 3.0.0 contains 55. The
source of truth is `src/fideslang/default_taxonomy/{data_categories,data_uses,data_subjects}.py`, so
the refresh script installs the package at the pinned tag and exports from it.

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
scripts/refresh-taxonomy.sh <tag>
```

Then update the table above and `CHANGELOG.md`, review `git diff taxonomy/` (a *removed* key can
break users' manifests, so call it out), and run `cargo test`.

# fideslang-tools

<p align="center">
  <img src="https://raw.githubusercontent.com/noru-tech/fideslang-tools/main/assets/fideslang-tools.png" alt="fideslang-tools" width="720">
</p>

> `fl` — a fast, offline command-line toolbox for [Fideslang](https://github.com/IABTechLab/fideslang)
> privacy taxonomy and Fideslang manifests: browse, search and draw
> the taxonomy; cat, convert, merge, split, validate, summarize and graph your data maps.

[![License: MIT](https://img.shields.io/badge/Code-MIT-blue.svg)](./LICENSE)
[![Taxonomy: CC BY 4.0](https://img.shields.io/badge/Taxonomy-CC%20BY%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by/4.0/)
[![ci](https://github.com/noru-tech/fideslang-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/noru-tech/fideslang-tools/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fideslang-cli.svg)](https://crates.io/crates/fideslang-cli)

The [IAB Tech Lab Privacy Taxonomy](https://github.com/IABTechLab/fideslang) (Fideslang) is the
industry-standard vocabulary for describing personal data and how it is processed: hierarchical
**data categories** (`user.contact.email`), **data uses** (`marketing.advertising`) and **data
subjects** (`customer`), plus a YAML manifest language for describing **datasets**, **systems**,
**policies** and **organizations** with those labels. It is governed by IAB Tech Lab's Privacy
Implementation & Accountability Task Force and maps onto GDPR, CCPA/CPRA, LGPD and ISO 19944, which
makes it the canonical, standards-body-backed way to label a data map so that it interoperates with
other vendors, consent frameworks (TCF/GVL) and privacy tooling.

Upstream ships the taxonomy as a Python library. `fl` is a single static binary that makes the
taxonomy and your manifests easy to explore and work with from a terminal, a script or CI, with no
Python and no network. It bundles the **IAB Tech Lab fideslang 3.0.0** taxonomy (85 data categories,
55 data uses, 15 data subjects); provenance, license and the refresh recipe live in
[`taxonomy/SOURCE.md`](./taxonomy/SOURCE.md).

## Install

Prebuilt binaries for macOS (Apple Silicon, Intel) and Linux (x86_64, aarch64, fully static):

```bash
# Homebrew (macOS and Linux)
brew install noru-tech/tap/fl
```

```bash
# Shell installer (downloads the right binary into ~/.cargo/bin, no Rust toolchain needed)
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/noru-tech/fideslang-tools/releases/latest/download/fideslang-cli-installer.sh | sh
```

```bash
# From crates.io (needs a Rust toolchain)
cargo install fideslang-cli
```

```bash
# Prebuilt via cargo-binstall (no compile)
cargo binstall fideslang-cli
```

Or grab a tarball from the [releases page](https://github.com/noru-tech/fideslang-tools/releases).
Archives ship SHA-256 sums and GitHub artifact attestations
(`gh attestation verify <archive> --repo noru-tech/fideslang-tools`). A tarball downloaded with a
browser on macOS is quarantined by Gatekeeper; `xattr -d com.apple.quarantine fl` clears it.
`brew` and `curl` installs are not affected.

Shell completions and man pages: `fl completions zsh|bash|fish|…` and `fl manpage --out-dir DIR`.

## Quick start

### Browse the taxonomy

```console
$ fl taxonomy tree categories --depth 2
data_category (iab 3.0.0)
├── system  System Data
│   ├── authentication  Authentication Data
│   └── operations  Operations Data
└── user  User Data
    ├── account  Account Information
    ├── authorization  Authorization Information
    ├── behavior  Observed Behavior
    …
    └── workplace  Workplace
28 keys shown, depth ≤ 2 (iab 3.0.0)

$ fl taxonomy show user.contact.email
user.contact.email  —  User Contact Email  [data_category · iab 3.0.0 · added 2.0.0]
  User's contact email address.
ancestors:   user › user.contact › user.contact.email
children:    (none)

$ fl taxonomy search cookie --keys-only
cat   user.device.cookie  Device Cookie
cat   user.device.cookie_id  Cookie ID

```

`fl taxonomy tree` also emits Graphviz DOT, Mermaid or nested JSON (`--format dot|mermaid|json`,
`--root user.contact`), `fl taxonomy list` prints tables/plain keys/JSON/YAML/CSV, and
`fl taxonomy cat uses --format csv` reproduces upstream's CSV export.

### Work with manifests

Manifest commands take files, directories (searched recursively for `*.yml`, `*.yaml`, `*.json`)
or `-` for stdin, and default to `./.fides/` when no path is given.

```console
$ fl validate tests/fixtures/demo_resources
tests/fixtures/demo_resources/demo_dataset.yml  dataset[demo_users_dataset].collections[0].fields[3].data_categories
  W002 field `food_preference` has no data_categories
tests/fixtures/demo_resources/demo_dataset.yml  dataset[demo_users_dataset].collections[0].fields[4].data_categories[0]
  E001 unknown data category `user.contact.state` — did you mean `user.contact.address.state`, `user.contact.address`, `user.contact.email`?
tests/fixtures/demo_resources/demo_system.yml  system[demo_marketing_system].privacy_declarations[0].data_categories[0]
  E001 unknown data category `user.cookie_id` — did you mean `user.device.cookie_id`, `user.device.cookie`, `user.unique_id`?
tests/fixtures/demo_resources/demo_system.yml  system[demo_marketing_system].privacy_declarations[0].data_use
  E001 unknown data use `advertising` — did you mean `marketing.advertising`?
  …
5 files, 7 resources checked against iab 3.0.0: 6 errors, 1 warning
$ echo $?
1
```

```console
$ fl cat .fides/ --type dataset --format tree
dataset (1)
└── demo_users_dataset  Demo Users Dataset
    └── users
        ├── created_at  system.operations
        ├── email  user.contact.email
        ├── first_name  user.name
        ├── food_preference  (uncategorized)
        └── uuid  user.unique_id

$ fl graph .fides/ --format mermaid
flowchart LR
  dataset_demo_users_dataset[("demo_users_dataset")]:::dataset
  system_demo_analytics_system["demo_analytics_system"]:::system
  use_improve_system{{"improve.system"}}:::use
  dataset_demo_users_dataset -->|"ingress"| system_demo_analytics_system
  system_demo_analytics_system -.->|"user.contact, user.device.cookie_id"| use_improve_system
  …

$ fl graph .fides/ --include fields,categories,uses,subjects | dot -Tsvg -o datamap.svg
```

```console
$ fl merge .fides/ -o all.yml                # union every file into one document
$ fl split all.yml --out-dir out/            # out/system.yml, out/dataset.yml, …
$ fl split all.yml --by resource --format json --out-dir out/   # out/system/<fides_key>.json
$ fl convert datamap.yml --to json           # YAML ⇄ JSON ⇄ CSV
$ fl convert taxonomy.csv --to yaml          # upstream taxonomy CSV back to a manifest
$ fl cat .fides/ --key 'demo_*' --format json | jq '.system[].fides_key'
$ fl stats .fides/ --rollup
```

## Commands

| Command | What it does |
| --- | --- |
| `fl taxonomy list <categories\|uses\|subjects\|all>` | Keys as a table, plain list, JSON, YAML or CSV (`--prefix`, `--deprecated`) |
| `fl taxonomy cat <kind>` | The vendored taxonomy file, byte-exact YAML or converted to JSON / CSV |
| `fl taxonomy tree [kind]` | Hierarchy as a terminal tree, DOT, Mermaid or JSON (`--depth`, `--root`, `--descriptions`) |
| `fl taxonomy show KEY` | One key with description, ancestors, children and version metadata |
| `fl taxonomy search PATTERN` | Substring or `--regex` search over keys, names and descriptions |
| `fl taxonomy diff PATH…` | Show what a manifest set's custom `data_category` / `data_use` / `data_subject` resources add to or override in the bundled taxonomy |
| `fl taxonomy info` | Bundled taxonomy provenance: upstream, tag, commit, date, counts |
| `fl cat PATH…` | Print manifests as YAML, JSON or a resource tree; filter with `--type` and `--key GLOB` |
| `fl convert INPUT` | Convert one file between YAML, JSON and CSV (`--to`, or inferred from `-o`) |
| `fl merge PATH…` | Union many files into one document (`--fail-on-duplicate`, `--dedupe`) |
| `fl split INPUT` | One file per resource type (`--by type`) or per resource (`--by resource`) |
| `fl validate PATH…` | Check keys, references, duplicates, custom taxonomy and structure; text, JSON or GitHub annotations |
| `fl stats PATH…` | Counts, categorized-field coverage, orphan datasets, key usage (`--rollup`, `--top`) |
| `fl graph PATH…` | Systems ↔ datasets ↔ flows ↔ uses/categories/subjects as DOT, Mermaid or JSON (`--include`, `--focus KEY --depth N`) |
| `fl completions <shell>` / `fl manpage` | Shell completions and man pages |

Global flags: `--color auto|always|never` (honors `NO_COLOR`), `-o FILE`, `-q`.

Exit codes: `0` ok · `1` findings (validation errors, or a non-empty `diff --exit-code`) ·
`2` usage, I/O or parse error.

## Validation rules

| Code | Severity | Check |
| --- | --- | --- |
| E001 | error | Unknown data category / use / subject key (with "did you mean" suggestions) |
| E002 | error | Duplicate `fides_key` within a resource type, across all loaded files |
| E003 | error | Dangling reference: `dataset_references`, `ingress`/`egress`, `fides_meta.references`, `after`/`erase_after` |
| E004 | error | Custom taxonomy `parent_key` missing or not the dotted prefix of the key |
| E005 | error | Custom taxonomy record references itself |
| E006 | error | Invalid `fides_key` syntax (letters, digits, `.`, `_`, `<`, `>`, `-`) |
| E007 | error | Structure: missing required fields, wrong types, unknown resource type |
| W001 | warning | Deprecated key (suggests `replaced_by`) |
| W002 | warning | Dataset field with no `data_categories` |
| W003 | warning | Privacy declaration with no categories or no subjects |
| W004 | warning | `data_purposes`, the deprecated alias of `data_uses` |
| W005 | warning | Field not defined by the upstream models (only with `--strict`) |

Custom `data_category` / `data_use` / `data_subject` resources declared in the manifest set extend
the taxonomy for E001 (disable with `--no-custom-taxonomy`). `--deny CODE` promotes a code to an
error, `--allow CODE` silences it, `-W` treats every warning as an error. `--format github` prints
`::error file=…::` annotations for GitHub Actions.

## Notes on fidelity

- YAML is read with YAML 1.2 semantics: unquoted `yes`/`no`/`on`/`off` stay strings and
  `version_added: 2.0.0` stays a string. Output uses the same layout as upstream's PyYAML export
  (2-space indent, block sequences flush with their key).
- Unknown fields on any resource are preserved through `cat`, `convert`, `merge` and `split`.
- Directory loading unions files in sorted path order, like upstream's `ingest_manifests`.
- The demo manifests bundled as test fixtures come from upstream and predate Fideslang 3.0; they
  deliberately fail validation and make a good playground: `fl validate tests/fixtures/demo_resources`.

## Development

```bash
cargo build
cargo test            # unit + assert_cmd integration tests + insta snapshots
cargo clippy --all-targets -- -D warnings
```

See [CONTRIBUTING.md](./CONTRIBUTING.md) for the layout, how to add a validation rule, and how to
refresh a taxonomy snapshot.

## License

The Rust code is MIT-licensed (see [LICENSE](./LICENSE)). The vendored IAB Tech Lab Privacy Taxonomy
under `taxonomy/` and the demo fixtures under `tests/fixtures/demo_resources/` remain under
CC BY 4.0 — see [NOTICE](./NOTICE) and [`taxonomy/SOURCE.md`](./taxonomy/SOURCE.md) for attribution.

# fideslang-tools

Rust CLI for Fideslang privacy taxonomies and Fides manifests. Browse, validate, merge, convert and graph data maps offline.

<p align="center">
  <img src="https://raw.githubusercontent.com/noru-tech/fideslang-tools/main/assets/fideslang-tools.png" alt="fideslang-tools" width="720">
</p>

`fl` is a fast, offline command-line toolbox for the [Fideslang](https://github.com/IABTechLab/fideslang)
privacy taxonomy and Fides manifests: browse, search and draw the taxonomy; cat, convert, merge,
split, validate, summarize and graph your data maps.

[![Release](https://img.shields.io/github/v/release/noru-tech/fideslang-tools)](https://github.com/noru-tech/fideslang-tools/releases/latest)
[![ci](https://github.com/noru-tech/fideslang-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/noru-tech/fideslang-tools/actions/workflows/ci.yml)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/noru-tech/fideslang-tools/badge)](https://scorecard.dev/viewer/?uri=github.com/noru-tech/fideslang-tools)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
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

<!-- Demo: render docs/demo.tape with VHS (https://github.com/charmbracelet/vhs) when a recording is wanted. -->

## How do I install fl?

Run `brew install noru-tech/tap/fl`, or `cargo binstall fideslang-cli` if you have Rust tooling;
prebuilt binaries for macOS (Apple Silicon, Intel) and Linux (x86_64, aarch64, fully static) are on
every release.

### Homebrew (macOS and Linux)

```bash
brew install noru-tech/tap/fl
```

From the first release after 0.1.3, the formula also installs bash, zsh and fish completions and
the man pages (`man fl`, `man fl-validate`, …).

### From crates.io

The crate is `fideslang-cli` (the crate `fl` on crates.io is an unrelated tool). The binary it
installs is `fl`.

```bash
cargo binstall fideslang-cli              # downloads the prebuilt release binary
cargo install --locked fideslang-cli      # builds from source, needs a Rust toolchain
```

### Prebuilt binaries

Download the archive for your platform from the
[releases page](https://github.com/noru-tech/fideslang-tools/releases/latest):

| Platform | Archive |
| --- | --- |
| macOS, Apple Silicon | `fideslang-cli-aarch64-apple-darwin.tar.xz` |
| macOS, Intel | `fideslang-cli-x86_64-apple-darwin.tar.xz` |
| Linux, x86_64 (static musl) | `fideslang-cli-x86_64-unknown-linux-musl.tar.xz` |
| Linux, aarch64 (static musl) | `fideslang-cli-aarch64-unknown-linux-musl.tar.xz` |

Each archive unpacks to `fideslang-cli-<target>/fl`; put `fl` anywhere on your `PATH`. From the
first release after 0.1.3 the archive also holds `NOTICE` (taxonomy attribution), `completions/`
(`fl.bash`, `_fl` for zsh, `fl.fish`) and `man/` (`fl.1` and one page per subcommand); copy the
completions to your shell's completion directory and the pages to a `man1` directory on your
`MANPATH`. A tarball
downloaded with a browser on macOS is quarantined by Gatekeeper; `xattr -d com.apple.quarantine fl`
clears it.

### How do I verify a release before running it?

Run `gh attestation verify` on the archive, or check its SHA-256 file. Every archive is built by
GitHub Actions from a tagged commit and carries a GitHub artifact attestation (SLSA build
provenance). Check that the archive was built by this repository's release workflow:

```bash
gh attestation verify fideslang-cli-aarch64-apple-darwin.tar.xz \
  --repo noru-tech/fideslang-tools \
  --signer-workflow noru-tech/fideslang-tools/.github/workflows/release.yml
```

Each archive also has a `<archive>.sha256` file next to it on the release (and `sha256.sum` lists
them all):

```bash
TARGET=aarch64-apple-darwin   # or x86_64-apple-darwin, x86_64-unknown-linux-musl, aarch64-unknown-linux-musl
BASE=https://github.com/noru-tech/fideslang-tools/releases/latest/download
curl -LO "$BASE/fideslang-cli-$TARGET.tar.xz"
curl -LO "$BASE/fideslang-cli-$TARGET.tar.xz.sha256"
shasum -a 256 -c "fideslang-cli-$TARGET.tar.xz.sha256"    # or: sha256sum -c
tar xJf "fideslang-cli-$TARGET.tar.xz"
```

Shell completions and man pages for any install method: `fl completions zsh|bash|fish|…` and
`fl manpage --out-dir DIR`.

## Quick start

Nothing to configure: the taxonomy is compiled in, and every command below works in an empty
directory without network access.

### How do I look up a Fideslang data category, data use or data subject?

Use `fl taxonomy show KEY` for one key, `fl taxonomy search WORD` to find keys, and
`fl taxonomy tree` to see the hierarchy; all three work offline against the bundled taxonomy.

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

### How do I validate a Fides manifest offline?

Run `fl validate PATH`: it checks every manifest file against the bundled taxonomy and the manifest
structure, with no network and no Fides server, and exits `1` if it finds an error. Write a
two-resource manifest (one dataset, one system) and check it:

```console
$ cat > datamap.yml <<'EOF'
dataset:
  - fides_key: users_db
    collections:
      - name: users
        fields:
          - name: email
            data_categories: [user.contact.email]
          - name: tracking_id
            data_categories: [user.cookie_id]
system:
  - fides_key: newsletter
    system_type: Service
    ingress:
      - fides_key: users_db
        type: dataset
    privacy_declarations:
      - data_use: marketing.communications.email
        data_categories: [user.contact.email]
        data_subjects: [customer]
EOF

$ fl validate datamap.yml
datamap.yml  dataset[users_db].collections[0].fields[1].data_categories[0]
  E001 unknown data category `user.cookie_id` — did you mean `user.device.cookie_id`, `user.device.cookie`, `user.unique_id`?
see https://github.com/noru-tech/fideslang-tools/blob/main/docs/rules/E001.md
1 files, 2 resources checked against iab 3.0.0: 1 error, 0 warnings
$ echo $?
1
```

Every code (`E001` above) has a page that explains it and how to fix it, and the report ends with a
link to the page of each code it found; see [Validation rules](#validation-rules).

### How do I draw a data map from Fides manifests?

Run `fl graph PATH`: it renders systems, datasets, data flows and data uses as Graphviz DOT
(default), Mermaid or JSON.

```console
$ fl graph datamap.yml --format mermaid
flowchart LR
  …
  dataset_users_db[("users_db")]:::dataset
  system_newsletter["newsletter"]:::system
  use_marketing_communications_email{{"marketing.communications.email"}}:::use
  dataset_users_db -->|"ingress"| system_newsletter
  system_newsletter -.->|"user.contact.email"| use_marketing_communications_email

$ fl convert datamap.yml --to json -o datamap.json
```

### With your own manifests

Manifest commands take files, directories (searched recursively for `*.yml`, `*.yaml`, `*.json`)
or `-` for stdin, and default to `./.fides/` when no path is given.

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

$ fl graph .fides/ --include fields,categories,uses,subjects | dot -Tsvg -o datamap.svg
$ fl merge .fides/ -o all.yml                # union every file into one document
$ fl split all.yml --out-dir out/            # out/system.yml, out/dataset.yml, …
$ fl split all.yml --by resource --format json --out-dir out/   # out/system/<fides_key>.json
$ fl convert taxonomy.csv --to yaml          # upstream taxonomy CSV back to a manifest
$ fl cat .fides/ --key 'demo_*' --format json | jq '.system[].fides_key'
$ fl stats .fides/ --rollup
```

A clone of this repository has upstream's demo manifests under `tests/fixtures/demo_resources`;
they predate Fideslang 3.0 and deliberately fail validation, which makes them a good playground:
`fl validate tests/fixtures/demo_resources`.

## What it does

| Area | Commands |
| --- | --- |
| Browse the taxonomy | `fl taxonomy list`, `tree`, `show`, `search`, `cat`, `info` |
| Check a data map | `fl validate` ([stable E0xx/W0xx codes](docs/rules/README.md), "did you mean" suggestions, GitHub annotations, SARIF) |
| Transform manifests | `fl cat`, `fl convert` (YAML, JSON, CSV), `fl merge`, `fl split` |
| Understand a data map | `fl graph` (DOT, Mermaid, JSON), `fl stats`, `fl taxonomy diff` |
| Shell integration | `fl completions <shell>`, `fl manpage` |

Full command reference:

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
| `fl validate PATH…` | Check keys, references, duplicates, custom taxonomy and structure; text, JSON, YAML, GitHub annotations or SARIF |
| `fl stats PATH…` | Counts, categorized-field coverage, orphan datasets, key usage (`--rollup`, `--top`) |
| `fl graph PATH…` | Systems ↔ datasets ↔ flows ↔ uses/categories/subjects as DOT, Mermaid or JSON (`--include`, `--focus KEY --depth N`) |
| `fl completions <shell>` / `fl manpage` | Shell completions and man pages |

## What it is not, and known limitations

- **Not a Fides server or client.** `fl` works on files. It does not talk to a Fides instance,
  scan databases or discover data; it checks and transforms the manifests you already have.
- **Not a compliance verdict.** Validation checks labels against the taxonomy and the manifest
  structure. It says nothing about whether the processing described is lawful.
- **One taxonomy version at a time.** Each build embeds one snapshot (currently IAB Tech Lab
  fideslang 3.0.0). Custom `data_category` / `data_use` / `data_subject` resources in your manifest
  set extend it; there is no switch to validate against an older upstream release.
- **Not a byte-for-byte round trip of your formatting.** Output follows upstream's PyYAML layout, so
  comments and custom indentation in input YAML are not preserved.

### Notes on fidelity

- YAML is read with YAML 1.2 semantics: unquoted `yes`/`no`/`on`/`off` stay strings and
  `version_added: 2.0.0` stays a string. Output uses the same layout as upstream's PyYAML export
  (2-space indent, block sequences flush with their key).
- Unknown fields on any resource are preserved through `cat`, `convert`, `merge` and `split`.
- Directory loading unions files in sorted path order, like upstream's `ingest_manifests`.

## How it works

The taxonomy snapshot (`taxonomy/`, exported from upstream's Python source by
`scripts/refresh-taxonomy.sh`) is compiled into the binary, so lookups, suggestions and validation
need no files and no network. Manifest commands load every input file, union the resources by type
(like upstream's `ingest_manifests`), apply `--type` / `--key` filters, and then print, transform,
graph or validate the result.

### Validation rules

`fl validate` runs these rules. Each code has a page with a failing and a passing example and how to
fix it; the index is [docs/rules](docs/rules/README.md).

| Code | Severity | Check |
| --- | --- | --- |
| [E001](docs/rules/E001.md) | error | Unknown data category / use / subject key (with "did you mean" suggestions) |
| [E002](docs/rules/E002.md) | error | Duplicate `fides_key` within a resource type, across all loaded files |
| [E003](docs/rules/E003.md) | error | Dangling reference: `dataset_references`, `ingress`/`egress`, `fides_meta.references`, `after`/`erase_after`, `organization_fides_key` |
| [E004](docs/rules/E004.md) | error | Custom taxonomy `parent_key` missing or not the dotted prefix of the key |
| [E005](docs/rules/E005.md) | error | Custom taxonomy record references itself |
| [E006](docs/rules/E006.md) | error | Invalid `fides_key` syntax (letters, digits, `.`, `_`, `<`, `>`, `-`) |
| [E007](docs/rules/E007.md) | error | Structure: missing required fields, wrong types, unknown resource type |
| [W001](docs/rules/W001.md) | warning | Deprecated key (suggests `replaced_by`) |
| [W002](docs/rules/W002.md) | warning | Dataset field with no `data_categories` |
| [W003](docs/rules/W003.md) | warning | Privacy declaration with no categories or no subjects |
| [W004](docs/rules/W004.md) | warning | `data_purposes`, the deprecated alias of `data_uses` |
| [W005](docs/rules/W005.md) | warning | Field not defined by the upstream models (only with `--strict`) |

Custom `data_category` / `data_use` / `data_subject` resources declared in the manifest set extend
the taxonomy for E001 (disable with `--no-custom-taxonomy`). `--deny CODE` promotes a code to an
error, `--allow CODE` silences it, `-W` treats every warning as an error. An unknown code in
`--deny` / `--allow` is a usage error (exit `2`) that lists the valid codes.

### Building from source

```bash
cargo build
cargo test            # unit + assert_cmd integration tests + insta snapshots
cargo clippy --all-targets -- -D warnings
```

See [CONTRIBUTING.md](./CONTRIBUTING.md) for the layout, how to add a validation rule, and how to
refresh a taxonomy snapshot.

## Output formats and exit codes

| Command | `--format` values (default first) |
| --- | --- |
| `fl taxonomy list` | `table`, `plain`, `json`, `yaml`, `csv` |
| `fl taxonomy cat` | `yaml`, `json`, `csv` |
| `fl taxonomy tree` | `tree`, `dot`, `mermaid`, `json` |
| `fl taxonomy show`, `search`, `diff`, `info` | `text`, `json`, `yaml` |
| `fl cat` | `yaml`, `json`, `tree` |
| `fl convert` (`--to`), `fl merge`, `fl split` | `yaml`, `json`, `csv` (`convert` and `merge` infer from the `-o` extension) |
| `fl validate` | `text`, `json`, `yaml`, `github` (`::error file=…::` annotations for GitHub Actions), `sarif` (SARIF 2.1.0 for GitHub code scanning) |
| `fl stats` | `text`, `json`, `yaml` |
| `fl graph` | `dot`, `mermaid`, `json` |

`fl validate --format json` (and `yaml`) prints `schema_version` (currently `1`; it changes only
when a field is removed, renamed or changes meaning), `taxonomy`, `files`, `resources` and
`diagnostics`; each finding has `code`, `severity`, `file`, `resource`, `path`, `message`, an
optional `suggestion` and `help_uri`, the link to the code's page. `--format sarif` lists every code
as a rule with its `helpUri`; each result carries the file and a line: the line of the resource's
`fides_key` when `fl` can find it, else line 1 (findings read from stdin point at `stdin`). To show the findings in GitHub code scanning:

```yaml
- run: fl validate --format sarif .fides/ -o fl.sarif || [ $? -eq 1 ]
- uses: github/codeql-action/upload-sarif@v3   # pin to a commit hash
  with:
    sarif_file: fl.sarif
```

Manifest commands read YAML, JSON or CSV, detected from the file extension (YAML for stdin);
`--from yaml|json|csv` overrides detection. Global flags:

| Flag | Effect |
| --- | --- |
| `--color auto\|always\|never` | When to color output (also `FL_COLOR`; `auto` honors `NO_COLOR` and `CLICOLOR_FORCE`) |
| `--no-color` | Same as `--color never`; wins over `--color` and `FL_COLOR` |
| `-o FILE` | Write output to `FILE` instead of stdout |
| `-q`, `--quiet` | Suppress the summary lines on stderr |
| `-v`, `--verbose` | Extra diagnostics on stderr: files loaded, resource counts, timings. Stdout is identical with and without it |

When the reader of a pipe goes away (`fl taxonomy list all --format plain | head -1`), `fl` stops
quietly with exit `0`.

Exit codes (details in [docs/exit-codes.md](docs/exit-codes.md)):

| Code | Meaning |
| --- | --- |
| `0` | Success, including `fl validate` with warnings only, and output cut short by a closed pipe |
| `1` | Findings: `fl validate` reported at least one error (warnings count once promoted by `-W` or `--deny`), or `fl taxonomy diff --exit-code` found a difference |
| `2` | Error: invalid command line (including an unknown code in `--deny` / `--allow`), missing, unreadable or unparsable input, I/O failure, or `fl merge --fail-on-duplicate` found a duplicate `fides_key` |

## How do I cite fl?

Cite it with the metadata in [`CITATION.cff`](./CITATION.cff); GitHub's "Cite this repository"
button renders it as APA or BibTeX. Please also cite the IAB Tech Lab Privacy Taxonomy itself when
you rely on its vocabulary.

## Trust

- **Offline.** `fl` makes no network calls and collects no telemetry. It reads the files you name
  and writes only where you tell it to (`-o`, `--out-dir`). Treat manifests and anything derived
  from them as sensitive: a data map describes where personal data lives.
- **Verifiable releases.** Release archives are built by GitHub Actions with
  [dist](https://opensource.axo.dev/cargo-dist/), carry SHA-256 checksums and GitHub artifact
  attestations (see
  [How do I verify a release before running it?](#how-do-i-verify-a-release-before-running-it)),
  and releases after 0.1.1 include a CycloneDX SBOM (`fideslang-cli.cdx.xml`).
- **Supply chain.** CI actions are pinned to commit hashes, dependencies are checked with
  `cargo-deny`, and the repository is scored by
  [OpenSSF Scorecard](https://scorecard.dev/viewer/?uri=github.com/noru-tech/fideslang-tools).
- **Security reports.** Please report vulnerabilities privately; see [SECURITY.md](./SECURITY.md).
- **License.** The Rust code is MIT-licensed (see [LICENSE](./LICENSE)). The vendored IAB Tech Lab
  Privacy Taxonomy under `taxonomy/` and the demo fixtures under `tests/fixtures/demo_resources/`
  remain under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/); see [NOTICE](./NOTICE)
  and [`taxonomy/SOURCE.md`](./taxonomy/SOURCE.md) for attribution.

Maintained by [Noru](https://noru.tech), a continuous compliance platform.

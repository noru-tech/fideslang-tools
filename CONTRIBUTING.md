# Contributing to fideslang-tools

Thanks for your interest in improving `fl`! Bug reports, new subcommands, renderers, validation
rules and docs are all welcome.

## Ground rules

- Be respectful — see [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).
- `fl` is **offline and self-contained**: the taxonomy snapshot is compiled into the binary and
  the tool never touches the network at runtime. Keep it that way.
- Don't hand-edit anything under `taxonomy/`. It is a generated snapshot — see *Updating the taxonomy
  snapshot* below.
- Keep the dependency tree pure Rust (no C libraries) so static musl builds keep working.

## Project layout

```
src/cli/        clap definitions, one file per subcommand
src/taxonomy/   snapshot loading, hierarchy, search, diff
src/manifest/   loading/union of Fides manifest files, typed views, filters
src/format/     YAML / JSON / CSV reading and writing
src/render/     terminal tree, tables, Graphviz DOT, Mermaid
src/graph/      relationship graph built from a manifest set
src/validate/   diagnostics and rules (E0xx errors, W0xx warnings)
src/stats/      counts and usage summaries
taxonomy/       vendored IAB Tech Lab snapshot (CC BY 4.0) + SOURCE.md provenance
tests/          integration tests (assert_cmd) and insta snapshots; fixtures under tests/fixtures
scripts/        refresh-taxonomy.sh + export_taxonomy.py
```

## Development

```bash
cargo build
cargo test
cargo run -- taxonomy tree categories --depth 2
```

Before opening a PR:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Rendering output (tree, DOT, Mermaid, validate text) is covered by [insta](https://insta.rs)
snapshots. If you change output on purpose, review and accept the new snapshots with
`cargo insta review` (install with `cargo install cargo-insta`) and commit the `.snap` files.

Adding a validation rule: add a file under `src/validate/rules/`, give it the next free stable code
(`E0xx` for errors, `W0xx` for warnings), register it in `src/validate/mod.rs`, add a minimal failing
fixture under `tests/fixtures/invalid/`, and document it in the README table.

## Updating the taxonomy snapshot

```bash
scripts/refresh-taxonomy.sh 3.0.0
```

The script installs `fideslang` at that tag from `IABTechLab/fideslang` into a throw-away
virtualenv, exports the taxonomy from the Python source (upstream's `data_files/` directory is a
stale export and must not be used), and writes `taxonomy/snapshot.json`. Then update
`taxonomy/SOURCE.md` and `CHANGELOG.md`, review the diff (a *removed* key can break users' manifests,
so call it out), and run `cargo test` (the counts in `snapshot.json` are asserted).

## Releases

Releases are cut by tagging `vX.Y.Z` on `main`. **Always tag the current head of `main`**: GitHub
refuses to let the workflow token create a release whose target commit is behind a later change to
any `.github/workflows/*.yml` file (it demands a `workflows` scope the built-in token cannot have), so
a tag that trails a workflow edit fails in the `host` job with `HTTP 403: Resource not accessible by
integration`. If that happens, delete the tag and re-tag the head. [cargo-dist](https://opensource.axo.dev/cargo-dist/)
builds the binaries, installer, Homebrew formula and GitHub Release. Run `dist plan` locally after
changing `dist-workspace.toml`, and keep `.github/workflows/release.yml` generated (`dist generate`),
never hand-edited.

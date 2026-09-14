# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security
- `fl split` now refuses resource types and `fides_key`s that are not plain file names (path
  separators, `.`, `..`), so a crafted manifest can no longer write files outside `--out-dir`.

## [0.1.0] - 2026-09-13

### Added
- Initial `fl` command-line tool for working with Fideslang taxonomies and Fides manifests.
- Bundled, offline snapshot of the IAB Tech Lab Privacy Taxonomy (IABTechLab/fideslang 3.0.0).
- `fl taxonomy list|cat|tree|show|search|diff|info` for browsing and visualizing the taxonomy.
- `fl cat|convert|merge|split` for reading and transforming manifests between YAML, JSON and CSV.
- `fl validate` with stable diagnostic codes and "did you mean" suggestions; `fl stats`.
- `fl graph` rendering system/dataset/data-use relationships as Graphviz DOT or Mermaid.
- Shell completions (`fl completions`) and man pages (`fl manpage`).

[Unreleased]: https://github.com/noru-tech/fideslang-tools/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/noru-tech/fideslang-tools/releases/tag/v0.1.0

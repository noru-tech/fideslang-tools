# Validation rules

`fl validate` checks Fides manifests against the bundled IAB Tech Lab Privacy Taxonomy (fideslang
3.0.0) and against the manifest structure, and reports each finding with a stable code: a code keeps its meaning
across releases, so you can pin `--deny` / `--allow` lists and CI filters to it.

| Code | Severity | Rule |
| --- | --- | --- |
| [E001](E001.md) | error | A data category, data use or data subject key is not in the taxonomy |
| [E002](E002.md) | error | Two resources of the same type share a `fides_key` |
| [E003](E003.md) | error | A reference points at a dataset, system, collection or organization that is not loaded |
| [E004](E004.md) | error | A custom taxonomy record's `parent_key` is missing or is not the dotted prefix of its key |
| [E005](E005.md) | error | A custom taxonomy record names itself as its parent or replacement |
| [E006](E006.md) | error | A `fides_key` contains characters other than letters, digits, `.`, `_`, `<`, `>` and `-` |
| [E007](E007.md) | error | Structure: a required field is missing, a value has the wrong type, or the resource type is unknown |
| [W001](W001.md) | warning | A taxonomy key is deprecated |
| [W002](W002.md) | warning | A dataset field has no `data_categories` |
| [W003](W003.md) | warning | A privacy declaration has no `data_categories` or no `data_subjects` |
| [W004](W004.md) | warning | A resource uses `data_purposes` instead of `data_uses` |
| [W005](W005.md) | warning | A resource has a field the upstream models do not define (only with `--strict`) |

## Reading a finding

Each finding names the file, the resource and the path inside it, then the code and the message:

```console
$ fl validate tests/fixtures/invalid/w004_data_purposes.yml
tests/fixtures/invalid/w004_data_purposes.yml  dataset[legacy_purposes].data_purposes[0]
  W004 `data_purposes` is deprecated; use `data_uses`
1 files, 1 resources checked against iab 3.0.0: 0 errors, 1 warning
```

`--format json` prints the same findings as objects with `code`, `severity`, `file`, `resource`,
`path`, `message` and, when there is one, `suggestion`. `--format github` prints GitHub Actions
annotations whose title starts with the code.

## Errors, warnings and the exit code

Errors make `fl validate` exit with `1`; warnings alone exit `0`. See [exit codes](../exit-codes.md).

## Promoting and silencing codes

These flags apply to the whole run. There is no inline or per-file suppression.

| Flag | Effect |
| --- | --- |
| `--deny CODE[,CODE…]` | Report these codes as errors, so they fail the run |
| `-W`, `--warnings-as-errors` | Report every warning (W001–W005) as an error |
| `--allow CODE[,CODE…]` | Drop these codes from the report entirely |
| `--strict` | Also run W005 |
| `--no-custom-taxonomy` | Ignore `data_category` / `data_use` / `data_subject` resources in the manifests when checking E001 |

Codes are case-insensitive (`--allow w002` works); an unknown code is a usage error (exit `2`) whose
message lists the valid codes. `--allow` wins over `--deny` and `-W` for the same
code. `--type` and `--key` narrow which resources are loaded, which also narrows what is checked; a
reference to a resource you filtered out is then reported as [E003](E003.md).

## Examples on these pages

Every failing example is a fixture under
[`tests/fixtures/invalid/`](../../tests/fixtures/invalid/) and every passing example is its corrected
copy under [`tests/fixtures/valid/`](../../tests/fixtures/valid/). The output shown is what
`fl validate` prints for them (run from the repository root). `cargo test` checks that each failing
fixture still raises its code and that every passing fixture is clean, even with `--strict`.

## Adding a rule

See [CONTRIBUTING.md](../../CONTRIBUTING.md#development): add the rule, give it the next free code,
add a failing and a passing fixture, and add a page here. `cargo test` fails if a code has no page.

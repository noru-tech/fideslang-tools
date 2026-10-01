# Exit codes

`fl` exits with `0` on success, `1` when a check reports findings, and `2` when it could not do what
was asked. These three values are stable across releases, so scripts and CI can branch on them.

| Code | Meaning | Returned by |
| --- | --- | --- |
| `0` | Success | Every command that completes. `fl validate` with no errors, including a run with warnings only. `--help` and `--version`. |
| `1` | Findings | `fl validate` reported at least one error. Warnings count once promoted with `-W` or `--deny CODE`. `fl taxonomy diff --exit-code` found a difference. |
| `2` | Error | Invalid command line (unknown subcommand, flag or value). Unreadable, missing or unparsable input. No path given and no `./.fides/` directory. I/O failure, such as an output file that cannot be created. `fl merge --fail-on-duplicate` found a duplicate `fides_key`. |

The codes are defined once, in the `Exit` type in [`src/lib.rs`](../src/lib.rs); `fl --help` prints
a one-line summary of them.

## Examples

```console
$ fl validate tests/fixtures/invalid/e001_unknown_key.yml > /dev/null; echo $?
1
$ fl validate tests/fixtures/invalid/w004_data_purposes.yml > /dev/null; echo $?
0
$ fl validate -W tests/fixtures/invalid/w004_data_purposes.yml > /dev/null; echo $?
1
$ fl validate nope.yml; echo $?
error: nope.yml: no such file or directory
2
```

## Using them in CI

```bash
fl validate .fides/                 # fails the job on any error (exit 1) or problem (exit 2)
fl validate -W .fides/              # also fails on warnings
fl validate .fides/ || [ $? -eq 1 ] # report findings without failing, but still fail on exit 2
```

On GitHub Actions, `fl validate --format github .fides/` prints the findings as annotations on the
pull request diff and exits the same way.

## Error messages

Errors that end a run with `2` are printed to stderr as `error: ` followed by the cause. Findings are
not errors: `fl validate` prints them to stdout (or to `-o FILE`) in the chosen `--format`, and only
the exit code `1` tells a script that there were any. See [validation rules](rules/README.md) for what
each finding means.

[README](../README.md#output-formats-and-exit-codes) · [Validation rules](rules/README.md)

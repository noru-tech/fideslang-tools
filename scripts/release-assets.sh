#!/usr/bin/env bash
# Generate the shell completions and man pages that go into every release archive.
#
#   scripts/release-assets.sh [OUT_DIR]     (default: target/release-assets)
#
# Writes OUT_DIR/completions/{fl.bash,_fl,fl.fish} and OUT_DIR/man/*.1, and fails if any of them
# is missing or empty. Uses $FL if set, otherwise builds and runs `fl` with cargo.
#
# The release workflow runs this before `dist build` (see .github/build-setup.yml), and
# dist-workspace.toml `include`s both directories in each archive; the Homebrew formula installs
# them (see scripts/homebrew-formula-extras.sh). CI runs it too, so a broken generator fails a PR.
set -euo pipefail

out="${1:-target/release-assets}"
if [ -n "${FL:-}" ]; then
  fl=("$FL")
else
  fl=(cargo run --locked --quiet --bin fl --)
fi

rm -rf "$out/completions" "$out/man"
mkdir -p "$out/completions" "$out/man"
"${fl[@]}" --color never completions bash > "$out/completions/fl.bash"
"${fl[@]}" --color never completions zsh > "$out/completions/_fl"
"${fl[@]}" --color never completions fish > "$out/completions/fl.fish"
"${fl[@]}" --color never -q manpage --out-dir "$out/man"

status=0
for f in "$out/completions/fl.bash" "$out/completions/_fl" "$out/completions/fl.fish" "$out/man/fl.1" "$out/man/fl-validate.1"; do
  if [ ! -s "$f" ]; then
    echo "error: $f is missing or empty" >&2
    status=1
  fi
done
for f in "$out"/man/*.1; do
  [ -s "$f" ] || { echo "error: $f is empty" >&2; status=1; }
done
[ "$status" -eq 0 ] && echo "release assets in $out: $(ls "$out/completions" | tr '\n' ' ')+ $(ls "$out/man" | wc -l | tr -d ' ') man pages"
exit "$status"

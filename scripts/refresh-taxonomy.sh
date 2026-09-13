#!/usr/bin/env bash
# Refresh the vendored IAB Tech Lab Privacy Taxonomy snapshot from a pinned upstream release tag.
#
#   scripts/refresh-taxonomy.sh 3.0.0
#
# Installs `fideslang` at that tag from github.com/IABTechLab/fideslang into a throw-away
# virtualenv, exports the default taxonomy with scripts/export_taxonomy.py into taxonomy/, and
# records provenance in taxonomy/snapshot.json. Requires python3 and gh (authenticated).
#
# Why not download upstream's data_files/*.yml? They are a stale export (last regenerated
# 2023-12); the Python source under src/fideslang/default_taxonomy/ is the source of truth.
# See taxonomy/SOURCE.md.
set -euo pipefail

tag="${1:-}"
[[ -n "$tag" ]] || { echo "usage: $0 <tag>   (e.g. 3.0.0)" >&2; exit 2; }
org="IABTechLab"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="$repo_root/taxonomy"
venv="$(mktemp -d)/venv"
trap 'rm -rf "$(dirname "$venv")"' EXIT

echo "> installing fideslang@$tag from github.com/$org/fideslang into a temp venv"
python3 -m venv "$venv"
"$venv/bin/pip" install --quiet "fideslang @ git+https://github.com/$org/fideslang@$tag" pyyaml

echo "> exporting taxonomy to $out_dir"
counts="$("$venv/bin/python" "$repo_root/scripts/export_taxonomy.py" "$out_dir" | tail -n1)"

commit="$(gh api "repos/$org/fideslang/git/ref/tags/$tag" --jq .object.sha)"
# annotated tags point at a tag object; dereference to the commit
if [[ "$(gh api "repos/$org/fideslang/git/ref/tags/$tag" --jq .object.type)" == "tag" ]]; then
  commit="$(gh api "repos/$org/fideslang/git/tags/$commit" --jq .object.sha)"
fi

python3 - "$out_dir/snapshot.json" "$org" "$tag" "$commit" "$counts" <<'PY'
import datetime, json, sys
path, org, tag, commit, counts = sys.argv[1:]
doc = {
    "snapshot": "iab",
    "upstream": f"https://github.com/{org}/fideslang",
    "tag": tag,
    "commit": commit,
    "snapshot_date": datetime.date.today().isoformat(),
    "license": "CC-BY-4.0",
    "counts": json.loads(counts),
}
with open(path, "w", encoding="utf-8") as fh:
    json.dump(doc, fh, indent=2)
    fh.write("\n")
print(f"> wrote {path}")
PY

echo
echo "Done. Now: review 'git diff taxonomy/', update taxonomy/SOURCE.md and CHANGELOG.md, run cargo test."

#!/usr/bin/env bash
# Refresh a vendored Fideslang taxonomy snapshot from a pinned upstream release tag.
#
#   scripts/refresh-taxonomy.sh ethyca 3.1.4
#   scripts/refresh-taxonomy.sh iab    3.0.0
#
# Installs `fideslang` at that tag into a throw-away virtualenv, exports the default
# taxonomy with scripts/export_taxonomy.py into taxonomy/<snapshot>/, and records
# provenance in taxonomy/<snapshot>/snapshot.json. Requires python3 and gh (authenticated).
#
# Why not download upstream's data_files/*.yml? They are a stale export (last
# regenerated 2023-12); the Python source under src/fideslang/default_taxonomy/ is the
# source of truth. See taxonomy/SOURCE.md.
set -euo pipefail

snapshot="${1:-}"; tag="${2:-}"
case "$snapshot" in
  ethyca) org="ethyca" ;;
  iab)    org="IABTechLab" ;;
  *) echo "usage: $0 <ethyca|iab> <tag>" >&2; exit 2 ;;
esac
[[ -n "$tag" ]] || { echo "usage: $0 <ethyca|iab> <tag>" >&2; exit 2; }

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="$repo_root/taxonomy/$snapshot"
venv="$(mktemp -d)/venv"
trap 'rm -rf "$(dirname "$venv")"' EXIT

echo "> installing fideslang@$tag from github.com/$org/fideslang into a temp venv"
python3 -m venv "$venv"
"$venv/bin/pip" install --quiet "fideslang @ git+https://github.com/$org/fideslang@$tag"

echo "> exporting taxonomy to $out_dir"
counts="$("$venv/bin/python" "$repo_root/scripts/export_taxonomy.py" "$out_dir" | tail -n1)"

commit="$(gh api "repos/$org/fideslang/git/ref/tags/$tag" --jq .object.sha)"
# annotated tags point at a tag object; dereference to the commit
if [[ "$(gh api "repos/$org/fideslang/git/ref/tags/$tag" --jq .object.type)" == "tag" ]]; then
  commit="$(gh api "repos/$org/fideslang/git/tags/$commit" --jq .object.sha)"
fi

python3 - "$out_dir/snapshot.json" "$snapshot" "$org" "$tag" "$commit" "$counts" <<'PY'
import datetime, json, sys
path, snapshot, org, tag, commit, counts = sys.argv[1:]
doc = {
    "snapshot": snapshot,
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
echo "Done. Now: review 'git diff taxonomy/', update taxonomy/SOURCE.md and CHANGELOG.md."

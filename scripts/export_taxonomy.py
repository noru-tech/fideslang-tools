#!/usr/bin/env python3
"""Export the default Fideslang taxonomy from an installed `fideslang` package as YAML.

Run inside a virtualenv where `fideslang` is installed at the pinned upstream tag
(see scripts/refresh-taxonomy.sh). Reproduces upstream's own
`scripts/export_default_taxonomy.py` layout (one file per resource type, singular
top-level key, insertion order preserved) but from the *current* Python source rather
than the stale `data_files/` directory.

Usage: export_taxonomy.py OUT_DIR
"""
import json
import pathlib
import sys

import yaml
from fideslang.default_taxonomy import DEFAULT_TAXONOMY

RESOURCES = (
    ("data_categories", "data_category"),
    ("data_uses", "data_use"),
    ("data_subjects", "data_subject"),
)


def dump(model):
    if hasattr(model, "model_dump"):  # pydantic v2
        return model.model_dump(mode="json")
    return json.loads(model.json())  # pydantic v1 fallback


def main(out_dir: str) -> int:
    out = pathlib.Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    counts = {}
    for filename, resource_type in RESOURCES:
        rows = [dump(x) for x in getattr(DEFAULT_TAXONOMY, resource_type)]
        counts[resource_type] = len(rows)
        with (out / f"{filename}.yml").open("w", encoding="utf-8") as fh:
            yaml.dump({resource_type: rows}, fh, sort_keys=False, indent=2,
                      allow_unicode=True, width=100000)
        print(f"> wrote {out / filename}.yml ({len(rows)} entries)")
    print(json.dumps(counts))
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    raise SystemExit(main(sys.argv[1]))

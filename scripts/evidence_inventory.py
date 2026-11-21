"""Measure a named commit's tracked evidence using an isolated git ls-files index."""
import argparse
from collections import defaultdict
import json
import os
from pathlib import Path
import subprocess
import tempfile

import evidence_store as store


def inventory(root, revision):
    commit = store.git(root, "rev-parse", revision).decode().strip()
    with tempfile.TemporaryDirectory() as scratch:
        environment = dict(os.environ, GIT_INDEX_FILE=str(Path(scratch) / "index"))
        subprocess.run(["git", "read-tree", commit], cwd=root, env=environment, check=True)
        records = subprocess.check_output(["git", "ls-files", "--stage", "-z"], cwd=root, env=environment).decode().split("\0")
    objects = []
    for record in filter(None, records):
        header, path = record.split("\t", 1)
        objects.append((header.split()[1], path))
    sizes = subprocess.check_output(["git", "cat-file", "--batch-check=%(objectsize)"], cwd=root,
                                    input=("\n".join(blob for blob, _ in objects) + "\n").encode()).decode().splitlines()
    rows = [{"path": path, "size": int(size)} for (_, path), size in zip(objects, sizes)]
    evidence = [row for row in rows if row["path"].startswith(store.FOLDER)]
    registry = json.loads(store.git(root, "show", f"{commit}:.config/exact-baselines.json"))
    references = [item["reference"] for item in registry["baselines"]]
    caps = {}
    for cap in (2 * 1024**2, 1024**2, 512 * 1024):
        selected = [row for row in evidence if row["size"] > cap and not any(
            row["path"] == ref or ref.endswith("/") and row["path"].startswith(ref) for ref in references)]
        types = defaultdict(lambda: {"count": 0, "bytes": 0})
        for row in selected:
            group = types[Path(row["path"]).suffix or "no-extension"]
            group["count"] += 1
            group["bytes"] += row["size"]
        caps[str(cap)] = {"count": len(selected), "bytes": sum(row["size"] for row in selected), "by_type": dict(sorted(types.items()))}
    return {"commit": commit, "method": "git read-tree COMMIT (isolated index); git ls-files --stage -z; git cat-file --batch-check",
            "tracked": {"count": len(rows), "bytes": sum(row["size"] for row in rows)},
            "evidence": {"count": len(evidence), "bytes": sum(row["size"] for row in evidence)}, "caps_excluding_exact_references": caps}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("revision")
    args = parser.parse_args()
    print(json.dumps(inventory(store.ROOT, args.revision), indent=2))

"""Recover bulk evidence from Git history or the shared, machine-local store.

restore PATH writes an ignored copy; put SOURCE [--path PATH] retains new bytes.
archive moves tracked oversized evidence to the history manifest without rewriting Git.
The one manifest is tracked under .config/; nothing is written under docs/evidence except restored copies,
which the root .gitignore keeps untracked.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ".config/evidence-manifest.json"
# The tracked evidence folder, which only this store and its restored copies use.
FOLDER = "docs/evidence/"
CAP = 2 * 1024 * 1024


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args])


def evidence_path(value):
    path = Path(value)
    if path.is_absolute() or ".." in path.parts or not value.startswith("docs/evidence/") or path.as_posix() != value:
        raise ValueError(f"invalid evidence path: {value}")
    return value


def destination(root, value):
    path = root / evidence_path(value)
    if not path.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"evidence path escapes repository: {value}")
    return path


def load(root, manifest=MANIFEST):
    path = root / manifest
    if not path.exists():
        return {"schema": 1, "entries": []}
    data = json.loads(path.read_text())
    if data.get("schema") != 1 or not isinstance(data.get("entries"), list):
        raise ValueError("invalid bulk manifest schema")
    seen = set()
    for entry in data["entries"]:
        name = evidence_path(entry["path"])
        if name in seen:
            raise ValueError(f"duplicate bulk path: {name}")
        seen.add(name)
        if not isinstance(entry["size"], int) or entry["size"] < 0 or not re.fullmatch(r"[0-9a-f]{64}", entry["sha256"]):
            raise ValueError(f"invalid bulk size/hash: {name}")
        if "relocated" in entry and (entry["kind"] != "history" or entry["relocated"] is not True):
            raise ValueError(f"invalid relocated mark: {name}")
        if entry["kind"] == "history":
            if not all(re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", entry[key]) for key in ("blob", "commit")):
                raise ValueError(f"invalid Git identity: {name}")
        elif entry["kind"] == "local":
            if "blob" in entry or "commit" in entry or not entry.get("date") or not entry.get("store"):
                raise ValueError(f"invalid local entry: {name}")
        else:
            raise ValueError(f"unknown bulk kind: {name}")
    return data


def entries(root):
    """History and local entries, including the historical link targets of evidence that became test data."""
    return load(root)["entries"]


def save(root, data):
    path = root / MANIFEST
    path.parent.mkdir(parents=True, exist_ok=True)
    data["entries"].sort(key=lambda entry: entry["path"])
    path.write_text(json.dumps(data, indent=2) + "\n")


def store(root):
    if os.environ.get("THRUM_EVIDENCE_STORE"):
        return Path(os.environ["THRUM_EVIDENCE_STORE"]).expanduser().resolve()
    common = Path(git(root, "rev-parse", "--git-common-dir").decode().strip())
    common = (root / common).resolve()
    return common.parent.parent / "thrum-evidence-store"


def tracked(root):
    return {p for p in git(root, "ls-files", "-z").decode().split("\0") if p}


def references(root):
    path = root / ".config/exact-baselines.json"
    if not path.exists():
        return []
    return [item["reference"] for item in json.loads(path.read_text())["baselines"]]


def oversized(root, paths):
    refs = references(root)
    return sorted(p for p in paths if p.startswith("docs/evidence/")
                  and (root / p).is_file() and (root / p).stat().st_size > CAP
                  and not any(p == ref or (ref.endswith("/") and p.startswith(ref)) for ref in refs))


def verified(entry, data):
    if len(data) != entry["size"] or hashlib.sha256(data).hexdigest() != entry["sha256"]:
        raise ValueError(f"size/SHA-256 mismatch: {entry['path']}")
    return data


def content(root, entry):
    if entry["kind"] == "history":
        blob = git(root, "rev-parse", f"{entry['commit']}:{entry['path']}").decode().strip()
        if blob != entry["blob"]:
            raise ValueError(f"commit/path does not match blob: {entry['path']}")
        return verified(entry, git(root, "cat-file", "blob", entry["blob"]))
    location = store(root) / entry["sha256"]
    if not location.is_file():
        raise FileNotFoundError(f"{entry['path']}: not on this machine; originally put in {entry['store']}; current store {store(root)}")
    return verified(entry, location.read_bytes())


def check(root):
    errors = [f"{p}: evidence-size: over 2 MiB; keep captures in tmp/<task>/ with reproduction record or use evidence_store.py put SOURCE --path {p}"
              for p in oversized(root, tracked(root))]
    for entry in entries(root):
        try:
            content(root, entry)
        except FileNotFoundError as error:
            print(error)
        except (ValueError, subprocess.CalledProcessError) as error:
            errors.append(f"{entry['path']}: {error}")
    return errors


def atomic_write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as temporary:
        name = Path(temporary.name)
        temporary.write(data)
    try:
        name.replace(path)
    finally:
        name.unlink(missing_ok=True)


def restore(root, value):
    path = destination(root, value)
    entry = next((entry for entry in entries(root) if entry["path"] == value), None)
    if entry is None:
        raise ValueError(f"{value}: not listed in history or local store manifest")
    data = content(root, entry)
    if value in tracked(root):
        raise ValueError(f"refusing to overwrite tracked file: {value}")
    if path.exists() and path.read_bytes() != data:
        raise ValueError(f"refusing to overwrite different local bytes: {value}")
    atomic_write(path, data)
    print(f"restored {value}: {entry['sha256']}")


def put(root, source, value):
    destination(root, value)
    if value in tracked(root):
        raise ValueError(f"{value}: already tracked; new bulk captures must stay out of Git")
    if any(entry["path"] == value and entry.get("relocated") for entry in load(root)["entries"]):
        raise ValueError(f"{value}: reserved for historical evidence; choose a new path")
    data = Path(source).read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    manifest = load(root)
    previous = next((entry for entry in manifest["entries"] if entry["path"] == value), None)
    if previous:
        if previous["sha256"] != digest:
            raise ValueError(f"{value}: manifest path already identifies different bytes; choose a new path")
    else:
        manifest["entries"].append({"kind": "local", "path": value, "size": len(data), "sha256": digest,
                                    "date": datetime.now(timezone.utc).isoformat(), "store": str(store(root))})
    atomic_write(store(root) / digest, data)
    save(root, manifest)
    print(f"stored {value}: {digest} in {store(root)} (local only, no remote backup)")


def archive(root):
    manifest = load(root)
    commit = git(root, "rev-parse", "HEAD").decode().strip()
    paths = oversized(root, tracked(root))
    known = {entry["path"] for entry in manifest["entries"]}
    entries = []
    for path in paths:
        if path in known:
            raise ValueError(f"already listed: {path}")
        data = (root / path).read_bytes()
        blob = git(root, "rev-parse", f"{commit}:{path}").decode().strip()
        entry = {"kind": "history", "path": path, "size": len(data), "sha256": hashlib.sha256(data).hexdigest(),
                 "blob": blob, "commit": commit}
        content(root, entry)  # Confirm durable recovery before removing anything.
        entries.append(entry)
    manifest["entries"].extend(entries)
    save(root, manifest)
    for path in paths:
        git(root, "rm", "--", path)  # Refuses local modifications.
    print(f"archived {len(paths)} files, {sum(entry['size'] for entry in entries)} bytes at {commit}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("check")
    commands.add_parser("archive")
    commands.add_parser("restore").add_argument("path")
    put_parser = commands.add_parser("put")
    put_parser.add_argument("source")
    put_parser.add_argument("--path", help="logical docs/evidence path, required for a scratch source")
    args = parser.parse_args()
    try:
        if args.command == "check":
            errors = check(ROOT)
            if errors:
                raise ValueError("\n".join(errors))
            print("evidence_store: check passed")
        elif args.command == "restore":
            restore(ROOT, args.path)
        elif args.command == "put":
            put(ROOT, args.source, args.path or args.source)
        else:
            archive(ROOT)
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print(f"evidence_store: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

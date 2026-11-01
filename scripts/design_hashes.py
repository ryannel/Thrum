"""Maintain only current design digests; never rewrite historical reviews or maturity."""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def current_slots(ledger):
    """Yield (containing object, key, design-relative file) for live hash fields."""
    yield ledger, "contract_sha256", "CONTRACTS.md"
    if "i1_headless" in ledger:
        row = ledger["i1_headless"]
        yield row, "design_sha256", row["design"]
    for row in ledger["modules"]:
        yield row, "current_design_sha256", row["design"]
        for name in row.get("initial_specifications", {}):
            yield row["initial_specifications"], name, name
    row = ledger["composition_supplement"]
    yield row, "current_design_sha256", row["design"]
    row = ledger["architecture_resolution"]
    yield row, "record_sha256", row["record"]
    yield row, "contract_sha256", "CONTRACTS.md"
    for name in row["specifications"]:
        yield row["specifications"], name, name
    for field in ("simplification_review", "reconciliation"):
        row = ledger[field]
        yield row, "record_sha256", row["record"]
    for name in ledger["reconciliation"]["current_files"]:
        yield ledger["reconciliation"]["current_files"], name, name


def check_hashes(root, ledger):
    errors = []
    for obj, key, name in current_slots(ledger):
        path = root / "docs/design" / name
        if not path.is_file():
            errors.append(f"missing design: {name}")
        elif obj[key] != digest(path):
            errors.append(f"stale current design digest: {name}")
    return sorted(set(errors))


def main():
    path = ROOT / "docs/design/maturity.json"
    ledger = json.loads(path.read_text())
    if sys.argv[1:] == ["--write"]:
        for obj, key, name in current_slots(ledger):
            obj[key] = digest(ROOT / "docs/design" / name)
        path.write_text(json.dumps(ledger, indent=2) + "\n")
        print("Refreshed current design hashes; review history and maturity unchanged.")
    elif not sys.argv[1:]:
        errors = check_hashes(ROOT, ledger)
        if errors:
            raise SystemExit("\n".join(errors))
        print("Current design hashes match.")
    else:
        raise SystemExit("usage: design_hashes.py [--write]")


if __name__ == "__main__":
    main()

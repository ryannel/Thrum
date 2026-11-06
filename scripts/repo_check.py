"""Dependency-free checks for the repository's documented conventions.

Markdown scope: inline local links and ATX headings/explicit HTML anchors, outside
fenced code. Remote URLs are not fetched. This is not a general Markdown parser.
"""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib
from urllib.parse import unquote, urlsplit

from design_hashes import check_hashes

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"\[[^\]\n]*\]\(([^)\n]+)\)")
IOS_FORBIDDEN_RUNTIME_PACKAGES = {
    "thrum-cook", "thrum-source", "rusqlite", "libsqlite3-sys",
    # G5 authoring closure: importers, encoders and the watcher never ship.
    "meshopt", "intel_tex_2", "astcenc-rs", "gltf", "notify",
}


def prose(text):
    return re.sub(r"(?ms)^\s*(`{3,}|~{3,}).*?^\s*\1\s*$", "", text)


def anchors(text):
    text = prose(text)
    result = set(re.findall(r'<a\s+id=[\"\x27]([^\"\x27]+)', text))
    counts = {}
    for heading in re.findall(r"(?m)^#{1,6}\s+(.+?)\s*#*\s*$", text):
        name = re.sub(r"[^\w\- ]", "", heading.lower()).replace(" ", "-")
        count = counts.get(name, 0)
        counts[name] = count + 1
        result.add(name if count == 0 else f"{name}-{count}")
    return result


def check_links(root, files):
    errors = []
    count = 0
    for path in files:
        for match in LINK.finditer(prose(path.read_text(encoding="utf-8"))):
            value = match[1].strip()
            if value.startswith("<"):
                value = value[1:value.index(">")]
            else:
                value = value.split(' "', 1)[0]
            url = urlsplit(value)
            if url.scheme or url.netloc:
                continue
            dest = (path.parent / unquote(url.path)).resolve() if url.path else path
            count += 1
            label = f"{path.relative_to(root)}: {value}"
            if not dest.is_relative_to(root) or not dest.exists():
                errors.append(f"missing/outside local link: {label}")
            elif url.fragment and dest.suffix == ".md" and unquote(url.fragment) not in anchors(dest.read_text(encoding="utf-8")):
                errors.append(f"missing anchor: {label}")
    return errors, count


def check_packages(root, metadata, policy):
    errors = []
    members = set(metadata["workspace_members"])
    packages = {p["name"]: p for p in metadata["packages"] if p["id"] in members}
    configured = policy["packages"]
    if len(packages) != len(members):
        errors.append("workspace package names must be unique")
    if packages.keys() != configured.keys():
        errors.append(f"package registry mismatch: actual={sorted(packages)}, policy={sorted(configured)}")
    manifests = {str(Path(p["manifest_path"]).resolve().parent): name for name, p in packages.items()}
    boundary = policy.get("boundaries", {})
    game = boundary.get("external_game")
    if game is not None and (
        game not in packages
        or (root / boundary.get("game_manifest", "")).resolve()
        != Path(packages[game]["manifest_path"]).resolve()
    ):
        errors.append("V1 external game must be registered at its declared manifest")
    external = policy.get("external_dependencies", {})
    used_external = set()
    for name, package in packages.items():
        if name not in configured:
            continue
        entry = configured[name]
        manifest_path = Path(package["manifest_path"]).resolve()
        if not manifest_path.is_relative_to(root.resolve()):
            errors.append(f"workspace manifest outside repository: {name}")
        if manifest_path != (root / entry["manifest"]).resolve():
            errors.append(f"unexpected manifest path: {name}")
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        if manifest.get("lints") != {"workspace": True}:
            errors.append(f"{name} must inherit workspace lints")
        for key in ("version", "edition", "rust-version", "publish"):
            if manifest["package"].get(key) != {"workspace": True}:
                errors.append(f"{name} must inherit package.{key}")
        allowed = set(entry["allowed_workspace_dependencies"])
        if allowed - packages.keys():
            errors.append(f"unknown dependency in policy for {name}: {sorted(allowed - packages.keys())}")
        actual_edges = set()
        # Cargo metadata preserves every declared dependency, including dev/build/target/rename.
        for dep in package["dependencies"]:
            dep_path = dep.get("path")
            if dep_path:
                dependency = manifests.get(str(Path(dep_path).resolve()))
                if dependency is None:
                    errors.append(f"{name}: path dependency outside registered workspace: {dep_path}")
                else:
                    actual_edges.add(dependency)
                    if dependency == game and name != game and name != "v1-game-ios":
                        errors.append(f"engine package depends on external game: {name}")
                    if dependency not in allowed:
                        errors.append(f"forbidden workspace dependency: {name} -> {dependency}")
            else:
                dependency = dep["name"]
                used_external.add(dependency)
                declared = external.get(dependency)
                if declared is None:
                    errors.append(f"unregistered external dependency: {name} -> {dependency}")
                    continue
                required = {"requirement", "license", "purpose", "target_cost"}
                missing = required - declared.keys()
                if missing or any(not str(declared.get(key, "")).strip() for key in required):
                    errors.append(f"incomplete external dependency policy: {dependency}: {sorted(missing)}")
                if dep.get("source") is None or not str(dep["source"]).startswith("registry+"):
                    errors.append(f"external dependency must use a pinned registry release: {dependency}")
                if dep.get("req") != declared.get("requirement") or not str(dep.get("req", "")).startswith("="):
                    errors.append(f"external dependency must use declared exact version: {dependency}")
        if allowed - actual_edges:
            errors.append(f"unused workspace dependency policy for {name}: {sorted(allowed - actual_edges)}")
    if external.keys() - used_external:
        errors.append(f"unused external dependency policy: {sorted(external.keys() - used_external)}")
    return errors


def check_unused_direct_dependencies(metadata, policy):
    """Reject a direct dependency whose crate name is never referenced in package Rust source."""
    errors = []
    members = set(metadata["workspace_members"])
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        directory = Path(package["manifest_path"]).parent
        implicit = policy["packages"][package["name"]].get("implicit_dependencies", {})
        declared = {
            (dependency.get("rename") or dependency["name"])
            for dependency in package["dependencies"]
        }
        for name, reason in implicit.items():
            if name not in declared or not reason.strip():
                errors.append(f"invalid implicit dependency record: {package['name']} -> {name}")
        source = "\n".join(
            path.read_text(encoding="utf-8")
            for path in directory.rglob("*.rs")
            if "target" not in path.parts
        )
        for dependency in package["dependencies"]:
            name = (dependency.get("rename") or dependency["name"]).replace("-", "_")
            if dependency["name"] not in implicit and not re.search(r"\b" + re.escape(name) + r"\s*::", source):
                errors.append(f"unused direct dependency: {package['name']} -> {name}")
    return errors


def check_manifest_inventory(root, metadata):
    """Every package manifest under owned crate/tool roots joins the workspace."""
    members = set(metadata["workspace_members"])
    registered = {Path(p["manifest_path"]).resolve() for p in metadata["packages"] if p["id"] in members}
    discovered = set()
    for owned in (root / "crates", root / "tools", root / "examples"):
        if not owned.exists():
            continue
        for directory, names, files in os.walk(owned):
            names[:] = [n for n in names if n not in {"target", "tmp", ".git"}]
            if "Cargo.toml" in files:
                discovered.add((Path(directory) / "Cargo.toml").resolve())
    errors = []
    for path in sorted(discovered - registered):
        display = path.relative_to(root.resolve()).as_posix() if path.is_relative_to(root.resolve()) else str(path)
        errors.append(f"unregistered package manifest: {display}")
    return errors


def check_ios_runtime_closure(root, channel):
    """Keep authoring and SQLite out of the normal iOS game dependency graph."""
    command = [
        "rustup", "run", channel, "cargo", "tree", "--package", "v1-game-ios",
        "--target", "aarch64-apple-ios", "--edges", "normal", "--prefix", "none",
        "--color", "never", "--locked", "--offline",
    ]
    result = subprocess.run(command, cwd=root, capture_output=True, text=True, check=False)
    if result.returncode:
        return [f"iOS runtime dependency tree failed: {result.stderr.strip()}"]
    present = {line.split(" ", 1)[0] for line in result.stdout.splitlines()}
    return [
        f"iOS runtime includes authoring package: {name}"
        for name in sorted(present & IOS_FORBIDDEN_RUNTIME_PACKAGES)
    ]


def check_skills(root):
    errors = []
    for path in (root / ".agents/skills").glob("*/SKILL.md"):
        match = re.match(r"---\n(.*?)\n---", path.read_text(encoding="utf-8"), re.S)
        if not match:
            errors.append(f"missing skill metadata: {path.name}")
            continue
        fields = dict(re.findall(r"^(name|description): (.+)$", match[1], re.M))
        if fields.get("name") != path.parent.name or not fields.get("description"):
            errors.append(f"invalid skill name/description: {path.parent.name}")
    return errors


def check_maturity(ledger):
    errors = []
    ids = [row["id"] for row in ledger["modules"]]
    contracts = ledger["contracts"]
    if len(ids) != len(set(ids)) or len(contracts) != len(set(contracts)):
        errors.append("duplicate module/contract ID")
    for row in ledger["modules"]:
        if set(row["contracts"]) - set(contracts):
            errors.append(f"unknown contract reference: {row['id']}")
        for field, values in ledger["state_vocabulary"].items():
            if field in row and row[field] not in values:
                errors.append(f"invalid {field} for {row['id']}")
    return errors


RUST_ROOTS = ("crates", "examples", "tools")
TEST_MOD = re.compile(r"(pub(\([^)]*\))?\s+)?mod\s+\w+\s*[;{]")
BARE_IGNORE = re.compile(r"#\[ignore\]")
# Size backstops count code lines: no blanks, comments or inline test modules. A note is a
# prompt to check cohesion, not an instruction to split; the limit stops unbounded growth.
# Small limits fragment coherent code, so these stay deliberately generous.
FILE_LINES = {"production": (1_000, 2_500), "test": (2_000, 4_000)}


def is_test_file(relative):
    name = relative.rsplit("/", 1)[-1]
    return (
        not f"/{relative}".count("/src/")
        or "/src/tests/" in f"/{relative}"
        or name == "tests.rs"
        or name.endswith(("_test.rs", "_tests.rs"))
    )


def code_lines(lines):
    count, skip_to_close, pending_test = 0, False, False
    for line in lines:
        stripped = line.strip()
        if skip_to_close:
            skip_to_close = line != "}"
            continue
        if stripped == "#[cfg(test)]":
            pending_test = True
            continue
        if pending_test and TEST_MOD.match(stripped) and stripped.endswith("{"):
            skip_to_close, pending_test = True, False
            continue
        if not stripped.startswith(("#[", "//")):
            pending_test = False
        if stripped and not stripped.startswith("//"):
            count += 1
    return count


def rust_files(root):
    result = []
    for base in RUST_ROOTS:
        for directory, names, files in os.walk(root / base):
            names[:] = [n for n in names if n not in {"target", "tmp"}]
            result.extend(Path(directory) / f for f in files if f.endswith(".rs"))
    return sorted(result)


def check_rust_layout(root, files):
    """Keep test code out of production items, explain skipped tests and bound file growth.

    Returns blocking errors and advisory notes.
    """
    errors, notes = [], []
    for path in files:
        relative = path.relative_to(root).as_posix()
        lines = path.read_text(encoding="utf-8").splitlines()
        production = not is_test_file(relative)
        note_at, limit = FILE_LINES["production" if production else "test"]
        size = code_lines(lines)
        if size > limit:
            errors.append(f"{relative}: {size} code lines exceeds {limit}; split by responsibility")
        elif size > note_at:
            notes.append(f"{relative}: {size} code lines; check it still holds one responsibility")
        for index, line in enumerate(lines):
            stripped = line.strip()
            if BARE_IGNORE.search(stripped):
                errors.append(f"{relative}:{index + 1}: #[ignore] needs a reason; timing belongs in benches/")
            if not production or stripped != "#[cfg(test)]":
                continue
            following = next(
                (l.strip() for l in lines[index + 1:] if l.strip() and not l.strip().startswith(("#[", "//"))),
                "",
            )
            if not TEST_MOD.match(following):
                errors.append(f"{relative}:{index + 1}: #[cfg(test)] may gate only a test module, not production items")
    return errors, notes


def markdown_files(root):
    result = []
    for directory, names, files in os.walk(root):
        names[:] = [n for n in names if n not in {".git", "target", "tmp", "__pycache__"}]
        result.extend(Path(directory) / f for f in files if f.endswith(".md"))
    return sorted(result)


def main():
    files = markdown_files(ROOT)
    errors, links = check_links(ROOT, files)
    errors += check_skills(ROOT)
    layout_errors, notes = check_rust_layout(ROOT, rust_files(ROOT))
    errors += layout_errors
    for note in notes:
        print(f"note: {note}", file=sys.stderr)
    ledger = json.loads((ROOT / "docs/design/maturity.json").read_text(encoding="utf-8"))
    errors += check_maturity(ledger)
    errors += check_hashes(ROOT, ledger)
    policy = tomllib.loads((ROOT / ".config/architecture.toml").read_text(encoding="utf-8"))
    # Explicit rustup avoids silently using an unrelated standalone Cargo installation.
    channel = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))["toolchain"]["channel"]
    output = subprocess.check_output(["rustup", "run", channel, "cargo", "metadata", "--no-deps", "--format-version", "1", "--locked", "--offline"], cwd=ROOT, encoding="utf-8")
    metadata = json.loads(output)
    errors += check_packages(ROOT, metadata, policy)
    errors += check_ios_runtime_closure(ROOT, channel)
    errors += check_unused_direct_dependencies(metadata, policy)
    errors += check_manifest_inventory(ROOT, metadata)
    if errors:
        print("\n".join(sorted(set(errors))), file=sys.stderr)
        return 1
    print(f"Repository checks passed: {len(files)} Markdown files, {links} local links, {len(ledger['modules'])} module records, crate policy and current design hashes.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

"""Count recurring code shapes and hold their first-parent historical low marks.

check --rule NAME --zero PATH... ignores allowances within the named paths.
tighten records current counts; it never grants an increase over Git history.
inventory emits the same counts without requiring an existing baseline.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib
import evidence_store

from shape_matchers import (CODE, RULES, dependency_sites, evidence_path_lines, limit_exempt, main_tier_lines, mask,
                            package_for, path_attributes, rust_sites, test_binary, test_edge_sites, uncalled_scripts,
                            unrun_benches, LONG_TESTS)

BASELINE = ".config/shape-baseline.json"
PATTERNS = ".agents/skills/thrum-engineering/references/patterns.md"
ROOT = Path(__file__).resolve().parents[1]


def git(root, *args):
    return subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)


def owns_history(root):
    result = git(root, "rev-parse", "--show-toplevel")
    return result.returncode == 0 and Path(result.stdout.strip()).resolve() == root.resolve()


def files_in(root):
    if owns_history(root):
        result = git(root, "ls-files", "-co", "--exclude-standard", "-z")
        if result.returncode:
            raise ValueError(f"cannot inventory Git files: {result.stderr.strip()}")
        return sorted({p for p in result.stdout.split("\0") if p and (root / p).is_file()})
    return sorted(p.relative_to(root).as_posix() for p in root.rglob("*")
                  if p.is_file() and not {".git", "target", "tmp", "__pycache__"} & set(p.relative_to(root).parts))


def pinned_names(root, paths):
    """Names pinned by executable checks, xtask and the baseline registry only."""
    sources = []
    for path in sorted((root / "docs/plans").glob("*/DONE.json")):
        for milestone in json.loads(path.read_text()).get("milestones", []):
            if milestone.get("status") == "done":
                continue
            for item in milestone.get("items", []):
                if item.get("check"):
                    sources.append((f"{path.relative_to(root)}:{item['id']}", item["check"]))
    for relative in paths:
        if relative.startswith("tools/xtask/") or relative == ".config/exact-baselines.json":
            sources.append((relative, (root / relative).read_text()))
    pins = {}
    for source, text in sources:
        # Match full tokens, not a short code occurring by chance in prose.
        for token in re.findall(r'[A-Za-z0-9_./-]+', text):
            if CODE.search(token):
                pins.setdefault(token, set()).add(source)
                for part in Path(token).parts:
                    if CODE.search(part):
                        pins.setdefault(part, set()).add(source)
    return pins


def inventory(root, honor_pins=False):
    paths = files_in(root)
    manifests = {Path(p).parent: tomllib.loads((root / p).read_text()) for p in paths
                 if Path(p).name == "Cargo.toml" and "package" in tomllib.loads((root / p).read_text())}
    registry_path = root / ".config/architecture.toml"
    registry = tomllib.loads(registry_path.read_text()) if registry_path.exists() else {}
    workspace_path = root / "Cargo.toml"
    workspace = tomllib.loads(workspace_path.read_text()).get("workspace", {}).get("dependencies", {}) if workspace_path.exists() else {}
    pins = pinned_names(root, paths)
    sites = {name: [] for name in RULES}
    evidence_paths = evidence_store.tracked(root) if owns_history(root) else paths
    sites["evidence-size"] = [(p, 1) for p in evidence_store.oversized(root, evidence_paths)]
    sites["script-caller"] = [(p, 1) for p in uncalled_scripts(root, paths)]
    sites["bench-runner"] = list(unrun_benches(root, paths, manifests))
    sites["limit-exempt"], refused = limit_exempt(root, paths)
    sites["main-tier"] = [(LONG_TESTS, line) for line, _ in main_tier_lines(root, paths)]
    sites["test-binary"], unplanned = test_binary(root, paths, manifests)
    refusals = [(path, line, "limit-exempt", why) for path, line, why in refused]
    refusals += [(path, line, "test-binary", why) for path, line, why in unplanned]
    for relative in paths:
        path = Path(relative)
        if not path.parts or path.parts[0] not in {"crates", "examples", "tools", "scripts"}:
            continue
        if path.suffix != ".rs":
            sites["evidence-path"] += [(relative, line) for line in evidence_path_lines(relative, (root / path).read_text(errors="ignore"))]
        package = package_for(path, manifests)
        if path.suffix == ".rs":
            text = (root / path).read_text()
            sites["evidence-path"] += [(relative, line) for line in evidence_path_lines(relative, text)]
            # Matchers resolve relative includes from the repository root.
            for rule, offset in rust_sites(root, relative, text, package, manifests.get(package), registry):
                sites[rule].append((relative, text.count("\n", 0, offset) + 1))
        if any(CODE.search(part) and (not honor_pins or part not in pins) for part in path.parts):
            sites["history-names"].append((relative, 1))
        if path.suffix in {".rs", ".py", ".mjs", ".js", ".sh", ".ps1", ".toml", ".html", ".wgsl", ".metal",
                           ".swift", ".c", ".h", ".cpp", ".mm", ".ts", ".tsx", ".jsx", ".kt"}:
            text = (root / path).read_text()
            content = mask(text) if path.suffix == ".rs" else text
            for match in re.finditer(r'["\x27]([A-Z][A-Z0-9_]*)["\x27]', content):
                if "_" in match.group(1) and CODE.search(match.group(1)):
                    sites["history-names"].append((relative, text.count("\n", 0, match.start()) + 1))
        if path.name == "Cargo.toml":
            manifest = manifests.get(path.parent, {})
            text = (root / path).read_text()
            for alias in test_edge_sites(manifest, workspace, registry):
                match = re.search(r'(?m)^\s*' + re.escape(alias) + r'\s*=', text)
                sites["test-crate-edges"].append((relative, text.count("\n", 0, match.start()) + 1 if match else 1))
            fields = [(key, manifest.get("package", {}).get(key, "")) for key in ("name", "description")]
            fields.append(("name", manifest.get("lib", {}).get("name", "")))
            for kind in ("bin", "test", "bench", "example"):
                fields.extend(("name", target.get("name", "")) for target in manifest.get(kind, []))
            for key, value in fields:
                if isinstance(value, str) and CODE.search(value) and (not honor_pins or value not in pins):
                    offset = text.find(value)
                    if offset < 0:
                        match = re.search(r'(?m)^\s*' + key + r'\s*=', text)
                        offset = match.start() if match else 0
                    sites["history-names"].append((relative, text.count("\n", 0, offset) + 1))
            if path.parts[0] == "crates":
                for name, dependency in dependency_sites(manifest, path.parent):
                    if isinstance(dependency, dict) and dependency.get("workspace"):
                        dependency = workspace.get(name, {})
                        base = Path(".")
                    else:
                        base = path.parent
                    if isinstance(dependency, dict) and "path" in dependency:
                        destination = (root / base / dependency["path"]).resolve()
                        if destination.is_relative_to(root / "examples"):
                            match = re.search(r'(?m)^\s*' + re.escape(name) + r'\s*=', text)
                            sites["package-reach"].append((relative, text.count("\n", 0, match.start()) + 1 if match else 1))
    counts = {rule: {"version": RULES[rule][0], "files": dict(sorted(Counter(p for p, _ in sites[rule]).items()))}
              for rule in RULES}
    # Refusals ride beside the sites: no count records them, and check() fails on any.
    sites["refused"] = refusals
    return {"schema": 1, "rules": counts}, sites, pins


def history_errors(root, baseline, sites=None):
    if not owns_history(root):
        return [], "shape_rules: no Git history; tree counts only"
    # Reject shallow history rather than silently lose a rule's accepted low mark.
    if git(root, "rev-parse", "--is-shallow-repository").stdout.strip() == "true":
        return ["shape_rules: fetch full history before checking the ratchet"], ""
    revisions = git(root, "log", "--first-parent", "--format=%H", "HEAD", "--", BASELINE)
    if revisions.returncode:
        return [], "shape_rules: no committed baseline yet; registering rules"
    lows = {}
    for revision in revisions.stdout.splitlines():
        shown = git(root, "show", f"{revision}:{BASELINE}")
        if shown.returncode:
            continue
        past = json.loads(shown.stdout)
        for rule, entry in past["rules"].items():
            key = (rule, entry["version"])
            total = sum(entry["files"].values())
            if key not in lows or total <= lows[key][0]:
                lows[key] = (total, entry["files"], revision)
    errors = []
    for rule, entry in baseline["rules"].items():
        low = lows.get((rule, entry["version"]))
        total = sum(entry["files"].values())
        if low and total > low[0]:
            errors.append(f"{rule}: total {total} exceeds historical low {low[0]} at {low[2]}")
            for path, count in entry["files"].items():
                if count > low[1].get(path, 0):
                    lines = [line for file, line in (sites or {}).get(rule, []) if file == path] or [1]
                    for line in lines:
                        errors.append(f"{path}:{line}: {rule}: {count} above historical file count {low[1].get(path, 0)}; {guidance(rule)}")
    return errors, ""


def guidance(rule):
    _, section, remedy = RULES[rule]
    return f"{remedy}; read {PATTERNS}#{section}"


def external_boundary(game, engine):
    """Apply package reach and test-edge rules to a detached external consumer.

    External path edges may target registered packages in engine/crates only.
    Includes and module paths still stay within their own game package.
    """
    registry = tomllib.loads((engine / ".config/architecture.toml").read_text())
    # An external consumer cannot be the engine's registered proving ground,
    # even if the game's package has the same name as that in-tree package.
    registry = dict(registry, boundaries={})
    manifests = {p.parent: tomllib.loads(p.read_text()) for p in game.rglob("Cargo.toml")}
    workspace_folder = game
    workspace_manifest = manifests.get(game, {})
    if "workspace" not in workspace_manifest and (game.parent / "Cargo.toml").is_file():
        workspace_folder = game.parent
        workspace_manifest = tomllib.loads((workspace_folder / "Cargo.toml").read_text())
    workspace = workspace_manifest.get("workspace", {}).get("dependencies", {})
    registered = {(engine / entry["manifest"]).resolve().parent
                  for entry in registry.get("packages", {}).values()}
    errors = []
    for folder, manifest in manifests.items():
        targets = [manifest.get("lib", {})]
        for kind in ("bin", "test", "bench", "example"):
            targets.extend(manifest.get(kind, []))
        build = manifest.get("package", {}).get("build")
        if isinstance(build, str):
            targets.append({"path": build})
        for target in targets:
            if "path" in target and not (folder / target["path"]).resolve().is_relative_to(game):
                errors.append(f"{folder.relative_to(game)}/Cargo.toml: package-reach: target source leaves game")
        for alias, dependency in dependency_sites(manifest, folder):
            base = folder
            if isinstance(dependency, dict) and dependency.get("workspace"):
                if alias not in workspace:
                    errors.append(f"{folder.relative_to(game)}/Cargo.toml: package-reach: unresolved workspace dependency {alias}")
                    continue
                dependency, base = workspace[alias], workspace_folder
            if isinstance(dependency, dict) and "path" in dependency:
                destination = (base / dependency["path"]).resolve()
                if destination not in registered or not destination.is_relative_to((engine / "crates").resolve()):
                    errors.append(f"{folder.relative_to(game)}/Cargo.toml: package-reach: {alias} must name an engine crates package")
        for alias in test_edge_sites(manifest, workspace, registry):
            errors.append(f"{folder.relative_to(game)}/Cargo.toml: test-crate-edges: {alias}: {guidance('test-crate-edges')}")
    packages = {folder.relative_to(game): manifest for folder, manifest in manifests.items() if "package" in manifest}
    for path in game.rglob("*.rs"):
        relative = path.relative_to(game)
        package = package_for(relative, packages)
        for rule, offset in rust_sites(game, relative.as_posix(), path.read_text(), package, packages.get(package), registry):
            if rule == "package-reach":
                errors.append(f"{relative}:{path.read_text().count(chr(10), 0, offset) + 1}: {rule}: {guidance(rule)}")
    return errors


def save_baseline(path, value):
    # One file entry per line, sorted by path, for independent builders' edits.
    lines = ['{', '  "schema": 1,', '  "rules": {']
    for index, (rule, entry) in enumerate(sorted(value["rules"].items())):
        lines.extend([f'    "{rule}": {{', f'      "version": {entry["version"]},', '      "files": {'])
        for number, (name, count) in enumerate(sorted(entry["files"].items())):
            lines.append(f'        {json.dumps(name)}: {count}' + ("," if number + 1 < len(entry["files"]) else ""))
        lines.extend(['      }', '    }' + ("," if index + 1 < len(value["rules"]) else "")])
    lines.extend(['  }', '}'])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n")


def explicit_rust_sources(root, paths):
    """Follow explicit Rust module paths that leave the supplied check roots."""
    indexed = {root / relative for relative in files_in(root) if relative.endswith(".rs")}
    sources = {source for source in indexed if any(source.is_relative_to(path) for path in paths)}
    pending = list(sources)
    while pending:
        source = pending.pop()
        text = source.read_text()
        for _, reference in path_attributes(mask(text), mask(text, strings=True)):
            target = (source.parent / reference).resolve()
            if target in indexed and target not in sources:
                sources.add(target)
                pending.append(target)
    return sources


def check(root, rule=None, zero=None, tighten=False):
    current, sites, pins = inventory(root, honor_pins=zero is not None)
    if zero is not None:
        errors = []
        for name, sources in sorted(pins.items()):
            if rule == "history-names":
                print(f"pinned {name}: {', '.join(sorted(sources))}")
        paths = []
        for value in zero:
            path = (root / value).resolve()
            if not path.exists() or not path.is_relative_to(root):
                errors.append(f"{value}:1: {rule}: --zero path missing or outside repository")
                continue
            paths.append(path)
        sources = explicit_rust_sources(root, paths) if rule == "device-request" else set()
        for relative, line in sites[rule]:
            candidate = root / relative
            if candidate in sources or any(candidate.is_relative_to(path) for path in paths):
                errors.append(f"{relative}:{line}: {rule}: {guidance(rule)}")
        return errors
    # What a rule refuses outright, whatever the counts: a site it cannot count, or one outside its list.
    refused = [f"{relative}:{line}: {name}: {why}; {guidance(name)}"
               for relative, line, name, why in sites["refused"] if rule in (None, name)]
    if refused:
        return sorted(set(refused))
    path = root / BASELINE
    if not path.exists() and not tighten:
        return [f"{BASELINE}: missing; initialise with shape_rules.py tighten"]
    errors, note = history_errors(root, current, sites)
    if note:
        print(note)
    if tighten:
        if not errors:
            save_baseline(path, current)
        return errors
    expected = json.loads(path.read_text())
    if expected.get("schema") != 1 or expected.get("rules", {}).keys() != RULES.keys():
        return errors + [f"{BASELINE}: rule registry/schema mismatch; run shape_rules.py tighten"]
    for name in ([rule] if rule else RULES):
        entry = expected["rules"][name]
        actual = current["rules"][name]
        if entry != actual:
            errors.append(f"{BASELINE}: {name}: counts/version differ; remove new violations or run shape_rules.py tighten after removing old ones")
            for relative, line in sites[name]:
                if actual["files"][relative] != entry["files"].get(relative, 0):
                    errors.append(f"{relative}:{line}: {name}: {guidance(name)}")
    # Check the reviewed baseline too, so stale or inflated allowances are refused.
    extra, _ = history_errors(root, expected, sites)
    return sorted(set(errors + extra))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["check", "tighten", "inventory"])
    parser.add_argument("--rule", choices=sorted(RULES))
    parser.add_argument("--zero", nargs="+")
    args = parser.parse_args()
    if args.zero is not None and (not args.rule or args.command != "check"):
        parser.error("--zero requires check --rule NAME")
    # Match include paths relative to ROOT, independent of the invoking directory.
    import os
    os.chdir(ROOT)
    if args.command == "inventory":
        print(json.dumps(inventory(ROOT)[0], indent=2))
        return 0
    errors = check(ROOT, args.rule, args.zero, args.command == "tighten")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"shape_rules: {args.command} passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())

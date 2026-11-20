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
import shape_rules
import evidence_store
import sim2d_profile

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
    listed = {entry["path"]: entry for entry in evidence_store.entries(root)}
    tracked = evidence_store.tracked(root) if shape_rules.owns_history(root) else None
    directories = {parent.as_posix() for path in (tracked or []) for parent in Path(path).parents}
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
            if url.path and Path(os.path.normpath(path.parent / unquote(url.path))).is_relative_to(root / ".orchestrate"):
                continue  # orchestrate's working folder: evidence kept on this computer, outside git
            dest = (path.parent / unquote(url.path)).resolve() if url.path else path
            count += 1
            label = f"{path.relative_to(root)}: {value}"
            relative = dest.relative_to(root).as_posix() if dest.is_relative_to(root) else None
            known = relative in listed or (relative in tracked or relative in directories if tracked is not None else dest.exists())
            if relative is None or not known:
                errors.append(f"missing/outside local link: {label}")
            elif url.fragment and dest.suffix == ".md" and unquote(url.fragment) not in anchors(
                    dest.read_text(encoding="utf-8") if relative not in listed else
                    evidence_store.content(root, listed[relative]).decode()):
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


LONG_TESTS = ".config/long-tests.txt"
REASONED_IGNORE = re.compile(r'#\[ignore\s*=\s*"((?:[^"\\]|\\.)*)"\s*\]')
MOD_DECLARATION = re.compile(r'((?:#\[path\s*=\s*"([^"]+)"\]\s*)?)(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;')


def parse_main_tier_list(text):
    """Parse the main-tier list into its long-test binaries, its feature entries and its errors.

    A long-test line is `<package> <test binary> release|debug`. A feature line is
    `<package> features=<feature>[,<feature>...] [<test binary>...]`: with no binary it covers every test
    binary of the package. A not-run line, `<package> not-run <test binary> features=<feature>[,...]
    owner=<MODULE> # <its failure>`, records a binary behind a feature that failed when it was listed: it
    runs in no tier until its owner fixes it, and the line accounts for it. Returns
    ({(package, binary)}, [(package, {feature}, {binary} or None, line)], errors); a not-run line joins the
    feature entries as covering its one binary, so the coverage guard accepts it.
    """
    listed, features, errors = set(), [], []
    for number, line in enumerate(text.splitlines(), 1):
        fields = line.split("#", 1)[0].split()
        if not fields:
            continue
        if len(fields) >= 2 and fields[1] == "not-run":
            parts = dict(field.split("=", 1) for field in fields[3:] if "=" in field)
            failure = line.split("#", 1)[1].strip() if "#" in line else ""
            names = frozenset(name for name in parts.get("features", "").split(",") if name)
            if len(fields) != 5 or not names or not parts.get("owner") or not failure:
                errors.append(f"{LONG_TESTS}:{number}: expected `<package> not-run <test binary> "
                              "features=<feature>[,...] owner=<MODULE> # <its failure>`")
                continue
            features.append((fields[0], names, frozenset([fields[2]]), number))
            continue
        if len(fields) >= 2 and fields[1].startswith("features="):
            names = frozenset(name for name in fields[1].removeprefix("features=").split(",") if name)
            if not names:
                errors.append(f"{LONG_TESTS}:{number}: `features=` names no feature")
                continue
            features.append((fields[0], names, frozenset(fields[2:]) or None, number))
            continue
        if len(fields) != 3 or fields[2] not in {"release", "debug"}:
            errors.append(f"{LONG_TESTS}:{number}: expected `<package> <test binary> release|debug` "
                          "or `<package> features=<feature>[,...] [<test binary>...]`")
            continue
        listed.add((fields[0], fields[1]))
    return listed, features, errors


def long_test_list(text):
    """Parse the main-tier list's long-test binaries into ({(package, binary)}, errors)."""
    listed, _, errors = parse_main_tier_list(text)
    return listed, errors


def module_files(root_file):
    """The root file of a test binary and every file its `mod name;` declarations reach."""
    seen, pending = [], [root_file]
    while pending:
        path = pending.pop()
        if path in seen or not path.is_file():
            continue
        seen.append(path)
        text = path.read_text(encoding="utf-8")
        own = path.parent if path == root_file or path.name == "mod.rs" else path.parent / path.stem
        for match in MOD_DECLARATION.finditer(text):
            if match.group(2):
                pending.append((path.parent / match.group(2)).resolve())
            else:
                pending += [own / f"{match.group(3)}.rs", own / match.group(3) / "mod.rs"]
    return seen


def check_long_tests(root, metadata, list_text):
    """A long test runs in a binary the main-tier list names, and a listed binary holds only long tests.

    `./dev long-tests` runs every ignored test of each listed binary, so a long test outside the list
    runs in no tier, and another ignored test inside it would join the main tier unnoticed.
    """
    listed, errors = long_test_list(list_text)
    members = set(metadata["workspace_members"])
    binaries = {}  # file -> {(package, binary)}
    targets = set()
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        for target in package["targets"]:
            if "test" not in target["kind"]:
                continue
            targets.add((package["name"], target["name"]))
            for path in module_files(Path(target["src_path"]).resolve()):
                binaries.setdefault(path, set()).add((package["name"], target["name"]))
    for package, binary in sorted(listed - targets):
        errors.append(f"{LONG_TESTS}: {package} has no test binary {binary}")
    for path in rust_files(root):
        text = path.read_text(encoding="utf-8")
        if "#[ignore" not in text:
            continue
        relative = path.relative_to(root).as_posix()
        holders = binaries.get(path.resolve(), set())
        for match in REASONED_IGNORE.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            name = re.search(r"fn\s+(\w+)", text[match.end():])
            name = name.group(1) if name else "?"
            if match.group(1).startswith("long:"):
                if not holders & listed:
                    errors.append(
                        f"{relative}:{line}: long test {name} is in no test binary {LONG_TESTS} names, so it runs in "
                        "no tier; list its binary there or drop `long:` from its reason")
            elif holders & listed:
                found = ", ".join(" ".join(pair) for pair in sorted(holders & listed))
                errors.append(
                    f"{relative}:{line}: ignored test {name} is in {found}, which {LONG_TESTS} names, so "
                    "`./dev long-tests` would run it; only a test whose reason starts `long:` may sit there")
    return errors


CFG_START = re.compile(r"\bcfg(?:_attr)?!?\s*\(")
CFG_FEATURE = re.compile(r'\bfeature\s*=\s*"([^"]+)"')


def blank_literals(text):
    """The text with comments and string and character literals turned to spaces, newlines kept, so braces
    and attributes can be matched by position in it while names are read from the original."""
    out, i, n = list(text), 0, len(text)

    def blank(start, end):
        for k in range(start, min(end, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = text[i]
        if text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif (raw := re.match(r'b?r(#*)"', text[i:i + 300])) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            close = '"' + raw.group(1)
            end = text.find(close, i + raw.end())
            end = n if end < 0 else end + len(close)
            blank(i + raw.end(), end - len(close))
            i = end
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            blank(i + 1, j)
            i = j + 1
        elif c == "'" and (char := re.match(r"'(?:\\.[^']*|[^'\\])'", text[i:i + 12])):
            blank(i + 1, i + char.end() - 1)
            i += char.end()
        else:
            i += 1
    return "".join(out)


CFG_START = re.compile(r"\bcfg(?:_attr)?!?\s*\(")
CFG_FEATURE = re.compile(r'\bfeature\s*=\s*"([^"]+)"')
NOT_START = re.compile(r"\bnot\s*\(")


def balanced(text, start):
    """The index just past the `)` that closes the `(` before `start`."""
    depth, end = 1, start
    while end < len(text) and depth:
        depth += {"(": 1, ")": -1}.get(text[end], 0)
        end += 1
    return end


def cfg_spans(text):
    """Each `cfg`, `cfg_attr` or `cfg!` in the text as (start, end of its arguments, start of arguments)."""
    return [(m.start(), balanced(text, m.end()), m.end()) for m in CFG_START.finditer(text)]


def gating_features(arguments):
    """The features a `cfg`'s arguments need on. A feature under `not(...)` only turns code off, and code
    that runs with a feature off already runs in the merge tier, so it is left out."""
    kept, last = [], 0
    for m in NOT_START.finditer(arguments):
        if m.start() < last:
            continue
        kept.append(arguments[last:m.start()])
        last = balanced(arguments, m.end())
    kept.append(arguments[last:])
    return CFG_FEATURE.findall("".join(kept))


def cfg_features(text, spans=None):
    """Every Cargo feature a `cfg`, `cfg_attr` or `cfg!` in the text gates on, with its line. With `spans`,
    only cfgs that start inside one of those (start, end) ranges."""
    found = []
    for start, end, arguments in cfg_spans(text):
        if spans is not None and not any(a <= start < b for a, b in spans):
            continue
        line = text.count("\n", 0, start) + 1
        found += [(line, name) for name in gating_features(text[arguments:end - 1])]
    return found


ATTRIBUTE = re.compile(r"#!?\[")
TEST_TOKEN = re.compile(r"\btest\b")
TEST_MOD = re.compile(r"\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*([;{])")


def test_code(text):
    """Where test code sits in a source file that is not itself a test file, and the modules it declares
    test-only. Returns ([(start, end)], [(module name, `#[path]` or None)]).

    Test code is a `#[cfg(...)]`-gated module whose cfg names `test` (its body, or its file), any item whose
    attributes hold `#[test]` or a cfg naming `test` (its attributes), and those attributes' cfgs.
    """
    plain = blank_literals(text)
    spans, files = [], []
    position = 0
    while (m := ATTRIBUTE.search(plain, position)):
        # One run of attributes, then the item they belong to.
        run_start, end = m.start(), m.start()
        attributes = []
        while (a := ATTRIBUTE.match(plain, end)) or (attributes and plain[end:end + 1].isspace()):
            if a:
                close = end + plain[end:].find("[") + 1
                depth, j = 1, close
                while j < len(plain) and depth:
                    depth += {"[": 1, "]": -1}.get(plain[j], 0)
                    j += 1
                attributes.append(plain[a.end():j - 1])
                end = j
            else:
                end += 1
        position = max(end, m.end())
        testing = any(attr.strip() == "test" or attr.strip().endswith("::test")
                      or (attr.lstrip().startswith("cfg") and TEST_TOKEN.search(attr)) for attr in attributes)
        if not testing:
            continue
        spans.append((run_start, end))
        module = TEST_MOD.match(plain, end)
        if module and any(attr.lstrip().startswith("cfg") and TEST_TOKEN.search(attr) for attr in attributes):
            if module.group(2) == "{":
                body = balanced(plain.replace("{", "(").replace("}", ")"), module.end())
                spans.append((module.end(), body))
                position = body
            else:
                path = next((re.search(r'"([^"]+)"', text[run_start:end]) for attr in attributes
                             if attr.lstrip().startswith("path")), None)
                files.append((module.group(1), path.group(1) if path else None))
    return spans, files


def workspace_features(packages):
    """The features `cargo test --workspace` turns on for each package on this Mac.

    A package's default features, and those another member turns on through a dependency that is not
    optional and not only for wasm32, with what each of them turns on in turn. A test those gate runs in
    the merge tier already.
    """
    names = {package["name"]: package for package in packages}
    enabled = {name: set(package["features"].get("default", [])) for name, package in names.items()}
    for package in packages:
        for dependency in package["dependencies"]:
            target = dependency.get("target") or ""
            if dependency["name"] not in names or dependency["optional"] or (
                    "wasm32" in target and not target.startswith("cfg(not(")):
                continue
            enabled[dependency["name"]].update(dependency["features"])
    for name, features in enabled.items():
        pending = list(features)
        while pending:
            for implied in names[name]["features"].get(pending.pop(), []):
                if implied not in features and implied in names[name]["features"]:
                    features.add(implied)
                    pending.append(implied)
    return enabled


LIBRARY_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"}


def check_feature_tests(root, metadata, list_text):
    """No Cargo feature takes a test out of every tier unnoticed, however the test is gated.

    A test target's `required-features`, and every feature a `cfg` gates test code on, must be covered by a
    feature line of the main-tier list for that file's package: whole, or by the test binary the code builds
    into, where `lib` names the library's own tests. Test code is every file a test target reaches, every
    file of a test-only module, and in any other source file the test modules, `#[test]` items and cfgs that
    name `test`. A file several binaries share, or test code outside the library's tree and the test
    targets, is covered only by a whole entry. `./dev check` lints the listed packages with those features
    and `./dev long-tests` runs them.
    """
    _, entries, _ = parse_main_tier_list(list_text)
    members = set(metadata["workspace_members"])
    packages = [p for p in metadata["packages"] if p["id"] in members]
    names = {p["name"]: p for p in packages}
    errors = []
    for package, features, binaries, number in entries:
        if package not in names:
            errors.append(f"{LONG_TESTS}:{number}: no workspace package {package}")
            continue
        targets = {t["name"] for t in names[package]["targets"] if "test" in t["kind"]}
        if any(set(t["kind"]) & LIBRARY_KINDS for t in names[package]["targets"]):
            targets.add("lib")
        for binary in sorted((binaries or set()) - targets):
            errors.append(f"{LONG_TESTS}:{number}: {package} has no test binary {binary}")
        known = set(names[package].get("features", {}))
        for feature in sorted(f for f in features if "/" not in f and f not in known):
            errors.append(f"{LONG_TESTS}:{number}: {package} has no feature {feature}")

    always = workspace_features(packages)

    def covered(package, feature, holders):
        """One line turns on `feature` (a name or a set that must all be on together) for these holders."""
        wanted = ({feature} if isinstance(feature, str) else set(feature)) - always.get(package, set())
        if not wanted:
            return True
        for listed, features, binaries, _ in entries:
            if listed != package or not wanted <= features:
                continue
            if binaries is None or (len(holders) == 1 and holders <= binaries):
                return True
        return False

    def missing(where, package, feature, how):
        return (f"{where}: {how} feature {feature}, and no line of {LONG_TESTS} covers {package} with it, so "
                f"its tests run in no tier; add `{package} features={feature}` there, whole or with the test "
                "binary (`lib` for the library's own tests)")

    binaries, owners, library = {}, [], set()  # file -> {binary}; (package directory, name); library files
    for package in packages:
        directory = Path(package["manifest_path"]).resolve().parent
        owners.append((directory, package["name"]))
        for target in package["targets"]:
            source = Path(target["src_path"]).resolve()
            if set(target["kind"]) & LIBRARY_KINDS:
                library.update(module_files(source))
            if "test" not in target["kind"]:
                continue
            required = target.get("required-features", [])
            if required and not covered(package["name"], required, {target["name"]}):
                # Cargo builds the target only when every one is on, so one line must give them all.
                where = Path(package["manifest_path"]).resolve().relative_to(root).as_posix()
                errors.append(missing(f"{where} [[test]] {target['name']}", package["name"], ",".join(required),
                                      "requires"))
            for path in module_files(source):
                binaries.setdefault(path, set()).add(target["name"])
    owners.sort(key=lambda owner: len(owner[0].parts), reverse=True)

    # Source files: test-only module files, then the test code inside every other file.
    sources = {path.resolve(): path for path in rust_files(root)}
    test_files, scanned = set(), {}
    for resolved, path in sources.items():
        if resolved in binaries:
            continue
        relative = path.relative_to(root).as_posix()
        if is_test_file(relative) or "/tests/" in f"/{relative}":
            test_files.add(resolved)
            continue
        text = path.read_text(encoding="utf-8")
        if "test" not in text:
            continue
        spans, modules = test_code(text)
        scanned[resolved] = (text, spans)
        own = resolved.parent if resolved.name in {"lib.rs", "main.rs", "mod.rs"} else resolved.parent / resolved.stem
        for name, explicit in modules:
            roots = [(resolved.parent / explicit).resolve()] if explicit else [own / f"{name}.rs", own / name / "mod.rs"]
            for module_root in roots:
                if module_root.is_file():
                    test_files.update(module_files(module_root))
    for resolved, path in sorted(sources.items(), key=lambda item: item[1]):
        if resolved in binaries or resolved in test_files:
            text, spans = path.read_text(encoding="utf-8"), None
        elif resolved in scanned and scanned[resolved][1]:
            text, spans = scanned[resolved]
        else:
            continue
        if "feature" not in text:
            continue
        package = next((name for directory, name in owners if resolved.is_relative_to(directory)), None)
        if package is None:
            continue
        holders = binaries.get(resolved) or ({"lib"} if resolved in library else set())
        reported = set()
        relative = path.relative_to(root).as_posix()
        for line, feature in cfg_features(text, spans):
            if feature not in reported and not covered(package, feature, holders):
                reported.add(feature)
                errors.append(missing(f"{relative}:{line}", package, feature, "a `cfg` gates a test on"))
    return errors


def markdown_files(root):
    result = []
    for directory, names, files in os.walk(root):
        # A nested checkout owns its own checks, including in-progress docs.
        # Worktrees use a .git file; ordinary clones use a directory.
        names[:] = [n for n in names
                    if n not in {".git", "target", "tmp", "__pycache__"}
                    and not (Path(directory) / n / ".git").exists()]
        result.extend(Path(directory) / f for f in files if f.endswith(".md"))
    return sorted(result)


def main():
    files = markdown_files(ROOT)
    errors, links = check_links(ROOT, files)
    errors += shape_rules.check(ROOT)
    errors += check_skills(ROOT)
    layout_errors, notes = check_rust_layout(ROOT, rust_files(ROOT))
    errors += layout_errors
    for note in notes:
        print(f"note: {note}", file=sys.stderr)
    ledger = json.loads((ROOT / "docs/design/maturity.json").read_text(encoding="utf-8"))
    errors += check_maturity(ledger)
    errors += check_hashes(ROOT, ledger)
    policy = tomllib.loads((ROOT / ".config/architecture.toml").read_text(encoding="utf-8"))
    rule_packages = {name for name, entry in policy["packages"].items() if entry.get("rule_package")}
    for name in rule_packages:
        manifest = ROOT / policy["packages"][name]["manifest"]
        errors += [f"{name}: {error}" for error in sim2d_profile.configuration_errors(
            manifest.parent, tomllib.loads(manifest.read_text()), {"thrum-core", "thrum-sim2d", *rule_packages})]
    # Explicit rustup avoids silently using an unrelated standalone Cargo installation.
    channel = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))["toolchain"]["channel"]
    output = subprocess.check_output(["rustup", "run", channel, "cargo", "metadata", "--no-deps", "--format-version", "1", "--locked", "--offline"], cwd=ROOT, encoding="utf-8")
    metadata = json.loads(output)
    errors += check_packages(ROOT, metadata, policy)
    errors += check_ios_runtime_closure(ROOT, channel)
    errors += check_unused_direct_dependencies(metadata, policy)
    errors += check_manifest_inventory(ROOT, metadata)
    main_tier = (ROOT / LONG_TESTS).read_text(encoding="utf-8")
    errors += check_long_tests(ROOT, metadata, main_tier)
    errors += check_feature_tests(ROOT, metadata, main_tier)
    if errors:
        print("\n".join(sorted(set(errors))), file=sys.stderr)
        return 1
    print(f"Repository checks passed: {len(files)} Markdown files, {links} local links, {len(ledger['modules'])} module records, crate policy and current design hashes.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

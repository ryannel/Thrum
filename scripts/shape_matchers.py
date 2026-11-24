"""Text rules, not a Rust type checker. Comments and literals retain source offsets.

Rules recognise spelled-out APIs (including whitespace and multiline signatures),
not aliases or macro expansion. Keep this envelope explicit when adding a rule.
"""
import json
from pathlib import Path
import re
import tomllib


RULES = {
    "test-crate-edges": (1, "tests", "use support only in dev-dependencies and fixtures normally only in the proving ground"),
    "evidence-size": (1, "evidence", "the merge refuses any new or changed file under the tracked evidence folder: write run records to the working folder, .orchestrate/evidence/<MODULE>/, keep bulk captures in tmp/<task>/, or keep the bytes in the local store with python3 scripts/evidence_store.py put SOURCE --path <logical path> and commit only .config/evidence-manifest.json; read docs/ASSET-LIFECYCLE.md (local store only, no remote backup)"),
    "string-errors": (5, "errors", "return a boundary error enum implementing Display and Error"),
    "package-reach": (1, "crossing-crate-boundaries", "use a public package API or registered shared fixture package"),
    "path-attribute": (1, "tests", "use the normal module tree (foo.rs and foo/)"),
    "library-env": (2, "configuration", "parse settings at the process edge and pass typed values to the library"),
    "device-request": (1, "device-setup", "use the shared capabilities device recipe and test-support gate"),
    "whole-file-gpu-gate": (1, "tests", "gate only the individual tests that need a GPU"),
    "history-names": (3, "names", "name the file, variable or Cargo target for what it does"),
    "evidence-path": (1, "evidence", "write run records to the working folder, .orchestrate/evidence/<MODULE>/, and read test data from tests/fixtures/<module>/ or beside the crate's tests; source never names the tracked evidence folder"),
    "script-caller": (1, "scripts-and-benches", "name the script from what runs it: ./dev or tools/xtask, a CI workflow, the done-when item whose check runs it, or a tracked document outside the plans folder; or delete it in the landing that removed its last caller"),
    "bench-runner": (1, "scripts-and-benches", "name the bench from what runs it, such as `cargo bench --bench NAME` in a kept script, a ./dev command or a CI workflow; or delete the [[bench]] target in the landing that removed its last runner"),
    "test-readback": (1, "tests", "read back with thrum_test_support::readback::{buffer, texture}, or append a Readback to the encoder under test and wait on its submission"),
    "test-deadline": (1, "tests", "wait with thrum_test_support::wait::until, which fails the test at its deadline"),
    "shader-text-edit": (1, "shader-source", "make the variant a separate entry point or a generated constant prepended to the shader, not a .replace( of WGSL text"),
    "limit-exempt": (1, "the-smallest-world", "make the test fast, on the smallest world that shows its behaviour; no landing adds a test to the overrides of .config/nextest.toml that exempt it from the limit on one test, and an override that changes the limit names its tests one by one, as (binary_id(=<package>::<binary>) & test(=<test>))"),
    # Raise the version only in a landing that a done-when item asks for: a new version starts its count again.
    # Version 2: GATE-TIME M3.1 and M3.4 mark the water scene's moored foam run, moored wake tests and float off track.
    "main-tier": (4, "a-main-tier-test", "make the test fast; an entry in .config/long-tests.txt or a test marked #[ignore = \"long: ...\"] needs a done-when item that asks for it, and that item's landing raises this rule's version in scripts/shape_matchers.py; a builder with no such item writes the proposal, with the fault it would catch, in its module's LOG.md for a planner"),
    # Version 2: a check names a target only in the packages its command selects.
    "test-binary": (2, "an-existing-test-binary", "add the test to a test file that exists, or to the unit tests of the module it tests; a new test target needs a done-when item whose check runs it with --test, and .config/unplanned-test-binaries.txt only shrinks"),
}
# Built in two parts so this file does not name the folder it matches.
EVIDENCE_FOLDER = "docs/" + "evidence/"
EVIDENCE_PATH = re.compile(r"docs[/\\]" + "evidence" + r"(?![\w-])")
# The evidence store and its suite restore history, so they name the folder by design.
EVIDENCE_STORE = {"scripts/evidence_store.py", "scripts/test_evidence_store.py"}
SOURCE_SUFFIXES = {".rs", ".py", ".mjs", ".js", ".sh", ".ps1", ".swift", ".c", ".h", ".cpp", ".mm", ".ts",
                   ".tsx", ".jsx", ".kt", ".wgsl", ".metal", ".html", ".in"}
TEXT_EDIT = re.compile(r'\.\s*replacen?\s*\(')
LONG_IGNORE = re.compile(r'#\[\s*ignore\s*=\s*"long:')
CODE = re.compile(r"(?i)(?:^|[^a-z0-9])(?:g[0-9]+[a-z]?|t2|a1|m[0-9]+[a-z]?)(?=$|[^a-z0-9])")
# Raw strings, quoted strings, chars (not lifetimes), nested block comments and
# line comments are scanned rather than stripped with a single greedy regex.
LEXEME = re.compile(r'r(#+)?"|"|\'(?:\\.|[^\'\\\n])\'|//|/\*')


def mask(text, strings=False):
    chars = list(text)
    at = 0
    while match := LEXEME.search(text, at):
        start = match.start()
        token = match.group()
        end = match.end()
        comment = token in ("//", "/*")
        if token == "//":
            end = text.find("\n", end)
            end = len(text) if end < 0 else end
        elif token == "/*":
            depth = 1
            while depth and end < len(text):
                nested = re.search(r'/\*|\*/', text[end:])
                if not nested:
                    end = len(text)
                    break
                depth += 1 if nested.group() == "/*" else -1
                end += nested.end()
        elif token.startswith('r'):
            close = '"' + (match.group(1) or "")
            found = text.find(close, end)
            end = len(text) if found < 0 else found + len(close)
        elif token == '"':
            while end < len(text):
                if text[end] == "\\":
                    end += 2
                elif text[end] == '"':
                    end += 1
                    break
                else:
                    end += 1
        if comment or strings:
            chars[start:end] = ["\n" if c == "\n" else " " for c in text[start:end]]
        at = end
    return "".join(chars)


def test_file(relative):
    path = Path(relative)
    return (bool({"tests", "benches"} & set(path.parts))
            or path.stem in {"test", "tests"} or path.stem.endswith("_tests"))


def without_test_modules(code):
    chars = list(code)
    for match in re.finditer(r'#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?mod\s+\w+\s*\{', code):
        end, depth = match.end(), 1
        while end < len(code) and depth:
            depth += (code[end] == "{") - (code[end] == "}")
            end += 1
        chars[match.start():end] = ["\n" if c == "\n" else " " for c in code[match.start():end]]
    return "".join(chars)


def package_for(path, packages):
    return next((p for p in sorted(packages, key=lambda p: len(p.parts), reverse=True)
                 if path.is_relative_to(p)), None)


def device_owner(relative, registry):
    # HOST registers the capabilities module, TEST-SUPPORT its GPU gate. This
    # does not grant the rest of either engine crate permission to request devices.
    path = Path(relative)
    if relative == "crates/thrum-render/src/capabilities.rs" or path.is_relative_to("crates/thrum-render/src/capabilities"):
        return True
    return any(entry.get("test_role") == "support"
               and path.is_relative_to(Path(entry["manifest"]).parent / "src")
               for entry in registry.get("packages", {}).values())


def support_package(relative, registry):
    """True inside a registered test-support package: the kit holds the shapes its rules refuse elsewhere."""
    return any(entry.get("test_role") == "support"
               and Path(relative).is_relative_to(Path(entry["manifest"]).parent)
               for entry in registry.get("packages", {}).values())


MAP_CALL = re.compile(r'\.\s*(?:map_async|map_buffer_on_submit)\s*\(')
MAP_READ = re.compile(r'\bMapMode\s*::\s*Read\b')
DEADLINE_ASSERT = re.compile(r'\bassert\s*!\s*\(\s*(?:(?:::\s*)?std\s*::\s*time\s*::\s*)?Instant\s*::\s*now\s*\(\s*\)\s*<')


# Test readbacks that do a different job from the kit's, by file and enclosing function, each with its reason.
# Add one only for a site that is not a readback the kit could do; see docs/plans/TEST-SUPPORT/DESIGN.md#the-kit.
TEST_READBACK_EXEMPT = {
    ("examples/mirror/tests/pool_gpu/retire.rs", "copy_buffer"):
        "the wasm view fixture's readback; it yields to a browser timer between polls, which a blocking wait cannot",
}
FUNCTION = re.compile(r'\bfn\s+(\w+)')


def enclosing_function(code, offset):
    """Name of the innermost function whose body holds `offset`, in masked code."""
    name = None
    for function in FUNCTION.finditer(code, 0, offset):
        body = code.find("{", function.end())
        if body < 0 or body > offset:
            continue
        end, depth = body + 1, 1
        while end < len(code) and depth:
            depth += (code[end] == "{") - (code[end] == "}")
            end += 1
        if end > offset:
            name = function.group(1)
    return name


def readback_sites(code, relative=""):
    """Offsets of a buffer mapped for reading: a map call whose arguments name `MapMode::Read`, outside the
    sites `TEST_READBACK_EXEMPT` names."""
    for match in MAP_CALL.finditer(code):
        if (relative, enclosing_function(code, match.start())) in TEST_READBACK_EXEMPT:
            continue
        end, depth = match.end(), 1
        while end < len(code) and depth:
            depth += (code[end] == "(") - (code[end] == ")")
            end += 1
        if MAP_READ.search(code, match.end(), end):
            yield match.start()


def public_function_headers(code):
    modifiers = r'(?:(?:async|unsafe|const|extern)\s+)*'
    function = r'\b' + modifiers + r'fn\s+\w+'
    yield from re.finditer(r'\bpub\s+' + modifiers + r'fn\s+\w+', code)
    # Trait methods inherit the visibility of their public trait. Scan only
    # top-level headers so local helpers in a default body stay private.
    for trait in re.finditer(r'\bpub\s+(?:(?:unsafe|auto)\s+)*trait\s+\w+[^{};]*\{', code):
        depth = 1
        for token in re.compile(r'[{}]|' + function).finditer(code, trait.end()):
            if token[0] == "{":
                depth += 1
            elif token[0] == "}":
                depth -= 1
                if depth == 0:
                    break
            elif depth == 1:
                yield token


def public_returns(code):
    """Find the return arrow outside a public function's parameters/generics.

    Semicolons in arrays and braces in const generics are part of the type,
    while a body or declaration without a return arrow ends the search.
    """
    for function in public_function_headers(code):
        angle = paren = bracket = brace = 0
        index = function.end()
        while index < len(code):
            char = code[index]
            top = angle == paren == bracket == brace == 0
            if top and char == "w" and re.match(r'where\b', code[index:]):
                break
            if code[index:index + 2] == "->":
                if top:
                    yield function.start(), code[index + 2:].lstrip()
                    break
                index += 2
                continue
            if top and char in "{;":
                break
            if not brace:
                angle += (char == "<") - (char == ">")
            paren += (char == "(") - (char == ")")
            bracket += (char == "[") - (char == "]")
            brace += (char == "{") - (char == "}")
            index += 1


def path_attributes(comments, code):
    """Yield real Rust path attributes and their literal targets."""
    literal = r'(?:r(\#*)"(.*?)"\1|"((?:\\.|[^"\\])*)")'
    for match in re.finditer(r'#\s*\[\s*path\s*=\s*' + literal + r'\s*\]', comments, re.S):
        if code[match.start()] == "#":
            yield match.start(), match.group(2) if match.group(2) is not None else match.group(3)


TEST_DECLARATION = re.compile(r'#\s*\[\s*(?:\w+\s*::\s*)*test\s*\]|\bmacos_gpu_test\s*!')
PLATFORM = re.compile(r'\b(?:target_os\s*=|target_family\s*=|target_vendor\s*=|(?:unix|windows)\b)')
# A platform shell's own tests may share its condition when the comment directly
# above the declaration says why, e.g. `// platform-only tests: drive the AppKit shell`.
PLATFORM_ONLY = re.compile(r'//[ \t]*platform-only tests:[ \t]*\S[^\n]*\n\s*$')


def alternatives(cfg):
    """Top-level arguments of `any(...)`, or the whole condition."""
    outer = re.fullmatch(r'\s*any\s*\((.*)\)\s*', cfg, re.S)
    if not outer:
        return [cfg.strip()]
    parts, depth, start = [], 0, 0
    for index, char in enumerate(outer[1]):
        depth += (char == "(") - (char == ")")
        if char == "," and depth == 0:
            parts.append(outer[1][start:index].strip())
            start = index + 1
    return parts + [outer[1][start:].strip()]


def platform_condition(cfg):
    """True when `cfg(...)` removes the item on some OS even in a test build."""
    # A `test` alternative with no OS term, such as `all(test, not(target_arch = "wasm32"))`,
    # keeps the module in every operating system's test harness.
    if any(re.search(r'(?<![\w"])test(?![\w"])', part) and not PLATFORM.search(part) for part in alternatives(cfg)):
        return False
    return bool(PLATFORM.search(cfg))


DECLARED_MODULE = re.compile(r'\bmod\s+(\w+)\s*;')


def child_dir(path):
    return path.parent if path.stem in {"lib", "main", "mod"} else path.with_suffix("")


def module_files(path, name):
    """Files of `mod name;` declared in `path`, then their declared children."""
    found, pending = {}, []
    # Crate roots under tests/, benches/ and src/bin/ resolve beside themselves; try both.
    for base in dict.fromkeys((child_dir(path), path.parent)):
        pending += [child for child in (base / f"{name}.rs", base / name / "mod.rs") if child.is_file()]
    while pending:
        child = pending.pop()
        if child in found:
            continue
        found[child] = mask(child.read_text(errors="ignore"), strings=True)
        for declared in DECLARED_MODULE.finditer(found[child]):
            base = child_dir(child)
            pending += [c for c in (base / f"{declared[1]}.rs", base / declared[1] / "mod.rs") if c.is_file()]
    return found


def has_tests(files):
    return any(TEST_DECLARATION.search(code) for code in files.values())


def os_test_module_sites(root, relative, code, text):
    """Find OS conditions that remove tests: on a test module, or on any module or file holding tests."""
    attributes = re.compile(r'(?P<attrs>(?:\#\s*\[[^\]]*\]\s*)+)'
                            r'(?:pub(?:\([^)]*\))?\s+)?mod\s+(?P<name>\w+)\s*(?P<end>[;{])')
    test_only = re.compile(r'^\s*(?:test\s*$|all\s*\([^\]]*\btest\b)')
    path = root / relative
    for match in attributes.finditer(code):
        conditions = list(re.finditer(r'\#\s*\[\s*cfg\s*\(([^\]]*)\)\s*\]', match["attrs"]))
        platform = [condition for condition in conditions if platform_condition(condition[1])]
        if not platform:
            continue
        site = match.start() + platform[0].start()
        name = match["name"]
        if any(test_only.search(attribute[1]) for attribute in conditions) or name in {"test", "tests"}:
            yield site
            continue
        if PLATFORM_ONLY.search(text, 0, match.start()):
            continue
        if match["end"] == "{":
            end, depth = match.end(), 1
            while end < len(code) and depth:
                depth += (code[end] == "{") - (code[end] == "}")
                end += 1
            if TEST_DECLARATION.search(code, match.end(), end):
                yield site
        elif has_tests(module_files(path, name)):
            yield site
    # A file-level condition hides the file's tests and its child modules'.
    if not test_file(relative):
        for match in re.finditer(r'#!\s*\[\s*cfg\s*\(([^\]]*)\)\s*\]', code):
            if platform_condition(match[1]) and not PLATFORM_ONLY.search(text, 0, match.start()):
                children = {}
                for declared in DECLARED_MODULE.finditer(code):
                    children |= module_files(path, declared[1])
                if TEST_DECLARATION.search(code) or has_tests(children):
                    yield match.start()


def rust_sites(root, relative, text, package, manifest, registry):
    """Yield (rule, byte offset) for each source site, once per rule."""
    comments = mask(text)
    code = mask(text, strings=True)
    if relative.startswith(("crates/", "examples/")):
        for offset, returned in public_returns(code):
            result = re.match(r'(?:::)?\s*(?:(?:std|core)\s*::\s*result\s*::\s*)?Result\s*<', returned)
            if not result:
                continue
            tail = returned[result.end():]
            depth, parens, brackets, braces, comma = 1, 0, 0, 0, None
            for index, char in enumerate(tail):
                if not braces:
                    depth += (char == "<") - (char == ">" and (index == 0 or tail[index - 1] != "-"))
                parens += (char == "(") - (char == ")")
                brackets += (char == "[") - (char == "]")
                braces += (char == "{") - (char == "}")
                if char == "," and depth == 1 and parens == brackets == braces == 0 and comma is None:
                    comma = index
                if depth == 0:
                    error = tail[comma + 1:index] if comma is not None else ""
                    if re.fullmatch(r'\s*(?:::)?\s*(?:(?:std|alloc)\s*::\s*string\s*::\s*)?String\s*,?\s*', error):
                        yield "string-errors", offset
                    break
    for offset, target in path_attributes(comments, code):
        yield "path-attribute", offset
        if package and not (root / Path(relative).parent / target).resolve().is_relative_to((root / package).resolve()):
            yield "package-reach", offset
    for match in re.finditer(r'\binclude(?:_str|_bytes)?\s*!\s*\(\s*(?:r(\#*)"(.*?)"\1|"((?:\\.|[^"\\])*)")', comments, re.S):
        if code[match.start():match.start() + 7] != "include":
            continue
        target = match.group(2) if match.group(2) is not None else match.group(3)
        if package and not (root / Path(relative).parent / target).resolve().is_relative_to((root / package).resolve()):
            yield "package-reach", match.start()
    library = manifest and ((root / package / "src/lib.rs").exists() or "lib" in manifest)
    if library and not test_file(relative) and Path(relative).name != "build.rs":
        name = manifest.get("package", {}).get("name")
        if registry.get("packages", {}).get(name, {}).get("test_role") != "support":
            for match in re.finditer(r'\bstd\s*::\s*env\s*::\s*(?:var|var_os|vars)\b', without_test_modules(code)):
                # Binaries inside a library package are process edges too.
                local = Path(relative).relative_to(package).as_posix()
                if local != "src/main.rs" and not local.startswith("src/bin/"):
                    yield "library-env", match.start()
    if not device_owner(relative, registry):
        for match in re.finditer(r'(?:\.|::)\s*(?:request_adapter|request_device)\s*\(', code):
            yield "device-request", match.start()
    if test_file(relative):
        for match in re.finditer(r'\binclude\s*!\s*[(\[{]', code):
            yield "path-attribute", match.start()
        for match in re.finditer(r'#!\s*\[\s*cfg\s*\([^\]]*\b(?:target_(?:os|family)\s*=|(?:unix|windows)\b)', code):
            yield "whole-file-gpu-gate", match.start()
    if test_file(relative) and not support_package(relative, registry):
        for offset in readback_sites(code, relative):
            yield "test-readback", offset
        for match in DEADLINE_ASSERT.finditer(code):
            yield "test-deadline", match.start()
    for offset in os_test_module_sites(root, relative, code, text):
        yield "whole-file-gpu-gate", offset
    for match in LONG_IGNORE.finditer(comments):
        yield "main-tier", match.start()
    # Coarse on purpose: any text edit in a non-test file that mentions WGSL anywhere.
    if not test_file(relative) and re.search(r'(?i)wgsl', text):
        for match in TEXT_EDIT.finditer(without_test_modules(code)):
            yield "shader-text-edit", match.start()


def dependency_sites(manifest, package):
    """All kinds, including target-specific, renamed and workspace dependencies."""
    def visit(table):
        for key, value in table.items():
            if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
                yield from value.items()
            elif isinstance(value, dict):
                yield from visit(value)
    yield from visit(manifest)


def test_edge_sites(manifest, workspace, registry):
    """Resolve dependency identities across renamed, inherited and target tables."""
    roles = {name: entry.get("test_role") for name, entry in registry.get("packages", {}).items()}
    consumer = manifest.get("package", {}).get("name")
    proving_ground = registry.get("boundaries", {}).get("proving_ground")

    def visit(table):
        for kind, value in table.items():
            if kind in {"dependencies", "dev-dependencies", "build-dependencies"}:
                for alias, dependency in value.items():
                    if isinstance(dependency, dict) and dependency.get("workspace"):
                        dependency = workspace.get(alias, {})
                    name = dependency.get("package", alias) if isinstance(dependency, dict) else alias
                    role = roles.get(name)
                    if ((role == "support" and kind != "dev-dependencies")
                            or (role == "fixtures" and (kind == "build-dependencies"
                                or (kind == "dependencies" and consumer != proving_ground)))):
                        yield alias
            elif isinstance(value, dict):
                yield from visit(value)
    yield from visit(manifest)


def evidence_path_lines(relative, text):
    """Lines of a source file under crates, examples, scripts or tools that name the evidence folder."""
    path = Path(relative)
    if (not path.parts or path.parts[0] not in {"crates", "examples", "scripts", "tools"}
            or path.suffix not in SOURCE_SUFFIXES or relative in EVIDENCE_STORE):
        return []
    return sorted({text.count("\n", 0, match.start()) + 1 for match in EVIDENCE_PATH.finditer(text)})


TOKEN = re.compile(r'[A-Za-z0-9_.\-]+')
IMPORT = re.compile(r'(?m)^[ \t]*(?:from[ \t]+\.*([\w.]+)[ \t]+import|import[ \t]+([\w., \t]+))'
                    r'|import_module\(\s*["\x27]([\w.]+)')


def names_in(text):
    """File names a text spells out, and Python modules it imports."""
    names = {token.rstrip(".") for token in TOKEN.findall(text)}
    modules = set()
    for match in IMPORT.finditer(text):
        for group in match.groups():
            for part in re.split(r'[,\s]+', group or ""):
                if part:
                    modules.add(part.split(".")[-1])
                    modules.add(part.split(".")[0])
    return names, modules


def caller_document(relative):
    """A tracked document a reader acts on: outside the plans folder and the evidence folder."""
    return (relative.endswith(".md") and not relative.startswith(("docs/plans/", EVIDENCE_FOLDER))
            and not relative.startswith("scripts/"))


def runner_file(relative):
    return (relative == "dev" or relative.startswith(("tools/xtask/", ".github/workflows/"))
            or caller_document(relative))


def read(root, relative):
    try:
        return (root / relative).read_text(errors="ignore")
    except OSError:
        return ""


def done_when_checks(root):
    """Every done-when item's check, so closing a work package moves no count."""
    checks = []
    for path in sorted((root / "docs/plans").glob("*/DONE.json")):
        try:
            plan = json.loads(path.read_text())
        except ValueError:
            continue
        for work_package in plan.get("work_packages", []):
            checks.extend(item["check"] for item in work_package.get("items", []) if item.get("check"))
    return checks


XTASK_MAIN = "tools/xtask/src/main.rs"
EXCLUDED_SUITES = re.compile(r'const EXCLUDED_SCRIPT_SUITES\b.*?\];', re.S)


def uncalled_scripts(root, paths):
    """Files under scripts/ that nothing names, as the rule `script-caller` defines it.

    A name is a file name spelled out as a whole token, or a Python module imported.
    Callers: ./dev, tools/xtask, CI workflows, Rust source, every done-when check,
    tracked documents outside the plans and evidence folders (ORCHESTRATE.md's
    `Main-branch check` lines among them), and a script that is itself named. A suite
    `test_X` lives and is counted with a script `X` beside it; one named for no script
    is not counted. `./dev check` runs every suite directly under scripts/ but those
    its EXCLUDED_SCRIPT_SUITES names, so such a suite it runs is a caller of what it imports.
    """
    scripts = [p for p in paths if p.startswith("scripts/") and not p.endswith(".md")]
    if not scripts:
        return []
    def named(script, names, modules):
        path = Path(script)
        return path.name in names or (path.suffix == ".py" and path.stem in modules)
    roots = [read(root, p) for p in paths
             if not p.startswith(("scripts/", EVIDENCE_FOLDER))
             and (runner_file(p) or p.endswith(".rs"))]
    roots += done_when_checks(root)
    root_names, root_modules = set(), set()
    for text in roots:
        names, modules = names_in(text)
        root_names |= names
        root_modules |= modules
    script_names = {s: names_in(read(root, s)) for s in scripts}
    live = {s for s in scripts if named(s, root_names, root_modules)}
    def script_for(suite):
        path = Path(suite)
        if not path.stem.startswith("test_"):
            return None
        beside = [s for s in scripts if Path(s).parent == path.parent and Path(s).stem == path.stem[5:]]
        return beside[0] if beside else None
    lone = {s for s in scripts if Path(s).stem.startswith("test_") and script_for(s) is None}
    xtask = read(root, XTASK_MAIN)
    exclusions = EXCLUDED_SUITES.search(xtask)
    if xtask and not exclusions:
        # Without the list, every excluded suite would count as run and hide the scripts it tests.
        raise ValueError(f"{XTASK_MAIN} no longer defines EXCLUDED_SCRIPT_SUITES; "
                         "script-caller reads the suites ./dev check excludes from it")
    excluded = set(re.findall(r'"(test_\w+)"', exclusions.group(0))) if exclusions else set()
    live |= {s for s in lone if Path(s).parent == Path("scripts") and Path(s).stem not in excluded}
    changed = True
    while changed:
        changed = False
        for script in scripts:
            if script in live:
                continue
            target = script_for(script)
            if (target in live) or any(named(script, *script_names[c]) for c in live if c != script):
                live.add(script)
                changed = True
    return sorted(s for s in scripts if s not in live and s not in lone)


BENCH_WORD = re.compile(r'(?i)(?<![\w])bench(?:es)?(?![\w])')


def bench_names_in(text):
    """Names a runner gives benches: a compound token such as `valley_culling` anywhere; a plain word such
    as `frame` only on a line that says bench or the two lines after it, since plain words are everywhere."""
    lines = text.splitlines()
    names = {m.group(1) for m in re.finditer(r'benches/([\w-]+)\.rs', text)}
    names |= {token for token in (t.rstrip(".") for t in TOKEN.findall(text)) if re.search(r'[\w][_-][\w]', token)}
    for index, line in enumerate(lines):
        if BENCH_WORD.search(line):
            for near in lines[index:index + 3]:
                names |= {token.rstrip(".") for token in TOKEN.findall(near)}
    return names


def unrun_benches(root, paths, manifests):
    """[[bench]] targets that no script, ./dev command, CI workflow or document names.

    Yields (manifest path, line) for each, as the rule `bench-runner` defines it.
    """
    runners = set()
    for relative in paths:
        if relative.startswith("scripts/") or runner_file(relative):
            runners |= bench_names_in(read(root, relative))
    for folder, manifest in sorted(manifests.items()):
        relative = (folder / "Cargo.toml").as_posix()
        text = read(root, relative)
        for bench in manifest.get("bench", []):
            name = bench.get("name", "")
            if name and name not in runners:
                match = re.search(r'(?m)^\s*name\s*=\s*["\x27]' + re.escape(name) + r'["\x27]', text)
                yield relative, (text.count("\n", 0, match.start()) + 1 if match else 1)


NEXTEST = ".config/nextest.toml"
LONG_TESTS = ".config/long-tests.txt"
UNPLANNED_TESTS = ".config/unplanned-test-binaries.txt"
# The test tiers' limit on one test in a merge's check, in seconds: the default may not be later
# (GATE-TIME mechanism 13; `./dev check` runs a test it stops once more, alone, under the same limit).
TEST_LIMIT_SECONDS = 10
EXEMPT_TERM = re.compile(r'\(\s*binary_id\(=([\w-]+)::([\w-]+)\)\s*&\s*test\(=(\w+(?:::\w+)*)\)\s*\)')
EXEMPT_FILTER = re.compile(r'\s*' + EXEMPT_TERM.pattern + r'(?:\s*\|\s*' + EXEMPT_TERM.pattern + r')*\s*')
DURATION = re.compile(r'(\d+(?:\.\d+)?)\s*(ms|s|m|h)')
TEST_FLAG = re.compile(r'--test(?:=|\s+)["\x27]?([\w-]+)')


def line_of(text, offset):
    return text.count("\n", 0, max(offset, 0)) + 1


def seconds(duration):
    """A cargo-nextest duration such as `10s` or `1m 30s`, in seconds; None if it reads otherwise."""
    if not isinstance(duration, str) or not re.fullmatch(r'(?:\s*' + DURATION.pattern + r')+\s*', duration):
        return None
    scale = {"ms": 0.001, "s": 1, "m": 60, "h": 3600}
    return sum(float(value) * scale[unit] for value, unit in DURATION.findall(duration))


def default_limit_line(text):
    """The line of the default profile's `slow-timeout`, as a key or a sub-table, else of the profile's header."""
    table = re.search(r'(?m)^[ \t]*\[\s*profile\.default\.slow-timeout\s*\]', text)
    if table:
        return line_of(text, table.start())
    header = re.search(r'(?m)^[ \t]*\[\s*profile\.default\s*\][ \t]*$', text)
    if not header:
        return 1
    following = re.search(r'(?m)^[ \t]*\[', text[header.end():])
    body = text[header.end():header.end() + following.start()] if following else text[header.end():]
    key = re.search(r'(?m)^[ \t]*slow-timeout\s*=', body)
    return line_of(text, header.end() + key.start()) if key else line_of(text, header.start())


def limit_exempt(root, paths):
    """The tests exempt from the limit on one test, and the overrides that cannot be counted.

    Returns ([(path, line)] for each test an override names, [(path, line, why)] refused outright).
    An override that changes the limit is one with a `slow-timeout`; it names its tests one by one, exactly.
    """
    if NEXTEST not in paths:
        return [], []
    text = read(root, NEXTEST)
    try:
        config = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        return [], [(NEXTEST, 1, f"cannot read the runner's configuration: {error}")]
    profile = config.get("profile", {}).get("default", {})
    sites, refused = [], []
    # A missing default is cargo-nextest's own, a 60 s period that stops no test, so it is refused too.
    timeout = profile.get("slow-timeout")
    if isinstance(timeout, str):
        timeout = {"period": timeout}
    period = seconds(timeout.get("period")) if isinstance(timeout, dict) else None
    after = timeout.get("terminate-after") if isinstance(timeout, dict) else None
    if period is None or not isinstance(after, int) or period * after > TEST_LIMIT_SECONDS:
        refused.append((NEXTEST, default_limit_line(text), f"the default limit on one test is missing, later than "
                        f"the test tiers' {TEST_LIMIT_SECONDS} s, or never stops a test: give [profile.default] a "
                        f"slow-timeout whose period times terminate-after is {TEST_LIMIT_SECONDS} s or less"))
    # The n-th table header in the text is the n-th override, so each site has its own line.
    headers = [match.start() for match in re.finditer(r'(?m)^[ \t]*\[\[\s*profile\.default\.overrides\s*\]\]', text)]
    for index, override in enumerate(profile.get("overrides", [])):
        if "slow-timeout" not in override:
            continue
        start = headers[index] if index < len(headers) else 0
        filter_text = override.get("filter")
        if not isinstance(filter_text, str) or not EXEMPT_FILTER.fullmatch(filter_text):
            refused.append((NEXTEST, line_of(text, start), "an override that changes the limit on one test names "
                            "something wider than single tests, so its tests cannot be counted"))
            continue
        found = text.find(filter_text, start)
        for term in EXEMPT_TERM.finditer(filter_text):
            sites.append((NEXTEST, line_of(text, found + term.start()) if found >= 0 else line_of(text, start)))
    return sites, refused


def main_tier_lines(root, paths):
    """Each entry of the main-tier list, as (line, fields)."""
    if LONG_TESTS not in paths:
        return []
    entries = []
    for number, line in enumerate(read(root, LONG_TESTS).splitlines(), 1):
        fields = line.split("#", 1)[0].split()
        if fields:
            entries.append((number, fields))
    return entries


def main_tier_binaries(entries):
    """(package, binary) for each test binary the main-tier list names."""
    named = set()
    for _, fields in entries:
        if len(fields) < 2:
            continue
        if fields[1] == "not-run":
            named.update((fields[0], name) for name in fields[2:3])
        elif fields[1].startswith("features="):
            named.update((fields[0], name) for name in fields[2:])
        else:
            named.add((fields[0], fields[1]))
    return named


def test_targets(root, paths, manifests):
    """Each integration test target as Cargo finds it: (package, name, source path, manifest line).

    A `[[test]]` table is a target; with `autotests` on (Cargo's default), so is each `tests/<name>.rs`
    and `tests/<name>/main.rs` that no table already builds. A helper module in a folder under `tests/`
    is not a target.
    """
    files = set(paths)
    targets = []
    for folder, manifest in sorted(manifests.items()):
        package = manifest.get("package", {})
        name = package.get("name")
        if not name:
            continue
        relative = (folder / "Cargo.toml").as_posix()
        text = read(root, relative)
        explicit_names, explicit_paths = set(), set()
        for table in manifest.get("test", []):
            target = table.get("name")
            if not target:
                continue
            source = table.get("path")
            if source is None:
                single = (folder / "tests" / f"{target}.rs").as_posix()
                source = single if single in files else (folder / "tests" / target / "main.rs").as_posix()
            else:
                source = (folder / source).as_posix()
            match = re.search(r'(?m)^\s*name\s*=\s*["\x27]' + re.escape(target) + r'["\x27]', text)
            targets.append((name, target, source, relative, line_of(text, match.start()) if match else 1))
            explicit_names.add(target)
            explicit_paths.add(Path(source).as_posix())
        if package.get("autotests", True) is False:
            continue
        prefix = (folder / "tests").as_posix() + "/"
        for path in sorted(files):
            if not path.startswith(prefix):
                continue
            rest = path[len(prefix):].split("/")
            if len(rest) == 1 and rest[0].endswith(".rs"):
                target = rest[0][:-3]
            elif len(rest) == 2 and rest[1] == "main.rs":
                target = rest[0]
            else:
                continue
            if target in explicit_names or path in explicit_paths:
                continue
            targets.append((name, target, path, path, 1))
    return targets


SHELL_SEPARATOR = re.compile(r'&&|\|\||[;|\n]')


def checked_targets(check, packages, folders):
    """(package, target) pairs a done-when check runs with `--test`.

    A command names a target only in the packages it selects: each `-p`/`--package`, the package of a
    `--manifest-path`, or else the package of the folder an earlier `cd` in the check entered, or else, for
    `--workspace` or a command run in the workspace root, every package.
    """
    named, folder = set(), None
    for segment in SHELL_SEPARATOR.split(check):
        tokens = [token.strip("\"'()") for token in segment.split()]
        tokens = [token for token in tokens if token]
        if len(tokens) >= 2 and tokens[0] == "cd":
            folder = tokens[1].rstrip("/")
            continue
        targets = [match.group(1) for match in TEST_FLAG.finditer(" ".join(tokens))]
        if not targets:
            continue
        selected = set()
        for index, token in enumerate(tokens):
            value = None
            if token in ("-p", "--package") and index + 1 < len(tokens):
                value = tokens[index + 1]
            elif token.startswith("--package="):
                value = token.split("=", 1)[1]
            elif token.startswith("-p") and len(token) > 2 and not token.startswith("--"):
                value = token[2:]
            elif token == "--manifest-path" and index + 1 < len(tokens):
                selected.add(folders.get(str(Path(tokens[index + 1]).parent), ""))
            elif token.startswith("--manifest-path="):
                selected.add(folders.get(str(Path(token.split("=", 1)[1]).parent), ""))
            if value:
                selected.add(value.split("@", 1)[0])
        if not selected or "--workspace" in tokens or "--all" in tokens:
            selected |= {folders[folder]} if folder in folders and "--workspace" not in tokens else packages
        named |= {(package, target) for package in selected for target in targets}
    return named


def test_binary(root, paths, manifests):
    """The lines of the list of test targets no plan names, and the targets refused outright.

    Returns ([(path, line)] for each line of the list, [(path, line, why)]). A plan names a target when a
    done-when item's check, open or Done, runs it with `--test`, or when the main-tier list names it. A
    package's one target `it` needs neither a name nor a line.
    """
    packages = {manifest["package"]["name"] for manifest in manifests.values() if manifest.get("package", {}).get("name")}
    folders = {folder.as_posix(): manifest["package"]["name"] for folder, manifest in manifests.items()
               if manifest.get("package", {}).get("name")}
    planned = set()
    for check in done_when_checks(root):
        planned |= checked_targets(check, packages, folders)
    in_main_tier = main_tier_binaries(main_tier_lines(root, paths))
    targets = test_targets(root, paths, manifests)
    existing = {(package, name) for package, name, *_ in targets}
    listed, sites, refused = set(), [], []
    if UNPLANNED_TESTS in paths:
        for number, line in enumerate(read(root, UNPLANNED_TESTS).splitlines(), 1):
            fields = line.split("#", 1)[0].split()
            if not fields:
                continue
            sites.append((UNPLANNED_TESTS, number))
            if len(fields) != 2:
                refused.append((UNPLANNED_TESTS, number, "expected `<package> <test target>`"))
                continue
            key = (fields[0], fields[1])
            if key in listed:
                refused.append((UNPLANNED_TESTS, number, f"{fields[0]} {fields[1]} is listed twice: remove a line"))
            elif key not in existing:
                refused.append((UNPLANNED_TESTS, number, f"{fields[0]} has no test target {fields[1]}: remove the line"))
            listed.add(key)
    for package, name, source, where, line in targets:
        if name == "it" or (package, name) in planned or (package, name) in in_main_tier or (package, name) in listed:
            continue
        refused.append((where, line, f"test target {package} {name} ({source}) is named by no plan and not listed in "
                        f"{UNPLANNED_TESTS}"))
    return sites, refused

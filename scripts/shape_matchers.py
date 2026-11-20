"""Text rules, not a Rust type checker. Comments and literals retain source offsets.

Rules recognise spelled-out APIs (including whitespace and multiline signatures),
not aliases or macro expansion. Keep this envelope explicit when adding a rule.
"""
from pathlib import Path
import re
import tomllib


RULES = {
    "test-crate-edges": (1, "tests", "use support only in dev-dependencies and fixtures normally only in the proving ground"),
    "evidence-size": (1, "evidence", "keep bulk captures in tmp/<task>/ with a reproduction record, or run python3 scripts/evidence_store.py put SOURCE --path docs/evidence/<module>/<capture>; read docs/ASSET-LIFECYCLE.md (local store only, no remote backup)"),
    "string-errors": (4, "errors", "return a boundary error enum implementing Display and Error"),
    "package-reach": (1, "crossing-crate-boundaries", "use a public package API or registered shared fixture package"),
    "path-attribute": (1, "tests", "use the normal module tree (foo.rs and foo/)"),
    "library-env": (2, "configuration", "parse settings at the process edge and pass typed values to the library"),
    "device-request": (1, "device-setup", "use the shared capabilities device recipe and test-support gate"),
    "whole-file-gpu-gate": (1, "tests", "gate only the individual tests that need a GPU"),
    "history-names": (3, "names", "name the file, variable or Cargo target for what it does"),
}
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


def public_returns(code):
    """Find the return arrow outside a public function's parameters/generics.

    Semicolons in arrays and braces in const generics are part of the type,
    while a body or declaration without a return arrow ends the search.
    """
    prefix = r'\bpub\s+(?:(?:async|unsafe|const|extern)\s+)*fn\s+\w+'
    for function in re.finditer(prefix, code):
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


def platform_condition(cfg):
    """True when `cfg(...)` removes the item on some OS even in a test build."""
    # A top-level `test` alternative keeps the module in every test harness.
    alternatives = re.fullmatch(r'\s*any\s*\((.*)\)\s*', cfg, re.S)
    if alternatives and "test" in (part.strip() for part in re.sub(r'\([^()]*\)', '', alternatives[1]).split(",")):
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
    for offset in os_test_module_sites(root, relative, code, text):
        yield "whole-file-gpu-gate", offset


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

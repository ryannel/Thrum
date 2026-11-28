#!/usr/bin/env python3
"""Check resolved type identities and examples in the Game SDK's public API."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
OWN_CRATES = {"thrum_derive"}
STANDARD_CRATES = {"core", "alloc", "std"}
# Cargo uses the repository-pinned rustdoc. Fail explicitly on schema changes.
RUSTDOC_FORMAT = 61
RUST_EXAMPLE = re.compile(r"^\s*```(?:rust)?\s*$\n.+?^\s*```\s*$", re.M | re.S)


def cargo(arguments, directory=ROOT, crate=None):
    env = os.environ.copy()
    if crate:
        # JSON output is unstable. Scope the opt-in to just the documented crate.
        env["RUSTC_BOOTSTRAP"] = crate
    result = subprocess.run(
        ["cargo", *arguments], cwd=directory, env=env, check=True,
        text=True, stdout=subprocess.PIPE,
    )
    return result.stdout


def document(crate, directory=ROOT):
    metadata = json.loads(cargo(["metadata", "--locked", "--offline", "--format-version", "1",
                                 "--no-deps"], directory))
    cargo(["rustdoc", "--locked", "--offline", "-p", crate, "--lib", "--",
           "-Z", "unstable-options", "--output-format", "json",
           "--document-private-items", "--document-hidden-items"], directory,
          crate.replace("-", "_"))
    with (Path(metadata["target_directory"]) / "doc" / f'{crate.replace("-", "_")}.json').open() as source:
        result = json.load(source)
    if result["format_version"] != RUSTDOC_FORMAT:
        raise ValueError(f"rustdoc JSON format {result['format_version']}, expected {RUSTDOC_FORMAT}")
    return result


def referenced_paths(value):
    """Rustdoc paths use id, path and args, unlike item and relationship IDs."""
    if isinstance(value, dict):
        if {"id", "path", "args"} <= value.keys():
            yield value["id"]
        for child in value.values():
            yield from referenced_paths(child)
    elif isinstance(value, list):
        for child in value:
            yield from referenced_paths(child)


class Surface:
    def __init__(self, document, examples):
        self.document = document
        self.examples = examples
        self.errors = set()
        self.visited = set()
        self.entries = {int(key): value for key, value in document["index"].items()}
        self.paths = {int(key): value for key, value in document["paths"].items()}

    def check(self):
        self.visit(self.document["root"], self.entries[self.document["root"]]["name"])
        return sorted(self.errors)

    def check_path(self, path_id, owner, followed=None):
        if path_id is None:
            return
        path = self.paths.get(path_id)
        if path is None:
            if path_id not in self.entries:
                self.errors.add(f"{owner} references unresolved item {path_id}")
            return
        if path["crate_id"]:
            crate = self.document["external_crates"][str(path["crate_id"])]["name"]
            if crate not in OWN_CRATES | STANDARD_CRATES:
                self.errors.add(f"{owner} exposes {'::'.join(path['path'])}")
            return
        entry = self.entries.get(path_id, {}).get("inner", {})
        if entry and not {"module", "use"}.intersection(entry):
            self.visit(path_id, "::".join(path["path"]))
        if "type_alias" in entry:
            if followed is None:
                followed = set()
            if path_id not in followed:
                followed.add(path_id)
                for child in referenced_paths(entry["type_alias"]):
                    self.check_path(child, owner, followed)

    def visit(self, item_id, label, inherited=False):
        entry = self.entries.get(item_id)
        if not entry or entry["crate_id"] or item_id in self.visited:
            return
        kind, payload = next(iter(entry["inner"].items()))
        if entry["visibility"] != "public" and not inherited:
            return
        self.visited.add(item_id)
        if kind == "use":
            self.check_path(payload["id"], label)
            self.visit(payload["id"], label, inherited=True)
            return
        signature = payload
        nested = []
        if kind == "impl":
            if payload["is_synthetic"] or payload["blanket_impl"] is not None:
                return
            signature = {key: payload[key] for key in ("for", "trait", "generics")}
            trait = payload["trait"]
            if trait is not None:
                declaration = self.entries.get(trait["id"])
                if declaration is not None and not declaration["crate_id"] and declaration["visibility"] != "public":
                    return
            for child in payload["items"]:
                child_entry = self.entries.get(child)
                if child_entry and child_entry["span"] is not None:
                    # A trait declaration documents its own members. Check the
                    # implementing signature even when the declared trait is std.
                    nested.append((child, trait is not None, trait is None))
        elif kind == "module":
            nested.extend((child, False, True) for child in payload["items"])
        elif kind in {"struct", "union", "enum", "trait", "variant"}:
            for child in payload.get("items", payload.get("variants", [])):
                nested.append((child, True, True))
            for child in payload.get("impls", []):
                nested.append((child, True, False))
            shape = payload.get("kind")
            fields = payload.get("fields", [])
            if isinstance(shape, dict):
                layout, data = next(iter(shape.items()))
                fields = data if layout == "tuple" else data.get("fields", [])
            for child in fields:
                if child is not None:
                    nested.append((child, kind == "variant", True))
        for path_id in referenced_paths(signature):
            self.check_path(path_id, label)
        if self.examples and self.needs_example(entry, kind, inherited):
            if not RUST_EXAMPLE.search(entry["docs"] or ""):
                self.errors.add(f"{label} needs a runnable Rust documentation example")
        for child_id, child_inherited, child_docs in nested:
            child = self.entries.get(child_id)
            if child is None:
                continue
            child_kind, child_payload = next(iter(child["inner"].items()))
            child_name = child["name"]
            if child_kind == "use":
                child_name = child_payload["name"]
            child_label = f"{label}::{child_name}" if child_name else label
            self.visit(child_id, child_label, child_inherited)
            if self.examples and child_docs and child["visibility"] == "default":
                if child_kind in {"function", "assoc_type", "assoc_const", "variant", "struct_field"}:
                    if not RUST_EXAMPLE.search(child["docs"] or ""):
                        self.errors.add(f"{child_label} needs a runnable Rust documentation example")

    @staticmethod
    def needs_example(entry, kind, inherited):
        return (kind not in {"impl", "extern_crate"}
                and entry["visibility"] == "public" and (not inherited or kind == "module"))


CONTROLS = {
    "private_import": ("mod hidden { use backend::Handle; pub struct Item; impl Item { pub fn get(&self) -> Handle { Handle } } } pub fn item() -> hidden::Item { hidden::Item }", "hidden::Item::get"),
    "alias": ("pub type Entity = backend::Handle;", "Entity"),
    "reexport": ("pub use backend::Handle as Entity;", "Entity"),
    "field": ("pub struct Item { pub handle: backend::Handle }", "Item::handle"),
    "nested": ("pub struct Item; impl Item { pub fn get() -> Option<backend::Handle> { None } }", "Item::get"),
    "own_type": ("pub struct Entity; pub fn entity() -> Option<Entity> { None }", None),
    "generic": ("pub struct Content<T>(T); impl<T> Content<T> { pub fn get(&self) -> &T { &self.0 } }", None),
}


def controls():
    scratch = ROOT / "tmp" / "sdk-surface"
    scratch.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="controls-", dir=scratch) as path:
        directory = Path(path)
        (directory / "backend" / "src").mkdir(parents=True)
        (directory / "src").mkdir()
        (directory / "backend" / "Cargo.toml").write_text(
            '[package]\nname = "backend"\nversion = "0.0.0"\nedition = "2024"\n')
        (directory / "backend" / "src" / "lib.rs").write_text("pub struct Handle;\n")
        (directory / "Cargo.toml").write_text(
            '[workspace]\n[package]\nname = "surface_control"\nversion = "0.0.0"\n'
            'edition = "2024"\n[dependencies]\nbackend = { path = "backend" }\n')
        (directory / "src" / "lib.rs").touch()
        cargo(["generate-lockfile", "--offline"], directory)
        for name, (source, item) in CONTROLS.items():
            (directory / "src" / "lib.rs").write_text(source + "\n")
            actual = Surface(document("surface_control", directory), False).check()
            expected = [f"surface_control::{item} exposes backend::Handle"] if item else []
            if actual != expected:
                raise ValueError(f"{name}: expected {expected}, got {actual}")
            print(f"{name}: {actual[0] if actual else 'accepted'}")


def check():
    errors = Surface(document("thrum"), True).check()
    errors.extend(Surface(document("thrum-derive"), True).check())
    if errors:
        raise ValueError("\n".join(sorted(errors)))
    print("Game SDK public signatures and documentation examples passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["check", "controls"])
    command = parser.parse_args().command
    try:
        {"check": check, "controls": controls}[command]()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"SDK surface: {error}\n")


if __name__ == "__main__":
    main()

"""Violating and corrected fixtures for every registered M1 text rule."""
import contextlib
import io
import json
from pathlib import Path
import tempfile
import subprocess
import unittest

import shape_rules as shapes


class Fixture(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()
        self.write("crates/engine/Cargo.toml", '[package]\nname = "engine"\nversion = "0.1.0"\n')
        self.write("crates/engine/src/lib.rs", "")

    def write(self, path, text):
        destination = self.root / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(text)

    def sites(self, rule):
        return shapes.inventory(self.root)[1][rule]

    def pair(self, rule, bad, good, path="crates/engine/src/lib.rs"):
        self.write(path, bad)
        with contextlib.redirect_stdout(io.StringIO()):
            errors = shapes.check(self.root, rule, [path])
        self.assertTrue(errors, (rule, bad))
        self.assertIn(f"{path}:1: {rule}:", "\n".join(errors))
        self.assertIn("patterns.md#", "\n".join(errors))
        self.write(path, good)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(shapes.check(self.root, rule, [path]), [])

    def test_string_errors(self):
        self.pair("string-errors", "pub async fn load() -> Result<\n Vec<(u8, u8)>, String\n> { todo!() }",
                  "pub async fn load() -> Result<Vec<(u8, u8)>, LoadError> { todo!() }")
        self.pair("string-errors", "pub fn load() -> Result<(u8, fn() -> u8), String,> { todo!() }",
                  "pub fn load() -> Result<(u8, fn() -> u8), LoadError,> { todo!() }")
        self.pair("string-errors", "pub fn load(x: [u8; 4]) -> Result<[u8; 4], String> { todo!() }",
                  "pub fn load(x: [u8; 4]) -> Result<[u8; 4], LoadError> { todo!() }")
        self.pair("string-errors", "pub fn load() -> Result<Array<{1 > 0}>, String> { todo!() }",
                  "pub fn load() -> Result<Array<{1 > 0}>, LoadError> { todo!() }")
        self.pair("string-errors", "pub fn load() -> ::std::result::Result<(), ::std::string::String> { todo!() }",
                  "pub fn load() -> ::std::result::Result<(), LoadError> { todo!() }")
        self.pair("string-errors", "extern crate alloc; pub fn load() -> ::core::result::Result<(), alloc::string::String> { todo!() }",
                  "extern crate alloc; pub fn load() -> ::core::result::Result<(), LoadError> { todo!() }")
        self.write("crates/engine/src/lib.rs", 'pub(crate) fn local() -> Result<(), String> { todo!() }\n// pub fn fake() -> Result<(), String> {}\nconst DOC: &str = r#"pub fn fake() -> Result<(), String> {}"#;')
        self.assertEqual(self.sites("string-errors"), [])
        self.write("crates/engine/src/lib.rs", 'pub fn local<F>(f: F) where F: Fn() -> Result<(), String> { f(); }')
        self.assertEqual(self.sites("string-errors"), [])
        self.write("crates/engine/src/lib.rs", 'pub fn run(game: impl FnOnce() -> Result<(), String>) { game(); }')
        self.assertEqual(self.sites("string-errors"), [])

    def test_include_reach_and_path_attribute(self):
        for macro in ("include", "include_str", "include_bytes"):
            with self.subTest(macro=macro):
                self.pair("package-reach", f'const X: &str = {macro}!("../../other/private.rs");',
                          f'const X: &str = {macro}!("local.rs");')
        self.pair("package-reach", '#[path = "../../other/private.rs"] mod other;', 'mod other;')
        self.pair("path-attribute", '#[path = "local.rs"] mod local;', 'mod local;')
        self.pair("path-attribute", '#[path = r#"local.rs"#] mod local;', 'mod local;')
        self.pair("package-reach", '#[path = r#"../../other/private.rs"#] mod other;', 'mod other;')

    def test_manifest_reach_all_kinds(self):
        for table in ("dependencies", "dev-dependencies", "build-dependencies", 'target.\'cfg(unix)\'.dev-dependencies'):
            self.write("crates/engine/Cargo.toml", '[package]\nname="engine"\n[' + table + ']\nrenamed = { package="game", path="../../examples/game" }\n')
            self.assertTrue(self.sites("package-reach"), table)
            self.write("crates/engine/Cargo.toml", '[package]\nname="engine"\n[' + table + ']\nrenamed = { package="fixture", path="../fixture" }\n')
            self.assertEqual(self.sites("package-reach"), [])
        self.write("Cargo.toml", '[workspace.dependencies]\ngame={path="examples/game"}\n')
        self.write("crates/engine/Cargo.toml", '[package]\nname="engine"\n[dev-dependencies]\ngame.workspace=true\n')
        self.assertTrue(self.sites("package-reach"))

    def test_library_env(self):
        for call in ("var", "var_os", "vars"):
            argument = '"SETTING"' if call != "vars" else ''
            self.pair("library-env", f'fn settings() {{ std :: env :: {call}({argument}); }}',
                      'fn settings(value: Settings) {}')
        bad = 'fn settings() { std::env::var("SETTING"); }'
        self.pair("library-env", 'fn settings() { std::env::var::<&str>("SETTING"); }',
                  'fn settings(value: Settings) {}')
        for role, exempt in (("support", True), ("fixtures", False), ("other", False)):
            self.write(".config/architecture.toml", f'[packages.engine]\nmanifest="crates/engine/Cargo.toml"\ntest_role="{role}"\n')
            self.write("crates/engine/src/lib.rs", bad)
            self.assertEqual(not self.sites("library-env"), exempt)
        self.write("crates/engine/src/lib.rs", '#[cfg(test)] mod tests { fn config() { std::env::var("TEST"); } }')
        self.assertEqual(self.sites("library-env"), [])
        self.write("crates/engine/src/bin/cli.rs", bad)
        self.assertEqual(self.sites("library-env"), [])
        self.write("crates/engine/tests/config.rs", bad)
        self.assertEqual(self.sites("library-env"), [])
        self.write("crates/engine/build.rs", bad)
        self.assertEqual(self.sites("library-env"), [])

    def test_test_crate_edges(self):
        self.write(".config/architecture.toml", '''
[boundaries]
proving_ground="ground"
[packages.support]
manifest="crates/support/Cargo.toml"
test_role="support"
[packages.fixtures]
manifest="crates/fixtures/Cargo.toml"
test_role="fixtures"
''')
        for consumer in ("engine", "ground", "game"):
            for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
                for name in ("support", "fixtures"):
                    for inherited in (False, True):
                        for target in (False, True):
                            with self.subTest(consumer=consumer, kind=kind, name=name, inherited=inherited, target=target):
                                self.write("Cargo.toml", f'[workspace.dependencies]\nrenamed={{package="{name}",path="crates/{name}"}}\n')
                                table = f"target.'cfg(unix)'.{kind}" if target else kind
                                dep = '{workspace=true}' if inherited else f'{{package="{name}",path="../{name}"}}'
                                self.write("crates/engine/Cargo.toml", f'[package]\nname="{consumer}"\n[{table}]\nrenamed={dep}\n')
                                allowed = kind == "dev-dependencies" or (name == "fixtures" and kind == "dependencies" and consumer == "ground")
                                self.assertEqual(bool(self.sites("test-crate-edges")), not allowed)

    def test_device_request(self):
        for call in ("request_adapter", "request_device"):
            self.pair("device-request", f'fn device() {{ instance . {call}(&options); }}',
                      'fn device() { recipe::request(profile); }')
            self.pair("device-request", f'fn device() {{ wgpu::Instance::{call}(&instance, &options); }}',
                      'fn device() { recipe::request(profile); }')
        self.write("crates/thrum-render/src/capabilities/recipe.rs", 'fn device() { instance.request_device(&options); }')
        self.assertEqual(self.sites("device-request"), [])

    def test_device_request_checks_explicit_benchmark_modules(self):
        bench = "crates/engine/benches"
        self.write(f"{bench}/route.rs", '#[path = "../tests/support/route.rs"] mod route;')
        self.write("crates/engine/tests/support/route.rs", '#[path = "device.rs"] mod device;')
        self.write("crates/engine/tests/support/device.rs", 'fn open() { adapter.request_device(&options); }')
        self.write("crates/engine/tests/unrelated.rs", 'fn test() { adapter.request_device(&options); }')
        errors = shapes.check(self.root, "device-request", [bench])
        self.assertEqual(len(errors), 1)
        self.assertIn("crates/engine/tests/support/device.rs:1: device-request:", errors[0])
        self.write("crates/engine/tests/support/device.rs", 'fn open() { recipe.open(&instance, None); }')
        self.assertEqual(shapes.check(self.root, "device-request", [bench]), [])

    def test_whole_file_gate(self):
        self.pair("whole-file-gpu-gate", '#![cfg(target_os = "macos")]\n#[test] fn cpu() {}',
                  '#[cfg(target_os = "macos")]\n#[test] fn gpu() {}\n#[test] fn cpu() {}',
                  "crates/engine/tests/render.rs")

    def test_history_names_all_surfaces(self):
        self.write("crates/engine/tests/g2s_runtime.rs", "")
        self.assertIn(("crates/engine/tests/g2s_runtime.rs", 1), self.sites("history-names"))
        (self.root / "crates/engine/tests/g2s_runtime.rs").unlink()
        self.pair("history-names", 'fn config() { std::env::var("THRUM_G' + '5_CAPTURE_DIR"); }',
                  'fn config() { std::env::var("THRUM_CAPTURE_DIR"); }')
        for field in ('[[bench]]\nname="g3c_culling"', '[package]\ndescription="A G5 viewer"'):
            self.write("crates/engine/Cargo.toml", '[package]\nname="engine"\n' + field if field.startswith("[[") else field + '\nname="engine"\n')
            self.assertTrue(self.sites("history-names"))
        for bad in ("[package]\nname='engine'\ndescription='A G5 viewer'\n",
                    '[package]\nname="engine"\ndescription="""\nA G5 viewer\n"""\n',
                    "[package]\nname='engine'\n[[test]]\nname='g2_state'\n"):
            self.write("crates/engine/Cargo.toml", bad)
            self.assertTrue(self.sites("history-names"))
        self.write("crates/engine/Cargo.toml", '[package]\nname="engine"\ndescription="An asset viewer"\n[[bench]]\nname="culling"\n')
        self.write("crates/engine/src/lib.rs", 'fn rgba8(vec3: u8) {}')
        self.write("crates/engine/tests/rgba8_vec3.rs", "")
        self.assertEqual(self.sites("history-names"), [])

    def test_pins_are_discovered_and_expire(self):
        self.write("scripts/g5_probe.py", "")
        plan = {"milestones": [{"status": "in_progress", "items": [{"id": "M1.1", "check": "python3 scripts/g5_probe.py"}]}]}
        self.write("docs/plans/OTHER/DONE.json", json.dumps(plan))
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(shapes.check(self.root, "history-names", ["scripts"]), [])
        plan["milestones"][0]["status"] = "done"
        self.write("docs/plans/OTHER/DONE.json", json.dumps(plan))
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertTrue(shapes.check(self.root, "history-names", ["scripts"]))

    def test_missing_zero_and_stale_allowance(self):
        self.assertTrue(shapes.check(self.root, "string-errors", ["missing"]))
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(shapes.check(self.root, tighten=True), [])
        baseline = json.loads((self.root / shapes.BASELINE).read_text())
        baseline["rules"]["string-errors"]["files"]["crates/engine/src/lib.rs"] = 1
        shapes.save_baseline(self.root / shapes.BASELINE, baseline)
        self.assertTrue(shapes.check(self.root))
        # An archive nested inside a checkout has no history of its own. Git's
        # parent discovery must not inventory the outer checkout or ratchet it.
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        self.write("archive/crates/engine/Cargo.toml", '[package]\nname="engine"\n')
        self.write("archive/crates/engine/src/lib.rs", 'pub fn fail() -> Result<(), String> { todo!() }')
        archive = self.root / "archive"
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            self.assertEqual(shapes.check(archive, tighten=True), [])
        self.assertIn("no Git history; tree counts only", output.getvalue())
        self.assertEqual(shapes.inventory(archive)[0]["rules"]["string-errors"]["files"],
                         {"crates/engine/src/lib.rs": 1})


if __name__ == "__main__":
    unittest.main()

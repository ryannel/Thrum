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
        self.pair("path-attribute", 'mod support { include!("local.rs"); }', 'mod support;',
                  "crates/engine/tests/render.rs")

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
        self.pair("whole-file-gpu-gate", '#[cfg(all(test, target_os = "macos"))]\npub(super) mod tests;',
                  '#[cfg(test)]\npub(super) mod tests;')
        self.pair("whole-file-gpu-gate", '#[cfg(windows)]\n#[cfg(test)]\nmod tests;',
                  '#[cfg(test)]\nmod tests;')
        self.write("crates/engine/tests/render/metal.rs", '#[test] fn gpu() {}')
        self.pair("whole-file-gpu-gate", '#[cfg(target_os = "macos")]\nmod metal;',
                  'mod metal;', "crates/engine/tests/render.rs")
        self.pair("whole-file-gpu-gate", '#[cfg(target_os = "macos")] mod metal { #[test] fn gpu() {} }',
                  '#[cfg(target_os = "macos")] mod helpers { fn prepare() {} }\n#[test] fn gpu() {}',
                  "crates/engine/tests/render.rs")

    def test_os_condition_on_a_product_module_with_tests(self):
        # The condition sits on a product module whose declared subtree holds a CPU test.
        self.write("crates/engine/src/watch.rs", "#[cfg(test)]\nmod tests;")
        self.write("crates/engine/src/watch/tests.rs", "#[test] fn cpu() {}")
        self.pair("whole-file-gpu-gate", '#[cfg(target_os = "macos")]\nmod watch;',
                  '#[cfg(any(test, target_os = "macos"))]\nmod watch;')
        self.pair("whole-file-gpu-gate", '#[cfg(any(target_os = "macos", all(test, unix)))]\nmod watch;',
                  '#[cfg(any(target_os = "macos", all(test, not(target_arch = "wasm32"))))]\nmod watch;')
        self.write("crates/engine/src/shell.rs", "mod inner;")
        self.write("crates/engine/src/shell/inner.rs", "#[tokio::test] async fn cpu() {}")
        self.pair("whole-file-gpu-gate", '#[cfg(target_vendor = "apple")]\npub mod shell;',
                  '// platform-only tests: they drive the AppKit shell\n#[cfg(target_vendor = "apple")]\npub mod shell;')
        self.write("crates/engine/src/lib.rs", '//platform-only tests:\n#[cfg(windows)]\nmod shell;')
        self.assertTrue(self.sites("whole-file-gpu-gate"))
        self.pair("whole-file-gpu-gate", '#![cfg(unix)]\n#[cfg(test)]\nmod tests;', "#[cfg(test)]\nmod tests;",
                  "crates/engine/src/watch.rs")
        # A Cargo target that only shares a directory is not part of the module.
        self.write("crates/engine/tests/support.rs", "pub fn device() {}")
        self.write("crates/engine/tests/support/field_gpu.rs", "#[test] fn gpu() {}")
        self.write("crates/engine/src/lib.rs", "")
        self.write("crates/engine/tests/clouds.rs", '#[cfg(target_os = "macos")]\npub mod support;')
        self.assertEqual(self.sites("whole-file-gpu-gate"), [])

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

    # The evidence folder's name is built in parts so this suite does not name it.
    EVIDENCE = "docs/" + "evidence"

    def counted(self, rule):
        return sorted({path for path, _ in self.sites(rule)})

    def test_evidence_path(self):
        self.pair("evidence-path", f'const RECORD: &str = "../../{self.EVIDENCE}/CLOUDS/cost.json";',
                  'const RECORD: &str = "tests/fixtures/clouds/cost.json";')
        self.pair("evidence-path", f"RECORD = ROOT / '{self.EVIDENCE}/WATERS/M2-cost.json'\n",
                  "RECORD = ROOT / '.orchestrate/evidence/WATERS/M2-cost.json'\n", "scripts/cost_record.py")
        # The evidence store restores history, and data files are not source.
        self.write("scripts/evidence_store.py", f'MANIFEST = "{self.EVIDENCE}/BULK.json"\n')
        self.write("scripts/test_evidence_store.py", f'NAME = "{self.EVIDENCE}/example/large.mp4"\n')
        self.write("examples/game/tests/fixtures/controls.json", f'["{self.EVIDENCE}/render/a.png"]')
        self.write(f"{self.EVIDENCE}/notes.py", f'"{self.EVIDENCE}/x"')
        self.assertEqual(self.sites("evidence-path"), [])

    def script_plan(self, check, status):
        plan = {"milestones": [{"id": "M1", "status": status, "items": [{"id": "M1.1", "check": check}]}]}
        self.write("docs/plans/OTHER/DONE.json", json.dumps(plan))

    def test_script_caller_refuses_a_script_nothing_names(self):
        self.write("scripts/orphan_tool.py", "print('orphan')\n")
        with contextlib.redirect_stdout(io.StringIO()):
            errors = shapes.check(self.root, "script-caller", ["scripts"])
        self.assertIn("scripts/orphan_tool.py:1: script-caller: name the script from what runs it", "\n".join(errors))
        self.assertIn("patterns.md#scripts-and-benches", "\n".join(errors))
        # Each caller the design names keeps it out of the count.
        callers = {
            "dev": "python3 scripts/orphan_tool.py\n",
            "tools/xtask/src/main.rs": 'python(&["scripts/orphan_tool.py"])?;',
            ".github/workflows/ci.yml": "run: python3 scripts/orphan_tool.py\n",
            "crates/engine/tests/route.rs": 'const TOOL: &str = "../../scripts/orphan_tool.py";',
            "ORCHESTRATE.md": "- **Main-branch check tool:** `python3 scripts/orphan_tool.py`; every 1h\n",
            "docs/TOOLS.md": "Run `python3 scripts/orphan_tool.py` to rebuild the table.\n",
        }
        for caller, text in callers.items():
            with self.subTest(caller=caller):
                self.write(caller, text)
                self.assertEqual(self.counted("script-caller"), [])
                (self.root / caller).unlink()
        for status in ("in_progress", "done"):
            with self.subTest(status=status):
                self.script_plan("python3 scripts/orphan_tool.py --check", status)
                self.assertEqual(self.counted("script-caller"), [])
        (self.root / "docs/plans/OTHER/DONE.json").unlink()
        # Plans prose and records are not callers.
        for record in ("docs/plans/OTHER/PLAN.md", f"{self.EVIDENCE}/OTHER/M1.md"):
            self.write(record, "Ran `python3 scripts/orphan_tool.py`.\n")
        self.assertEqual(self.counted("script-caller"), ["scripts/orphan_tool.py"])

    def test_script_caller_follows_scripts_and_suites(self):
        self.write("dev", "python3 scripts/kept_entry.py\n")
        self.write("scripts/kept_entry.py", "import kept_helper\nfrom kept_pkg.inner import thing\n")
        self.write("scripts/kept_helper.py", "subprocess.run(['node', 'scripts/kept_page.mjs'])\n")
        self.write("scripts/kept_page.mjs", "")
        self.write("scripts/kept_pkg/inner.py", "")
        self.write("scripts/test_kept_entry.py", "import kept_entry\n")
        # A suite counts with its script; one named for no script is not counted.
        self.write("scripts/stray_tool.py", "import stray_helper\n")
        self.write("scripts/stray_helper.py", "")
        self.write("scripts/test_stray_tool.py", "import stray_tool\n")
        self.write("scripts/test_shared_fixtures.py", "")
        self.assertEqual(self.counted("script-caller"),
                         ["scripts/stray_helper.py", "scripts/stray_tool.py", "scripts/test_stray_tool.py"])
        # ./dev check runs every suite directly under scripts/, so a suite named for no script
        # keeps what it imports; one in a subfolder does not.
        self.write("scripts/suite_helper.py", "")
        self.write("scripts/test_suite_only.py", "import suite_helper\n")
        self.write("scripts/nested/nested_helper.py", "")
        self.write("scripts/nested/test_nested_only.py", "import nested_helper\n")
        self.assertEqual(self.counted("script-caller"),
                         ["scripts/nested/nested_helper.py", "scripts/stray_helper.py",
                          "scripts/stray_tool.py", "scripts/test_stray_tool.py"])
        # A suite the check excludes by name runs nowhere, so it keeps nothing.
        self.write("tools/xtask/src/main.rs",
                   'const EXCLUDED_SCRIPT_SUITES: &[(&str, &str)] = &[("test_suite_only", "fails")];\n')
        self.assertIn("scripts/suite_helper.py", self.counted("script-caller"))
        (self.root / "tools/xtask/src/main.rs").unlink()
        for path in ("scripts/suite_helper.py", "scripts/test_suite_only.py",
                     "scripts/nested/nested_helper.py", "scripts/nested/test_nested_only.py"):
            (self.root / path).unlink()
        # Removing the last caller orphans the chain behind it.
        self.write("dev", "")
        self.assertEqual(len(self.counted("script-caller")), 8)

    def bench_manifest(self):
        self.write("crates/engine/Cargo.toml", '[package]\nname="engine"\n[[bench]]\nname="frame"\nharness=false\n'
                   '[[bench]]\nname="valley_culling"\nharness=false\n')

    def test_bench_runner(self):
        self.bench_manifest()
        with contextlib.redirect_stdout(io.StringIO()):
            errors = shapes.check(self.root, "bench-runner", ["crates/engine/Cargo.toml"])
        self.assertIn("crates/engine/Cargo.toml:4: bench-runner: name the bench from what runs it", "\n".join(errors))
        self.assertIn("crates/engine/Cargo.toml:7: bench-runner:", "\n".join(errors))
        # A plain word counts as a name only beside the word bench; a compound name anywhere.
        self.write("scripts/frame_report.py", "frame = 1\nprint('valley_culling done')\n")
        self.assertEqual(self.sites("bench-runner"), [("crates/engine/Cargo.toml", 4)])
        self.write("scripts/frame_report.py", "run(['cargo', 'bench',\n     '--bench', 'frame'])\n")
        self.assertEqual(self.sites("bench-runner"), [("crates/engine/Cargo.toml", 7)])
        for runner in ("dev", "tools/xtask/src/main.rs", ".github/workflows/ci.yml", "docs/TIMING.md"):
            with self.subTest(runner=runner):
                self.write(runner, "cargo bench -p engine --bench valley_culling\n")
                self.assertEqual(self.sites("bench-runner"), [])
                (self.root / runner).unlink()
        for record in ("docs/plans/OTHER/PLAN.md", f"{self.EVIDENCE}/OTHER/M1.md"):
            self.write(record, "cargo bench -p engine --bench valley_culling\n")
        self.assertEqual(self.sites("bench-runner"), [("crates/engine/Cargo.toml", 7)])

    def test_shader_text_edit(self):
        bad = 'const SHADER: &str = include_str!("lit.wgsl");\nfn variant() -> String { SHADER.replace("LIGHTS", "4") }'
        good = 'const SHADER: &str = include_str!("lit.wgsl");\nfn variant() -> String { format!("const LIGHTS = 4u;\\n{SHADER}") }'
        self.write("crates/engine/src/lit.rs", bad)
        with contextlib.redirect_stdout(io.StringIO()):
            errors = shapes.check(self.root, "shader-text-edit", ["crates/engine/src/lit.rs"])
        self.assertIn("crates/engine/src/lit.rs:2: shader-text-edit: make the variant a separate entry point", "\n".join(errors))
        self.write("crates/engine/src/lit.rs", good)
        self.assertEqual(self.sites("shader-text-edit"), [])
        self.write("crates/engine/src/lit.rs", "// WGSL\nfn edit(s: &str) -> String { s.replacen(\n 'a', \"b\", 1) }")
        self.assertEqual(len(self.sites("shader-text-edit")), 1)
        # Coarse but bounded: test files, test modules, strings and files without WGSL are left out.
        self.write("crates/engine/src/lit.rs", "fn edit(s: &str) -> String { s.replace('a', \"b\") }")
        self.write("crates/engine/tests/lit.rs", bad)
        self.write("crates/engine/src/doc.rs", 'const WGSL: &str = "x.replace(a)";\n#[cfg(test)]\nmod tests { fn t(s: &str) { s.replace("a", "b"); } }')
        self.assertEqual(self.sites("shader-text-edit"), [])

    KIT = '[packages.support]\nmanifest="crates/support/Cargo.toml"\ntest_role="support"\n'

    def frozen(self, rule, bad, more, path):
        """One more site than the committed count fails the check, which names the kit and the patterns."""
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        self.write(path, bad)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(shapes.check(self.root, tighten=True), [])
        git = ["git", "-C", str(self.root), "-c", "user.name=t", "-c", "user.email=t@t"]
        subprocess.run([*git, "add", "-A"], check=True)
        subprocess.run([*git, "commit", "-qm", "baseline"], check=True)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(shapes.check(self.root, rule), [])
        self.write(path, bad + "\n" + more)
        with contextlib.redirect_stdout(io.StringIO()):
            errors = "\n".join(shapes.check(self.root, rule))
            refused = shapes.check(self.root, tighten=True)
        self.assertIn(f"{path}:", errors)
        self.assertIn(f"{rule}: ", errors)
        self.assertIn("thrum_test_support::", errors)
        self.assertIn("patterns.md#tests", errors)
        self.assertTrue(refused, "tighten cannot grant the new site")

    READBACK = 'fn read(b: &wgpu::Buffer) { b.slice(..).map_async(wgpu::MapMode::Read, |_| {}); }'

    def test_test_readback(self):
        good = 'fn read() -> Vec<u8> { thrum_test_support::readback::buffer(&device, &queue, &source, ..) }'
        self.pair("test-readback", self.READBACK, good, "crates/engine/tests/gpu.rs")
        self.pair("test-readback", 'fn read() { encoder.map_buffer_on_submit(\n        &staging,\n        MapMode::Read,\n        ..,\n        |_| {},\n    );\n}',
                  'fn read() { let bytes = readback.wait(&device, queue.submit([encoder.finish()])); }',
                  "crates/engine/src/visibility/tests.rs")
        # Writing, comments and strings are not a readback.
        self.write("crates/engine/tests/gpu.rs", 'fn write(b: &wgpu::Buffer) { b.map_async(wgpu::MapMode::Write, .., |_| {}); }\n'
                   '// b.map_async(wgpu::MapMode::Read, .., |_| {});\nconst S: &str = "x.map_async(MapMode::Read, ..)";')
        self.assertEqual(self.sites("test-readback"), [])
        # The same code outside a test file, and in the kit itself, is not counted.
        self.write("crates/engine/tests/gpu.rs", "")
        self.write(".config/architecture.toml", self.KIT)
        for path in ("crates/engine/src/readback.rs", "crates/support/src/readback.rs",
                     "crates/support/src/readback/tests.rs", "crates/support/tests/readback.rs"):
            self.write(path, self.READBACK)
        self.assertEqual(self.sites("test-readback"), [])
        self.write("crates/support/src/readback.rs", "")
        self.write(".config/architecture.toml", "")
        self.assertEqual(len(self.sites("test-readback")), 2)

    def test_test_readback_is_frozen(self):
        self.frozen("test-readback", self.READBACK,
                    'fn again(b: &wgpu::Buffer) { b.map_async(wgpu::MapMode::Read, .., |_| {}); }',
                    "crates/engine/tests/gpu.rs")

    DEADLINE = 'fn settle() { loop { assert!(Instant::now() < deadline, "never settled"); } }'

    def test_test_deadline(self):
        good = 'fn settle() { thrum_test_support::wait::until("settled", LIMIT, || done().then_some(())); }'
        self.pair("test-deadline", self.DEADLINE, good, "crates/engine/tests/clock.rs")
        self.pair("test-deadline", 'fn settle() { assert!(\n        std::time::Instant::now() <= deadline\n    );\n}', good,
                  "crates/engine/src/sim/tests.rs")
        # A timing bound on elapsed time, a comment and a string are not a hand-written deadline wait.
        self.write("crates/engine/tests/clock.rs", 'fn bound() { assert!(start.elapsed() < LIMIT); }\n'
                   '// assert!(Instant::now() < deadline);\nconst S: &str = "assert!(Instant::now() < deadline)";')
        self.assertEqual(self.sites("test-deadline"), [])
        self.write("crates/engine/tests/clock.rs", "")
        self.write(".config/architecture.toml", self.KIT)
        for path in ("crates/engine/src/clock.rs", "crates/support/src/wait.rs", "crates/support/src/tests.rs"):
            self.write(path, self.DEADLINE)
        self.assertEqual(self.sites("test-deadline"), [])

    def test_test_deadline_is_frozen(self):
        self.frozen("test-deadline", self.DEADLINE,
                    'fn again() { assert!(Instant::now() < deadline); }', "crates/engine/tests/clock.rs")


if __name__ == "__main__":
    unittest.main()

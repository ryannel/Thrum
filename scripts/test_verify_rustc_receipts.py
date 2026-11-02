"""Complete and refusal fixtures for candidate compiler invocation receipts."""

import json
import os
from pathlib import Path
import tempfile
import unittest

from verify_rustc_receipts import inspect, parse_receipt


def field(value):
    return len(value).to_bytes(4, "little") + value


def receipt(rustc, cwd, arguments, environment=(), kind=1, completed=True):
    result = bytearray(b"THRMRCP1")
    result += (1).to_bytes(4, "little") + bytes([kind])
    result += field(os.fsencode(rustc)) + field(os.fsencode(cwd))
    result += len(arguments).to_bytes(4, "little")
    for argument in arguments:
        result += field(argument)
    result += len(environment).to_bytes(4, "little")
    for name, value in environment:
        result += field(name) + field(value)
    result += b"PEND"
    if completed:
        result += b"DONE\0" + (0).to_bytes(4, "little", signed=True)
    return bytes(result)


@unittest.skipUnless(os.name == "posix", "receipt byte paths require POSIX")
class VerifyRustcReceipts(unittest.TestCase):
    def fixture(self, root):
        source = root / "source"
        (source / "src").mkdir(parents=True)
        (source / "src/main.rs").write_text("fn main() {}\n")
        cache = root / "cache"
        cache.mkdir()
        output = root / "output"
        (output / "debug/deps").mkdir(parents=True)
        artifact = output / "debug/headless"
        artifact.write_bytes(b"binary")
        directory = output / "rustc-receipts"
        directory.mkdir()
        (output / "rustc-receipt-wrapper").write_bytes(b"wrapper")
        rustc = root / "rustc"
        rustc.write_bytes(b"compiler")
        environment = sorted((
            (b"CARGO_HOME", os.fsencode(cache)),
            (b"CARGO_INCREMENTAL", b"0"),
            (b"CARGO_NET_OFFLINE", b"true"),
            (b"CARGO_TARGET_DIR", os.fsencode(output)),
            (b"RUSTC", os.fsencode(rustc)),
            (b"RUSTC_WRAPPER", os.fsencode(output / "rustc-receipt-wrapper")),
        ))
        arguments = [
            b"--crate-name", b"headless", b"src/main.rs", b"--emit=dep-info,link",
            b"--out-dir", os.fsencode(output / "debug/deps"),
        ]
        (directory / "101.receipt").write_bytes(receipt(
            rustc, source, [b"-vV"], environment, kind=0,
        ))
        compile_path = directory / "102.receipt"
        compile_path.write_bytes(receipt(rustc, source, arguments, environment))
        transcript = root / "cargo.jsonl"
        transcript.write_text("\n".join(json.dumps(value, separators=(",", ":")) for value in (
            {"reason": "compiler-artifact", "fresh": False,
             "target": {"name": "headless", "src_path": str(source / "src/main.rs"),
                        "kind": ["bin"]}, "filenames": [str(artifact)]},
            {"reason": "build-finished", "success": True},
        )) + "\n")
        return directory, rustc, source, cache, output, transcript, compile_path, arguments, environment

    def test_complete_receipts_match_cargo_artifacts(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve())
            report = inspect(*args[:6])
            self.assertEqual((report["probes"], report["compiles"]), (1, 1))
            self.assertEqual(report["units"][0]["artifacts"], ["debug/headless"])
            self.assertFalse(report["buildReady"])

    def test_incomplete_wrong_source_and_extra_unit_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve())
            receipts, rustc, source, cache, output, transcript, path, arguments, environment = args
            path.write_bytes(receipt(rustc, source, arguments, environment, completed=False))
            with self.assertRaisesRegex(ValueError, "completed execution"):
                inspect(*args[:6])
            outside = Path(directory).resolve() / "outside.rs"
            outside.write_text("fn main() {}\n")
            escaped = [outside.as_posix().encode() if item == b"src/main.rs" else item
                       for item in arguments]
            path.write_bytes(receipt(rustc, source, escaped, environment))
            with self.assertRaisesRegex(ValueError, "escapes selected roots"):
                inspect(*args[:6])
            path.write_bytes(receipt(rustc, source, arguments, environment))
            (receipts / "103.receipt").write_bytes(receipt(rustc, source, arguments, environment))
            with self.assertRaisesRegex(ValueError, "duplicate crate-name"):
                inspect(*args[:6])

    def test_cargo_target_source_must_match_receipt_within_selected_root(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve())
            receipts, rustc, source, cache, output, transcript, path, arguments, environment = args
            other = source / "src/other.rs"
            other.write_text("fn main() {}\n")
            changed = [b"src/other.rs" if item == b"src/main.rs" else item for item in arguments]
            path.write_bytes(receipt(rustc, source, changed, environment))
            with self.assertRaisesRegex(ValueError, "target source differs"):
                inspect(*args[:6])

    def test_rustc_extern_must_be_an_artifact_of_the_named_cargo_unit(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve())
            receipts, rustc, source, cache, output, transcript, path, arguments, environment = args
            undeclared = output / "debug/deps/libdep.rlib"
            undeclared.write_bytes(b"library")
            path.write_bytes(receipt(rustc, source, arguments + [
                b"--extern", b"dep=" + os.fsencode(undeclared),
            ], environment))
            with self.assertRaisesRegex(ValueError, "Cargo dependency artifact"):
                inspect(*args[:6])

            dependency_source = source / "src/dep.rs"
            dependency_source.write_text("pub fn value() {}\n")
            (receipts / "100.receipt").write_bytes(receipt(rustc, source, [
                b"--crate-name", b"dep", b"src/dep.rs", b"--emit=dep-info,link",
                b"--out-dir", os.fsencode(output / "debug/deps"),
            ], environment))
            entries = [json.loads(line) for line in transcript.read_text().splitlines()]
            entries.insert(0, {
                "reason": "compiler-artifact", "fresh": False,
                "target": {"name": "dep", "src_path": str(dependency_source), "kind": ["lib"]},
                "filenames": [str(undeclared)],
            })
            transcript.write_text("\n".join(json.dumps(entry) for entry in entries) + "\n")
            self.assertEqual(inspect(*args[:6])["compiles"], 2)

            path.write_bytes(receipt(rustc, source, arguments + [
                b"--extern", b"wrong_name=" + os.fsencode(undeclared),
            ], environment))
            with self.assertRaisesRegex(ValueError, "Cargo dependency artifact"):
                inspect(*args[:6])

    def test_wrong_compiler_control_environment_and_artifact_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve())
            receipts, rustc, source, cache, output, transcript, path, arguments, environment = args
            other = rustc.with_name("other-rustc")
            path.write_bytes(receipt(other, source, arguments, environment))
            with self.assertRaisesRegex(ValueError, "another compiler"):
                inspect(*args[:6])
            path.write_bytes(receipt(rustc, source, arguments, environment + [
                (b"THRUM_SELECTED_RUSTC", os.fsencode(rustc)),
            ]))
            with self.assertRaisesRegex(ValueError, "control variables"):
                inspect(*args[:6])
            path.write_bytes(receipt(rustc, source, arguments, environment))
            outside = Path(directory).resolve() / "outside-binary"
            outside.write_bytes(b"binary")
            record = json.loads(transcript.read_text().splitlines()[0])
            record["filenames"] = [str(outside)]
            transcript.write_text(json.dumps(record) + "\n" +
                                  json.dumps({"reason": "build-finished", "success": True}) + "\n")
            with self.assertRaisesRegex(ValueError, "escapes selected roots"):
                inspect(*args[:6])

    def test_receipt_parser_refuses_trailing_and_unordered_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            rustc = Path(directory) / "rustc"
            cwd = Path(directory)
            raw = receipt(rustc, cwd, [b"-vV"], [(b"B", b"1"), (b"A", b"2")], kind=0)
            with self.assertRaisesRegex(ValueError, "unordered"):
                parse_receipt(raw)
            with self.assertRaisesRegex(ValueError, "trailing"):
                parse_receipt(receipt(rustc, cwd, [b"-vV"], kind=0) + b"junk")

    def test_fixed_environment_and_library_search_escape_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            args = self.fixture(Path(directory).resolve())
            receipts, rustc, source, cache, output, transcript, path, arguments, environment = args
            changed = [(name, b"/other" if name == b"CARGO_HOME" else value)
                       for name, value in environment]
            path.write_bytes(receipt(rustc, source, arguments, changed))
            with self.assertRaisesRegex(ValueError, "environment differs"):
                inspect(*args[:6])
            path.write_bytes(receipt(rustc, source, arguments + [
                b"-L", b"dependency=/outside/deps",
            ], environment))
            with self.assertRaisesRegex(ValueError, "library path differs"):
                inspect(*args[:6])
            path.write_bytes(receipt(rustc, source, arguments + [
                b"--crate-type", b"proc-macro",
            ], environment))
            with self.assertRaisesRegex(ValueError, "refuses proc macros"):
                inspect(*args[:6])


if __name__ == "__main__":
    unittest.main()

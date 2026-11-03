"""Reconcile bounded rustc invocation receipts with one fresh Cargo build.

This is a first macOS headless compiler candidate. It records actual arguments,
environment and selected artifacts; it cannot attest every compiler file read
or make a BuildRevision ready.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat


MAGIC = b"THRMRCP1"
MAX_RECEIPT = 1024 * 1024
MAX_TRANSCRIPT = 4 * 1024 * 1024
MAX_RECEIPTS = 64
NAME = re.compile(r"[0-9]+\.receipt\Z")
PROBE_QUERY = (
    b"-", b"--crate-name", b"___", b"--print=file-names",
    b"--crate-type", b"bin", b"--crate-type", b"rlib",
    b"--crate-type", b"dylib", b"--crate-type", b"cdylib",
    b"--crate-type", b"staticlib", b"--crate-type", b"proc-macro",
    b"--print=sysroot", b"--print=split-debuginfo", b"--print=crate-name",
    b"--print=cfg", b"-Wwarnings",
)


def ordinary_bytes(path, maximum):
    if not path.is_absolute():
        raise ValueError(f"receipt input must be absolute: {path}")
    before = path.lstat()
    if not stat.S_ISREG(before.st_mode) or before.st_size > maximum:
        raise ValueError(f"receipt input is aliased, irregular or too large: {path}")
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), "rb") as stream:
        opened = os.fstat(stream.fileno())
        if (before.st_dev, before.st_ino, before.st_size) != (
            opened.st_dev, opened.st_ino, opened.st_size,
        ):
            raise ValueError(f"receipt input changed before read: {path}")
        raw = stream.read(maximum + 1)
        if len(raw) > maximum or opened.st_size != os.fstat(stream.fileno()).st_size:
            raise ValueError(f"receipt input changed during read: {path}")
    after = path.lstat()
    if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) != (
        after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns,
    ):
        raise ValueError(f"receipt input changed after read: {path}")
    return raw


class Reader:
    def __init__(self, raw):
        self.raw = raw
        self.offset = 0

    def take(self, length):
        if length > len(self.raw) - self.offset:
            raise ValueError("truncated rustc receipt")
        result = self.raw[self.offset:self.offset + length]
        self.offset += length
        return result

    def u32(self):
        return int.from_bytes(self.take(4), "little")

    def field(self):
        length = self.u32()
        if length > 128 * 1024:
            raise ValueError("rustc receipt field is oversized")
        return self.take(length)


def parse_receipt(raw, allow_pending=False):
    if len(raw) > MAX_RECEIPT:
        raise ValueError("rustc receipt is oversized")
    reader = Reader(raw)
    if reader.take(8) != MAGIC or reader.u32() != 1:
        raise ValueError("unsupported rustc receipt format")
    kind = reader.take(1)[0]
    if kind not in (0, 1):
        raise ValueError("unsupported rustc receipt kind")
    rustc = reader.field()
    cwd = reader.field()
    argument_count = reader.u32()
    if argument_count > 1024:
        raise ValueError("too many rustc receipt arguments")
    arguments = [reader.field() for _ in range(argument_count)]
    environment_count = reader.u32()
    if environment_count > 512:
        raise ValueError("too many rustc receipt environment variables")
    environment = {}
    previous = b""
    for _ in range(environment_count):
        name = reader.field()
        value = reader.field()
        if not name or name <= previous or b"=" in name or b"\0" in name + value:
            raise ValueError("rustc receipt environment is malformed or unordered")
        previous = name
        environment[name] = value
    if reader.take(4) != b"PEND":
        raise ValueError("rustc receipt lacks completed execution")
    if allow_pending and reader.offset == len(raw):
        return {
            "kind": kind, "rustc": rustc, "cwd": cwd, "arguments": arguments,
            "environment": environment, "pending": True,
            "sha256": hashlib.sha256(raw).hexdigest(),
        }
    if len(raw) - reader.offset < 9 or reader.take(4) != b"DONE":
        raise ValueError("rustc receipt lacks completed execution")
    outcome = reader.take(1)[0]
    status = int.from_bytes(reader.take(4), "little", signed=True)
    if reader.offset != len(raw) or outcome not in range(5):
        raise ValueError("rustc receipt has trailing or invalid completion data")
    return {
        "kind": kind, "rustc": rustc, "cwd": cwd, "arguments": arguments,
        "environment": environment, "outcome": outcome, "status": status,
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def selected_file(path, roots, label):
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise ValueError(f"{label} is not an ordinary selected file: {path}")
    selected = path.resolve(strict=True)
    for name, root in roots.items():
        if selected.is_relative_to(root):
            return name, selected.relative_to(root).as_posix()
    raise ValueError(f"{label} escapes selected roots: {path}")


def selected_directory(path, root, label):
    if (not path.is_absolute() or path.is_symlink() or not path.is_dir()
            or path.resolve(strict=True) != root / "debug/deps"):
        raise ValueError(f"{label} differs from selected dependency output")


def flag_value(arguments, name):
    matches = []
    for index, argument in enumerate(arguments):
        if argument == name:
            if index + 1 == len(arguments):
                raise ValueError(f"rustc flag lacks value: {name!r}")
            matches.append(arguments[index + 1])
        elif argument.startswith(name + b"="):
            matches.append(argument[len(name) + 1:])
    if len(matches) != 1 or not matches[0]:
        raise ValueError(f"rustc requires exactly one {name!r} flag")
    return matches[0]


def flag_values(arguments, name):
    values = []
    for index, argument in enumerate(arguments):
        if argument == name:
            if index + 1 == len(arguments):
                raise ValueError(f"rustc flag lacks value: {name!r}")
            values.append(arguments[index + 1])
        elif argument.startswith(name + b"="):
            values.append(argument[len(name) + 1:])
        elif name == b"-L" and argument.startswith(b"-L") and len(argument) > 2:
            values.append(argument[2:])
    return values


def compile_identity(receipt, roots):
    arguments = receipt["arguments"]
    validate_first_profile_codegen(arguments, receipt["environment"])
    emit = flag_value(arguments, b"--emit")
    if emit not in (b"dep-info,metadata,link", b"dep-info,link"):
        raise ValueError("first profile refuses unexpected rustc emission")
    crate_types = flag_values(arguments, b"--crate-type")
    if (not crate_types or len(crate_types) != len(set(crate_types))
            or any(value not in (b"lib", b"bin", b"rlib", b"staticlib", b"cdylib")
                   for value in crate_types)):
        raise ValueError("first headless profile refuses proc macros or unsupported crate types")
    crate = flag_value(arguments, b"--crate-name").decode("ascii")
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", crate):
        raise ValueError("invalid compiled crate name")
    cwd = Path(os.fsdecode(receipt["cwd"]))
    if not cwd.is_absolute() or not cwd.is_dir() or cwd.is_symlink():
        raise ValueError("compiler working directory is not selected")
    cwd = cwd.resolve(strict=True)
    if not any(cwd.is_relative_to(root) for root in roots.values()):
        raise ValueError("compiler working directory escapes selected roots")
    out_dir = Path(os.fsdecode(flag_value(arguments, b"--out-dir")))
    if not out_dir.is_absolute() or out_dir.resolve(strict=True) != roots["output"] / "debug/deps":
        raise ValueError("compiler output directory differs from selected output")
    sources = []
    for argument in arguments:
        if argument.endswith(b".rs") and not argument.startswith(b"-"):
            path = Path(os.fsdecode(argument))
            sources.append(selected_file(path if path.is_absolute() else cwd / path,
                                         {key: roots[key] for key in ("source", "cache")},
                                         "rustc source"))
    if len(sources) != 1:
        raise ValueError("compile receipt needs one selected source entry")
    externs = []
    for index, argument in enumerate(arguments):
        if argument == b"--extern":
            if index + 1 == len(arguments):
                raise ValueError("rustc extern flag lacks value")
            extern = arguments[index + 1]
        elif argument.startswith(b"--extern="):
            extern = argument[len(b"--extern="):]
        else:
            continue
        name, separator, path = extern.partition(b"=")
        if not separator or not re.fullmatch(rb"[A-Za-z_][A-Za-z0-9_]*", name):
            raise ValueError("rustc extern lacks selected artifact path")
        _, relative = selected_file(Path(os.fsdecode(path)),
                                    {"output": roots["output"]}, "rustc extern")
        externs.append((name.decode("ascii"), relative))
    if len({name for name, _ in externs}) != len(externs):
        raise ValueError("first profile refuses duplicate rustc extern names")
    for value in flag_values(arguments, b"-L"):
        kind, separator, path = value.partition(b"=")
        if not separator or kind != b"dependency":
            raise ValueError("first profile refuses other rustc library search paths")
        selected_directory(Path(os.fsdecode(path)), roots["output"], "rustc library path")
    return crate, sources[0], sorted(externs), [value.decode("ascii") for value in crate_types]


def compile_manifest(receipt, roots, source_identity):
    environment = receipt["environment"]
    manifest_path = environment.get(b"CARGO_MANIFEST_PATH")
    manifest_dir = environment.get(b"CARGO_MANIFEST_DIR")
    if not manifest_path or not manifest_dir:
        raise ValueError("rustc child lacks Cargo manifest environment")
    selected = {key: roots[key] for key in ("source", "cache")}
    manifest = selected_file(Path(os.fsdecode(manifest_path)), selected,
                             "rustc child Cargo manifest")
    directory = Path(os.fsdecode(manifest_dir))
    if (not directory.is_absolute() or directory.is_symlink() or not directory.is_dir()
            or directory.resolve(strict=True) != (roots[manifest[0]] / manifest[1]).parent):
        raise ValueError("rustc child manifest differs from selected package directory")
    source_path = roots[source_identity[0]] / source_identity[1]
    if source_identity[0] != manifest[0] or not source_path.is_relative_to(directory.resolve(strict=True)):
        raise ValueError("rustc child source is outside its package")
    return manifest


def selected_xcode_linker(environment):
    sdk_bytes = environment.get(b"SDKROOT")
    linker_bytes = environment.get(b"CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER")
    if not sdk_bytes or not linker_bytes:
        raise ValueError("first profile lacks selected macOS SDK or linker")
    sdk = Path(os.fsdecode(sdk_bytes))
    linker = Path(os.fsdecode(linker_bytes))
    if (not sdk.is_absolute() or sdk.is_symlink() or not sdk.is_dir()
            or not linker.is_absolute() or linker.is_symlink() or not linker.is_file()):
        raise ValueError("selected macOS SDK or linker is not ordinary and absolute")
    developers = [parent for parent in sdk.parents
                  if parent.name == "Developer" and parent.parent.name == "Contents"]
    if len(developers) != 1:
        raise ValueError("selected SDK lacks one Xcode developer root")
    expected = developers[0] / "Toolchains/XcodeDefault.xctoolchain/usr/bin/clang"
    if linker.resolve(strict=True) != expected.resolve(strict=True):
        raise ValueError("selected linker differs from Xcode default Clang")
    return linker_bytes


def validate_first_profile_codegen(arguments, environment):
    values = []
    for index, argument in enumerate(arguments):
        if (argument in (b"-O", b"-g", b"--codegen", b"--sysroot", b"--target")
                or argument.startswith((b"--codegen=", b"--sysroot=", b"--target=", b"-Z"))
                or argument == b"--print" or argument.startswith(b"--print=")):
            raise ValueError("first profile refuses alternate compiler control flags")
        if argument == b"-C":
            if index + 1 == len(arguments):
                raise ValueError("rustc codegen flag lacks value")
            values.append(arguments[index + 1])
        elif argument.startswith(b"-C") and len(argument) > 2:
            raise ValueError("first profile refuses joined rustc codegen flags")
    if len(values) != len(set(values)):
        raise ValueError("first profile refuses duplicate rustc codegen flags")
    linker = selected_xcode_linker(environment)
    required = {b"embed-bitcode=no", b"debuginfo=2", b"split-debuginfo=unpacked",
                b"linker=" + linker}
    if not required.issubset(values):
        raise ValueError("first profile lacks selected rustc codegen flags")
    for value in values:
        if value in required:
            continue
        if re.fullmatch(rb"metadata=[0-9a-f]{16}", value):
            continue
        if re.fullmatch(rb"extra-filename=-[0-9a-f]{16}", value):
            continue
        raise ValueError("first profile refuses unsupported rustc codegen flag")
    if sum(value.startswith(b"metadata=") for value in values) != 1:
        raise ValueError("first profile requires one rustc metadata codegen flag")
    if sum(value.startswith(b"extra-filename=") for value in values) > 1:
        raise ValueError("first profile refuses multiple rustc output suffixes")


def strict_pairs(pairs):
    result = {}
    for name, value in pairs:
        if name in result:
            raise ValueError(f"duplicate Cargo JSON field: {name}")
        result[name] = value
    return result


def validate_macos_dynamic_path(environment, rustc, cache, output):
    home = environment.get(b"HOME")
    if home != os.fsencode(cache.parent):
        raise ValueError("rustc HOME differs from selected private cache parent")
    search = environment.get(b"DYLD_FALLBACK_LIBRARY_PATH")
    if not search:
        raise ValueError("Cargo macOS dynamic library path is missing")
    entries = search.split(b":")
    allowed = {
        os.fsencode(output / "debug/deps"), os.fsencode(output / "debug"),
        os.fsencode(rustc.parent.parent / "lib"),
        home + b"/lib", b"/usr/local/lib", b"/usr/lib",
    }
    if len(entries) != len(set(entries)) or any(entry not in allowed for entry in entries):
        raise ValueError("Cargo macOS dynamic library path escapes selected locations")


def validate_cargo_probe(receipt, source):
    cwd = Path(os.fsdecode(receipt["cwd"]))
    if not cwd.is_absolute() or cwd.is_symlink() or cwd.resolve(strict=True) != source:
        raise ValueError("Cargo rustc probe working directory differs from admitted source")
    if tuple(receipt["arguments"]) not in ((b"-vV",), PROBE_QUERY):
        raise ValueError("first profile refuses unrecognized Cargo rustc probe arguments")


def inspect(receipt_dir, rustc, source, cache, output, transcript,
            expect_failure=False):
    roots = {"source": source.resolve(strict=True), "cache": cache.resolve(strict=True),
             "output": output.resolve(strict=True)}
    if not all(path.is_absolute() and path.is_dir() and not path.is_symlink()
               for path in (source, cache, output)):
        raise ValueError("selected compiler roots must be ordinary absolute directories")
    if not receipt_dir.is_absolute() or receipt_dir.resolve(strict=True) != roots["output"] / "rustc-receipts":
        raise ValueError("receipt directory differs from selected output")
    if not rustc.is_absolute() or rustc.is_symlink() or not rustc.is_file():
        raise ValueError("selected rustc must be an ordinary absolute file")
    entries = sorted(receipt_dir.iterdir())
    if not 1 <= len(entries) <= MAX_RECEIPTS:
        raise ValueError("rustc receipt set is empty or oversized")
    compiles = {}
    failed_crate = None
    failed_receipt = None
    probes = 0
    receipt_hashes = []
    for path in entries:
        if not NAME.fullmatch(path.name):
            raise ValueError(f"unexpected rustc receipt entry: {path.name}")
        receipt = parse_receipt(ordinary_bytes(path, MAX_RECEIPT))
        if receipt["rustc"] != os.fsencode(rustc):
            raise ValueError("rustc receipt names another compiler")
        if b"THRUM_SELECTED_RUSTC" in receipt["environment"] or b"THRUM_RECEIPT_DIR" in receipt["environment"]:
            raise ValueError("rustc child inherited wrapper control variables")
        environment = receipt["environment"]
        fixed = {
            b"CARGO_HOME": os.fsencode(roots["cache"]),
            b"CARGO_TARGET_DIR": os.fsencode(output),
            b"RUSTC": os.fsencode(rustc),
            b"CARGO_NET_OFFLINE": b"true",
            b"CARGO_INCREMENTAL": b"0",
            b"RUSTC_WRAPPER": os.fsencode(output / "rustc-receipt-wrapper"),
        }
        for name, value in fixed.items():
            if environment.get(name) != value:
                raise ValueError(
                    "rustc child environment differs from selected compiler inputs: "
                    + name.decode("ascii")
                )
        for name in environment:
            if (name.startswith(b"DYLD_") and name != b"DYLD_FALLBACK_LIBRARY_PATH") or name in (
                b"RUSTC_BOOTSTRAP", b"CARGO_ENCODED_RUSTFLAGS", b"RUSTC_WORKSPACE_WRAPPER",
            ):
                raise ValueError("rustc child environment contains an unsupported override: "
                                 + name.decode("ascii"))
        if receipt["kind"] == 1 or b"DYLD_FALLBACK_LIBRARY_PATH" in environment:
            validate_macos_dynamic_path(environment, rustc, roots["cache"], output)
        failed = receipt["outcome"] != 0 or receipt["status"] != 0
        if failed and (not expect_failure or receipt["kind"] != 1
                       or receipt["outcome"] != 0 or receipt["status"] <= 0
                       or failed_crate is not None):
            raise ValueError("build includes unexpected failed or incomplete rustc invocation")
        receipt_hashes.append(receipt["sha256"])
        if receipt["kind"] == 0:
            validate_cargo_probe(receipt, roots["source"])
            probes += 1
            continue
        crate, source_identity, externs, crate_types = compile_identity(receipt, roots)
        if receipt["environment"].get(b"CARGO_CRATE_NAME") != crate.encode("ascii"):
            raise ValueError("rustc child Cargo crate name differs from compile receipt")
        manifest_identity = compile_manifest(receipt, roots, source_identity)
        if crate in compiles:
            raise ValueError("first profile has duplicate crate-name invocations")
        if failed:
            failed_crate = crate
            failed_receipt = receipt["sha256"]
        compiles[crate] = {"source": source_identity, "externs": externs,
                           "crateTypes": crate_types,
                           "manifest": manifest_identity,
                           "receiptSha256": receipt["sha256"]}
    raw_transcript = ordinary_bytes(transcript, MAX_TRANSCRIPT)
    artifacts = {}
    finished = False
    error_diagnostics = 0
    for line in raw_transcript.splitlines():
        if not line:
            continue
        message = json.loads(line, object_pairs_hook=strict_pairs)
        reason = message.get("reason")
        if reason == "compiler-artifact":
            if message.get("fresh") is not False:
                raise ValueError("first profile requires a fresh output tree")
            target = message["target"]["name"].replace("-", "_")
            if target in artifacts:
                raise ValueError("duplicate Cargo artifact target")
            cargo_source = selected_file(
                Path(message["target"]["src_path"]),
                {key: roots[key] for key in ("source", "cache")},
                "Cargo target source",
            )
            if target not in compiles or compiles[target]["source"] != cargo_source:
                raise ValueError("Cargo target source differs from rustc compile receipt")
            cargo_manifest = selected_file(
                Path(message["manifest_path"]),
                {key: roots[key] for key in ("source", "cache")},
                "Cargo manifest",
            )
            if compiles[target]["manifest"] != cargo_manifest or cargo_source[0] != cargo_manifest[0]:
                raise ValueError("rustc child manifest differs from Cargo unit")
            if not isinstance(message["target"]["kind"], list) or any(
                kind in ("custom-build", "proc-macro") for kind in message["target"]["kind"]
            ):
                raise ValueError("first profile refuses executable Cargo hooks")
            if message["target"].get("crate_types") != compiles[target]["crateTypes"]:
                raise ValueError("Cargo target crate types differ from rustc receipt")
            paths = message["filenames"]
            if not isinstance(paths, list) or not paths:
                raise ValueError("Cargo unit has no artifacts")
            identities = []
            for filename in paths:
                path = Path(filename)
                identity = selected_file(path, {"output": roots["output"]}, "Cargo artifact")
                identities.append(identity[1])
            artifacts[target] = sorted(identities)
        elif reason == "build-finished":
            if finished or message.get("success") is not (not expect_failure):
                raise ValueError("Cargo build did not finish with the selected outcome once")
            finished = True
        elif reason == "build-script-executed":
            raise ValueError("first profile refuses build scripts")
        elif reason == "compiler-message":
            if (not expect_failure or message.get("target", {}).get("name", "").replace("-", "_") != failed_crate
                    or message.get("message", {}).get("level") != "error"):
                raise ValueError("unexpected Cargo compiler message")
            failed = compiles[failed_crate]
            selected = {key: roots[key] for key in ("source", "cache")}
            kinds = message["target"].get("kind")
            if (not isinstance(kinds, list) or any(kind in ("custom-build", "proc-macro")
                                                   for kind in kinds)
                    or selected_file(Path(message["manifest_path"]), selected, "failed Cargo manifest")
                    != failed["manifest"]
                    or selected_file(Path(message["target"]["src_path"]), selected,
                                     "failed Cargo target source") != failed["source"]
                    or message["target"]["crate_types"] != failed["crateTypes"]):
                raise ValueError("failed Cargo message differs from selected rustc unit")
            error_diagnostics += 1
        else:
            raise ValueError(f"unexpected Cargo build message: {reason}")
    if (not finished or set(compiles) - ({failed_crate} if failed_crate else set()) != set(artifacts)
            or (expect_failure and failed_crate is None)):
        raise ValueError("rustc compile receipts differ from Cargo artifact units: "
                         f"finished={finished} compiles={sorted(compiles)} "
                         f"artifacts={sorted(artifacts)} failed={failed_crate} "
                         f"errorDiagnostics={error_diagnostics}")
    artifact_owners = {}
    for crate, paths in artifacts.items():
        for path in paths:
            if path in artifact_owners:
                raise ValueError("duplicate Cargo artifact path")
            artifact_owners[path] = crate
    for crate, compile in compiles.items():
        for name, path in compile["externs"]:
            if artifact_owners.get(path) != name or name == crate:
                raise ValueError("rustc extern differs from a Cargo dependency artifact")
        del compile["manifest"]
    report = {
        "format": 1, "probes": probes, "compiles": len(compiles),
        "units": [{"crate": name, **compiles[name], "artifacts": artifacts[name]}
                  for name in sorted(artifacts)],
        "receiptSetSha256": hashlib.sha256("\n".join(sorted(receipt_hashes)).encode()).hexdigest(),
        "buildReady": False,
    }
    if expect_failure:
        report["compiles"] -= 1
        report["failedCompile"] = {"crate": failed_crate,
                                   "receiptSha256": failed_receipt,
                                   "errorDiagnostics": error_diagnostics}
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("receipt-dir", "rustc", "source", "cache", "output", "transcript"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--expect-failure", action="store_true")
    args = parser.parse_args()
    report = inspect(args.receipt_dir, args.rustc, args.source, args.cache,
                     args.output, args.transcript, args.expect_failure)
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()

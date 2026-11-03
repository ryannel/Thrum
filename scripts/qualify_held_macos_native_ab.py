"""Opt-in admitted A/B build using three cold-verified held APFS tool roots.

This local qualification is unready. It relies on the selected reviewed-tool
family assumption for successful process-group cleanup before owned scratch
is unsealed and removed. On failure, it preserves the named private scratch.
An outer timeout can kill the A/B test's supervisor before that supervisor
cleans its separately sessioned inner Cargo; such scratch must be quarantined
and the child state investigated before any later cleanup.
"""

from contextlib import ExitStack
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import subprocess
import sys
import tempfile
import time

if sys.platform == "darwin":
    import fcntl

from hold_macos_tool_root_clone import _names, _open_lock, open_sealed_clone, prepare_sealed_clone
from inventory_headless_tool_roots import inventory_root
from prepare_headless_cargo_home import collect_bounded
from prepare_macos_tool_root_clone import _checked_parent

_TARGET = "aarch64-apple-darwin"


def _selected_sources():
    rustc = Path(subprocess.check_output(
        ["rustup", "which", "--toolchain", "1.99.1", "rustc"], text=True,
    ).strip())
    clang = Path(subprocess.check_output(["xcrun", "--find", "clang"], text=True).strip())
    sdk = Path(subprocess.check_output(
        ["xcrun", "--sdk", "macosx", "--show-sdk-path"], text=True,
    ).strip())
    return {
        "rust": rustc.parent.parent.resolve(strict=True),
        "linker": clang.parents[2].resolve(strict=True),
        "sdk": sdk.resolve(strict=True),
    }


def _clear_owned_scratch(root):
    if root.is_symlink() or not root.is_dir():
        raise ValueError("owned scratch changed before cleanup")
    for directory, directories, files in os.walk(root):
        for name in [*directories, *files]:
            path = Path(directory) / name
            if not path.is_symlink():
                info = os.stat(path, follow_symlinks=False)
                if info.st_uid != os.geteuid():
                    raise ValueError("owned scratch contains another owner")
                os.chflags(path, info.st_flags & ~stat.UF_IMMUTABLE)
                path.chmod(0o700 if path.is_dir() else 0o600)
        info = os.stat(directory, follow_symlinks=False)
        if info.st_uid != os.geteuid():
            raise ValueError("owned scratch directory has another owner")
        os.chflags(directory, info.st_flags & ~stat.UF_IMMUTABLE)
        os.chmod(directory, 0o700)
    shutil.rmtree(root)


def main():
    if sys.platform != "darwin" or platform.machine() != "arm64":
        raise ValueError("held native A/B qualification requires macOS ARM64")
    checkout = Path(__file__).resolve().parent.parent
    scratch = checkout / "tmp" / "retained-root-lease"
    if not scratch.is_dir() or scratch.is_symlink():
        raise ValueError("expected owned retained-root scratch directory")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--resume-owner", type=Path)
    args = parser.parse_args()
    if args.resume_owner is None:
        owner = Path(tempfile.mkdtemp(prefix="held-ab-", dir=scratch))
    else:
        owner = args.resume_owner
        info = owner.lstat()
        if (not owner.is_absolute() or owner.parent != scratch or not owner.name.startswith("held-ab-")
                or not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid()
                or stat.S_IMODE(info.st_mode) != 0o700):
            raise ValueError("resume owner is not the selected private scratch")
    owner = _checked_parent(owner)
    owner_lock = _open_lock(owner, ".qualification.lock", fcntl.LOCK_EX, create=True)
    success = False
    selected = {}
    try:
        sources = _selected_sources()
        preparation = {}
        for name, source in sources.items():
            started = time.perf_counter_ns()
            expected = inventory_root(source)
            inventoried = time.perf_counter_ns()
            if args.resume_owner is None:
                root, _record = prepare_sealed_clone(source, owner, expected)
            else:
                root = owner / f"root-{expected['sha256']}"
            prepared = time.perf_counter_ns()
            selected[name] = (root, expected)
            preparation[name] = {
                "files": expected["files"],
                "sha256": expected["sha256"],
                "sourceInventoryMs": (inventoried - started) / 1e6,
                "prepareMs": (prepared - inventoried) / 1e6,
                "reusedSealedRoot": args.resume_owner is not None,
            }
        wrapper_build = [
            "rustup", "run", "1.99.1", "cargo", "build", "--locked", "--offline",
            "--target", _TARGET, "-p", "xtask", "--bin", "rustc_receipt",
        ]
        outer_env = {**os.environ, "RUSTC_WRAPPER": "",
                     "CARGO_TARGET_DIR": str(checkout / "target")}
        outer_env.pop("CARGO_BUILD_TARGET", None)
        wrapper_status, _wrapper_stdout, _wrapper_stderr = collect_bounded(
            wrapper_build, checkout, outer_env,
            stdout_limit=4 * 1024 * 1024, stderr_limit=4 * 1024 * 1024,
            timeout_seconds=120, label="held A/B wrapper build",
        )
        if wrapper_status != 0:
            raise RuntimeError("receipt-wrapper build failed under bounded process group")
        wrapper = checkout / "target" / _TARGET / "debug" / "rustc_receipt"
        if not wrapper.is_file():
            raise ValueError("selected receipt wrapper was not built")
        env = {
            **outer_env,
            "THRUM_PYTHON": "python3.11",
            "THRUM_TEST_NATIVE_SANDBOX": "1",
            "THRUM_TEST_PREPARE_CARGO_HOME": "1",
            "THRUM_TEST_RUSTC_RECEIPT_WRAPPER": str(wrapper),
            "THRUM_TEST_RETAINED_RUST_ROOT": str(selected["rust"][0]),
            "THRUM_TEST_RETAINED_LINKER_ROOT": str(selected["linker"][0]),
            "THRUM_TEST_RETAINED_SDK_ROOT": str(selected["sdk"][0]),
            "RUSTC_WRAPPER": "",
        }
        command = [
            "rustup", "run", "1.99.1", "cargo", "test", "--locked", "--offline",
            "-p", "thrum-cook",
            "admitted_workspace_builds_headless_game_and_failed_edit_keeps_prior_binary",
            "--", "--ignored", "--nocapture",
        ]
        log_path = owner / "admitted-ab.log"
        if log_path.is_symlink():
            raise ValueError("admitted A/B log path is aliased")
        log_path.unlink(missing_ok=True)
        with ExitStack() as leases:
            cold_open = {}
            held_leases = []
            for name, (root, expected) in selected.items():
                started = time.perf_counter_ns()
                lease = leases.enter_context(open_sealed_clone(owner, expected))
                held_leases.append(lease)
                if lease.root != root:
                    raise ValueError("held root differs from selected clone")
                cold_open[name] = (time.perf_counter_ns() - started) / 1e6
            started = time.perf_counter_ns()
            status, stdout, stderr = collect_bounded(
                command, checkout, env,
                stdout_limit=16 * 1024 * 1024, stderr_limit=4 * 1024 * 1024,
                timeout_seconds=300, label="held admitted A/B qualification",
            )
            log_path.write_bytes(stdout + b"\n--- stderr ---\n" + stderr)
            build_ms = (time.perf_counter_ns() - started) / 1e6
            if status != 0:
                raise RuntimeError(f"held A/B qualification failed; see {log_path}")
            if b"test result: ok. 1 passed;" not in stdout:
                raise ValueError(f"held A/B test did not report one passing case: {log_path}")
            required = (
                "admitted-headless-a rustc_receipts",
                "admitted-headless-a candidate_sha256",
                "admitted-headless-a retained_reload_ms",
                "admitted-headless-b rustc_receipts",
            )
            lines = stderr.decode("utf-8", errors="replace").splitlines()
            observations = {}
            for marker in required:
                matches = [line.strip()[:512] for line in lines if marker in line]
                if len(matches) != 1:
                    raise ValueError(f"held A/B result lacks one {marker} observation: {log_path}")
                observations[marker] = matches[0]
            for lease in held_leases:
                lease.assert_live()
            checked = time.perf_counter_ns()
            for root, expected in selected.values():
                if inventory_root(root) != expected:
                    raise ValueError("held tool bytes changed during admitted A/B build")
            postcheck_ms = (time.perf_counter_ns() - checked) / 1e6
        print(json.dumps({
            "buildReady": False,
            "checkoutCommit": subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=checkout, text=True,
            ).strip(),
            "preparation": preparation,
            "coldOpenMs": cold_open,
            "admittedAbMs": build_ms,
            "postBuildFullInventoryMs": postcheck_ms,
            "observations": observations,
        }, sort_keys=True))
        success = True
    finally:
        try:
            if success:
                with ExitStack() as cleanup_locks:
                    for _root, expected in selected.values():
                        lock_fd = _open_lock(owner, _names(expected)[2], fcntl.LOCK_EX)
                        cleanup_locks.callback(os.close, lock_fd)
                    _clear_owned_scratch(owner)
            else:
                print(f"unready held A/B scratch preserved: {owner}", file=sys.stderr)
        finally:
            os.close(owner_lock)


if __name__ == "__main__":
    main()

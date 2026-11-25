#!/usr/bin/env python3
"""Run the ordinary check in the repository's bounded merge verification cache."""

import fcntl
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


GIB = 1024 ** 3
CAP = 8 * GIB
FREE_FLOOR = 30_000_000_000
COLD_HEADROOM = 8 * GIB
WARM_HEADROOM = 2 * GIB
MAIN = "refs/remotes/origin/main"
COMMAND = ("./dev", "check")
PROFILE_CONTROLS = "/tmp/sim2d/profile-controls/*/target/"


class Unavailable(Exception):
    """The caller must run its own check."""


def git(root, *args, lease=()):
    result = subprocess.run(["git", *args], cwd=root, capture_output=True, check=True, pass_fds=lease)
    return result.stdout


def say(message):
    print(f"merge verification cache: {message}", flush=True)


def check_environment(root, env):
    if env.get("THRUM_MERGE_CACHE") == "0":
        raise Unavailable("disabled by THRUM_MERGE_CACHE=0")
    if any(key.startswith("THRUM_") and key not in ("THRUM_MERGE_CACHE", "THRUM_REQUIRE_GPU", "THRUM_PYTHON")
           for key in env):
        raise Unavailable("explicit Thrum check input override")
    # These overrides can redirect worktree inputs or build outputs. A relative
    # search path can also resolve a different tool after changing directories.
    git_overrides = git(root, "rev-parse", "--local-env-vars").decode().splitlines()
    output_overrides = ("CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_BUILD_DIR")
    if any(key in env for key in (*git_overrides, *output_overrides)):
        raise Unavailable("explicit Git or Cargo output override")
    for key in ("PATH", "PYTHONPATH"):
        if key in env and any(not Path(value).is_absolute() for value in env[key].split(os.pathsep)):
            raise Unavailable(f"checkout-relative {key}")
    for key in ("CARGO_HOME", "RUSTUP_HOME", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
                "RUSTDOC", "THRUM_PYTHON", "CLIPPY_CONF_DIR"):
        if env.get(key) and not Path(env[key]).is_absolute():
            raise Unavailable(f"checkout-relative {key}")


def revision(root, common):
    entries = git(root, "-c", "core.fsmonitor=false", "status", "--porcelain=v1", "-z",
                  "--untracked-files=all", "--ignored=matching", "--ignore-submodules=none")
    evidence = root / ".orchestrate/evidence"
    for entry in filter(None, entries.split(b"\0")):
        if not entry.startswith(b"!! "):
            raise Unavailable("staged, unstaged or untracked caller changes")
        path = Path(os.fsdecode(entry[3:]))
        if path.parts[0] in ("target", "tmp") and not (root / path.parts[0]).is_symlink():
            continue
        if "__pycache__" in path.parts and not (root / path).is_symlink():
            continue
        if path == Path(".orchestrate/evidence") and evidence.is_symlink() and evidence.resolve() == common / "orchestrate/evidence":
            continue
        raise Unavailable(f"checkout-local ignored path {path}")

    # Git status can hide paths marked assume-unchanged or skip-worktree. A
    # submodule's checked-out files also need more than the parent's commit.
    index = git(root, "ls-files", "--cached", "--stage", "-v", "-z").split(b"\0")
    if any(not entry.startswith((b"H 100", b"H 120000 ")) for entry in index if entry):
        raise Unavailable("index contains sparse, assumed-unchanged or submodule paths")
    return git(root, "rev-parse", "--verify", "HEAD^{commit}").decode().strip()


def verify_tracked_contents(root, commit):
    # Refresh an index built from the commit without trusting the caller's
    # cached file stats, which can outlive an assume-unchanged bit.
    with tempfile.TemporaryDirectory(prefix="merge-index-") as scratch:
        env = dict(os.environ, GIT_INDEX_FILE=str(Path(scratch) / "index"))
        command = ["git", "-c", "core.fsmonitor=false", "-c", "core.splitIndex=false",
                   "-c", "core.ignoreStat=false"]
        subprocess.run([*command, "read-tree", commit], cwd=root, env=env,
                       capture_output=True, check=True)
        refreshed = subprocess.run([*command, "update-index", "--refresh"], cwd=root, env=env,
                                   capture_output=True)
        if refreshed.returncode:
            raise Unavailable("tracked files do not match the caller's commit")


def inherited_cargo_config(root):
    """Cargo reads configuration in all parents of its working directory."""
    return [path for parent in root.parents for name in ("config", "config.toml")
            if (path := parent / ".cargo" / name).exists() or path.is_symlink()]


def check_cargo_layout(root):
    # Keep Cargo's ordinary output layout for every step. In particular, a
    # fixture's own Cargo invocation must keep its own build directory.
    metadata = subprocess.check_output(
        ["rustup", "run", "stable", "cargo", "metadata", "--locked", "--offline", "--no-deps",
         "--format-version", "1"], cwd=root, stderr=subprocess.PIPE,
    )
    layout = json.loads(metadata)
    target = str(root / "target")
    if (layout.get("target_directory") != target or layout.get("build_directory", target) != target
            or git(root, "ls-files", "-z", "--", "target")):
        raise Unavailable("checkout-local Cargo layout")


def real_directory(path):
    if path.is_symlink() or path.exists() and not path.is_dir():
        raise Unavailable(f"expected an owned directory at {path}")


def prepare_checkout(root, checkout, common, commit, lease):
    real_directory(checkout)
    if not checkout.exists():
        git(root, "worktree", "add", "--quiet", "--detach", str(checkout), commit, lease=lease)
    else:
        # Refuse to clean a replacement path or another repository.
        if (git(checkout, "rev-parse", "--path-format=absolute", "--git-common-dir",
                "--show-toplevel").splitlines() != [os.fsencode(common), os.fsencode(checkout)]
                or git(checkout, "rev-parse", "--git-dir").strip() == os.fsencode(common)):
            raise Unavailable("cache checkout is not a dedicated linked worktree")
        git(checkout, "checkout", "--quiet", "--force", "--detach", commit, lease=lease)
    clean_checkout(checkout, lease)
    if git(root, "config", "--null", "--list") != git(checkout, "config", "--null", "--list"):
        raise Unavailable("checkout-local Git configuration")
    if inherited_cargo_config(root) != inherited_cargo_config(checkout):
        raise Unavailable("checkout-local inherited Cargo configuration")
    base = git(root, "merge-base", commit, MAIN)
    if (git(checkout, "merge-base", "HEAD", MAIN) != base
            or revision(root, common) != commit or revision(checkout, common) != commit):
        raise Unavailable("caller and cache inputs differ")
    verify_tracked_contents(root, commit)
    verify_tracked_contents(checkout, commit)
    check_cargo_layout(root)
    check_cargo_layout(checkout)
    return base.decode().strip()


def clean_checkout(checkout, lease):
    # Keep Cargo products in the standard location and start per-run state clean.
    # The 2D guard builds standalone Cargo fixture workspaces. Keep their build
    # products too, while regenerating the fixture inputs on every run.
    git(checkout, "clean", "-ffdxq", "-e", "/target/", "-e", PROFILE_CONTROLS, lease=lease)
    real_directory(checkout / "target")
    for path in (checkout / "target/nextest", checkout / "target/tmp"):
        if path.is_symlink() or path.is_file():
            path.unlink()
        elif path.is_dir():
            shutil.rmtree(path)


def build_directories(checkout):
    paths = [checkout / "target", *sorted(checkout.glob(PROFILE_CONTROLS.strip("/")))]
    for path in paths:
        real_directory(path)
        if not path.resolve().is_relative_to(checkout):
            raise Unavailable(f"build directory leaves the cache checkout: {path}")
    return [path for path in paths if path.exists()]


def retained_bytes(checkout):
    paths = build_directories(checkout)
    if not paths:
        return 0
    result = subprocess.run(["du", "-skc", *paths], capture_output=True, check=True)
    return int(result.stdout.splitlines()[-1].split()[0]) * 1024


def space(cache, checkout):
    return retained_bytes(checkout), shutil.disk_usage(cache).free


def reset(checkout, reason):
    paths = build_directories(checkout)
    if paths:
        say(f"reset ({reason})")
    for path in paths:
        shutil.rmtree(path)


def required_free_space(retained):
    return FREE_FLOOR + max(WARM_HEADROOM, COLD_HEADROOM - retained)


def admit_space(cache, checkout):
    retained, free = space(cache, checkout)
    if retained > CAP or free < required_free_space(retained):
        reset(checkout, f"admission, retained={retained} bytes, available={free} bytes")
        retained, free = space(cache, checkout)
    if free < required_free_space(retained):
        raise Unavailable(f"low free space, retained={retained} bytes, available={free} bytes")
    return retained, free


def finish(cache, checkout, incomplete, lease):
    try:
        clean_checkout(checkout, lease)
        retained, free = space(cache, checkout)
        if retained > CAP or free < FREE_FLOOR:
            reset(checkout, f"retention, built={retained} bytes, available={free} bytes")
            retained, free = space(cache, checkout)
        say(f"retained={retained} bytes, available={free} bytes")
        incomplete.unlink()
    except (Unavailable, OSError, subprocess.CalledProcessError, ValueError) as error:
        say(f"maintenance deferred to the next run ({error})")


def cached_check(root, env):
    check_environment(root, env)
    common = Path(os.fsdecode(git(root, "rev-parse", "--path-format=absolute", "--git-common-dir").rstrip(b"\n")))
    commit = revision(root, common)
    state = common / "orchestrate"
    cache = state / "merge-cargo"
    for directory in (state, cache):
        real_directory(directory)
        directory.mkdir(exist_ok=True)
    checkout = cache / "checkout"
    if root == checkout or root.is_relative_to(cache):
        raise Unavailable("already in the cache checkout")

    fd = os.open(cache / "lock", os.O_CREAT | os.O_NOFOLLOW | os.O_RDWR, 0o600)
    with os.fdopen(fd, "r+b") as lease:
        try:
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise Unavailable("busy") from error
        lease_fds = (lease.fileno(),)
        base = prepare_checkout(root, checkout, common, commit, lease_fds)
        incomplete = cache / "incomplete"
        if incomplete.exists():
            reset(checkout, "interrupted check or maintenance")
            incomplete.unlink()
        incomplete.touch()
        try:
            retained, free = admit_space(cache, checkout)
        except Unavailable:
            incomplete.unlink()
            raise
        say(f"{'warm' if retained else 'cold'} candidate={commit} base={base} "
            f"retained={retained} bytes, available={free} bytes")
        # The check inherits the lease. If the helper is interrupted, the
        # next caller still cannot clean up while the check is running.
        child = subprocess.Popen(COMMAND, cwd=checkout, env=env, pass_fds=lease_fds)
        try:
            status = child.wait()
        except OSError as error:
            say(f"could not wait for the started check ({error})")
            return 1
        if status < 0:
            return 128 - status
        finish(cache, checkout, incomplete, lease_fds)
        return status


def main():
    root = Path.cwd()
    env = dict(os.environ)
    env.pop("THRUM_MERGE_VERIFY", None)
    try:
        return cached_check(root, env)
    except (Unavailable, OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        say(f"ordinary check in {root} ({error})")
    os.execve(COMMAND[0], COMMAND, env)


if __name__ == "__main__":
    sys.exit(main())

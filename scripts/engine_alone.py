#!/usr/bin/env python3
"""Compile every engine package's tests and benches in a workspace without the games."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parents[1]
GAMES = ("examples/v1-game", "examples/v1-game-ios")


def copy_workspace(copy: Path) -> None:
    files = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--",
         ".", *(f":(exclude){game}" for game in GAMES)],
        cwd=ROOT,
    )
    for name in dict.fromkeys(os.fsdecode(files).split("\0")[:-1]):
        source, destination = ROOT / name, copy / name
        if not source.exists() and not source.is_symlink():
            continue  # A tracked file may have been removed in the working tree.
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination, follow_symlinks=False)
    for game in GAMES:
        if (copy / game).exists() or (copy / game).is_symlink():
            raise ValueError(f"{game} was copied into the engine-only workspace")

    manifest = copy / "Cargo.toml"
    original = manifest.read_text()
    members = tomllib.loads(original)["workspace"]["members"]
    remaining = [member for member in members if member not in GAMES]
    if len(remaining) != len(members) - len(GAMES):
        raise ValueError("expected both games to be explicit workspace members")
    updated, count = re.subn(
        r"(?m)^members\s*=\s*\[[^\]]*\]",
        lambda _: "members = " + json.dumps(remaining),
        original,
    )
    if count != 1 or tomllib.loads(updated)["workspace"]["members"] != remaining:
        raise ValueError("could not edit the copied workspace members")
    manifest.write_text(updated)


def check() -> None:
    scratch = ROOT / "tmp"
    scratch.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="game-split-engine-alone-", dir=scratch) as directory:
        copy = Path(directory)
        copy_workspace(copy)
        packages = [
            tomllib.loads(path.read_text())["package"]["name"]
            for path in sorted((copy / "crates").glob("*/Cargo.toml"))
        ]
        if not packages:
            raise ValueError("no engine packages found in the copy")
        print(f"Absent from workspace copy: {', '.join(GAMES)}", flush=True)
        print(f"Compiling test and bench targets for {', '.join(packages)}", flush=True)
        env = dict(os.environ, CARGO_TARGET_DIR=str(copy / "target"), CARGO_INCREMENTAL="0",
                   CARGO_PROFILE_TEST_DEBUG="0")
        env.setdefault("RUSTC_WRAPPER", "")
        # Removing members changes the lockfile. Cargo prunes the copied lockfile offline.
        subprocess.run(
            ["rustup", "run", "stable", "cargo", "test", "--offline", "--no-run",
             "--lib", "--tests", "--benches", *(f"--package={name}" for name in packages)],
            cwd=copy, env=env, check=True,
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["check"])
    parser.parse_args()
    check()

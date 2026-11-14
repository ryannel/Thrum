"""Qualify SEA's integration source under the caller's shared GPU lock.

This changes only this worktree's branch. Final integration still belongs to
agent_loop.py merge. An existing exact-baseline proof is reused only when the
repository's own gate accepts it for the rebased covered source.
"""

from pathlib import Path
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]


def run(*command):
    return subprocess.run(command, cwd=ROOT, check=True)


def main():
    dirty = subprocess.check_output(
        ["git", "status", "--porcelain", "--untracked-files=all"], cwd=ROOT, text=True
    )
    if dirty:
        raise RuntimeError("Commit this worktree before qualifying its integration source")
    run("git", "fetch", "origin")
    try:
        run("git", "rebase", "origin/main")
    except subprocess.CalledProcessError:
        subprocess.run(["git", "rebase", "--abort"], cwd=ROOT, check=False)
        raise
    run("git", "rev-parse", "HEAD")
    gate = subprocess.run(
        [sys.executable, "scripts/exact_baselines.py", "gate"], cwd=ROOT, check=False
    )
    if gate.returncode == 0:
        print("REUSED: the exact-baseline gate accepts this integration source", flush=True)
    else:
        if shutil.disk_usage(ROOT).free < 16 * 1024**3:
            target = ROOT / "target"
            if target.resolve() != target:
                raise RuntimeError("Refusing to clean a linked or shared target directory")
            clean = (
                "rustup", "run", "1.99.1", "cargo", "clean", "--locked", "--offline",
                "--target-dir", str(target),
            )
            print("Low disk space: inspecting this worktree's rebuildable output", flush=True)
            run(*clean, "--dry-run")
            run(*clean)
            print(f"Free bytes after private target cleanup: {shutil.disk_usage(ROOT).free}", flush=True)
        run(sys.executable, "scripts/exact_baselines.py", "verify")


if __name__ == "__main__":
    main()

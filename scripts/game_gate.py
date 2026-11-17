"""Build a detached game against this engine checkout, without editing either tree.

Usage: game_gate.py [--allow-dirty] [--record PATH] GAME COMMAND [ARG...]
`test` runs Cargo tests. Other commands run the game's scripts/game.py after
building it. `boundary` checks the shared shape rules without building.
Records default to tmp/game-gate/<unique run>/record.json and copies/builds are
removed on exit. Exit codes: 3 missing repository, 4 dirty tree, 5 boundary,
6 build failure, 7 game command failure, 2 invalid invocation/configuration.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

import shape_rules

ROOT = Path(__file__).resolve().parents[1]


def git(root, *args):
    return subprocess.check_output(["git", "--no-optional-locks", "-C", str(root), *args], text=True).strip()


def games_repository(engine):
    if "THRUM_GAMES" in os.environ:
        return Path(os.environ["THRUM_GAMES"]).expanduser().resolve()
    common = Path(git(engine, "rev-parse", "--git-common-dir"))
    if not common.is_absolute():
        common = engine / common
    return common.resolve().parent.parent / "thrum-games"


def copy_tree(source, destination):
    # Dereference links only when their targets remain in the repository. A copy
    # must never leave a live link through which a command can edit its source.
    for path in source.rglob("*"):
        if ".git" in path.relative_to(source).parts:
            continue
        if path.is_symlink() and not path.resolve().is_relative_to(source):
            raise ValueError(f"external symlink: {path.relative_to(source)}")
    shutil.copytree(source, destination, ignore=shutil.ignore_patterns(
        ".git", "target", "__pycache__"), symlinks=False)


def tree_digest(root):
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        if path.is_file():
            digest.update(path.relative_to(root).as_posix().encode() + b"\0")
            digest.update(path.read_bytes())
    return digest.hexdigest()


def run(args, engine=ROOT):
    repository = games_repository(engine)
    if not repository.is_dir() or not (repository / ".git").exists():
        print(f"game_gate: missing games repository: {repository}", file=sys.stderr)
        return 3
    if git(repository, "rev-parse", "--show-toplevel") != str(repository):
        raise ValueError("games path must be a repository root")
    if engine.is_relative_to(repository):
        raise ValueError("engine checkout must stay outside the games repository")
    if Path(args.game).name != args.game or args.game in {".", ".."}:
        raise ValueError("game must be a single folder name")
    dirty = git(repository, "status", "--porcelain=v1", "--untracked-files=all")
    if dirty and not args.allow_dirty:
        print("game_gate: games repository has uncommitted changes; use --allow-dirty to take it as it stands", file=sys.stderr)
        return 4
    scratch = engine / "tmp/game-gate"
    scratch.mkdir(parents=True, exist_ok=True)
    record_dir = Path(tempfile.mkdtemp(prefix="run-", dir=scratch))
    record_path = Path(args.record).resolve() if args.record else record_dir / "record.json"
    if record_path.is_relative_to(repository):
        raise ValueError("record output must stay outside the games repository")
    record = {"engine_revision": git(engine, "rev-parse", "HEAD"),
              "engine_dirty": bool(git(engine, "status", "--porcelain")),
              "game_revision": git(repository, "rev-parse", "HEAD"),
              "games_repository": str(repository), "game": args.game,
              "command": [args.command, *args.arguments], "taken_as_stands": bool(dirty),
              "games_status": dirty}
    code = 2
    try:
        with tempfile.TemporaryDirectory(prefix="copy-", dir=record_dir) as directory:
            stage = Path(directory)
            # Stop Cargo walking up into the engine workspace. A game's own
            # workspace, when present, remains the nearer authority.
            (stage / "Cargo.toml").write_text('[workspace]\nresolver="2"\nmembers=[' + json.dumps("thrum-games/" + args.game) + ']\n')
            (stage / "thrum").symlink_to(engine, target_is_directory=True)
            detached = stage / "thrum-games"
            copy_tree(repository, detached)
            record["source_snapshot_sha256"] = tree_digest(detached)
            game = detached / args.game
            if not (game / "Cargo.toml").is_file():
                raise ValueError(f"game has no Cargo.toml: {args.game}")
            # Standalone games join the staging workspace. Seed its lockfile
            # from the game's; games with their own workspace use their own.
            if (game / "Cargo.lock").is_file():
                shutil.copyfile(game / "Cargo.lock", stage / "Cargo.lock")
            errors = shape_rules.external_boundary(game, engine)
            if errors:
                print("\n".join(errors), file=sys.stderr)
                code = 5
            elif args.command == "boundary":
                code = 0
            else:
                env = dict(os.environ, CARGO_TARGET_DIR=str(stage / "target"), RUSTC_WRAPPER="")
                cargo = ["rustup", "run", "1.99.1", "cargo"]
                build = subprocess.run([*cargo, "build", "--offline", "--manifest-path", str(game / "Cargo.toml")], cwd=game, env=env)
                record["build_exit_code"] = build.returncode
                if build.returncode:
                    code = 6
                else:
                    command = ([*cargo, "test", "--offline", *args.arguments] if args.command == "test"
                               else [sys.executable, str(game / "scripts/game.py"), args.command, *args.arguments])
                    result = subprocess.run(command, cwd=game, env=env)
                    record["command_exit_code"] = result.returncode
                    code = 7 if result.returncode else 0
    finally:
        record["exit_code"] = code
        record_path.parent.mkdir(parents=True, exist_ok=True)
        record_path.write_text(json.dumps(record, indent=2) + "\n")
        print(f"game_gate: record {record_path}")
    return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allow-dirty", action="store_true")
    parser.add_argument("--record")
    parser.add_argument("game")
    parser.add_argument("command")
    parser.add_argument("arguments", nargs=argparse.REMAINDER)
    try:
        return run(parser.parse_args())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"game_gate: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())

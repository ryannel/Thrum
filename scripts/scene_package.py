"""Resolve a scene folder to its workspace package and window binary from Cargo's metadata.

Usage: scene_package.py [--lock] <folder>

Prints `<package>\t<binary>`. With `--lock`, a folder that resolves is then added to Cargo.lock
offline if the lockfile does not cover it yet; nothing is written for a folder that does not
resolve. `./dev scene` and `./dev starter` call this before building.
"""

import json
import os
from pathlib import Path
import subprocess
import sys

GUIDE = 'see examples/starter/README.md, "Make your own scene"'


class SceneError(Exception):
    pass


def cargo():
    return os.environ.get("CARGO", "cargo")


def workspace_metadata(root):
    """Cargo's own reading of the workspace members; resolves no dependencies, writes nothing."""
    result = subprocess.run(
        [cargo(), "metadata", "--format-version", "1", "--no-deps", "--offline"],
        cwd=root, capture_output=True, text=True,
    )
    if result.returncode != 0:
        raise SceneError(f"cargo metadata failed; fix the manifest it names:\n{result.stderr.strip()}")
    return json.loads(result.stdout)


def resolve(root, folder, metadata):
    """Return (package, binary) for the workspace member whose manifest is `<folder>/Cargo.toml`."""
    manifest = (Path(root) / folder / "Cargo.toml").resolve()
    if not manifest.is_file():
        raise SceneError(f"{folder} has no Cargo.toml; a scene is a package folder such as "
                         f"examples/starter ({GUIDE})")
    members = set(metadata["workspace_members"])
    for package in metadata["packages"]:
        if package["id"] not in members or Path(package["manifest_path"]).resolve() != manifest:
            continue
        binaries = [target["name"] for target in package["targets"] if "bin" in target["kind"]]
        if len(binaries) != 1:
            found = ", ".join(binaries) or "none"
            raise SceneError(f"{folder} ({package['name']}) builds {len(binaries)} binaries ({found}); "
                             "a scene package builds exactly one, its window")
        binary = binaries[0]
        # Every workspace binary lands in one target/release, so a shared name would replace the
        # other program; xtask would then run the scene where it means to run thrum-cook.
        for other in metadata["packages"]:
            if other["id"] in members and other["id"] != package["id"] and any(
                    target["name"] == binary and "bin" in target["kind"] for target in other["targets"]):
                raise SceneError(f"{folder} ({package['name']}) names its binary {binary}, which "
                                 f"{other['name']} also builds into the same target folder; name the "
                                 "scene's binary after the scene, such as its package name")
        return package["name"], binary
    raise SceneError(f'{folder} is not a workspace member; add "{folder}" to [workspace] members in '
                     "Cargo.toml and a [packages.<name>] entry to .config/architecture.toml "
                     f"({GUIDE})")


def lockfile_current(root):
    result = subprocess.run(
        [cargo(), "metadata", "--format-version", "1", "--locked", "--offline"],
        cwd=root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    return result.returncode == 0


def main(argv, root=None):
    lock = argv[:1] == ["--lock"]
    args = argv[1:] if lock else argv
    if len(args) != 1:
        print("usage: scene_package.py [--lock] <folder>", file=sys.stderr)
        return 2
    root = Path.cwd() if root is None else root
    try:
        package, binary = resolve(root, args[0], workspace_metadata(root))
    except SceneError as error:
        print(error, file=sys.stderr)
        return 1
    if lock and not lockfile_current(root):
        print("Adding new workspace packages to Cargo.lock (offline); commit the change with them.",
              file=sys.stderr)
        subprocess.run([cargo(), "update", "--offline", "--workspace", "--quiet"], cwd=root, check=True)
    print(f"{package}\t{binary}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

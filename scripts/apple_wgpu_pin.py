"""Verify the source of the patched wgpu crates without accessing the network."""

import hashlib
from pathlib import Path
import subprocess
import tempfile
import tomllib


def verify(root, name="wgpu-hal"):
    architecture = tomllib.loads((root / ".config/architecture.toml").read_text())
    pin = architecture.get("registry_patches", {}).get(name)
    if pin is None:
        raise ValueError(f"no pin registered for {name}")
    version = pin["version"].removeprefix("=")
    crate = f"{name}-{version}"
    tree = (root / pin["manifest"]).parent
    workspace = tomllib.loads((root / "Cargo.toml").read_text())
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    packages = [package for package in lock["package"] if package["name"] == name]
    if (workspace["patch"]["crates-io"].get(name) != {"path": str(tree.relative_to(root))}
            or len(packages) != 1 or packages[0].get("source") or packages[0]["version"] != version
            or tomllib.loads((tree / "Cargo.toml").read_text())["package"]["version"] != version):
        raise ValueError(f"Cargo does not select the registered {name} copy")

    archive = root / pin["release"]
    if hashlib.sha256(archive.read_bytes()).hexdigest() != pin["sha256"]:
        raise ValueError(f"{crate}.crate differs from the pinned release")
    patches = sorted((root / pin["patches"]).glob("*.patch"))
    if not patches:
        raise ValueError(f"no {name} patch files")

    scratch = root / "tmp"
    scratch.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="wgpu-pin-", dir=scratch) as folder:
        subprocess.run(["tar", "-xf", archive, "-C", folder], check=True)
        release = Path(folder) / crate
        for patch in patches:
            subprocess.run(["patch", "--batch", "--fuzz=0", "-p1", "-i", patch],
                           check=True, cwd=release, stdout=subprocess.DEVNULL)
        actual = {p.relative_to(tree): p.read_bytes() for p in tree.rglob("*") if p.is_file()}
        rebuilt = {p.relative_to(release): p.read_bytes() for p in release.rglob("*") if p.is_file()}
        changed = sorted(p for p in actual.keys() | rebuilt.keys()
                         if actual.get(p) != rebuilt.get(p))
        if changed:
            raise ValueError(f"{crate} differs from release plus patches: {', '.join(map(str, changed))}")
    return {"version": version, "sha256": pin["sha256"],
            "files": len(actual), "patches": [str(p.relative_to(root)) for p in patches]}

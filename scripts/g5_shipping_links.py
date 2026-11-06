#!/usr/bin/env python3
"""G5 proof that shipping-shaped binaries do not link authoring code.

Checks the normal (non-dev, non-build) dependency graph of each shipping package, then builds the
release asset-loop viewer and scans its symbol table for authoring crates. Run from the repository
root: `python3 scripts/g5_shipping_links.py`.
"""

import re
import subprocess
import sys

AUTHORING = ["thrum-cook", "meshopt", "intel_tex_2", "astcenc-rs", "gltf", "image", "notify"]
SHIPPING = ["asset-loop", "v1-game", "thrum-asset"]
# Rust v0 symbols name crates as `Cs<disambiguator>_<len><name>`; C libraries by their entry points.
SYMBOL = re.compile(
    r"Cs[0-9A-Za-z]*_(10thrum_cook|7meshopt|11intel_tex_2|10astcenc_rs|4gltf|6notify|5image)"
    r"|_?meshopt_|_?astcenc_|CompressBlocks|ispc|FSEventStream"
)


def main():
    failures = []
    for package in SHIPPING:
        tree = subprocess.run(
            ["cargo", "tree", "-p", package, "-e", "normal", "--prefix", "none", "--format", "{p}"],
            check=True, capture_output=True, text=True,
        ).stdout
        names = {line.split()[0] for line in tree.splitlines() if line.strip()}
        linked = sorted(names & set(AUTHORING))
        print(f"{package}: {len(names)} packages in normal graph; authoring packages: {linked or 'none'}")
        failures += [f"{package} -> {name}" for name in linked]
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "asset-loop"], check=True)
    symbols = subprocess.run(["nm", "-a", "target/release/asset-loop"], check=True, capture_output=True, text=True).stdout
    hits = sorted({line.split()[-1] for line in symbols.splitlines() if SYMBOL.search(line)})
    print(f"asset-loop release binary: {len(symbols.splitlines())} symbols; authoring matches: {len(hits)}")
    failures += [f"symbol {name}" for name in hits[:20]]
    # Positive control: the same scan must find the authoring code in the cooker binary.
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "thrum-cook", "--bin", "thrum-cook"], check=True)
    control = subprocess.run(["nm", "-a", "target/release/thrum-cook"], check=True, capture_output=True, text=True).stdout
    found = sum(1 for line in control.splitlines() if SYMBOL.search(line))
    print(f"control: thrum-cook release binary authoring matches: {found}")
    if found == 0:
        failures.append("symbol scan found nothing in the cooker binary; the pattern is vacuous")
    if failures:
        print("\n".join(failures))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

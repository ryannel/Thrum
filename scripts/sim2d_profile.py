#!/usr/bin/env python3
"""Enforce the deterministic rule-package boundary and exercise refusal controls."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / ".config/sim2d-profile/clippy.toml"
LINTS = ["float_arithmetic", "disallowed_methods", "disallowed_types"]


def packages(root=ROOT):
    return tomllib.loads((root / ".config/architecture.toml").read_text())["packages"]


def configuration_errors(directory, manifest, allowed):
    errors = []
    config = directory / "clippy.toml"
    if not config.exists() or config.read_bytes() != PROFILE.read_bytes():
        errors.append("missing profile configuration: clippy.toml must match .config/sim2d-profile/clippy.toml")
    for path in directory.rglob("*.rs"):
        source = path.read_text()
        # Refuse suppression even through cfg_attr, and compiler/environment macros.
        if re.search(r"#\s*!?\s*\[[^\]]*\b(?:allow|expect)\s*\(", source):
            errors.append(f"suppressed lint: {path.name}")
        if re.search(r"\b(?:env|option_env|include)\s*!", source):
            errors.append(f"environment read or outside source: {path.name}")
        if re.search(r"#\s*\[\s*path\s*=", source):
            errors.append(f"outside source: {path.name}")
    if manifest.get("package", {}).get("build") or (directory / "build.rs").exists():
        errors.append("outside dependency: rule packages cannot execute build scripts")
    if manifest.get("lib", {}).get("proc-macro"):
        errors.append("outside dependency: rule packages cannot be proc macros")
    if manifest.get("features"):
        errors.append("outside dependency: rule packages cannot alter dependency features")
    scopes = [manifest, *manifest.get("target", {}).values()]
    for scope in scopes:
        for kind in ["dependencies", "dev-dependencies", "build-dependencies"]:
            for alias, spec in scope.get(kind, {}).items():
                name = spec.get("package", alias) if isinstance(spec, dict) else alias
                if name not in allowed:
                    errors.append(f"outside dependency: {name}")
                if isinstance(spec, dict) and spec.get("features"):
                    errors.append(f"outside dependency: {name} feature selection is not in the profile")
    return errors


def clippy(manifest):
    environment = dict(os.environ, RUSTC_WRAPPER="")
    result = subprocess.run(["rustup", "run", "1.99.1", "cargo", "clippy", "--offline",
                             "--manifest-path", str(manifest), "--all-targets", "--no-deps", "--",
                             *[flag for lint in LINTS for flag in ["-D", f"clippy::{lint}"]]],
                            cwd=ROOT, env=environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    return result.returncode, result.stdout


def check():
    registry = packages()
    rules = {name: entry for name, entry in registry.items() if entry.get("rule_package")}
    if not rules:
        raise RuntimeError("no registered rule packages")
    allowed = {"thrum-core", "thrum-sim2d", *rules}
    results = []
    for name, entry in rules.items():
        manifest = ROOT / entry["manifest"]
        errors = configuration_errors(manifest.parent, tomllib.loads(manifest.read_text()), allowed)
        if errors:
            raise RuntimeError(f"{name}: " + "; ".join(errors))
        code, output = clippy(manifest)
        if code:
            raise RuntimeError(f"{name}: profile Clippy failed\n{output}")
        results.append({"package": name, "result": "pass", "output": output})
    return results


def controls():
    """Every package starts from the same passing manifest, config and source."""
    fixtures = tomllib.loads((ROOT / ".config/sim2d-profile/fixtures.toml").read_text())
    scratch = ROOT / "tmp/sim2d/profile-controls"
    scratch.mkdir(parents=True, exist_ok=True)
    baseline = fixtures["baseline"]
    rows = []
    for name, case in {"passing": {"source": baseline}, **fixtures["cases"]}.items():
        directory = scratch / name
        (directory / "src").mkdir(parents=True, exist_ok=True)
        manifest = ('[workspace]\n[package]\nname = "profile-control"\nversion = "0.0.0"\nedition = "2024"\n'
                    '[dependencies]\nthrum-core = { path = "' + str(ROOT / "crates/thrum-core") + '" }\n')
        if name == "dependency":
            manifest += 'thrum-network = { path = "' + str(ROOT / "crates/thrum-network") + '" }\n'
        path = directory / "Cargo.toml"
        path.write_text(manifest)
        (directory / "src/lib.rs").write_text(case["source"])
        config = directory / "clippy.toml"
        config.write_bytes(PROFILE.read_bytes())
        if name == "configuration":
            config.unlink()
        errors = configuration_errors(directory, tomllib.loads(manifest), {"thrum-core", "thrum-sim2d"})
        if errors:
            code, output = 1, "\n".join(errors)
        else:
            code, output = clippy(path)
        expected = case.get("diagnostic")
        if name == "passing":
            if code:
                raise RuntimeError(f"passing fixture refused:\n{output}")
        elif not code or expected not in output:
            raise RuntimeError(f"{name}: expected refusal naming {expected!r}, got {code}:\n{output}")
        rows.append({"class": name, "result": "pass", "diagnostic": expected, "output": output})
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["check", "controls", "sync"])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.action == "sync":
        for entry in packages().values():
            if entry.get("rule_package"):
                (ROOT / entry["manifest"]).with_name("clippy.toml").write_bytes(PROFILE.read_bytes())
        return
    rows = globals()[args.action]()
    record = {"commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
              "target": "mac-metal", "action": args.action, "rows": rows}
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))


if __name__ == "__main__":
    main()

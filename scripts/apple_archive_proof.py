"""Metal archive harvest, fresh-process witnesses and launch qualification."""

from collections import Counter
import hashlib
import json
from pathlib import Path
from statistics import median
import tempfile

from apple_creation_proof import READY, cache_inventory, probe_build, witness
from apple_metallib_proof import cook_recipe, describe, manifest
from apple_presentation import ROOT, run

RECIPE = "apple-tile"
OMIT = "synthetic history publisher"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def receipts(directory, marker):
    return [line.removeprefix(marker) for line in (directory / "process.log").read_text().splitlines()
            if line.startswith(marker)]


def setup(output, scratch):
    binary, env, route, build = probe_build(scratch)
    store, expected, cook = cook_recipe(output, scratch, binary, env, RECIPE)
    path = output / "pipeline-manifest.json"
    manifest_bytes = json.dumps(cook["pipelines"], sort_keys=True, separators=(",", ":")).encode()
    path.write_bytes(manifest_bytes + b"\n")
    inputs = dict(env, THRUM_METALLIB_STORE=str(store), THRUM_PIPELINE_MANIFEST=str(path))
    return binary, inputs, route, build, expected, cook


def lookup_rows(directory):
    return [tuple(row.split(" ", 1))
            for row in receipts(directory, "THRUM_METAL_ARCHIVE_LOOKUP ")]


def names(expected):
    counts = Counter()
    for (_, label, _), count in expected.items():
        counts[label] += count
    return counts


def harvest(output, name, binary, env, expected, omit=None):
    """Launch an explicit harvest process and verify exactly what it was asked to archive."""
    file = output / f"{name}.archive"
    directory = output / f"{name}-harvest"
    launch = dict(env, THRUM_PIPELINE_ARCHIVE_HARVEST=str(file))
    if omit is not None:
        launch["THRUM_PIPELINE_ARCHIVE_OMIT"] = omit
    result = witness(directory, binary, launch, RECIPE, "launch", stage_path="cooked")
    rows = [tuple(row.split(" ", 1))
            for row in receipts(directory, "THRUM_METAL_ARCHIVE_HARVEST ")]
    wanted = Counter({("omit" if omit in ("*", label) else "add", label): count
                      for label, count in names(expected).items()})
    reported, _, _ = manifest(directory, "cooked", parallel=True)
    saved, = receipts(directory, "THRUM_METAL_ARCHIVE_SAVED ")
    data = file.read_bytes().split(b"\n", 5)[-1]
    if (Counter(rows) != wanted or reported != expected
            or not data.startswith(b"WGPUPLCH") or len(data) != int(saved)
            or any(result["after_ready"].values())):
        raise ValueError(f"{name}: incomplete archive harvest")
    if (omit == "*") != (len(data) == 64):
        raise ValueError(f"{name}: Metal payload size disagrees with the harvest controls")
    return file, dict(sha256=sha(file), data_bytes=int(saved), added=sum(action == "add" for action, _ in rows),
                      omitted=[label for action, label in rows if action == "omit"], result=result)


def archive_harvest(output):
    with tempfile.TemporaryDirectory(prefix="apple-archive-harvest-", dir=ROOT / "tmp") as temp:
        binary, env, _, build, expected, cook = setup(output, Path(temp))
        file, receipt = harvest(output, "complete", binary, env, expected)
    return dict(target="mac-metal", build=build, cook=cook, archive=file.name, harvest=receipt)


def archive_witness(output):
    with tempfile.TemporaryDirectory(prefix="apple-archive-witness-", dir=ROOT / "tmp") as temp:
        binary, env, _, build, expected, cook = setup(output, Path(temp))
        cases = {}
        for name, omit in (("complete", None), ("empty", "*"), ("partial", OMIT)):
            file, receipt = harvest(output, name, binary, env, expected, omit)
            digest = sha(file)
            file.chmod(0o400)
            directory = output / f"{name}-strict"
            launch = dict(env, THRUM_PIPELINE_PREPARATION="strict-archive", THRUM_PIPELINE_ARCHIVE=str(file))
            result = witness(directory, binary, launch, RECIPE, "launch",
                             stage_path="cooked", expect_refusal=omit is not None)
            rows = lookup_rows(directory)
            if sha(file) != digest or receipts(directory, "THRUM_METAL_ARCHIVE_SAVED "):
                raise ValueError(f"{name}: the strict process changed the archive")
            if len(receipts(directory, "THRUM_METAL_ARCHIVE_IMPORT ")) != 1:
                raise ValueError(f"{name}: strict process did not open the cache")
            if omit is None:
                prepared, _, _ = manifest(directory, "cooked", parallel=True)
                if (prepared != expected or any(result["after_ready"].values())
                        or Counter(rows) != Counter({("Hit", label): count
                                                     for label, count in names(expected).items()})):
                    raise ValueError("strict witness did not hit the full manifest")
            else:
                lines = (directory / "process.log").read_text().splitlines()
                missing = [label for status, label in rows if status == "Miss"]
                if (result["exit_code"] == 0 or any(READY.fullmatch(line) for line in lines)
                        or not missing or any(status not in ("Hit", "Miss") for status, _ in rows)
                        or (omit == "*" and any(status == "Hit" for status, _ in rows))
                        or (omit != "*" and set(missing) != {omit})):
                    raise ValueError(f"{name}: strict negative control failed to expose only its omission: {rows}")
            cases[name] = dict(harvest=receipt, result=result, lookups=rows,
                               unchanged_archive_sha256=digest)
    return dict(target="mac-metal", build=build, cook=cook, cases=cases)


def file_fault(source, target, cause):
    magic, device, os_build, key, digest, data = source.read_bytes().split(b"\n", 5)

    def change_byte(value):
        return (b"0" if value[0] != ord("0") else b"1") + value[1:]

    if cause == "corrupt":
        data = data[:-1] + bytes([data[-1] ^ 1])
    else:
        key = change_byte(key)
    target.write_bytes(b"\n".join((magic, device, os_build, key, digest, data)))
    if target.stat().st_size != source.stat().st_size:
        raise ValueError(f"{cause}: control changed archive length")


def archive_faults(output):
    with tempfile.TemporaryDirectory(prefix="apple-archive-faults-", dir=ROOT / "tmp") as temp:
        binary, env, route, build, expected, cook = setup(output, Path(temp))
        complete, full = harvest(output, "complete", binary, env, expected)
        partial, omitted = harvest(output, "partial", binary, env, expected, OMIT)
        cases = {}
        for cause in ("stale", "corrupt", "miss"):
            file = partial if cause == "miss" else output / f"{cause}.archive"
            if cause != "miss":
                file_fault(complete, file, cause)
            before = sha(file)
            directory = output / f"{cause}-route"
            launch = dict(env, THRUM_PIPELINE_PREPARATION="archive", THRUM_PIPELINE_ARCHIVE=str(file))
            result = witness(directory, binary, launch, RECIPE, "route", stage_path="cooked")
            prepared, _, _ = manifest(directory, "cooked", parallel=True)
            if (prepared != expected or any(result["after_ready"].values())
                    or result["routed_ticks"] != list(range(route["keys"][-1]["tick"] + 1))
                    or len(result["finished"]) != 1 or result["finished"][0][1:] != [0, 0]
                    or sha(file) != before or receipts(directory, "THRUM_METAL_ARCHIVE_SAVED ")):
                raise ValueError(f"{cause}: readiness, full route or archive integrity failed")
            rows = lookup_rows(directory)
            imports = receipts(directory, "THRUM_METAL_ARCHIVE_IMPORT ")
            rejections = receipts(directory, "THRUM_METAL_ARCHIVE_REJECT ")
            if cause == "miss":
                wanted = Counter({("Hit", label): count for label, count in names(expected).items()})
                del wanted[("Hit", OMIT)]
                wanted["Miss", OMIT] = 1
                if rejections or len(imports) != 1 or Counter(rows) != wanted:
                    raise ValueError(f"{cause}: expected one native miss and no rejection: {rows}")
            elif rejections != [cause] or imports or rows:
                raise ValueError(f"{cause}: expected rejection before import, got {rejections} {imports} {rows}")
            cases[cause] = dict(archive_sha256=before, result=result, archive_import_calls=len(imports),
                                rejections=rejections, lookups=rows)
    return dict(target="mac-metal", build=build, cook=cook, harvests=dict(complete=full, partial=omitted),
                cases=cases)


def launch_final(output):
    with tempfile.TemporaryDirectory(prefix="apple-launch-final-", dir=ROOT / "tmp") as temp:
        scratch = Path(temp)
        binary, env, _, build, expected, cook = setup(output, scratch)
        file, harvest_receipt = harvest(output, "complete", binary, env, expected)
        env = dict(env, THRUM_PIPELINE_ARCHIVE=str(file))
        probe = scratch / "cache-probe.dylib"
        run("xcrun", "clang", "-Wall", "-Werror", "-dynamiclib", "-framework",
            "CoreFoundation", str(ROOT / "scripts/apple_metal_cache_probe.c"), "-o", str(probe))
        launches = []
        modes = ("wgsl", "parallel", "archive")
        for repeat, order in enumerate((modes, modes[::-1]), 1):
            for mode in order:
                cache = scratch / f"cache-{mode}-{repeat}"
                cache.mkdir()
                for temperature in ("cold", "warm"):
                    before = cache_inventory(cache)
                    if bool(before) != (temperature == "warm"):
                        raise ValueError(f"{mode}: unexpected {temperature} cache inventory")
                    directory = output / f"{mode}-{repeat}-{temperature}"
                    stage_path = "wgsl" if mode == "wgsl" else "cooked"
                    launch = dict(env, THRUM_PIPELINE_PREPARATION=mode)
                    result = witness(directory, binary, launch, RECIPE, "launch", (probe, cache),
                                     stage_path=stage_path)
                    after = cache_inventory(cache)
                    prepared, stages, threads = manifest(directory, stage_path, parallel=mode != "wgsl")
                    rows = lookup_rows(directory)
                    expected_hits = names(expected) if mode == "archive" else Counter()
                    if (prepared != expected or not after or result["finished"]
                            or any(result["after_ready"].values())
                            or any(status != "Hit" for status, _ in rows)
                            or Counter(label for _, label in rows) != expected_hits):
                        raise ValueError(f"{directory}: launch did not prepare the requested manifest")
                    launches.append(dict(mode=mode, repeat=repeat, temperature=temperature,
                                         cache_before=before, cache_after=after, **result,
                                         **describe(prepared, stages, threads)))
        defaults = output / "registry-default"
        result = witness(defaults, binary, env, RECIPE, "launch", stage_path="cooked")
        rows = lookup_rows(defaults)
        selected, = receipts(defaults, "THRUM_METAL_PREPARATION ")
        stage_path = "wgsl" if selected == "wgsl" else "cooked"
        prepared, _, _ = manifest(defaults, stage_path, parallel=selected != "wgsl")
        if prepared != expected or (selected == "archive" and Counter(rows) != Counter(
                {("Hit", label): count for label, count in names(expected).items()})):
            raise ValueError("registry default did not exercise its selected path")
        totals = {mode: {temperature: median(row["launch_to_ready_ns"] for row in launches
                                            if row["mode"] == mode and row["temperature"] == temperature)
                         for temperature in ("cold", "warm")} for mode in modes}
        ranked = sorted(modes, key=lambda mode: sum(totals[mode].values()))
    return dict(target="mac-metal", build=build, cook=cook, harvest=harvest_receipt, launches=launches,
                median_ns=totals, ranking=ranked, default_mode=selected, default_launch=result,
                ranking_policy="equal-weight cold and warm median launch-to-ready wall time",
                cache_scope="fresh Metal process each time. Cold driver cache directory is empty, warm reuses its cold run. Local archive was harvested separately and stays present for both archive launches.",
                timing_decides_pass=False)

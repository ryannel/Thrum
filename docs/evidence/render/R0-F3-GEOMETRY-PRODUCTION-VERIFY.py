"""Recheck production exact geometry against retained independent rational cases."""

from fractions import Fraction
from pathlib import Path
from tempfile import TemporaryDirectory
import hashlib
import math
import random
import struct
import subprocess
import tarfile


ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / "crates/thrum-source/src/render_geometry_exact.rs"
ARCHIVE = ROOT / "docs/evidence/render/R0-F3-EXACT-TRIANGLE-PROBE-2026-11-03.tar.gz"
SEED = 0xAA033F3


def bits(value):
    return f"{struct.unpack('>Q', struct.pack('>d', value))[0]:016x}"


def radius_cases():
    randomizer = random.Random(SEED)
    positions = []
    for _ in range(6000):
        positions.append([
            max(-1000.0, min(1000.0, math.ldexp(
                randomizer.uniform(-1, 1), randomizer.randrange(-1074, 11)
            ))) for _ in range(3)
        ])
    for first in (0.0, 600.0, 800.0, 1000.0):
        second = math.sqrt(max(0.0, 1_000_000 - first * first))
        for a in (math.nextafter(first, -math.inf), first, math.nextafter(first, math.inf)):
            for b in (math.nextafter(second, -math.inf), second, math.nextafter(second, math.inf)):
                for tiny in (0.0, 5e-324, 1e-6):
                    positions.append([a, b, tiny])
    return "".join(
        " ".join(map(bits, point)) + " "
        + str(int(sum(Fraction.from_float(x) ** 2 for x in point) <= 1_000_000))
        + "\n"
        for point in positions
    )


HARNESS = r"""
fn main() {
    let mut fast = 0;
    let mut cases = 0;
    for line in std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap().lines() {
        let mut fields = line.split_whitespace();
        let p: [[f64; 3]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| {
            f64::from_bits(u64::from_str_radix(fields.next().unwrap(), 16).unwrap())
        }));
        let expected: [i8; 3] = std::array::from_fn(|_| fields.next().unwrap().parse().unwrap());
        assert_eq!(exact_cross(p), expected, "exact case {cases}");
        if let Some(found) = small_cross(p) {
            assert_eq!(found, expected, "small case {cases}");
            fast += 1;
        }
        let nonzero = expected.iter().any(|&sign| sign != 0);
        assert_eq!(triangle_is_nondegenerate(&p, [0, 1, 2]), Ok(nonzero), "entry {cases}");
        assert_eq!(triangle_is_nondegenerate(&p, [0, 2, 1]), Ok(nonzero), "reverse {cases}");
        cases += 1;
    }
    let mut radii = 0;
    for line in std::fs::read_to_string(std::env::args().nth(2).unwrap()).unwrap().lines() {
        let mut fields = line.split_whitespace();
        let p = std::array::from_fn(|_| {
            f64::from_bits(u64::from_str_radix(fields.next().unwrap(), 16).unwrap())
        });
        let expected = fields.next().unwrap() == "1";
        assert_eq!(position_within_local_radius(p), Ok(expected), "radius {radii}");
        radii += 1;
    }
    assert_eq!((cases, radii), (8_009, 6_108));
    println!("triangle_oracle={cases} integer_fast={fast} fixed_fallback={} both_windings={} radius_oracle={radii}", cases - fast, 2 * cases);
}
"""


def main():
    with tarfile.open(ARCHIVE, "r:gz") as archive:
        triangles = archive.extractfile("cases.tsv").read()
    with TemporaryDirectory(prefix="render-geometry-exact-", dir=ROOT / "tmp") as temporary:
        work = Path(temporary)
        (work / "cases.tsv").write_bytes(triangles)
        (work / "radius.tsv").write_text(radius_cases())
        source = SOURCE.read_text()
        (work / "verify.rs").write_text(source + HARNESS)
        print("production_source_sha256=" + hashlib.sha256(SOURCE.read_bytes()).hexdigest(), flush=True)
        for mode in ("debug", "release"):
            binary = work / f"verify-{mode}"
            command = ["rustup", "run", "1.99.1", "rustc", "--edition", "2024",
                       str(work / "verify.rs"), "-o", str(binary)]
            if mode == "release":
                command.append("-O")
            subprocess.run(command, check=True)
            print(mode, flush=True)
            subprocess.run([binary, work / "cases.tsv", work / "radius.tsv"], check=True)


if __name__ == "__main__":
    main()

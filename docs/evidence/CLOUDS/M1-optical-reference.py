"""Print independent optical-oracle values used by M1's Rust tests.

Run from the repository root with NumPy installed:
python3 docs/evidence/CLOUDS/M1-optical-reference.py
"""
import importlib.util
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location(
    "cloud_reference", root / "scripts/reference/cloud/optical_reference.py")
reference = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = reference
spec.loader.exec_module(reference)
a = reference.beer_segment(0.04, 0.03, 25.0)
b = reference.beer_segment(0.02, 0.01, 40.0)
print("a", a)
print("b", b)
print("a then b", reference.compose(a, b))
print("zero extinction", reference.beer_segment(0.0, 0.03, 25.0))
print("weights", reference.WEIGHTS)
print("extinction", reference.BRANCH_B)
print("later asymmetries", reference.LIQUID_G[1:])

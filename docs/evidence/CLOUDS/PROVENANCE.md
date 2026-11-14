# CLOUDS provenance

M1 copies or translates no file from Broadsides, and introduces no third-party
code or asset. Its specialised CPU authority follows the approved Thrum design
and the local design sketches in [design-proof](design-proof/README.md).

The point phase implements equations 2, 5 and 7–10 of Jendersie and d'Eon's
[2023 supplemental](https://research.nvidia.com/labs/rtr/approximate-mie/publications/approximate-mie-supplemental.pdf),
read directly during implementation. The disc average is quadrature of that
normalised phase. No Broadsides Sun-disc spread rule is used. Beer and later-order
expected values come from the retained Thrum oracle, invoked by
[M1-optical-reference.py](M1-optical-reference.py), with its printed
[values](M1-optical-reference.txt). No GPL or share-alike material was introduced.

# CLOUDS provenance

This is CLOUDS's record of what it copied or translated, from where and under what licence, and the one to
keep current. It was first copied from `PROVENANCE.md` in CLOUDS's working folder,
`.orchestrate/evidence/CLOUDS/`. The `design-proof/` and `M1-` files it names are kept in that working folder.

M2 translates the local CPU weather rules to WGSL and implements the approved
Weather/Shade design. It copies no Broadsides file and adds no third-party asset.

M1 copies or translates no file from Broadsides, and introduces no third-party
code or asset. Its specialised CPU authority follows the approved Thrum design
and the local design sketches in `design-proof/README.md`.

The point phase implements equations 2, 5 and 7–10 of Jendersie and d'Eon's
[2023 supplemental](https://research.nvidia.com/labs/rtr/approximate-mie/publications/approximate-mie-supplemental.pdf),
read directly during implementation. The disc average is quadrature of that
normalised phase. No Broadsides Sun-disc spread rule is used. Beer and later-order
expected values come from the retained Thrum oracle, invoked by
`M1-optical-reference.py`, with its printed
values (`M1-optical-reference.txt`). No GPL or share-alike material was introduced.

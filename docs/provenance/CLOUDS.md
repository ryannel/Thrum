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

## M6 placed volumes

Source repository is Broadsides at commit `79637f78908d2c626846a93bdbb1157aea81c2e4`.
The copied files are original project content under `LicenseRef-Broadsides-Original`.
The source licence is retained beside them. The import is authorised by the CLOUDS port decision.

### Accepted heaped-cloud library

The source column is relative to the Broadsides repository. The destination column is relative to
`examples/v1-game/assets/clouds/`. All files in this table were recovered unchanged from the
Git LFS objects at the pinned commit.

| Destination | Source |
|---|---|
| `LICENSE.txt` | `assets/clouds/heaped-fair/LICENSE.txt` |
| `library.json` | `assets/clouds/heaped-fair/library.json` |
| `shapes/fair-1500-wide/density.png` | `assets/clouds/heaped-fair/shapes/fair-1500-wide/density.png` |
| `shapes/fair-1500-wide/metadata-default.json` | `assets/clouds/heaped-fair/shapes/fair-1500-wide/metadata-default.json` |
| `shapes/fair-low-bank/density.png` | `assets/clouds/heaped-fair/shapes/fair-low-bank/density.png` |
| `shapes/fair-low-bank/metadata-default.json` | `assets/clouds/heaped-fair/shapes/fair-low-bank/metadata-default.json` |
| `shapes/fair-triad/density.png` | `assets/clouds/heaped-fair/shapes/fair-triad/density.png` |
| `shapes/fair-triad/metadata-default.json` | `assets/clouds/heaped-fair/shapes/fair-triad/metadata-default.json` |
| `shapes/fair-head-tail/density.png` | `assets/clouds/heaped-fair/shapes/fair-head-tail/density.png` |
| `shapes/fair-head-tail/metadata-default.json` | `assets/clouds/heaped-fair/shapes/fair-head-tail/metadata-default.json` |
| `shapes/fair-three-crowns/density.png` | `assets/clouds/heaped-fair/shapes/fair-three-crowns/density.png` |
| `shapes/fair-three-crowns/metadata-default.json` | `assets/clouds/heaped-fair/shapes/fair-three-crowns/metadata-default.json` |
| `shapes/fair-open-crescent/density.png` | `assets/clouds/heaped-fair/shapes/fair-open-crescent/density.png` |
| `shapes/fair-open-crescent/metadata-default.json` | `assets/clouds/heaped-fair/shapes/fair-open-crescent/metadata-default.json` |
| `shapes/mediocris-ridge/density.png` | `assets/clouds/heaped-fair/shapes/mediocris-ridge/density.png` |
| `shapes/mediocris-ridge/metadata-default.json` | `assets/clouds/heaped-fair/shapes/mediocris-ridge/metadata-default.json` |
| `shapes/mediocris-lean/density.png` | `assets/clouds/heaped-fair/shapes/mediocris-lean/density.png` |
| `shapes/mediocris-lean/metadata-default.json` | `assets/clouds/heaped-fair/shapes/mediocris-lean/metadata-default.json` |
| `shapes/mediocris-twin-turrets/density.png` | `assets/clouds/heaped-fair/shapes/mediocris-twin-turrets/density.png` |
| `shapes/mediocris-twin-turrets/metadata-default.json` | `assets/clouds/heaped-fair/shapes/mediocris-twin-turrets/metadata-default.json` |
| `shapes/congestus-core/density.png` | `assets/clouds/heaped-fair/shapes/congestus-core/density.png` |
| `shapes/congestus-core/metadata-default.json` | `assets/clouds/heaped-fair/shapes/congestus-core/metadata-default.json` |
| `shapes/congestus-forked/density.png` | `assets/clouds/heaped-fair/shapes/congestus-forked/density.png` |
| `shapes/congestus-forked/metadata-default.json` | `assets/clouds/heaped-fair/shapes/congestus-forked/metadata-default.json` |
| `shapes/congestus-windlean/density.png` | `assets/clouds/heaped-fair/shapes/congestus-windlean/density.png` |
| `shapes/congestus-windlean/metadata-default.json` | `assets/clouds/heaped-fair/shapes/congestus-windlean/metadata-default.json` |

### Accepted congestus growth track

The original replay archive was absent at the location recorded in
`factory/clouds/heaped-fair/review/m2-authored-growth-v2/local-storage.json`.
The key atlases and sidecars were recovered by running the source
`factory/clouds/heaped-fair/route_tests/cook_authored_growth.py bake` with the pinned
`factory/clouds/heaped-fair/recipes/m2-growth-c2-exposed.json` and the accepted C2 parent.
The bake used Python 3.14.10, NumPy 2.4.6, SciPy 1.17.1 and Pillow 12.2.0.
Its freeze, receipt and manifest match the source records byte for byte. Each atlas matches
the source manifest’s PNG and decoded-voxel SHA-256.

Key source paths refer to the `growth/` outputs of that pinned bake. The manifest was copied
unchanged from Git. No Broadsides generator code is part of the Thrum delivery.

| Destination | Source |
|---|---|
| `growth/growth-track-manifest.json` | `factory/clouds/heaped-fair/review/m2-authored-growth-v2/growth-track-manifest.json` |
| `growth/key-000.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-000.png` |
| `growth/key-000.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-000.json` |
| `growth/key-012.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-012.png` |
| `growth/key-012.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-012.json` |
| `growth/key-024.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-024.png` |
| `growth/key-024.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-024.json` |
| `growth/key-036.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-036.png` |
| `growth/key-036.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-036.json` |
| `growth/key-048.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-048.png` |
| `growth/key-048.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-048.json` |
| `growth/key-060.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-060.png` |
| `growth/key-060.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-060.json` |
| `growth/key-072.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-072.png` |
| `growth/key-072.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-072.json` |
| `growth/key-084.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-084.png` |
| `growth/key-084.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-084.json` |
| `growth/key-096.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-096.png` |
| `growth/key-096.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-096.json` |
| `growth/key-108.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-108.png` |
| `growth/key-108.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-108.json` |
| `growth/key-120.png` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-120.png` |
| `growth/key-120.json` | `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/growth/key-120.json` |

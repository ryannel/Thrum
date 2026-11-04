# Complete candidate R0 authored source fixture

3 November 2026. **Reproducible candidate source and CPU checks only.** The
[archive](../ARCHIVE-ACCESS.md#archive-inventory) fills in exact authored bytes
for the [candidate F3 grammar](../../design/research/R0-F3-RENDER-SOURCE-GRAMMAR-2026-11-03.md),
[first-source handoff](../../design/research/R0-FIRST-ADMITTED-RENDER-SOURCE-2026-11-03.md)
and [Sun-shadow recipe](../../design/research/R0-FIRST-SUN-SHADOW-RECIPE-2026-11-03.md).
It is 19,131 bytes, SHA-256
`f29041ee23013d2199e93736345a540a7d2dd4ac56586791c661ab9edbe8502b`.
No production crate, admitted root, SourceRevision, built-in definition hash,
F4 product/build binding or device result is supplied by this experiment.

The archive retains a deterministic generator, independent verifier, six-file
candidate source tree, exact declared-file list, candidate preparation settings,
results, package checker and reproducible packer. The manifest pins consulted
repository inputs and the Python executable/version. The fixture code/data are
original repository research content, with no external assets/code imported.

## Complete declared candidate tree

`core.project@2` lists four record paths and one entry scene; its game-schema
list is empty and its required `core.registry-history@1` has empty field/version
arrays. These spellings match the existing
[constant-light source fixture](../../../crates/thrum-source/src/sqlite_store.rs)
and [project parser](../../../crates/thrum-source/src/project_v2_json.rs).
The scene selects one environment and one render source through
`environment.source` and `render.source`. The render root has no second
active-environment selector. The environment/Sun records preserve the
[existing constant-light grammar](../../../crates/thrum-source/src/environment_json.rs):
Sun-only identity/driver, raw receiver-to-source direction, unattenuated lux,
unit-luminance colour, angular radius, ambient/background and all seven explicit
unsupported quantities. No new environment schema or light identity meaning is
invented.

| Candidate source file | Bytes | SHA-256 |
| --- | ---: | --- |
| `project.json` | 385 | `d755b16408e28a74ae620f614eb20ec3847c932f38e433c5a2b08474b9670c56` |
| `records/environment.record.json` | 563 | `562dd1c6f30c17d831984030a0b574747c9d07233c515bdfca90558e941dfd8c` |
| `records/render.record.json` | 4,008 | `ed502194e1e488a563c8c321b3445d3f2299dbea85aeb9837e6c6de6e3bf2467` |
| `records/sun.record.json` | 142 | `d56d5c1d35b49ad2a084c07db7767e283a0c4e04b203c88f62d9dc8a1602f0f2` |
| `scenes/main.scene.json` | 269 | `214a3b9e98fafb2483dd479ed231d56e59337d54c89354e54e0319cd40e126e3` |
| `schemas/history.registry.json` | 143 | `54fe2a36d26dbf1e1c5c7800288b3466979030fb104050a9afff74f870edf427` |

The tree is 5,510 bytes. All 24 UUIDs are canonical, globally distinct and
assigned deterministically under `a6b9c300-0000-4000-8000-…`; 18 are embedded
camera/mesh/instance/material/texture/local-light identities. The independent
edge list contains 16 required typed references including the project's entry
scene. Project file membership and embedded containment are not additional
required-reference edges. The current `StagedRecords::required_edges` source
capacity excludes the project entry-scene edge; adding scene@3/render@1 under
that existing unit would charge 15 summaries for this fixture while still
checking all 16 graph edges. The external `declared_tree.json` is a prospective
capture caller's complete list, not a Foundation TreeManifest.

## Authored content and mask expectations

The source contains exactly three identity-transform, one-sided +Z quads with
six triangles: receiver `[-3,3]×[-2,2]@z=0`, masked fence
`[-0.6,0.6]×[-1,1]@z=1`, and offscreen caster
`[3.4,3.8]×[-0.4,0.4]@z=2`. Vertex order is bottom-left, bottom-right,
top-right, top-left; `triangles` is `[[0,1,2],[0,2,3]]`, and UVs are
`[[0,1],[1,1],[1,0],[0,0]]` for top-left image origin.
The source camera is `(0,0,5)`, identity orientation, decoded FOV
`1.0471975511965976` radians, near/far `0.1/20` metres and exposure `0.0001`.
The Sun is `(0.6,0,0.8)` at 100,000 lux; ambient is RGB 1,000 lux and
background black. Four white 1,000-cd lights occupy `(±1,±1,3)` with `0.1 m`
radius. These are fixture choices, not universal capacities or lighting limits.

Four exact images total 100 decoded bytes: 2×2 sRGB receiver base, 4×4 sRGB
fence base/alpha, 2×2 shared linear ORM and 1×1 sRGB emission. Every decoded
byte is retained as base64 in source and as hex plus SHA-256 in results.
Three materials exercise dielectric and metal factors, base/ORM/emission
composition and one mask. Effective minimum roughness is about `0.40157`,
`0.6` and `0.40157`; there is no hidden change to the public `0.045` minimum.
There are three optional authored texture **slots per material**, with four
unique images and five material-to-texture edges in this scene. A scene-wide
three-image reading of the candidate product ABI would contradict the grammar;
it must mean slots per material or explicitly select a narrower F4 recipe.

All material samplers use equal nearest min/mag filtering and clamp-to-edge.
The fence's alpha factor is 1 and cutoff `0.5`. Its upper-left-origin rows
are `255,0,255,0 / 0,255,0,255 / 255,0,255,0 / 0,255,0,255`.
At the sixteen normalized texel centers `((i+1/2)/4,(j+1/2)/4)`, the CPU
`alpha >= cutoff` expectation keeps exactly eight texels, those with even
`i+j`. The results enumerate all centers and decisions. Separate auxiliary
scalar controls refuse `127/255`, keep `128/255`, and keep exact equality
`1×0.5 = 0.5`; these are not extra texels in the saved source. No device
filtering or depth/shadow/forward coverage claim follows from CPU checks.

## Independent checks actually run

`verify.py` does not import `generate.py`. It checks exact file membership,
strict decoded duplicate/unknown keys, canonical global UUIDs, typed closure,
finite numeric domains, exact decoded-binary64 local-radius and triangle
predicates, material/texture transfer, canonical padded base64 and dimensions.
Identity-array reordering preserves its identity/edge result. Seventeen
negative mutants refuse at their named gates, including global collision,
wrong-type/dangling edges, rival environment selector, missing cutoff, null
present map, invalid/noncanonical base64, overflow-shaped dimensions,
exact degeneration, a rounded-radius trap and an escaped duplicate key.
This fixture-oriented checker is independent evidence, not the production
Rust decoder, an F3 conformance suite or proof of all general valid sources.

Fraction arithmetic proves all six triangle normals point +Z and all 36
Sun/camera/four-local source predicates are positive. Twelve exact rectangle
closest-point checks remain outside emitter radii; the minimum squared
distance is about `4.16 m²`. On this host, binary64 `tan` gives a caster-plane
camera half-width of `3.079201435678003 m`, below minimum caster x `3.4 m`.
Exact raw-decoded Sun ratios project its receiver shadow to x approximately
`[1.9,2.3]`. A full Sun negation reverses every source component; the
resulting travel direction points away from z=0. All four caster-corner
intersections have exact negative ray parameter
`−9007199254740992/3602879701896397` (about −2.5), so the forward-ray routine
refuses them before using a projected point. An unchecked infinite-line control
would misleadingly recover the original receiver interval behind the ray origin.
The separate **horizontal-ray-sign** mutant reverses only travel's x component,
retains negative z and projects forward to `[4.9,5.3]`, outside the receiver.
The camera-width result is a host floating-point observation; the ray ratio is
not a substitute for the future normalized F4 Sun product/basis.

The retained package was freshly extracted, every member compared byte for
byte, the generator and verifier rerun, all protected hashes rechecked, and
repacked to exactly the same archive bytes. Generator SHA-256:
`6a99df293848eb0be79ace0743c3bfde572ef1dffd8d50c143537279c130b997`;
verifier: `6f5b01c80f6852519c069f413a3aad317189cc6b91a1c635f0a90a95cbc10234`;
results: `95905634102cd7c7533c5d05bbbb1dbe6fe443c3f45780950bd17f0901cbdabc`.
Extract the archive and follow `REPRODUCE.md`; only Python 3.11+ standard
library is needed. `./dev check-docs` is the repository documentation gate,
separate from these CPU checks.

## Unresolved gates

The field spellings are now concrete candidate bytes, but F3/F4/W1/V2/V3
adoption, immutable definitions/evaluator hashes, typed decoder projections,
complete declared-namespace admission and actual accepted-source pin remain
open. The history does not fabricate hashes for proposed built-ins. No new
universal source cap is selected.

F4 must cook and authenticate the 160-byte constant-light product and all
mesh/material/texture/camera/local-light products, derive normalized Sun/basis
and exact physical/representation certificates, verify byte/ABI reflection and
publish a complete build binding from one admitted pin. The external candidate
recipe records viewport 1280×720, storage scale 1/1024 and the proposed fixed
2048² light box, while leaving `b0_m`, slope coefficient, grazing cutoff and
maximum contact detachment **null**. Those decisions cannot be silently filled
from this fixture or the prior finite numeric probe.

A named target must still prove front-face/culling, shared masked
shadow/depth/forward samples and cutoff boundaries, linear-light sRGB handling,
four shadow comparisons, packed source error, raster/D32 bias adequacy,
contact/edge quality, moving camera/origin and retained-map certificates,
lease/device-loss behavior and assembled CPU/GPU/residency/whole-frame cost.
No image capture or target measurement was run here.

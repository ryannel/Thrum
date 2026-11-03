# Raw 03 natural-lobe diagnostic — no component selected for cooking

3 November 2026. This is source inspection only. No mask, density volume, EDT, renderer substitution, candidate asset or GPU run was made.

## Pinned input and method

- Raw scalar: `../cloud-authored-master/cloud-03-256.density.r16f`, SHA256 `42431f9da2d76e253d649cc35802fd1be7b23ddb698e73c6c4247ac3c90a97c9`, little-endian R16F `[Z,Y,X]`, `256³`, 2 m local pitch, centres `(2i+1,2j+1,2k+1)` m. Metadata SHA is in `diagnostic.json`. Original active support is XYZ approximately `[99,175,25]` to `[413,337,487]` m; the broad XZ bank cannot be called one isolated crown.
- `raw_diagnostic.py` streams four Z slabs from a read-only mmap. Native front integrates along Z, side along X, top along Y, with `κ=.019/m` only for radiographic display. The `.041` panels and XZ slices are threshold diagnostics, not an optical asset. Orange boxes show the **old** B domain, cyan dots its existing root/head. `component_slices.py` reads only 2D native slices and reports the 4-connected `.041` component touching the known head. Neither script alters the master or applies a candidate mask.
- Actual process high water: 100.69 MiB for 16 annotated views, 40.63 MiB for slice connectivity; both below the planned 128 MiB. Output files together are under 0.3 MiB. The first script took about 0.33 s; no EDT or GPU time is included. The scripts do not allocate a whole decoded 3D float master.

## Coordinate evidence

Inspect [front](front-opacity.png), [side](side-opacity.png), [top](top-opacity.png), and [front `.041` support](front-peak041.png). The existing B box `[104,296]×[160,352]×[128,320]` m encloses the positive bridge, but it crosses naturally extended raw material. In front projection the raw crown is a bank from roughly X100 to400, with several upper scallops. Side/top projections show long Z branches and another mass to the southeast. The apparent front-facing head is not a self-contained 3D lobe.

The most promising *diagnostic target* is the natural upper-left head around the existing positive root `[225,265,197]` and head `[221,295,207]` m. The frozen raw control reports `headρ=.11627`, raw path minimum `.11145`, 23 path vertices, a guarded raw `.041` head ball 17.42 m and path-vertex ball minimum 15.93 m. These are a rooted connection in the **uncut** master, not proof for a future separated component.

At Y=301 m the 4-connected `.041` slice component containing `(221,207)` in XZ has 2,154 native cells and bbox X131–271/Z171–261 m. At Y=311 it has 1,440 cells, X147–253/Z175–257; at Y=321 it has 892, X155–247/Z183–249. Those three high slices expose the head's irregular natural outer contour. [Y300](slice-y300-seam.png) and [Y320](slice-y320-seam.png) show it beside a distinct right/depth mass. **At Y=295, however, the head's 4-connected `.041` component already reaches X353/Z79** (4,691 cells), and at Y=265 it reaches X353/Z83. The upper lobe rejoins the broad raw bank before its known root. The switch from isolated upper silhouette to joined lower bank is real source topology, not an image-framing accident.

The diagnostic diagonal `X−Z=80 m` separates the two upper islands at Y321 (zero `.041` samples on that line), but intersects material at Y300 (17 samples, peak `.172`), Y280 (30, peak `.212`) and Y260 (67, peak `.243`). [Y260](slice-y260-seam.png) and [Y280](slice-y280-seam.png) show those crossings. Orange `X−Z=120` is another *visual guide*, not a second candidate. A fixed diagonal or axis-aligned crop would create a high-density planar selection face through the lower bank; the known old B box already crosses raw support on four faces in the v3 diagnostic. Neither is a certified hidden seam.

## Selection decision and exact remaining gap

Preserve the natural upper-left contour above about Y301 and keep the proven root/head lineage as the **one region worth testing**, but **do not cook a separated crown yet**. An exact seam rule has not been established from this source alone. It must keep raw density and its native outer support unchanged on the visible upper head; separate its southeast branch through a measured 3D low-density valley; retain a positive connection to `[225,265,197]`; and place any unavoidable lower cut entirely inside mapped BODY opacity for both actual scale-5/6 parts at their source220 poses. The fixed diagonal fails the raw-empty test. The existing old capsule owns the outline and defeats the purpose.

The missing input is a 3D seam/visibility certificate: raw density along a proposed separator *and* mapped BODY-versus-crown opacity at that same seam in front, side and oblique actual tuples. A future single candidate needs this before writing a selection mask or choosing a new domain. If no such hidden seam exists, this master offers a convincing **whole-bank Cu ingredient** but no demonstrated standalone natural crown from these coordinates. The finite active height is about 162 m local; it cannot supply a much taller tower without the separate live component map/growth, and that map is outside this diagnostic.

The attached JSON retains each native height's mass, `.041` support count and X/Z span, slice component boxes, seam samples, source pins and measured peaks. Native stills cannot establish lifecycle motion, post-taper attachment, optical beauty, repeated variety or art admission.

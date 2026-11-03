# Covered Cb: anatomy, lighting and optical calibration discrimination

3 November2026. **The covered source remains rejected for appearance.** This preserves an actual fixed-density comparison, not an accepted Cb, source lifecycle or production atmosphere renderer. [CLOUDS](../../design/modules/CLOUDS.md) and the single [migration queue](../../design/modules/CLOUD-MIGRATION.md) govern further work.

## Reproduce the exact comparison

```sh
python3 docs/evidence/cloud-light-diagnosis-2026-11-03/restore.py --output tmp/cloud-light-review
python3 tmp/cloud-light-review/viewer/serve.py --port 8154
```

Open `http://localhost:8154/`. The default restored viewer is **daab547852af225b**, actually loaded at8152 in the Mac in-app browser. Choose Selected production,384ray steps,25km,Oblique,Sun30° andκ=.019 for the calibrated low/high comparison. High128×160×128/Sun128 builds4Zslices per fenced stage and then stops; it is a slow reference, not a shipping profile. Current32×24×24/Sun32 is the inherited coarse cache. Source-normal mode bypasses optical lighting. First/extra/ambient modes isolate one cloud-source term while clear air, fog and ground remain; they were implemented but not all visually inspected in this checkpoint.

`--variant f06` restores **f06bd77e72c5d79d**, the original covered-source/legacy-light viewer. `--variant 0c3` restores **0c3e028a5bf5622c**, retained but not browser-run because it initially inherited the legacy profiles. All destinations must be fresh. The source/research ZIP contains exact three-viewer code, shaders, small metadata, rights/notices, complete source cook/controls/metrics, compact failed cooks, and CPU lighting reproduction/results. The compressed32MiB scalar is retained here; its decoded SHA is `377e8f9def4ff30ac38c02266b8666ae5c592d59f9a36820c62731fb52d89eef`. There is no required ignored-only scalar or live scratch dependency in restoration. A fresh full recipe recook has not been verified bit-for-bit; exact binary restoration has.

The archived `sources/daab/verify.py` is historical: its sibling scratch paths and same-inode assertion do not apply to a standalone restoration. Do not use it as the portable verifier. The outer restore checks package hashes, source bytes, every served hash and all three aggregate identities through the retained restore tool. The portable `node tmp/cloud-light-review/viewer/tests/profile.test.mjs` checks selected/legacy/first-only uniforms and invalidation. Independent review reconstructed all three actual served identities (53/53/50files), resolved JS imports and ran that Node test.

## What actual images establish

Every image below uses the identical scalar and1280×720,384ray steps,25km oblique,Sun30°. Each adjacent JSON pins the actual loaded source, uniforms, quality and allocation facts.

| Held comparison | Actual result |
| --- | --- |
| [Selected lowκ=.0045](observations/light-low-selected-cb-covered-256-oblique-mode0-daab547852af225b-976f7489-1.png) vs [selected highκ=.0045](observations/light-high-selected-cb-covered-256-oblique-mode0-daab547852af225b-976f7489-3.png) | Finer light changes little of the smooth club/pear outline; it does not repair anatomy. |
| [Source normals atq≈.03](observations/light-low-selected-cb-covered-256-oblique-mode7-daab547852af225b-976f7489-2.png) | Many small surface wrinkles/lobes exist beneath the soft beauty. The cap exposes parallel tubes; coverage of the original scaffold does not prove final cauliflower form. |
| [Selected highκ=.019](observations/light-high-selected-cb-covered-256-oblique-mode0-daab547852af225b-976f7489-5.png) vs [κ=.038](observations/light-high-selected-cb-covered-256-oblique-mode0-daab547852af225b-976f7489-6.png) | .019 reveals materially more internal contrast; .038 mainly darkens. Both retain the narrow pointed base, club body and insufficient anvil. .038 is not selected. |
| [Selected lowκ=.019](observations/light-low-selected-cb-covered-256-oblique-mode0-daab547852af225b-976f7489-7.png) vs [selected highκ=.019](observations/light-high-selected-cb-covered-256-oblique-mode0-daab547852af225b-976f7489-5.png) | Higher cache quality changes internal light accuracy/contrast, but anatomy still fails. |
| [Legacy lowκ=.019](observations/light-low-legacy-cb-covered-256-oblique-mode0-daab547852af225b-976f7489-9.png) vs selected low | Legacy(1,.25,0) fills the shadowed body more strongly than the baseline-selected(.5,.5,.5). This named optical difference is not a source change. |

The qualified authored Cu master was inspected withκ=.019; the covered-Cb viewer had inheritedκ=.0045. At peakq≈.14697 this gives an interior mean free path≈1,512m, compared with≈338m median lobe radius. At.019 it is≈358m. Raw VDB/authored q is not a measured physical water-content unit: source profile calibration must accompany its geometry/scale. This explains some washed-out detail but cannot supply the absent broad base, unequal rising towers or a continuous broad cap. The [calibration analysis](research/optical-calibration.md) separates local optical depth from saturated whole-column depth.

## Accuracy and cost are separate

The [actual16-point comparison](point-comparison.json) at matchedq≈.03 surface positions,Sun30°,κ=.019 and selected scattering gives:

| Cache | Mean absolute ordinarySun T error vs direct512 | Maximum |
| --- | ---: | ---: |
| Low32×24×24 / Sun32 | .0445134 | .174833 |
| High128×160×128 / Sun128 | .00798831 | .0291592 |

Direct256vs512 maximum on those same points is.000459015. The earlier highκ=.0045 probe is separately labelled; it is not the comparator for theκ=.019 low probe. These16points do not establish full-field/image/ambient accuracy. A distinct [2,048-point CPU study](research/lighting-coupling-review.md), atκ=.0045, also implicates spatial cache loss; its full recipe/results live in the archive. The source itself passed actual aliased-R16 GPU upload/phase-default checks in the [upload receipt](observations/upload-cb-covered-daab547852af225b-976f7489-10.json).

Actual high/low probe receipts count65,732,812/44,908,556 logical GPU bytes. These are not process/device residency. The high producer has roughly264times the coefficient-query count of the low producer before view work; it is a discrimination reference with staged submissions, not a performance fix or5ms admission. No M2,512MiB,full-sky,temporal-cache or complete-air claim follows. The source cook separately recorded763.75MiB simultaneous parent/worker RSS under its768MiB offline allowance; that is not runtime cost.

## Design consequence

Keep shared optical query semantics and the selected scattering profile. Correct source calibration explicitly. Do not hide the anatomy deficit with extinction or an expensive cache. The [pre-cook structural card](research/next-cb-anatomy-decision.md) replaces the pointed pear and uniform small-lobe skin with a broad level condensation base, unequal rooted towers, separated major cauliflower lobes and one continuous, tapered downwind anvil. This card is a challenger, not measured geometry or selection. Root commissioned only low-memory source-only analytic previews before another256³density cook; those previews and any later live phase/weather programme need their own acceptance.

The primary references and explicit deductions are recorded in the card and [lighting review](research/lighting-coupling-review.md): WMO/NWS anatomy, SideFX filled-volume/billowy/wispy operations, and Guerrilla's public2023Nubis³deck. No private studio implementation or2026device performance is inferred from them.

# Frozen storm anatomy: two rejected inputs

3 November 2026. **Neither source passes Cb appearance.** This bounded content test retained the renderer and changed the actual three-dimensional density. It did not build a lifecycle, phase model or production source representation.

The first assembly (`f30d926dcc6695fc`) joins qualified CC0 Cu ingredients into an irregular base, unequal towers and an upper cap. Its numerical connection passes, but actual oblique and side images reveal a skeletal body, a large central opening and thin connectors. Raising ray count from 96 to 384 does not fill that missing anatomy. A still earlier base-gap failure is retained as a compact recipe, projection and metrics.

The second assembly (`0ab02b3a38d03427`) adds occupied bulk and two broad outer volume members at unchanged optical calibration. It closes the central opening but exposes a smooth column. Detailed peripheral ingredients do not establish complete surface coverage. This also fails; do not continue this cook with density, opacity or noise tuning presented as a solved cloud model.

| Actual input / view | First assembly | Filled assembly |
| --- | --- | --- |
| Oblique, native 384³, 384 rays, Sun 30°, 25 km | [Skeletal body](observations/master-cb-anatomy-384-oblique-mode0-f30d926dcc6695fc-76fa7225-2.png) | [Smooth bulk](observations/master-cb-anatomy-384-oblique-mode0-0ab02b3a38d03427-80842a3a-1.png) |
| Side, same settings | [Thin body](observations/master-cb-anatomy-384-side-mode0-f30d926dcc6695fc-76fa7225-3.png) | [Exposed column](observations/master-cb-anatomy-384-side-mode0-0ab02b3a38d03427-80842a3a-3.png) |

The parent opened these actual native images. This is a static appearance rejection, not watched motion or family acceptance. Both samples have a recognizable independent reflected shape and receiver attenuation, but neither supplies a good storm, ice optics, complete coupled lighting or a full-sky performance result.

## Geometry, fidelity and numerical scope

The 384³ field spans 12 × 14 × 12 km, with pitch 31.25 × 36.4583 × 31.25 m. Its 256³ comparison is an overlap average followed by half rounding. Gaussian ingredient prefiltering is an approximation; transforms, scalar gains, filtering losses, source hashes and original recipes are retained. These fields are static liquid ingredients. No cloud-specific lighting is baked into them.

The first assembly's body-to-core-to-cap connection survives a relative peak threshold of 0.1; its bottleneck bracket is approximately 0.1371–0.1383 of peak. Nevertheless, a fixed central volume has only 23.57% occupancy above 0.1 peak. Connectivity is insufficient evidence of a filled convective body.

The second assembly raises that fixed-volume occupancy to 99.36% and total density sum to 1.631 times the first, at unchanged peak density and κ = 0.0045/m. The central front chord at 6 km grows from 0.50 to 3.094 km; its optical depth grows from 0.293 to 1.694. Upper-cap half voxels above 9.9 km remain identical. These diagnose the missing bulk, but the actual image still rejects the surface. [Native slices](cook/v2/native-central-slices.png), [line cuts](cook/v2/native-linecuts.png) and [structured diagnostics](cook/v2/bulk-diagnostics.json) preserve the distinction.

The actual final GPU upload receipt compares **every native scalar half voxel** in eight-slice chunks: zero differences, exact density sum, aliased snapshots, and sampled G=0/B=0/A=1 defaults. Requested staging uses 2,359,328 bytes and retires afterward. This proves the supplied field reached the GPU; it does not accept the field's art. The parent renderer's internal ancestry label remains `6045d1ce1c9ccebe`; the frozen `adb13e8a53d0bd95` optical/lighting/view/shadow/reflection math is byte-identical in both instruments. Exact recovered payload identities are authoritative.

The final 384³ instrument reports 124,600,332 logical GPU bytes (about 118.83 MiB), including a 108 MiB dense source. This is a diagnostic layout, not a per-cloud production allocation or the 96 MiB source-pool admission. Final cook peak process RSS is 531,988,480 bytes, narrowly below its 512 MiB task limit; no portable memory margin or device performance is claimed. No timing benchmark ran.

## Exact recovery and authoring inputs

The two `instrument-*.zip` archives preserve all source, metadata, density payloads, scoped tests and original license bytes. [Build records](builds.json) pin every archive member; compression changes no decoded bytes. Recovery needs Python standard library, plus Node to run the retained tests:

```sh
python3 restore.py --build 0ab02b3a38d03427 --output /tmp/cloud-cb-v2 --test
cd /tmp/cloud-cb-v2
python3 serve.py --port 8149
```

Use a fresh output directory. The same command with `f30d926dcc6695fc` restores the first assembly. The server redirects to its exact build URL, refuses changed payload identities and binds only localhost. [Recovery results](recovery.json) record actual restored identities and test results; no running `tmp/` experiment is required.

The editable cook sources and diagnostic recipes are in [cook](cook/). Original CC0 inputs, rights and native masters belong to [the selected master package](../cloud-authored-masters-2026-11-03/README.md); [input hashes](cook/source-inputs.json) identify 03/256 and 08/256. They are dependencies, not copied new masters. To recook in a fresh scratch directory, place the `cook/v1` contents at its root, copy `cook/v2` into its `v2` child, restore `inputs` at both the scratch root and its `v2` child from those exact master files (immutable hardlinks are sufficient), and restore the first instrument at `negative-f30d/instrument`. Copy the v1 metadata to `negative-f30d` as required by the original v2 recipe. Follow the original tool versions in each metadata file; exact cross-platform recook identity is not certified. Byte-exact archive recovery is independent of those numerical tools.

The [canonical queue](../../design/modules/CLOUD-MIGRATION.md) remains active. Static Cu input and local erosion are qualified only in their named domains. Believable Cb anatomy, attached growth, indefinite renewal, tower-fed ice and full-sky costs remain open.

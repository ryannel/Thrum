# APPLE-TILE M1 evidence

The builder implemented generation recipe selection and graph-derived attachment storage on the M3 Max.
Portable remains the default. `THRUM_RECIPE=apple-tile` requires actual Metal, unified-memory and transient-storage
facts. Invalid/unsupported requests fail before preparation. Enabled features remain distinct from adapter offers,
and display facts stay with the platform. Selection, mutation, actual storage, replacement, scoped Clippy, Wasm
and workspace tests pass. The milestone check passes D1.4's named moving-route command. [Check records](checks).

Both native recipes match all retained M0 route bytes and sample/exposure identities. Thirty route comparisons
cover SDR/linear and portable repeats, and eight 1920×1080 comparisons cover Sun/night. The final diagnostic build
passes another 20 route comparisons. Sources, binary, shader/content/route hashes, registry digests and Metal
`Texture::as_hal` storage logs are retained. [Native manifest](native-manifest.json.gz),
[comparisons](native-comparisons), [final probe](final-native.json.gz), [build provenance](final-provenance.json).

At the 1280×720 fixed route extent, two frame slots retain the following unique graph attachments.

| Recipe | Textures | Nominal bytes |
| --- | ---: | ---: |
| Portable | 12 | 66,224,128 |
| Apple tile | 14 | 73,596,928 |
| Both sets retained | 26 | 139,821,056 |

Each Apple slot adds discard-only day depth alongside stored depth sampled by stars. Prepared capture variants share
these textures. Fallback-only target bytes are zero. No stored twin exists solely for a force override. Tests cover
SDR/linear, day/star and frustum/occlusion, with unchanged pipeline/texture counts through prepared switches.
Adding readers, explicit stores, history/persistent/imported ownership or capture-only reads disables transient
eligibility only in the affected variant. Locked wgpu's single-layer/mip restrictions are enforced.

The complete cooked-host probe flushes preparation uploads with capture OFF. Metal reports 86,130,688 bytes for
portable, 171,032,576 while both complete generations remain alive, and 86,163,456 after old-host retirement.
These driver totals include assets, buffers and aligned allocations. Capture-enabled route setups report
92,700,672 bytes for either recipe, including their diagnostic surface control. The table retains its nominal
attachment meaning. Performance and physical bandwidth remain unmeasured. [Overlap measurement](residency-overlap.json.gz).

Chrome's day/night SDR/linear comparisons each pass all 481 full frames, totaling 1,924 exact comparisons.
Linear pairs include exact GPU exposure words. The earlier M0 SDR baseline remains authoritative. Added linear/night
baselines use a build restoring every tracked renderer source/manifest from `a6761604`, plus the browser capture
extension. Midnight is selected before route installation on both builds. The actual Wasm tree excludes native Metal
bindings. Separate realtime sequences retain engine motion. The inherited midnight local-light fixture saturates
nearby SDR surfaces equally on both builds. [Chrome evidence](chrome).

The iPhone 18 Pro / iOS 27.2 simulator reports Metal, unified memory false, Portable and registry digest
`f0bba45f1b65747d`. Its 109.46-second diagnostic run passes stalls, hidden/replaced surfaces and background/foreground
lease races, preserving the paused world tick. Simctl images prove endpoint/function within those observations.
Physical iPhone, cooked-scene and simulator submitted-motion qualification remain open. [Simulator evidence](ios).

The builder inspected full frames, native crops, mirror/local-light and cut checkpoints, Sun/stars/shadows and
recorded motion sequences. Lead review is owed. T2 fixed/still supplies exact checkpoints and realtime supplies
host-time motion. The retained 1x movies are viewing aids. Panel cadence and promotion remain open. [Motion](motion).

Reproduce native runs with `scripts/apple_tile_m1.py` and simulator function with `scripts/apple_tile_m1_sim.py`.
`apple_tile_residency` measures complete preparation overlap through the real game facade. Chrome uses
`scripts/capture_v1_browser.mjs` and `scripts/apple_tile_m1_compare_browser.py`. Run GPU commands under the repository
lock. The manifests preserve arguments. Bulk lossless sources, qualified binaries and pre-change Wasm remain in
owned `tmp/g3t-m1` through review, without remote backup. [Retained-file hashes](retained-files.json).

M1 is claimed for lead review. The initial migration gate failure was resolved by rebasing onto repaired main
`8a4d4f0c`, preserving planner changes. Both the named moving command and the complete `./dev check` pass in the
rebased milestone check with zero failures. The intervening main changes leave renderer/shader/game inputs and M1
criteria unchanged. [Final milestone check](checks/rebased-milestone-check.json.gz).

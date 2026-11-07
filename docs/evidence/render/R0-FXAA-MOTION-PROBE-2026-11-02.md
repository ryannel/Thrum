# R0 FXAA moving-input and timestamp probe — 2 November 2026

**Evidence level:** isolated synthetic native-size Metal output and a rejected
GPU timing method. This is not a complete R0 frame, HLSL-reference equivalence,
moving-content visual acceptance or device-performance qualification.

The [source archive](../ARCHIVE-ACCESS.md#archive-inventory), SHA-256
`72d8603f67f20d9ed78f516c55f753daefb77e6d92b8dba919aad4c9bea29bb1`,
contains an independent encoded-image generator, the same pinned FXAA WGSL
port (`fxaa.wgsl` SHA-256
`5f26af44c01e3e605c7822b1881b4200ec8bebb6dfcdbabd41fe850cacb64384`),
Cargo lock and source notices. Rust 1.99.1 built it offline with `wgpu 30.0.1`
and Metal. Direct `naga 30.0.1` WGSL parse and validation with empty optional
capabilities passed for both shader modules before GPU creation. The reported
adapter was Apple M3 Max. It generated pairs at
1280×720 and 1920×1080, moving diagonal, thin-stroke and narrow-line content
by 0.37 pixel between frames, then ran the selected quality-12 FXAA pass.
The synthetic input is already display encoded with alpha luma; it is not a
rendered game scene. Running the extracted source requires a Metal device and
`RUSTC_WRAPPER= rustup run 1.99.1 cargo build --offline`, followed by
`target/debug/r0-fxaa-motion`.

| Size | Output A/B | Raw RGB PPM pair (gzip SHA-256; decompressed SHA-256) | Exact RGB change between pair |
| --- | --- | --- | --- |
| 720p | [A](R0-FXAA-MOTION-720P-A-2026-11-02.png) / [B](R0-FXAA-MOTION-720P-B-2026-11-02.png) | [A](R0-FXAA-MOTION-720P-A-PPM-2026-11-02.gz) `86543294ddf71aa1c9e0c5dc9384bdcba0b5fc704a33cbb8b52a107d3ce665a2` / `b478bea651ca82335b8863ff99639be857c06cedbf9623c88943fa635ffed0e1`; [B](R0-FXAA-MOTION-720P-B-PPM-2026-11-02.gz) `d45c10287912816b18721896dcc91b64beab069bb7d66d7e080611f126fdf975` / `d8b8e6666e95beeb64725717e75bc71bf93e5f0e66aa5436368629c9e19601a1` | 67,901 / 921,600 pixels changed; largest channel difference 118/255 |
| 1080p | [A](R0-FXAA-MOTION-1080P-A-2026-11-02.png) / [B](R0-FXAA-MOTION-1080P-B-2026-11-02.png) | [A](R0-FXAA-MOTION-1080P-A-PPM-2026-11-02.gz) `06aefde8ffad0d006d11b28c118dd6b84e943bc6e0532dba60cfec93502a5650` / `8017cd899514ec7b6b7f71bf0b60d8a17e61cc667e82fdf082`; [B](R0-FXAA-MOTION-1080P-B-PPM-2026-11-02.gz) `140c47d103683e27f67d76c8fa334b9ef8e98d5ec82739c0c718eb2c3d82643e` / `5861024b5f53c4826f36fd050d40ce8c4209d603446b55b954dd2b533cd09b39` | 148,322 / 2,073,600 pixels changed; largest channel difference 150/255 |

The PNGs are viewing copies from the PPM readbacks. Two held frames establish
that the shader runs at the named sizes and responds to a subpixel translation.
Pixel-change counts are **not** a shimmer or quality score; a real moving
sequence, camera/content motion, native-size inspection and reference filter
comparison remain open. A large channel change can be expected on a narrow,
high-contrast feature and does not itself identify an FXAA defect.

The timing attempt used 32 untimed passes and 64 timestamped passes at each
size, on one input/output pair, with `TIMESTAMP_QUERY` and reported period
1 ns. Across repeated runs, apparent 720p per-pass medians ranged about
0.09–0.84 ms and 1080p medians about 1.3–1.9 ms. In the final run **63/63**
successive begin/end intervals overlapped at each size. The 64-pass GPU spans
were only 10.325 ms at 1080p and 5.500 ms at 720p, inconsistent with summing
the apparent individual durations. Therefore this
query layout does not identify serial single-pass latency or a stable whole-R0
cost; no timing target is marked passed. The next measurement must use a
complete admitted R0 frame, deliberate frame/submission boundaries and GPU
capture or otherwise verified timestamp ordering. Record throughput, critical
path, memory overlap, preparation and sustained frame pacing separately.

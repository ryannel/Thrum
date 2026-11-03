# Standalone FXAA package and next R0 gate — 2 November 2026

**Completed:** a repository-owned [version 1 implementation-input package](fxaa-quality12-v1/README.md), exact upstream notices/provenance, offline validation, fresh independent-reference regression and comparator fault checks. The verified WGSL is unchanged. This is a concrete standalone handoff, not an admitted F4 product or integrated V1/V4 pass. No shared runtime crate, build plan, renderer specification or Broadsides file was changed.

## Package and independent regression

The package contains the [WGSL](fxaa-quality12-v1/fxaa.wgsl), [configuration/interface recipe](fxaa-quality12-v1/product.json), [NVIDIA redistribution notice](fxaa-quality12-v1/NOTICE-FXAA.txt), [archive repository licence](fxaa-quality12-v1/LICENSE-INTEL-APACHE.txt) and [offline verifier](fxaa-quality12-v1/verify_package.py). Source SHA-256 remains `5f26af44c01e3e605c7822b1881b4200ec8bebb6dfcdbabd41fe850cacb64384`. The verifier checks exact source/notice hashes and pinned Naga 30.0.1 WGSL validation. Notices must accompany source and applicable binary distributions; no endorsement is implied.

The retained regression reads this actual package, freshly executes the independently compiled unchanged HLSL5 header's SPIR-V and compares every RGB pixel. It also requires input and reference-output hashes to match the [previous independent captures](R0-FXAA-INDEPENDENT-REFERENCE-2026-11-02.md), verifies reference alpha retains centre luma and package alpha is 1, and rejects a vacuous identity reference. The fixed header SHA-256 is `4834fada78774fc55726adeb717c53ca7324b66820df08ef773515cafb148aef`; SPIR-V is `f6bd37c2096d8ae82deca6a2336e0dc5c5cf08aa7316277ae0c6fc0f1d6cd185`. Header, wrapper, defines and compiled reference are included in the source archive. Compilation remains clang preprocessing → installed NDK glslc HLSL5 Quality12 → spirv-val/Naga 30; compiler provenance is in the earlier proof and the retained source.

Normal package RGB matches at **all 1,978,368 pixels**: asymmetric 64² orientation control, prior 256² masked scene, prior 256² diagonal/thin-stroke/wave fixture and 1280×720 A/B subpixel-motion pair. Non-control reference filtering changes 5,548/6,152/61,028/63,352 pixels respectively. The regression uses RGBA16Float diagnostic output; actual RGBA8Unorm agreement remains established by the earlier independent proof. No new shader changes were needed.

Two deliberately incorrect shader files are generated only in scratch to test the comparator:

| Variant | Orientation differing pixels | Masked | Thin fixture | Motion A / B | Observed comparator exit |
| --- | ---: | ---: | ---: | ---: | ---: |
| Retained package | 0 | 0 | 0 | 0 / 0 | 0 |
| Replace alpha luma with green | 189 | 1,805 | 324 | 22,259 / 12,471 | 1 |
| Flip input UV vertically | 4,096 | 23,048 | 65,536 | 353,264 / 356,862 | 1 |

These failures exercise independent colour/luma and orientation assumptions. They are not shipping options. The shader, reference and fixture hashes constrain what ran; the comparator does not derive expected pixels by reimplementing the WGSL formula.

## Isolated pass and logical target cost

The M3 Max/Metal probe used the retained shader, one 1280×720 or 1920×1080 RGBA16Float encoded input and a separate same-size RGBA16Float FXAA attachment, clamp/linear/mip0 sampling, one fullscreen draw, no blending/depth/history. The input is the deterministic motion diagnostic held at 0.37-pixel translation; input generation is completed **before** measurement. This is an isolated FXAA pass, not tone→AA→UI→output or an R0 frame.

To address the previous overlapping-query failure, every sample has its own command submission and a blocking completion before the next. Each submission records pass begin/end, resolves the two queries and maps a 16-byte readback. Runs use 16 warmup and 64 measured samples at each size. Raw ticks, CPU encode/submit time, submit-to-completion time, overlap flags and invalid-pair flags are retained. The latter wall interval includes synchronization/query overhead and is not a frame-time measurement. The shader/product identity is constant across runs; host Clippy cleanup only replaced equivalent byte-iteration spelling and an unnecessary print-index cast.

| Independent run | 720p measured valid/nonoverlapping | 720p GPU interval median / p95 / max, µs | 1080p invalid measured pairs | 1080p invalid warmup pairs |
| --- | --- | --- | ---: | ---: |
| A | 64/64 | 83.1045 /96.667 /122.750 | 2/64 | 0/16 |
| B | 64/64 | 83.5835 /98.083 /100.917 | 1/64 | 0/16 |
| C | 64/64 | 85.6040 /94.166 /96.458 | 0/64 | 1/16 |

All 192 measured 720p pairs are valid/nonoverlapping. These are **isolated timestamp observations**, not accepted assembled-graph cost, sustained game pacing or a transferable target budget. At 1080p, end-before-begin pairs occurred despite serial completion; one warmup end was zero. The method is therefore **unqualified at 1080p**. Valid-subset statistics in the result JSON remain diagnostic and are not accepted latency. Do not silently discard bad pairs and report a passed profile.

A later [1080p measurement diagnosis](R0-FXAA-1080P-TIMESTAMP-2026-11-03.md)
separates counter resolution into a second completed submission and validates
its finite selected capture. The same-submission method documented here
remains rejected.

Exact 1080p invalid diagnostics are retained in raw logs. For example run A/sample53 reported begin `2172894190552083`, end `2172894189598916`; run B/sample33 reported begin `2172894439528333`, end `2172894438614333`. Run C/warmup−16 reported begin `2172894640495250`, end 0. An earlier provisional subtraction panicked on such a warmup pair; the retained probe uses checked subtraction, emits NaN plus an invalid flag and preserves the raw clocks. The timestamp failure's platform/driver cause is unresolved; no further 1080p investigation was pursued in this slice.

The **nominal pixel payload**, excluding driver tiling/allocation and metadata, is:

| Extent | Borrowed encoded input8P | Additional FXAA target8P | Simultaneously live sum16P |
| --- | ---: | ---: | ---: |
| 1280×720 | 7.03125MiB | 7.03125MiB | 14.0625MiB |
| 1920×1080 | 15.8203125MiB | 15.8203125MiB | 31.640625MiB |

This probe additionally owns a 16-byte input-generator uniform,256-byte query-resolve buffer,16-byte timestamp readback and two-query object; shader/pipeline/driver allocations are not measured. It does not allocate an R0 shadow map, depth, HDR scene image, UI, final output, upload arena or retirement overlap. Those costs cannot be inferred from this sum. In the future AA-off recipe, omit the AA pass and its extra output target; the input can remain for another display consumer. That recipe's integrated resource retirement is still unimplemented.

## Focused moving-capture inspection

The retained package regenerated the deterministic fullscreen-raster A/B inputs, separated by0.37 pixel, and filtered them. The same named-device image bytes also match the prior independent reference. A focused [native-pixel A/B crop](R0-FXAA-STANDALONE-MOTION-CROP-2026-11-02.png) was inspected: thin grid/stroke edge intensities shift with the subpixel translation; no gross inversion or missing content is apparent. The cyan vertical line is explicitly authored by the generator. [A](R0-FXAA-STANDALONE-MOTION-A-2026-11-02.png) and [B](R0-FXAA-STANDALONE-MOTION-B-2026-11-02.png) retain native 1280×720 viewing copies. These are two held synthetic frames, not a watched continuous sequence or ordinary moving-game acceptance. The coordinator independently inspected the prior identical A/B diagnostic captures with the same limitation.

## Concrete integration handoff and stopping boundary

No general V1/V4 renderer or render-graph consumer exists in the workspace. Creating a public pass API or a fake F4 shader schema here would invent the missing core boundary. The package therefore supplies the following reviewable handoff instead:

1. F4 binds the selected WGSL/configuration/notices to admitted source/build identity and exact shader/cook/toolchain/ABI dependencies. The package recipe is not that admitted product.
2. V1 prepares one concrete pipeline per selected output format/device generation: the pinned vertex/fragment entries, no vertex buffers, fullscreen triangle, one sampled-float2D texture and one filtering sampler, no uniforms/storage, no blending/depth. The adapter validates filterability and format/usage; readiness refuses an unavailable tuple.
3. The typed graph reads initialized encoded/luma mip0/layer0 and writes a distinct same-extent attachment. Its output is sampled downstream for decode→linear UI→final encoding once. No sRGB attachment conversion occurs at the FXAA boundary. Unsupported extent/subrect/MSAA/aliasing/luma/sampler conditions refuse preparation.
4. V1 owns resources, submission and fence retirement. Same-format pipelines can be shared; outputs are scoped by actual view identity/extent/generation, not quality-profile identity. Off-state removes this pass/target demand after readers/GPU completion.
5. The future integration must prove actual tone/luma values, display/UI encoding, masked scene motion, feature-off/resize/device-loss/restart lifetime and whole-R0 CPU/GPU/live-memory cost. Ordinary moving game content cannot be accepted from this scratch generator.

The chosen form is retained and regression-checked as a **standalone implementation input**. Full FXAA/R0 promotion remains open at the admitted graph/product and ordinary-content gates; 720p isolated timestamp observations do not close them.

## Retained evidence and reproduction

[Source archive](R0-FXAA-STANDALONE-SOURCE-2026-11-02.tar.gz) includes the exact package, fixed header/SPIR-V/reference recipe, Cargo manifest/lock, independent runner, prior fixture generators, mutation/comparator scripts, frozen independent fixture/reference hashes, serial-completion timing probe and raw selected timing logs. [Results](R0-FXAA-STANDALONE-RESULTS-2026-11-02.json) retains complete regression/measurement summaries and every invalid row. [Manifest](R0-FXAA-STANDALONE-MANIFEST-2026-11-02.json) records source/artifact hashes and check logs. Fresh normal input/reference readbacks have the same hashes as the prior retained independent readback archive; no duplicate raw image bundle is needed.

Extract the source archive into an owned `tmp/<task>/`, create `out/`, build from that directory with `RUSTC_WRAPPER= CARGO_TARGET_DIR=target rustup run 1.99.1 cargo build --offline --manifest-path ../Cargo.toml --bins`, then run `python3 ../run_regression.py`. It reads the archived `package/` by default (override with `FXAA_PACKAGE_PATH`), recreates inputs, runs both expected faults and the fresh independent reference, and restores normal captures. `FXAA_SHADER_PATH=../package/fxaa.wgsl ./target/debug/measure` runs the isolated timing probe; any invalid pairs remain failures, not negative durations. Metal access is required. Package validation alone uses `python3 package/verify_package.py`, with optional argument selecting the pinned Naga 30 CLI.

Owned build/raw scratch cleanup follows member-by-member archive/source verification. Scope checks are recorded in the manifest; none implies integrated renderer maturity or device-performance acceptance.

Verification: pinned Naga30 package validation, fresh independent regression, Rust formatting, offline Clippy with warnings denied, `RUSTC_WRAPPER= ./dev check-docs` (264 Markdown files,1,566 local links,30 module records,policy/current hashes) and `git diff --check` passed. All31 source-archive members were restored/byte-verified, and all15 fresh half-float captures matched the prior retained exact readbacks before only the owned898MiB build/raw scratch and restore directory were removed.

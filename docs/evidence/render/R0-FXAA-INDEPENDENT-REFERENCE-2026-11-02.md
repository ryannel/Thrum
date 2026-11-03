# Independently compiled FXAA reference — 2 November 2026

**Result:** the pinned, unchanged FXAA 3.11 header's HLSL5 Quality-12 branch executed through SPIR-V and wgpu 30.0.1 on Apple M3 Max/Metal. The current unchanged WGSL port matched reference **RGB exactly at every captured pixel**, both at RGBA16Float diagnostic output and the port's actual RGBA8Unorm output format. This closes the independent compiled-reference comparison for these inputs on this device. It does not qualify other devices, arbitrary inputs, motion quality, integrated R0 execution or performance. No port/spec change was made.

## Independent execution and identity

The [selected reference](FXAA-SOURCE-SELECTION-2026-11-02.md) is the archived GameTechDev/CMAA2 header at commit `071c6b0857559f4e36f614362e6d2aab1b61938a`. The compiled header SHA-256 was `4834fada78774fc55726adeb717c53ca7324b66820df08ef773515cafb148aef`. It was copied byte-for-byte, not translated into another handwritten reference. The source archive retains the full header's NVIDIA notice/disclaimer and the existing provenance/licence notices.

The wrapper selects `FXAA_PC=1`, `FXAA_HLSL_5=1`, `FXAA_QUALITY__PRESET=12`, `FXAA_GREEN_AS_LUMA=0`, `FXAA_LUMA_SEPARATE_R8=0`, `FXAA_GATHER4_ALPHA=0`, `FXAA_EARLY_EXIT=1`, `FXAA_DISCARD=0`. It supplies the same image/sampler to all four texture parameters, reciprocal dimensions as a uniform, quality constants `0.75/0.125/0.0625`, and zero unused console parameters. Preset12's search distances are 1, 1.5, 2, 4, 12. Only the API wrapper, bindings and input/output orchestration are new; the reference fragment algorithm is the unchanged header.

Compilation was `/usr/bin/clang -E -P -x c -`, then installed NDK `glslc -x hlsl -fshader-stage=frag --target-env=vulkan1.1 -O -o - -`. External preprocessing avoids glslc's HLSL preprocessor failing on an apostrophe in an inactive header `#error`. No algorithm edit was needed. The compiler identifies shaderc v2022.3, SPIRV-Tools v2022.4 and glslang `ndk-r29-beta2`; it resides under the local `tmp/android-sdk/ndk/28.2.13676358/shader-tools/darwin-x86_64/` tree. Compiler bytes/tool versions are in the manifest.

The resulting **10,808-byte SPIR-V** SHA-256 is `f6bd37c2096d8ae82deca6a2336e0dc5c5cf08aa7316277ae0c6fc0f1d6cd185`. `spirv-val --target-env vulkan1.1 -` passed. Naga CLI 30.0.1 SPIR-V validation passed; the executable also parses and validates it with Naga 30.0.1 and empty optional capabilities before creating `ShaderSource::SpirV`. wgpu's Metal backend successfully compiled/executed that module. No standalone `metal` command was used: the installed Xcode `metal` launcher reports its separate Metal Toolchain component missing, which did not prevent runtime Metal library compilation. No tools were downloaded.

The compared WGSL source SHA-256 is `5f26af44c01e3e605c7822b1881b4200ec8bebb6dfcdbabd41fe850cacb64384`, identical to the [prior motion probe](R0-FXAA-MOTION-PROBE-2026-11-02.md). The vertex stage is its explicit fullscreen triangle for both fragment modules; the reference uses `SV_Position.xy * rcp`, while the port obtains the same reciprocal dimensions from the texture. Sharing this vertex stage does not share the FXAA fragment algorithm. Pixel-centre/orientation behavior is checked separately below.

The host was macOS 26.6.2 (25G83), Rust 1.99.1, Apple M3 Max, Metal. The default sandbox reported no Metal adapter; the scoped owned proof executables ran successfully with the GPU sandbox override. No automatic approval rejection occurred.

## Inputs, sampling and observed errors

Both passes read the **same GPU texture**, with sRGB-encoded RGB and encoded-channel `(0.299,0.587,0.114)` luma in alpha, stored in RGBA16Float. No sRGB texture decoding occurs. The sampler is linear min/mag, clamp-to-edge, nearest mip selection, min/max LOD zero; the texture has one mip. The HLSL reference retains native `SampleLevel(...,0,integer_offset)` for neighboring texels and level-zero sampling for searches. The WGSL port retains its normalized-coordinate offsets. Readbacks use identical top-left row order and checked row alignment. Reference and port have separate pipelines/outputs; the source texture is not overwritten.

| Capture | Pixels | RGB error: half-float / RGBA8 | Reference pixels changed from encoded input |
| --- | ---: | --- | ---: |
| Asymmetric 64×64 orientation control | 4,096 | 0 / 0 | 0, deliberately early-exit |
| Prior masked scene, 256×256 | 65,536 | 0 / 0 | 5,548 |
| Prior diagonal/thin-stroke/wave fixture, 256×256 | 65,536 | 0 / 0 | 6,152 |
| Prior motion generator A, 1280×720 | 921,600 | 0 / 0 | 61,028 |
| Same motion generator B, translated 0.37 pixel | 921,600 | 0 / 0 | 63,352 |

Thus all **1,978,368** captured RGB pixels match exactly after each tested target conversion; counts above 1/255, 4/255 and 16/255 are all zero. No nonfinite output values were observed. This does not establish equality of unobserved intermediate f32 arithmetic. The non-control captures change thousands of pixels, ruling out an early-exit-only or identity-pass comparison.

The orientation control has exact binary16 red/green/blue/asymmetric mixed-colour quadrants and constant 0.5 alpha. This intentional luma control forces early exit; both outputs reproduce input RGB exactly at every pixel, detecting flips, channel swaps and output placement independently of edge filtering. The other fixtures use actual RGB-weighted alpha luma. The masked/fixture inputs were regenerated by the copied prior diagnostic and its unchanged input shaders; the native-size pair uses the prior motion shader unchanged. Its source and exact captured input hashes are retained.

Reference alpha equals the input centre-luma channel at every pixel. Port alpha equals 1 at every pixel, as its selected terminal-output adaptation specifies. RGB equivalence does **not** qualify alpha interchangeability. The earlier scalar f64 reference's thin-stroke threshold differences are not reproduced by this independently compiled GPU reference on these captures; they must not be counted as an observed HLSL-versus-WGSL GPU defect.

The [comparison sheet](R0-FXAA-INDEPENDENT-COMPARISON-2026-11-02.png) has masked input on the top row and fixed encoded fixture below. Its four columns are input / independent reference / unchanged port / RGB absolute error ×8. The black error panels agree with the full readback comparison. [720p A](R0-FXAA-INDEPENDENT-MOTION-A-2026-11-02.png) and [B](R0-FXAA-INDEPENDENT-MOTION-B-2026-11-02.png) retain native-size reference viewing copies. The sheet and A were visually inspected for orientation and expected diagnostic content; this is not moving-scene beauty/shimmer acceptance. No timing experiment was made.

## Retention and reproduction

- [Source archive](R0-FXAA-INDEPENDENT-SOURCE-2026-11-02.tar.gz), 44,938 bytes, SHA-256 `f320434a1322cc199541a3c7d28038ef9a5049c70e7f12a42b325e106894e47a`: Cargo manifest/lock, final Rust sources, prior input generators, unchanged WGSL, pinned header, wrapper/defines, preprocessed HLSL, compiled SPIR-V, analysis/compile scripts and notices.
- [Exact readbacks](R0-FXAA-INDEPENDENT-READBACKS-2026-11-02.tar.gz), 889,577 bytes, SHA-256 `261e626a5f761d23c11ffad9e53889d1d3e1353d9f41ffd82ff7c2fbf842b685`: all five captured inputs, reference/port half-float outputs and reference/port RGBA8 outputs. RGBA16F is little-endian four IEEE binary16 channels; RGBA8 is four unorm bytes, tightly packed top-left rows.
- [Comparison data](R0-FXAA-INDEPENDENT-COMPARISON-2026-11-02.json) records per-capture hashes and complete error counts. [Manifest](R0-FXAA-INDEPENDENT-MANIFEST-2026-11-02.json) records source/artifact/compiler hashes, tool versions and successful execution logs.

Extract the source archive into an owned `tmp/<task>/`, create its `out/`, then from `out/` run `RUSTC_WRAPPER= CARGO_TARGET_DIR=target rustup run 1.99.1 cargo build --offline --manifest-path ../Cargo.toml --bins`, `./target/debug/prior`, `./target/debug/r0-fxaa-independent`, and `OUT8=1 ./target/debug/r0-fxaa-independent`. Run `python3 ../analyze.py` to regenerate statistics and viewing copies. Execution needs Metal access. The archived SPIR-V is directly executable; rebuilding it with `python3 ../compile_reference.py` additionally requires the exact installed NDK tools at the recorded sibling path and Naga CLI under `~/.cargo/bin/naga`.

Both retained archives were decoded and every member compared byte-for-byte with its selected original before owned target caches/raw scratch captures were removed. Archives contain no build cache or executable. Direct GLSL ingestion remains blocked by the previously observed combined-sampler frontend/import limitations; the successful HLSL route supplies the needed independent reference without that adaptation. Cross-device validation, larger adversarial input coverage and complete R0 quality/cost remain separate gates.

Verification: `RUSTC_WRAPPER= ./dev check-docs` passed (259 Markdown files, 1,533 local links, 30 module records, crate policy and current design hashes); `git diff --check` passed. The inherited wrapper returned silent exit 2, so the successful repository check explicitly cleared it.

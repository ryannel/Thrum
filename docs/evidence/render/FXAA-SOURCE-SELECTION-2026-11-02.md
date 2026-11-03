# R0 spatial AA source selection — 2 November 2026

**Status:** exact reference and port configuration selected; an independently
compiled HLSL reference and the isolated WGSL port ran on Metal. Their RGB
readbacks match exactly for all 1,978,368 pixels in the selected still and
native-size motion fixtures. The earlier scalar f64 comparison found
thin-stroke ties up to 69/255; those differences did not reproduce against
the compiled GPU reference. Integrated R0 execution, broader visual acceptance
and sustained device measurement remain open.

The R0 [render contract](../../design/RENDER-SPEC.md#8-r0-first-rendered-game-contract-and-proof)
uses FXAA 3.11 Quality on a scene-only sRGB-encoded intermediate. The
accessible reference is the archived [GameTechDev CMAA2 copy of NVIDIA
FXAA 3.11](https://github.com/GameTechDev/CMAA2/blob/071c6b0857559f4e36f614362e6d2aab1b61938a/Projects/CMAA2/FXAA/Fxaa3_11.h).
It identifies the original NVIDIA D3DSamples file and adds an optional
separate-R8-luma path. This selection does not import the 87 KB HLSL header or
make Intel's other anti-aliasing system a dependency.

| Pinned item | Value |
| --- | --- |
| Archive commit | `071c6b0857559f4e36f614362e6d2aab1b61938a` (3 January 2023) |
| Reference path | `Projects/CMAA2/FXAA/Fxaa3_11.h` |
| Reference SHA-256 | `4834fada78774fc55726adeb717c53ca7324b66820df08ef773515cafb148aef` |
| Repository licence file SHA-256 | `be3cb07dcb1bb6b191d4fa72910d9ccfcdd7d035d2be92c97a7e84a476406c54` |
| Per-file provenance | NVIDIA copyright 2014; source/binary redistribution terms and disclaimer in header; the archived repository also carries Apache-2.0 terms. A derived port must retain the applicable notices and origin in the distribution review |

The first WGSL port selects the PC quality branch, quality preset **12**, alpha
channel luma (`FXAA_GREEN_AS_LUMA=0`, `FXAA_LUMA_SEPARATE_R8=0`),
`FXAA_GATHER4_ALPHA=0` for the portable explicit-sample path, early exit
enabled and no discard. The tone pass writes sRGB-encoded RGB and its
reference-weighted `(0.299,0.587,0.114)` encoded-channel perceptual luma into alpha of the filterable
`RGBA16Float` intermediate. The selected starting constants are
`subpix=0.75`, `edge_threshold=0.125`, `edge_threshold_min=0.0625`; the
reference header describes these as a default subpixel value and high-quality
edge choices. Pixel centre, address/clamp and gather-versus-sample semantics
must match the port's reference output; it cannot silently fall back to
green-only luma or sample in linear light. The post-filter pass decodes RGB to
display-linear before UI and final output encoding. FXAA is scene-only; UI
edges are not promised that filter.

The isolated diagnostic writes alpha `1` at its terminal FXAA output rather
than preserving the reference's centre-luma alpha. This is an output/compositing
adaptation; reference equivalence must compare **RGB** before any later UI and
check alpha separately against the selected final-output policy.

The [diagnostic evidence](R0-GPU-DIAGNOSTIC-2026-11-02.md) records the first
WGSL/source hashes and notices. The [compiled-reference proof](R0-FXAA-INDEPENDENT-REFERENCE-2026-11-02.md)
compares exact whole-image readbacks from the unchanged port and pinned HLSL
header. It closes the observed scalar search-threshold question for those
fixtures on Apple M3 Max/Metal. The port still needs promotion with notices,
validation of that promoted form, broader fixed and ordinary moving-content
inspection, and integrated R0 qualification.
The [native-size synthetic motion probe](R0-FXAA-MOTION-PROBE-2026-11-02.md)
captures two 720p/1080p frames, but its GPU timestamp intervals overlap and
provide no accepted pass-cost result.
Measure the pass and its extra live target with the complete R0 graph on the
named device. A copy that compiles or one appealing still is insufficient.
The input copy was inspected and hashed in task scratch under `tmp/r0-fxaa/`;
that scratch is not a shipped product or durable authority.

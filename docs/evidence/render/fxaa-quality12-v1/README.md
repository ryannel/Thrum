# FXAA Quality12 standalone implementation input, version 1

This package retains the verified WGSL form with its upstream notices. It is an implementation input for future R0 integration, **not an admitted F4 product, a public renderer API or a completed R0 graph pass**. The [independent Metal reference proof](../R0-FXAA-INDEPENDENT-REFERENCE-2026-11-02.md) identifies the upstream source and tested equivalence. [product.json](product.json) pins this package's bytes, configuration and standalone interface. The [shader](fxaa.wgsl), [NVIDIA notice](NOTICE-FXAA.txt) and [archive repository licence](LICENSE-INTEL-APACHE.txt) travel together; binary distributions must reproduce the applicable notice/disclaimer in accompanying materials. Upstream attribution does not imply endorsement.

## Selected standalone interface

Draw a fullscreen triangle with vertices 0..3 and instance 0..1, no vertex buffers, `vs_fxaa`/`fs_fxaa`, blending/depth disabled. Group 0 binding 0 is a filterable float 2D texture: single-sample, one-mip RGBA16Float, full extent, scene-only sRGB-encoded RGB and alpha `dot(encodedRGB,[0.299,0.587,0.114])`. This alpha is perceptual luma, not material opacity. Binding 1 is a filtering sampler: clamp-to-edge in U/V, linear min/mag, nearest mip, min/max LOD 0. The shader requires no uniform/storage buffer. Input/output extents and full viewport agree; zero extents, subrects, array layers, MSAA or read/write-in-place are unsupported by this package.

Output RGB remains encoded; output alpha is 1. RGBA16Float and RGBA8Unorm targets were independently compared. Do not use an sRGB target conversion here. For linear UI, decode RGB after filtering and apply the final output encoding once later. UI is excluded from the input. No history, exposure calculation, HDR-to-display mapping or automatic luma generation occurs in this shader.

The fixed configuration is Quality12, alpha luma, no gather/discard, early exit, subpixel 0.75, edge threshold 0.125 and minimum 0.0625. Dynamic tuning requires another explicitly tested product version. The selected shader SHA-256 is `5f26af44c01e3e605c7822b1881b4200ec8bebb6dfcdbabd41fe850cacb64384`; its fragment algorithm is unchanged from the verified port.

## Future producer/consumer handoff

F4 must eventually cook expanded WGSL plus this selected interface/configuration and provenance under an admitted source/build binding. Its product key includes shader bytes, dependency/ABI/cook/toolchain versions, concrete target formats and immutable pipeline descriptor. This JSON is a reviewable recipe, not that implemented product format. F2 enumerates the reachable format/profile tuple; V1 validates Naga/reflection and adapter limits, prepares the device pipeline and binds initialized input before readiness. V4 supplies the encoded/luma producer and receives the filtered image. Only V1 owns GPU resource lifetime/submission; this package registers no private renderer or queue.

The FXAA output is a separate target with attachment-write then downstream sampled-read usage. For the selected half-float interface, its logical pixel payload is `8*width*height` bytes per simultaneously live view: 7.03125MiB at 1280×720 or 15.8203125MiB at1920×1080. The same-size borrowed encoded input adds another8P while both are live. Actual driver allocation/tiling is unmeasured. No extra history, depth, storage or compute scratch is required. At feature-off, omit this pipeline demand/pass and its output target; retain the encoded input only if another display consumer requires it. Retire prior targets after final CPU readers/GPU completion. Two views require separately identified output extents/resources; profile identity is not view identity.

Initial readiness failures include hash/version/layout mismatch, unavailable pipeline, invalid size/format/sample count, uninitialized or aliased input, incompatible sampler or missing luma. Source/device/size replacement prepares a coherent new tuple before publication; failure preserves the previous ready generation. This is the ownership plan for the future graph, not runtime validation currently provided by the shader file.

## Gate still required

The standalone regression/cost evidence is reported beside this package. Full promotion still needs the real V1/V4 graph API, admitted F4 shader/build binding, actual tone→AA→linear-UI→output integration, ordinary moving game content and whole-frame cost/resource lifecycle on the named device. None exists solely because this file is retained in the repository.

## Standalone verification and regression

Run `python3 verify_package.py` here for exact source/notice hashes and offline pinned Naga 30 validation (optional first argument selects that executable). The [standalone gate report](../R0-FXAA-STANDALONE-PACKAGE-2026-11-02.md) retains the independent HLSL reference regression and measured scope. Its source archive is self-contained with this exact package; the runner consumes the package path rather than keeping another WGSL authority. Normal runs must match the independent reference/input hashes; deliberate green-luma and UV-flip mutations must return failure. These are meaningful comparator sensitivity checks, not runtime feature options.

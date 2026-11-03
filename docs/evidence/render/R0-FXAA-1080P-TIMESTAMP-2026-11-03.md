# 1080p FXAA Metal timestamp diagnosis — 3 November 2026

**Result:** the earlier invalid intervals are reproducible query-result failures. Resolving counters in a **second submission after the render submission has completed** produced valid bounded captures on the Apple M3 Max. The selected fresh-index method passed all **432 pairs** across three processes: 384 measured and 48 warmup. Its measured vertex-start→fragment-end medians were **144.250 / 145.771 / 145.917 µs**. The corresponding CPU measurement workflow medians were **1.948 / 2.019 / 2.005 ms**. Those quantities are separate; neither is whole-R0 frame cost.

The same-submission method remains rejected. No shader, source package, production crate, build plan, contract or maturity record was changed. This establishes a bounded measurement method, not a general Metal profiler fix or ordinary-game visual acceptance.

## What failed, and what the ablations distinguish

The [previous standalone probe](R0-FXAA-STANDALONE-PACKAGE-2026-11-02.md) occasionally returned an end timestamp before its begin timestamp. In this investigation, reused-slot failures returned a **new begin plus exactly the previous sample's end**. A discovery example has sample 101 begin `2179270566491375`, end `2179270565742083`; sample 100 had that identical end. This is mixed-generation result evidence, not negative GPU execution time.

The final frozen experiment holds shader, encoded input, target, sampler, draw and device constant. It varies slot reuse, resolve submission and CPU mapping order. Every render is submitted and completed before the next sample; delayed modes additionally complete a separate resolve/copy submission before mapping. Methods execute in rotated order across three processes. Each method has 16 warmup and 128 measured samples per process; every raw record is retained.

| Method | Slots | Resolve | Map request | Invalid measured / warmup, all three processes |
| --- | --- | --- | --- | ---: |
| `reuse_early` | Same pair | Same submission | Before render-completion wait | 4 / 0 |
| `reuse_same` | Same pair | Same submission | After render-completion wait | 5 / 3 |
| `unique_same` | Fresh pair per sample | Same submission | After render-completion wait | 2 / 0 |
| `reuse_delayed` | Same pair | Separate submission after render completion | After resolve completion | 0 / 0 |
| **`unique_delayed`** | Fresh pair per sample | Separate submission after render completion | After resolve completion | **0 / 0** |
| `no_queries` | None | None | None | Not applicable: no GPU counter interval |
| `clear_delayed` | Fresh pair, no draw | Separate submission after render completion | After resolve completion | 384 / 48: zero counters |

Fresh slots alone do not solve the failure: `unique_same` returned a zero end in repetition 0/sample 55 and repetition 1/sample 127. Reuse exposes the missing/currently unavailable end as an old value; fresh storage exposes zero. Mapping before versus after the render wait also does not solve it. Both delayed modes passed every warmup and measured pair. This isolates the **counter-resolution/readiness boundary** in these observations. It does not prove whether the underlying cause belongs to Metal's implementation or wgpu's use of it.

The clear-only control executes a real clear/store attachment pass but no vertex/fragment draw. Its counters are zero; zero is not a zero-cost measurement. Its final black image is checked independently. The final output hashes for all six drawing modes agree, including no-query mode, across all repetitions: `7ae90fa18debf4e367305df1686c4331cc9775a5307e49cc2113c7fe48a84049`. This guards the final image per method; images were not read back inside each timed sample. No visual claim is made.

## Source and API trace

The selected source remains SHA-256 `5f26af44c01e3e605c7822b1881b4200ec8bebb6dfcdbabd41fe850cacb64384`, asserted over the actual loaded file by the executable. It is the existing [Quality12 package](fxaa-quality12-v1/README.md). Its source/notice hashes and pinned Naga 30 validation were also rechecked. Input is the retained motion generator held at 0.37-pixel translation, not changing game content.

Inspected wgpu-hal 30.0.1 `src/metal/command.rs` assigns pass-begin to start-of-vertex and pass-end to end-of-fragment counter samples (around lines 1015–1040), makes `reset_queries` a no-op (around 853), and resolves timestamp counters through a blit encoder (around 877–887). `src/metal/device.rs` around 1940–1980 creates a shared counter-sample buffer. These are source observations, not proof that any particular call violates Metal's requirements. The [wgpu-hal source](https://docs.rs/crate/wgpu-hal/30.0.1/source/src/metal/command.rs) and [wgpu timestamp API](https://docs.rs/wgpu/30.0.1/wgpu/struct.CommandEncoder.html#method.write_timestamp) provide the public context; absolute GPU tick values are not converted to wall-clock time here.

The installed Apple SDK's `MTLRenderPass.h` documents the corresponding vertex/fragment sample indices. `MTLBlitCommandEncoder.h` documents counter resolve and `MTLCounterErrorValue`; `MTLCounters.h` defines that error sentinel as all-one bits. Exact inspected file paths, hashes and locators are retained in the archive manifest. The relevant [Apple sample-buffer descriptor](https://developer.apple.com/documentation/metal/mtlrenderpasssamplebufferattachmentdescriptor) is stage-based; it is not a command-buffer elapsed-time API. Apple's web reader returned JavaScript shells/unsupported Markdown, so the installed SDK headers were the inspected primary text.

A native-command-buffer observation was attempted without changing rendering. wgpu 30 rejects mixing ordinary encoding with `CommandEncoder::as_hal_mut`, even when the callback only retains the native command-buffer handle. It panicked before sample submission; that failed source/log is retained. The guard is in wgpu-core 30.0.1 `src/command/mod.rs::EncodingApi::set`, around548–570. No library patch or raw-pointer workaround was used. Consequently this report does **not** claim a native `GPUStartTime`→`GPUEndTime` measurement. The installed `MTLCommandBuffer.h` distinguishes those GPU host-time properties from CPU completion; [Apple's API](https://developer.apple.com/documentation/metal/mtlcommandbuffer/gpuendtime) remains a possible future independently owned native measurement path.

## Selected serial-completion method and independent validity gate

`unique_delayed` uses one query set with **288 entries** per process/method. Sample−16 uses slots 0/1, sample−15 slots 2/3, through sample 127 slots 286/287. Each pair is written once. Its protocol is:

1. Encode only the FXAA render pass with begin/end writes and submit it.
2. Wait for that specific render submission to complete.
3. Encode counter resolve plus 16-byte copy in another command buffer, submit and wait for it.
4. Request mapping, poll/wait for mapping, read both raw u64 values and unmap.
5. Finish recording the sample before starting the next one.

Fresh indices remain valid across **144 distinct render completions and 144 resolve completions in each process**, not just a single first command buffer. Across three processes this gives432 pairs, including384 measured pairs. `reuse_delayed` independently supports the resolve-boundary inference, but the selected capture retains fresh pairs so a missing write cannot masquerade as an old plausible result.

The independent Python evaluator reads raw CSV, not producer validity flags. It checks complete coverage and ordering; declared shader/device/period; exact slot assignment; finite nonnegative CPU durations; zero/all-ones sentinels; positive intervals; duplicate pairs; overlap with the preceding completed interval; and GPU duration exceeding its enclosing CPU submit-to-completion duration. It also checks final-image agreement, an independently constructed black clear image, and consistent drawing output across processes. Every invalid row remains in the analysis. The selected method fails qualification if **any** of its warmup or measured pairs fails; invalid variants' valid-subset statistics are diagnostic only.

Evaluator sensitivity checks inject an end-before-begin value, a wrong slot and a missing row in memory; each is detected. No measured records are repaired. The chosen method passed all checks. This is evidence for this finite capture on this machine, not proof against all future counter faults.

## Valid GPU interval distribution versus CPU elapsed

Selected `unique_delayed`, all 128 measured pairs accepted in every repetition; GPU period reported 1 ns:

| Repetition | GPU interval min / median / p95 / max, µs | CPU render submit→completion median, µs | CPU resolve submit→completion median, µs | CPU total sample median / p95 / max, µs |
| --- | --- | ---: | ---: | --- |
| 0 | 126.583 /144.250 /155.125 /188.875 | 622.584 | 1,186.354 | 1,948.042 /3,978.792 /4,093.834 |
| 1 | 132.084 /145.771 /163.667 /216.500 | 764.813 | 1,240.396 | 2,019.188 /4,032.291 /5,025.083 |
| 2 | 128.875 /145.917 /157.625 /169.667 | 663.208 | 1,292.271 | 2,004.563 /4,050.167 /5,047.750 |

P95 uses nearest rank. CPU encode/submit medians are 157.229/208.938/161.125µs and are retained with all distributions/raw records. CPU intervals include driver, scheduling and wait overhead; the total includes encoder construction, both submissions/completions, map polling and result extraction. Its component medians need not sum, and encode/submit timing partly overlaps the recorded submit→completion interval.

No-query CPU total-sample medians are608.292/599.375/594.563µs. This is an overhead/workflow control, **not GPU cost**; do not subtract it from the counter interval or assume a whole-frame saving. Clear-only CPU total medians are1,007.729/1,013.750/999.292µs, with unusable GPU counters. Delayed resolution deliberately adds synchronization and a submission; this intrusive offline capture procedure is not a proposed shipping frame loop.

The GPU measurement is elapsed **vertex-start→fragment-end**, not an active-cycle count, native command-buffer duration, attachment load/store cost, resolve cost or end-to-end displayed latency. The isolated pass has no scene geometry, shadow, tone, UI or final-output path. Host background activity, thermal and power state were not controlled sufficiently for platform budget qualification. These valid 1080p observations replace neither the rejected old method nor an assembled R0 cost gate.

## Workload, memory and reproducibility

The executable uses Rust 1.99.1, wgpu/wgpu-hal/Naga 30.0.1, Metal only and `TIMESTAMP_QUERY`; shader validation requests no optional shader capabilities. Device is Apple M3 Max; exact OS/toolchain/dependency/source/executable identities are in the retained manifest. GPU access required the same scoped sandbox override as earlier probes. No tools were downloaded and no Broadsides file was accessed or changed.

The 1920×1080 workload uses RGBA16Float input/output, one mip/sample, full viewport, clamp-to-edge linear min/mag and LOD0, blending/depth disabled. One fullscreen draw is identical in all six drawing modes. Source generation and final-image readback/hashing are outside timing. The package source is read-only and unchanged.

Nominal live pixel payload during timing is two 8P images: **33,177,600 bytes = 31.640625MiB**, including the additional FXAA output target of15.8203125MiB. The largest query set has 288 logical 8-byte results (2,304 bytes); wgpu-hal also creates a same-size raw result buffer. Add 256-byte resolve,16-byte readback and16-byte input uniform: **4,896bytes** of these logical non-image payloads. Driver counter allocation, padding, pipeline/code and actual residency are unmeasured. Reused-slot modes need only two logical counter entries. Controls still retain the common input/output targets.

Untimed output checking additionally creates a 15.8203125MiB GPU readback and a same-size CPU byte vector. Its nominal image/readback/vector peak is 63.28125MiB before driver/allocator overhead; it is not included in the timed target floor. No extra target/history is introduced by delaying resolve.

The [selected source/data archive](R0-FXAA-1080P-TIMESTAMP-SOURCE-2026-11-03.tar.gz) contains frozen code/Cargo lock, the unchanged package and notices, all final raw CSVs, independent evaluator/results, inspected-source identities, exact tool/device configuration, the discovery/pilot records with their source versions, and the rejected native-observer attempt. Pilot captures that happened to pass are retained to show why a short clean run alone did not identify the failure. No binary/build cache is retained.

Extract into an owned scratch directory. Build with `RUSTC_WRAPPER= CARGO_TARGET_DIR=out/target rustup run 1.99.1 cargo build --locked --offline`; from `out/`, run `FXAA_SHADER_PATH=../package/fxaa.wgsl ./target/debug/fxaa-1080p-timestamp 0 > rep-0.csv`, then repetitions 1 and 2. Metal access and cached pinned crates are required. Run `python3 validate.py` from the extracted root; it reconstructs validity/distributions from all three raw files and refuses a failed selected capture. Its code/source/data hashes and restore checks are in the archive manifest. No production integration or ordinary-game acceptance is implied.

The selected archive is 218,647 bytes, SHA-256 `c6c39f88f1db740a99bfe00003f86c23d3deeac1f22285b98c3c20facaa69936`. Frozen measured Rust source SHA-256 is `c41d815f84b354ea8b56e9012cdacfdf058fae2348ea45204ccb42d69c2944ce`; the measured executable hash is retained in its manifest. All 33 archive members were restored and byte-checked. The restored independent evaluator reproduced the analysis JSON exactly, and restored package hashes/Naga validation passed before owned build-output cleanup.

Scoped verification passed: Rust formatting, offline Clippy with warnings denied, the independent raw-data evaluator and its corruption checks, unchanged/restored package hashes and pinned Naga validation, `RUSTC_WRAPPER= ./dev check-docs` (275 Markdown files,1,627 local links,30 module records,policy/current hashes), and `git diff --check`. Only the owned885MiB build/data scratch, restore directory and duplicated archived pilot/discovery/rejected copies were removed.

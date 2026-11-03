# E1 trusted cooker class and target lock — candidate

3 November 2026. Bounded F4/T2 producer proposal for the unresolved class and lock fields in the [E1 persistent identity candidate](E1-CONSTANT-LIGHT-PERSISTENT-IDENTITY-2026-11-03.md). It keeps the [selected CPU constant-light product](../modules/ENVIRONMENT.md#selected-first-f4-constant-light-product-and-binding) and C05 source/product separation. No existing Thrum build issues a verified E1 cooker class, profile lock, durable product or ready build.

## Two different selections

`CookClass` identifies the **actual host executable and execution envelope that normalize the 160-byte E1 payload**. The headless game's current Cargo reuse `selectionKey` describes a game build recipe and does not identify the `thrum-cook` code currently executing in the host. It cannot substitute for the cooker class. A Mac host that cooks a product does not implicitly qualify an iOS, Android, Wasm or other player target.

`ProfileTargetLock` describes the selected **player consumer/profile contract** to which an exact-source build binding applies. Changing player target or delivery may change only this lock and binding when the same approved host class emits the same portable CPU payload. Both fields are digests of typed, versioned descriptors **and** require a trusted producer/retained lease. A caller-supplied digest or successfully decoded descriptor is an identity claim, not authorization.

The first candidate is one retained macOS ARM64 native E1 cooker executable, CPU-only `constant-light-v1`, with the specified ordered binary64 arithmetic and no renderer dependency. Build its executable from a distinct, verified engine-cooker source/dependency closure; keep the final executable hash in an **external** post-link provenance record to avoid self-hash circularity. No current release/root inventory, synthetic release bridge or headless A/B game build proves this closure. In particular, the existing T2 release manifest proposal still needs a real reviewed manifest and the selected runtime/support/OS bytes and reads remain open.

## Candidate fixed descriptors

All integers below are little-endian and hashes are raw SHA-256 octets. A future decoder accepts only the exact supported combination and refuses other flags, enum values, lengths or extra bytes.

`CookClassDescriptorV1` is **156 bytes**:

| Offset | Bytes | Field |
| ---: | ---: | --- |
| 0 | 8 | ASCII `THRE1CC` then zero |
| 8 | 2 | Descriptor version `1` |
| 10 | 2 | Reserved flags `0` |
| 12 | 2 | Host execution target code `1`: `aarch64-apple-darwin` |
| 14 | 2 | CPU codegen contract code `1`: fixed baseline selected in verified cooker recipe; no ambient `target-cpu=native` |
| 16 | 2 | FP contract code `1`: ordered binary64, round-to-nearest-even, gradual underflow, no reassociation/contraction that changes the specified expressions |
| 18 | 10 | Five `u16` recipe, product, cooker, normalizer and evaluator revisions, initially all `1` |
| 28 | 32 | Exact retained cooker executable SHA-256 |
| 60 | 32 | Digest of T2-verified external cooker build-provenance record |
| 92 | 32 | Digest of selected, reviewed release manifest's exact canonical bytes |
| 124 | 32 | Digest of T2-selected host runtime/support closure and execution policy |

`CookClass = SHA-256("thrum.e1.constant-light.cook-class.v1\0" || descriptor[0..156])`. Fixed code values state **requirements**, not claims that current Rust tests proved FP control, generated instructions or support-library closure. The canonical nested provenance records are T2-owned and unselected, so this descriptor is a candidate byte envelope rather than an issuable credential.

`E1ProfileTargetLockV1` is **92 bytes**:

| Offset | Bytes | Field |
| ---: | ---: | --- |
| 0 | 8 | ASCII `THRE1PL` then zero |
| 8 | 2 | Descriptor version `1` |
| 10 | 2 | Reserved flags `0` |
| 12 | 2 | Semantic profile code `1`: `constant-light-v1` |
| 14 | 2 | Player target code `1`: first CPU-only macOS ARM64 development target |
| 16 | 2 | Delivery code `1`: native headless |
| 18 | 2 | Consumer ABI code `1`: 160-byte `ConstantLightProductV1` |
| 20 | 2 | Environment provider code `1`: authored constant over position/time |
| 22 | 2 | Renderer code `0`: absent |
| 24 | 4 | Unsupported-quantity mask exactly `0x7f` |
| 28 | 32 | Selected `CookClass` digest used for this product |
| 60 | 32 | Digest of exact player target ABI/features/deployment-policy record |

`ProfileTargetLock = SHA-256("thrum.e1.constant-light.profile-target-lock.v1\0" || descriptor[0..92])`. The first combination performs no tick evaluation, weather work, cloud attenuation, view state or GPU allocation. Ambient and background remain independent authored values. A render consumer or non-Mac player target needs a separately selected lock/consumer qualification. Operational source capacities, observed disk space and host work allowances stay outside this semantic lock unless a source definition meaning itself changes; that change already invalidates the F3 definition hash and CookKey.

## Producer handoff and invalidation

1. T2 prepares and retains the cooker executable from its **own** exact engine source, lock/dependency, compiler/linker/SDK and release closure. Its external canonical build-provenance record must bind ordered invocation/environment for each unit, resolved host features, FP-relevant codegen settings, complete verified inputs/receipts and final executable hash. T2's selected host-runtime record binds actual loader/support/OS execution policy. Neither record grammar nor a real reviewed release manifest exists yet.
2. T2 execution selection verifies the record→executable→runtime associations, holds the bytes and returns an opaque `VerifiedCookClassLease`. A raw `[u8; 32]`, local exact-hash inventory or test class cannot construct that lease. Recheck or preserve actual execution bytes throughout the cook; release/owner loss prevents durable publication.
3. F2/F4 profile resolution checks the explicit player target and provider/consumer ABI, yields an engine-owned typed `VerifiedProfileTargetLock`, and retains the target-policy record. Authored F3 source can request `constant-light-v1`; it cannot attest its target or trust manifest.
4. F4 accepts both verified contexts and the intended project-store pin. It checks that the lock's class is the leased executing class, computes the already specified semantic recipe and expected bytes, and retains those contexts through product installation and exact-source binding. The wider F4/T2 ready publisher still binds player code, compiler, dependency, product and provider closure under its own atomic transaction.

Changing executable bytes, build provenance, compiler/toolchain/linker/SDK release selection, host CPU/features, FP mode/codegen, host runtime/support closure or any of the five revisions changes `CookClass` and hence `CookKey`. Changing only player target, delivery or consumer-profile details changes `ProfileTargetLock` and exact binding; it need not recook the 160-byte payload if the host class and known source values are unchanged. Different classes may emit identical payload bytes and share `BlobHash` while retaining distinct keys/manifests. Installation paths, source labels/opaque fields and unrelated game content do not change either descriptor; exact source still changes its binding.

## Rejection and cost proof

Independent byte/hash goldens, strict decoder refusals, same-width field-swap/context checks and every-field invalidation are required. Presenting the game's `selectionKey` as cooker provenance, substituting a different retained executable/runtime, mixing release records or issuing a verified class from an unreviewed candidate must refuse. Prove host-versus-player target separation, lease loss/cancellation, repeated-process noncardinal/subnormal/signed-zero results, generated arithmetic/FP control and a valid wrong-class refusal. Cold executable/provenance/runtime selection cost is measured separately and shared across cooks; no light rescans tool roots. Warm fixed-size descriptor/key work is measured with complete accepted-pin read/decode and concurrent T2 work before a latency claim.

This candidate cannot become a trusted durable publication class until T2 selects and verifies the nested build/runtime/target-policy records and actual release provenance. The private [identity byte proof](../../evidence/i2a/E1-CONSTANT-LIGHT-IDENTITY-BYTE-PROOF-2026-11-03.md) may use fixture digests for byte tests only. The native product-store lease, disk-space composition and wider ready-build transaction remain separate gates.

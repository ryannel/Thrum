# Held two-credit four-role Metal diagnostic — 3 November 2026

**Selected result:** the synthetic four-role R0 pass chain ran with A/B
genuinely GPU-incomplete on one tracked 720p graph while a new 1080p graph
was prepared. A third submitted-packet credit refused before packet extraction,
uniform/readback creation, staging or submission. Releasing A alone reclaimed
one credit while B remained blocked; C then used the new graph and that credit.
All three independent native tails completed, and an independent half-aware
CPU oracle found **zero code error in all 3,916,800 pixels**. Old application
view/output ownership retired only after GPU completion and two explicit CPU
leases ended. This is one Apple M3 Max / Metal synthetic scenario, not authored
R0, a reusable allocator, physical memory or ordinary-frame qualification.

The [source, locked dependencies, shader, notices, result and final log](R0-FOUR-ROLE-HELD-LIFETIME-2026-11-03.tar.gz)
archive has SHA-256 `9b1b4fb9297eb97909aedcf38cd23008e069b49a6280ea4b6d5b97e247928f21`.
The archived `src/main.rs` is SHA-256
`1501be68d88b4660d26ad868809669ad614631195588169524f2adbd7123d0c6`;
the pinned FXAA file remains
`5f26af44c01e3e605c7822b1881b4200ec8bebb6dfcdbabd41fe850cacb64384`.
`RESULT.md` documents descriptor checks, gate/credit sequence, pass dependencies,
allocation accounting and limitations; `SHA256SUMS` identifies each source
member. Rust 1.99.1, wgpu 30.0.1 and Metal are pinned in the archive.

| Packet | Graph allocation IDs | Extent | Exact observed/expected RGBA | Pixels |
| --- | --- | --- | --- | ---: |
| A | `[1,2,3,4,5]` | 1280×720 | `[93,93,176,255]` | 921,600 |
| B | `[1,2,3,4,5]` | 1280×720 | `[122,122,201,255]` | 921,600 |
| C | `[1,6,7,8,9]` | 1920×1080 | `[90,125,125,255]` | 2,073,600 |

The allocation IDs are harness-assigned at `create_texture` sites, not native
addresses. Before any gate release A/B returned API timeout and native tails
`[0,0,0]`, including after new-graph preparation. After A alone, tails were
`[1,0,0]` and B still timed out; after B/C they were `[1,1,1]`.
The strict-event gate prevents execution until released. A single CPU thread
encodes and submits nonempty ordinary-wgpu command buffers; native gate/tail
references remain owned through verification. Error cleanup drains or retains
references rather than freeing in-flight native inputs. API completion alone
is not treated as execution success: native tails and pixels also pass.

Checked descriptor payloads were **76,681,216 B** overlapping old/new
four-role scratch, **11,980,800 B** independent outputs, **15,667,200 B**
retained A/B/C readbacks and 48 B immutable uniforms. Another 48 B is charged
conservatively for cumulative mapped-at-creation initialization staging, for a
selected **104,329,312 B** logical total. Two credits bound submitted GPU
packets only: completed packet readbacks/uniforms remain retained and are
separately preflighted for this fixed scenario. Named call-site counters do
not observe every driver allocation. Staging's simultaneous residency,
alignment, pipelines, surface buffers and physical driver high-water were not
measured. Application `Arc` retirement is not observed physical deallocation.

The same uniform-input FXAA limitations as the [single-frame chain](R0-FOUR-ROLE-SYNTHETIC-2026-11-03.md)
remain: no spatial edge filtering, encoded-luma sensitivity, PCF border or
independent output decode/encode qualification. No F4 product/upload leases,
real surface, preparation-failure rollback, 60 MiB loading branch, device
loss/stale-generation path, authored visual review or frame cost was run.

Reproduce on a Metal host from the repository root:

```sh
mkdir -p tmp/r0-four-role-held-repro
tar -xzf docs/evidence/render/R0-FOUR-ROLE-HELD-LIFETIME-2026-11-03.tar.gz -C tmp/r0-four-role-held-repro
RUSTC_WRAPPER= rustup run 1.99.1 cargo run --offline --locked --release --manifest-path tmp/r0-four-role-held-repro/Cargo.toml
```

The final rebuilt run, rustfmt check and release Clippy `-D warnings` passed.
An independent read-only source/log review found no blocking defect in this
scenario and explicitly bounded the claims above. Production V1/F4 integration
and measured ordinary-frame/device behavior remain separate gates.

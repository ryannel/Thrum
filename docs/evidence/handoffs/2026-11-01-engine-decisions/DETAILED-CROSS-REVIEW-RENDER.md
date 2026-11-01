# Detailed cross-review: network and combined baseline

1 November 2026. Read-only analytical trace of [NETWORK-SPEC](../../../design/NETWORK-SPEC.md),
[BASELINE-SPEC](../../../design/BASELINE-SPEC.md), [N1](../../../design/modules/NETWORKING.md),
[C3](../../../design/CONTRACTS.md) and the water/render decisions. No build or transport
experiment. Scenario: a browser peer joins a pinned four-player server match with a prescribed sea,
finite river and joint air/fog profile; its GPU device rebuilds during sync while a separate authoring
source edit is accepted.

## Concrete corrections

1. **Content loading versus baseline expiry.** NETWORK-SPEC gives required gameplay content up to
   30 seconds to load, but retains post-baseline state for only 2 seconds/256 KiB and allows two
   baseline restarts in 10 seconds. If the baseline is captured before a cold client's physical
   products are ready, it will repeatedly expire and disconnect before the stated content timeout.
   **Replace the order:** negotiate semantic manifest and fetch/validate required client gameplay
   closure first, under the 30-second cap and no leased world baseline/history; then capture and
   stream a fresh baseline under the 2-second/256 KiB and restart limits. If content becomes
   unavailable after BaselineBegin, release that baseline, return to content-loading, then recapture.
   Local V1 pipeline preparation is separate: a coherent replica may stay network-active/view-loading
   while state updates coalesce under N1 limits. No server tick or world snapshot waits for this peer.

2. **Passive admission acknowledgements versus history.** The group history cap is 64 ticks
   (1.067 seconds at 60 Hz). The only specified periodic client message is Clock once per second;
   an admission ack piggybacked on that message can reach the server after history expiry at the
   specified 300 ms RTT + 100 ms jitter fault condition, even with no actual packet loss. This can
   force repeated full groups for a passive observer. **Add:** after a committed Baseline/StateGroup,
   send an ack-bearing Clock immediately when no other outbound message is due, with at most 100 ms
   acknowledgement delay; regular clock synchronization remains once per second. Header ack sequence
   is cumulative over *admitted* messages only. A duplicate BaselineAck/ack is idempotent; a missed
   admission ack may cause a full group, never guessed delta application. At 300 ms RTT + 100 ms
   jitter + 100 ms ack scheduling, the nominal half-second remains below the 64-tick window;
   longer stalls still take the declared resync path.

3. **N1 memory maxima cannot fit 4 MiB.** Per active server peer, documented maxima include 256 KiB
   receive queue, 128 KiB outbound unsent state, 64 KiB reliable control, 256 KiB transport send
   buffer and 256 KiB post-baseline state: 960 KiB. Four peers can reserve 3.75 MiB before any
   retained group baselines, deduplication, parser/TLS buffers or serialization. Listen/client
   processes may also hold 256 KiB baseline staging per joining peer. **Replace the 4 MiB
   whole-session N1 target with an 8 MiB initial reservation cap**, while keeping the listed
   per-peer limits. Charge actual parser/TLS/internal send buffers and group-baseline history
   against it; reject a content/peer recipe if its maximum reservations exceed the cap. This is
   <1% of the desktop memory envelope and remains a design target, not measured residency.

4. **Native WebSocket defaults violate the envelope.** The selected `tokio-tungstenite` 0.30.0
   re-exports tungstenite, whose default message/frame caps are 64/16 MiB and whose read/write
   buffers default to 128 KiB each. An application-level 64 KiB header check occurs after those
   allocations. **Require configured accept/connect before authentication:** maximum WebSocket
   message payload 64 KiB, maximum frame payload ≤64 KiB, read buffer 4 KiB for unauthenticated
   sockets and an explicitly budgeted small active buffer, eager/small write buffering and
   bounded maximum write buffer consistent with one maximum message plus queue headroom. Reject
   fragmented messages whose aggregate exceeds the message cap; keep compression disabled.
   Charge these capacities to finding 3. Browser WebSocket may allocate a full message before
   application validation; the spec already labels that limitation honestly.
   [tokio-tungstenite 0.30.0](https://docs.rs/tokio-tungstenite/0.30.0/tokio_tungstenite/),
   [tungstenite 0.30.0 defaults/configuration](https://docs.rs/crate/tungstenite/0.30.0/source/src/protocol/mod.rs).

5. **Water group is not yet a bounded wire projection.** A complete 128×64 shallow grid cannot
   fit the one-message/atomic-group budget; the protocol's phrase “complete water query sample”
   must not imply a full solver state or arbitrary client query support. For the initial four
   nonpredicted floating bodies, select a ≤64-byte confirmed query record per body per state
   publication: body/wire ID; domain ID/generation; sample tick; edge/source revision key;
   `available/outside/stale` status; surface height in millimetres; octahedral normal in two
   signed 16-bit components; planar flow in signed centimetres/s; and a documented contact/error
   flag. At most four such records (≤256 bytes) form one atomic body/water dependency group with
   the matching body pose, or the client marks that query unavailable. A river grid remains on the
   server; the client can interpolate confirmed body/near-contact presentation and use a local
   visual field within its declared shoreline/contact error. This schema does **not** authorize
   predicting water contact or arbitrary off-body physical queries. If the game needs those,
   it selects another replicated query/product closure and proves its bandwidth and correction.

## Trace after those corrections

The server match stays pinned to its gameplay source/build/semantic manifest. The author's new
source root becomes a separate desired revision and cannot alter its river bank, collision, fog
visibility or solver state. The browser peer first obtains compatible client query/collision
content; server source product hashes are provenance, while its own optional visual meshes,
shader bytes and GPU cache may differ. It receives an atomic baseline and ≤256-byte body/water
groups with correct tick/source revisions. GPU loss invalidates local V1 handles and medium/
image histories; the replica continues consuming bounded state. Required joint fog pipelines
prepare locally before the view is playable; an approximate fog fallback is allowed only if
the selected profile declared its optical and gameplay-cue limits. Input admission remains
an N1/P6 rule, not a render-device side effect. Device recovery never changes the server
water flux ledger or authoring RecoveryRoot.

## Checks that are bounded qualification, not new architecture gaps

The 0.5 ms/tick N1 serialization allocation, 8 MiB proposed reservation, 256 KiB/s peer
bandwidth, 64-tick retention, 64 KiB parser cap, baseline retry policy, water quantization
error and the root's 5/8/7 ms CPU and 10/16 ms GPU envelopes remain **unmeasured targets**.
R2's reduced gameplay population is explicitly a different demo variant; R4 clients cannot
silently drop server-relevant actors. The browser 25 MiB/50 Mbit/s cold-start arithmetic in
BASELINE-SPEC is internally correct (~4.2 seconds transfer), though it leaves <0.8 seconds
for unoverlapped work and may fail qualification. wgpu 30.0.1 default portable limits include
four bind groups and 16 sampled textures/stage; the selected four-group material layout fits
those counts, subject to actual adapter verification. [wgpu limits](https://docs.rs/wgpu/30.0.1/wgpu/struct.Limits.html).

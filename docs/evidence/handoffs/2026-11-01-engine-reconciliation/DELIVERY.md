# Delivery reconciliation handoff

1 November 2026. Paper design only; no build, browser/device run, byte measurement or certification.

## Owned changes

- `BASELINE-SPEC.md`: R0 is one bounded preloaded game with ordinary source admission, local authority, action/result, private save, restart and one export. R2's first portable recipe preloads its gameplay closure and uses restart or visible stop/load for substantive replacement. Wider live authoritative streaming remains required as a separate qualified recipe. R4 refers to `NETWORK-SPEC.md` for encoded fixture and latency classes.
- `CONSTRUCTION-PLAN.md`: I2a (one snapshot/content/save) precedes I3a (useful game/export); I2n uses the same actions and develops alongside I3a before broad I3b; I2b adds region/editor/artist work after the small path. In-memory authority/replica starts in I1/I2a. Early thin mobile boot/suspend/surface checks stay in parallel. Browser authoring is outside first SDK; browser play/cache/save remain.

## Admission arithmetic

- Browser 512 MiB shares: world 76.8, decoded CPU 102.4, persistent GPU 179.2, transient GPU 51.2, staging 51.2, reserve 51.2 MiB. Steady shares total 409.6 MiB. Full world copy at its allowance exceeds staging by 25.6 MiB before providers/products/readers/scratch.
- Mobile 1 GiB shares: world 153.6, decoded CPU 204.8, persistent GPU 358.4, transient GPU 102.4, staging 102.4, reserve 102.4 MiB. Full world copy exceeds staging by 51.2 MiB. Stop/load releases old leases first. Live transitions need a named `M_live + M_staged_W1 + M_providers + M_old/new_products + M_readers + M_scratch + headroom` peak and provider transfer proof.
- Browser cold payload planning caps: R0 8 MiB, R2 16 MiB, below the 25 MiB ceiling. At 50 Mbit/s, ideal transfers are 1.34/2.68 s. Three 40 ms dependency turns and 1.5/1.8 s provisional post-byte critical paths give 2.96/4.60 s, leaving 2.04/0.40 s unmodeled margin to 5 s. R2's margin is tight and wholly unmeasured. R2 scratch phase maxima 40 MiB CPU, 32 MiB GPU, with 48 MiB combined cap inside the 51.2 MiB staging share. Provisional pipeline inventories are 10 R0 and 40 R2 exact-role budget slots, subject to actual descriptor closure.
- Network owner supplied canonical R4 full-group fixture: 4 groups ×32 actors, 56-byte actor records; 7,552 B/publication ×20 =151,040 B/s. With action outcomes/events/clock, 156,248 B/s versus 262,144 B/s state cap; worst quarter-second burst calculated 63,158 B versus 65,536 B. Full groups fit the proposed fixture without delta history. `NETWORK-SPEC.md` owns exact records, reserves and latency; no parallel numeric table was added to baseline.

## Cross-owner alignment needed

1. `FOUNDATION-SPEC.md`, C10 and T1 must complete working files → explicit immutable snapshot admission, native recovery, first-game restart and browser-source-authority deferral while keeping browser runtime cache/player saves. Source owner is editing these.
2. `NETWORK-SPEC.md` and N1 must own exact R4 encoded fixture, atomic full StateGroups, per-action committed outcomes, compatibility, and playable 50/150 ms versus degraded 300 ms/100 ms jitter cases. Network owner is editing these; baseline/construction link to that spec.
3. `RENDER-SPEC.md` should scope 512 total/64 boot/128 live-switch descriptors to reference recipe gates rather than SDK-wide ceilings. The actual R0/R2 descriptor-key inventory and cold pipeline time still need construction evidence.
4. Platform/runtime and C05 should distinguish first R2 preloaded/stop-load from later scoped live authoritative streaming. Client visual residency and N1 interest do not substitute for server region activation.
5. Current maturity/status text should describe analytical admission separately from implementation or device qualification. This worksheet closes no measured browser/mobile gate.

## Independent N1 cross-check sent to its owner

- The 63,158 B quarter-second fixture charges two empty 9,904 B baselines, but a replacement baseline may include up to 256 unacknowledged results (8,192 B) and 128 events (5,120 B). Two such baselines plus regular state exceed the 65,536 B bucket unless sends are paced or replaceable state suppressed. Recalculate the admitted resync case.
- R4 needs concrete client action payload sizes/rates against 32 KiB/s; 120 records/s with every protocol-allowed 256-byte payload does not fit. Specify a recipe-specific smaller action inventory.
- A result's `resolvedTick` can precede the 20 Hz group's `commitTick`. The group should pair its watermark with controller state at its own commit tick; wording that a rejection appears with state at the resolution tick needs clarification.
- `BaselineAck` needs an explicit release rule for mandatory results/events included in an atomically admitted same-epoch replacement baseline. Header/chunk ACK cannot release them.
- If input 7 remains future-pending while input 8 resolves now, the watermark remains 6 yet controller state includes input 8. A replacement baseline must carry individually resolved results above its contiguous watermark too, and its ACK must release exactly those carried records.
- Native mobile OS suspend/resume needs a client epoch/close or fresh-baseline policy with authority continuing and uncertain old one-shots discarded. At cap pressure, distinguish refusal before sequence admission from the promised committed/rejected result for an accepted sequence.

## Product check

The newer 1 November custom-engine selection in `DESIGN.md` and `ARCHITECTURE-BRIEF.md` supersedes the earlier Rust/Bevy-branch direction for the cozy concept. The first-game reduction retains reusable 3D, orthographic/isometric/third/first cameras, Mac/PC/browser/native iOS+Android, quality/visual ambition, AI development/performance workflows, durable storage lifecycle and first-class optional multiplayer. The brief now records E14–E20 for analytical research, first-time use, AI optimisation, lifecycle, simplicity and multiple providers. No additional product requirement gap found in the sections inspected.

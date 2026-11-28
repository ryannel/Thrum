# AUDIO log

- 2026-11-27 User: approved the Game SDK's design and answered its scope question 3, audio in the first release, with the planner's recommendation: yes, in the small form, behind an engine module (as the lead's task and [the module list](../ARCHITECTURE.md#track-7-the-game-sdk) record it).
- 2026-11-27 Lead, in the task for this plan: "No agent can hear, so items that judge sound wait for the owner's ear; keep those few and clearly marked."
- 2026-11-27 Planned. Design, plan and the items of WP1 and WP2. Five work packages; `GAME-SDK` WP18 waits for WP4.
- 2026-11-27 [Planning probe](../../../.orchestrate/evidence/AUDIO/planning-probe.md): a mixer-shaped loop gave one digest natively, under Node and inside Chrome's audio worklet at every pull size tried; the worklet ran an import-free WebAssembly module with no cross-origin isolation and kept its rate through a 500 ms stall of the page; the Mac's default output unit took 48 kHz stereo float at 512 frames a pull.
- 2026-11-27 Chosen: one software mix on every target, pulled; own outputs (an audio unit on Apple, an audio worklet with its own WebAssembly instance in Chrome) and not `cpal`, whose web backends either mix on the page's thread or need cross-origin isolation; QOA for music, decoded inside the pull; 16-bit samples for sounds; `lewton`, `hound` and `rubato` in the cook only. [The design](DESIGN.md#techniques-and-evidence) has the account of each.
- 2026-11-27 Settled the SDK's open question "which sound formats a `MusicAsset` streams from": either source format, `.wav` or `.ogg`, cooks to the one streamed form.
- 2026-11-28 [Design review 1](../../../.orchestrate/evidence/AUDIO/design-review-1.md) asked for changes, with 11 Must Fix findings. Revised the design, WP1 and WP2; [what changed for each finding](../../../.orchestrate/evidence/AUDIO/design-revision-1.md).
  - Findings 1 to 3, in the design's mechanisms 2, 10 and 12: plays carry an event identity, a tick and the output epoch and end in one outcome (WP1.6, new WP1.11); outputs open silenced with an output tap, and a handle run goes through each device; Chrome's port is bounded by the worklet's answers.
  - Findings 4 to 8, in the items: WP1.3 names which plays start; mechanism 5 is the one emitter policy WP1.4 judges by; mechanism 8 gives the fade's gains and a tail slot, checked in WP2.4; WP1.1(7) holds pitch 0.9 to 1.1; WP2.1(3) holds a resampled tone's image 40 dB down.
  - Findings 9 to 11: WP1.7 claims allocations only and WP1.4(7) watches the release; WP2.3 drops the encoder-error comparison; WP5 and its owner sitting are removed, with `GAME-SDK` WP19 the one acceptance point.
  - Suggestion taken: the plan's assumptions say a track is downloaded whole in Chrome before its first play.
- 2026-11-28 For the planner of WP3 and WP4:
  - WP3 carries the silenced opening and the output tap on the Mac and in the simulator, the handle run on both, the reference script's digest folded from the tap, and a suspend that expires a play caught by it.
  - WP4 carries the same in Chrome, and the bounded port: a track of minutes transferred while plays and listener updates are made each frame, with every play in the tap or given an outcome, the last listener applied, the in-flight high-water marks inside their bounds and no pull skipped. It also carries the engine's guide to sound with the listening room as its runnable example, the maturity ledger's entry with "heard" left unjudged, and the Chrome risks the design numbers 1 and 2.
  - WP4 is the module's last work package, so it also carries a check that the listening room's sounds and music are in `docs/provenance/AUDIO.md` with permissive licences, and an item that deletes whatever of `scripts/audio.py` only a Record used, keeping the build of the mix module. No item carries `human_feedback`.
- 2026-11-28 For the project planner, since this planner changed no other module's file:
  - The module list's interface row for sound output says "until `AUDIO`'s design defines it". It can point at [What AUDIO provides](DESIGN.md#what-audio-provides), and `GAME-SDK` WP18's `depends_on` is `AUDIO:WP4`.
  - `GAME-SDK` WP18 gives each play the event identity and tick of the event behind it, and may read outcomes for `screen.sounds()`. It forwards the app's lifecycle to the output's suspend and resume.
  - `GAME-SDK` WP19's sitting is the one place the owner judges sound. [The design's list of nine](DESIGN.md#what-only-an-ear-can-judge) is the engine's part of what to listen for.
- 2026-11-28 [Design review 2](../../../.orchestrate/evidence/AUDIO/design-review.md) asked for changes, with 4 Must Fix findings. It was the second in a row, so the design left by this revision is approved without another review. [What changed for each finding](../../../.orchestrate/evidence/AUDIO/design-revision-2.md).
  - Finding 1, a change part-way through a cross-fade took level out of the music. Mechanism 8 has no tail now: both streams of the fade under way fall across the whole of the new fade, so the squares of the gains sum to 1 throughout. A change while three streams sound waits for the fade to end, and only the last one asked for waits. WP2.4(5) to (7) are rewritten to those gains, with the change made at 1/20 and at 1/2 of the first fade and the summed squares held at every frame. WP2.6 and the performance tripwire render three streams.
  - Finding 2, a cook that encodes the wrong channel passed. New WP2.3(3) compares a cooked stereo track with its source, a different tone in each channel, from a `.wav` and from an `.ogg`.
  - Finding 3, an outcome could not be matched to its play. Mechanism 2 gives every play call a play number, which the call returns and the outcome carries. Outcomes have no promised order. WP1.11(1) to (3) check the numbers, and two plays of one event with different fates.
  - Finding 4, four plays could be admitted and one mixed. WP1.3 is rewritten on a sound of one constant value: every frame holds that value times the number of started plays that cover it, with the frames named for each case.
  - Suggestion taken: the glossary's Emitter entry follows mechanism 5. Outcome names the play number, and Play number is a new proposed name.
- 2026-11-28 For the project planner: `GAME-SDK` WP18 keeps each play's place and volume under the play number its call returns, and reads `screen.sounds()` from the outcomes whose fate is `started`, matched by that number.
- 2026-11-28 WP1 built the 48 kHz buffer, the game-side handle and the import-free mix module from the same
  mix source. Set the initial 64-voice pool to 48 one-shots and 16 emitters, with 256 commands, 512 outcomes,
  256 sounds, a 128-frame ramp, a 1/128 glide step, a 0.75 knee, a 0.25 listener `near` and a 0.125 emitter
  margin. The placed-stereo downmix weights the channels equally and preserves the level of identical channels.
- 2026-11-28 WP1 established reference digest `cf7b0128f410d7db`. The
  [proof record](../../../.orchestrate/evidence/AUDIO/WP1/README.md) gives the Mac buffer, Node and source
  checks. The [module command](../../../crates/thrum-audio/README.md) builds the artifact for the web build step.
- 2026-11-28 WP1 review repair. State retries leave room for emitter changes under continuous control
  writes. Resume reconciles slots with the restored state, and dropping the mix closes its epoch. The mix
  completes outgoing ramps after PCM exhaustion, glides stereo routing, and counts same-sound emitters and
  audible releases. A radius exit can extend beyond the ordinary ramp to respect the glide step after pan.
  Active slot indices preserve summation order without visiting vacant slots on each frame.
- 2026-11-28 All ten automated WP1 item checks and the source audit passed on their specified targets.
  The Mac release suite passed 44 tests, and Node matched the native reference digest at both required pull
  sizes. The [proof record](../../../.orchestrate/evidence/AUDIO/WP1/README.md) includes the full-pool and sparse
  timings. The later output work packages own the device routes.
- 2026-11-28 Second WP1 review repair. Deferred state gets a queue turn during continuous play and stop
  requests. A hold and release preserve the loop phase and use one envelope while voice gain glides wait.
  A settled hold keeps its slot and sample lease outside the per-frame voice list.
  [Mac, Node and source proof](../../../.orchestrate/evidence/AUDIO/WP1/hold-repair/README.md)
  includes all ten automated checks and the held-voice cost under shared load.
- 2026-11-28 WP1 resume repair. A deferred retry with queue space publishes the current desired state before
  enqueueing it. The command revision now describes the same emitter state a resume can restore, including
  when the output stops again before the queue drains. A full queue still returns immediately.
- 2026-11-28 All ten automated WP1 item checks and the source audit passed. The Mac buffer probes reached
  silence when a restored emitter ended, 0.125 when it released, and 0.0625 after a volume change during
  queue pressure. Node matched `cf7b0128f410d7db` at both pull sizes.
  [Target proof, cost and source audit](../../../.orchestrate/evidence/AUDIO/WP1/resume-repair/README.md)

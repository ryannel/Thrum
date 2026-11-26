# APPLE-PRESENTATION plan

Reconstruction, display and the quality governor (gate G4), governed by
[the G3t boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) and C04, C06, C07 and C12.
[Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list and seam rulings](../ARCHITECTURE.md#interfaces) · paired plan [APPLE-TILE](../APPLE-TILE/PLAN.md)

Read the [design](DESIGN.md#wp4-to-wp12-settled-14-november-2026) and seams 5–9 before starting.
Build in the order below.

## Open for the owner

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: the governor drops scale after three frames over budget, recovers at most one step per second,
  and makes at most two scale changes on an unloaded route.
- Assumption: at full window size the frame rate may dip until the governor lands in WP7. Record times for
  that interval without applying a frame-time gate.
- Assumption: WP10 pins wgpu-hal 30.0.1 for the Metal library loader. WP12 adds the archive to that copy.
- Assumption: WP12 also pins wgpu-core 30.0.1, changed only so that a pipeline cache's header can be written and
  checked on Metal. Released wgpu-core does that for Vulkan alone, so the archive cannot be saved or loaded
  through wgpu without it. The lead took this route on 26 November; the owner can overturn it.
- Assumption: WP16 keeps the device safe for timestamp query sets created at the engine's one call site. Code
  that creates one with wgpu directly, in the same process and at the same moment as an engine request, is not
  protected, because that needs a change to wgpu's Metal backend. The owner can ask for that change.
- Assumption: a viewer cannot see a brightness difference under 1% of a frame's mean luminance, so D8.3 allows
  MetalFX that much. It is a little under one step of the 8-bit output at the brightness of D8.3's frames.
- Assumption: "later" for offering the cache upstream means after WP11, not after the V1 exit. So
  U1's notes are written while V1's last modules build, and nothing waits for the owner's answer in U2.
- Assumption: agents draft no text for the owner to post upstream, not even a comment to rewrite. The upstream
  notes give facts, options and a recommendation, and the owner writes the comment. The owner can ask for drafts.
- Assumption: the vision's "smooth at the display's full refresh" holds at full screen on the built-in display,
  at its pixel size. So WP11's steady 60 Hz there is an interim rate, the full 120 Hz stays owed by WP17, and
  the milestone is reached at WP17. The owner can accept 60 Hz at full screen instead, which changes the
  vision's bar and is theirs alone to say.
- Assumption: until WP17 shows 120 Hz held, a full-screen window asks for 60 Hz by itself and a smaller window
  keeps the display's full refresh. A launch setting forces either.
- Assumption: WP17 comes after WP11, so `SHOWCASE` WP7 does not wait for it, and the V1 exit rests on it.
  `SHOWCASE` WP8 depends on WP17 and its WP8.12 lists it. The owner can take WP17 out of the exit, for instance
  by leaving it to the performance phase the vision puts after V1.

## WP1 Every pixel of the valley carries correct motion under jitter, including the sky and stars: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP1-review.md)

## WP2 Depth, motion, reactive and exposure are stored for reconstruction, and history advances only with submitted frames: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP2-review.md)

## WP3 TAAU reconstructs the valley at native scale in the game, on the Mac and in Chrome: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP3-review.md)

## WP4 The valley fills the window at any size, and keeps presenting while the window is resized: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP4-review.md)

## WP14 The retained native valley images reflect the corrected material mip bias: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP14-review.md)

## WP13 A small exposure check catches a twilight brightness rebound: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP13-review.md)

## WP5 Render scale steps during play with no hitch and no history reset: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP5-review.md)

## WP15 Chrome's SDR valley matches the retained frames again: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP15-review.md)

## WP6 Highlights go brighter than SDR white on the built-in XDR display: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP6-review.md)

## WP16 The long tests pass again, and running out of GPU timers is a named error, not a lost device: done

## WP7 The governor holds the display rate under forced load, then recovers step by step: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7-review.md)

## WP8 MetalFX reconstructs the valley on Apple tile, with TAAU as its prepared fallback: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8-review.md)

## WP9 Surface, device and display changes create nothing during play, and launch-to-ready time is on record: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP9-review.md)

## WP10 Shaders compiled ahead of time load at launch with no WGSL translation, and pipelines are prepared in parallel: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP10-review.md)

## WP12 A warm launch compiles no pipeline: the local pipeline archive is proved by a strict fresh-process run: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12-review.md)

## WP11 Apple tile with MetalFX and EDR is compared with portable TAAU on the route, and each clause of G4 is proved

`APPLE-TILE` WP12's record holds the fixed-extent baseline and the eligibility rows this proof reruns with real
reconstruction readers and scale steps. The evidence page is `.orchestrate/evidence/render/G4-PAIRED-<date>.md`, and the
ledger is `docs/design/maturity.json`. Ask for the owner's look at the panel in the evidence page and in your
report, and never wait for it: WP11 closes on the agent and device evidence, with the look recorded as
outstanding.

Read [the requested frame rate at full screen](DESIGN.md#the-requested-frame-rate-at-full-screen-wp11) first:
it answers the 26 November handoff. Full screen runs at an interim 60 Hz, and no more work on the frame graph
belongs here; the three costly passes are WP17's. `build/apple-presentation` at `1eacafa62` holds the two
changes that section keeps, and
[the retake](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11/mac-budget-retake/README.md) is the 120 Hz
shortfall record. What is left: the requested frame rate and its launch setting, the two pacing takes, the cue
windows with the capture sized for them, the paired proof again on the merge candidate because the EDR pass
changed, and the closing record.

Pacing takes run under whatever load the host has, so repeat one that a busy host spoils and keep every take;
don't end `BLOCKED` asking for a quiet machine. If six takes in a row miss only for an absent or a late
callback, end `NEEDS_CONTEXT` with those takes: a sitting with the other builders paused is the lead's to grant.

## WP17 The valley holds the display's full 120 Hz at full screen on the built-in display

## U1 The owner can read, in one reviewed document, what Thrum's Metal pipeline cache would offer wgpu and what to settle there first

Not part of V1. [The design](DESIGN.md#offering-the-cache-upstream-u1-to-u3) gives the owner's rules, what no
agent does, wgpu's side as read on 26 November, and the six parts of the upstream notes; it is all of the
design this work package needs. The two cache patches are `vendor/wgpu-hal-patches/0002-metal-pipeline-archive.patch`
and `vendor/wgpu-core-patches/0001-metal-cache-header.patch`. WP12's launch record is D12.4's evidence,
[the paired record](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12/review-repair/registry-first-run/launch.json);
[its README](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12/review-repair/README.md) also links the rerun
on the merge candidate, which is a second record and not the one part 5 quotes. `gh` is signed in
here: use it to read wgpu's issue, pull requests and files, and for nothing that writes. The notes are evidence,
so the landing is a log entry and no more, and `./dev check-docs` is the check to build against. U1.4's
reviewer reads what each session ran from the host's own record of it, found through `sessions.jsonl` in the
folder `agent_loop.py settings` prints as `paths.history`, so there is nothing to keep for that part.

## U2 The owner has said go ahead or stop on offering the Metal pipeline cache to wgpu, and the answer is on record with whatever the issue's thread settled

The upstream notes are U1's evidence; U1.1's `evidence` field has the path. Do U2.1, then end
`BLOCKED needs_human_feedback` with the question U2.2 carries. The owner answers when they choose: a stop may
come at once and with nothing posted, and a go ahead follows the owner's own comment and replies that can take
weeks. A long wait here is expected and is not a fault to work around. The log entry follows the owner's answer.

## U3 The Metal pipeline-cache change is offered to wgpu by the owner: on trunk, tested in wgpu's suite, reviewed and walked through

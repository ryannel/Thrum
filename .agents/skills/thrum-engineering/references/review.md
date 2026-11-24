# Review

What a Thrum reviewer checks, in addition to the orchestrate reviewer skill. The owner-graded verdicts are in
`docs/design/GRADED-EXAMPLES.md`. They are the best guide to how strict to be.

Contents: [The bar and the weight of a review](#the-bar-and-the-weight-of-a-review) ·
[Design reviews](#design-reviews) · [Code reviews](#code-reviews) ·
[Checks the script already ran](#checks-the-script-already-ran) · [Visual reviews](#visual-reviews) ·
[Performance claims](#performance-claims) ·
[Calibration: failures this engine has already paid for](#calibration-failures-this-engine-has-already-paid-for)

## The bar and the weight of a review

The owner's bar (17 November 2026, in `ORCHESTRATE.md`): rendering and visual work passes when it looks as good as
a shipped high-end game against its inspection list and runs fast. Judge to that bar.

- **A reference is a diagnostic, not a finding.** That a shader, a light model or a GPU product differs from a
  physical or `f64` reference is not a Must Fix. It becomes one only when the difference shows as a failed
  inspection item, breaks a contract between modules, or changes what a query answers a game. Don't ask a design
  or a work package for a reference, a ceiling or a measured bound for how something looks.
- **A deliberate change to an exact image is not a regression.** It needs the refresh record, not a model that
  justifies the new pixels.
- **Keep full rigour where a failure is silent and costly:** networking and transport, simulation determinism,
  cooked formats, and the contracts between modules.
- **Weigh the review by the risk.** New rendering work gets the whole inspection list. A fix round is judged on
  the items that failed and on what the fix touched; pass the rest from the earlier review unless the fix could
  have moved them. A tripwire record is judged by its check, and by a question only in a module's last work package.
  A design re-review after a change of bar or proof checks the changed parts against the planning inputs.
- **A Must Fix is something a player, a game maker or another module would meet.** Paperwork that could be
  tidier is a Suggestion.

## Design reviews

Read the vision and scope document, the planning task and any module brief it references. Derive what the
module must do before reading its design. Look for three kinds of gap.
- omissions;
- additions the planning inputs don't support;
- couplings that can't work.

An explicit, safe deferral is not an omission.

For each defining cue, ask:
- **Can the mechanism produce it across the operating range?** A numerical reproduction of the core maths often
  settles this quickly. Ask whether the cue would read on screen, not whether the numbers match physics.
- **Is a modern alternative missing?**
- **Is the method heavy or dated by design?**

Walk a normal frame and a coupled interaction through the real producers and consumers, and note where an
implementer would still have to invent a mechanism, a query or an update order.

Look for shortcuts that pass at the demo point and fail elsewhere:
- fixed domains;
- constant depths;
- hard-coded origins;
- wrapping boundaries;
- single-regime constants;
- a local field that replaces the module's other regimes instead of composing with them.

## Code reviews

Read callers, shared data, shaders, fixtures, lifecycle and tools together, because the faults sit at the joins. Look
for:
- duplicate producers;
- mismatched clocks or encodings;
- dead alternative paths;
- unbounded catch-up loops;
- allocation on a hot path;
- readbacks to make progress;
- redundant transforms.

Check that the change uses the codebase's adopted shapes for recurring jobs (the table in this skill's `SKILL.md`).
**A change that adds a new way to do an existing job is a finding**, even if each line is clean. Name the adopted
shape it should have used. Most High findings in the 14 Nov 2026 code review were splits of this kind, and each
started as one reasonable-looking change.

Also check that the change doesn't reach across a crate boundary through `#[path]`, `include_str!` or a
dev-dependency on an example.

## Checks the script already ran

The script ran `./dev check` and every item's check on the merged commit before it asked for this review, and
saved each check's whole output. For a check that takes the `gpu`, `chrome` or `simulator` lock, read that
saved output and don't run the check again: a second run takes the device from a builder and tells you what
the first already did. Run one again only for a doubt you have written down first, such as output that names
another commit or a result the code can't produce.

Spend the time where reviews here have found defects. Of 21 Must Fix findings in 40 reviews, 14 came from
reading code or from a reviewer's own CPU probe, one from the exact verifier and one from stills. So read the
code, write a small CPU test for anything you doubt, and look at the captures.

The scheduled main-branch check `exact-images` verifies the registered images after merges.
For a planned refresh, read the work package's verifier record under
[the baseline rules](../../../../docs/plans/ARCHITECTURE.md#exact-baselines).

## Visual reviews

Look at captured images and sequences yourself. Say whether you watched motion or sampled stills, at native size or
downscaled, and which build you saw. A reviewer that can't play video asks for a frame strip at 10 fps or more over
the window in question.

Judge each indispensable cue separately, against the work package's inspection list and its shipped-game bar. Good
lighting or performance doesn't make up for failed dynamics. Cover the transitions (start from rest, stops, turns,
camera cuts), because onset faults hide there.

## Performance claims

Challenge:
- the benchmark's identity and fairness;
- the tails, not just the medians;
- equivalence claims;
- the work actually saved;
- the memory accounting;
- contention on the shared GPU.

A timing taken while another agent used the GPU is labelled as contended, or it doesn't count. Inside a
work package that is expected: such a timing is a tripwire, and it fails only on a collapse or a missing
measurement. That holds in a module's last work package too. A record taken under load is not a finding, and no
review asks for a sitting with the other builders paused unless the module's design names one. A figure quoted
without its load is a finding.

## Calibration: failures this engine has already paid for

Look for these first. Each one passed its author's own checks.

- **Tuning a mechanism that can't make the cue.** Hours of gain, resolution, foam and shading work went into a wake
  whose wave model couldn't form a Kelvin wedge at varying speed. A short CPU reproduction showed it.
- **Correct but invisible.** Physically calibrated wakes and foam were true to scale and unreadable at the game camera.
- **Judged in the lab.** A cloud look judged at seconds per frame shipped as a 288 ms fog. Critics required close
  views and offline-paced clips while the real run stuttered.
- **A plan that only grows.** A cloud plan grew to 399 lines of amendments, and a water plan was a list of effects in
  build order with the speed range never designed. Neither had a criterion that moved.
- **Trials without a governing question.** Nine or more cloud method families were trialled with no question or
  stopping point, and six rounds of horizon-shelf fixes all held base height, profile and lighting fixed.
- **Swinging between cost-only and look-only work.** Chasing invented budgets before the look existed, then ignoring
  cost, produced designs that were heavy by design.
- **Copying the better-looking prototype.** An earlier implementation looked better but was too heavy. It was a look
  bar, not a technique.
- **Attractive frames without the behaviour.** Moving texture passed for growth and breakup. A fade or age label is
  not structural change.
- **Paperwork displacing the defect.** Notes and qualification grew while the visible defect stayed.
- **Verdicts from descriptions.** Techniques were endorsed from their write-ups, and critics' passes were relayed as
  fact. Both proved wrong.
- **Copying the nearest example.** Each work package copied the closest working code, so GPU data packing, errors,
  device setup and GPU test admission each ended up done three to five different ways. The copies then drifted until one
  of them broke a platform.

# Orchestrate profile: Thrum

Thrum's work follows the generic orchestrate skills (`/plan-module`, `/build-milestone`, `/review-milestone`,
`/build`). This file holds what's specific to Thrum.

- **Purpose:** [the brief](docs/plans/BRIEF.md): purpose, the V1 exit and the gates.
- **Quality bar:** looks as good as a shipped high-end game of this kind, named per milestone, e.g. sea and wakes as
  in Sea of Thieves, skies as in Red Dead Redemption 2, and runs fast, smooth at the display's full refresh. Judged in
  words against an agent-written inspection list, never against target images or a physical reference.
  [What is proved, and how hard](#what-is-proved-and-how-hard) says what that asks of a plan, a build and a review.
  *(owner, 17 Nov 2026: "Ok let's lower the bar.")*
- **Plans:** docs/plans
- **Evidence:** .orchestrate/evidence
- **Graded examples:** docs/design/GRADED-EXAMPLES.md  *(loop: past verdicts the owner agreed with or overturned)*
- **Test command:** ./dev check  *(loop: CI is manual-only, so this is the gate; documentation-only work may run ./dev check-docs while building)*
- **Merge:** push  *(loop: owner allows verified checkpoints straight to main)*
- **Branch:** main  *(loop)*
- **Remote:** origin  *(loop)*
- **Builders:** ten Codex builders
- **Codex model:** gpt-6.2-sol  *(loop; owner, 17 Nov 2026: moved from gpt-6.1-sol to the cheaper, stronger 6.2 models)*
- **Claude model:** claude-opus-5-5  *(loop)*
- **Default host:** codex  *(loop: builders)*
- **Review host:** codex  *(loop: owner, 11 Nov 2026: "move milestone reviews to astra on medium"; design reviews stay on Codex since Claude plans)*
- **Planner host:** claude  *(loop)*
- **Claude planner:** claude-fable-5-2 high  *(loop: owner, 10 Nov 2026)*
- **Codex design reviewer:** gpt-6.2-astra high  *(loop: owner, 10 Nov 2026; owner, 17 Nov 2026: moved from gpt-6-astra to 6.2)*
- **Codex reviewer:** gpt-6.2-astra medium  *(loop: owner, 11 Nov 2026: "move milestone reviews to astra on medium"; owner, 17 Nov 2026: moved from gpt-6-astra to 6.2)*
- **Claude reviewer:** high  *(loop: owner, 10 Nov 2026; Opus 5.5; used only when a review is sent to Claude by hand)*
- **Codex sandbox:** danger-full-access  *(owner, 14 Nov 2026: agents get full access; the Mac stays unlocked)*
- **Writable roots:** ~/.cargo  *(loop)*

## Starting from a clean clone
`./dev help` lists the commands; `./dev check` builds and tests the workspace. Builders work in their own worktree on
a `build/<module>` branch and push `HEAD:main`. Large captures and recordings go in `tmp/<task>/` under
[the storage rules](docs/ASSET-LIFECYCLE.md). Evidence is never committed: run records, reviews and captures go in
the working folder, `.orchestrate/evidence/<MODULE>/`, and the merge refuses a new or changed file under
`docs/evidence`. Data that a check reads is test data, kept in `tests/fixtures/<module>/` or beside the crate's
tests. A record a person must read to use or publish the code is not evidence either: a module's provenance
record, which lists each file copied or translated from elsewhere with its source and licence, is tracked at
`docs/provenance/<MODULE>.md`, and a design or item that names `PROVENANCE.md` in a module's evidence folder
means that tracked record. *(owner, 20 Nov 2026: provenance records are tracked in Git)*

## Targets

| Target | What it is | Run | Capture | Edge conditions |
|---|---|---|---|---|
| mac-metal | This M3 Max, 120 Hz ProMotion, Metal | `scripts/run_v1_showcase.py`, `./dev` scenarios | [T2 engine capture](docs/plans/T2-CAPTURE/DESIGN.md): fixed-time images and elapsed-time motion, lossless sources/contact sheets; active-panel rows separate | Low sun, storm, distance, fast camera, full refresh |
| chrome-webgpu | Chrome, portable WebGPU recipe | the browser showcase | [T2 engine capture](docs/plans/T2-CAPTURE/DESIGN.md) M1: submitted fixed checkpoints, realtime output and time records; DOM/browser chrome separate | Same scenarios as mac-metal |
| ios-sim | iPhone 18 Pro simulator; function only, never speed | the iOS route | [T2 engine capture](docs/plans/T2-CAPTURE/DESIGN.md) M2: engine output/time records; `simctl` screenshots/video corroborate simulator function/UI/lifecycle within recorded timing limits | Rotation, background and foreground |

Rules for proof on these targets:
- A visual milestone that claims submitted output or elapsed-time engine motion uses T2 engine capture on its
  target and names the capture mode it selects: `still`, `fixed` or `realtime` on the Mac, `fixed` or `realtime` in
  Chrome. Chrome endpoint images and `simctl` captures alone cannot prove submitted temporal output or elapsed-time
  engine motion. Simulator evidence is function only.
- Engine capture needs no OS Screen Recording grant, by design. A check that claims independence from desktop
  capture requires denied, attributed read-only Screen Recording preflights. Console state is recorded as a
  witness, never an admission or pass condition and never a manual-lock wait. Evidence accepted from locked runs
  stays valid history.
- Built-in-panel cadence is **attended setup**: an unlocked active panel and any Instruments authorization, with
  the agent's actions and review automated. Cadence is qualified with capture OFF; capture-ON arms show content
  and observer cost separately.
- OS-window capture, as in `APPLE-TILE`'s actual-window checks, separately needs attended unlocked setup and Screen
  Recording permission for its observer. That grant is not required by the engine tap and is not inferred from Mac
  panel setup. Native or compositor UI and physical output checks name their own attended reason.
- Never treat engine pixels or movie PTS as display swaps or photons.

Rules for when a proof runs ([why](docs/plans/ARCHITECTURE.md#proofs-run-once-on-main), 15 November 2026):
- **The proof of record** is the script's run of `./dev check` and every item's check on the merged tip. Prove a
  thing on your branch once, merge, and don't prove it again after a rebase: rerun only what the script says
  failed. A visual milestone takes one capture pass, after its last merge.
- **Exact images** are verified on main after the merge, by the main-branch check `exact-images`. No proof is
  needed before a merge. An item whose check is `python3 scripts/exact_baselines.py verify` passes from that
  shared run: set it `pass` with the evidence `main-branch check exact-images`, and don't run it yourself.
- **A lock is about capacity.** A command takes one lock, for the device it loads most, around the part that needs
  it. It never takes a second lock and never runs `merge` inside one. `gpu.alone` is only for a run that needs the
  whole GPU, such as all its memory. It doesn't make a timing trustworthy, because builds elsewhere take no lock.
  The `locks` command lists only the runs that took a lock. The GPU tests inside `./dev check`, a builder's or a
  merge's, take none, so the GPU can be busy while `gpu` reads free. That is expected, and nothing to report.
- **Nothing waits for a person under a lock.** Before an attended run, check that the console is unlocked and the
  panel awake, and only then take the lock. If they aren't, end `BLOCKED needs_access` at once.
- **Timing is a tripwire, in a module's last milestone too.** It is taken while other builders work, under the
  lock its run needs, records the load, and fails only on a collapse or a missing measurement. Nobody is paused
  for a timing, and no builder ends `BLOCKED` asking for a quiet machine. The evidence and the maturity ledger
  call the figures taken under load. Figures for a quiet machine, where they are still wanted, wait for one
  window the lead batches for every module, and no milestone waits for it. The one exception is a sitting that
  a module's design asks for by name, because smoothness at 1x is judged in it, as `SHOWCASE` M8's is: that
  builder ends `BLOCKED` asking the lead for it, and the lead keeps the others paused until the milestone is
  Ready for Review. *(lead, 18 Nov 2026, on the owner's bar of 17 Nov; from 15 Nov every last milestone asked
  for a paused sitting, and `CULLING` M3 and `SCENE-VIEWS` M3 each stopped on it)*

- **A Done item's check is its record.** Nothing runs a Done milestone's item checks again: the script compares
  only what the item asks for, and `./dev check` runs no item's check. So later work may move, rename,
  merge or delete a file, a test target or a script that only Done items name, and it leaves the Done item's
  text as it was. Nobody edits a Done item to follow a move, because a changed item un-Dones its milestone.
  Three things still bind: a name an open item's check uses, a registered exact baseline with its comparison
  files, and what `./dev check` runs. A test that `./dev check` runs is still a proof, so deleting or
  shortening one is planned as an item that says what guards the behaviour afterwards. *(planner, 20 Nov 2026,
  from the cleanup review: about 770 path mentions in Done items were being treated as frozen)*

Label Mac stand-ins for iPhone `proxy:iphone`. No physical iPhone is available, so don't wait for one.

## What is proved, and how hard

Owner decision, 17 November 2026: "Ok let's lower the bar." Thrum is a game engine. The aim is the most plausible
result as fast as possible, not physical accuracy. Planners write items to these rules, builders prove to them and
reviewers judge by them.

- **Rendering and visual work passes on look and speed.** The look is judged in the running game against the
  milestone's inspection list and its shipped-game bar. Speed is a tripwire record: no budget is invented.
- **A physical or f64 reference is a diagnostic.** Use one when output looks wrong, to find the cause. No item
  holds a shader, a GPU product or a light model to a reference within a bound, and a plan never asks a builder
  to measure a bound first and then assert it. A miss against a reference is not a finding. A builder may tune
  by eye where the list fails, and says in the evidence what was changed.
- **An exact image baseline guards against a change nobody meant.** A deliberate shader change refreshes the
  references in its own landing, under [the refresh procedure](CONTRIBUTING.md#exact-image-baselines-and-the-reference-guard),
  and no review asks it to justify
  the new pixels by a model.
- **Correctness proofs stay where a failure is silent and costly.** Those are networking and transport; simulation
  determinism; asset and cooked formats; and the contracts between modules, which are the frame contract's
  outputs, one owner for each kind of state, lifecycle through pauses, hitches and a replaced device, a capability
  left out costing nothing, and what a query answers a game. GPU state that a game or another module reads through
  a CPU query (where water lies, where cloud stands) is tested against that query at a tolerance the test states.
  Output that must equal a simple statement of itself, such as a sprite's texels or a culled image against the
  unculled one, is a regression guard and stays.
- **Proof is proportional to risk.** New rendering work gets a full inspection. A fix round is reviewed on what
  the last review found and what the fix touched. A tripwire record passes on its check alone, with a reviewer's
  question only in a module's last milestone. An evidence item that costs
  more than the fault it would catch is not written.

Three rules for how a planner words an item *(planner, 20 Nov 2026, from the cleanup review)*:

- **An item names a behaviour, not a place.** It says what must be true of the engine, a tool or a record, and
  names a file only where that file is the thing delivered. `SHOWCASE` M9.6 failed review over a move that kept
  every byte, because its items named folders.
- **A check that asserts absence fails when its subject is missing.** `! grep X path` and `test ! -e path` pass
  when `path` has moved, so the check proves nothing from then on. Search the whole of `crates` and `examples`
  with `git grep`, count what must exist exactly once, or test first that the thing searched is there.
- **No item asks for a tracked file under `docs/evidence`.** The merge refuses one. A record a reviewer reads is
  "this milestone's evidence", in the working folder. A file that a later check reads is test data, and the
  item says so. A provenance record is neither: it is tracked under `docs/provenance`.

Three rules for the tests a plan adds *(owner, 21 Nov 2026: "Yes, plan it"; they bind an item written or
re-planned from that day)*:

- **A check is a guard or a record, and the item says which.** A guard is a test or a rule that stays in the tree
  and runs in `./dev check` from then on, so it meets [the test tiers](#test-tiers)' limits like any other test. A
  record runs once, to show the milestone: a capture, a measurement, a before and after. Its script and data are
  the milestone's evidence where they can run from there. What has to be tracked for the script's own run is
  deleted by the module's last milestone, which carries an item for it. The item's statement closes with
  `Guard:` or `Record:` and what it is.
- **A plan that adds a GPU test or a main-tier test says what only that test can show**: what a device alone
  shows, or what full size or a long run shows that the smallest world cannot. An item that cannot say so asks
  for a CPU test on a small world. A main-tier test is added only by such an item.
- **A reviewer reruns the tests a change touches, not the whole check**, which the merge already ran, and puts
  [the two questions](#test-tiers) to any new test that takes over about a second. No lint can tell that a GPU
  test should have been a CPU test, so that judgement is the reviewer's.

A milestone that was In Progress or Ready for Review on 17 November 2026 keeps the items it was approved with.

## Test tiers

Owner decisions, 20 November 2026. On the ten-minute gale: "we can't have tests that run for this long in the
regular test suite. We need to think carefully about what risks we're willing to take and when we run everything
vs when our tests can be more targeted. I think it has to stay off every merge check." And later that day, as the
lead gave it to the planner: the merge tier is targeted. "Every merge runs only the tests for what the change
touches, and the full workspace runs on main." So a test runs in the cheapest tier that still catches its fault
before the fault costs more than the test. And once more that day, with the long tests gone and a merge's check
still over twenty minutes: "A 20min merge check still sounds crazy. Tests are meant to be fast." So a merge's
check is planned to take about five minutes for a typical landing, and a test in it about ten seconds or less.
*(five minutes is the planner's assumption until the owner answers; the owner accepted ten seconds on
21 November)* And on 21 November, of four changes to
make tests faster: "These all sound really reasonable." Three are rules below: a test uses the smallest world that
shows its behaviour, logic is tested without a device, and a package's tests are one binary. The fourth is a
measurement of where a builder's session goes, `GATE-TIME` M10, which changes no rule. And later that day, of
guard rules so that those habits hold without anyone remembering them: "Yes, plan it." What must hold every time
is in the merge check, under [What keeps the merge tier fast](#what-keeps-the-merge-tier-fast).

[GATE-TIME](docs/plans/GATE-TIME/DESIGN.md#mechanisms) builds the tiers. Each part below binds from the milestone
that builds it: the long tests' move (M1), the tests behind a Cargo feature (M6), the side-by-side run and the
limit on one test (M8), the selection with the whole run on main (M7), the guard rules (M12), the ten-second
bound (M9, and M3 for the water scene), the stop at ten seconds (M13), and one test binary a package (M11).
Until its milestone is Done, `./dev check` runs what it ran
before. The two rules for how a test is written need nothing built: they bind a test written or changed from
21 November 2026, and M9 and M3 apply them to the tests over ten seconds that exist.

| Tier | When | What runs | Risk it accepts | How a fault gets back to its cause |
|---|---|---|---|---|
| **Merge** | Before every merge | `./dev check`: the build and the lints of the whole workspace, a compile of every test behind a Cargo feature, the repository guards and script suites, and the fast tests of the packages the change can reach, run side by side. Then the merging milestone's item checks, which test what that milestone changed | A fault that only a long run, a full-size world, an exact image or a test outside the selection shows reaches main | The merge is refused, so the cause is the branch |
| **Main** | On main's tip after merges, by the `Main-branch check` lines under [Rules](#rules), each no more often than its `every` | Exact images; the long tests in a release build and the tests behind a Cargo feature; and every fast test in the workspace | The fault sits on main for up to the check's interval and one run, and later merges build on it. It costs the module that caused it; nothing ships from main | A failure is run again once, then traced through the commits since the last pass to the first that fails. Each step of the trace is one whole run of the check, so the commit is named some hours after its merge, not within the hour. The lead gives that commit's module the fix as its next task, or reverts the commit |
| **Milestone and release** | Once, when a milestone claims Ready for Review; and at a release point, which is the V1 exit's closing record or anything published | A milestone's captures, inspections, cost records and sittings, and the test command once more on main's tip, which there is the whole workspace's fast tests unless the merge's own pass is reused. At a release point, everything on the one commit: the whole-workspace check, every main-branch check, and the release's own items | A fault only an eye, a device or a quiet machine shows lasts until the milestone that looks, or the release | The review names the item. At a release point the tier that fails names it, as above |

- **A test in the merge tier is fast: about ten seconds or less**, run alone on this Mac in the build the check
  uses. A test that takes longer is made fast where the time comes from how it was written or built: one case to
  a test, setup made once, a smaller fixture that keeps every assertion. Otherwise it is a **long test**. It is
  marked `#[ignore]` with a reason that starts `long:`, is named in the main-tier list, `.config/long-tests.txt`,
  with the fault it leaves unguarded before a merge written beside it, and runs in the main tier through
  `./dev long-tests`; a test is marked only once a `Main-branch check` line under Rules runs that command. No
  shortened run of it stays in the merge tier for comfort. One stays only where a design shows it is fast and
  catches a kind of fault nothing else catches, in any tier. A proof of a fault that is silent and costly is
  the last to move: it is made fast where it can be. Until `GATE-TIME` M9, and M3 for the water scene, a long test is one over
  about a minute, and the tests between ten seconds and that are on `GATE-TIME`'s exempt list.
  *(owner, 21 Nov 2026: accepted the ten-second limit, and that a new long test needs a plan's decision)*
- **A merge-tier test uses the smallest world that shows its behaviour.** A world is what the test builds to
  run on: a grid, a valley, a cooked store, a count of entities, ticks or frames. The test proves its behaviour
  on the smallest one where the behaviour appears and each assertion could fail: a rule about a solver's step
  budget needs a grid whose solve takes more than a few steps, not the showcase valley solved forty times. A
  run at full size belongs to the main tier. It is kept only where full size shows a fault the small world
  cannot, as a long test with that fault written beside it. The small test stays in the merge tier beside it,
  because it proves the behaviour itself and is not a slice of the long run.
  *(owner, 21 Nov 2026: "These all sound really reasonable")*
- **Logic is tested without a device.** What a CPU can decide is tested in a test that asks for no GPU: a
  selection, a layout, a state machine, a record's bytes, a refusal, arithmetic a model shares with a shader. A
  GPU test covers only what a device alone can show: that a shader compiles and a pipeline is accepted under the
  limits it names, that submitted output or GPU state equals what the CPU side states, that a device's
  lifecycle holds. It draws to the smallest target, for the fewest frames, that show it.
  *(owner, 21 Nov 2026, with the rule above)*
- **Who holds those two rules.** Whoever writes or changes a test writes it to them, and a planner writes items
  to them. A reviewer who reads a new or changed test asks two questions of it, once: what is the smallest
  world that would show this, and what here needs a device? A test already in the tree is held to them only when
  it is changed or when the check reports it as slow. From `TEST-SUPPORT` M6 the engineering skill says how to
  write a test to them, in one block, and the test kit has the small target a GPU test draws to.
- **A package's integration tests are one test binary**, from `GATE-TIME` M11. macOS checks every newly linked
  binary the first time it runs, about a third of a second each, and Cargo makes each file in a `tests/` folder
  a binary. So a test file is a module of its package's one test target, `it`, and its tests run as
  `cargo test -p <package> --test it <file>::`. A new test file is declared there, and the repository guards
  fail a file under `tests/` that no target compiles. A file is a target of its own only while something names
  its binary or it has to run apart: an open item's check, the main-tier list, a long test or a test behind a
  Cargo feature in it, a `required-features` or `harness = false` entry, a build for another target, or a
  script. From M11 a planner's new item runs a file's tests through `it`, and a planner who re-plans a module
  writes the checks of its unstarted milestones the same way. An item approved before M11 keeps its check: its
  file is a target of its own, and joins `it` in the first landing that edits it after the item's milestone is
  Done. Until M11 a new test goes in a test file that exists. *(owner, 21 Nov 2026, approving the lead's
  proposal of fewer test binaries, from 265 towards one a package)*
- **The merge tier is targeted.** `./dev check` runs the test binaries of the packages a change can reach: each
  workspace package that holds a changed file, and every package that depends on one, directly or through others,
  by Cargo's own graph. Where a test reads another package's files when it runs, which the graph cannot show, the
  selection declares the edge: a change under that path selects the reader's package too. The change is what
  differs from the checkout's merge base with main. What would stop
  every builder is not selected: the build, the lints, the repository guards and the script suites run whole on
  every merge. So do the fast proofs of faults that are silent and costly, which
  [the bar above](#what-is-proved-and-how-hard) lists, for every package the change can reach.
- **A shared input selects everything.** A change to the root manifest, the lockfile, the toolchain file, the lint
  configuration, `./dev` itself, `tools/xtask` or the test runner's own configuration runs the whole workspace's
  tests. So does a path the selection has no rule for, and a checkout whose change it cannot compute. Documents,
  the plans, `scripts/` and the data under `.config/` reach no Rust test and select none; the guards and suites
  that read them run on every merge.
- **The risk the selection accepts.** A test whose result depends on something Cargo's graph does not show, such
  as a file another package owns that it reads at run time, is not selected when only that thing changes, unless
  the selection declares the edge. The one such test known is declared; nothing finds a new one when it is added.
  The missed edge reaches main and sits there until the next whole-workspace run, the main-branch check
  `workspace-tests`: up to its interval and one run, and some hours more to name the commit. Until that commit
  is fixed or reverted the script still counts main as passing, and two things fail for a fault they did not
  cause. One is a later merge whose selection holds the failing test. The other is every milestone's Ready for
  Review: after a builder merges, the script runs the test command once more on main's tip, where nothing
  differs from main, so that run is the whole workspace unless no other merge has landed since and the merge's
  own pass is reused. So no merge waits for the whole run, but a fault on main holds the Ready for Review of
  every module's milestones, not only of the module that caused it. A builder whose merge or Ready for Review
  fails in a test its change cannot reach reads `python3 "$ORCHESTRATE_SCRIPT" main-check workspace-tests`
  before it debugs. Where that shows the test failing on main, the builder leaves the test alone and ends
  `BLOCKED main_broken:` with the test's name, and the lead has the commit fixed or reverted. *(planner's
  assumption until the owner answers: the other choice selects the Ready for Review run by the milestone's own
  change, which needs a change to the orchestrate script)*
- **A test behind a Cargo feature** (`creation-counters`, `draw-probe`, `fault-probe`) is compiled in the merge
  tier and run in the main tier: the main-tier list names its package and features beside the long tests, whole
  or by test binary, and `./dev long-tests` runs it. The docs check reads the test files as well as the
  manifests: a feature named in a test file's `cfg`, or required by a test target, that the list does not cover
  fails it. A feature that no tier names is a fault in the list, not a way to keep a test out of a tier.
  *(planner's assumption until the owner answers: the other choice runs these only in the item checks that name
  them)*
- <a id="what-keeps-the-merge-tier-fast"></a>**What keeps the merge tier fast.** `./dev check` runs tests side by side and ends with the time of each of
  its steps and its ten slowest tests, and from `GATE-TIME` M12 with the summed time of its tests and of its GPU
  tests. Those are reports: nothing fails on how long a check takes, because checks run beside builders. Four
  rules do fail a merge, beside the other counts that only fall. *(owner, 21 Nov 2026: "Yes, plan it")*
  - *The limit on one test.* A test still running at ten seconds is stopped (`GATE-TIME` M13), and the check
    runs it once more, alone, after the others: it fails unless it then passes within ten, and the closing lines
    name it with its time alone. A test that fails any other way is not run again. *(the owner accepted ten seconds; the second run is the
    planner's assumption until the owner answers, because load alone takes a short test past ten)*
  - *The exempt list only shrinks*, from M12. A test is exempt from the limit only by its exact name in
    `.config/nextest.toml`. `GATE-TIME` empties the list, and no landing adds a name or loosens the limit.
  - *A main-tier test is added only by a plan*, from M12. The entries of the main-tier list and the tests marked
    `long:` are counted, and the count only falls. The one exception is a landing that a done-when item asks
    for, [as above](#what-is-proved-and-how-hard): it raises the rule's version, which starts the count again.
    A builder with no such item makes the test fast, or writes the proposal for a planner in its module's log.
  - *No new test binary unless a plan names it*, from M12. A plan names an integration test target when a
    done-when item's check runs it with `--test`, open or Done, or the main-tier list names it. The targets no
    plan names are listed in `.config/unplanned-test-binaries.txt`, which only shrinks, and a target that is
    neither named nor listed fails. So a new test goes in a test file that exists, and a planner who wants a
    new binary names it in an item's check. A planner does not drop the last check that names a binary which
    exists: the next merge in any module would fail on it.

  The docs check also fails when a test marked `long:` sits in a binary the main-tier list does not name.
- **A test runs in a process of its own, beside others**, from `GATE-TIME` M8. It relies on no other test of its binary having run and
  on no static shared with one, so a mutex in a test file does not make tests take turns. A test that needs a
  device to itself joins a group in `.config/nextest.toml`, and one that starts a thread pool is counted there as
  heavy, in the landing that adds it. A failed test is not run again, but for the one second run of a test the
  limit stopped. `./dev check` needs cargo-nextest installed, and says so where it is missing.
- **A change is tested where it is made.** A builder runs the tests of what it touched while it works, and
  `./dev check` once, to merge. A milestone that changes what a long test or a test behind a feature guards
  carries an item whose check is `python3 "$ORCHESTRATE_SCRIPT" main-check long-tests`, so it is not Done until
  those tests pass on main with its commit in.
- **A diagnostic runs in no tier.** A physical or f64 reference is run by hand, when output looks wrong.

## Rules
- **Exclusive resources:** simulator (the one booted iPhone simulator; an `ios-sim` check takes it)
- **Shared resources:** gpu: 2, chrome: 1  *(owner, 15 Nov 2026; a Mac GPU run takes `gpu` and a run that starts Chrome takes `chrome`)*
- **Lock hold limit:** 30 minutes
- **Main-branch check exact-images:** `python3 scripts/exact_baselines.py verify --full`; every 4h; lock gpu, chrome; timeout 45m  *(owner, 15 Nov 2026; every 4h from 21 Nov, owner: main-branch checks held the GPU 12.7 h a day and builders waited 6.9 h for it. Accepted risk: a broken exact image is found up to about 4 h after its merge, and bisected. The lead runs `main-check --due` each hour)*
- **Main-branch check long-tests:** `./dev long-tests`; every 4h; lock gpu; timeout 90m  *(owner, 20 Nov 2026: long tests stay off every merge check; every 4h from 21 Nov, owner, for the same GPU starvation and accepted risk as exact-images)*
- **Main-branch check cross-target:** `bash scripts/check_cross_target.sh`; every 1h; timeout 15m  *(planner, 20 Nov 2026: the Linux and Windows type-check of the tests runs on main, not in a merge's check)*
- **Main-branch check workspace-tests:** `./dev check-all`; every 1h; timeout 60m  *(owner, 20 Nov 2026: a merge runs the tests its change touches, and the full workspace runs on main. Planner, 21 Nov 2026: it takes no GPU slot, like a merge's check, whose load it is once an hour; held for the whole command, a slot would be kept through the build and the CPU tests, and builders would wait for it)*
- **Human decides:** changing purpose or the V1 exit; spending; publishing, licensing or deploying; irreversible
  actions. Gate order within the V1 exit is the agents' call.
- **Licences and paths not to touch:** no GPL or share-alike code or data (restrictive-licence code may inform ideas
  only); never modify Broadsides (ported module by module under [the port decision](docs/design/decisions/SEA-AND-CLOUD-PORT.md));
  preserve the owner's uncommitted files.

## Project guidance
[thrum-engineering](.agents/skills/thrum-engineering/SKILL.md) holds the engine rules for planning, building and
reviewing. The architecture brief is [docs/design/ARCHITECTURE-BRIEF.md](docs/design/ARCHITECTURE-BRIEF.md), the
shared contracts are [docs/design/CONTRACTS.md](docs/design/CONTRACTS.md), and capability status is
[docs/design/maturity.json](docs/design/maturity.json) (implementation, correctness, visuals and device kept apart).
Scratch goes in `tmp/<task>/`, under [the storage rules](docs/ASSET-LIFECYCLE.md).

Some items need a fresh reader, for example CREATOR's M3.1: an agent that has opened only the documents the item
names. A reviewer cannot be that reader, because the review skill and these settings load engineering guidance
before any item is read. The lead commissions the fresh reader as a separate new session, never the builder or the
reviewer, from the brief the module's design names. The builder retains its record as evidence, and the reviewer
judges the record and reruns what it retains.

# SIM-2D M3 review

Reviewed `647b38f2079026e1836e3f515fe80b97c541993d`, with builder work ending at
`e8054007a4b4`. Result: **FAIL**, because the closing module review found one verified
Must Fix outside M3's individual items. All nine individual M3 proofs pass; M3 returns to
`in_progress`.

**Must Fix — `profile-resource-nonfinite-commit` (verified; outside M3's items).**
The module's promise that a candidate storing a non-finite profile number is refused before
commit is not true for game-owned authority resources. DESIGN.md mechanism 1 promises
whole-tick refusal; mechanism 4 explicitly includes game resources in authority state.
The new guide likewise tells authors to encode components and resources and says a
non-finite candidate is refused before commit (`docs/SIM2D-RULES.md:21`).

In `crates/thrum-core/src/sim/runtime.rs:1518`, preparation validates registered component
storage, after validating edits. It never validates candidate resources. Commit then calls
`game.apply` at line 1556. `world_digest` at line 618 encodes supplied resources but does
not refuse their non-finite values. `StateEncode::is_finite` exists, but neither path calls
it for resources. Thus implementing a complete resource encoding, as the guide requires,
does not provide the promised refusal.

I built a public-API probe from this checkout, based on the existing profile-refusal test's
support game. Its candidate holds a valid component plus a resource `Number`; reset stages
the requested resource value and apply copies that candidate value to committed game state.
No engine source was edited. Both `ONE / ZERO` and `ZERO / ZERO` returned `Ok(1)`, advanced
to tick 1, committed a non-finite resource and changed the world digest. A finite control
also committed. See [probe source](M3-review/resource_probe.rs),
[manifest](M3-review/Cargo.toml) and [actual output](M3-review/resource-probe.log).
Reproduce from the repository root:

```sh
CARGO_TARGET_DIR=tmp/sim2d-m3-review-target cargo run --locked --offline --manifest-path docs/evidence/SIM-2D/M3-review/Cargo.toml
```

The expected behavior is an error before any authority state or tick advances for both
invalid values, followed by successful finite input. The probe asserts the currently
observed bad behavior to make reproducing it unambiguous. This is an ordinary profile
operation producing an invalid result, not a hostile encoding or a lint bypass. The
reference crowd's current resource calculations stay finite; I did not observe corrupt
Crowd/Mixed traces, and I did not run this new probe in Chrome or the simulator. The same
missing validation is visible in the shared runtime code, but the execution evidence is
Mac debug only.

Provide a profile resource validation path before commit, with complete refusal/rollback
coverage; do not impose the portable profile on existing non-profile games. A proposed
item to catch this gap is: “A rule staging NaN or infinity in an encoded authority resource
is refused atomically, preserving components, resources, tick and publication; a subsequent
finite tick succeeds.” The current component-only refusal fixture cannot catch this.

**Item judgments**

| Item | Verdict | Judgment of every named proof |
| --- | --- | --- |
| M3.1 | pass | Read all eleven fixture sources against the common passing source and manifest/config construction. Nine guard classes each introduce only their named violation; the two component cases substitute `usize` or `f32` and fail the `encode` lookup. `controls` requires a failing exit and the expected diagnostic, and refuses a failing positive control. The recorded clean-checkout check passes. This proves the stated fixtures, not resource refusal. |
| M3.2 | pass | `crowd-2d` is a registered rule package. `sim2d_profile.check` discovers registry entries, compares configuration bytes with the canonical file, checks dependency kinds and runs Clippy with explicit `CLIPPY_CONF_DIR` and forbidden profile lints. `repo_check.py` also checks configuration/dependencies; `tools/xtask/src/main.rs` invokes both profile commands in `check`. The crowd has no suppressions and its recorded check passes. |
| M3.3 | pass | Read engine profile/query code and crowd/runner callers; a case-insensitive search finds no crowd names in either engine crate. `--package` resolves registered runner, sizes, traces, vectors and witnesses; native build/binary and Wasm module selection use that runner. The documented CLI/Wasm adapter is sufficient for a subsequent package without script edits. The sine control uses the same adapter as its second user, with registered control sizes; its retained real execution diverges at tick 1. Alternate-package unit tests exercise dispatch, although with mocks (suggestion below). |
| M3.4 | pass | The creator guide links `docs/SIM2D-RULES.md`. It lists every operation in mechanism 1, each refused class and reason, random function, query order/domain, rule-set cell size, registration, runner protocol, candidate recording and all three targets. Commands match the CLI and installed loop path. Profile and target commands have passing receipts; the guide's record exercise produced a matching candidate, and code refuses overwrite of registered/existing files. The resource-refusal assertion is covered by the separate module-contract Must Fix above rather than counted twice. |
| M3.5 | pass | The clean-checkout Mac check passes: both traces at 2,000/3,600, 5,000/3,600 and 10,000/36,000 bodies/ticks, smallest wire routes, two debug rows and profile vectors. I independently compared every saved row's fold and checkpoints to the current retained files. Those files have no diff from the M2 pass. |
| M3.6 | pass | The clean-checkout Chrome check passes with eight retained rows and profile vectors, through real Chrome Wasm, not Node simulation. All saved folds/checkpoints match the references; runner source identity agrees with Mac. |
| M3.7 | pass | The clean-checkout iPhone 18 Pro simulator check passes with eight rows and profile vectors through `simctl spawn`. All saved folds/checkpoints match the references and runner source identity agrees. Function only; no simulator speed conclusion. |
| M3.8 | pass | Read both W1/P3 `sim2d` ledger entries. Implementation, correctness, visual, device and performance claims are separated. Both explicitly say physical iPhone, x86, Android and other browsers were not run, and disclaim simulator speed. Historical shared-host measurements are not promoted into new M3 performance claims. |
| M3.9 | pass | `main-branch check exact-images`. The script's receipt reports `COVERED` for `8ec690e7d573` by passing main commit `f560ef86e0ac`. No retained image was moved by the milestone. No duplicate GPU/browser verification was run. |

**Suggestion — exercise a real second rule package (verified test limitation).**
The alternate-package test in `scripts/test_sim2d_conformance.py:15` mocks registration,
Runner, references and comparison. It tests dispatch well but cannot catch build/export
or retained-file integration mistakes for another package. A tiny independent adapter
would improve regression coverage. Code inspection supports M3.3 today, so this is not a
Must Fix or a request to add another production package.

**Scope and evidence**

I formed the item judgments from DONE, design, implementation and checks before reading the
builder's M3 narrative and log. Read M3's implementation changes in `957ac2396` and
`cc01f067e`, the closing evidence changes in `e8054007a4b4`, and the current module's profile,
query, crowd, trace, runner, cost and test code, including component registration,
transaction validation, digest and resource commit interfaces. M3 is the last milestone,
which is why the older resource path was examined. The defect is pre-existing within this
module; it is not attributed to the M3 guard change. No prior failed M3 reviews were supplied.
No glossary file exists in the plans tree.

The script's passing checks on `8ec690e7d573` are accepted as authorized by the task. I read
their saved receipts/output tails and retained compact copies in
[check receipts](M3-review/check-receipts.json). These supersede the builder's older dirty
receipts and pre-merge `NOT_ON_MAIN` result. I also read the script's complete target JSON
records and compared their folds/checkpoints against retained truth without rerunning the
product. [Comparison summary](M3-review/retained-comparison.json) records all three clean
runs, their shared source identity and 10/8/8 row counts. The comparison implementation
checks lengths, folds, checkpoint sequences and nonzero replay/wire witnesses; the runner
actually steps registered runtime/history and replaces superseded timeline entries.

There are no `inspect` or human-feedback items in M3 and no rendered product in SIM-2D.
Accordingly there is no visual inspection list or screenshot evidence. Low sun, storm,
distance, fast camera, full-refresh display, simulator rotation and foreground/background
are not applicable to these headless CPU rule runners. Relevant edges read in the proofs
include every named refusal class, format and wire rounding boundaries, corrected replay,
slot reuse, both trace variants and local/wire routes. My additional executed edges were
non-finite resource values and a finite control on the Mac. I did not rerun the expensive
accepted target checks or make new performance claims. Owned probe build output was
removed; reproduction source, lockfile and compact logs remain in the review folder.

Review closeout: `git diff --check` passed, and `./dev check-docs` passed using a temporary
Git index containing the new review artifacts ([log](M3-review/check-docs.log)). The first
run correctly refused links to untracked artifacts; the temporary index let the checker
validate their post-commit presence without staging or committing this review. Only M3
reviewer-owned DONE fields and the allowed review files changed.

FAIL M3: 0 fail, 0 could_not_judge, 1 outside the items

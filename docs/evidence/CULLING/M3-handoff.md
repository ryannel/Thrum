# CULLING M3 handoff

M3 must establish that culling the moving valley changes no submitted pixel,
select the default from paired rows, retain target motion inspection and close
the brief's G3c clauses. Completion still means all ten builder items pass with
target evidence and the checked merge succeeds, leaving M3 `in_progress`.
This handoff commit does not complete or merge the milestone.

The session resumed clean `2fd5cf063800447260bfe14143922879b5948074`.
The implementation, historical timing rows and inspection crops already exist;
[M3 evidence](M3.md) and the [investigation](M3-investigation.md) record their
scope. None of the M3 items has a builder pass. Historical captures need final
source qualification, and the G3c maturity row still describes M2.

The last production change in this handoff rejects the earlier explicit-UV-
gradient candidate from `52450643`: `crates/thrum-render/src/synthetic.wgsl`
now exactly matches blob `a8d8926fc15c2dfcaaab6d1f8b1779ae7609d2d3`
from `64c81cd2`. It retains the original implicit material texture sampling.
The candidate's extra gradient state is gone. This is a restoration, not a fix
for the original instability.

On clean `2fd5cf06`, the literal item checker passed M3.1, M3.2, M3.5 and M3.6.
M3.4 failed at tick 255 on Portable: six SDR pixels differ between Direct and
Frustum. A separate run with `MTL_DEBUG_LAYER=1 MTL_SHADER_VALIDATION=1`
failed at tick 251 with 18 differing scene-linear pixels. Metal reported both
validation modes enabled but no validation error before the test panic. The
prior session's repeated scene-linear test also failed between two Direct
hosts, so culling is not required for an observed mismatch. These results do
not identify the responsible stage.

M3.10 rejected APPLE-TILE's `portable-sdr-approach` with 4,859 changed pixels;
`./dev check` then refused the missing baseline proof. The
[qualification folder](M3/qualification-2fd5cf06/items.txt) retains checker
output, the exact comparison and verifier identities, the six-pixel failure's
compressed original bytes and receipts, and the Metal validation log. Those
results qualify only the rejected candidate. No target proof was rerun after
restoring the shader.

The boundary requiring this stop is in
[CULLING's plan](../../plans/CULLING/PLAN.md): “If your change fails an exact
test or baseline another module owns, do not edit it. Write the proposal in the
log and end `BLOCKED`.” No baseline or other module's criteria were edited.
The proposed continuation is to diagnose the original intermittent shading
difference while preserving the existing sampling and exact references. The
explicit-gradient candidate must not be accepted from occasional passing
routes: it both reproduces the failure and changes retained output.

One possible next experiment, not yet performed, is simultaneous original
colour and shadow/air-map readback from the same failing submission. Earlier
single-stage diagnostic runs changed either their readback or shader and
passed individually; they cannot explain a failure on a different submission.
The original first comparison must remain exact and must not be replaced by
the converging same-time recaptures. The shader's responsible fault remains
unknown; derivative placement, texture filtering and shadow evaluation are
hypotheses, not findings.

Both background jobs from this session have ended:
`20261113-212027-47794` is the failed item checker, and
`20261113-212216-57348` is the validation runner (its retained inner run exits
101; the wrapper itself exits zero). No active GPU job needs to be stopped.
Owned scratch and failed captures remain under `tmp/culling-m3/` and
`tmp/baseline-guard/verify-20261113T202526.707700Z/` for diagnosis.

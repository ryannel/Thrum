# Reconcile implementation with the selected target

Use the saved independent design as the target. Inspect relevant code, architecture rules, running behaviour and
tests, including neighbouring owners and consumers. Verify revision and distinguish working behaviour from docs,
stubs or acceptance claims. Do not reopen research merely to make the old implementation look preferable.

Map current responsibilities and contracts onto the target. Recommend retention, replacement, restructuring or
removal for consequential differences. Code earns retention by satisfying the selected model and interfaces;
volume, sunk effort and tests of old assumptions are not evidence of fit. Useful assets, authoring inputs,
observations and regression cases can survive replacement. An earlier prototype that looks better (another
language, an older engine) sets a look bar and supplies references; its technique still has to earn its place against
the target, since it may be too heavy or physically wrong.

When porting a module into this engine, map it onto the engine's existing owners: the one sky and atmosphere, the
frame contract, the capability registry, cooked products. Don't bring a parallel stack along with it.

Prefer clean boundaries over adapters or duplicate producers that preserve an inferior design. Include neighbouring
changes needed for cohesion; a compatibility layer needs an actual consumer and retirement condition. Identify
architecture/policy changes explicitly rather than weakening checks. A rewrite preference grants no permission to
delete unrelated work, change protected policy or discard user data.

Keep transition management separate from the ideal target. Plan complete dependency-ordered slices, state/content
conversion, consumer contracts, regression coverage and rollback where warranted. A temporary comparison harness
may retain both versions while each run has one authority for each contribution. Share per-frame or per-world
services at their real reuse scope; do not copy a prototype's resources into every feature instance.

Change the target when requirements or evidence overturn it, recording why. Migration convenience is not a technical
requirement. If platform/access constraints prevent the target, distinguish the constrained proposal and unmet needs.
Deliver the fit assessment and restructure/replacement plan with deletions, risks and validation. Implement the
commissioned scope through `/build-milestone`, including [performance](performance.md) when relevant.

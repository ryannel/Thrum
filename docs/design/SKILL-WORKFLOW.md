# Engine skill rationale and forward tests

> **Superseded for process (8 November 2026).** How work runs is now in
> [AI development: how work runs](AI-DEVELOPMENT.md#how-work-runs) and the level-named skills. This document is kept as
> the record of the 1–2 November skill rationale and forward tests; its process instructions (model and effort, delegation, coordination) no longer apply.

The [design programme](DESIGN-PROGRAMME.md) owns this commission's orchestration, evidence standard and completion
gate. The architecture skill (now [plan-engine](../../.agents/skills/plan-engine/SKILL.md)) governs whole-engine work; the
module skill (now [plan-module](../../.agents/skills/plan-module/SKILL.md)) governs a bounded module. This note records why
their workflow boundary was adjusted and how to test it independently.

## Source rationale

OpenAI's [Astra skill guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)
recommends short, discriminating descriptions, progressive disclosure and room for judgment instead of elaborate
recipes. It also calls for explicit completion and exploration limits so a capable agent continues through the
commissioned work without stopping after a first pass. The official [Build skills guide](https://developers.openai.com/plugins/build/skills)
emphasizes one recognizable goal per skill, clear input/output and decision boundaries, routed supporting material
and representative behavioral tests. These sources support keeping the reusable skills compact while placing this
project's parallel-wave coordination and maturity criteria in the programme note. They do not prescribe this engine's
module map or prove any technical choice.

The architecture skill now commissions modules with relevant contracts, budgets and research depth. The module skill
returns cross-boundary changes to the architecture owner and reports evidence maturity. Both route to the canonical
programme for this project. The repo-local skills are authoritative here; a duplicate user-level
`~/.codex/skills/engine-module-design` may be selected in other contexts and should be reconciled separately.

## Independent forward tests

The [AI development design](AI-DEVELOPMENT.md) applies the same Astra guidance to future engine-contributor and
game-SDK workflows. On 1 November 2026 the existing engine skill entrypoints and module implementation/review routes
were reread against the official article and current skill-creator guidance. Their focused descriptions, conditional
references, local-fix exception and commissioned completion boundaries remain useful. Structural validation passed
for both skill files; this is not a new behavioral evaluation.

| Astra principle | Concrete application / remaining limitation |
| --- | --- |
| Precise discovery | Keep whole-engine and substantial-module triggers distinct; proposed routine contribution/game/diagnosis routes are narrowly scoped |
| Progressive detail | Load the relevant module reference or SDK task recipe, not every workflow; avoid copying checker rules or whole API manuals into the skill |
| Capable-agent judgment | State intent, real invariants and outcome criteria; do not prescribe every implementation step or a fixed reviewer/test quota |
| Scope and persistence | Finish the commissioned design or implementation stage; no artificial approval after each local check, no authority expansion from skill invocation |
| Lean default context | Future scoped instructions replace blanket prerequisite reading for unrelated engine/SDK tasks; current Broadsides instructions remain active |
| Behavioral evidence | Future skill trials inspect artifacts, real API use, integration and recovery; frontmatter validity alone does not establish value |

Proposed SDK skills will be written against implemented, version-matched commands. No placeholder skill or fictional
tool workflow was installed in this pass. The AI design's larger rule/tool tables are architectural reference material,
not proposed mandatory text for every skill invocation.

A fresh Sol High [instruction audit](../evidence/handoffs/2026-11-01-engine-ai-workflows/ASTRA-ALIGNMENT.md) recommended
no changes to the two skill entrypoints. Its one refinement was applied: the seed/pointer creation example is explicitly
illustrative, so it cannot become a compulsory recipe for headless or other input styles. This is a document review,
not a test of Astra's behavior.

Run these later with a fresh agent and only the raw brief and relevant skill. Do not supply an expected answer or
earlier evaluator conclusions. Judge the actual artifact and decision trail, not copied wording.

1. “Use the engine-architecture skill to plan module commissions for a 3D engine supporting an isometric town game,
   a third-person exploration game and a browser demo. The host is custom; identify shared contracts, budgets and
   the first risky design questions. Stop at research/design.”
2. “Use the engine-module-design skill to design a character animation module for desktop and mobile profiles. It
   must coordinate with physics, gameplay state, camera and content import. Identify a consequential boundary change
   if one appears, and report what evidence would be needed before implementation.”
3. “Use the engine-module-design skill to assess reusing an open-source engine's asset-loading subsystem in a custom-host engine.
   Compare source-level reuse, reimplementation of the idea and an alternate library; state provenance, license
   obligations and the evidence needed before declaring the choice integrated.”

The first prompt checks whole-engine ownership and bounded commissions; the second checks module integration and
design maturity; the third checks selective reuse without accidentally adopting another engine as the host.

## Lifecycle skill refinement — 1 November 2026

The [lifecycle walkthroughs](LIFECYCLE-WALKTHROUGHS.md) reuse `repo-structure` for placement/delivery/retention and the
module-design performance reference for campaign handoff. Two conditional references were refined; no skill per
lifecycle stage, new broad description or fictional cleanup command was added. Current factory records remain
canonical. The official Astra guidance above was reopened for this pass; precise discovery, progressive detail and
proportional completion boundaries support this choice. An independent Sol High paper trial tests decisions from
raw operator cases; it is neither an Astra execution test nor proof of runtime tooling safety.

## C4 reconciliation check — 1 November 2026

Reopened the official [Astra skill guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)
while reconciling the complete conversation. The existing architecture/module entrypoints still distinguish their
scopes, route detail conditionally, require shared-owner reconciliation and define completion without a build-by-default
rule. No new giant skill or fictional SDK commands are needed. The C4 protocols live in their owner specifications;
future contribution/game skills should expose the small file/admission/run workflow and call implemented checks.
Behavioral trials, deterministic convention enforcement and actual game/asset/campaign handoffs remain required when
those tools exist. This was a source/instruction review, not an Astra behavioral test or implementation of the tooling.

## Water and Clouds skill transfer — 2 November 2026

The available Water and Clouds Codex chat turns showed a recurring gap between researched components and a
working module: attractive cloud frames without convincing growth and breakup, and water diagnostics that passed
while wakes or impact were not visible. Their later developer walkthroughs exposed missing source laws and timing;
the content reviews exposed an incomplete path from editable inputs to the runtime consumer. These findings led to
focused additions in the engine-module-design research, review and implementation references.

Thrum's existing performance guidance remains authoritative: choose representations with an analytical cost account,
including repeated cheap work, data movement and creator workload. The new creator journey should follow C04/C05's
provider and F4 product authorities, C10's authored operations, and the current `docs/BUILD-PLAN.md`. Describe
not-yet-implemented tools as proposals; `./dev help` is the source for real commands. A documentation walkthrough
does not advance the maturity ledger or qualify a rendered scene.

The entrypoints remain focused and route detail to references, following
[OpenAI's Astra skill guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)
and [prompting guidance](https://developers.openai.com/api/docs/guides/latest-model). No fixed reviewer count or
universal offline pipeline was added.

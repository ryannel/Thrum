# Blue-sky research and design

## Connect the commission to the engine

Start with the engine brief's required experiences, module responsibility, external consumers and budget. Resolve
missing context only as far as it affects this module; use [engine architecture](../../engine-architecture/SKILL.md)
for whole-engine questions. A proposed boundary change belongs in both the engine brief and the module design.

Use one canonical design as the current recommendation. Research notes and experiments support it. Track requirement
coverage, decision status, implementation and proof separately; a researched or implemented component is not an
accepted system. Scope changes update this design and its queue, not only the latest experiment's README.

When the engine serves multiple games, cover contrasting supported profiles and their important combinations.
Specify required/optional dependencies, substitutes and disabled-resource behaviour, including consumers that need
an effect without its visible source. Prove these boundaries alongside the most demanding enabled configuration.
Keep research challengers distinct from the reliable default until their complete interaction and costs qualify.

## Define the experience independently

Describe observable success, consequential interactions, conditions and transitions. Separate requested requirements,
preferences, assumptions and stretch goals. Save the independent target before inspecting existing module code or old
technical plans. If already exposed, disclose that limitation and use a separate requirements brief and external
evidence rather than claiming a blind study. Read repository/product instructions relevant to the decision.

Itemise the module's capability inventory from the brief and representative use cases before selecting techniques.
List observable outcomes rather than subsystem names. Include required presentation, dynamics, queries, interactions,
authoring/control and lifecycle/operating conditions where relevant. Use these as discovery prompts, not a universal
feature checklist. Distinguish explicit requirements, necessary implications, assumptions, optional extensions and
exclusions; inferred scenarios must not silently become product requirements.
Split entries when their outcomes have different requirement bases instead of labelling a mixed bundle as required.

Discover dependencies through inputs → evolving state → representations → consumers and feedback. Give capabilities
stable references and a compact coverage account: requirement basis and observable success, selected mechanism/owner,
important dependencies or coupled consumers, and planned proof plus actual status. A table is useful; use the smallest
format that keeps omissions visible. Keep undecided mechanisms explicit during discovery, then close required design
gaps before substantial construction. Research support, implementation and demonstrated acceptance are separate states.
Scale detail to the commissioned stage: a preliminary inventory may leave mechanisms provisional and research pending;
that does not satisfy the later build-ready design criterion. Do not turn capability discovery into a compulsory
literature survey or implementation exercise.

Use an independent subagent, when available and authorized, to derive missing capabilities from the original brief
and use cases, rather than only checking the author's inventory; follow [review](review.md). Reconcile its discoveries
into the inventory and design.
Maintain coverage when requirements, assumptions or fidelity change. Revisit sparse paths independently of where the
bibliography is largest. Similar appearance need not imply shared dynamics or ownership.

Translate broad words such as realistic, coherent or evolving into indispensable observable cues. If the user relaxes
physical fidelity, revise methods and validation burden while preserving their remaining experience requirements.
An honestly labelled approximation can still fail the commissioned result.

## Research the decisions that matter

Research is followed by analytical synthesis, not an open-ended search through prototypes. Derive the state,
operators, invariants, approximation regimes and consequential error/cost behaviour of the recommended design.
Use published results and domain mathematics to resolve representation choices as far as they allow. Identify
which uncertainty genuinely requires measurement, what the design predicts, and how a contrary observation would
change it. Do not defer missing producers or essential coupling under a promise to experiment later.

Use current primary papers, supplements, author implementations, developer-authored technical articles and white
papers, substantive conference material and official engine documentation/source. Research competing whole-system
approaches as well as component algorithms. Search recent successors and credible competitors, then trace useful lineage. Reuse retained
substantive research; update decisions where recency or uncovered responsibilities matter. Novelty alone is not a
selection criterion. Stop expanding a covered topic when more sources would not change a decision or resolve a
consequential uncertainty.

Inspect conference recordings, slides or detailed transcripts substantively. Cite useful timestamps, slides/pages or
passages and their design consequences. State what was actually inspected; slides are not a watched recording. An
inaccessible title or abstract does not supply an algorithm. Try legitimate author/institutional copies for important
access gaps, then record their effect on the decision.

Retain a compact evidence account for consequential choices:

- Choice, affected experience, credible alternatives and selection reason.
- Primary links, useful locators, version/date and material actually inspected.
- Assumptions, applicability, cost conditions, evidence maturity and adaptation.
- Remaining uncertainty, what would change the decision and a discriminating experiment where needed.

A broad engine document or governing law does not establish every algorithm under its heading. Distinguish disclosed
methods, mathematical deductions, authored choices and unrun qualification. Published demonstrations, shipped practice
and proposed combinations support different claims. Depth follows consequence and uncertainty, not equal paper counts.

Investigate major engines as systems where relevant: state/resource owners, preparation, streaming, scheduling,
consumers and invalidation. Compare their content, camera, world and hardware constraints with the brief. Published
limitations and disabled capabilities are design evidence. A demonstrated result may depend on prepared content,
artist intervention or an undisclosed subsystem; identify that dependency rather than assigning the whole result to
one component. Rendering moving detail, for example, does not establish structural evolution.

## Design how the whole system works in practice

Choose governing relationships, drivers, state, representations, ownership, update order and public contracts, including
prepared content and authoring. Prefer demonstrated production combinations where they fit; justify consequential
departures. Individually credible techniques do not establish compatibility.

Save this design before substantial implementation. Cover all required responsibilities together, including nonvisual
queries, interactions and feedback, rather than completing the most visible component and leaving its neighbours as
future integration. Bound the design to the commissioned experience; external systems need explicit contracts, not
unsolicited implementation. A subsystem inventory, bibliography or boxes-and-arrows diagram alone is insufficient.

Complete the causal chain with domain knowledge where publications leave gaps. For each defining behaviour, connect
its driver, state change, mathematical operator or authoring operation, representation and observed response. Derive
consequential couplings and time/length scales. Identify published, inferred and authored links, with a discriminating
proof for uncertain ones. Equations need sufficient inputs, boundary conditions and closure assumptions to explain
the response; naming the governing law is insufficient. Do not claim this synthesis reveals private engine internals.

Assign authoritative state, derived detail, prepared content and reference comparators explicit roles. Name actual
producers behind downstream inputs; base state does not automatically supply every required quantity. Select each
consumer's fidelity, update rate and CPU/GPU placement. Shared causality does not require a universal solver or make
every rendered detail authoritative.

For consequential connections, specify the actual input/output quantities or queries, their producing/consuming owners,
and the mathematical relationship or operation that joins them. Describe how consumers refer to the same event,
location and time, including any inverse lookup or reconstruction. When representations differ, state which
differences are acceptable and how consistency is maintained. A surface can look correct while contact queries or
interaction-generated disturbances describe a different surface.

Check handoffs for definitions, units, signs, coordinate frames, normalization, support, timestamps and sampling.
Preserve necessary identity and local relationships as well as totals. Assign contributions exactly once across
ordinary, bypass and fallback paths. Define cache validity and failure behaviour. Use [system diagnosis](system-diagnosis.md)
for consequential reconstruction or integration risks, and [performance](performance.md) for practical cost controls
and ranked reductions. Unknown hardware can defer numeric certification, but not a viable reduction strategy.

Work through representative operating cases in the saved design: initial state, a normal step/frame, a coupled
interaction and a consequential transition or failure. Trace inputs, queries, state updates, feedback, derived data
and observed outputs in execution order. Resolve circular dependencies, timestep differences, CPU/GPU transfers,
latency and invalidation where they affect behaviour. Estimate combined work/residency with the important consumers
active together; individually affordable components can exceed the assembled budget.

The design is detailed enough when another implementer can follow these cases and explain what each consumer reads,
what changes next and why the results agree, without choosing a missing core algorithm or coupling contract. Resolve
structural choices upfront; tuning values and bounded feasibility uncertainties may remain explicit. Use small probes
to settle such uncertainties, then update the design. Do not require perfect predictions or exhaustive pseudocode,
but do not defer an essential mechanism under a capability name or a promise to integrate later.

Check the design against every required capability and consequential connection, including reviewer discoveries.
An impressive worked case must not conceal an unowned requirement elsewhere. Deferrals remain explicit and do not
silently reduce the commissioned deliverable; whole-system acceptance requires its required capabilities to pass.

Keep the target distinct from the first proof and constrained profiles. Coding effort or migration convenience alone
is not a reason to weaken it. Retained research must have an explicit adopted contract and present role; earlier
qualification does not automatically transfer to a new assembly.

## Prove what could invalidate the design

Put the least-established defining behaviour on the critical path, ahead of broad infrastructure or costly content
production. Choose a small complete experience with consequential consumers, representative inputs, ordinary
presentation, intended distances/speeds and explicit rejection criteria. When content is uncertain, inspect a prepared
sample before a larger bake; when coupling is uncertain, include that coupling in the proof.

Derive the proof from the worked interactions. Reduce world size, content count or regime breadth while preserving
the required connections. Isolated component tests support this proof; they cannot replace observing the components
responding to the same interaction together. Keep any unexercised required connection explicitly unqualified.

Name a plausible wrong substitute that could pass the proposed checks while missing the goal, and the observation
that rejects it. A still cannot qualify motion; global conservation does not establish local behaviour. For evolving
structure, distinguish transport, deformation, creation/destruction and connectivity changes where relevant. An age
label, global scale or fade is insufficient evidence of those changes. Normal speed establishes pacing; accelerated
speed can reveal repetition, discontinuities and absent development. Neither alone qualifies the other.

Use governing identities, independent formulations, measured references and perceptual comparisons where they
discriminate plausible errors. Two evaluators sharing an omission cannot bound it. Separate missing-model error from
sampling/reconstruction error: refinement cannot supply an absent mechanism. Bound expensive qualification to the
uncertainty it can resolve, and label proposed thresholds as proposals.

Save the recommendation, evidence account and dependency-ordered proof/build plan before [review](review.md). Keep
current-code inventory and migration advice separate. Design is ready when consequential paths have justified
mechanisms or genuine explicit limits, and remaining qualification has candidates and rejection rules. Continue
through implementation when commissioned; research alone does not authorize building the proposal.

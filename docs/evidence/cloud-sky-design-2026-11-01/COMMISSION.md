# Complete-cloud commission

User objective received1 November2026. This retained copy governs the full goal; it supersedes earlier narrow optional-family/weather labels.

Use the updated repo-local module-design skill, Thrum’s plans, our cloud research and the lessons from the previous cloud and water systems. Design the complete system before extending implementation. Retain the current renderer, research and reference tools wherever they fit.

1. Establish the complete capability inventory and design

Itemise the required cloud families, weather conditions, behaviours, authoring controls and interactions with neighbouring systems. Start with:
- Fair-weather cumulus.
- Towering cumulus and cumulonimbus thunderheads, including vertical development and anvil formation.
- Broken low cloud and stratocumulus.
- Layered stratus and overcast.
- High wispy cirrus.
Reconcile that selection with Thrum’s actual requirements and the research. Distinguish required capabilities from optional extensions.

Explain how the system produces recognizable shapes, variation, formation, growth, mature behaviour and breakup for each family. Define how wind, altitude, coverage and weather progression control them, and how those causes remain consistent across visible clouds, shadows and reflections.

Design isolated formations and full-sky composition together, including resource sharing, distant representation, transitions, authoring and integration contracts. Get independent subagent review to find missing capabilities and challenge the complete operating model before building.

2. Prove each cloud family in isolation

Build repeatable inspection scenarios for each required family. Give each family explicit visual and behavioural acceptance criteria.

Prove:
- Recognizable structure, scale and altitude.
- Meaningful variation between formations.
- Appropriate formation, growth, mature behaviour and breakup.
- Coherent motion with persistent structure.
- Convincing lighting under the required Sun and sky conditions.
- Consistent receiver shadows and reflected appearance.
- Correct behaviour from the relevant viewing distances and angles.
Inspect actual native images and sampled motion at normal and accelerated speeds. Use the research and references to judge appearance and behaviour. Each family must demonstrate its defining characteristics; an attractive still alone does not close its proof.

Correct failed isolated proofs before incorporating those families into the full sky. Preserve verified parts when revising a failing representation.

3. Prove combinations and changing weather

Once the isolated families pass, demonstrate multiple formations and overlapping layers working together.

Prove shared wind and lighting, believable differences between layers, consistent shadows and reflections, and weather changes that preserve the identities and continuity of existing formations.

Demonstrate clear-to-scattered, increasing cloud cover, overcast development and a developing or approaching thunderstorm. Establish concrete mechanisms for those transitions and check them at normal and accelerated speeds.

Provide the integration seams needed for precipitation, lightning and other weather consumers. Their separate implementations should follow the existing Thrum scope.

4. Prove a complete sky

Assemble representative full-sky scenarios covering fair weather, broken cloud, overcast and storm conditions.

Judge believable coverage, distribution, depth, scale variation, layering, distant clouds and horizon treatment. Inspect wide views and closer views. Check for obvious repetition, synchronized lifecycles, appearance changes with camera movement, disappearing consumers and inconsistencies between the sky, shadows and reflections.

Produce repeatable weather sequences showing how the complete sky changes over time.

5. Measure throughout and assess the complete workload

Track actual work, memory and scaling as each family and interaction is added. Use those measurements to catch unsuitable architectural choices early.

Prioritize proving the complete required behaviour. Assess final optimization against representative full-sky workloads, preserving the visual and behavioural witnesses.

Compare results with Thrum’s proposed budgets, including the cloud allocation and complete-frame limits. 

Budget failures remain explicit open gates. A successful isolated cloud does not establish full-sky viability.

Delivery and working discipline

Continue in the cloud worktree and preserve unrelated foundation work. Keep Thrum’s construction dependencies and shared contracts authoritative.

Commit and push at logical, verified checkpoints: reviewed design, accepted isolated families, combined-weather proof and full-sky proof.

Finish with:
- The revised canonical system design and capability inventory.
- A clear pass/fail/unproven assessment for every required capability.
- Repeatable isolated-family and full-sky scenarios.
- Actual images and normal/accelerated motion evidence.
- Independent review findings and their dispositions.
- Performance and memory results, remaining risks and the next concrete work.
Leave the best verified proof available and paused. If a capability cannot be completed, retain the working results and explain the specific failed assumption, supporting evidence and proposed correction. Claim completion only for what has actually been demonstrated.
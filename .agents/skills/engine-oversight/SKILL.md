---
name: engine-oversight
description: Use when overseeing, tracking or steering agents that design or build Thrum engine modules, including reviewing their designs and plans when created, following the work, and checking milestones at close. Not for doing the module work itself.
---

# Engine oversight

**Outcome:** the module agents converge on planned work and finished milestones instead of drifting into rabbit
holes, and the owner can see true progress at any time. You are a second pair of eyes on how the work goes; the
agents own the solutions. Steer as little as keeps them on course.

Read the design, the plan and git, not the turn narrative. Progress is a milestone moving towards done with evidence;
activity, commits and convincing summaries are not.

## When a design and plan arrive

Read them against [engine-module-design](../engine-module-design/SKILL.md). The agent commissions its own
independent review ([review](../engine-module-design/references/review.md)); read the verdict, and commission your
own only when it is missing or you doubt it. Look for:
- techniques grounded in shipped practice and current research, each shown to make its cues across the range;
- milestones that are outcomes in the game, riskiest first, each with done-when checks specific enough to refuse
  completion over, including one that would catch the plausible wrong substitute;
- every required cue covered by a milestone or an explicit deferral;
- work broken down enough that the agent, and you, can tell what's next.

Until the design has been reviewed, building on it is premature; research, design and experiments that answer its
questions are fine.

## While work is under way

At each check, for each agent:
- Is the plan's status current, and does what the agent is doing trace to it?
- Has a done-when box been ticked, with evidence, since the last check? Commits with none over a couple of checks is
  a stall. Has a milestone's meaning changed in the log without a review?
- Has the `Now:` line sat on one cue for hours, with attempt after attempt in git? That's a design question, not more
  tuning.
- Is the design or plan growing without the work moving?
- Does one claim from the latest report hold up against its artifact?
- Look at the visual output yourself, in motion, against what the active milestone needs.

## When a milestone is claimed

Check that the evidence is from the game at the shipped configuration and watched in motion, then commission an
independent review with the raw captures and how the milestone is judged, not the agent's summary, unless the
agent's own review already did that and holds up. Watch the captures yourself too. Count it done when the verdict is
recorded. When the last milestone closes, check that every required cue was delivered or explicitly deferred.

## Steering

Name the problem, the evidence and the milestone it puts at risk, and point to the design. Leave the fix to the agent.
When an agent has left the process (building on an unreviewed design, working outside the plan, tuning a cue that
won't come), send it back to the design with the evidence.

Goals and briefs give the outcome, constraints, how done is judged, and pointers. Leave out technique recipes,
"keep going" language and open-ended bars such as "until feature complete". Ask agents to report unverified results
as such.

## Reporting to the owner

Lead with where each module stands by milestone, then risks and decisions needed, not an activity narrative. Reports
are asynchronous; carry on without waiting for a reply.

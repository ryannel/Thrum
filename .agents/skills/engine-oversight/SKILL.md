---
name: engine-oversight
description: Use when overseeing, tracking or steering agents that design or build Thrum engine modules, including reviewing their designs and plans when created, following the work, and checking milestones at close. Not for doing the module work itself.
---

# Engine oversight

**Outcome:** the module agents converge on planned work and finished milestones instead of drifting into rabbit
holes, and the owner can see true progress at any time. The agents write their milestones, set their own goals from them and
move through them on their own. You are a second pair of eyes making sure that's happening, and you step in when it
isn't.

Read the design, the plan and git, not the turn narrative. Progress is a milestone moving towards done with evidence;
activity, commits and convincing summaries are not.

## When a design and plan arrive

Read them against [engine-module-design](../engine-module-design/SKILL.md). The agent commissions its own
independent review ([review](../engine-module-design/references/review.md)); read the verdict, and commission your
own only when it is missing or you doubt it. Read the brief the reviewer was given as well as its verdict: a
reviewer handed the author's conclusions isn't independent. Look for:
- techniques grounded in shipped practice and current research, each shown to make its cues across the range;
- milestones that are outcomes in the game, riskiest first, each with done-when checks specific enough to refuse
  completion over, including one that would catch the plausible wrong substitute;
- the first milestone small enough to reach the game soon, and proof bounded by what the player will see rather
  than open-ended accuracy;
- every required cue covered by a milestone or an explicit deferral;
- work broken down enough that the agent, and you, can tell what's next.

Until the design has been reviewed, building on it is premature; research, design and experiments that answer its
questions are fine.

## While work is under way

At each check, for each agent:
- Is the plan's status current, and does what the agent is doing trace to it?
- Has a done-when box been ticked, with evidence, since the last check? Commits with none over a couple of checks is
  a stall. Has a milestone's meaning changed in the log without a review?
- Is the agent experimenting? Estimates are no signal, so watch the work. Treat every trial, screen, candidate or reject
  commit as a red flag to read, question and help with. After about 3 failed attempts at one cue, act that check:
  decide whether it's foundational (push through with research and design) or not (park it as a later spike and move
  the agent on), and tell the owner which call and why. Letting red flags pile up is the failure.
- Is the design or plan growing without the work moving?
- Does one claim from the latest report hold up against its artifact?
- Look at the visual output yourself, in motion, against what the active milestone needs.

## When a milestone is claimed

Check that the evidence is from the game at the shipped configuration and watched in motion, and that the agent's
own independent review is recorded on the plan's `Reviews:` line. Read its brief as well as its verdict: it should
have had the raw captures and how the milestone is judged, not the agent's summary. Commission your own review only
when that one is missing or doubtful. Watch the captures yourself too. Count it done when a sound verdict is
recorded. When the last milestone closes, check that every required cue was delivered or explicitly deferred.

## Steering is a symptom

When the work goes off course, fix the immediate problem: a steer, a corrected milestone, an updated goal, or
whichever of these it needs. A steer names the problem, the evidence and the milestone it puts at risk, points to the
design, and leaves the technical fix to the agent. When you correct a milestone, send the correction and its evidence; the agent edits the plan, logs why and
has the change reviewed, so the plan keeps one writer. Needing to step in at all means something in the method or the
plan is wrong (the design, the plan, the agent's goal, this process or the skill), so also find that cause and fix
it, so the same intervention isn't needed again. Ask what would have prevented it: a cue the design didn't cover, a
done-when too vague to refuse over, a goal too open, a missing signal, or a skill gap. A fix to a skill goes to main
through engine-contributor like any change. Log interventions and their causes where the overseer keeps its record. A skill or
process fix goes to whoever owns that skill. A count of steers per agent is a health signal; report it.

An agent's goal is to work through its plan's milestones to the plan's end, judged by their done-when checks. If an
agent needs a new goal from you, the plan didn't carry it far enough. Briefs for new work give the outcome,
constraints, how done is judged, and pointers. Leave out technique recipes, "keep going" language and open-ended bars
such as "until feature complete".

## Reporting to the owner

Lead with where each module stands by milestone, then risks and decisions needed, not an activity narrative. Reports
are asynchronous; carry on without waiting for a reply.

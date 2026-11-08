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

Read them against [engine-module-design](../engine-module-design/SKILL.md) and the design-review questions in
[review](../engine-module-design/references/review.md#the-two-reviews-that-matter-most). The agent commissions its
own independent review; read the verdict and the brief the reviewer was given (a reviewer handed the author's
conclusions isn't independent), and commission your own only when it is missing or you doubt it. Check also that
every required cue has a milestone or an explicit deferral, and that each visual milestone names an owner-approved
reference or has one queued for the owner's window.

Until the design has been reviewed, building on it is premature; research, design and experiments that answer its
questions are fine.

## While work is under way

At each check, for each agent:
- Is the plan's status current, and does what the agent is doing trace to it?
- Has a done-when box been ticked, with evidence, since the last check? Commits or work items with none over a
  couple of checks is a stall: ask for a split or re-scope, not a reworded steer. Has a milestone's meaning changed
  in the log without a review?
- Is the agent experimenting? Estimates are no signal, so watch the work. Treat every trial, screen, candidate or reject
  commit as a red flag to read, question and help with. After about three failed attempts at one cue, settle the
  agent's park, re-scope or foundational call that check. Confirm foundational only when the cue could change the
  module's design, and tell the owner which call and why. Letting red flags pile up is the failure.
- Is the design or plan growing without the work moving?
- Does one claim from the latest report hold up against its artifact?
- Look at the visual output yourself, in motion, against what the active milestone needs.

## When a milestone is claimed

Check that the evidence is from the game at the shipped configuration, side by side with the approved reference and
watched in motion, and that the agent's
own independent review is recorded on the plan's `Reviews:` line. Read its brief as well as its verdict: it should
have had the raw captures and how the milestone is judged, not the agent's summary. Commission your own review only
when that one is missing or doubtful. Watch the captures yourself too. Count it done when a sound verdict is
recorded. When the last milestone closes, check that every required cue was delivered or explicitly deferred.

## Steering is a symptom

When the work goes off course, fix the immediate problem. Steers that only reword a problem rarely end a loop;
changes of structure do: park, re-scope, split, a research order, or a reference. A steer names the problem, the
evidence and the milestone it puts at risk, and leaves the technique to the agent. Don't set numeric gates or
targets, and check that any screen or quiet-machine slot you grant can finish unattended. When you correct a
milestone, send the correction and its evidence; the agent edits the plan, logs why and has the change reviewed, so
the plan keeps one writer.

Then find what let it happen, and fix it in the plan, milestone or goal first. Change a skill only when the same
failure recurs across agents and no existing line covers it. If a line exists and was skipped, another line won't
help: find why it was skipped (a conflicting goal, a long file, a loophole) and remove or fold rather than add. Skill
edits go to main through engine-contributor. Log interventions, their causes and the steer count per agent where the
overseer keeps its record, and report the count.

An agent's goal is to work through its plan's milestones to the plan's end, judged by their done-when checks. If an
agent needs a new goal from you, the plan didn't carry it far enough. Briefs for new work give the outcome,
constraints, how done is judged, and pointers. Leave out technique recipes, "keep going" language and open-ended bars
such as "until feature complete".

## Reporting to the owner

Lead with where each module stands by milestone, then risks and decisions needed, not an activity narrative. Batch
what needs the owner (references to approve, foundational calls, direction) into one review window a day. Reports
are asynchronous; carry on without waiting for a reply.

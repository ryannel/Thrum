# The plan

The plan is how the design becomes work, and how anyone can see where that work is. It sits next to the design (in Thrum,
`docs/design/modules/<MODULE>-PLAN.md`; elsewhere, beside the module's design) as `<DESIGN>-PLAN.md`, is reviewed with the design, and is committed with the work it describes. Shape it however serves
the module; what matters is below.

- **Milestones are outcomes in the game.** Each is one small, complete experience you can watch, such as one boat's
  wake reading correctly from drift to planing at the chase camera, not a component such as "the solver". The
  riskiest behaviour comes first, at shipped quality. Every milestone is proven in the game
  ([implementation](implementation.md#make-the-first-visual-checkpoint-real)), so keep each small enough that this
  stays cheap; a milestone that needs more than about three done-when items is usually two milestones.
- **A visual milestone names its reference.** It opens from a reference the owner has approved (a capture, photo or
  earlier build, with its camera, time and settings). Propose references ahead of need, with your recommendation, so
  they reach the owner's daily window; until one is approved, work on what it doesn't gate.
- **Each milestone says how you'll know it's done.** List the checks, views and speeds, as boxes you tick when the
  evidence exists, with a link to it. Keep to what you would genuinely refuse to call it done over, including a check
  that would catch the plausible wrong substitute. A visual box is a side-by-side with the reference watched in
  motion; figures that explain a mismatch are diagnostics, not boxes. Judge only what your module's mechanisms
  produce: a cue that hinges on another owner's choices uses a fixed reference setup or moves to the milestone that
  integrates them. Cost appears only as recorded measurements and [tripwires](performance.md), never as a target.
- **Change a milestone in the open.** Splitting, re-scoping or parking is legitimate: edit it in the plan, say why in
  the log, update the design if the change alters what the module must do, and have the change reviewed.
- **Break the work down your way.** Each work item says in one line what exists or passes when it's done and serves
  one done-when item; tick it off and revise the list as you learn. Experiments belong here too, each with the
  question it answers and when you'll stop.
- **Keep the status current.** A few lines at the top: the milestone in progress, what you're doing now and since
  when, what's next, and anything blocking you (what you tried, the evidence, what would unblock it). A blocker is
  something you probed directly, not inferred from a symptom; another owner is a blocker only once you have posted
  the request where they will see it. Keep these as plain `Milestone:`, `Now:`, `Next:` and `Blocked:` lines, and
  milestone headings as `## M<n> <name>: <state>`, so tools can read them; everything else is yours to shape.
- **Rewrite in place.** The plan reflects what is true now. A one-line log at the end records meaningful changes.

A starting shape, if useful:

```markdown
# <Module> plan

Design: <path>
Reviews: <what was reviewed @ revision, brief link, verdict>; one entry per review
Milestone: M2 <name> · Updated: <YYYY-MM-DD HH:MM UTC>
Now: <what you're doing> (since <date>)
Next: <what follows>
Blocked: none

## M1 <name>: done (<review link>)
Done when:
- [x] <check, view or reference> (<evidence link>)
Work:
- [x] ...

## M2 <name>: in progress
Reference: <owner-approved reference, camera/time/settings> (approved <date>)
Done when:
- [x] <check> (<evidence link>)
- [ ] <check>
Work:
- [x] ...
- [ ] ...

## Log
- <date> <what changed and why>
```

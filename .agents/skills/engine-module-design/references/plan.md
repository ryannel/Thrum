# The plan

The plan is how the design becomes work, and how anyone can see where that work is. It sits next to the design (in Thrum,
`docs/design/modules/<MODULE>-PLAN.md`; elsewhere, beside the module's design) as `<DESIGN>-PLAN.md`, is reviewed with the design, and is committed with the work it describes. Shape it however serves
the module; what matters is below.

- **Milestones are outcomes in the game.** Each is a small, complete experience you can watch, such as one boat's wake
  reading correctly from drift to planing at the chase camera, not a component such as "the solver". The riskiest
  behaviour comes first, at shipped quality.
- **Every milestone is proven in the game** ([implementation](implementation.md#make-the-first-visual-checkpoint-real)),
  so keep each one small enough that this stays cheap.
- **Each milestone says how you'll know it's done.** List the checks and the views, speeds and references it's
  judged at, as boxes you tick when the evidence exists, with a link to it. Keep to what you would genuinely refuse to
  call it done over, including a check that would catch the plausible wrong substitute. Judge only what your module's
  mechanisms produce: a cue that hinges on another owner's choices (route, camera, ship speed) uses a fixed reference
  setup or moves to the milestone that integrates them. Ticking these is how progress
  inside a milestone shows. If what a milestone means has to change, edit it in the plan, say why in the log,
  update the design if the change alters what the module must do, and have the change reviewed.
- **Break the work down your way.** List the work under each milestone at whatever grain helps you, each item
  saying in one line what exists or passes when it's done and serving one of that milestone's done-when items (work
  that serves none belongs to another milestone), tick it off as you finish it, and revise the list as you learn. Experiments belong here too, each with the question it answers and
  when you'll stop.
- **Keep the status current.** A few lines at the top: the milestone in progress, what you're doing now and since
  when, what's next
  and anything blocking you (what you tried, the evidence, and what would unblock it). Probe a blocker directly
  before you record it, and again before you work around it; a cause inferred from a symptom is a hypothesis.
  When another owner must act, post the request where they will see it and link it; an owner who hasn't been
  asked isn't a blocker.
  Update the status when it changes.
  Keep these as plain `Milestone:`, `Now:`, `Next:` and `Blocked:` lines, and milestone headings as
  `## M<n> <name>: <state>`, so tools can read them; everything else is yours to shape.
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
Done when:
- [x] <check> (<evidence link>)
- [ ] <check>
Work:
- [x] ...
- [ ] ...

## Log
- <date> <what changed and why>
```

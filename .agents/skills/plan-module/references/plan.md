# The plan and the done-when file

The plan turns the design into work that anyone can follow. The done-when file is how a milestone's close is decided.
That decision is mechanical where possible: `scripts/agent_loop.py` reads the done-when file, and so do the evaluator
and the owner.

## The plan: `docs/design/modules/<MODULE>-PLAN.md`

- **Milestones are outcomes in the game.** Each is one small, complete experience you can watch, such as one boat's
  wake reading correctly from drift to planing at the chase camera, not a component such as "the solver".
  - The riskiest behaviour comes first, at shipped quality.
  - A milestone that needs more than about three done-when items is usually two milestones.
- **Work items** are the generator's to shape. Each says in one line what exists or passes when it's done.
- **Keep the status current** with plain lines that tools read:
  - `Milestone:` the one in progress;
  - `Now:` what's happening, and since when;
  - `Next:` what follows;
  - `Blocked:` a blocker probed directly, with what would unblock it.

  Use `## M<n> <name>: <state>` headings.
- **Rewrite in place**, so the plan reads as current. A one-line log at the end records meaningful changes.

```markdown
# <Module> plan

Design: docs/design/modules/<MODULE>.md · Done-when: docs/design/modules/<MODULE>-DONE.json
Reviews: <what was reviewed @ revision, brief link, verdict>
Milestone: M2 <name> · Updated: <YYYY-MM-DD HH:MM UTC>
Now: <what is happening> (since <date>)
Next: <what follows>
Blocked: none

## M1 <name>: done
## M2 <name>: in progress
Work:
- [x] ...
- [ ] ...

## Log
- <date> <what changed and why>
```

## The done-when file: `docs/design/modules/<MODULE>-DONE.json`

It is JSON so that agents edit fields rather than rewriting prose. Generators change only `status` and `evidence` on
items. The evaluator increments `fails` on each failed review, and `scripts/agent_loop.py next` turns that count into
the escalation. The planner owns everything else.

```json
{
  "module": "WATER",
  "milestones": [
    {
      "id": "M2",
      "name": "Wake reads from drift to planing at the chase camera",
      "state": "open",
      "review": true,
      "fails": 0,
      "bar": "Would pass in a shipped high-end naval game: wake wedge and foam as in Sea of Thieves and Assassin's Creed IV at the chase camera, judged in words, not against frames.",
      "items": [
        {
          "id": "M2.1",
          "statement": "A Kelvin wedge forms and its angle holds from 4 to 20 knots",
          "proof": "oracle",
          "check": "./dev scenario wake-speed-sweep",
          "target": "mac-metal",
          "status": "open",
          "evidence": ""
        },
        {
          "id": "M2.2",
          "statement": "Foam forms where the hull meets the water, then fades; no popping as the camera moves",
          "proof": "inspection",
          "check": "scenario chase-turn, 10 s frame sequence and contact sheet",
          "target": "mac-metal",
          "status": "open",
          "evidence": ""
        }
      ]
    }
  ]
}
```

Fields:
- **`state`:** one of `open`, `in_progress`, `done` or `parked`.
- **`review`:** `true` when an evaluator must close the milestone. Set it for visual milestones, for milestones that
  change a shared contract, and for the module's last milestone. Otherwise the oracle closes it.
- **`proof`:** one of these:
  - `oracle`: a command whose exit status decides;
  - `inspection`: an item on the inspection list, judged in the game by the evaluator;
  - `review`: an evaluator judgement that isn't visual, such as a code or contract review.
- **`target`:**
  - `mac-metal`;
  - `chrome-webgpu`;
  - `ios-sim` (function only, never speed);
  - `proxy:<what>` for a stand-in, which says what it stands in for.
- **`bar`** (visual milestones): the shipped-game standard in words, naming the examples it rests on. It is not a
  reference image to match. Golden captures are regression tripwires, never done-when items.

What makes a good item:
- **Something you'd genuinely refuse to call the milestone done over.**
- **A check that would catch the plausible wrong substitute,** such as moving texture standing in for growth.
- **Only cues this module's mechanisms produce.** A cue that hinges on another owner's choices uses a fixed setup or
  moves to the milestone that integrates them.
- **No cost targets.** Cost appears as recorded measurements and [tripwires](performance.md), never as a target.

## Changing a milestone

Splitting, re-scoping or parking happens in the open. Edit the plan and done-when file, and say why in the log.
Update the design if the change alters what the module must do. A milestone whose meaning changes gets a review of the
change, and the work carries on while it runs.

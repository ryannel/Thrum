# System diagnosis

Use this when a visual or behavioural cue stays wrong after a local fix. The cause may be the model, the
representation, an owner or a join, rather than a bug in the code you are looking at. The goal is a correction at
the layer that's responsible, checked across that layer's consumers. A local bug can still have a local fix.

## Ask whether the model can produce the behaviour at all

Before tuning, reproduce the core equations outside the engine, in a short CPU script with the shipped constants.
Then compare the result with the cue and a reference. This often settles in minutes what visual iteration doesn't
settle in hours.

The engine has paid for skipping this step. Hours went into gain, resolution, foam and shading work on a wake whose
wave model couldn't form a Kelvin wedge at varying speed, and a short CPU reproduction showed it. More resolution
can't supply a missing mechanism, or fix another quantity's calibration.

## Look for one shared cause

When several symptoms appear together, look for a common cause before fixing each one. For example, a weak wake, no
wake when turning, sudden starts and stops, and missing foam appeared together. Common causes:
- a frozen input;
- a pinned domain;
- a filter cancelling the signal;
- state that stops updating after a hitch.

Trace the quantity from its source to every consumer. Ask who owns it, and whether another path derives a
conflicting version of it.

## Name the layer

Classify the fault, and fix it at its layer:

| Kind of fault | Example |
|---|---|
| Implementation mistake | a wrong index, a sign error |
| Missing or incompatible mechanism | a single-speed wave model asked for a speed-dependent wake |
| Missing input or content | a driver that is never fed |
| Calibration | a unit or scale constant |
| Representation or scale | resolution, band or level of detail |
| Presentation | tone mapping, compositing |
| Lifecycle | hitch, reset, resize, device loss |

Damping a response, turning a term off or adding noise can isolate a defect, or hide it. Keep such a change only if
it restores the intended behaviour.

## Quantities that cross a boundary

For each quantity two modules share, the owning design states:
- its source;
- its units and coordinate frame;
- its spatial and temporal support;
- its encoding;
- how contributions combine;
- the fidelity each consumer needs.

Where consumers must agree, one module is the authority, and the others approximate it within a stated bound.

Derived quantities follow their source. Displacement and its shading normal need compatible derivatives, and a
changing frequency needs an integrated phase. Detail is resolved, filtered or statistical, and never counted twice.

## Check the correction

Write a test that exposes the broken relationship, against an analytic or independent reference where one exists.
Two evaluators that share an omission can't catch it. Then re-inspect the ordinary output in motion, with its
neighbours running, and measure the cost. Record what the experiment established and what remains unproved in the
plan log.

# Host: one device recipe, one host core, one contract for a game

14 November 2026, revised 15 November after [the first design review](../../../.orchestrate/evidence/HOST/design-review.md).
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../../.orchestrate/evidence/code-review-2026-11-14/CODE-REVIEW.md), theme T4 (H01, H15, M01, M02, M03, M56,
M76, M78, M79) · [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md)

## The request

Two requests meet here. The lead's order for the review's fixes: "one device recipe and host core (T4)", then the
error convention. And the owner on the game: "it sounds like we're blurring the lines between the engine and the
game a ton. Please let's separate out the game as much as we can", for which the lead's task says "engine
host/shell code moves into engine crates (also H15/M03)".

## Purpose and who it is for

There are three shells (the Mac window, the browser page, the iPhone app) and several capture paths, and each
builds its own device request, fault watch, recovery, pause policy and wait-until-drawn loop. The copies disagree,
and the disagreements reach players: no compressed textures and no sea on iPhone, a GPU error that kills the Mac
window and is survived on iPhone. The V1 game then wraps the engine's frame host in about sixty forwarding
methods, which every shell and bench goes through, so the game's public surface is mostly the engine's.

This module gives every host one implementation of each, in the engine. Its users are the people who run a game
on each platform, and the next game: [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) can only lift a game out of the
repository once the game holds none of the engine's host code.

## What others do

Bevy asks for its device in one place (`RenderPlugin`, from `WgpuSettings`, which takes the adapter's limits by
default), and winit gives every platform one `ApplicationHandler` to implement. Unreal's RHI and SDL's GPU device
are the same idea: the platform differs under one request and one lifecycle. wgpu's native default for an
uncaptured error is to panic, so a host that installs no handler dies on an error another host records.

## Mechanisms

**1. The device recipe**, in `thrum-render`'s capabilities, beside the registry it feeds. One entry point takes
an instance, an optional surface, a profile and an optional restriction, and returns the adapter, device, queue
and capability record. It owns:

- **The request.** A *native* profile asks for every timing and content feature the adapter offers and for the
  adapter's own limits. A *portable* profile is the browser's: the request Chrome gets today. It is a named input
  with its reason, the hardware-native decision's "the browser runs the portable recipe", not a second copy. So
  Chrome's cooked content keeps the portable profile and no exact baseline moves. Compressed textures in Chrome
  would change default output; that is `CONTENT-LOOP`'s to propose under the baseline rules.
- **The restriction.** A caller may lower limits and remove features from what its profile asks, and can add
  nothing. Tests need it: `thrum-sea` proves that it refuses a device with seven storage buffers a stage, and runs
  its harness on WebGPU's default limits as well as the adapter's. Those are test inputs, so they go through the
  one request as a restricted profile and not around it. No shell passes a restriction.
- **The fault watch.** One watch holds the lost flag, the device generation, the first loss message and the first
  uncaptured error. An uncaptured error is recorded and the process lives. Only the abort goes: what a fault
  invalidates is unchanged. A delayed execution error still retires the device generation and resets its
  histories, as [C07](../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4) and
  `APPLE-PRESENTATION` require, where an isolated validation error is recorded and the generation carries on.
- **The surface.** The SDR format preference, and a configure that clears a stale drawable after a loss by
  retrying inside an error scope where the platform has one.
- **The first device generation**, as one constant.

`TEST-SUPPORT`'s gate hands tests a device from the same recipe, by profile and restriction. The
`device-request` rule holds every other call site at zero.

**2. The host core.** Three things every shell decides, each in one place:

| What | Where | Why there |
|---|---|---|
| Pause and overload: draining, the explicit resume, one count for each overload episode, the race with a resume not yet consumed | `thrum-core`, beside the clock that defines overload | It decides whether simulated time is lost or counted twice, and it has no platform in it. It serves a world on its own thread and one stepped inline |
| What a surface outcome means: reconfigure, recover, skip | One platform-neutral function in `thrum-app` | The same outcome is recovered on the Mac and fatal on the web today |
| Drawing one frame without a readback, and waiting for cooked content to be drawn | The frame host, and `thrum-asset-gpu` | About thirty sites pump staging by reading a full frame back |

**3. The game holds the engine's frame host.** The game exposes its scene, its world and its own controls, and
shells, benches and the iPhone app hold `thrum_render`'s frame host directly. Capture to disk goes one way, on the
engine's offscreen driver; the second capture path under `src/bin/window/` goes.

**4. One contract for a hosted game.** `thrum-app`'s `Game` exists only on macOS because it names winit's key type,
and the browser has a second contract. The contract becomes one platform-neutral half (prepare, apply edits, read
back, replay, controls, world) with a key type the shell owns, and an adapter for each of the window, the page and
the offscreen driver. The browser's binding of typed controls becomes the engine's, as the Mac panel is. The Mac
qualification harness takes its control and schedule from its caller and attaches as one observer, so the shell
names nothing of any game. The test of the whole: a second game copies nothing from the first.

The same platform-neutral half holds the render owner's frame job and reply: the request a shell sends its render
owner for one frame and what comes back (the late-acquired target time, the frame samples, the outcome). The Mac's
render actor and the iPhone app's render owner each define their own today, with the same two sample slots. They
become one pair of types that both use. The iPhone app keeps its owner thread and everything UIKit; it does not
go onto the game contract here.

**5. Typed errors in the shells** (T3), last, when the signatures have stopped moving.

## The drifts, and where each ends

| Drift between hosts today | Ends as | Closed in |
|---|---|---|
| Features: content features on the Mac, timing only on iPhone and in the browser | The native profile everywhere native; the portable profile, by name, in the browser | WP1 |
| Limits: `downlevel_defaults` on iPhone, wgpu's defaults elsewhere | The adapter's limits on every native host | WP1 |
| Adapter choice differs between the two game capture paths | The recipe's one choice | WP1 |
| Fault watch: three, with different contents | The recipe's one watch | WP1 |
| An uncaptured error is fatal on the Mac and in the browser, survived on iPhone | Recorded and survived | WP1 |
| Surface configure retries on the Mac only | The recipe's configure | WP1 |
| The SDR format list, three copies; the first generation, eleven literals | One of each | WP1 |
| A validation failure at acquire is fatal in the browser | Recovered, by the one outcome mapping | WP2 |
| Pause and overload, four copies | One policy in core | WP2 |
| Waiting for cooked content by reading frames back, about thirty sites | Render once and await drawn | WP3 |
| The game's forwarding host, and two ways to capture the game to disk | The engine's frame host held directly; one capture path | WP3 |
| `Game` exists on macOS only; the browser has a second contract; the sky kit builds its host twice | One contract, three adapters, one construction | WP4 |
| The Mac qualification harness names a game's control | An observer given its control by its caller | WP4 |
| The render owner's frame job and reply, written twice (Mac and iPhone) | One pair of types in `thrum-app` | WP4 |
| The iPhone app's owner thread, UIKit lifecycle, surface lease, touch and command table | Stay in the iPhone app; `GAME-UI` WP6 moves them to `thrum-app` | Not here |
| The display link: Swift on iPhone, `objc2` on the Mac | A fact of the platform | Not a drift |

A milestone is judged on its own rows. WP2 closes the device, fault, surface and lifecycle policies; the contract
and the render owner's types are WP4's, and a review of WP2 does not ask for them.

## Interfaces

| Interface | For |
|---|---|
| **Device recipe**: a device, its capability record and its fault watch from one request, by profile and optional restriction | Every shell, capture, bench and test; `TEST-SUPPORT` |
| **World lifecycle policy**: pause, overload and resume for both drive styles | The three shells; `SESSION` WP7's driven world extends it |
| **Render once** and **await drawn** | Shells, captures, tests |
| **Game contract**: what a game implements to run in the window, on the page and offscreen | Games, the kit scenes; `GAME-UI` WP6 adds the iPhone adapter |
| **Frame job and reply**: what a shell's render owner is asked for one frame, and what it answers | The Mac render actor, the iPhone app's owner; `GAME-UI` WP6 |

## Not built here

The iPhone app's UIKit lifecycle and render owner stay where they are until `GAME-UI` WP6 moves them into
`thrum-app`; this module makes that a move by taking out first what the copies share: the device request (WP1),
the lifecycle and surface policy (WP2), the forwarding host (WP3) and the frame job and reply (WP4). A still scene that renders at
the display's full rate forever (M58) is left: idling a host changes what the cadence qualification measured, and
belongs with the governor.

## What it must not do

Change what a registered baseline draws; edit a check or a test another module left for its lifecycle; put a
game's control, scene or type in an engine crate; read the environment in a library.

## Proof

Unit tests hold the recipe, the fault watch, the policy and the outcome mapping, each with a case that fails
without the fix. Each target then runs its own lifecycle: a real window, a real Chrome and the iPhone simulator,
through the checks the platform modules left, unedited. The simulator proves function only. The verifier holds the
image in every milestone.

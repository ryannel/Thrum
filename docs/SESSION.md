# Put a game in a match

A match has one authoritative host. Each client loads a baseline of the world, then runs the same rules on the
input the host admitted for each tick. The host can run headless or reserve a slot for a local player. Keep
`thrum-network` and `thrum-transport` in your multiplayer package. Your standalone game can use the same rules
without depending on either crate.

## Declare the game rules

1. Put the simulation in a rule package under Thrum's deterministic rules profile. Encode and decode every
   authority component and resource the rules can read. Put mutable gameplay settings in authority resources.
   Keep rendering, local UI and wall-clock reads outside the rules. Run your rule package's conformance trace on
   each target you plan to support. [Rules profile](plans/SIM-2D/DESIGN.md#mechanisms),
   [conformance](plans/SIM-2D/DESIGN.md#proof-and-builder-handoff)
2. Derive a `RuleSetIdentity` when you build. Hash the rule sources and constants, their engine dependencies,
   the toolchain and your action wire. Use the same inputs on all targets, and mix in every immutable setting
   the rules read outside the world. The identity is the deterministic rule set and build in the gameplay
   semantic manifest. The host refuses a different identity at negotiation.
   [Reference identity build](../examples/match-proof/build.rs)
3. Implement `thrum_network::session::World` for your game-owned runtime. Supply the identity, committed tick,
   atomic `step`, digest, world image and restore. With `GameRuntime`, write the image and digest from the same
   authority state. On restore, decode into a fresh unseeded runtime, check the expected tick, publish its first
   endpoint and replace your replica only when all of that succeeds. Keep the rules and resources in the game
   package. [World interface](../crates/thrum-network/src/session.rs#L215)

`World::step` receives one frame for each slot and the roster events for that same tick. The rule set must step
for every set it admits. If an action is illegal in the world it meets, the rule can leave gameplay state alone
and commit the tick. If a step on the host refuses the set, the match ends. The host sends only committed sets.
The client compares world digests at the match's configured interval and requests repair when one differs. A
transient difference that the rules overwrite before the next digest tick goes unreported.

## Wire the actions and the roster

Implement `thrum_network::input_batch::ActionWire` for your action enum. Give each action a stable schema ID
and canonical byte layout. The encoder has up to 256 bytes per action. Validate the ID, exact length and value
on decode. A slot frame holds up to eight actions. Apply actions in the order the frame supplies them.

| Choose | Send through | What happens |
| --- | --- | --- |
| Timed action, such as a jump at the sampled moment | The `Controls` returned from the sampling callback to `Client::advance` | The frame targets a specific tick. A late timed action is lost. Missing frames may hold axes briefly and carry zero timed actions. |
| Ordered command, such as choosing an upgrade | `Client::command` | On a live link the host admits the command in sequence at the first barrier with room. The command has no target-tick expiry. |

A committed result identifies the tick that included the input. The game rule decides whether the included
action had an effect. A disconnect can leave a command's outcome uncertain. The new connection starts from a
fresh baseline and the session never resends old commands. Use that world to recover what the player sees.

`RosterEvents` uses one bit per slot, in the same order as the frames. On `joined`, create the character in
the rule. On `away`, keep the character and let it idle. On `back`, resume the same character. On `left`,
remove it. These transitions commit on all worlds with the same tick. Store any slot-to-entity mapping in the
authority state so a baseline carries it. `Roster::occupied` and `Roster::away` supply the display status.
[Integer game fixture](../crates/thrum-network/tests/support.rs)

## Draw a predicted controller

Implement `PredictedController<YourWorld>` for the local character if it needs immediate feedback. Give it a
complete copyable state, immutable rule constants, a read of the confirmed character for a slot, and a
bit-exact comparison. Call the same `step` from the authority rule and the predictor with wire-canonical axes
and timed actions. Keep any dependency of that step in the controller state or rule constants. Construct the
replica with `Client::with_controller` and pass `None` for the fixed-lead override to use adaptive lead.
`Client::adaptive` runs without a predicted controller.

Interpolate the local hero using `Client::controller(now)` and draw the crowd and other heroes with the
confirmed endpoints at `Client::playout(now)`. The crowd is drawn behind the predicted hero by the input lead
plus the confirmed delay. A hero the crowd pushes is corrected every tick the crowd pushes it. Correction
events from `advance_with_prediction_events` let the game smooth the rendered pose. The predictor keeps 64
ticks of history and returns to the confirmed character when that span is exhausted or the world is repaired.
[Controller design](plans/SESSION/DESIGN.md#11-the-predicted-controller)

## Start a host and join with its ticket

1. Enable `thrum-transport`'s `native` feature for the Mac and iPhone clients and Mac host, or `browser` for
   the Wasm client. On the Mac start a `thrum_transport::native::Host` with the bind address and transport `Profile`. Construct
   a `thrum_network::session::Host` with the seeded world, action codec, `Settings`, a fresh nonzero match ID,
   seed and current monotonic time in microseconds. Pass `false` for a headless host. For a listen host, pass
   `true` to reserve slot zero and submit its frames with `Host::stage_local`.
2. Poll the transport host's `ticket()` and share its text with your players. Anyone with the ticket, a
   matching build, network access and a free slot can join. The ticket pins the host's certificate. The session
   supplies no accounts or passwords. Feed each accepted `Link` into `Host::connect`.
3. To join, parse the ticket and call `thrum_transport::native::connect` on the Mac or iPhone, or
   `thrum_transport::browser::connect` from a secure page in Chrome. Give the link, unseeded world, action wire
   and monotonic time to your client constructor. Each peer uses a consistent local clock in microseconds.
   [Transport setup and ticket format](TRANSPORT.md#start-a-host-and-hand-out-a-ticket)
4. Call `Host::advance(now, u64::MAX)` regularly to poll links. Call `Host::start(now)` when the lobby is ready
   to run ticks. Call `Client::advance(now, sample_controls)` regularly on each client. Its sampling callback
   receives the target tick. After every advance, call `Client::result()` until it returns `None`. Drain both
   timed-frame and command results even if your game ignores their dispositions. Unread results count towards
   input capacity and eventually stop the client sampling new frames. Use this loop for sockets and rehearsal.
   Poll before drawing so the game sees the latest state the session can present.

   ```rust
   client.advance(now_us, sample_controls)?;
   while let Some(result) = client.result() {
       handle_result(result); // This can be a no-op if the game ignores dispositions.
   }
   ```

5. Use `Client::state`, `baseline_progress`, `newest`, `held`, `reported_lead`, `take_stall` and `take_skip` to
   report connection and playout status. Show the reason on `Refused`, `Ended` or `Closed`. The host can read
   `Host::stream` and `Host::take_disconnect` for peer progress and close reasons. A peer too slow for the
   match is dropped so the host and the other peers can keep playing.
6. Use `Client::goodbye` and keep polling until the host closes the link to acknowledge departure. On link loss,
   pass the token from `Client::token` to `Client::with_token` on a new link to reclaim the slot during the
   grace period. A match ends with its host.
   When stopping cleanly, call `Host::stop` and keep advancing so clients can receive the final endpoint.

## Rehearse, then run on the targets

Run from the repository root after the [Rust and target setup](../README.md). For a small rehearsal, create a
`thrum_transport::memory::Memory` with `Profile::default()` and `Conditioner::default()`. `Memory::connect`
returns two links. Give the first to the client and the second to the host. Start at time zero and drive
`Memory::advance(now_us / 1_000)`, `Host::advance(now_us, through)` and `Client::advance(now_us, sample)`
from the same simulated clock. Each side owns its own world. The integer fixture covers actions, commands and
roster events. The full crowd rehearsal runs A0 and harsh profiles in virtual time, checking every scheduled
digest across four replicas.

```sh
cargo test --locked --offline -p thrum-network --test it
python3 scripts/session_match.py closure
python3 scripts/session_match.py memory steady
```

For sockets, use a Mac with Node.js, Chrome, the workspace's `wasm-bindgen` CLI (the version is in the
[starter setup](../examples/starter/README.md)), and Xcode with the iPhone 18 Pro simulator runtime installed.
Export `ORCHESTRATE_SCRIPT` with your installed `agent_loop.py` path so the command can admit Chrome and the
simulator under their resource locks. Run the full reference match with one headless Mac host, two native
Mac clients, a headless Chrome page and an iPhone simulator client. It runs those clients together for ten
minutes through A0's loopback relay and prints the saved JSON record path when the checks pass.

```sh
python3 scripts/session_match.py sockets steady
```

For one target's socket fault scenario, the `sockets faults` subcommand accepts `--target` with `mac-metal`,
`chrome-webgpu` or `ios-sim`. The host stays on the Mac. The full steady scenario always exercises all three
client targets.

The reference match is a headless loopback proof on one Mac. The simulator qualifies function. The match uses
one full world per peer, samples digests at committed ticks, and predicts only the local controller. It has no
host migration or whole-world rollback. [Networking design and evidence scope](design/modules/NETWORKING.md)

## Run your game's proof programs

Build a self-contained Mac executable with your game's world, action codec and scripted player. Set
`GAME_MATCH_PROGRAM` to its path. The `game` scenario accepts this executable directly.

```sh
python3 scripts/session_match.py memory game --program "$GAME_MATCH_PROGRAM" --ticks 120
```

The executable receives `memory PROFILE TICKS`. Set `--profile` to `a0` (the default), `harsh`, or `local`.
Run a host and at least one independently restored client in virtual time. Send one JSON object to standard
output, with `digest_interval`, a `host` object and a `players` array. Each world object supplies its final
`tick`, its hexadecimal `rule_set` identity and its own ordered `digests` array of `[tick, digest]` pairs.
Record a digest at each positive multiple of the interval. The runner checks the complete cadence, rule-set
identity and equality across all recorded worlds.

Reuse the same Mac executable for the socket host and one Mac client. Supply `GAME_IOS_PROGRAM` and
`GAME_CHROME_PAGE` to add an iPhone simulator executable and a prepared Chrome proof directory. Omit either
flag to leave out that client. The runner takes device locks, starts the processes and relays, and compares
the baseline and digest records. Use `--bodies` to pass a game world size (default 128).

```sh
python3 scripts/session_match.py sockets game --program "$GAME_MATCH_PROGRAM" --ticks 120 \
    --ios-program "$GAME_IOS_PROGRAM" --chrome-page "$GAME_CHROME_PAGE"
```

The socket program uses these arguments, in order.

| Mode | Arguments after the mode |
| --- | --- |
| `socket-host` | `BODIES TICKS CONTROLLER LISTEN TRACE JOIN_MS CONTROL_FILE` |
| `socket-client` | `TICKET BODIES CONTROLLER CONTROL_FILE TOKEN_FILE TRACE` |

For `sockets game`, the controller is `bbr`, listen is `false` and the trace is `default`. Start the authority
clock when the runner writes `run` to the host control file. Flush each event as one JSON object per line,
with an `event` key and the fields below. Use the same digest encoding on both endpoints.

| Endpoint | Event | Fields |
| --- | --- | --- |
| Host | `host` | `ticket` as text, `pid`, `rule_set`, `digest_interval` |
| Host | `image` | Baseline `tick`, `digest`, `bytes` |
| Host | `start` | `clock_us` on the host's monotonic clock |
| Host | `commit` | `tick`, `clock_us`, per-slot `admissions` booleans and cumulative `held` counts |
| Host | `finish` | Final `tick` |
| Native client | `client` | `pid` |
| Client | `baseline` | Restored `tick`, `digest`, `bytes`, `rule_set` |
| Both | `digest` | `tick`, `digest` at every positive multiple of the interval |
| Client | `state` | Committed `tick` and `state`, formatted as `Ended(...)` at the final tick |

The runner starts after each client has restored the lobby at tick zero. Continue advancing after the last
commit so the host can deliver the end of the match. [Reference host and native client](../examples/match-proof/src/sockets/native.rs),
[reference client events](../examples/match-proof/src/sockets.rs)

For Chrome, put your `socket_player.js` beside the `match_proof.js` and `match_proof_bg.wasm` generated from
your game by `wasm-bindgen --target web --out-name match_proof`. The runner serves the directory's three
files, reports the Chrome PID and supplies the ticket at `/config`. Your page posts its session events
to `/rows`. [Reference browser entry point](../examples/match-proof/socket_player.js)

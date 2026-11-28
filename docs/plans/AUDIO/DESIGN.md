# AUDIO: a game's sounds and music are heard on the Mac, in Chrome and on the iPhone simulator

27 November 2026, revised 28 November 2026 after [design review 1](../../../.orchestrate/evidence/AUDIO/design-review.md). [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk) · [the vision and scope document](../VISION.md).
[C11 in CONTRACTS](../../design/CONTRACTS.md#c11-input-audio-and-ui-lifecycle--p1a1u1-with-f1) governs, and
[the Game SDK's audio entry](../GAME-SDK/DESIGN.md#audio) is what the first consumer expects.

## The request

The owner approved the Game SDK's design on 27 November 2026 and answered its third scope question, "Audio in
the first release?", with the planner's recommendation: yes, in the small form, behind an engine module named
`AUDIO`. [The module list](../ARCHITECTURE.md#module-planning-input-track-7) gives this module its input:

> one-shot sounds with a voice limit, looped sounds with a place, streamed music with a cross-fade and three
> volumes, with no effects and no mixing graph. Its planner chooses how each target's device is reached and
> which crates decode and play, and registers them. No agent can hear, so its items are proved on the mix
> rendered to a buffer, and how it sounds is judged by the owner

The lead's task for this plan adds: "No agent can hear, so items that judge sound wait for the owner's ear;
keep those few and clearly marked."

**When it is built.** After V1's work: an `AUDIO` work package takes a builder only when no V1 work package is
waiting for one ([the module list](../ARCHITECTURE.md#off-the-v1-path-track-7)). It waits for no other module.
`GAME-SDK` WP18 waits for [WP4 here](#the-work-packages).

This design is not a blind study. Read for it: [the SDK's audio entry](../GAME-SDK/DESIGN.md#audio) and
[its interface request](../GAME-SDK/DESIGN.md#interfaces); C11; the earlier family design
[`docs/design/modules/AUDIO-UI.md`](../../design/modules/AUDIO-UI.md), which this design replaces for sound;
`thrum-asset` and `thrum-cook` as built; `thrum-app`'s shells; the source of `cpal` 0.18.2; and
[a planning probe](../../../.orchestrate/evidence/AUDIO/planning-probe.md) run on the Mac and in Chrome. The
probe's code was thrown away on purpose.

## Purpose and scope

A game says what should be heard, and the engine makes it heard: on the Mac's output, in a Chrome tab and in
the iPhone simulator, from one mix, without the sound ever waiting for a frame or a frame for the sound.

**Consumer.** `GAME-SDK`, in WP18: its `Audio` parameter, its `Sound` look, `Screen<Music>` and its `Mix`
setting are written over [what this module provides](#what-audio-provides). A game written on the engine's own
API may use it the same way. Nothing else uses it.

**What exists.** No sound code. `thrum-asset` and `thrum-cook` hold five product kinds and none is a sound.
The shells in `thrum-app` open no audio device.

**Commissioned.** The twelve mechanisms below, for the behaviours in the next section.

**Not built.** Effects of any kind: reverb, filters, occlusion, doppler. A mixing graph, or any volume beyond
the three. Ducking. Surround or binaural output: the mix is stereo. More than one listener. A sound scheduled
for an exact future frame or on a beat. Loudness normalisation. Microphone input, voice chat, synthesis.
Music fetched from the network while it plays: a track is fetched whole. An output on Linux or Windows, where
the crate builds and renders to a buffer only. Safari and Firefox are designed for and not run. A physical
iPhone is not run (the vision and scope document, "Where it runs").

**Limits carried from the vision and scope document.** No GPL or share-alike code or data comes in, and a
file copied or translated from elsewhere is listed in `docs/provenance/AUDIO.md`. The simulator proves
function, never speed. No time, byte count or allocation count is a budget (decision 3): costs are tripwire
records, apart from the two contracts in [Performance](#performance).

## What a game must be able to hear

Each behaviour has its range, and the wrong substitute that would look right to an agent that cannot hear.

| Behaviour | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **B1 A sound that happens** plays once, from its first frame to its last | Any cooked sound, mono or stereo, from one frame long to longer than many pulls; volume 0 to 1; pitch 0.5 to 2; forty plays a second of one sound (the SDK's example) | One voice a sound, so a new play cuts the last. Rejected by two plays that overlap and must sum. A pitch snapped to a few ratios. Rejected by a ratio inside the SDK's 0.9 to 1.1, whose frames are stated |
| **B2 A limit** keeps at most `n` voices of one sound | `n` from 1 up; forty plays a second under a limit of 4 | Counting plays, not voices that are still sounding. Rejected by plays spread over longer than the sound lasts. A cap that ignores `n`. Rejected by naming exactly which of the forty plays start, for three lengths of sound |
| **B3 A sound a thing makes** loops while the thing exists, follows it, and is turned up, down, held and stopped | 10,000 things with a sound of which 1,000 to 2,000 move each frame, the representative scene of decision 1; a loop from one frame long; updates every frame | Restarting the loop on each update. Rejected by position continuity across an update. A loop with a gap or a repeated frame at its wrap. Rejected by an exact comparison across three wraps |
| **B4 A place** makes a sound quieter with distance and puts it on the side it stands | From the listener's own position to the sound's radius and beyond; the place, the listener and the listener's facing all moving between pulls; a place passing through the listener | Gains worked out once, when the sound starts. Rejected by a moving thing and a turning listener. Left and right swapped. Rejected by a place on the listener's left that must be louder in the first channel |
| **B5 Music** plays from its compressed form, loops from its loop point for ever, and cross-fades to the next track | Tracks of minutes; a loop point anywhere in the track; a change made part-way through a fade; the same track asked for again; no track | The whole track decoded when it is added. Rejected by what adding a long track allocates. A cut, a fade to silence and back, or a fade squeezed into a few frames. Rejected by each track's gain at five points across the fade. A restart when the same track is said again. Rejected by position continuity |
| **B6 Three volumes**, master, music and effects, each move what they govern | 0 to 1, changed at any moment | A volume applied when a voice starts. Rejected by a change while a loop sounds |
| **B7 Nothing waits.** A call from a frame returns at once, and a pull never blocks | A pull with every voice sounding; a page held for half a second; a sound added while others play; a track of minutes arriving in Chrome while a menu clicks | Mixing on the page's own thread in Chrome, which is what the usual Rust device crate does. Rejected by a held page during which the mix must keep its rate |
| **B8 The same sound everywhere, and with no device** | The Mac, Chrome and the simulator; a pull split into any sizes; no device at all | A second mixer for the browser built from Web Audio nodes. Rejected because Chrome must render [the reference script](#12-the-reference-script-and-the-handle-run) to the same digest. An output that ignores a game's calls, or hands the device silence. Rejected by a sound played through the handle and found in [the output tap](#10-sound-outputs) |
| **B9 Quiet when it should be** | Before a browser's first key or pointer press; a hidden tab; an app in the background; no device; a device that goes away | Queued sounds that all burst out when the output starts. Rejected by a count of sounds dropped while nothing was pulling. A sound queued just before the app went to the background, played on its return. Rejected by that play's outcome and by the frames after pulling starts again |
| **B10 Loud is safe** | Every voice sounding at full volume at once | A hard clamp at the device, or none. Rejected by a bound on every output sample and by sums below the knee passing unchanged |
| **B11 Nothing when unused** | A game that plays no sound | A feature inside an existing crate. Rejected by the dependency closure of a package that does not name the audio crate |
| **B12 A record of what it did** | Every count in [the record](#11-the-record), on every output | Counters on the Mac only. The record marks what an output cannot know as absent |

## Mechanisms

### 1. One mix, pulled

**The mix** sums every sounding voice into stereo frames of 32-bit float at 48,000 frames a second. It is one
piece of Rust source and it is the only place sound is summed. It never runs by itself: something **pulls** a
number of frames from it. A sound output pulls from its device's callback, and a test pulls into a buffer.
Both platforms took 48 kHz stereo float as asked in the probe, 512 frames a pull on the Mac and 128 in Chrome.

Three rules bind the mix's code, and the rest of the design rests on them:

- **A pull does bounded work and nothing else.** It takes no lock, waits for nothing, allocates and frees
  nothing, and reads no file, clock or system call. Its work is linear in the voices sounding and the frames
  asked for. C11 asks this of an audio callback.
- **The frames depend only on the commands and the frame at which each was applied**, never on how the pull
  was split. Rendering 512 frames at once and rendering them one at a time give the same bits.
- **The frames are the same bits on every target.** The mix path uses only operations whose results are
  fixed by IEEE 754 and by integer arithmetic: add, subtract, multiply, divide, square root, compare. No
  platform maths function (`sin`, `exp`, `pow`), no fused multiply-add, no table built with one. The probe's
  mixer-shaped loop gave one digest natively, under Node and inside Chrome's audio worklet, at every pull
  size tried.

The third rule is here for proof, not for players. Every fault in sound is silent to an agent, so equal bits
are the one check of "Chrome plays what the Mac plays" that needs no ear and no tolerance.

### 2. The game side says what should be heard

A game holds **a handle**. Every call on it returns at once and never fails a frame. The handle is the one
authority for what *should* be heard: the sounds added, each emitter with its place and volume, the music
track, the three volumes and the listener. The mix is the one authority for what *is* sounding: voices, music
streams, their positions and their present gains. Neither holds world state, and nothing heard is read back
into a game's rules.

The handle turns each change into **a command**, which is plain data with a fixed layout. Commands travel one
way, from the handle to the mix, by a route that never makes either side wait:

- beside the mix in one process (the Mac, the simulator, a test), through a queue for one writer and one
  reader that neither locks nor allocates;
- in Chrome, as one message a frame to the audio worklet, where the mix lives in a WebAssembly instance of its
  own ([mechanism 10](#10-sound-outputs)).

Because commands are bytes either way, the mix that applies them is the same code in both.

A mix applies the commands that have arrived at the start of a pull, in order. A sound started by a frame is
therefore heard from the next pull: up to 512 frames later on this Mac, and 128 in Chrome.

**Nothing is in flight that the mix has no room for.** The handle sends a command only when the mix's queue
has room for it. Natively it sees the queue's room. In Chrome it learns it from the worklet's answers
([mechanism 10](#10-sound-outputs)). The rule for a full queue, in the table below, is the one rule for
pressure on every route.

**A play has an identity, an epoch and one fate.** A play carries **an event identity**, 64 bits that its
caller chooses, and the tick its event belongs to, or none. These are C11's durable event identity and
simulation time. The SDK gives its event's. A caller with no event, such as a menu's click, gives any number.
AUDIO carries both and gives them back, and reads no meaning into either: it never compares two plays by
identity, never refuses a repeat and never schedules by the tick. Which predicted event is heard, and that it
is heard once, stays the SDK's.

The handle stamps each play with [**the output epoch**](#10-sound-outputs) in which it was called, which is
C11's audio-device epoch. A play is given a voice only in the epoch it was called in.

Each play ends in exactly one **fate**. It is `started`, when the mix gave it a voice and rendered its first
frame, or it is the one cause it never started: `limit`, `pool`, `out of range`, `nothing pulling`,
`interrupted`, `queue full` or `bad value`. The fate is published as **an outcome**: the play's event
identity, its tick, its epoch and its fate. This is the bookkeeping C11 asks of audio: the call is the event
seen, the command is the event queued, and the outcome says rendered or expired. The handle gathers outcomes,
its own and the mix's, into a list of a fixed size that the game drains when it chooses. If more arrive
between two drains than the list holds, the oldest are lost and the loss is counted, and
[the record](#11-the-record)'s counts by cause stay exact. An outcome says a play was mixed. It cannot say a
speaker played it.

**What a call does when it cannot be honoured.** Sound is presentation, so nothing is refused with an error
after setup ([the engineering rules](../../../.agents/skills/thrum-engineering/SKILL.md#engine-rules)):

| Case | What happens |
|---|---|
| A volume or pitch outside its range | Clamped, and counted |
| A value that is not finite | The call is dropped, and counted. A play's fate is `bad value`. A debug build asserts |
| The command queue is full | A sound that happens is dropped, with the fate `queue full`. A change of state is never lost: the handle keeps the latest value and sends it when there is room |
| Nothing is pulling ([B9](#what-a-game-must-be-able-to-hear)) | A sound that happens is dropped, with the fate `nothing pulling`. State is kept and is what sounds when pulling starts |
| Pulling stopped after the call and before the mix applied it | The play expires, with the fate `interrupted`, and is never heard ([mechanism 10](#10-sound-outputs)) |

Setup can fail, and returns a typed error with the state unchanged: adding a product that does not validate,
adding more sounds than the mix was made for, opening an output.

**The handle can rebuild a mix.** Since it holds everything that should be heard, an output that starts late,
or again after its device went away, is sent the present state: emitters and music sound, and no sound that
happened before is played. A new mix begins a new output epoch, so the rule for an interruption in
[mechanism 10](#10-sound-outputs) covers it.

### 3. Voices

**A voice** is one sounding instance of a sound. A sound that happens takes one voice, plays its frames once
and frees it. Two plays of one sound are two voices and are summed.

A cooked sound is 16-bit samples at 48 kHz, mono or stereo ([mechanism 9](#9-cooked-sounds-and-the-cook)).
A voice reads it at a **pitch**, a ratio from 0.5 to 2 that changes speed and pitch together, through a
four-point cubic between samples. At pitch 1 a voice gives the sound's own samples exactly, each divided by
32,768. The range is an octave either way because the cubic has no filter: above 2 the folding back of high
frequencies stops being slight. A voice with no place plays a mono sound equally in both channels and a
stereo sound channel for channel.

### 4. A place and the listener

**The listener** is one position, one unit vector pointing to its right, and one length, `near`. There is one
listener for a mix, as C11 asks. **A place** is a position and a **radius**, the distance at which the sound
is silent. All are three numbers, in the game's own units. A 2D game gives 0 for the third.

For a voice with a place, with `o` the place minus the listener's position, `d = |o|` and `x = o · right`:

- **gain** is `(1 - d/radius)²` inside the radius and 0 at it and beyond. It is 1 at the listener, falls all
  the way, and meets zero with no step, so a sound crossing its radius makes no click.
- **pan** is `p = x / sqrt(d² + near²)`, between -1 and 1. The channels are scaled by `sqrt(1 - p)` and
  `sqrt(1 + p)`, so the power heard does not change with pan, and a sound at the listener is as loud as the
  same sound with no place. `near` keeps the pan continuous as a place passes through the listener: it goes
  through the centre and does not flip.
- a stereo sound with a place is first summed to mono.

The curve's shape, `near`'s starting value and the glide below are tuned by ear at
[the owner's sitting](#what-only-an-ear-can-judge). The properties in the three points are not.

Gains are recomputed when a command moves a place or the listener, never once a pull, and each voice's two
channel gains then **glide** to the new values by a fixed step a frame ([mechanism 7](#7-glides-ramps-and-the-clip)).
Audibility is by radius alone. Whether a thing is on screen has no part in it, which C11 asks.

### 5. Emitters

**An emitter** is a looping sound that a thing makes: a sound, a place or none, a volume and a pitch. It
lasts until it is ended. The handle holds every emitter as plain data, and only some hold a voice in the mix.
Which do is settled once a frame, in the handle's update, by one policy. The items judge by this policy and
by nothing else.

- **Eligible.** An emitter is eligible while it is inside its radius, or has no place. A held emitter stays
  eligible.
- **Loudness.** An emitter's loudness is its volume times its distance gain from
  [mechanism 4](#4-a-place-and-the-listener), which is 1 with no place. Nothing else counts: not the sound's
  samples, the pan, a glide under way, a hold, or the three volumes. Of two equally loud emitters, the one
  started earlier ranks as the louder.
- **The margin** is a ratio in the mix's configuration. A challenger beats a holder only when its loudness is
  more than the holder's times one plus the margin.
- **The pass**, once an update, in this order:
  1. An emitter that holds a voice and is no longer eligible gives it up.
  2. While a voice of the emitters' share of the pool ([mechanism 6](#6-the-limit-and-the-pool)) is free and
     an eligible emitter has none, the loudest such emitter takes one.
  3. While the loudest eligible emitter without a voice beats the quietest emitter with one, the two change
     places.

So an eligible emitter that holds a voice keeps it until another beats it by the margin, and two emitters of
nearly equal loudness do not swap each frame. After a pass, the emitters holding voices are all eligible,
they number the smaller of the share and the eligible, and no emitter without a voice beats one with a
voice. An emitter with no place is always eligible. It holds a voice when the policy gives it one, like any
other.

An emitter given a voice starts at the loop's first frame, at zero gain, and glides up. One that gives its
voice up ramps out. The pass is arithmetic over plain arrays, with work that grows with the emitters and not
with their square, and with no command for an emitter whose state the mix need not know.

An emitter's voice wraps from its last frame to its first with no gap and no repeated frame. Setting its
volume or pitch, or moving it, glides and never restarts it. **Holding** it ramps it to silence and freezes
its position, and letting go ramps it back from that frame: a paused world's things wait this way. Ending it
ramps it out.

### 6. The limit and the pool

**A limit** belongs to a play: at most `n` voices of that sound at once. The mix counts the voices of the
sound that are still sounding when the command is applied, and drops the play if there are `n`. A voice stops
counting once its last frame has been rendered, so a play applied at the very next frame is admitted, and a
play is admitted whenever fewer than `n` are sounding. It is counted in the mix because only the mix knows
when a voice ended.

**The pool** is the fixed number of voices a mix was made with, in two shares: one for emitters, which the
handle gives out by the policy above, and one for sounds that happen. When that share is full, a new play
takes the place of its quietest voice if the new one would be louder, and the displaced voice ramps out. If it
would not be louder, the new play is dropped. Both are counted.

The sizes of the pool, of each share, of the command queue, of the list of outcomes and of the table of
sounds, and the emitters' margin, are fields of the mix's configuration. Their defaults are the builder's to choose in WP1 and are recorded with the cost measured at
them. None is a limit this design states.

### 7. Glides, ramps and the clip

Nothing in the mix changes a gain in one step.

- **A glide** moves a gain towards its target by at most a fixed step each frame. Volumes, pan, distance and
  the three volumes all glide. A sound that happens starts at its gains, so its attack is its own. An
  emitter's voice starts at zero and glides up.
- **A ramp** takes a voice to silence over a fixed number of frames before it is stopped, held, displaced or
  cut, and back up when a held one is let go. The one voice that ends with no ramp is a sound that happens
  caught by an interruption ([mechanism 10](#10-sound-outputs)): nothing was pulling, so there was no frame
  to ramp over, and the new epoch rises from silence.
- **The clip** is the last thing a frame passes through. Below a knee a sample is unchanged, bit for bit.
  Above it the sample bends towards 1 and never reaches it, by a rational curve that is continuous and
  rising. Frames that reached the knee are counted, so a game that runs hot can see that it does.

Steps and lengths are counted in frames, which is what keeps the second rule of mechanism 1. Their starting
values, and the knee's, are the builder's and are tuned by ear after
[the owner's sitting](#what-only-an-ear-can-judge).

### 8. Music

A music track plays from its compressed form: what is added to the mix is the cooked bytes, and a stream
decodes only the samples each pull asks for. Nothing holds a track decoded.

- **Three stream slots: the current, the outgoing and the tail.** One track is current. On a change the
  current stream becomes the outgoing one and the new track starts as the current. The fade runs over the
  time the call gives, counted in frames, with `t` going from 0 to 1 across it. The outgoing stream's gain is
  the gain it had at the change times `sqrt(1 - t)`, and the new stream's is `sqrt(t)`. From a steady track
  that is `sqrt(1 - t)` against `sqrt(t)`: both tracks sound all the way across the fade, each gain moves one
  way only, and the squares of the two sum to 1 at every frame.
- **A change during a fade.** The stream that was already outgoing moves to the tail, and goes from the gain
  it has to silence by a ramp of [mechanism 7](#7-glides-ramps-and-the-clip). The stream that was fading in
  becomes the outgoing one and falls from the gain it had reached, and the new track fades in, both by the
  rule above. No gain steps, and no gain that was falling rises. The tail holds one stream. A change that
  arrives while the tail is still ramping waits until it is silent, which is at most one ramp, and only the
  track asked for last waits. So three streams decode together for at most one ramp, and otherwise no more
  than two.
- **Saying the current track again does nothing.** Saying no track fades the current one out by the same
  rule, with nothing fading in. A track asked for while it is the outgoing stream is a change like any other:
  a new stream of it starts from its first frame.
- **A loop point.** A track that loops plays to its end and goes on from its loop point, for ever, with the
  frames of the cooked track in exactly that order. The cooked form is laid out so that going back to the
  loop point decodes nothing that is not played.
- **Music plays at pitch 1 and has no place.** The music volume and the master volume govern it. Holding
  emitters does not touch it.
- **Data that has not arrived.** In Chrome a track reaches the mix in pieces. A stream that reaches the end of
  what it has been given renders silence, counts it, and goes on from the same frame when more arrives.

**The codec is QOA**, "Quite OK Audio": 3.2 bits a sample by its specification, which is about 2.3 MB a
minute for 48 kHz stereo. Its decoder is integer arithmetic of a few operations a sample with no allocation,
so it runs inside a pull on every target, including the worklet's module, and needs no decoding thread and
no buffer between a decoder and the mix. The specification fixes the stream and the samples a decoder gives,
and leaves an encoder free to choose among valid streams. So the engine's decoder is held to the reference
decoder's samples exactly, and the cook's encoder is held only to writing streams that decode that way. How
close its choices come to the reference encoder's is not a pass condition. The cooked form names its codec, so another can be added without
changing what a game writes. [Techniques and evidence](#techniques-and-evidence) gives the account.

### 9. Cooked sounds and the cook

Two product kinds join the five in `thrum-asset`: **a sound** and **a music track**. Both are
content-addressed products in the store, validated when opened and before any frame is exposed, as a sprite
sheet is.

| | A sound | A music track |
|---|---|---|
| Holds | 16-bit samples at 48 kHz, one or two channels | QOA frames at 48 kHz, one or two channels, the codec's name, and the loop point or none |
| Played | Whole, from memory | From its compressed form |
| Size, by arithmetic | 5.8 MB a minute mono, 11.5 MB stereo | About 1.2 MB a minute mono, 2.3 MB stereo |

**The cook** is in `thrum-cook`, which is authoring only: no shipping package links it, its decoders or its
resampler. It reads a `.wav` (8, 16 or 24-bit integer or 32-bit float PCM) or an `.ogg` holding Vorbis, of
one or two channels at any rate, resamples to 48 kHz with a band-limited resampler, and writes one of the two
products. Band-limited has a number here, because a wrong resampler keeps the length, the pitch and the level
right and adds a tone the source does not have: a tone cooked from another rate leaves its image or its alias
at least 40 dB below itself. By arithmetic for an 8 kHz tone at 22,050 Hz, taking the nearest sample leaves
the image at 14,050 Hz about 7 dB below the tone and drawing a line between two samples about 10 dB below. A
windowed-sinc resampler leaves it past 60. A source is a sound unless the cook is told it is music, with its loop point in seconds. The cook
is told by a small file beside the source, in the form the cooker's other small files have when WP2 is built.
How a game spells that file, the SDK's `Music(..)`, and the asset catalog's entry are `GAME-SDK`'s.

The cook refuses, naming the file and the cause, and the store keeps the last good product: more than two
channels; no frames; a WAV encoding other than those above; an `.ogg` that does not hold Vorbis; a file that
is cut short or does not parse; a loop point at or past the end. Levels are left as authored.

**This settles the SDK's open question**, "which sound formats a `MusicAsset` streams from": a music track is
cooked from either source format, `.wav` or `.ogg`, into the one streamed form. A `.wav` source loses less,
since an `.ogg` has already been through one lossy codec.

### 10. Sound outputs

**A sound output** connects one mix to one device and pulls from it. There are three, and a mix has at most
one.

**Apple, for the Mac and for iOS.** An output audio unit, asked for 48 kHz stereo float, whose render
callback pulls the mix. On the Mac it is the default output unit, which follows the system's default device
when headphones are plugged in or pulled out, and which converts to the hardware's rate. On iOS it is the
remote I/O unit, with the audio session set to the ambient category, so the game's sound plays beside other
apps' and obeys the silent switch. It is reached through `objc2-audio-toolbox` and, for the session,
`objc2-avf-audio`, of the `objc2` family the shells already use.

**Chrome.** An `AudioContext` asked for 48 kHz and one `AudioWorkletNode`. The worklet's processor holds **the
mix module**: the mix built from the same source as a WebAssembly module that imports nothing, with a plain
exported-function interface and no `wasm-bindgen`. The page hands the output that module's bytes when it
opens it. Each frame the handle posts one message of commands. A sound's bytes are transferred to the worklet
before the first command that uses them, in pieces small enough that copying one does not make a pull late,
and the port keeps their order. No `SharedArrayBuffer` is used, so a page needs no cross-origin isolation. The
probe ran this shape: while the page was held for 500 ms the worklet rendered 501 ms of frames, with no pull
skipped.

**Chrome's port is bounded by answers.** Each message posted to the worklet is a task queued on the audio
thread, so only the sender can bound the work waiting there. The worklet answers every message it takes,
with the number of commands the mix has applied, the number of pieces it has copied, the record and the
outcomes. The handle posts by those answers:

- never more commands than the mix's queue has room for, by the last answer;
- never a piece of sound data while an earlier piece is unanswered, so at most one piece is in flight;
- a frame's commands ahead of the next piece, so a play or a listener update waits behind the copy of one
  piece at most, however long the track;
- no message at all while a fixed number are unanswered.

When a frame's commands cannot all be posted, the full-queue rule of
[mechanism 2](#2-the-game-side-says-what-should-be-heard) holds as it does natively: a sound that happens is
dropped with the fate `queue full`, and a change of state is kept at its latest value and posted when there
is room. So a play made while a track of minutes is arriving reaches the mix or is counted as dropped, the
listener the mix ends with is the last one set, and pulls go on throughout. The record holds the most
commands, pieces and messages that were ever in flight.

**The buffer.** No device. A test, or the SDK's test harness, pulls frames when it chooses, so its time is
the frames it has pulled.

**Not pulling.** An output is in one state: starting, waiting for a press, running, suspended, or without a
device. Only `running` pulls.

- Chrome starts an `AudioContext` suspended until the page's first key or pointer press. The output listens
  for that press itself and resumes then. In the probe, commands posted before the press were delivered
  before the first pull, so music said on a title screen starts with the press.
- **Suspending is asked for, not observed.** The shell's lifecycle is `HOST`'s, as built. Whoever holds the
  output forwards "the app went to the background" and "the tab is hidden" to `suspend`, and the return to `resume`.
  The mix's clock is the frames pulled, so it stops while suspended. The iOS output also follows the audio
  session's interruptions, which are the device's and not the app's.
- **No device is not an error for a game.** Opening an output can fail with a typed error, and a game then
  runs with a mix nothing pulls.

**The output epoch.** An output counts its stretches of pulling. The output epoch goes up by one every time
pulling stops, by any route: a suspend, an interruption of the audio session, a device that went away, a
context the browser suspended, an output closed, or a new mix. It is the audio-device epoch C11 asks for,
and it is neither the tick nor the count of frames. The handle reads it to stamp each play. The output gives
a new epoch to the mix itself, outside the command queue and ahead of the first pull of that epoch, so no
play still on its way can get in front of it. A mix with no output is told that pulling stopped and started
by the same two calls, so a test drives an interruption with no device. At the first pull of a new epoch,
before any frame is rendered:

- every play stamped with an older epoch expires, with the fate `interrupted`, whether it was waiting in the
  queue or arrives later;
- every voice of a sound that happens is freed without another frame, and counted as cut by the
  interruption;
- emitters' voices and music streams are where they stopped. They take the handle's present state, and the
  whole output rises from silence over one ramp.

That is C11's rule for an interruption: one-shots already queued are consumed and never replay, and what is
continuous goes on from current state.

**Silenced, with a tap.** An output can be opened silenced, for proof. It does everything an audible output
does, up to and including writing the mix's frames into the device's own channel buffers in the device's
layout. **The output tap** then copies what those buffers hold into a store of a fixed size, made when the
output was opened, and only after that are the buffers zeroed. So the tap holds what the device was handed,
after the adapter's conversion, and nothing is heard. A silenced output differs from an audible one in those
two steps alone. The tap is read from the game's side after a run.

### 11. The record

The mix publishes a record that the handle reads without waiting: voices sounding and the most there have
been; emitters, how many are eligible and how many hold a voice; plays started; plays that never started, by
fate (the limit, a full pool, out of range, nothing pulling, interrupted, a full queue, a bad value); voices
displaced; voices cut by an interruption; outcomes lost; values clamped; frames that reached the clip's knee;
music streams, the current track and its position; frames a stream rendered silent for want of data; the
command queue's high-water mark; frames pulled; the output epoch; and the output's state. An output adds what
it knows and marks the rest absent: the device's rate and channels as read back, its reported latency, the
times at which a command was written and applied, and in Chrome the most commands, pieces and messages in
flight.

### 12. The reference script and the handle run

Two proofs go through every device. They show different things, and neither stands in for the other.

**The reference script** shows that the arithmetic is the same everywhere. It is one script of commands, each
with the frame at which it is applied, over sounds made by integer arithmetic and, from WP2, cooked fixtures.
It exercises mechanisms 3 to 8 as the mix sees them. Its rendered frames are folded into **a digest**, and
the digest is kept beside the crate's tests. The same script is run to a buffer natively, by the mix module
under Node, through the Mac's output, in the simulator and inside Chrome's worklet, and each must give the
kept digest.

A run through a device applies each command at its frame from inside the pull, by splitting the pull there,
which the second rule of mechanism 1 makes harmless. The output is opened
[silenced](#10-sound-outputs), and the digest is folded from the output tap: from what was written to the
device's channels, and not from the mix's own frames. An adapter that hands the device silence, or the
channels the wrong way round, gives another digest.

**The handle run** shows that a game's own calls reach the device. The script's commands never pass through
the handle, the queue or, in Chrome, the port, so the digest alone would pass an output that ignored every
call a game made. The handle run uses only [what AUDIO provides](#what-audio-provides), as a game does, and
applies nothing from inside the pull. It makes a mix, opens a silenced output, adds a stereo sound whose two
channels differ, and plays it once from the game's side with no place, at volume 1 and pitch 1. The tap must
then hold that sound's samples exactly, whole and once, with its first channel in the device's first channel
and zeros around it, and the play's outcome must be `started`. The same run with no play must tap only
zeros. Where in the tap the sound begins is not fixed, since the call is tied to no frame. WP3 runs it on the
Mac and in the simulator, and WP4 in Chrome.

Both are silent, so a check makes no noise on the owner's Mac, and a reviewer judges each from its record.

The digest changes only when the mix's arithmetic or the script is changed on purpose, such as a curve tuned
after the owner has listened. The log says so each time.

## What AUDIO provides

To `GAME-SDK`, and to any game on the engine's own API. This section is the definition
[the module list's interface row](../ARCHITECTURE.md#interfaces-track-7) points to. Names of Rust items are the
builder's; the calls and their meaning are fixed here.

| Call | Meaning |
|---|---|
| Make a mix, from a configuration | Fixed capacities. Gives the handle |
| Add a sound, add a music track, from a cooked product | Validates. A typed error if it does not, or if the table is full. Gives an identity |
| Replace or remove a sound | Voices already sounding finish on the data they started with. New plays use the new data. The old data is released on the game side, after the mix has let go of it |
| Play a sound | Volume 0 to 1, pitch 0.5 to 2, a place or none, a limit or none, an event identity, and a tick or none. One voice, [B1](#what-a-game-must-be-able-to-hear) and [B2](#what-a-game-must-be-able-to-hear). No result to wait for: its fate comes later, as an outcome |
| Read the outcomes | Drains the outcomes gathered since the last read: for each play its event identity, tick, epoch and fate ([mechanism 2](#2-the-game-side-says-what-should-be-heard)) |
| Start an emitter | A sound, volume, pitch, a place or none. Gives an identity |
| Set an emitter's volume or pitch; move it; hold it or let it go; end it | [Mechanism 5](#5-emitters) |
| Set the listener | Position, right, `near` |
| Say the music | A track or none, and a fade time in seconds |
| Set the volumes | Master, music, effects. Each is a linear gain from 0 to 1. How a slider maps to it is the SDK's |
| Update, once a frame | Settles which emitters hold a voice and sends the frame's commands |
| Read the record | The latest the mix published |
| Open an output for a mix, audible or silenced; suspend; resume; read its state and its epoch; read a silenced output's tap | [Mechanism 10](#10-sound-outputs). In Chrome, opening takes the mix module's bytes |
| Pull frames into a buffer; say that pulling stopped or started | For a mix with no output. Interleaved stereo float. Stopping and starting are the calls an output uses, and advance the epoch the same way |
| Cook a source | In `thrum-cook`: source bytes and "sound" or "music with this loop point" give product bytes or a named refusal |

What stays the consumer's: which events make a sound, the event identity and tick each play is given, and
that a predicted event is heard once, since AUDIO plays what it is told and never chooses between plays; what a pause
holds; the camera a listener is taken from and the `near` it is given; the radius of a sound played at a
place with none said; the catalog and the `Music(..)` file; forwarding the lifecycle; and the web build step
that puts the mix module beside the game's own, for which AUDIO gives one documented command.

`thrum-audio` is a crate of its own. A package that does not name it links none of it.

## Techniques and evidence

**One software mix on every target, against the platform's own mixer in the browser.** The earlier family
design proposed a Web Audio node graph for the browser and a native mixer elsewhere, with "the same cue,
scheduling, voice and bus semantics" in both. That is two mixers to keep equal by ear. One mix needs the
browser to run it off the page's thread, which the probe showed an audio worklet does with no cross-origin
isolation. It would change if a target could not run the mix module on its audio thread.

**Own outputs, against `cpal`.** `cpal` 0.18.2 is the usual Rust route to a device. Read from its source: its
default web backend refills buffers from a timer on the page's thread, so a long frame is a gap in the sound;
its worklet backend needs atomics, `-Zbuild-std` and cross-origin headers, which no Thrum page build has; and
on the Mac it sets the device's nominal sample rate to the one asked for, which changes it for every other
program. Its
coverage of Windows, Linux and Android is not wanted yet. The Apple output is a few hundred lines over one C
API that the probe drove. It would change when a desktop target beyond the Mac is commissioned: `cpal`
would then be the output there, behind the same pull.

**No audio engine crate.** `kira`, `rodio` and Bevy's audio stack bring a mixing graph, clocks and effects,
which the small form rules out, and all reach the browser through `cpal`. miniaudio, the earlier design's
candidate, reaches the browser only through Emscripten.

**QOA for music, against Vorbis and Opus.**

| | QOA | Vorbis | Opus |
|---|---|---|---|
| Size, 48 kHz stereo | About 2.3 MB a minute | About half of that or less at the usual settings | Smaller again |
| Decoder | Integer, a few operations a sample, no allocation, about 400 lines in its reference | `lewton`, pure Rust, MIT or Apache-2.0, widely used, last released in 2021; allocates as it decodes | Pure-Rust ports of libopus appeared this year (`opus-rs`, `ruopus`), all at 0.1 |
| Where it decodes | Inside the pull, on every target | A thread and a buffer natively; in Chrome, the worklet or the page | As Vorbis |
| A loop point | Exact, at any frame | Needs decoding before the point to settle | As Vorbis, with more to settle |

QOA is the one that keeps mechanism 1 whole: one path, no thread, nothing between a decoder and the mix that
can run dry. Godot took it up in 4.3 for imported sounds, and raylib plays it. It costs size, and it is lossy
in its own way: whether its noise can be heard in music is [the owner's to judge](#what-only-an-ear-can-judge).
It would change if the owner hears it, or if a published demo's download matters more than this plan
assumes. Opus would then be added as a second codec in the cooked form, with a decoder audited at that time,
and no game changes.

**Sounds as 16-bit samples.** A short sound costs little held decoded, and a voice then reads memory. A long
ambience belongs in a music track.

**`lewton`, `hound` and `rubato` in the cook only.** Each is permissively licensed and none reaches a
shipping package. `symphonia` decodes more formats and is MPL-2.0, which the project's licence rule does not
clearly admit, so it is not used.

**Equal bits across targets.** Cheap here, because a mixer is sums and products. What it forbids is small:
platform maths functions, where a square root or a rational curve does the same job.

## The system as it runs

### State and owners

| State | Owner | AUDIO's part |
|---|---|---|
| The world, and which events happened | `thrum-core`, through the game | None. AUDIO is told what should be heard |
| What should be heard | The handle | Plain data, set by the game each frame |
| What is sounding: voices, streams, positions, gains, the frames pulled | The mix | The only place sound is summed |
| The device stream and whether it is pulling | The sound output | One for a mix |
| Cooked products, the store, the manifest, live edits | `CONTENT-LOOP`, as built | Adds two product kinds with their validation and their cook |
| The app's lifecycle | `HOST`, as built | Offers `suspend` and `resume`. Observes nothing of the app itself |
| Capability status | `docs/design/maturity.json` | An entry, with implementation, correctness and device kept apart, and "heard" not judged until the owner's sitting in `GAME-SDK` WP19 |

### From a source file to the ear

A `.wav` under a game's assets is cooked to a sound product and published in the store's manifest. The game
resolves its name to the product's bytes, by mapping it natively or by fetching it in Chrome, as it does any
product. That loading is off the frame and is the game's to schedule. Adding the product to the mix validates
it and costs no decode and, natively, no copy. A play is then a command, a voice, frames in a pull, and the
device.

### A frame and a pull, walked

1. Frame functions call the handle. Each call updates the handle's own data and, where the mix must know,
   appends a command. No call waits.
2. The game calls update once. The handle runs [the emitters' pass](#5-emitters), starts and ends voices
   for those that gained or lost one, and sends the frame's commands that the mix has room for: nothing more
   natively, one message in Chrome.
3. On the audio thread, a pull first takes a new output epoch if one has begun, then frees the voices that
   rendered their last frame, then applies the commands that have arrived, in order.
4. For each sounding voice it reads the sound through the cubic at the voice's pitch, multiplies by the two
   gliding gains and adds into the stereo sum.
5. Each music stream decodes the samples this pull needs, takes its fade gain and is added.
6. The sum takes the master gain, passes the clip, and the record is published.
7. The frames go to the device: 512 at a time on this Mac, 128 in Chrome.

## Proof

No agent can hear. An agent proves what was mixed and what the device was handed, and
[an ear](#what-only-an-ear-can-judge) judges the rest.

| What | Proof | Where |
|---|---|---|
| B1 to B6, B10 and the record, as arithmetic | CPU tests on the buffer, each on the smallest script that shows the behaviour, comparing frames with a plain statement of what they must be | WP1, WP2. Guards in `./dev check` |
| B9 and C11's interruption: a play caught by an interruption expires under its identity, one-shots are cut, and what is continuous goes on | CPU tests on the buffer, through the calls an output uses to say pulling stopped and started | WP1 |
| B7, a pull's bounded work | A test that counts allocations in a pull with every mechanism at work; a test that watches replaced data being released on the game's side, since the test kit's counter does not count frees; and a reviewer's question on locks, frees and system calls | WP1 |
| B8, one mix | The reference script's digest: natively, and by the mix module under Node | WP1, WP2 |
| Cooked forms | Each source format cooked and compared with its source; a resampled tone's image and alias held 40 dB down; hostile products refused; the decoder exact against QOA's reference, and the cook's streams decoded by the reference | WP2 |
| Each output | [The handle run](#12-the-reference-script-and-the-handle-run): a sound played through the handle, found in the output tap. The reference script through the device, silenced, to the kept digest folded from the tap. The format read back from the device; suspend and resume, with a play caught by the suspend expired; in Chrome a held page and a first press | WP3, WP4 |
| Chrome's port stays bounded | A track of minutes transferred while a script plays sounds and moves the listener each frame: every play is in the tap or has an outcome that says why not, the listener applied last is the last one set, the most commands, pieces and messages in flight stay inside their bounds, and no pull is skipped | WP4 |
| B11 | The dependency closure of a package without sound, and of the audio crate without the cook | WP2 |
| Costs | [Tripwire records](#performance) | WP1 to WP4 |

Proof stops at the device's own buffers. That a speaker played them, at the right speed and on the right
side, and that they sound right, is the owner's to hear.

## What only an ear can judge

The owner judges how the engine sounds once, in `GAME-SDK` WP19, the sitting the module list gives it, with a
2D game and a 3D game written on the SDK. That sitting is the single acceptance point for sound. No item in
this module waits for the owner, none judges sound, and no reviewer is asked to.

What an ear can find in the engine's part, for whoever writes that sitting's list:

1. Sound comes out, on the Mac and in Chrome.
2. Left is left and right is right.
3. A known sound is at its own pitch and speed.
4. Nothing crackles, drops out or clicks: in steady play, as sounds start and stop, at forty hits a second,
   and while the window is dragged or a heavy frame runs.
5. A thing moving past and away sounds natural: its loudness with distance, and its side.
6. Music is clean: no hiss or grit that its source does not have.
7. A cross-fade has no dip, and a loop has no bump.
8. A crowded loud moment gets dense, not harsh.
9. A press is heard when it is made.

A "no" to any of these comes back to this module as a work package planned from the owner's words, and is
tuned or fixed here: the curves of mechanism 4, the steps of mechanism 7, the codec of mechanism 8, or a fault
in an output. None reopens the SDK.

**The listening room** is a diagnostic and not a gate. It is the runnable example of the engine's guide to
sound, built in WP4 on the engine's own API: a few real sounds, a thing that moves past, and two music tracks
to change between, audible on the Mac and in Chrome. It lets a fault heard in a game be told apart from a
fault in the game, and lets a tuned value be heard. Nobody is asked to listen to it, and nothing waits for
anyone who does.

## Risks

1. **The worklet's instance stalls when a large sound arrives.** Copying a piece into the module's memory, or
   growing that memory, happens on the audio thread between pulls, and many small pieces can queue ahead of
   a click. [The port's bounds](#10-sound-outputs) keep one piece in flight and put commands ahead of the
   next. Early sign: the time one message takes in the worklet against the time of one pull, and the wait of
   a play made while a track arrives. WP4 measures both and sets the piece size from the result.
2. **Chrome's first press never comes**, for a demo that plays itself. The output stays waiting and the game
   runs silent. That is the browser's rule and is the SDK's to show to a player.
3. **QOA's noise is audible in quiet music.** Early sign: only the owner's ear, in `GAME-SDK` WP19. The cooked form's codec name
   is the way out.
4. **The simulator's output does not behave as a phone's.** It proves function. The audio session's category
   and its interruptions are designed for and cannot be shown there.
5. **A builder reaches for a maths function in the mix**, and the targets drift apart by a bit. Early sign:
   the mix module's digest under Node, in WP1.
6. **The digest is taken for the sound.** A wrong curve gives a stable digest too, and so does an output that
   ignores a game. The CPU tests compare frames with what they must be, the digest only carries that to the
   other targets, and the handle run shows a game's calls arrive.
7. **Two modules edit the cooker at once.** `GAME-SDK` WP3 and WP4 change `thrum-cook` for sheets and
   content. WP2 here adds a kind beside theirs and integrates on what has landed.

## Performance

**Two contracts**, each a guard and not a tripwire, because C11 asks them: a pull allocates nothing, and no
call from a frame waits on a pull.

**The account.** A pull is linear in sounding voices times frames, a few multiplications and one cubic for
each. A music stream adds a fixed cost a frame. The handle's update is linear in emitters, arithmetic only.
Adding a sound decodes nothing.

**Tripwires**, recorded with the load on the machine and investigated while building goes on. None is a pass
condition beyond a missing measurement or a collapse:

- the time to render a minute of sound with the pool full and two music streams, natively and in the mix
  module, as a share of real time. A collapse is a mix slower than real time;
- the handle's update with 10,000 emitters of which 2,000 moved;
- in Chrome, the longest a message keeps the worklet from a pull, and the longest a play waits from its call
  to the pull that applies it while a track of minutes is arriving;
- the time from a play call to its first frame leaving the mix, with the device's reported latency beside it;
- cooked bytes for a second of each kind, and the cook's time for a second of source;
- about 25% over a comparable earlier record, the vision and scope document's figure.

## Rules for builders

These hold in every work package.

- **No agent judges how anything sounds.** An item that seems to need an ear is a question for the log, not a
  guess.
- **Nothing this module adds makes a sound** during `./dev check`, an item's check or a build. A run through a
  device is opened silenced, as [mechanism 10](#10-sound-outputs) says. Only the listening room, when a person
  runs it, is audible.
- **Equal bits, or stop.** If a target cannot be made to give the kept digest inside the operations
  [mechanism 1](#1-one-mix-pulled) allows, keep the first frame that differs with both values and end
  `BLOCKED`. Do not compare with a tolerance, shorten the script or drop a target.
- **The reference script's digest changes only on purpose.** Record the cause and the old and new digest in
  the log.
- **No time, byte count or allocation count stops the build** (the vision and scope document, decision 3),
  apart from the two contracts in [Performance](#performance). Record a row that trips a tripwire as
  measured, note what you found in the log, and keep building.
- **Record only runs of the commit being claimed**, with the load on the machine.
- **`thrum-asset` and `thrum-cook` are `CONTENT-LOOP`'s.** This module adds two product kinds and their cook
  there. Any other change to them is a proposal in the log and `BLOCKED needs_dependency`.
- **Chrome and the simulator start outside the Codex sandbox.** If a launch is refused, leave the item open
  and say so. Do not substitute Node or another target's result for an item whose target is Chrome or the
  simulator.

## Open questions

- **The defaults of the pool, its shares, the queue, the list of outcomes and the table of sounds.** WP1's
  builder chooses them and records the cost at them.
- **The starting values of the ramp, the glide, the knee, `near` and the emitters' margin.** WP1's builder
  chooses. They are tuned after the owner's sitting, if the owner asks.
- **The piece size for sound data in Chrome, and how many messages may be unanswered.** WP4 measures and sets
  them.
- **Whether QOA is good enough for music.** The owner's sitting in `GAME-SDK` WP19 settles it.
- **Whether headed Chrome's reported latency differs from the 32 ms headless Chrome reported.** WP4 records it.

## The work packages

Four, in the order they are built. [The plan](PLAN.md) names each and [the done-when file](DONE.json) holds the
items of the first two.

| Work package | Builds | Waits for |
|---|---|---|
| WP1 | Mechanisms 1 to 7 and 11 with no device, with outcomes and the output epoch driven in the buffer, the reference script, and the mix module built and run under Node | Nothing |
| WP2 | Mechanisms 8 and 9: the two product kinds, the cook, the codec, and music in the buffer | Nothing |
| WP3 | The Apple output, on the Mac and in the simulator, with the silenced opening, the output tap, the output epoch across a suspend, and the handle run on both | Nothing |
| WP4 | The Chrome output with its bounded port, the handle run in Chrome, the build command for the mix module, the engine's guide to sound with the listening room as its example, and the maturity entry. **This is the work package `GAME-SDK` WP18 waits for**, and it is the module's last | Nothing |

The owner's judgement of how it sounds is not a work package here. It is `GAME-SDK` WP19's
([what only an ear can judge](#what-only-an-ear-can-judge)).

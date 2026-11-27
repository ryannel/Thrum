# AUDIO: a game's sounds and music are heard on the Mac, in Chrome and on the iPhone simulator

27 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
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
| **B1 A sound that happens** plays once, from its first frame to its last | Any cooked sound, mono or stereo, from one frame long to longer than many pulls; volume 0 to 1; pitch 0.5 to 2; forty plays a second of one sound (the SDK's example) | One voice a sound, so a new play cuts the last. Rejected by two plays that overlap and must sum |
| **B2 A limit** keeps at most `n` voices of one sound | `n` from 1 up; forty plays a second under a limit of 4 | Counting plays, not voices that are still sounding. Rejected by plays spread over longer than the sound lasts |
| **B3 A sound a thing makes** loops while the thing exists, follows it, and is turned up, down, held and stopped | 10,000 things with a sound of which 1,000 to 2,000 move each frame, the representative scene of decision 1; a loop from one frame long; updates every frame | Restarting the loop on each update. Rejected by position continuity across an update. A loop with a gap or a repeated frame at its wrap. Rejected by an exact comparison across three wraps |
| **B4 A place** makes a sound quieter with distance and puts it on the side it stands | From the listener's own position to the sound's radius and beyond; the place, the listener and the listener's facing all moving between pulls; a place passing through the listener | Gains worked out once, when the sound starts. Rejected by a moving thing and a turning listener. Left and right swapped. Rejected by a place on the listener's left that must be louder in the first channel |
| **B5 Music** plays from its compressed form, loops from its loop point for ever, and cross-fades to the next track | Tracks of minutes; a loop point anywhere in the track; a change made part-way through a fade; the same track asked for again; no track | The whole track decoded when it is added. Rejected by what adding a long track allocates. A cut, or a fade to silence and back. Rejected by both tracks present at once with constant power. A restart when the same track is said again. Rejected by position continuity |
| **B6 Three volumes**, master, music and effects, each move what they govern | 0 to 1, changed at any moment | A volume applied when a voice starts. Rejected by a change while a loop sounds |
| **B7 Nothing waits.** A call from a frame returns at once, and a pull never blocks | A pull with every voice sounding; a page held for half a second; a sound added while others play | Mixing on the page's own thread in Chrome, which is what the usual Rust device crate does. Rejected by a held page during which the mix must keep its rate |
| **B8 The same sound everywhere, and with no device** | The Mac, Chrome and the simulator; a pull split into any sizes; no device at all | A second mixer for the browser built from Web Audio nodes. Rejected because Chrome must render [the reference script](#12-the-reference-script) to the same digest |
| **B9 Quiet when it should be** | Before a browser's first key or pointer press; a hidden tab; an app in the background; no device; a device that goes away | Queued sounds that all burst out when the output starts. Rejected by a count of sounds dropped while nothing was pulling |
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

**What a call does when it cannot be honoured.** Sound is presentation, so nothing is refused with an error
after setup ([the engineering rules](../../../.agents/skills/thrum-engineering/SKILL.md#engine-rules)):

| Case | What happens |
|---|---|
| A volume or pitch outside its range | Clamped, and counted |
| A value that is not finite | The call is dropped, and counted. A debug build asserts |
| The command queue is full | A sound that happens is dropped and counted. A change of state is never lost: the handle keeps the latest value and sends it when there is room |
| Nothing is pulling ([B9](#what-a-game-must-be-able-to-hear)) | A sound that happens is dropped and counted. State is kept and is what sounds when pulling starts |

Setup can fail, and returns a typed error with the state unchanged: adding a product that does not validate,
adding more sounds than the mix was made for, opening an output.

**The handle can rebuild a mix.** Since it holds everything that should be heard, an output that starts late,
or again after its device went away, is sent the present state: emitters and music sound, and no sound that
happened in the meantime is replayed. That is C11's rule for an interruption.

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
[the listening](#what-only-an-ear-can-judge). The properties in the three points are not.

Gains are recomputed when a command moves a place or the listener, never once a pull, and each voice's two
channel gains then **glide** to the new values by a fixed step a frame ([mechanism 7](#7-glides-ramps-and-the-clip)).
Audibility is by radius alone. Whether a thing is on screen has no part in it, which C11 asks.

### 5. Emitters

**An emitter** is a looping sound that a thing makes: a sound, a place or none, a volume and a pitch. It
lasts until it is ended. The handle holds every emitter as plain data, and only an **audible** emitter holds a
voice in the mix. An emitter with a place is audible inside its radius. One with no place always is.

Once a frame the handle passes over its emitters. One that has come inside its radius is given a voice,
starting at the loop's first frame, where its gain is still zero. One that has left gives its voice up, at
zero gain. When more are audible than the emitters' share of the pool ([mechanism 6](#6-the-limit-and-the-pool)),
the loudest hold the voices, and a voice changes hands only when the newcomer is louder by a margin, so two
equal emitters do not swap each frame. The pass is arithmetic over plain arrays, linear in the emitters, with
no command for an emitter whose state the mix need not know.

An emitter's voice wraps from its last frame to its first with no gap and no repeated frame. Setting its
volume or pitch, or moving it, glides and never restarts it. **Holding** it ramps it to silence and freezes
its position, and letting go ramps it back from that frame: a paused world's things wait this way. Ending it
ramps it out.

### 6. The limit and the pool

**A limit** belongs to a play: at most `n` voices of that sound at once. The mix counts the voices of the
sound that are still sounding when the command is applied, and drops the play if there are `n`. It is counted
in the mix because only the mix knows when a voice ended.

**The pool** is the fixed number of voices a mix was made with, in two shares: one for emitters, which the
handle gives out by loudness as above, and one for sounds that happen. When that share is full, a new play
takes the place of its quietest voice if the new one would be louder, and the displaced voice ramps out. If it
would not be louder, the new play is dropped. Both are counted.

The sizes of the pool, of each share, of the command queue and of the table of sounds are fields of the mix's
configuration. Their defaults are the builder's to choose in WP1 and are recorded with the cost measured at
them. None is a limit this design states.

### 7. Glides, ramps and the clip

Nothing in the mix changes a gain in one step.

- **A glide** moves a gain towards its target by at most a fixed step each frame. Volumes, pan, distance and
  the three volumes all glide. A sound that happens starts at its gains, so its attack is its own. An
  emitter's voice starts at zero and glides up.
- **A ramp** takes a voice to silence over a fixed number of frames before it is stopped, held, displaced or
  cut, and back up when a held one is let go.
- **The clip** is the last thing a frame passes through. Below a knee a sample is unchanged, bit for bit.
  Above it the sample bends towards 1 and never reaches it, by a rational curve that is continuous and
  rising. Frames that reached the knee are counted, so a game that runs hot can see that it does.

Steps and lengths are counted in frames, which is what keeps the second rule of mechanism 1. Their starting
values, and the knee's, are the builder's and are tuned by ear at the listening.

### 8. Music

A music track plays from its compressed form: what is added to the mix is the cooked bytes, and a stream
decodes only the samples each pull asks for. Nothing holds a track decoded.

- **Two streams.** One track is current. On a change the current stream becomes the outgoing one and fades
  out from its present gain while the new one fades in, over the time the call gives, by square-root curves so
  the power stays constant. A stream that was already outgoing is ramped out at once, so no more than two
  streams fade at a time.
- **Saying the current track again does nothing.** Saying no track fades the current one out.
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
no buffer between a decoder and the mix. The cooked form names its codec, so another can be added without
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
products. A source is a sound unless the cook is told it is music, with its loop point in seconds. The cook
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
and the port keeps their order. The worklet posts the record back. No `SharedArrayBuffer` is used, so a page
needs no cross-origin isolation. The probe ran this shape: while the page was held for 500 ms the worklet
rendered 501 ms of frames, with no pull skipped.

**The buffer.** No device. A test, or the SDK's test harness, pulls frames when it chooses, so its time is
the frames it has pulled.

**Not pulling.** An output is in one state: starting, waiting for a press, running, suspended, or without a
device. Only `running` pulls.

- Chrome starts an `AudioContext` suspended until the page's first key or pointer press. The output listens
  for that press itself and resumes then. In the probe, commands posted before the press were delivered
  before the first pull, so music said on a title screen starts with the press.
- **Suspending is asked for, not observed.** The shell's lifecycle is `HOST`'s, as built. Whoever holds the
  output forwards "the app went to the background" and "the tab is hidden" to `suspend`, and the return to `resume`.
  The mix's clock stops while suspended. The iOS output also follows the audio session's interruptions, which
  are the device's and not the app's.
- **No device is not an error for a game.** Opening an output can fail with a typed error, and a game then
  runs with a mix nothing pulls.

### 11. The record

The mix publishes a record that the handle reads without waiting: voices sounding and the most there have
been; emitters and how many are audible; plays started; plays dropped, by cause (the limit, a full pool, out
of range, nothing pulling, a full queue, a bad value); voices displaced; values clamped; frames that reached
the clip's knee; music streams, the current track and its position; frames a stream rendered silent for want
of data; the command queue's high-water mark; frames pulled; and the output's state. An output adds what it
knows and marks the rest absent: the device's rate and channels as read back, its reported latency, and the
times at which a command was written and applied.

### 12. The reference script

One script of commands, each with the frame at which it is applied, over sounds made by integer arithmetic
and, from WP2, cooked fixtures. It exercises mechanisms 3 to 8 as the mix sees them. Its rendered frames are folded into
**a digest**, and the digest is kept beside the crate's tests. The same script is run to a buffer natively, by
the mix module under Node, through the Mac's output, in the simulator and inside Chrome's worklet, and each
must give the kept digest.

A run through a device applies each command at its frame from inside the pull, by splitting the pull
there, which the second rule of mechanism 1 makes harmless, and folds what it pulled before the frames go to
the device. Those runs are **silenced**: the output writes zeros to the device after
the mix has rendered, so a check makes no noise on the owner's Mac and still proves the device pulled the
right frames at the rate asked.

The digest changes only when the mix's arithmetic or the script is changed on purpose, such as a curve tuned
after the listening. The log says so each time.

## What AUDIO provides

To `GAME-SDK`, and to any game on the engine's own API. This section is the definition
[the module list's interface row](../ARCHITECTURE.md#interfaces-track-7) points to. Names of Rust items are the
builder's; the calls and their meaning are fixed here.

| Call | Meaning |
|---|---|
| Make a mix, from a configuration | Fixed capacities. Gives the handle |
| Add a sound, add a music track, from a cooked product | Validates. A typed error if it does not, or if the table is full. Gives an identity |
| Replace or remove a sound | Voices already sounding finish on the data they started with. New plays use the new data. The old data is released on the game side, after the mix has let go of it |
| Play a sound | Volume 0 to 1, pitch 0.5 to 2, a place or none, a limit or none. One voice, [B1](#what-a-game-must-be-able-to-hear) and [B2](#what-a-game-must-be-able-to-hear). No result to wait for |
| Start an emitter | A sound, volume, pitch, a place or none. Gives an identity |
| Set an emitter's volume or pitch; move it; hold it or let it go; end it | [Mechanism 5](#5-emitters) |
| Set the listener | Position, right, `near` |
| Say the music | A track or none, and a fade time in seconds |
| Set the volumes | Master, music, effects. Each is a linear gain from 0 to 1. How a slider maps to it is the SDK's |
| Update, once a frame | Settles which emitters are audible and sends the frame's commands |
| Read the record | The latest the mix published |
| Open an output for a mix; suspend; resume; read its state | [Mechanism 10](#10-sound-outputs). In Chrome, opening takes the mix module's bytes |
| Pull frames into a buffer | For a mix with no output. Interleaved stereo float |
| Cook a source | In `thrum-cook`: source bytes and "sound" or "music with this loop point" give product bytes or a named refusal |

What stays the consumer's: which events make a sound and that a predicted event is heard once; what a pause
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
| Capability status | `docs/design/maturity.json` | An entry, with implementation, correctness and device kept apart, and "heard" not judged until the listening |

### From a source file to the ear

A `.wav` under a game's assets is cooked to a sound product and published in the store's manifest. The game
resolves its name to the product's bytes, by mapping it natively or by fetching it in Chrome, as it does any
product. That loading is off the frame and is the game's to schedule. Adding the product to the mix validates
it and costs no decode and, natively, no copy. A play is then a command, a voice, frames in a pull, and the
device.

### A frame and a pull, walked

1. Frame functions call the handle. Each call updates the handle's own data and, where the mix must know,
   appends a command. No call waits.
2. The game calls update once. The handle passes over its emitters, starts and ends voices for those that
   crossed their radius, and sends the frame's commands: nothing more natively, one message in Chrome.
3. On the audio thread, a pull applies the commands that have arrived and frees the voices that ended.
4. For each sounding voice it reads the sound through the cubic at the voice's pitch, multiplies by the two
   gliding gains and adds into the stereo sum.
5. Each music stream decodes the samples this pull needs, takes its fade gain and is added.
6. The sum takes the master gain, passes the clip, and the record is published.
7. The frames go to the device: 512 at a time on this Mac, 128 in Chrome.

## Proof

No agent can hear. An agent proves what was mixed and that the device took it, and
[an ear](#what-only-an-ear-can-judge) judges the rest.

| What | Proof | Where |
|---|---|---|
| B1 to B6, B10 and the record, as arithmetic | CPU tests on the buffer, each on the smallest script that shows the behaviour, comparing frames with a plain statement of what they must be | WP1, WP2. Guards in `./dev check` |
| B7, a pull's bounded work | A test that counts allocations in a pull with every mechanism at work, and a reviewer's question on locks and system calls | WP1 |
| B8, one mix | The reference script's digest: natively, and by the mix module under Node | WP1, WP2 |
| Cooked forms | Each source format cooked and compared with its source; hostile products refused; the codec exact against QOA's reference | WP2 |
| Each output | The reference script through the device, silenced, to the kept digest; the format read back from the device; suspend and resume; in Chrome a held page and a first press | WP3, WP4 |
| B11 | The dependency closure of a package without sound, and of the audio crate without the cook | WP2 |
| Costs | [Tripwire records](#performance) | WP1 to WP4 |

Proof stops where the frames leave the engine. That a speaker played them, at the right speed and on the
right side, and that they sound right, is the listening's.

## What only an ear can judge

These wait for the owner, in one sitting, in WP5. No other item in this module judges sound, and no reviewer
is asked to. The owner listens to **the listening room**, a small program built for it with real sounds and
a real music track, on the Mac's own speakers or headphones and in Chrome, with this list:

1. Sound comes out, on both.
2. Left is left and right is right.
3. A known sound is at its own pitch and speed.
4. Nothing crackles, drops out or clicks: in steady play, as sounds start and stop, at forty hits a second,
   and while the window is dragged or a heavy frame runs.
5. A thing moving past and away sounds natural: its loudness with distance, and its side.
6. Music is clean: no hiss or grit that its source does not have.
7. A cross-fade has no dip, and a loop has no bump.
8. A crowded loud moment gets dense, not harsh.
9. A press is heard when it is made.

A "no" is tuned or fixed inside this module: the curves of mechanism 4, the steps of mechanism 7, the codec of
mechanism 8, or a fault in an output. None reopens the SDK. The lead may hold WP5 for the sitting `GAME-SDK`
WP19 already plans.

## Risks

1. **The worklet's instance stalls when a large sound arrives.** Copying a piece into the module's memory, or
   growing that memory, happens on the audio thread between pulls. Early sign: the time one message takes in
   the worklet against the time of one pull. WP4 measures it and sets the piece size from the result.
2. **Chrome's first press never comes**, for a demo that plays itself. The output stays waiting and the game
   runs silent. That is the browser's rule and is the SDK's to show to a player.
3. **QOA's noise is audible in quiet music.** Early sign: only the owner's ear. The cooked form's codec name
   is the way out.
4. **The simulator's output does not behave as a phone's.** It proves function. The audio session's category
   and its interruptions are designed for and cannot be shown there.
5. **A builder reaches for a maths function in the mix**, and the targets drift apart by a bit. Early sign:
   the mix module's digest under Node, in WP1.
6. **The digest is taken for the sound.** A wrong curve gives a stable digest too. The CPU tests compare
   frames with what they must be, and the digest only carries that to the other targets.
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
- in Chrome, the longest a message keeps the worklet from a pull;
- the time from a play call to its first frame leaving the mix, with the device's reported latency beside it;
- cooked bytes for a second of each kind, and the cook's time for a second of source;
- about 25% over a comparable earlier record, the vision and scope document's figure.

## Rules for builders

These hold in every work package.

- **No agent judges how anything sounds.** An item that seems to need an ear is a question for the log, not a
  guess.
- **Nothing this module adds makes a sound** during `./dev check`, an item's check or a build. A run through a
  device is silenced as [mechanism 12](#12-the-reference-script) says. Only the listening room, run by the
  owner, is audible.
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

- **The defaults of the pool, its shares, the queue and the table of sounds.** WP1's builder chooses them and
  records the cost at them.
- **The starting values of the ramp, the glide, the knee and `near`.** WP1's builder chooses. The listening
  tunes them.
- **The piece size for sound data in Chrome.** WP4 measures and sets it.
- **Whether QOA is good enough for music.** The listening settles it.
- **Whether headed Chrome's reported latency differs from the 32 ms headless Chrome reported.** WP4 records it.

## The work packages

Five, in the order they are built. [The plan](PLAN.md) names each and [the done-when file](DONE.json) holds the
items of the first two.

| Work package | Builds | Waits for |
|---|---|---|
| WP1 | Mechanisms 1 to 7 and 11 with no device, the reference script, and the mix module built and run under Node | Nothing |
| WP2 | Mechanisms 8 and 9: the two product kinds, the cook, the codec, and music in the buffer | Nothing |
| WP3 | The Apple output, on the Mac and in the simulator | Nothing |
| WP4 | The Chrome output, the build command for the mix module, the engine's guide to sound, and the maturity entry. **This is the work package `GAME-SDK` WP18 waits for** | Nothing |
| WP5 | The listening room, and the owner's one sitting | Nothing waits for it |

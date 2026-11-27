# AUDIO plan

A game's sounds and music are heard on the Mac, in Chrome and on the iPhone simulator, mixed in one place and
never stalling a frame. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria ·
[log](LOG.md) · [module list](../ARCHITECTURE.md#track-7-the-game-sdk)

Read [Mechanisms](DESIGN.md#mechanisms), [What AUDIO provides](DESIGN.md#what-audio-provides) and
[Rules for builders](DESIGN.md#rules-for-builders) before starting any work package. [The planning probe](../../../.orchestrate/evidence/AUDIO/planning-probe.md) says
what was measured before the design was written. Its code was thrown away on purpose, so build from the design.

## Open for the user

- Assumption: "audio in its small form" is sounds that happen, with a limit on how many of one play at once; looping sounds heard from a place; music that streams, loops and cross-fades; and three volumes. It is stereo, with no effects and no mixing graph.
- Assumption: you listen to the engine's own sound once, in WP5, with a small listening program on the Mac and in Chrome, and it can be the same sitting as `GAME-SDK` WP19. It is the only item in this module that waits for you, and nothing else waits for it. Agents prove what was mixed and that the operating system took it, with the output silenced so builds make no noise on your Mac. They cannot prove what a speaker played.
- Assumption: music is kept in a light compressed form, QOA, at about 2.3 MB a minute in stereo, because its small decoder runs the same way on every target. It is lossy. If you hear hiss or grit in music at the listening, the codec is changed inside this module and no game changes.
- Assumption: the targets are the Mac, Chrome and the iPhone simulator, which proves function only. A physical iPhone, Safari, Firefox, Windows and Linux are designed for and not run.
- Assumption: in Chrome a game is silent until the player's first key press or click, because browsers start sound that way. Showing a player that is the game SDK's.

## WP1 A program plays sounds into a buffer with no device, placed, pitched, looped and limited, and a test reads exactly what would be heard

Criteria: WP1.1–WP1.10 in the done-when file.

Nothing exists yet: `thrum-audio` is a new crate, registered in `.config/architecture.toml` with each outside
dependency at an exact version, and its integration tests are one target, `it`. It builds on Linux and Windows
too, where `scripts/check_cross_target.sh` type-checks tests, so nothing in this work package may need a
device. `thrum-test-support` has the counting allocator WP1.7 uses. The mix module of WP1.8 is the same
source built a second way; a mix core that is `no_std` makes "imports nothing" hold by construction, and
whether that core is a crate of its own is yours to choose. `scripts/audio.py` is new, and its `module`
command is the documented build a web game's build step will call, so keep what it builds stable.

The order matters in two places. Write the reference script and keep its digest once WP1.1 to WP1.6 hold,
then make the module reproduce it. Choose the defaults WP1.9 records before recording, and record at the
commit being claimed.

## WP2 A .wav or .ogg is cooked into a sound or a music track, and music plays from its compressed form, loops from its loop point and cross-fades

Criteria: WP2.1–WP2.8 in the done-when file.

WP1 left the mix, the handle, the reference script with its digest, and `scripts/audio.py`. The cooked
formats follow `SheetProduct` in `crates/thrum-asset/src/sheet.rs` and the cook follows
`crates/thrum-cook/src/sheet.rs`, with a new arm in the cooker's `classify`. `CONTENT-LOOP` owns the cooker,
the store and the manifest: add the two kinds beside the five and change nothing else of theirs. `GAME-SDK`
WP3 and WP4 edit the cooker for sheets and content at about the same time, so integrate on what has landed
and keep their tests passing. The small file that marks a source as music takes the form the cooker's other
small files have when you build this: `sheet.json` today, and the SDK's form if WP3 has landed one. The
SDK's own spelling, `Music(..)`, and the asset catalog are `GAME-SDK` WP18's.

`ffmpeg` on this Mac encodes Vorbis and Opus, which is enough to make the `.ogg` fixtures. Keep fixtures to a
second or less, track them beside the tests, and list each in `docs/provenance/AUDIO.md` with the command
that made it. The QOA reference is a single C header; pin its revision, confirm its licence is permissive
before using it, and use it to write the fixture streams and the kept digest of WP2.3. If its licence is not
permissive, stop and end `BLOCKED`. The digest of the reference script changes here on purpose, and the log
says so.

## WP3 A game's sound comes out of the Mac's output and the iPhone simulator's, and stops and starts with the app

## WP4 A game's sound comes out of a Chrome tab, keeps playing through a stalled frame, and the same script gives the same sound on all three targets

## WP5 The owner hears the listening room on the Mac and in Chrome, and says what is wrong

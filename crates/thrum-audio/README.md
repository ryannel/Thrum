# Thrum audio

`thrum-audio` mixes 48 kHz stereo PCM with no device required. Create a connection with
`Handle::new(Config::default())`, retain the handle on the game side and pass the mix to the
pulling side. Start the mix, register interleaved PCM, then call `Handle::play` or manage
emitters with the handle. Call `Handle::update` once per game frame to select emitter voices
and deliver pending state. Pull interleaved stereo samples using `Mix::pull`. Read play outcomes
by the returned play number. A pull while stopped writes silence and leaves the frame count
and loop positions unchanged.

From the repository root, build the WebAssembly module and verify its reference script under
Node with this command.

```sh
python3 scripts/audio.py module
```

It requires the workspace Rust toolchain with `wasm32-unknown-unknown` installed, Python 3
and Node. The command prints the artifact path, SHA-256, import list and digest for each pull size.
The artifact is `target/wasm32-unknown-unknown/release/thrum_audio.wasm`.

The module exports `audio_reference_prepare`, `audio_reference_pull` and
`audio_output_samples` for the frame-addressed proof. A pull returns the number of frames
written. Read twice that many consecutive float32 samples from the byte offset returned by
`audio_output_samples` before the next pull. `python3 scripts/audio.py costs` records the
same full-pool workload natively and under Node, along with a 10,000-emitter native update.
Both commands run against memory buffers in silence.

# Mac held-drawable recovery witness (WIP)

The production runtime base is `43e1d488b417f8dcf3677d6b1d80e98981d48d18`.
This diagnostic is for the Mac M1 acceptance rows. The runtime source on
main builds without these hooks. The offscreen Metal test passed with the
autonomous world paused at tick zero. The three live window rows and their
independent motion review are pending.

## Build in scratch

Copy the files in this directory into `tmp/g3b-mac-qualification/` in the
main checkout. Set `out` to that absolute path. For a fresh build, run the
following from the repository root with Rust 1.99.1, Xcode and a populated
Cargo cache.

```sh
mkdir -p "$out/live-diagnostic/source"
git archive 43e1d488 | tar -x -C "$out/live-diagnostic/source"
# Set cached_wgpu to the Cargo registry source directory for wgpu 30.0.1.
cp -R "$cached_wgpu" "$out/live-diagnostic/wgpu"
src="$out/live-diagnostic/source"
cp "$out/m1_loss_witness.rs" "$src/crates/thrum-app/src/app/"
python3 "$out/prepare_live_loss.py" "$src" "$out/live-diagnostic/wgpu"
(cd "$src" && RUSTC_WRAPPER= rustup run 1.99.1 cargo build --offline --release -p v1-game)
xcrun swiftc -module-cache-path "$out/swift-cache" -parse-as-library \
  "$out/record_owned_window.swift" -o "$out/record_owned_window"
python3 "$out/check_auditor_fixture.py"
```

The patched dependency only prints the native queue entry IDs and calls.
The WIP contains no vendored dependency source. The checked binary and its
inputs are identified by `qualified-build.sha256`.

For the offscreen Metal preflight, append `cooked_readback_preflight.rs`
to `examples/v1-game/src/host/tests/recovery_gpu.rs` in the archived
source. Build `cargo test --release --offline -p v1-game --lib --no-run`.
Run the emitted binary with
`cooked_noon_readback_matches_after_device_destroy --nocapture`,
`THRUM_SCENE=valley`, a cooked valley `THRUM_ASSET_STORE`, and a fresh
`THRUM_M1_READBACK_DIR`. Set `THRUM_M1_WITNESS` to enable queue logging.
The test uses the real GPU and no window.

## Approved owner-present run

Build the checked-in valley store with the normal cooker. The runner
defaults to the original checkout and store. On another checkout point
`worktree` in the runner at the checkout with the pinned noon preset
and pass the recreated store with `--store`. The original manifest root
starts `70d2baf8`. The three rows use the same binary, params and store.
The diagnostic pauses the initial world before starting its autonomous
lane. The app still sends and awaits the preset Pause acknowledgement.

Use `--preflight-only` to query the console without launching. Once the
owner grants the screen slot, use `--owner-window` for each of
`control`, `paused`, and `running`, giving each row a fresh output path
inside one slot directory under `$out`. Run the recorder's
`--preflight` first. The recorder filters by game PID and window.
The runner injects P before arming the running row and E after
recovery in both loss rows. The app logs each action and sends both
edges through its normal key handler. Stop on missing Screen Recording
permission or an unplanned dialog. Cap the slot at
eight minutes. Run `audit_live_witness.py` on the slot directory and
watch the retained recordings at 1× against the approved or proposed
noon reference. Offscreen equality and present calls alone leave the
live acceptance open.

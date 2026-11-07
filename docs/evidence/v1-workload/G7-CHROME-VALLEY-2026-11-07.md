# G7 cooked valley in Chrome — 7 November 2026

Chrome now draws the authored cooked valley, not only the diagnostic scene. This is the first slice of
[the build plan](../../BUILD-PLAN.md)'s G7 item: native and web share the scene. The page fetches one
publication from the G5 dev server and draws it with the same placements, camera, sky, shadows and exposure as
native. Chrome fetches Portable products. The native store can select compressed variants.

![Chrome 156 WebGPU on the M3 Max. Sunrise at the authored framing, sunset, twilight at the authored framing,
and noon looking up](g3s-first-light/chrome-valley.png)

## Change

- **Product source.** `thrum-asset-gpu` takes a `CookedSource`. It is either a store path, followed by the native
  loader thread as before, or one `RemoteGeneration` fetched by G5's `DevClient`.
- **Browser preparation.** The browser has no loader thread. A fetched generation is prepared on the calling
  thread when the contribution starts, then posted to the same mailbox. Staging, the per-frame copy budget,
  swap, culling, draw and Sun casting are unchanged. A new device generation prepares it again.
- **Game.** `fetch_cooked_valley(origin)` and `BrowserGame::attach_cooked_valley` load and attach the embedded
  showcase recipe. `index.html?assets=<origin>` does both before the first frame. The showcase recipe and
  `serde` now build for wasm32. The environment product stays native.
- **Check.** `scripts/check_v1_browser.mjs --case valley` cooks `examples/v1-game/assets/showcase` into a fresh
  store with `thrum-cook watch --serve`. It then runs the sky-reflections procedure against it (seven held
  phases and 24 camera and Sun samples) and requires the cooked generation to be drawn with no loader errors.
  This is the dev server's first Chrome run with an `Origin` header, so its CORS path is now exercised.

## Checks

- **Equivalence.** A Metal test fetches a generation from `DevServer` over loopback HTTP. It draws that
  generation at the same world time as the mapped store with Portable selected. No pixel differs by more than
  12 codes in any RGB channel. The fetched path uses the device's default preferences with BC compression enabled.
- **Handback recheck.** The Metal fetched-generation and edit cases passed. Chrome's `valley` and
  `valley-recovery` passed with Wasm SHA-256
  `172060dec8af3d7c432e5597ff81bb75e39e35f84f47905374b8adb5e81f3427`. The edit uploaded 5,376 bytes
  after the initial 1,523,712-byte upload. Two real device replacements each restaged the valley.
  The first valley run passed its scene assertions and timed out during Chrome shutdown. The repeat exited cleanly.
- **Chrome.** The `valley` case passed: generation 1 drawn after one swap, 1.52 MB uploaded, no loader errors.
  Wasm SHA-256 `81dba38fab05cf39e26f58e94eeb63dde7545d9be05f1c0c720f96a6c720df12`. The existing
  `sky-reflections` case passed with the same Wasm.
- **Repository.** `./dev check` passed.

## Live edits

![Chrome, paused world at noon. Left: generation 1. Right: after retinting the shared base-colour texture
in the watched source](g3s-first-light/chrome-live-edit.png)

- **Feed.** `CookedSource::Remote` now takes a `RemoteFeed`. `RemoteFeed::publish` hands in a newer fetched
  generation, and the contribution prepares it on the render thread at its next upload. It reuses every product
  the drawn set already holds and swaps in the new set through the same staging. A replacement device prepares
  the newest generation.
- **Page.** It long-polls the dev server with `CookedValley::next_publication`. Overlapping polls are refused,
  and failures are logged and retried after one second.
- **Chrome check.** The `valley` case now watches a copy of the showcase source. After its captures it retints
  `white.png` and requires generation 2 to be drawn with no errors, after exactly one more swap, uploading
  less than the first set. Result: 5,376 bytes uploaded against 1,523,712, with the watcher publishing 43 ms
  after the edit. The swap happened with the world paused.
- **Metal test.** A second publication through the feed swaps in with a smaller upload than the first, the
  dev client reuses the unchanged mesh, and the edit is visible.

## Device recovery

![Chrome after two GPU device replacements with the cooked valley attached](g3s-first-light/chrome-valley-recovered.png)

`check_v1_browser.mjs --case valley-recovery` runs the existing device-recovery procedure with the valley
attached. It destroys the actual `GPUDevice` twice and restores it with the real button, on the same world.
Each replacement must prepare the fetched generation again. Result: generation 1 still drawn, swaps 1 → 3, each
restage a full 1,523,712-byte upload, frames submitted 2 → 107, and no loader errors. `cooked_status` now
reports submitted frames. With Wasm SHA-256
`150993227878c5f7cd05d0f2af06a7ea1d2d41639f918ed9cd5f6be8992e5ae4`, the `valley`, `sky-reflections` and `device-recovery`
cases also passed, as did `./dev check`.

## Independent review

A fresh-context reviewer saw only Chrome and native stills and a 24-sample Chrome motion tile. It found:
- **Parity.** Chrome matches native at the authored framing: terrain, sphere, rock, sky colour and haze. There
  is no missing geometry and no corrupt region.
- **Lit floor turning grey in one step (samples 4–5).** The samples are 17.67 h and 17.73 h, 3.4 minutes of
  solar time apart. Native half-minute captures show the ridge shadow crossing the floor between 17.67 h and
  17.70 h, so this is sparse sampling of a real crossing, not a Chrome defect.
- **Saturated orange glow and a hard step at the near-floor silhouette (Sun-facing presets).** Native
  `sunset-sun` at the same time and direction shows both identically. They were shared look items: in-scatter
  over shadowed distant terrain. Fixed by [shadowed aerial in-scatter](G3S-SHADOWED-AERIAL-2026-11-07.md).
- **Noon preset.** It pitches up to the sky. The reviewer read it as dull and noted faint column banding and
  ridge fringes. The banding was 8-bit contouring, now dithered
  ([evidence](G3S-DISPLAY-DITHER-2026-11-07.md)). The fringes are a smooth crest fall-off, not a defect.

## Open

- **Look items.** Those above, shared with native.
- **Edit latency.** Chrome edit-to-visible latency is not measured beyond the watcher's publish time.
- **Measurements.** No Chrome frame-time or startup measurement.

# G7 cooked valley in Chrome — 7 November 2026

Chrome now draws the authored cooked valley, not only the diagnostic scene. This is the first slice of
[the build plan](../../BUILD-PLAN.md)'s G7 item: native and web share the scene. The page fetches one
publication from the G5 dev server and draws it with the same placements, camera, sky, shadows and exposure as
native.

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
  generation at the same world time as the mapped store, and the two images match pixel for pixel.
- **Chrome.** The `valley` case passed: generation 1 drawn after one swap, 1.52 MB uploaded, no loader errors.
  Wasm SHA-256 `01a18908ae606dda49773030a2dce708af36cde4f89b43c4dae4abd54047a5d1`. The existing
  `sky-reflections` case passed with the same Wasm.
- **Repository.** `./dev check` passed.

## Independent review

A fresh-context reviewer saw only Chrome and native stills and a 24-sample Chrome motion tile. It found:
- **Parity.** Chrome matches native at the authored framing: terrain, sphere, rock, sky colour and haze. There
  is no missing geometry and no corrupt region.
- **Lit floor turning grey in one step (samples 4–5).** The samples are 17.67 h and 17.73 h, 3.4 minutes of
  solar time apart. Native half-minute captures show the ridge shadow crossing the floor between 17.67 h and
  17.70 h, so this is sparse sampling of a real crossing, not a Chrome defect.
- **Saturated orange glow and a hard step at the near-floor silhouette (Sun-facing presets).** Native
  `sunset-sun` at the same time and direction shows both identically. They are shared look items: in-scatter
  over shadowed distant terrain, against a near floor in shadow.
- **Noon preset.** It pitches up to the sky. The reviewer read it as dull and noted faint column banding and
  ridge fringes. Recorded, not yet scored.

## Open

- **Look items.** Those above, shared with native.
- **Live edits.** Chrome loads one publication. Long-polling for later publications is the next step.
- **Device recovery.** Recovery with the valley attached is not yet a browser case. The native restage path
  it shares is tested.
- **Shared controls.** The demo panel still waits for G6's typed controls.
- **Measurements.** No Chrome frame-time or startup measurement.

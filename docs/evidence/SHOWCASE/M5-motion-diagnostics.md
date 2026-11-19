# SHOWCASE M5 motion diagnostics

The first review reproduced interruptions in actual submitted images. The Mac held frame stayed
unchanged for 49.81 ms and the travelling camera stepped over a foreground balloon after an 86.31 ms
gap. Chrome recorded a 92.8 ms gap around the same foreground crossing. Both retained all their
submitted images. [First review](M5-review.md)

The follow-up keeps the cooked root and flythrough. The Mac headless exit captures request 60 Hz
against the 60 Hz world, with optional GPU pass timing off. The capture-off arm runs the same route.
Chrome's exit capture uses the page's animation-frame callbacks with pass timing off. It transfers
  reusable image buffers through a worker and stages originals in a bounded disk spool. The 720p SDR
  arm can retain up to 64 in-flight exports. After the page
signals completion, the receiver verifies the originals and packages them losslessly. The elapsed
timeline records the actual submissions and their gaps.

The six refreshed fixed Chrome submissions contain 10,000 active rows and match the retained Chrome
RGBA bytes at each paired Mac stop. [Retained fixed images](../../../.orchestrate/evidence/SHOWCASE/M5-fixed-submitted.tar.gz)

## Diagnostic runs under shared load

- The first Chrome comparison with live gzip omitted 233 exports. With the worker, reusable arrays,
  pass timing disabled and the single raw spool, a later run retained 1,464 images with no omissions.
  That run still had 64.3 ms between submissions 725 and 726 at 11.6245 seconds. The render calls
  surrounding that gap each took about one millisecond. The adjacent capture-off take kept page
  callbacks within 18.8 ms of one another. The raw-spool experiment also had a separate overloaded
  run with 45 missing exports.
- The fresh Mac capture-off route still had an 83.51 ms interval between successful submission
  returns and 213 unsubmitted opportunities. A separate Mac route capture held both render
  slots for 18 intervening opportunities during a 157.74 ms image gap. The scene stepped again
  once a slot retired. Concurrent tests and builds were using the host during both runs.
- The rebuilt Mac held capture recorded 1,243 images with zero ring and writer omissions.
  Its longest repeated-image interval after release was 19.72 ms, and it still had a 183.08 ms
  gap from tick 605 to 616. The matching route had a 73.28 ms gap. Shorter duplicate holds alone
  therefore do not establish continuous motion.
- Another held take stopped submitting at 21.64 seconds and stayed at committed tick 1,226
  through the end of the 23-second interval. The runtime reported a complete take because every
  earlier submission had been stored. The exit recorder now drains and resumes the running world
  through its lifecycle barriers on overload. It records the refused ticks and classifies any
  take with overload recovery as incomplete, even when every submitted image was saved.
- A controlled 400 ms suspension of just the Metal capture process triggered this recovery with
  24 refused ticks. The world resumed around 2.5 ms after the recorder observed overload,
  subsequently rendering 1,173 submissions through tick 1,293. The take reported incomplete.

## Reproduced shadow transition

A fresh visual reader found a dark strip appearing in the terrain shadow between Metal submissions
521 and 522 at 4.449441 and 4.452691 seconds. The strip persists through submission 528. The camera
advanced from tick 233 to 236 across those two submissions, following two longer submission gaps.
The same balloons stay visible through the inspected sequence. [Consecutive submitted crops](M5-motion-shadow-crop.png)

A separate fixed diagnostic replayed the same camera and rendered every tick. The crops from committed
ticks 232, 233, 234 and 235 show the same strip appearing at tick 234. The comparison rectangle counts
280, 2,710 and 308 pixels that darken by more than 30 points in mean RGB across each adjacent pair.
This reproduces the shadow discontinuity without a missed render opportunity.
[Fixed diagnostic crops, left to right](M5-motion-fixed-shadow-crop.png)

Sampling the Mac real-time capture at the elapsed observation restores fractional interpolation. A
follow-up kept 1,305 held images over 22.99 seconds and 1,403 route images over 24.51 seconds, each
with zero capture omissions. It still had gaps of 54.02 ms in the held view and 70.00 ms on the route.
Chrome's 64-buffer profile retained 1,450 images in a complete take, with gaps of 121.8 ms at
9.238 seconds and 176.1 ms at 22.828 seconds.

[Per-attempt timings, build identities and fixed-stop receipts](M5-motion-diagnostics.json)
· [Lossless excerpts and source timing records](../../../.orchestrate/evidence/SHOWCASE/M5-motion-diagnostic-source.tar.gz)

Restore the archive via `python3 scripts/evidence_store.py restore docs/evidence/SHOWCASE/M5-motion-diagnostic-source.tar.gz`.

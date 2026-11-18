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
RGBA bytes at each paired Mac stop. [Retained fixed images](M5-fixed-submitted.tar.gz)

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

[Per-attempt timings, build identities and fixed-stop receipts](M5-motion-diagnostics.json)

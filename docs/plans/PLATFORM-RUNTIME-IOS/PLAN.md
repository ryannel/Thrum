# PLATFORM-RUNTIME-IOS plan

The iOS shell on the iPhone 18 Pro simulator, a function target (gate G3b). [Design](DESIGN.md) ·
[done-when file](DONE.json), which holds M7 · [log](LOG.md) · [module list](../ARCHITECTURE.md)

## M1 The simulator game presents frames while UIKit stays responsive through a render stall: done
[record and reviews](../../evidence/PLATFORM-RUNTIME-IOS/M1-M7-records.md#m1-off-main-frames-done)

## M2 A real simulator swipe moves the committed player, and a cancelled touch stops it: done
[record and reviews](../../evidence/PLATFORM-RUNTIME-IOS/M1-M7-records.md#m2-touch-input-done)

## M3 The game returns after backgrounding and after its surface is replaced, with the same world: done
[record and reviews](../../evidence/PLATFORM-RUNTIME-IOS/M1-M7-records.md#m3-lifecycle-and-surface-recovery-done)

## M4 The simulator route passes from a clean check, with its evidence and maturity recorded: done
[record and reviews](../../evidence/PLATFORM-RUNTIME-IOS/M1-M7-records.md#m4-closeout-done)

## M5 The game rotates between portrait and landscape and keeps presenting, with the player preserved: done
[record and reviews](../../evidence/PLATFORM-RUNTIME-IOS/M1-M7-records.md#m5-rotation-done)

## M6 The game recovers from a lost graphics device in the same process, with world and controls intact: done
[record and reviews](../../evidence/PLATFORM-RUNTIME-IOS/M1-M7-records.md#m6-device-loss-done-thrum-core-approved-the-seam-with-conditions)

## M7 Scene and internal render size follow the oriented iOS surface: done
[review](../../evidence/PLATFORM-RUNTIME-IOS/M7-review.md)

# PLATFORM-RUNTIME-MAC review briefs

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. The briefs are unchanged; the verdicts they led to are in [the log](../../plans/PLATFORM-RUNTIME-MAC/LOG.md).

## Design and plan review brief

Check the code and contracts over `3049433d`. Trace the world and platform
owners, loss notification around the held drawable, fresh generation
admission and the no-injection oracle. Check that each proposed
observation supports its stated conclusion. In particular distinguish
display link, submit, GPU completion, VSync and attributed presentation,
and find any claim that the proposed tools cannot measure. Confirm scope
separation from G3t/G4 and the shipped browser/iOS paths.

Sources: [Apple’s CADisplayLink documentation](https://developer.apple.com/documentation/QuartzCore/CADisplayLink) ·
[Instruments display and Metal events](https://developer.apple.com/documentation/xcode/analyzing-the-performance-of-your-metal-app) ·
[Metal HUD intervals](https://developer.apple.com/documentation/xcode/monitoring-your-metal-apps-graphics-performance) ·
[MTLDrawable presentedTime](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime).

## Lost frame review brief

Review the token and Mac actor diff over `f79e3434` against C06.
Verify timing and drop accounting, healthy staging flush, loss checks
before and after acquisition, and the real Metal queue-attempt control.
Keep encode-time failure and live-window recovery outside this proof.

## M2 pan plan review brief

Start from [the unattended-evidence brief](../../plans/T2-CAPTURE/BRIEF.md), then review the T2 design/JSON, C12, Mac M2
items and the split M3 post-G3t dependency. Confirm no M2 item waits on parked APPLE-TILE, and that M3
retains fresh motion, windowed-tap and panel/tooling proof on the changed path without renewing diagnostic attempts.
Check that M2.1 can expose runtime stalls without any desktop grant;
synthetic fixed-time movie smoothness cannot pass. For M2.2 require normal AppKit routing, same-build capture
brackets and fresh independent Game Performance swap attribution for the complete pan. Engine taps supply image
content, not final compositor/native-widget appearance, physical panel observation or photon timing. Confirm that
attended setup is justified by real display/tool needs, M2.3 proves the real-window tap, M2.4 owns a bounded
resumption of trace finalization, and only the capture-OFF traced arm qualifies shipping cadence. No parked failure
is treated as solved by this plan.

## Injected input review brief

Compare the diagnostic App with `43e1d488`. Check that injected P/E use
the physical keys' shared app handler, with the same lifecycle and edits.
Require injected P before the normal Resume acknowledgement and E inside
the recording. Keep the actual recovered motion review separate.

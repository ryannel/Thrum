# SHOWCASE M5 motion handoff

M5.8 and M5.9 remain open. The motion capture changes keep the same exit layout, cooked root,
flythrough, and controls. The six refreshed fixed Chrome shots reproduce the retained submitted
bytes and match the paired Mac props and balloons. The T2 changes reduce capture overhead and
detect an overload that previously left the Mac world stopped for the rest of an apparently
complete take. A controlled suspension exercised recovery on Metal. [Diagnostics](M5-motion-diagnostics.md)

Start with the Metal shadow transition at committed ticks 233 and 234 on the checked-in exit
camera. The fixed diagnostic reproduced it with every tick submitted, so fixing the elapsed-time
capture schedule alone cannot remove it. The archive contains the diagnostic route, exact source
images and identity records. Determine which part of the Sun-shadow renderer produces the strip
and coordinate the correction with the renderer owner so the registered exact references keep
their meaning.

Repeat the Chrome flythrough and the Mac flythrough and held view with Chrome and GPU capacity
available for the entire takes. Keep the back-to-back observer arms. A complete readback can
still contain whole-frame gaps. For example, the current Chrome capture saved all 1,450 submitted
images and had 121.8 ms and 176.1 ms gaps while the route was running. The Mac capture-off arm
also had gaps, so the writer alone cannot explain the native result. The shadow transition
and remaining whole-frame timing are distinct observations. The cause of individual host
scheduling delays is still unresolved.

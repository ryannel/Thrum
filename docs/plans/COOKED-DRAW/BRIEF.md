# Cooked draw path brief

Owner decision, 9 November 2026, relayed by the lead: give G5's cooked scene a portable draw path for GPUs without
indirect execution, so [T2-CAPTURE](../T2-CAPTURE/PLAN.md) M2 (iOS simulator capture) can resume.

G5 owns cooked GPU residency and drawing in `crates/thrum-asset-gpu` under
[the G5 decision](../../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md). Its scene contribution culls meshlets on the GPU and
draws them with `multi_draw_indexed_indirect`. The iPhone 18 Pro / iOS 27.2 simulator's Metal adapter does not report
`DownlevelFlags::INDIRECT_EXECUTION`, so no cooked scene draws there at all
([evidence and reproducer](../../evidence/render/t2-capture-m2/README.md)). The
[hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) and C04 require every accelerated path to be
selected from the capability registry, with a portable fallback. This path has none, so this closes a gap against
that decision. It is not a simulator workaround.

Deliver:
- A cooked draw path that works without indirect execution, chosen from the adapter's actual capabilities as recorded
  in the capability registry. Never choose by platform, OS or adapter name.
- The indirect path stays selected wherever it is supported (Mac Metal, Chrome WebGPU), and its output and per-frame
  cost stay unchanged. Byte-identical output before and after proves this.
- The cooked scene renders correctly on the simulator, judged against an inspection list. Simulator evidence proves
  function only, never speed.
- A per-frame cost account written before building, under [the performance policy](../../design/AI-PERFORMANCE.md#performance-investment-from-the-first-design).
  Record measurements and set tripwires; invent no budget.

Keep one authority for each kind of state. The contribution seam, motion history (C07), GPU lifetime (C06) and
evidence (C12) keep their meaning. Name every contract or interface change and the owners who must agree to it.
State how T2-CAPTURE M2 depends on this work. Broadsides and other worktrees are out of bounds.

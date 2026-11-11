# Performance philosophy — owner decision, 6 November 2026

At 11:15 the owner replaced numeric performance gates with three rules. The 1 November ambition remains:
**shocking performance with mature engineering, full use of hardware and a strong reusable foundation.**

1. Design for speed up front. Review expensive-to-reverse data layouts, GPU-driven paths, threading, memory ownership,
   hot-path storage reuse and hardware-native seams against best practice. Foundation decisions do not wait for benches.
2. Build with tripwires: visible stutter, dropped frames, slow startup or hitches; more than about 25% regression from
   the last comparable recorded baseline; or unexpected algorithmic growth. Investigate and fix causes without blocking
   other building. Keep measurements as history, with honest source/device/workload identities.
3. Tune later in a dedicated performance phase, using whole-frame profiles on real devices and targets derived from
   sufficient engine/demo content. This is the owner's AI performance push.

This overrides earlier numeric performance/allocation admission gates in plans and skills. Correctness, independent
oracle tolerances, lifecycle recovery, lint/checks and agent visual review remain requirements. G2S closes on its
functional correctness; G3b retains function and lifecycle work. Keep native Metal in G3t; its complete recorder is a
candidate for later performance work, not an early response to wgpu allocations. Park current allocation/timing-only
experiments at safe commits. Build visible sky, Sun, sunset and twilight next; finish the half-float storage correction.

[Policy](../AI-PERFORMANCE.md#performance-investment-from-the-first-design) owns the method;
[build plan](../../plans/BRIEF.md) owns current work. Historic measurements remain evidence, not current admission bars.

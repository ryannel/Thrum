Inspection list written before viewing captures (commit 783502e58ee6ed24161f812c249cfd7e19e872da).

At noon and low Sun, capture provider off/on slow pans on Metal realtime and Chrome fixed/realtime. Inspect whole frames and native-size crops for:
1. Terrain-to-cooked-mesh patch-edge alignment, including synthetic instances where visible.
2. Shaded ground beside sunlit ground: retained skylight, no black patch.
3. Cloud crossing an existing ridge shadow: no second darkening of geometric umbra.
4. A world-space landmark over two seconds: drift consistent with declared [160,40] m/s and receiver heights.
5. Cross-target shade placement and depth at matched route times.

Also exercise distant receivers, low-Sun support, storm-strength optical depth, and camera motion where applicable. Capture timing is engine elapsed-time evidence, not physical 120 Hz presentation. No image anchors are configured; use the written M5 bar, excluding diagnostic pattern shape/softness.

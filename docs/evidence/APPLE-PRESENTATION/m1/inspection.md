# M1 motion inspection checklist

Written before looking at the captures. All views use the portable recipe on mac-metal, through fixed-time
submitted engine output. This is motion-vector correctness/appearance, not panel cadence or elapsed-time performance.

- Whole noon pan: stationary holds have zero motion, pan begins/ends coherently, signed X follows history minus
  current (a rightward camera turn means scene pixels move left and their history lies to the right).
- Skyline at native scale: terrain and infinity meet without a band of cleared zero motion or invalid history.
- Near prop against far terrain: coherent camera correspondence; route translation may reveal parallax.
- Night star field: no zero-motion holes where stars replace sky motion. Camera and celestial phase are checked
  separately by attachment readback; during a camera pan stars and sky legitimately have matching infinity motion.
- Moving object: local object motion differs from its stationary surroundings; newly invalid geometry is magenta.
- Pan centre versus edges: magnitudes vary continuously with the projection and grow toward the horizontal edges.
- Review ordered frames through the start, middle and end of each movement, plus native-size crops. The movie keeps
  the fixed sample timing; an agent's sequence-of-frames inspection is recorded explicitly rather than claiming
  physical display observation.

Results are recorded in the milestone evidence page after inspection.

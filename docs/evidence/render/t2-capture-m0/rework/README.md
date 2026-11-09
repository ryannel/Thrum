# Historical T2-CAPTURE M0 first rework

The claim at `6be1b11b` failed its second review at `c65a90e6` (M0.3 and M0.8). Its raw-writer throughput and noisy
OFF/ON cost figures are historical, not current qualification. Measurements, manifests and profiles remain here.
The original narrative is preserved byte-for-byte in `historical-README.md.gz`.

The [current reliability rework](../reliability/README.md) retains the replacement series and stable baseline.
Its [retirement record](../reliability/historical-retirement.json) names 34 duplicate PNG/movie files removed
after selected replacement roots and full source hashes verified. Restore an exact historical file with
`git show c65a90e6:<repository-path> > <destination>`; nothing was rewritten in history. Local old source archives
remain under `tmp/t2-capture-m0-rework/integrated/` through the next review. No reviewer was launched.

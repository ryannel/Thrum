# Sampling cooked products from an engine pass

Residency owns all uploads. A module names the products it needs before the scene is prepared;
the game adds those `LeaseRequest`s to `CookedConfig::leased`, constructs `CookedAssets`, and
passes `assets.lease()` to the module's contribution. Attach both to `SceneContributions` in
either order. No game, sea, cloud or placement type appears in this interface.

```rust,ignore
config.leased.push(LeaseRequest {
    name: "foam".into(),
    dimension: wgpu::TextureDimension::D2,
});
config.leased.push(LeaseRequest {
    name: "density".into(),
    dimension: wgpu::TextureDimension::D3,
});
let assets = CookedAssets::new(config)?;
let consumer = MyPass::new(assets.lease());
```

During extraction/upload, call `ProductLease::offered()` once. `None` means loading or a new
device with nothing resident. `LentSet` contains the store generation and products in request
order. Each product carries its name, exact content hash, selected profile, device generation,
serial, texture descriptor access and retained view. Bind the view in your own pass. Rebuild
bind groups for changed views; their sampling layout stays filterable float D2 or D3 across
the portable and preferred profiles. No pipeline compilation is needed on an edit.

Compare `(device_generation, serial)` with what your last history-advancing submission used.
Record that comparison for the tentative frame, apply your declared history invalidation and
reactive coverage, and commit the observed keys only in `submitted(true)`. Discard commits
nothing. A capture calls `submitted(false)`: it may commit the publication, but your change
must remain pending for the next history-advancing frame. Residency keeps no state for each
consumer. The opt-in `v1_game::lent::LentDiagnostic` is an executable example using only this
public lease.

Products are offered as one whole publication after all copies have been submitted. The frame
recording the last copies still sees the previous set. Every following recording selects the
ready set through placements and leases together, independent of contribution order. Discarding
the switch retries that same set. A ready set cannot be superseded; a newer edit waits behind
it. While preparation is incomplete, newer publications replace it. A missing, malformed,
device-refused or failed product refuses the whole publication and retains the last good set.
The diagnostic reason names the product where resolution/validation fails.

On device replacement drop all cached bindings and observed keys in `rebuild`, even when
the lease is empty. Residency restages the content on the replacement device before offering
it. Do not bind a view whose device generation differs from the frame. Retaining a snapshot
keeps its handles alive; release old snapshots/bindings promptly after the submitted switch.

Residency supplies texels, not their meaning: physical extent, placement, optics, animation,
sampling conventions and history policy belong to the consumer. It never cooks, creates a
consumer's pipeline, keeps its history, submits its command buffer or provides another loader.
For a CPU reader, map the portable product named by the lease through `thrum-asset`, pinned by
its `portable_hash`; never independently follow `HEAD`. The selected GPU encoding's identity
is `hash`; both hashes belong to the same offered publication. A foam texture can use a `.linear.png` source;
volumes use portable R8 and preferred BC4 where sliced-3D BC is available. Font and sprite-sheet
metadata, array pages, volume atlases and iPhone-specific volume encodings remain later work.

See the [publication design](plans/CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2).

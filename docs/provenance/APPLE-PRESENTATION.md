# Apple presentation provenance

## wgpu-hal

The Metal stage record loader uses the published wgpu-hal 30.0.1 crate, licensed under MIT or Apache-2.0.
The crate records upstream revision `40f4a34ebaf56f9a046231f54125ad046239d3f3`. The release archive SHA-256 is
`b6b7fb58561a792bc237628ba0792e332de418fefe145f13b5ed8201e6d52f58`.
The repository keeps the release with its licenses intact under `vendor/wgpu-hal-30.0.1`.
Thrum's [stage record patch](../../vendor/wgpu-hal-patches/0001-metal-stage-records.patch) is kept beside the
copy so it can be reviewed and reapplied independently. The pin gives passthrough stages the same reflection
data as WGSL stages while keeping wgpu, wgpu-core, wgpu-types and Naga at their published versions.
Run `python3 scripts/apple_presentation.py wgpu-pin` to verify the copy against the retained crates.io release and patches.

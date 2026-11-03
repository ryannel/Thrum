# Cloud and atmosphere assembly

Maintained standalone proof for the [cloud programme](../../docs/design/modules/CLOUD-MIGRATION.md).
This directory is the assembly destination for selected implementations, not a Thrum runtime provider.
Currently it contains the reviewed pure CPU fog reference; the complete renderer/lifecycle assembly is unfinished.

```sh
node --test scripts/cloud_atmosphere_proof/tests/air/fog.test.mjs
```

`src/air/fog.js` owns validated immutable analytic ellipsoid profiles and point/segment optical queries.
It has no weather, clock, GPU, world-state or renderer dependency. Profiles use world metres and extinction per metre.
The binary64 reference assumes host sqrt/exp within four ulps; a GPU/f32 counterpart is unqualified.
[Design, exact source pins and review](../../docs/evidence/cloud-joint-media-design-2026-11-03/README.md)
keep CPU correctness separate from atmospheric appearance, numerical GPU error and frame/memory cost.

The existing frozen source91 viewer remains the runnable comparator. It is not imported from scratch or silently
modified by this reference. New content and all-consumer integration must be checked before replacing its pins.

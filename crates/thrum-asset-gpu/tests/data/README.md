# Mapped index fixture

The 7,504-byte CAS product is the showcase's `rock`, cooked on macOS at `a3dd15da8`
using the authoring cooker and the original valley source.
The filename is its BLAKE3 content hash; the test verifies it before opening the mapping.
It contains 672 indices across three meshlets. The cooker and cooked format are unchanged by M3.

The cooked-view GPU test also uses the fixture as a generation with a 1×1 white texture.
Its HEAD, manifest and texture variants were cooked with the default profiles from the
lighthouse example's copies of `meshes/rock` and `textures/white`. The test supplies its own
camera, placements and lighting, and loads the generation through the ordinary local store.

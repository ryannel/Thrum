//! Stdin fuzz target for every untrusted product parser in the shipping runtime: `HEAD`, the
//! manifest, mesh products (including reference validation) and KTX2 textures/volumes. Compile
//! once, then run the binary under an external stdin fuzzer.

use std::hint::black_box;
use std::io::Read;

use thrum_asset::{Head, Ktx2, Manifest, MeshProduct, ProductBytes};

fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin().take(64 << 20).read_to_end(&mut bytes)?;
    black_box(Head::decode(&bytes).ok());
    if let Ok(manifest) = Manifest::parse(ProductBytes::copy_from(&bytes)) {
        for entry in manifest.entries() {
            black_box((entry.name, entry.product, entry.kind, entry.profile));
        }
    }
    if let Ok(mesh) = MeshProduct::open(ProductBytes::copy_from(&bytes)) {
        black_box(mesh.validate_references().ok());
        black_box((
            mesh.meshlets().len(),
            mesh.meshlet_bounds().len(),
            mesh.submeshes().len(),
        ));
    }
    if let Ok(texture) = Ktx2::open(ProductBytes::copy_from(&bytes)) {
        for level in texture.levels() {
            black_box(texture.level_data(level).len());
        }
    }
    Ok(())
}

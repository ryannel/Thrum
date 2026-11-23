//! Source art shared by the atlas upload and submitted edge proofs.

/// Opaque colour for the outline and its backing.
pub const WHITE: [u8; 4] = [255; 4];
/// Opaque colour for the isolated frame and the left tile.
pub const CYAN: [u8; 4] = [0, 255, 255, 255];
/// Opaque colour for the right tile.
pub const YELLOW: [u8; 4] = [255, 255, 0, 255];
const PINK: [u8; 4] = [255, 0, 255, 255];

/// Width and height of the RGBA source sheet in texels.
pub const EXTENT: [u32; 2] = [24, 32];
/// Width and height of each source frame in texels.
pub const FRAME_SIZE: [u32; 2] = [8, 8];

/// Return row-major RGBA bytes and the authored origin of each frame.
pub fn art() -> (Vec<u8>, [[u32; 2]; 12]) {
    let tiles = [PINK, PINK, PINK, PINK, CYAN, PINK, PINK, PINK, PINK];
    let mut rgba: Vec<_> = (0..24)
        .flat_map(|y| (0..24).flat_map(move |x| tiles[y / 8 * 3 + x / 8]))
        .collect();
    // White with poisoned transparent RGB inside the same frame, over white.
    for y in 0..8 {
        for x in 0..8 {
            let i = ((y + 16) * 24 + x + 16) * 4;
            rgba[i..i + 4].copy_from_slice(if (2..6).contains(&x) && (1..7).contains(&y) {
                &WHITE
            } else {
                &[255, 0, 0, 0]
            });
        }
    }
    rgba.extend((0..8).flat_map(|_| {
        (0..24).flat_map(|x| match x / 8 {
            0 => WHITE,
            1 => CYAN,
            _ => YELLOW,
        })
    }));
    let mut origins = [[0; 2]; 12];
    for (tile, origin) in (0..).zip(&mut origins) {
        *origin = [tile % 3 * FRAME_SIZE[0], tile / 3 * FRAME_SIZE[1]];
    }
    // A 64-wide atlas shelf holds four of these padded 16×16 blocks.
    // Put the cyan witness second on the middle shelf.
    origins.swap(4, 5);
    (rgba, origins)
}

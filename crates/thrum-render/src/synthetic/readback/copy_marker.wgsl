// Copy timing only: consume one readback word without changing any image byte.
// The externally visible sink store keeps this dispatch real. Before/after
// reads bracket the blit's buffer write via storage/COPY_DST hazards.
@group(0) @binding(0) var<storage, read> image: array<u32>;
@group(0) @binding(1) var<storage, read_write> sink: array<u32>;
@compute @workgroup_size(1)
fn main() {
    sink[0] = image[0];
}

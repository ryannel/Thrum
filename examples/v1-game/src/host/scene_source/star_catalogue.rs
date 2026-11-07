//! Harvard Table I preview at the printed equinox 1900.
const STAR_COUNT: usize = 6632;
const PACKED: &[u8; STAR_COUNT * 16] = include_bytes!("harvard-1908.f32le");

pub(super) static HARVARD: [[f32; 4]; STAR_COUNT] = {
    let (records, _) = PACKED.as_chunks::<16>();
    let mut stars = [[0.0; 4]; STAR_COUNT];
    let mut index = 0;
    while index < STAR_COUNT {
        let (channels, _) = records[index].as_chunks::<4>();
        stars[index] = [
            f32::from_le_bytes(channels[0]),
            f32::from_le_bytes(channels[1]),
            f32::from_le_bytes(channels[2]),
            f32::from_le_bytes(channels[3]),
        ];
        index += 1;
    }
    stars
};

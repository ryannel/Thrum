//! The shipping height-map march against a flat plateau at a known height and extent.
use super::disk;
use super::observer::{SAMPLE_FLOATS, constant_depth_maps, device, pipeline, read_visibility};
use super::{HORIZON_RADIUS, SIZE, SUN_ANGULAR_RADIUS, Vec3, height_matrix};

const OBSERVER: &str = r"
@group(2) @binding(6) var<storage,read_write> result: array<f32>;
@compute @workgroup_size(64)
fn observe(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=3u { return; }
    let height=array(-5.0,5.0,120.0)[id.x];
    let receiver=ShadowReceiver(vec3(0.0,height,0.0),
        vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(0.0,1.0,0.0));
    result[id.x]=horizon_visibility(receiver);
}
";

#[test]
fn a_below_horizontal_sun_is_partially_hidden_by_distant_lower_terrain() {
    pollster::block_on(async {
        let (device, queue) = device().await;
        let (matrix, _) = height_matrix(Vec3::ZERO, Some((0.0, 0.0))).expect("height map fit");
        let texel = 2.0 * HORIZON_RADIUS / SIZE as f32;
        let depth = matrix.row(2);
        let mut samples = [0.0; SAMPLE_FLOATS];
        samples[48..64].copy_from_slice(&matrix.to_cols_array());
        samples[64..68].copy_from_slice(&[depth.y, depth.w, 0.5 / HORIZON_RADIUS, texel]);
        let maps = constant_depth_maps(&device, &queue, SIZE, &[0.0, 0.0, depth.w]);
        let elevation = -0.1_f32.to_radians();
        let (sin, cos) = elevation.sin_cos();
        let observed = read_visibility(
            &device,
            &queue,
            &pipeline(&device, OBSERVER),
            &samples,
            &maps.create_view(&wgpu::TextureViewDescriptor::default()),
            [cos, sin, 0.0, 0.0],
            3,
        );
        // The reference uses an actual flat mesh, with the same height allowance
        // as the sampled map. The GPU still allows 0.015 for the march's last
        // sample falling short of the geometric edge.
        let extent = f64::from(HORIZON_RADIUS);
        let height = -0.125 * f64::from(texel);
        let a = [-extent, height, -extent];
        let b = [extent, height, -extent];
        let c = [extent, height, extent];
        let d = [-extent, height, extent];
        let expected = disk::visibility(
            [0.0, 5.0, 0.0],
            [f64::from(cos), f64::from(sin), 0.0],
            f64::from(SUN_ANGULAR_RADIUS),
            &[[a, b, c], [a, c, d]],
        );
        assert!(observed[0] <= 1e-6, "receiver below plateau: {observed:?}");
        assert!(
            (f64::from(observed[1]) - expected).abs() < 0.015,
            "grazing Sun: GPU {}, plateau reference {expected}",
            observed[1]
        );
        assert!(
            observed[2] > 1.0 - 1e-6,
            "receiver high above plateau: {observed:?}"
        );
    });
}

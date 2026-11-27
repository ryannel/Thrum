fn cloud_layer_ground_path(world: vec3<f32>) -> vec2<f32> {
    return vec2(1.0, 0.0);
}

fn cloud_near_light(world: vec3<f32>, ray: vec3<f32>, true_sun: vec3<f32>,
                    distance: f32, transmission: f32) -> GroundLight {
    return GroundLight(0.0, 0.0, vec3(0.0), 1.0);
}

fn cloud_layer_at_height(height: f32) -> bool { return false; }

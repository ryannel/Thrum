struct View {raster:mat4x4<f32>,previous:mat4x4<f32>,camera:vec4<f32>,exposure:vec4<f32>,unjittered:mat4x4<f32>,extra:vec4<u32>}
struct Aux {inverse:mat4x4<f32>,extent:vec4<f32>,near_far:vec4<f32>,time:vec4<f32>,flags:vec4<u32>,rectangle:vec4<f32>}
@group(0) @binding(0) var<uniform> view:View;
@group(0) @binding(1) var<uniform> aux:Aux;

@group(2) @binding(0) var<uniform> cloud:CloudParams;
@group(2) @binding(1) var columns:texture_2d_array<f32>;
@group(2) @binding(2) var optical:texture_2d_array<f32>;
@group(2) @binding(3) var transport:texture_storage_2d<rgba16float,write>;
@group(2) @binding(4) var front:texture_storage_2d<rgba16float,write>;

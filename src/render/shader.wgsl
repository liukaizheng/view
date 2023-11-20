 // Because Downlevel flags BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED are required but not supported on web
// we use vec4 instead of vec3 
struct Material {
    ka: vec4<f32>,
    kd: vec4<f32>,
    ks: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> view: mat4x4<f32>;
@group(0) @binding(1)
var<uniform> proj: mat4x4<f32>;

// Because Downlevel flags BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED are required but not supported on web
// we use vec4 to represent light position instead of vec3 
@group(0) @binding(2)
var<uniform> light_pos: vec4<f32>;

@group(1) @binding(0)
var<uniform> material: Material;

@vertex
fn vs_main(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    let out = proj * view * vec4<f32>(pos, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return material.ka;
    // return light_pos;
}

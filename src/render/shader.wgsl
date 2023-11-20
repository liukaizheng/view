struct VertexInput {
    @location(0) point: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) pos_in_eye: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

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

@group(0) @binding(3)
var<uniform> normal_mat: mat4x4<f32>;

@group(1) @binding(0)
var<uniform> material: Material;

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let pos_in_eye = view * vec4<f32>(v.point, 1.0);
    out.pos_in_eye = pos_in_eye.xyz;

    let pos = proj * pos_in_eye;
    out.clip_pos = pos;

    var normal_in_eye = (normal_mat * vec4f(v.normal, 1.0)).xyz;
    out.normal = normalize(normal_in_eye);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // ambient intesensity
    let ia = material.ka.xyz;

    let eye_to_light = normalize(light_pos.xyz);
    let dot_prod = max(dot(eye_to_light, in.normal), 0.0);
    // diffuse intensity
    let id = material.kd.xyz * dot_prod;
   
    let reflect_in_eye = reflect(-eye_to_light, in.normal);
    let surface_to_viewer_eye = normalize(-in.pos_in_eye);
    let dot_prod_specular = max(dot(reflect_in_eye, surface_to_viewer_eye), 1.0);
    let specular_factor = pow(dot_prod_specular, 35.0);
    // secular intensity
    let is = material.ks.xyz * specular_factor;
    return vec4f(ia + is + id, 1.0);


}

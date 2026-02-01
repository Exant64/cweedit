override use_texture: bool = false;

// based on 
// specular: https://github.com/shaddatic/sa2b-render-fix/blob/master/sa2b-render-fix/rf_ninja/rj_cnk/rjcnk_cfunc.c#L184
// ambient: 
struct VertexOutput {
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) intensity: f32,
    @builtin(position) position: vec4<f32>,
};

struct UniformData {
    transform: mat4x4<f32>,
    inverse_transpose_modelview: mat4x4<f32>,
    diffuse_color: vec4f,
    palette_colors: array<vec4f, 48>,
    palette_index: i32,
    texture_size: f32,
    chao_mode: i32,
    use_env: i32,
    light_direction: vec3<f32>,
    use_bald: i32,
    bald_influence: vec3<f32>,
    pad: f32,
    bald_center: vec3<f32>,
    bald_radius: f32,
    bald_clip_face: i32,

    // rf-only for now
    ignore_light: i32,
    ignore_ambient: i32,
    ignore_specular: i32,
    ambient_color: vec3f,
    specular_exponent: f32,
    specular_color: vec3f
}

@group(0)
@binding(0)
var<uniform> uniformData: UniformData;

@group(0)
@binding(1)
var<uniform> projectionMatrix: mat4x4<f32>;

const LIGHT_INTENSITY = 1.0;
const LIGHT_AMBIENT = 0.3;

@vertex
fn vs_main(
    @location(0) position: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
) -> VertexOutput {
    var result: VertexOutput;

    result.tex_coord = tex_coord;
    result.normal = normal;
    result.position = projectionMatrix * vec4<f32>(position.xyz, 1.0f);
    
    // technically i think it should be -LIGHT_DIRECTION, keeping it like this for now though
    // in real RF this is supposed to support multiple lights but we don't really need that
    result.intensity = dot(normalize(normal), uniformData.light_direction) * LIGHT_INTENSITY;

    return result;
}

@group(1) @binding(0) var diffuse_texture_sampler : sampler;
@group(1) @binding(1) var diffuse_texture : texture_2d<f32>;

@fragment
fn fs_main(vertex: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    var material_color = uniformData.diffuse_color.rgb;
    var uv = vertex.tex_coord;

    var intensity = vertex.intensity;
    if !is_front {
        intensity = -intensity;
    }
    intensity = max(intensity, 0.0);

    if uniformData.use_env != 0 || uniformData.chao_mode == 1 || uniformData.chao_mode == 3 {
        uv = normalize(vertex.normal).xy * 0.5 + vec2<f32>(0.5, 0.5);
    }

    var diffuse_tex_color: vec4f;
    if use_texture {
        diffuse_tex_color = textureSample(diffuse_texture, diffuse_texture_sampler, uv);
    }
    else {
        diffuse_tex_color = vec4f(1,1,1,1);
    }

    if uniformData.ignore_light != 0 {
        return uniformData.diffuse_color * diffuse_tex_color;
    }

    let diffuse = vec4<f32>(vec3<f32>(intensity), 1.0) * uniformData.diffuse_color;
    let ambient = vec4<f32>(f32(1 - uniformData.ignore_ambient) * LIGHT_AMBIENT * uniformData.ambient_color, 0);
    let specular_intensity = pow(intensity, uniformData.specular_exponent);

    return (ambient + diffuse) * diffuse_tex_color + vec4<f32>(f32(1 - uniformData.ignore_specular) * specular_intensity * uniformData.specular_color, 0);
}
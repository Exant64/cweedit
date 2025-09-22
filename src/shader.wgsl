override use_texture: bool = false;

struct VertexOutput {
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
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
    bald_clip_face: i32
}
@group(0)
@binding(0)
var<uniform> uniformData: UniformData;

@group(0)
@binding(1)
var<uniform> projectionMatrix: mat4x4<f32>;

@vertex
fn vs_main(
    @location(0) position: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.tex_coord = tex_coord;
    result.normal = normal;
    
    if (uniformData.use_bald != 0) {
        var pos: vec3<f32> = (uniformData.transform * vec4<f32>(position.xyz, 1.0f)).xyz;
        var keep_face: f32 = 1.0;
        if (uniformData.bald_clip_face != 0) {
            keep_face = saturate(sign(-pos.z + 0.86f));
        }
        
        let center: vec3<f32> = pos - uniformData.bald_center;
        let len: f32 = length(center) - uniformData.bald_radius;
        
        pos = pos - uniformData.bald_influence * normalize(center) * len * keep_face;
        result.position = projectionMatrix * uniformData.inverse_transpose_modelview * vec4<f32>(pos, 1.0f);
    }
    else {
        result.position = projectionMatrix * vec4<f32>(position.xyz, 1.0f);
    }

    return result;
}

@group(1) @binding(0) var diffuse_texture_sampler : sampler;
@group(1) @binding(1) var diffuse_texture : texture_2d<f32>;
@group(1) @binding(2) var shiny_texture : texture_2d<f32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    //let tex = textureLoad(r_color, vec2<i32>(vertex.tex_coord * 256.0), 0);
    //let v = f32(tex.x) / 255.0;

    var material_color = uniformData.diffuse_color.rgb;
    var uv = vertex.tex_coord;
    if(uniformData.use_env != 0 || uniformData.chao_mode == 1 || uniformData.chao_mode == 3) {
        uv = normalize(vertex.normal).xy * 0.5 + vec2<f32>(0.5, 0.5);
    }

    var diffuse_tex_color: vec4f;
    //diffuse_tex_color = textureSample(diffuse_texture, diffuse_texture_sampler, vertex.tex_coord);
    if use_texture {
        diffuse_tex_color = textureSample(diffuse_texture, diffuse_texture_sampler, uv);
    }
    else {
        diffuse_tex_color = vec4f(1,1,1,1);
    }

    let diffuse = 0.3 + clamp(dot(normalize(vertex.normal), vec3<f32>(0.0, 1.0, 0.0)), 0.0, 1.0);

    if(uniformData.chao_mode == 1) {
        // jewel
        return vec4<f32>(diffuse * diffuse_tex_color.rgb, diffuse_tex_color.a);
    }
    else if(uniformData.chao_mode == 2){
        // monotone
        return vec4<f32>(diffuse * uniformData.diffuse_color.rgb, diffuse_tex_color.a);
    }
    else if(uniformData.chao_mode == 3){
        // shiny monotone
        return vec4<f32>(diffuse * (uniformData.diffuse_color.rgb + diffuse_tex_color.rgb), diffuse_tex_color.a);
    }
    else if(uniformData.chao_mode == 4){
        // shiny twotone
        let shiny_overlay = textureSample(shiny_texture, diffuse_texture_sampler, normalize(vertex.normal).xy * 0.5 + vec2<f32>(0.5, 0.5)).rgb;
        return vec4<f32>(diffuse * diffuse_tex_color.rgb * uniformData.diffuse_color.rgb + shiny_overlay, diffuse_tex_color.a);
    }

    return vec4<f32>(diffuse * uniformData.diffuse_color.rgb * diffuse_tex_color.rgb, diffuse_tex_color.a);
    //return vec4<f32>(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);
}

@fragment
fn fs_wire(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.5, 0.0, 0.5);
}
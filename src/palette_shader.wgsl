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
    palette_index: u32,
    texture_size: f32,
    chao_mode: i32,
    pad3: i32,
    light_direction: vec3<f32>,
    use_bald: i32,
    bald_influence: vec3<f32>,
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
@group(1) @binding(1) var diffuse_texture : texture_2d<u32>;
@group(1) @binding(2) var shiny_texture : texture_2d<f32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    //let tex = textureLoad(r_color, vec2<i32>(vertex.tex_coord * 256.0), 0);
    //let v = f32(tex.x) / 255.0;

    let integer_size = i32(uniformData.texture_size) - 1;
    var texcoord_left = vec2<i32>(vertex.tex_coord * uniformData.texture_size);
    texcoord_left.x = clamp(texcoord_left.x, 0, integer_size);
    texcoord_left.y = clamp(texcoord_left.y, 0, integer_size);

    let index_top_left: u32 = textureLoad(diffuse_texture, texcoord_left, 0).r / 17;
    let index_top_right: u32 = textureLoad(diffuse_texture, texcoord_left + vec2<i32>(1,0), 0).r / 17;
    let index_bottom_left: u32 = textureLoad(diffuse_texture, texcoord_left + vec2<i32>(0,1), 0).r / 17;
    let index_bottom_right: u32 = textureLoad(diffuse_texture, texcoord_left+ vec2<i32>(1,1) , 0).r / 17;

    let color_top_left = uniformData.palette_colors[index_top_left + uniformData.palette_index];
    let color_top_right = uniformData.palette_colors[index_top_right + uniformData.palette_index];
    let color_bottom_left = uniformData.palette_colors[index_bottom_left + uniformData.palette_index];
    let color_bottom_right = uniformData.palette_colors[index_bottom_right + uniformData.palette_index];

    let f = fract(vertex.tex_coord * uniformData.texture_size);

    let top = mix(color_top_left, color_top_right, f.x);
    let bottom = mix(color_bottom_left, color_bottom_right, f.x);
    let diffuse_tex_color = mix(top, bottom, f.y);

    let diffuse = 0.3 + clamp(dot(normalize(vertex.normal), vec3<f32>(0.0, 1.0, 0.0)), 0.0, 1.0);

    var material_color = uniformData.diffuse_color.rgb;
    var uv = vertex.tex_coord;
    if(uniformData.chao_mode == 1 || uniformData.chao_mode == 3) {
        uv = normalize(vertex.normal).xy * 0.5 + vec2<f32>(0.5, 0.5);
    }
    
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
}

@fragment
fn fs_wire(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.5, 0.0, 0.5);
}
use std::{borrow::Cow, collections::HashMap, f32::consts, rc::Rc, sync::Arc};

use bytemuck::{NoUninit, Pod, Zeroable};
use egui::mutex::Mutex;
use egui_wgpu::RenderState;

use wgpu::{
    util::{align_to, DeviceExt},
    BlendComponent, BlendFactor, BlendState, Device, PipelineCompilationOptions, Queue, RenderPass,
    TextureView,
};

use crate::ninja::vertexchunk::WeightStatus;

use super::{
    anim::NinjaMotion, chunkmodel::ChunkModel, math::Color, modelfile::NinjaChunkObject,
    ninjamatrix::NinjaMatrixStack, polychunk::PolyChunk, texlist::NinjaGpuTexEntry,
    texture::gvm::NinjaTexlist, AlphaInstruction,
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
    _normal: [f32; 3],
    _tex_coord: [f32; 2],
    _color: u32,
}

impl Default for Vertex {
    fn default() -> Self {
        Vertex {
            _pos: [0.0, 0.0, 0.0, 1.0],
            _normal: [0.0, 1.0, 0.0],
            _tex_coord: [0.0, 0.0],
            _color: 0xFFFFFFFF,
        }
    }
}

#[derive(Debug, Clone, Copy, NoUninit)]
#[repr(C)]
struct NinjaUniformEntry {
    mvp: [f32; 16],
    inverse_transpose_modelview: [f32; 16],
    diffuse_color: [f32; 4],
    palette_colors: [[f32; 4]; 48],
    palette_index: i32,
    texture_size: f32,
    chao_mode: u32,
    use_env: i32,
    light_direction: [f32; 3],
    use_bald: i32,
    bald_influence: [f32; 3],
    pad: f32,
    bald_center: [f32; 3],
    bald_radius: f32,
    bald_clip_face: i32,

    // rf-only for now
    ignore_light: i32,
    ignore_ambient: i32,
    ignore_specular: i32,
    ambient_color: [f32; 3],
    specular_exponent: f32,
    specular_color: [f32; 3],
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct NinjaSampler {
    address_mode_u: wgpu::AddressMode,
    address_mode_v: wgpu::AddressMode,
    min_mag_filter: wgpu::FilterMode,
    mip_filter: wgpu::FilterMode,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct NinjaPipeline {
    use_renderfix: bool,
    use_palette: bool,
    use_texture: bool,
    use_alpha: bool,
    has_normal: bool,
    has_vcolor: bool,
    double_sided: bool,
    blend_col: wgpu::BlendComponent,
    blend_alpha: wgpu::BlendComponent,
}

#[derive(Debug)]
struct NinjaDrawEntry {
    vertex_start: u32,
    vertex_end: u32,
    uniform_offset: u32,
    texture_bind_group: wgpu::BindGroup,
    pipeline_entry: NinjaPipeline,
}

pub struct NinjaDrawState {
    draw_entries: Vec<NinjaDrawEntry>,
    vertex_buffer: wgpu::Buffer,

    projection_matrix: glam::Mat4,

    constant_buffer: wgpu::Buffer,
    projection_matrix_buffer: wgpu::Buffer,

    bind_group_layout: wgpu::BindGroupLayout,
    constants_bind_group: wgpu::BindGroup,

    regular_pipeline_layout: wgpu::PipelineLayout,
    palette_pipeline_layout: wgpu::PipelineLayout,

    regular_texture_bind_group_layout: wgpu::BindGroupLayout,
    indexed_texture_bind_group_layout: wgpu::BindGroupLayout,

    sampler_cache: HashMap<NinjaSampler, wgpu::Sampler>,
    pipeline_cache: HashMap<NinjaPipeline, wgpu::RenderPipeline>,

    swapchain_format: wgpu::ColorTargetState,

    regular_shader: wgpu::ShaderModule,
    renderfix_shader: wgpu::ShaderModule,
    palette_shader: wgpu::ShaderModule,

    placeholder_tex: wgpu::TextureView,

    vertices: Vec<Vertex>,
    uniform_data: Vec<u8>,

    min_uniform_offset: usize,
}

impl NinjaDrawState {
    pub fn draw_entries(&self, rpass: &mut RenderPass) {
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        for entry in &self.draw_entries {
            rpass.set_pipeline(&self.pipeline_cache[&entry.pipeline_entry]);

            rpass.set_bind_group(0, &self.constants_bind_group, &[entry.uniform_offset, 0]);
            rpass.set_bind_group(1, &entry.texture_bind_group, &[]);

            rpass.draw(
                entry.vertex_start..(entry.vertex_start + entry.vertex_end),
                0..1,
            );
        }
    }

    pub fn clear_buffers(&mut self) {
        self.vertices.clear();
        self.uniform_data.clear();
        self.draw_entries.clear();
    }

    fn check_and_resize_buffers(&mut self, device: &Device) {
        // resize vertices if needed
        let vertices_size = self.vertices.len() as u64 * size_of::<Vertex>() as u64;
        if vertices_size >= self.vertex_buffer.size() {
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Vertex Buffer"),
                mapped_at_creation: false,
                size: vertices_size * 2,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        }

        let uniforms_size = self.uniform_data.len() as u64;
        if uniforms_size >= self.constant_buffer.size() {
            self.constant_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Constant Buffer"),
                mapped_at_creation: false,
                size: uniforms_size * 2,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            self.constants_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.constant_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(align_to(
                                size_of::<NinjaUniformEntry>(),
                                self.min_uniform_offset,
                            ) as u64),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.projection_matrix_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(align_to(
                                size_of::<glam::Mat4>(),
                                self.min_uniform_offset,
                            ) as u64),
                        }),
                    },
                ],
                label: None,
            });
        }
    }

    pub fn set_projection_matrix(&mut self, aspect_ratio: f32) {
        self.projection_matrix =
            glam::Mat4::perspective_rh(consts::FRAC_PI_4, aspect_ratio, 0.1, 100.0)
    }

    pub fn set_buffers(&mut self, device: &Device, queue: &Queue) {
        queue.write_buffer(
            &self.projection_matrix_buffer,
            0,
            bytemuck::cast_slice(self.projection_matrix.as_ref()),
        );

        self.check_and_resize_buffers(device);

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        queue.write_buffer(&self.constant_buffer, 0, &self.uniform_data);
    }

    fn push_vertices(&mut self, vertices: &[Vertex]) -> usize {
        let len = self.vertices.len();
        self.vertices.extend_from_slice(vertices);

        len
    }

    fn push_constant_data(&mut self, data: &NinjaUniformEntry) -> usize {
        let mut slc = Vec::from(bytemuck::bytes_of(data));
        let aligned = align_to(slc.len(), self.min_uniform_offset);
        if slc.len() < aligned {
            slc.resize(aligned, 0);
        }

        let len = self.uniform_data.len();
        self.uniform_data.extend_from_slice(slc.as_slice());

        len
    }

    fn push_draw(
        &mut self,
        device: &Device,
        data: &NinjaUniformEntry,
        vertices: &[Vertex],
        texture_view: Option<&TextureView>,
        second_texture_view: Option<&TextureView>,
        sampler: &NinjaSampler,
        pipeline_settings: &NinjaPipeline,
    ) {
        let vertex_start = self.push_vertices(vertices) as u32;
        let uniform_start = self.push_constant_data(data) as u32;

        if !self.sampler_cache.contains_key(sampler) {
            self.sampler_cache.insert(
                sampler.clone(),
                device.create_sampler(&wgpu::SamplerDescriptor {
                    mag_filter: sampler.min_mag_filter,
                    min_filter: sampler.min_mag_filter,
                    mipmap_filter: sampler.mip_filter,
                    lod_min_clamp: 0.0,
                    address_mode_u: sampler.address_mode_u,
                    address_mode_v: sampler.address_mode_v,
                    ..Default::default()
                }),
            );
        }

        if !self.pipeline_cache.contains_key(pipeline_settings) {
            // todo: this could be adjusted later on at runtime
            let vertex_buffer_layout = [wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 4 * 4,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 4 * 7,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Unorm8x4Bgra,
                        offset: 4 * 7 + 4 * 2,
                        shader_location: 3,
                    },
                ],
            }];

            let (shader, pipeline_layout) = if !pipeline_settings.use_palette {
                (
                    if !pipeline_settings.use_renderfix {
                        &self.regular_shader
                    } else {
                        &self.renderfix_shader
                    },
                    &self.regular_pipeline_layout,
                )
            } else {
                (&self.palette_shader, &self.palette_pipeline_layout)
            };

            let color_target = wgpu::ColorTargetState {
                blend: Some(BlendState {
                    color: pipeline_settings.blend_col,
                    alpha: pipeline_settings.blend_alpha,
                }),
                write_mask: wgpu::ColorWrites::ALL,
                ..self.swapchain_format
            };

            let mut constants_map: HashMap<String, f64> = HashMap::new();
            constants_map.insert(
                "use_texture".to_string(),
                if pipeline_settings.use_texture {
                    1.0
                } else {
                    0.0
                },
            );
            constants_map.insert(
                "has_normal".to_string(),
                if pipeline_settings.has_normal {
                    1.0
                } else {
                    0.0
                },
            );
            constants_map.insert(
                "has_vcolor".to_string(),
                if pipeline_settings.has_vcolor {
                    1.0
                } else {
                    0.0
                },
            );

            self.pipeline_cache.insert(
                pipeline_settings.clone(),
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &vertex_buffer_layout,
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: shader,
                        entry_point: Some("fs_main"),
                        compilation_options: PipelineCompilationOptions {
                            constants: &constants_map,
                            ..Default::default()
                        },
                        targets: &[Some(color_target)],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        cull_mode: if !pipeline_settings.double_sided {
                            Some(wgpu::Face::Back)
                        } else {
                            None
                        },
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        depth_write_enabled: !pipeline_settings.use_alpha,
                        depth_compare: wgpu::CompareFunction::LessEqual,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                }),
            );
        }

        let texture = if let Some(tex) = texture_view {
            tex
        } else {
            &self.placeholder_tex
        };

        let texture_bind_group = if second_texture_view.is_none() {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: if !pipeline_settings.use_palette {
                    &self.regular_texture_bind_group_layout
                } else {
                    &self.indexed_texture_bind_group_layout
                },
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(
                            self.sampler_cache.get(sampler).unwrap(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(texture),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.placeholder_tex),
                    },
                ],
                label: None,
            })
        } else {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: if !pipeline_settings.use_palette {
                    &self.regular_texture_bind_group_layout
                } else {
                    &self.indexed_texture_bind_group_layout
                },
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(
                            self.sampler_cache.get(sampler).unwrap(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(texture),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(second_texture_view.unwrap()),
                    },
                ],
                label: None,
            })
        };

        self.draw_entries.push(NinjaDrawEntry {
            vertex_start,
            vertex_end: vertices.len() as u32,
            uniform_offset: uniform_start,
            texture_bind_group,
            pipeline_entry: pipeline_settings.clone(),
        });
    }
}

struct TextureDrawState<'a> {
    texlist: &'a Rc<NinjaTexlist<NinjaGpuTexEntry, RenderState>>,
    current_texture: Option<&'a TextureView>,
    second_texture: Option<&'a TextureView>,
}

pub struct NinjaState {
    pub draw_state: Arc<Mutex<NinjaDrawState>>,

    pub matrix_stack: NinjaMatrixStack,
    palette_colors: [Color; 48],

    use_renderfix: bool,

    chao_alpha_mode: bool,

    chao_mode: u32,
    chao_mode_texid: usize,

    poly_cache: [Option<(Vec<PolyChunk>, usize)>; 128],

    blend_src: AlphaInstruction,
    blend_dst: AlphaInstruction,

    ninja_vertex_buffer: Vec<Vertex>,

    // cwe specific extension for new accessory system, sorry i had to jam it in for the preview
    // feel free to murder it in your own project
    use_bald: i32,
    bald_influence: [f32; 3],
    bald_center: [f32; 3],
    bald_radius: f32,
    bald_clip_face: i32,
}

impl NinjaState {
    pub fn get_renderfix(&self) -> bool {
        self.use_renderfix
    }

    pub fn set_renderfix(&mut self, rf: bool) {
        self.use_renderfix = rf;
    }

    pub fn disable_bald(&mut self) {
        self.use_bald = 0;
    }

    pub fn set_bald(
        &mut self,
        influence: &glam::Vec3,
        center: &glam::Vec3,
        radius: f32,
        clip_face: i32,
    ) {
        self.use_bald = 1;
        self.bald_clip_face = clip_face;
        self.bald_influence = [influence.x, influence.y, influence.z];
        self.bald_center = [center.x, center.y, center.z];
        self.bald_radius = radius;
    }

    fn create_constant_buffer(device: &wgpu::Device, size: usize) -> wgpu::Buffer {
        let default_buf = vec![0; size];

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&default_buf),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn init(cc: &eframe::CreationContext<'_>) -> Option<NinjaState> {
        // Get the WGPU render state from the eframe creation context. This can also be retrieved
        // from `eframe::Frame` when you don't have a `CreationContext` available.
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;

        let device = &wgpu_render_state.device;

        let vertices_test = vec![Vertex::default(); 32678 / 2];

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices_test),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let regular_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shader.wgsl"))),
        });

        let renderfix_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../renderfix_shader.wgsl"
            ))),
        });

        let palette_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../palette_shader.wgsl"))),
        });

        let minimum_uniform_size =
            wgpu::Limits::downlevel_webgl2_defaults().min_storage_buffer_offset_alignment;

        // Create pipeline layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(align_to(
                            size_of::<NinjaUniformEntry>(),
                            minimum_uniform_size as usize,
                        ) as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(align_to(
                            size_of::<glam::Mat4>(),
                            minimum_uniform_size as usize,
                        ) as u64),
                    },
                    count: None,
                },
            ],
        });

        let regular_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        let indexed_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Uint,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        let constant_buffer = Self::create_constant_buffer(
            device,
            align_to(
                size_of::<NinjaUniformEntry>(),
                minimum_uniform_size as usize,
            ),
        );

        let projection_matrix_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Projection Matrix"),
            mapped_at_creation: false,
            size: minimum_uniform_size as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group
        let constants_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: constant_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: projection_matrix_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        let regular_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout, &regular_texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let palette_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout, &indexed_texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let swapchain_format: wgpu::ColorTargetState = wgpu_render_state.target_format.into();

        let placeholder_texture =
            wgpu_render_state
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });

        let draw_state = Arc::new(Mutex::new(NinjaDrawState {
            vertices: Vec::new(),
            uniform_data: Vec::new(),
            draw_entries: Vec::new(),

            projection_matrix: glam::Mat4::IDENTITY,
            vertex_buffer: vertex_buf,
            constants_bind_group,
            bind_group_layout,

            constant_buffer,
            projection_matrix_buffer,

            regular_shader,
            renderfix_shader,
            palette_shader,

            sampler_cache: HashMap::new(),
            pipeline_cache: HashMap::new(),

            swapchain_format,

            placeholder_tex: placeholder_texture
                .create_view(&wgpu::TextureViewDescriptor::default()),

            regular_pipeline_layout,
            palette_pipeline_layout,

            regular_texture_bind_group_layout,
            indexed_texture_bind_group_layout,

            min_uniform_offset: minimum_uniform_size as usize,
        }));

        Some(Self {
            draw_state,

            ninja_vertex_buffer: vec![Vertex::default(); 32768],

            use_renderfix: true,

            chao_alpha_mode: false,
            chao_mode: 0,
            chao_mode_texid: 0,

            poly_cache: [const { None }; 128],
            blend_src: AlphaInstruction::SourceAlpha,
            blend_dst: AlphaInstruction::One,

            matrix_stack: NinjaMatrixStack::init(),

            palette_colors: [Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            }; 48],

            use_bald: 0,
            bald_clip_face: 1,
            bald_center: [0.0, 0.0, 0.0],
            bald_influence: [0.0, 0.0, 0.0],
            bald_radius: 1.0,
        })
    }

    fn parse_poly_chunk(
        &mut self,
        device: &Device,
        poly_chunk_list: &Vec<PolyChunk>,
        tex_draw: &mut TextureDrawState,
        pipeline_settings: &mut NinjaPipeline,
        sampler: &mut NinjaSampler,
        uniform_entry: &mut NinjaUniformEntry,
    ) {
        for (i, p) in poly_chunk_list.iter().enumerate() {
            match p {
                PolyChunk::BitsCachePolygonList(v) => {
                    self.poly_cache[*v as usize] = Some((poly_chunk_list.clone(), i + 1));
                    return;
                }
                PolyChunk::BitsDrawPolygonList(v) => {
                    let (vec, index) = self.poly_cache[*v as usize].as_ref().unwrap();
                    self.parse_poly_chunk(
                        device,
                        &vec.iter().skip(*index).cloned().collect(),
                        tex_draw,
                        pipeline_settings,
                        sampler,
                        uniform_entry,
                    );
                    self.poly_cache[*v as usize] = None;
                }
                PolyChunk::BitsBlendAlpha {
                    source_alpha,
                    destination_alpha,
                } => {
                    self.blend_src = *source_alpha;
                    self.blend_dst = *destination_alpha;
                }
                PolyChunk::BitsSpecularExponent(v) => {
                    uniform_entry.specular_exponent = *v as f32;
                }
                PolyChunk::TinyTextureID {
                    mipmap_d_adjust: _,
                    clamp_u,
                    clamp_v,
                    flip_u,
                    flip_v,
                    texture_id,
                    super_sample: _,
                    filter_mode,
                } => {
                    if self.chao_mode == 1 || self.chao_mode == 3 {
                        continue;
                    }

                    if *clamp_u {
                        sampler.address_mode_u = wgpu::AddressMode::ClampToEdge;
                    } else if *flip_u {
                        sampler.address_mode_u = wgpu::AddressMode::MirrorRepeat;
                    } else {
                        sampler.address_mode_u = wgpu::AddressMode::Repeat;
                    }

                    if *clamp_v {
                        sampler.address_mode_v = wgpu::AddressMode::ClampToEdge;
                    } else if *flip_v {
                        sampler.address_mode_v = wgpu::AddressMode::MirrorRepeat;
                    } else {
                        sampler.address_mode_v = wgpu::AddressMode::Repeat;
                    }

                    if self.get_renderfix() {
                        (sampler.min_mag_filter, sampler.mip_filter) = match *filter_mode {
                            super::FilterMode::Bilinear => {
                                (wgpu::FilterMode::Linear, wgpu::FilterMode::Nearest)
                            }
                            super::FilterMode::Trilinear => {
                                (wgpu::FilterMode::Linear, wgpu::FilterMode::Linear)
                            }
                            _ => (wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest),
                        }
                    }

                    let tex_id = *texture_id as usize;
                    if tex_id < tex_draw.texlist.textures.len() {
                        pipeline_settings.use_texture = true;

                        tex_draw.current_texture =
                            Some(&tex_draw.texlist.gpu_textures[tex_id].texture_view);

                        let tex_entry = &tex_draw.texlist.textures[tex_id];
                        let bank = tex_entry.bank;
                        if bank >= 0 {
                            uniform_entry.palette_index = 16 * bank as i32;
                            pipeline_settings.use_palette = true;
                        } else {
                            uniform_entry.palette_index = -1;
                            pipeline_settings.use_palette = false;
                        }

                        // todo: width != height?
                        uniform_entry.texture_size = tex_entry.width as f32;
                    }
                }
                PolyChunk::Material {
                    source_alpha,
                    destination_alpha,
                    diffuse,
                    ambient,
                    specular,
                } => {
                    self.blend_src = *source_alpha;
                    self.blend_dst = *destination_alpha;

                    if let Some(ambient_color) = ambient {
                        uniform_entry.ambient_color[0] = ambient_color.r as f32 / 255.0;
                        uniform_entry.ambient_color[1] = ambient_color.g as f32 / 255.0;
                        uniform_entry.ambient_color[2] = ambient_color.b as f32 / 255.0;
                    }

                    if let Some(specular_color) = specular {
                        uniform_entry.specular_color[0] = specular_color.r as f32 / 255.0;
                        uniform_entry.specular_color[1] = specular_color.g as f32 / 255.0;
                        uniform_entry.specular_color[2] = specular_color.b as f32 / 255.0;

                        uniform_entry.specular_exponent = specular_color.a as f32;
                    }

                    if let Some(diff_color) = diffuse {
                        uniform_entry.diffuse_color[0] = diff_color.r as f32 / 255.0;
                        uniform_entry.diffuse_color[1] = diff_color.g as f32 / 255.0;
                        uniform_entry.diffuse_color[2] = diff_color.b as f32 / 255.0;
                        uniform_entry.diffuse_color[3] = diff_color.a as f32 / 255.0;
                    }
                }
                PolyChunk::Strip {
                    flags,
                    user_flags: _,
                    strips,
                } => {
                    let mut vertices = Vec::new();

                    for s in strips {
                        let len = s.indices.len();

                        for i in 0..len {
                            let index = s.indices[i] as usize;

                            let mut vert = self.ninja_vertex_buffer[index];
                            match &s.uvs {
                                Some(uvs) => vert._tex_coord = [uvs[i].x, uvs[i].y],
                                None => vert._tex_coord = [0.0, 0.0],
                            };

                            if i == 0 {
                                if s.reversed && vertices.is_empty() {
                                    vertices.push(vert);
                                } else if !vertices.is_empty() {
                                    if (!s.reversed && (vertices.len() % 2) == 1)
                                        || (s.reversed && (vertices.len() % 2) == 0)
                                    {
                                        vertices.push(vertices[vertices.len() - 1]);
                                    }

                                    vertices.push(vertices[vertices.len() - 1]);
                                    vertices.push(vert);
                                }
                            }

                            vertices.push(vert);
                        }
                    }

                    pipeline_settings.blend_col.src_factor = self.blend_src.into();
                    pipeline_settings.blend_col.dst_factor = self.blend_dst.into();
                    pipeline_settings.blend_col.operation = wgpu::BlendOperation::Add;
                    pipeline_settings.blend_alpha.src_factor = self.blend_src.into();
                    pipeline_settings.blend_alpha.dst_factor = self.blend_dst.into();

                    pipeline_settings.use_texture = strips.iter().any(|str| str.uvs.is_some());

                    uniform_entry.ignore_light = (flags & 1).into();
                    uniform_entry.ignore_specular = (flags & 2).into();
                    uniform_entry.ignore_ambient = (flags & 4).into();

                    pipeline_settings.use_alpha = (flags & 8) != 0;
                    pipeline_settings.double_sided = (flags & 16) != 0;

                    uniform_entry.use_env = ((flags & 0x40) != 0).into();

                    if self.chao_alpha_mode {
                        // chcnk ignores blend modes
                        pipeline_settings.blend_col.src_factor = BlendFactor::SrcAlpha;
                        pipeline_settings.blend_col.dst_factor = BlendFactor::OneMinusSrcAlpha;
                        pipeline_settings.blend_col.operation = wgpu::BlendOperation::Add;
                        pipeline_settings.blend_alpha.src_factor =
                            pipeline_settings.blend_col.src_factor;
                        pipeline_settings.blend_alpha.dst_factor =
                            pipeline_settings.blend_col.dst_factor;

                        // force z write enabled
                        pipeline_settings.use_alpha = false;
                    }

                    self.draw_state.lock().push_draw(
                        device,
                        uniform_entry,
                        vertices.as_slice(),
                        tex_draw.current_texture,
                        tex_draw.second_texture,
                        sampler,
                        pipeline_settings,
                    );
                }
                _ => continue,
            }
        }
    }

    pub fn draw_mdl(
        &mut self,
        device: &Device,
        mdl: &ChunkModel,
        ref texlist: Rc<NinjaTexlist<NinjaGpuTexEntry, RenderState>>,
    ) {
        let mut pipeline_settings = NinjaPipeline {
            use_renderfix: self.use_renderfix,
            use_palette: false,
            use_alpha: false,
            use_texture: false,
            has_normal: false,
            has_vcolor: false,
            double_sided: false,
            blend_col: BlendComponent {
                operation: wgpu::BlendOperation::Add,
                ..Default::default()
            },
            blend_alpha: BlendComponent::default(),
        };

        let mvp = self.matrix_stack.get();
        let mvp_inv_trans = mvp.inverse().transpose();

        for x in &mdl.vertex_list {
            let buff_start = x.index_offset as usize;
            if let Some(ninja_flags) = &x.ninja_flags {
                let weight_status = x.weight_status.unwrap();
                let vert_flags = ninja_flags.iter().zip(x.vertices.iter());
                if let Some(normals) = &x.normals {
                    pipeline_settings.has_normal = true;

                    let vert_norm_flags = vert_flags.zip(normals.iter());
                    vert_norm_flags.for_each(|((nf, p), n)| {
                        let index = (nf & 0xFFFF) as usize;
                        let vert = &mut self.ninja_vertex_buffer[buff_start + index];

                        let weight = (nf >> 16) as f32 / 255.0;
                        let mut transformed_position = mvp * glam::vec4(p.x, p.y, p.z, 1.0);
                        let mut transformed_normal = mvp_inv_trans * glam::vec4(n.x, n.y, n.z, 0.0);

                        transformed_position.x *= weight;
                        transformed_position.y *= weight;
                        transformed_position.z *= weight;

                        transformed_normal.x *= weight;
                        transformed_normal.y *= weight;
                        transformed_normal.z *= weight;

                        match weight_status {
                            WeightStatus::Start => {
                                *vert = Vertex {
                                    _pos: transformed_position.to_array(),
                                    _normal: [
                                        transformed_normal.x,
                                        transformed_normal.y,
                                        transformed_normal.z,
                                    ],
                                    ..Default::default()
                                }
                            }
                            WeightStatus::Middle | WeightStatus::End => {
                                vert._pos[0] += transformed_position.x;
                                vert._pos[1] += transformed_position.y;
                                vert._pos[2] += transformed_position.z;

                                vert._normal[0] += transformed_normal.x;
                                vert._normal[1] += transformed_normal.y;
                                vert._normal[2] += transformed_normal.z;
                            }
                        };
                    });
                } else {
                    pipeline_settings.has_normal = false;

                    vert_flags.for_each(|(nf, p)| {
                        let index = (nf & 0xFFFF) as usize;
                        let vert = &mut self.ninja_vertex_buffer[buff_start + index];

                        let weight = (nf >> 16) as f32 / 255.0;
                        let mut transformed_position = mvp * glam::vec4(p.x, p.y, p.z, 1.0);

                        transformed_position.x *= weight;
                        transformed_position.y *= weight;
                        transformed_position.z *= weight;

                        match weight_status {
                            WeightStatus::Start => {
                                *vert = Vertex {
                                    _pos: transformed_position.to_array(),
                                    ..Default::default()
                                }
                            }
                            WeightStatus::Middle | WeightStatus::End => {
                                vert._pos[0] += transformed_position.x;
                                vert._pos[1] += transformed_position.y;
                                vert._pos[2] += transformed_position.z;
                            }
                        };
                    });
                }

                if let Some(diffuse) = &x.diffuse {
                    pipeline_settings.has_vcolor = true;

                    let vert_diff_flags = ninja_flags.iter().zip(diffuse.iter());
                    vert_diff_flags.for_each(|((nf, p))| {
                        let index = (nf & 0xFFFF) as usize;
                        self.ninja_vertex_buffer[buff_start + index]._color = *p;
                    });
                } else {
                    pipeline_settings.has_vcolor = false;
                }
            } else {
                if let Some(normals) = &x.normals {
                    pipeline_settings.has_normal = true;

                    let pos_norm_iter = x.vertices.iter().zip(normals.iter());
                    pos_norm_iter.enumerate().for_each(|(index, (p, n))| {
                        let vert = &mut self.ninja_vertex_buffer[buff_start + index];

                        let transformed_position = mvp * glam::vec4(p.x, p.y, p.z, 1.0);
                        let transformed_normal = mvp_inv_trans * glam::vec4(n.x, n.y, n.z, 0.0);

                        *vert = Vertex {
                            _pos: transformed_position.to_array(),
                            _normal: [
                                transformed_normal.x,
                                transformed_normal.y,
                                transformed_normal.z,
                            ],
                            ..Default::default()
                        };
                    });
                } else {
                    pipeline_settings.has_normal = false;

                    x.vertices.iter().enumerate().for_each(|(index, p)| {
                        let vert = &mut self.ninja_vertex_buffer[buff_start + index];

                        let transformed_position = mvp * glam::vec4(p.x, p.y, p.z, 1.0);

                        *vert = Vertex {
                            _pos: transformed_position.to_array(),
                            ..Default::default()
                        };
                    });
                }

                if let Some(colors) = &x.diffuse {
                    pipeline_settings.has_vcolor = true;

                    colors.iter().enumerate().for_each(|(index, p)| {
                        self.ninja_vertex_buffer[buff_start + index]._color = *p;
                    });
                } else {
                    pipeline_settings.has_vcolor = false;
                }
            }
        }

        let mut tex_draw = TextureDrawState {
            texlist,
            current_texture: None,
            second_texture: None,
        };

        let mut sampler: NinjaSampler = NinjaSampler {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            min_mag_filter: wgpu::FilterMode::Linear,
            mip_filter: wgpu::FilterMode::Nearest,
        };

        if self.chao_mode == 1 || self.chao_mode == 3 {
            tex_draw.current_texture =
                Some(&texlist.gpu_textures[self.chao_mode_texid].texture_view);
        } else if self.chao_mode == 4 {
            tex_draw.second_texture =
                Some(&texlist.gpu_textures[self.chao_mode_texid].texture_view);
        }

        let mut uniform_entry = NinjaUniformEntry {
            mvp: mvp.inverse().to_cols_array(),
            inverse_transpose_modelview: mvp.to_cols_array(),
            diffuse_color: [1.0, 1.0, 1.0, 1.0],
            palette_colors: self.palette_colors.map(|c| {
                [
                    c.r as f32 / 255.0,
                    c.g as f32 / 255.0,
                    c.b as f32 / 255.0,
                    c.a as f32 / 255.0,
                ]
            }),
            palette_index: -1,
            texture_size: 0.0,
            chao_mode: self.chao_mode,
            use_env: 0,
            light_direction: [0.0, 1.0, 0.0],
            use_bald: self.use_bald,
            bald_influence: self.bald_influence,
            bald_center: self.bald_center,
            bald_radius: self.bald_radius,
            bald_clip_face: self.bald_clip_face,
            pad: 0.0,
            ignore_light: 0,
            ignore_ambient: 0,
            ignore_specular: 0,
            ambient_color: [127.0 / 255.0, 127.0 / 255.0, 127.0 / 255.0],
            specular_exponent: 11.0,
            specular_color: [1.0, 1.0, 1.0],
        };

        self.parse_poly_chunk(
            device,
            mdl.poly_list.as_ref().unwrap(),
            &mut tex_draw,
            &mut pipeline_settings,
            &mut sampler,
            &mut uniform_entry,
        );
    }

    pub fn draw_motion(
        &mut self,
        device: &Device,
        obj: &NinjaChunkObject,
        motion: &NinjaMotion,
        ref texlist: Rc<NinjaTexlist<NinjaGpuTexEntry, RenderState>>,
        frame: f32,
        node_index: &mut usize,
    ) {
        self.matrix_stack.push();

        if let Some(mot_pos) = motion.get_motion_pos(*node_index, frame) {
            self.matrix_stack.translate(mot_pos.x, mot_pos.y, mot_pos.z);
        } else {
            self.matrix_stack.translate(obj.pos.x, obj.pos.y, obj.pos.z);
        }

        if let Some(mot_ang) = motion.get_motion_ang(*node_index, frame) {
            self.matrix_stack.rotate(&mot_ang);
        } else {
            self.matrix_stack.rotate(&obj.ang);
        }

        if let Some(mot_scl) = motion.get_motion_scl(*node_index, frame) {
            self.matrix_stack.scale(mot_scl.x, mot_scl.y, mot_scl.z);
        } else {
            self.matrix_stack.scale(obj.scl.x, obj.scl.y, obj.scl.z);
        }

        if let Some(ref mdl) = obj.model {
            self.draw_mdl(device, mdl, texlist.clone());
        }

        *node_index += 1;

        if let Some(ref child) = obj.child {
            self.draw_motion(device, child, motion, texlist.clone(), frame, node_index);
        }

        self.matrix_stack.pop();

        if let Some(ref sibling) = obj.sibling {
            self.draw_motion(device, sibling, motion, texlist.clone(), frame, node_index);
        }
    }

    pub fn draw(
        &mut self,
        device: &Device,
        obj: &NinjaChunkObject,
        ref texlist: Rc<NinjaTexlist<NinjaGpuTexEntry, RenderState>>,
    ) {
        self.matrix_stack.push();

        self.matrix_stack.translate(obj.pos.x, obj.pos.y, obj.pos.z);
        self.matrix_stack.rotate(&obj.ang);
        self.matrix_stack.scale(obj.scl.x, obj.scl.y, obj.scl.z);

        if let Some(ref mdl) = obj.model {
            self.draw_mdl(device, mdl, texlist.clone());
        }

        if let Some(ref child) = obj.child {
            self.draw(device, child, texlist.clone());
        }

        self.matrix_stack.pop();

        if let Some(ref sibling) = obj.sibling {
            self.draw(device, sibling, texlist.clone());
        }
    }

    pub fn set_colors(&mut self, colors: &[Color]) {
        self.palette_colors = colors.try_into().unwrap();
    }

    pub fn set_chao_alpha_mode(&mut self, enabled: bool) {
        self.chao_alpha_mode = enabled;
    }

    pub fn set_chao_mode(&mut self, mode: u32, texid: usize) {
        self.chao_mode = mode;
        self.chao_mode_texid = texid;
    }
}

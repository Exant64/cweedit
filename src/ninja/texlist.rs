use egui_wgpu::RenderState;

use super::texture::gvm::NinjaGpuTex;

pub struct NinjaGpuTexEntry {
    pub _texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
}

impl NinjaGpuTex<RenderState> for NinjaGpuTexEntry {
    fn create_texture(
        render_state: &RenderState,
        tex_entry: &super::texture::gvm::NinjaTex,
    ) -> Self {
        let texture_extent = wgpu::Extent3d {
            width: tex_entry.width as u32,
            height: tex_entry.height as u32,
            depth_or_array_layers: 1,
        };

        let wgpu_format = tex_entry.real_pixel_format.get_wgpu_format();

        let texture = render_state
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: texture_extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu_format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

        let texture_view: wgpu::TextureView =
            texture.create_view(&wgpu::TextureViewDescriptor::default());

        let block_dimensions = wgpu_format.block_dimensions();
        render_state.queue.write_texture(
            texture.as_image_copy(),
            &tex_entry.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(
                    tex_entry.width as u32 / block_dimensions.0
                        * wgpu_format.block_copy_size(None).unwrap(),
                ),
                rows_per_image: None,
            },
            texture_extent,
        );

        NinjaGpuTexEntry {
            _texture: texture,
            texture_view,
        }
    }
}

use crate::renderer::Graphics;

#[derive(Debug, Clone)]
pub struct HdrTextures {
    pub sampler: wgpu::Sampler,

    color: wgpu::Texture,
    color_view: wgpu::TextureView,

    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,

    bind_group_layout: wgpu::BindGroupLayout,
    color_bind_group: wgpu::BindGroup,

    physical_size: [u32; 2],
}

impl HdrTextures {
    pub fn new(gfx: &Graphics, physical_size: [u32; 2]) -> Self {
        let sampler = create_hdr_sampler(gfx);

        let color = create_hdr_color(gfx, physical_size[0], physical_size[1]);
        let color_view = create_hdr_color_view(&color);

        let depth = create_hdr_depth(gfx, physical_size[0], physical_size[1]);
        let depth_view = create_hdr_depth_view(&depth);

        let bind_group_layout = gfx
            .start_bind_group_layout()
            .label("hdr_bind_group_layout")
            .texture_filterable_binding(
                0,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureViewDimension::D2,
                false,
            )
            .sampler_binding(
                1,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::SamplerBindingType::Filtering,
            )
            .finish();

        let bind_group = create_hdr_bind_group(gfx, &bind_group_layout, &color_view, &sampler);

        Self {
            sampler,
            color,
            color_view,
            depth,
            depth_view,
            bind_group_layout,
            color_bind_group: bind_group,
            physical_size,
        }
    }

    pub fn resize(&mut self, gfx: &Graphics, physical_size: [u32; 2]) {
        if self.physical_size == physical_size {
            return;
        }

        self.physical_size = physical_size;

        log::info!(
            "Resizing render stack viewport to ({}, {})",
            physical_size[0],
            physical_size[1]
        );

        self.color = create_hdr_color(gfx, physical_size[0], physical_size[1]);
        self.color_view = create_hdr_color_view(&self.color);
        self.color_bind_group = create_hdr_bind_group(
            gfx,
            &self.bind_group_layout,
            &self.color_view,
            &self.sampler,
        );

        self.depth = create_hdr_depth(gfx, physical_size[0], physical_size[1]);
        self.depth_view = create_hdr_depth_view(&self.depth);
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.color_view
    }

    pub fn color_bind_group(&self) -> &wgpu::BindGroup {
        &self.color_bind_group
    }

    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// Size of hdr render attachments.
    pub fn _physical_size(&self) -> [u32; 2] {
        self.physical_size
    }
}

fn create_hdr_color(gfx: &Graphics, width: u32, height: u32) -> wgpu::Texture {
    let texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hdr_color_attachment"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: gfx.hdr_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    texture
}

fn create_hdr_depth(gfx: &Graphics, width: u32, height: u32) -> wgpu::Texture {
    let texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hdr_depth_attachment"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture
}

fn create_hdr_color_view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("hdr_color_attachment_view"),
        ..Default::default()
    })
}

fn create_hdr_depth_view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("hdr_depth_attachment_view"),
        ..Default::default()
    })
}

fn create_hdr_sampler(gfx: &Graphics) -> wgpu::Sampler {
    gfx.device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("hdr_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    })
}

fn create_hdr_bind_group(
    gfx: &Graphics,
    hdr_layout: &wgpu::BindGroupLayout,
    hdr_color_view: &wgpu::TextureView,
    hdr_sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("hdr_bind_group"),
        layout: &hdr_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&hdr_color_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&hdr_sampler),
            },
        ],
    })
}

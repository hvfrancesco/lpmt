//! Shared media textures for every output.
//!
//! A show has one copy of each decoded media asset no matter how many output
//! windows present it. Outputs only own their surface, pipelines and uniform
//! buffer; the textures and bind groups live here and are referenced by every
//! renderer.

use std::collections::HashMap;

use anyhow::Result;
use lpmt_core::model::{MediaId, SourceSpec};
use lpmt_media::DecodedImage;

use crate::gpu::Gpu;

const CHECKER_SIZE: u32 = 512;

struct TextureEntry {
    texture: wgpu::Texture,
    _view: wgpu::TextureView,
    width: u32,
    height: u32,
    bind_group: wgpu::BindGroup,
}

struct ImageData<'a> {
    width: u32,
    height: u32,
    rgba: &'a [u8],
}

/// Device-wide texture cache keyed by stable `MediaId`.
pub struct MediaCache {
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    checker: TextureEntry,
    media: HashMap<MediaId, TextureEntry>,
}

impl MediaCache {
    pub fn new(gpu: &Gpu) -> Result<Self> {
        let texture_bind_group_layout =
            gpu.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("lpmt-source-bind-group-layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let sampler = gpu.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lpmt-source-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let checker_pixels = create_checker_pixels();
        let checker = create_texture_entry(
            gpu,
            &texture_bind_group_layout,
            &sampler,
            "lpmt-checker",
            ImageData {
                width: CHECKER_SIZE,
                height: CHECKER_SIZE,
                rgba: &checker_pixels,
            },
        )?;

        Ok(Self {
            texture_bind_group_layout,
            sampler,
            checker,
            media: HashMap::new(),
        })
    }

    /// Layout shared with every output pipeline.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    pub fn has_media(&self, id: &MediaId) -> bool {
        self.media.contains_key(id)
    }

    pub fn clear(&mut self) {
        self.media.clear();
    }

    /// Upload a decoded still image. Re-uploading the same media ID replaces
    /// the texture and bind group, which is used by "reload media".
    pub fn upload_image(
        &mut self,
        gpu: &Gpu,
        media_id: MediaId,
        image: &DecodedImage,
    ) -> Result<()> {
        let label = format!("lpmt-media-{media_id}");
        let entry = create_texture_entry(
            gpu,
            &self.texture_bind_group_layout,
            &self.sampler,
            &label,
            ImageData {
                width: image.width,
                height: image.height,
                rgba: &image.rgba,
            },
        )?;
        self.media.insert(media_id, entry);
        Ok(())
    }

    /// Update a media texture in place when the frame size is unchanged. This
    /// is the video fast path: it avoids recreating the texture and bind group
    /// for every decoded frame.
    pub fn update_image(
        &mut self,
        gpu: &Gpu,
        media_id: MediaId,
        image: &DecodedImage,
    ) -> Result<()> {
        if let Some(entry) = self.media.get(&media_id) {
            if entry.width == image.width && entry.height == image.height {
                write_image(
                    gpu,
                    &entry.texture,
                    ImageData {
                        width: image.width,
                        height: image.height,
                        rgba: &image.rgba,
                    },
                )?;
                return Ok(());
            }
        }

        self.upload_image(gpu, media_id, image)
    }

    /// Bind group for a layer source, falling back to the checker test card.
    pub(crate) fn bind_group(&self, source: &SourceSpec) -> &wgpu::BindGroup {
        match source {
            SourceSpec::Image { media_id } | SourceSpec::Video { media_id } => self
                .media
                .get(media_id)
                .map_or(&self.checker.bind_group, |entry| &entry.bind_group),
            SourceSpec::TestPattern { .. } => &self.checker.bind_group,
        }
    }
}

fn create_texture_entry(
    gpu: &Gpu,
    bind_group_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    label: &str,
    image: ImageData<'_>,
) -> Result<TextureEntry> {
    let ImageData {
        width,
        height,
        rgba,
    } = image;
    anyhow::ensure!(width > 0 && height > 0, "image has an empty dimension");
    anyhow::ensure!(
        rgba.len() == width as usize * height as usize * 4,
        "image buffer does not match {width}x{height}"
    );

    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = gpu.device().create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    write_image(
        gpu,
        &texture,
        ImageData {
            width,
            height,
            rgba,
        },
    )?;

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });

    Ok(TextureEntry {
        texture,
        _view: view,
        width,
        height,
        bind_group,
    })
}

fn write_image(gpu: &Gpu, texture: &wgpu::Texture, image: ImageData<'_>) -> Result<()> {
    let ImageData {
        width,
        height,
        rgba,
    } = image;
    anyhow::ensure!(width > 0 && height > 0, "image has an empty dimension");
    anyhow::ensure!(
        rgba.len() == width as usize * height as usize * 4,
        "image buffer does not match {width}x{height}"
    );

    gpu.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    Ok(())
}

fn create_checker_pixels() -> Vec<u8> {
    let size = CHECKER_SIZE;
    let mut pixels = vec![0_u8; (size * size * 4) as usize];
    let tints: [[u8; 3]; 4] = [
        [235, 72, 64],
        [76, 214, 112],
        [72, 128, 245],
        [242, 206, 70],
    ];

    for y in 0..size {
        for x in 0..size {
            let cell = (x / 64 + y / 64) & 1;
            let quadrant = (usize::from(x >= size / 2)) + 2 * usize::from(y >= size / 2);
            let tint = tints[quadrant];
            let value = if cell == 0 { 1.0 } else { 0.42 };
            let index = ((y * size + x) * 4) as usize;
            pixels[index] = (f32::from(tint[0]) * value) as u8;
            pixels[index + 1] = (f32::from(tint[1]) * value) as u8;
            pixels[index + 2] = (f32::from(tint[2]) * value) as u8;
            pixels[index + 3] = 255;
        }
    }

    pixels
}

//! Shared media textures for every output.
//!
//! A show has one copy of each decoded media asset no matter how many output
//! windows present it. Outputs only own their surface, pipelines and uniform
//! buffer; the textures and bind groups live here and are referenced by every
//! renderer.

use std::collections::HashMap;

use anyhow::Result;
use lpmt_core::model::{MediaId, SourceSpec, TestPattern};
use lpmt_media::DecodedImage;

use crate::gpu::Gpu;

const PATTERN_SIZE: u32 = 512;

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
    test_patterns: HashMap<TestPattern, TextureEntry>,
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

        let mut test_patterns = HashMap::new();
        for pattern in TestPattern::ALL {
            let pixels = create_pattern_pixels(pattern);
            let entry = create_texture_entry(
                gpu,
                &texture_bind_group_layout,
                &sampler,
                &format!("lpmt-pattern-{pattern:?}"),
                ImageData {
                    width: PATTERN_SIZE,
                    height: PATTERN_SIZE,
                    rgba: &pixels,
                },
            )?;
            test_patterns.insert(pattern, entry);
        }

        Ok(Self {
            texture_bind_group_layout,
            sampler,
            test_patterns,
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
                .map_or_else(|| self.checker_bind_group(), |entry| &entry.bind_group),
            SourceSpec::TestPattern { pattern } => {
                let entry = self
                    .test_patterns
                    .get(pattern)
                    .or_else(|| self.test_patterns.get(&TestPattern::Checker))
                    .expect("the checker test pattern is always generated");
                &entry.bind_group
            }
        }
    }

    fn checker_bind_group(&self) -> &wgpu::BindGroup {
        &self
            .test_patterns
            .get(&TestPattern::Checker)
            .expect("the checker test pattern is always generated")
            .bind_group
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

fn create_pattern_pixels(pattern: TestPattern) -> Vec<u8> {
    match pattern {
        TestPattern::Checker => create_checker_pixels(),
        TestPattern::Grid => create_grid_pixels(false),
        TestPattern::ColorBars => create_color_bars_pixels(),
        TestPattern::Grayscale => create_grayscale_pixels(),
        TestPattern::Convergence => create_grid_pixels(true),
    }
}

fn create_checker_pixels() -> Vec<u8> {
    let size = PATTERN_SIZE;
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
            set_pixel(
                &mut pixels,
                size,
                x,
                y,
                tint.map(|channel| (f32::from(channel) * value) as u8),
            );
        }
    }

    pixels
}

fn create_grid_pixels(convergence: bool) -> Vec<u8> {
    let size = PATTERN_SIZE;
    let mut pixels = vec![0_u8; (size * size * 4) as usize];
    let grid = [196, 202, 214];
    let axis = [255, 214, 92];
    let spacing = 32;

    for y in 0..size {
        for x in 0..size {
            let on_grid = x % spacing == 0 || y % spacing == 0;
            let on_axis = x == size / 2 || y == size / 2;
            let color = if on_axis {
                axis
            } else if on_grid {
                grid
            } else {
                [10, 11, 15]
            };
            set_pixel(&mut pixels, size, x, y, color);
        }
    }

    if convergence {
        for y in (0..size).step_by(spacing as usize) {
            for x in (0..size).step_by(spacing as usize) {
                if x % 64 != 0 || y % 64 != 0 {
                    continue;
                }
                fill_rect(
                    &mut pixels,
                    size,
                    x.saturating_sub(4),
                    y.saturating_sub(1),
                    2,
                    2,
                    [255, 48, 48],
                );
                fill_rect(
                    &mut pixels,
                    size,
                    x.saturating_sub(1),
                    y.saturating_sub(1),
                    2,
                    2,
                    [48, 255, 96],
                );
                fill_rect(
                    &mut pixels,
                    size,
                    x + 2,
                    y.saturating_sub(1),
                    2,
                    2,
                    [64, 128, 255],
                );
            }
        }
    }

    pixels
}

fn create_color_bars_pixels() -> Vec<u8> {
    let size = PATTERN_SIZE;
    let mut pixels = vec![0_u8; (size * size * 4) as usize];
    let bars = [
        [235, 235, 235],
        [235, 235, 24],
        [24, 235, 235],
        [24, 235, 24],
        [235, 24, 235],
        [235, 24, 24],
        [24, 24, 235],
        [24, 24, 24],
    ];
    let split = size * 3 / 4;

    for y in 0..size {
        for x in 0..size {
            let color = if y < split {
                bars[(x * bars.len() as u32 / size) as usize]
            } else {
                let step = (x * 16 / size).min(15) as u8;
                let value = step * 17;
                [value, value, value]
            };
            set_pixel(&mut pixels, size, x, y, color);
        }
    }

    fill_rect(
        &mut pixels,
        size,
        0,
        split.saturating_sub(2),
        size,
        2,
        [0, 0, 0],
    );
    pixels
}

fn create_grayscale_pixels() -> Vec<u8> {
    let size = PATTERN_SIZE;
    let mut pixels = vec![0_u8; (size * size * 4) as usize];
    let split = size * 4 / 5;

    for y in 0..size {
        for x in 0..size {
            let value = if y < split {
                let step = (x * 16 / size).min(15) as u8;
                step * 17
            } else {
                (x * 255 / (size - 1)) as u8
            };
            set_pixel(&mut pixels, size, x, y, [value, value, value]);
        }
    }

    pixels
}

fn set_pixel(pixels: &mut [u8], size: u32, x: u32, y: u32, color: [u8; 3]) {
    if x >= size || y >= size {
        return;
    }
    let index = ((y * size + x) * 4) as usize;
    pixels[index] = color[0];
    pixels[index + 1] = color[1];
    pixels[index + 2] = color[2];
    pixels[index + 3] = 255;
}

fn fill_rect(
    pixels: &mut [u8],
    size: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: [u8; 3],
) {
    for row in y..(y + height).min(size) {
        for column in x..(x + width).min(size) {
            set_pixel(pixels, size, column, row, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(pixels: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * size + x) * 4) as usize;
        pixels[index..index + 4].try_into().unwrap()
    }

    #[test]
    fn every_test_pattern_is_opaque_and_full_size() {
        for pattern in TestPattern::ALL {
            let pixels = create_pattern_pixels(pattern);
            assert_eq!(pixels.len(), (PATTERN_SIZE * PATTERN_SIZE * 4) as usize);
            assert!(pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[3] == 255));
        }
    }

    #[test]
    fn grayscale_pattern_has_black_and_white_extremes() {
        let pixels = create_pattern_pixels(TestPattern::Grayscale);
        assert_eq!(pixel(&pixels, PATTERN_SIZE, 0, 0), [0, 0, 0, 255]);
        assert_eq!(
            pixel(&pixels, PATTERN_SIZE, PATTERN_SIZE - 1, 0),
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn color_bars_keep_white_and_black_bars_in_the_top_row() {
        let pixels = create_pattern_pixels(TestPattern::ColorBars);
        assert_eq!(pixel(&pixels, PATTERN_SIZE, 8, 8), [235, 235, 235, 255]);
        assert_eq!(
            pixel(&pixels, PATTERN_SIZE, PATTERN_SIZE - 8, 8),
            [24, 24, 24, 255]
        );
    }
}

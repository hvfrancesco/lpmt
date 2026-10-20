//! A window-backed output renderer.
//!
//! The renderer consumes a `Stage` and an output view, not a fixed window-sized
//! framebuffer. Content textures are uploaded by the application from decoded
//! media; the renderer owns the GPU-side cache and bind groups.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use lpmt_core::model::{BlendMode, MediaId, Rect, SourceSpec, Stage};
use lpmt_media::DecodedImage;
use winit::window::Window;

use crate::gpu::Gpu;

const CHECKER_SIZE: u32 = 512;
const MAX_LAYER_SLOTS: usize = 256;

const SHADER: &str = r#"
struct Uniforms {
    // Maps stage space back to the content's unit square.
    inv_surface: mat4x4<f32>,
    resolution: vec2<f32>,
    _padding: vec2<f32>,
    // Output view within the stage: x, y, width, height.
    view: vec4<f32>,
    opacity: f32,
    _padding2: f32,
    _padding3: f32,
    _padding4: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var source_texture: texture_2d<f32>;
@group(1) @binding(1) var source_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let output_uv = frag_coord.xy / uniforms.resolution;
    let stage_uv = uniforms.view.xy + output_uv * uniforms.view.zw;

    let source_h = uniforms.inv_surface * vec4<f32>(stage_uv, 0.0, 1.0);
    let source_uv = source_h.xy / source_h.w;

    if (any(source_uv < vec2<f32>(0.0)) || any(source_uv > vec2<f32>(1.0))) {
        discard;
    }

    let color = textureSampleLevel(source_texture, source_sampler, source_uv, 0.0);
    let alpha = color.a * uniforms.opacity;
    // Premultiplied output lets the fixed-function blend modes implement
    // Normal, Add, Screen and Multiply without reading the destination.
    return vec4<f32>(color.rgb * alpha, alpha);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    inv_surface: [f32; 16],
    resolution: [f32; 2],
    _padding: [f32; 2],
    view: [f32; 4],
    opacity: f32,
    _padding2: [f32; 3],
}

struct DrawItem {
    offset: u32,
    uniforms: Uniforms,
    blend_index: usize,
    bind_group: wgpu::BindGroup,
}

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

pub struct OutputRenderer {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipelines: [wgpu::RenderPipeline; 4],
    uniform_buffer: wgpu::Buffer,
    uniform_stride: u64,
    uniform_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    checker: TextureEntry,
    media: HashMap<MediaId, TextureEntry>,
    _window: Arc<Window>,
}

impl OutputRenderer {
    pub fn new(
        gpu: &Gpu,
        surface: wgpu::Surface<'static>,
        window: Arc<Window>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let config = configure_surface(gpu, &surface, width, height)?;

        let uniform_size = std::mem::size_of::<Uniforms>() as u64;
        let uniform_alignment = u64::from(
            gpu.adapter()
                .limits()
                .min_uniform_buffer_offset_alignment
                .max(1),
        );
        let uniform_stride = uniform_size.div_ceil(uniform_alignment) * uniform_alignment;
        let uniform_binding_size =
            wgpu::BufferSize::new(uniform_size).expect("the layer uniform struct is never empty");

        let uniform_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("lpmt-output-uniforms"),
            size: uniform_stride * MAX_LAYER_SLOTS as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout =
            gpu.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("lpmt-output-uniform-layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(uniform_binding_size),
                        },
                        count: None,
                    }],
                });

        let uniform_bind_group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lpmt-output-uniform-bind-group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: Some(uniform_binding_size),
                }),
            }],
        });

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

        let shader = gpu
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("lpmt-output-shader"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });

        let pipeline_layout =
            gpu.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("lpmt-output-pipeline-layout"),
                    bind_group_layouts: &[
                        Some(&uniform_bind_group_layout),
                        Some(&texture_bind_group_layout),
                    ],
                    immediate_size: 0,
                });

        let pipelines = BlendMode::ALL
            .map(|blend| create_pipeline(gpu, &shader, &pipeline_layout, config.format, blend));

        Ok(Self {
            surface,
            config,
            pipelines,
            uniform_buffer,
            uniform_stride,
            uniform_bind_group,
            texture_bind_group_layout,
            sampler,
            checker,
            media: HashMap::new(),
            _window: window,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn has_media(&self, id: &MediaId) -> bool {
        self.media.contains_key(id)
    }

    pub fn clear_media(&mut self) {
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

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        if self.config.width == width && self.config.height == height {
            return Ok(());
        }

        self.config = configure_surface(gpu, &self.surface, width, height)?;
        log::debug!(
            "output resized to {}x{} ({:?})",
            self.config.width,
            self.config.height,
            self.config.format
        );
        Ok(())
    }

    /// Render one stage through one output view.
    ///
    /// Layers are composited bottom-to-top in `Stage::layers` order. Each
    /// visible layer gets its own dynamic-uniform slice and blend pipeline.
    pub fn render(&mut self, gpu: &Gpu, stage: &Stage, view: Rect) -> Result<()> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }

        let mut draws = Vec::new();
        for layer in stage.layers.iter().filter(|layer| layer.visible) {
            if draws.len() >= MAX_LAYER_SLOTS {
                break;
            }
            let Some(surface) = stage.surface(&layer.surface_id) else {
                continue;
            };
            if !surface.visible {
                continue;
            }

            let opacity = (layer.opacity * surface.opacity).clamp(0.0, 1.0);
            if opacity <= 0.0 {
                continue;
            }

            let Ok(surface_to_stage) = surface.homography() else {
                continue;
            };
            let Ok(stage_to_surface) = surface_to_stage.inverse() else {
                continue;
            };

            let entry = match &layer.source {
                SourceSpec::Image { media_id } | SourceSpec::Video { media_id } => {
                    self.media.get(media_id).unwrap_or(&self.checker)
                }
                SourceSpec::TestPattern { .. } => &self.checker,
            };

            let offset = (draws.len() as u64 * self.uniform_stride) as u32;
            draws.push(DrawItem {
                offset,
                uniforms: Uniforms {
                    inv_surface: stage_to_surface.to_column_major_4x4(),
                    resolution: [self.config.width as f32, self.config.height as f32],
                    _padding: [0.0; 2],
                    view: [view.x, view.y, view.width, view.height],
                    opacity,
                    _padding2: [0.0; 3],
                },
                blend_index: pipeline_index(layer.blend),
                bind_group: entry.bind_group.clone(),
            });
        }

        for draw in &draws {
            gpu.queue().write_buffer(
                &self.uniform_buffer,
                u64::from(draw.offset),
                bytemuck::bytes_of(&draw.uniforms),
            );
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(())
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.config =
                    configure_surface(gpu, &self.surface, self.config.width, self.config.height)?;
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("output surface lost; recreate the window");
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                anyhow::bail!("validation error while acquiring output surface texture");
            }
        };

        let target = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("lpmt-output-encoder"),
            });

        {
            let background = stage.background;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lpmt-output-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: background[0] as f64,
                            g: background[1] as f64,
                            b: background[2] as f64,
                            a: background[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            for draw in &draws {
                pass.set_pipeline(&self.pipelines[draw.blend_index]);
                pass.set_bind_group(0, &self.uniform_bind_group, &[draw.offset]);
                pass.set_bind_group(1, &draw.bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }

        gpu.queue().submit(Some(encoder.finish()));
        gpu.queue().present(frame);
        Ok(())
    }
}

fn create_pipeline(
    gpu: &Gpu,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    blend: BlendMode,
) -> wgpu::RenderPipeline {
    let label = format!("lpmt-output-pipeline-{}", blend.label().to_lowercase());
    gpu.device()
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(blend_state(blend)),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
}

fn pipeline_index(blend: BlendMode) -> usize {
    BlendMode::ALL
        .iter()
        .position(|mode| *mode == blend)
        .expect("every blend mode has a pipeline")
}

fn blend_state(blend: BlendMode) -> wgpu::BlendState {
    // The fragment shader emits premultiplied colour. The fixed-function
    // factors below then implement each mode without a destination read.
    let color = match blend {
        BlendMode::Normal => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        BlendMode::Add => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        BlendMode::Screen => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrc,
            operation: wgpu::BlendOperation::Add,
        },
        BlendMode::Multiply => wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Dst,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    };
    wgpu::BlendState {
        color,
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

fn configure_surface(
    gpu: &Gpu,
    surface: &wgpu::Surface<'_>,
    width: u32,
    height: u32,
) -> Result<wgpu::SurfaceConfiguration> {
    let mut config = surface
        .get_default_config(gpu.adapter(), width.max(1), height.max(1))
        .context("surface is not supported by the selected GPU adapter")?;

    let capabilities = surface.get_capabilities(gpu.adapter());
    if let Some(srgb) = capabilities
        .formats
        .iter()
        .copied()
        .find(|format| format.is_srgb())
    {
        config.format = srgb;
    }
    if capabilities
        .present_modes
        .contains(&wgpu::PresentMode::Fifo)
    {
        config.present_mode = wgpu::PresentMode::Fifo;
    }

    surface.configure(gpu.device(), &config);
    Ok(config)
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

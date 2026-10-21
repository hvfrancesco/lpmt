//! Stage and output renderers.
//!
//! `OutputRenderer` draws a stage view into a window surface. `PreviewRenderer`
//! draws the full stage into an offscreen texture that the editor can display
//! through egui, so the preview does not depend on any output window.
//!
//! Content textures live in the device-wide `MediaCache`, so many outputs and
//! the preview sample the same decoded media.

use std::sync::Arc;

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use lpmt_core::model::{BlendMode, ColorCorrection, EdgeBlend, OutputSpec, Rect, Stage};
use winit::window::Window;

use crate::gpu::Gpu;
use crate::media::MediaCache;

const MAX_LAYER_SLOTS: usize = 256;
const PREVIEW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Layers render into this offscreen target first, so output-level colour and
/// edge blending are applied once to the completed stage view.
const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

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

/// Post-composite uniforms: edge feather widths and colour correction.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CompositeUniforms {
    edge_blend: [f32; 4],
    color: [f32; 4],
}

const COMPOSITE_SHADER: &str = r#"
struct Composite {
    // Left, right, top, bottom feather widths, normalized to the output.
    edge_blend: vec4<f32>,
    // Brightness, contrast, gamma, unused.
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> composite: Composite;
@group(1) @binding(0) var scene_texture: texture_2d<f32>;
@group(1) @binding(1) var scene_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

fn edge_fade(coord: f32, width: f32) -> f32 {
    if (width <= 0.0) {
        return 1.0;
    }
    return smoothstep(0.0, width, coord);
}

fn composite_color(frag_coord: vec4<f32>) -> vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(scene_texture));
    let uv = frag_coord.xy / dimensions;
    let edge = composite.edge_blend;

    let feather =
        edge_fade(uv.x, edge.x) *
        edge_fade(1.0 - uv.x, edge.y) *
        edge_fade(uv.y, edge.z) *
        edge_fade(1.0 - uv.y, edge.w);

    var rgb = textureSampleLevel(scene_texture, scene_sampler, uv, 0.0).rgb;

    let gamma = max(composite.color.z, 0.01);
    rgb = pow(max(rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / gamma));
    rgb = (rgb - vec3<f32>(0.5)) * composite.color.y + vec3<f32>(0.5);
    rgb = max(rgb * composite.color.x, vec3<f32>(0.0)) * feather;

    return vec4<f32>(rgb, 1.0);
}

fn gamma_from_linear(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3<f32>(0.0031308);
    let lower = rgb * vec3<f32>(12.92);
    let higher = vec3<f32>(1.055) * pow(rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(higher, lower, cutoff);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    return composite_color(frag_coord);
}

@fragment
fn fs_main_gamma(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let color = composite_color(frag_coord);
    return vec4<f32>(gamma_from_linear(color.rgb), color.a);
}
"#;

struct DrawItem {
    offset: u32,
    uniforms: Uniforms,
    blend_index: usize,
    bind_group: wgpu::BindGroup,
}

struct SceneTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

struct StageTarget<'a> {
    view: Rect,
    edge_blend: EdgeBlend,
    color: ColorCorrection,
    width: u32,
    height: u32,
    target: &'a wgpu::TextureView,
}

/// A frame that has been acquired and fully encoded but not submitted yet.
///
/// Splitting acquisition/encoding from submission lets the application collect
/// every output in a group, submit them in one queue call and then present the
/// batch together.
pub struct PreparedFrame {
    frame: wgpu::SurfaceTexture,
    encoder: wgpu::CommandEncoder,
}

impl PreparedFrame {
    pub fn finish(self) -> (wgpu::SurfaceTexture, wgpu::CommandBuffer) {
        (self.frame, self.encoder.finish())
    }
}

/// Shared stage-composition pipeline set used by window outputs and the
/// offscreen editor preview.
struct StageRenderer {
    pipelines: [wgpu::RenderPipeline; 4],
    uniform_buffer: wgpu::Buffer,
    uniform_stride: u64,
    uniform_bind_group: wgpu::BindGroup,
    scene: Option<SceneTarget>,
    scene_layout: wgpu::BindGroupLayout,
    composite_pipeline: wgpu::RenderPipeline,
    composite_uniform_buffer: wgpu::Buffer,
    composite_bind_group: wgpu::BindGroup,
    composite_sampler: wgpu::Sampler,
}

pub struct OutputRenderer {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    stage: StageRenderer,
    _window: Arc<Window>,
}

/// Offscreen full-stage preview. The texture is registered with egui by the
/// application; it is independent of every show output window.
pub struct PreviewRenderer {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    stage: StageRenderer,
    width: u32,
    height: u32,
}

impl StageRenderer {
    fn new(
        gpu: &Gpu,
        media: &MediaCache,
        output_format: wgpu::TextureFormat,
        gamma_encode: bool,
    ) -> Result<Self> {
        let uniform_size = std::mem::size_of::<Uniforms>() as u64;
        // Use the limits the device was actually created with, not the
        // adapter's maximums: `Limits::default()` can request a stricter
        // alignment than the hardware advertises.
        let uniform_alignment = u64::from(
            gpu.device()
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
                        Some(media.bind_group_layout()),
                    ],
                    immediate_size: 0,
                });

        let pipelines = BlendMode::ALL
            .map(|blend| create_pipeline(gpu, &shader, &pipeline_layout, SCENE_FORMAT, blend));

        let composite_uniforms = CompositeUniforms {
            edge_blend: [0.0; 4],
            color: [1.0, 1.0, 1.0, 0.0],
        };
        let composite_uniform_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("lpmt-output-composite-uniforms"),
            size: std::mem::size_of::<CompositeUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue().write_buffer(
            &composite_uniform_buffer,
            0,
            bytemuck::bytes_of(&composite_uniforms),
        );

        let composite_uniform_layout =
            gpu.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("lpmt-output-composite-layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<
                                CompositeUniforms,
                            >()
                                as u64),
                        },
                        count: None,
                    }],
                });

        let composite_bind_group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lpmt-output-composite-bind-group"),
            layout: &composite_uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: composite_uniform_buffer.as_entire_binding(),
            }],
        });

        let composite_sampler = gpu.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lpmt-output-composite-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let composite_shader = gpu
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("lpmt-output-composite-shader"),
                source: wgpu::ShaderSource::Wgsl(COMPOSITE_SHADER.into()),
            });
        let composite_pipeline_layout =
            gpu.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("lpmt-output-composite-pipeline-layout"),
                    bind_group_layouts: &[
                        Some(&composite_uniform_layout),
                        Some(media.bind_group_layout()),
                    ],
                    immediate_size: 0,
                });
        let scene_layout = media.bind_group_layout().clone();
        let composite_pipeline = create_composite_pipeline(
            gpu,
            &composite_shader,
            &composite_pipeline_layout,
            output_format,
            gamma_encode,
        );

        Ok(Self {
            pipelines,
            uniform_buffer,
            uniform_stride,
            uniform_bind_group,
            scene: None,
            scene_layout,
            composite_pipeline,
            composite_uniform_buffer,
            composite_bind_group,
            composite_sampler,
        })
    }
    fn encode(
        &mut self,
        gpu: &Gpu,
        media: &MediaCache,
        stage: &Stage,
        target: StageTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let StageTarget {
            view,
            edge_blend: edge,
            color,
            width,
            height,
            target,
        } = target;
        if width == 0 || height == 0 {
            return Ok(());
        }

        gpu.queue().write_buffer(
            &self.composite_uniform_buffer,
            0,
            bytemuck::bytes_of(&CompositeUniforms {
                edge_blend: [edge.left, edge.right, edge.top, edge.bottom],
                color: [color.brightness, color.contrast, color.gamma, 0.0],
            }),
        );

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

            let bind_group = media.bind_group(&layer.source);

            let offset = (draws.len() as u64 * self.uniform_stride) as u32;
            draws.push(DrawItem {
                offset,
                uniforms: Uniforms {
                    inv_surface: stage_to_surface.to_column_major_4x4(),
                    resolution: [width as f32, height as f32],
                    _padding: [0.0; 2],
                    view: [view.x, view.y, view.width, view.height],
                    opacity,
                    _padding2: [0.0; 3],
                },
                blend_index: pipeline_index(layer.blend),
                bind_group: bind_group.clone(),
            });
        }

        for draw in &draws {
            gpu.queue().write_buffer(
                &self.uniform_buffer,
                u64::from(draw.offset),
                bytemuck::bytes_of(&draw.uniforms),
            );
        }

        self.ensure_scene_target(gpu, width, height)?;
        let scene = self
            .scene
            .as_ref()
            .expect("scene target is created immediately before rendering");

        {
            let background = stage.background;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lpmt-output-scene-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &scene.view,
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

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lpmt-output-composite-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &self.composite_bind_group, &[]);
            pass.set_bind_group(1, &scene.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        Ok(())
    }

    /// Allocate the offscreen scene target lazily and recreate it on resize.
    fn ensure_scene_target(&mut self, gpu: &Gpu, width: u32, height: u32) -> Result<()> {
        if self
            .scene
            .as_ref()
            .is_some_and(|scene| scene.width == width && scene.height == height)
        {
            return Ok(());
        }

        let texture = gpu.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("lpmt-output-scene"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SCENE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lpmt-output-scene-bind-group"),
            layout: &self.scene_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.composite_sampler),
                },
            ],
        });

        self.scene = Some(SceneTarget {
            _texture: texture,
            view,
            bind_group,
            width,
            height,
        });
        Ok(())
    }
}

impl OutputRenderer {
    pub fn new(
        gpu: &Gpu,
        media: &MediaCache,
        surface: wgpu::Surface<'static>,
        window: Arc<Window>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let config = configure_surface(gpu, &surface, width, height)?;
        let stage = StageRenderer::new(gpu, media, config.format, false)?;
        Ok(Self {
            surface,
            config,
            stage,
            _window: window,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
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

    /// Acquire and encode one stage view for this output.
    ///
    /// Layers are composited bottom-to-top in `Stage::layers` order into an
    /// offscreen scene target, then a composite pass applies the output's edge
    /// blending and colour correction. The caller submits and presents the
    /// returned frame, optionally together with other outputs in the same group.
    pub fn prepare(
        &mut self,
        gpu: &Gpu,
        media: &MediaCache,
        stage: &Stage,
        output: &OutputSpec,
    ) -> Result<Option<PreparedFrame>> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(None);
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.config =
                    configure_surface(gpu, &self.surface, self.config.width, self.config.height)?;
                return Ok(None);
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
        self.stage.encode(
            gpu,
            media,
            stage,
            StageTarget {
                view: output.view,
                edge_blend: output.edge_blend.clamped(),
                color: output.color.clamped(),
                width: self.config.width,
                height: self.config.height,
                target: &target,
            },
            &mut encoder,
        )?;

        Ok(Some(PreparedFrame { frame, encoder }))
    }
}

impl PreviewRenderer {
    /// Create an offscreen preview target at `width` x `height`.
    pub fn new(gpu: &Gpu, media: &MediaCache, width: u32, height: u32) -> Result<Self> {
        let width = width.max(1);
        let height = height.max(1);
        let texture = gpu.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("lpmt-preview-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PREVIEW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let stage = StageRenderer::new(gpu, media, PREVIEW_FORMAT, true)?;
        Ok(Self {
            _texture: texture,
            view,
            stage,
            width,
            height,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The texture view to register with egui as a native image.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Render the full stage, with no output-specific edge blending or colour
    /// correction. The result is gamma encoded for egui's texture sampler.
    pub fn render(&mut self, gpu: &Gpu, media: &MediaCache, stage: &Stage) -> Result<()> {
        let mut encoder = gpu
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("lpmt-preview-encoder"),
            });
        self.stage.encode(
            gpu,
            media,
            stage,
            StageTarget {
                view: Rect::FULL,
                edge_blend: EdgeBlend::NONE,
                color: ColorCorrection::IDENTITY,
                width: self.width,
                height: self.height,
                target: &self.view,
            },
            &mut encoder,
        )?;
        gpu.queue().submit(Some(encoder.finish()));
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

fn create_composite_pipeline(
    gpu: &Gpu,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    gamma_encode: bool,
) -> wgpu::RenderPipeline {
    let label = if gamma_encode {
        "lpmt-preview-composite-pipeline"
    } else {
        "lpmt-output-composite-pipeline"
    };
    gpu.device()
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
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
                entry_point: Some(if gamma_encode {
                    "fs_main_gamma"
                } else {
                    "fs_main"
                }),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
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

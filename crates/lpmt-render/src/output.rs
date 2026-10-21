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
use lpmt_core::guide::GuideMode;
use lpmt_core::lut::{CalibrationLut, LUT_SAMPLES};
use lpmt_core::model::{
    BlendMode, ColorCorrection, Crop, EdgeBlend, OutputSpec, Rect, Stage, WarpGrid,
};
use winit::window::Window;

use crate::gpu::Gpu;
use crate::media::MediaCache;

const MAX_LAYER_SLOTS: usize = 256;
const PREVIEW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Layers render into this offscreen target first, so output-level colour and
/// edge blending are applied once to the completed stage view.
const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Crop support shared by the quad and grid fragment paths. Polygon mask
/// coverage is rasterized into a texture by `MediaCache` and sampled through
/// bind group 2 for both paths.
const EFFECTS_WGSL_TEMPLATE: &str = r#"
struct SurfaceEffects {
    // Top, right, bottom, left rectangular crop insets.
    crop: vec4<f32>,
    // Circle centre x/y, radius, enabled flag.
    circle: vec4<f32>,
};

fn crop_visible(uv: vec2<f32>) -> bool {
    let crop = @EFFECTS@.crop;
    let minimum = vec2<f32>(crop.w, crop.x);
    let maximum = vec2<f32>(1.0 - crop.y, 1.0 - crop.z);
    if (any(uv < minimum) || any(uv > maximum)) {
        return false;
    }
    if (@EFFECTS@.circle.w > 0.5) {
        if (distance(uv, @EFFECTS@.circle.xy) > @EFFECTS@.circle.z) {
            return false;
        }
    }
    return true;
}
"#;

fn effects_shader(shader: &str, effects_path: &str) -> String {
    format!(
        "{}\n{shader}",
        EFFECTS_WGSL_TEMPLATE.replace("@EFFECTS@", effects_path)
    )
}

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
    effects: SurfaceEffects,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var source_texture: texture_2d<f32>;
@group(1) @binding(1) var source_sampler: sampler;
@group(2) @binding(0) var mask_texture: texture_2d<f32>;
@group(2) @binding(1) var mask_sampler: sampler;

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
    if (!crop_visible(source_uv)) {
        discard;
    }

    let color = textureSampleLevel(source_texture, source_sampler, source_uv, 0.0);
    let coverage = textureSampleLevel(mask_texture, mask_sampler, source_uv, 0.0).r;
    let alpha = color.a * uniforms.opacity * coverage;
    // Premultiplied output lets the fixed-function blend modes implement
    // Normal, Add, Screen and Multiply without reading the destination.
    return vec4<f32>(color.rgb * alpha, alpha);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SurfaceEffects {
    crop: [f32; 4],
    circle: [f32; 4],
}

impl SurfaceEffects {
    fn new(crop: Crop) -> Self {
        let crop = crop.clamped();
        let circle = crop.circle.map_or([0.0; 4], |circle| {
            [circle.center.x, circle.center.y, circle.radius, 1.0]
        });
        Self {
            crop: [crop.top, crop.right, crop.bottom, crop.left],
            circle,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    inv_surface: [f32; 16],
    resolution: [f32; 2],
    _padding: [f32; 2],
    view: [f32; 4],
    opacity: f32,
    _padding2: [f32; 3],
    effects: SurfaceEffects,
}

const GRID_SHADER: &str = r#"
struct Frame {
    // Output view within the stage: x, y, width, height.
    view: vec4<f32>,
    effects: SurfaceEffects,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var source_texture: texture_2d<f32>;
@group(1) @binding(1) var source_sampler: sampler;
@group(2) @binding(0) var mask_texture: texture_2d<f32>;
@group(2) @binding(1) var mask_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
};

@vertex
fn vs_main(vertex: VertexIn) -> VertexOut {
    let output_uv = (vertex.position - frame.view.xy) / frame.view.zw;
    var out: VertexOut;
    out.position = vec4<f32>(
        output_uv.x * 2.0 - 1.0,
        1.0 - output_uv.y * 2.0,
        0.0,
        1.0,
    );
    out.uv = vertex.uv;
    out.opacity = vertex.opacity;
    return out;
}

@fragment
fn fs_main(vertex: VertexOut) -> @location(0) vec4<f32> {
    if (!crop_visible(vertex.uv)) {
        discard;
    }
    let color = textureSample(source_texture, source_sampler, vertex.uv);
    let coverage = textureSample(mask_texture, mask_sampler, vertex.uv).r;
    let alpha = color.a * vertex.opacity * coverage;
    return vec4<f32>(color.rgb * alpha, alpha);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GridFrameUniforms {
    view: [f32; 4],
    effects: SurfaceEffects,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GridVertex {
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    _padding: [f32; 3],
}

impl GridVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32,
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

const INITIAL_GRID_VERTICES: u64 = 4096;

const LUT_SLOTS: usize = LUT_SAMPLES / 4;
const _: () = assert!(LUT_SAMPLES.is_multiple_of(4));

/// Post-composite uniforms: edge feather widths, colour correction and an
/// optional display-space 1D calibration LUT.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CompositeUniforms {
    edge_blend: [f32; 4],
    color: [f32; 4],
    lut_size: u32,
    guide_mode: u32,
    _lut_padding: [u32; 2],
    lut_red: [[f32; 4]; LUT_SLOTS],
    lut_green: [[f32; 4]; LUT_SLOTS],
    lut_blue: [[f32; 4]; LUT_SLOTS],
}

impl CompositeUniforms {
    fn new(
        edge: EdgeBlend,
        color: ColorCorrection,
        calibration: Option<&CalibrationLut>,
        guide: GuideMode,
    ) -> Self {
        let mut uniforms = Self {
            edge_blend: [edge.left, edge.right, edge.top, edge.bottom],
            color: [color.brightness, color.contrast, color.gamma, 0.0],
            lut_size: 0,
            guide_mode: guide.shader_value(),
            _lut_padding: [0; 2],
            lut_red: [[0.0; 4]; LUT_SLOTS],
            lut_green: [[0.0; 4]; LUT_SLOTS],
            lut_blue: [[0.0; 4]; LUT_SLOTS],
        };

        if let Some(lut) = calibration.filter(|lut| !lut.is_identity()) {
            uniforms.lut_size = LUT_SAMPLES as u32;
            for index in 0..LUT_SAMPLES {
                let slot = index / 4;
                let lane = index % 4;
                uniforms.lut_red[slot][lane] = lut.red[index].clamp(0.0, 1.0);
                uniforms.lut_green[slot][lane] = lut.green[index].clamp(0.0, 1.0);
                uniforms.lut_blue[slot][lane] = lut.blue[index].clamp(0.0, 1.0);
            }
        }

        uniforms
    }
}

const COMPOSITE_SHADER: &str = r#"
struct Composite {
    // Left, right, top, bottom feather widths, normalized to the output.
    edge_blend: vec4<f32>,
    // Brightness, contrast, gamma, unused.
    color: vec4<f32>,
    lut_size: u32,
    guide_mode: u32,
    _lut_padding0: u32,
    _lut_padding1: u32,
    lut_red: array<vec4<f32>, 8>,
    lut_green: array<vec4<f32>, 8>,
    lut_blue: array<vec4<f32>, 8>,
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

fn gamma_from_linear(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3<f32>(0.0031308);
    let lower = rgb * vec3<f32>(12.92);
    let higher = vec3<f32>(1.055) * pow(rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(higher, lower, cutoff);
}

fn gamma_to_linear(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb <= vec3<f32>(0.04045);
    let lower = rgb / vec3<f32>(12.92);
    let higher = pow((rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    return select(higher, lower, cutoff);
}

fn lut_component(table: array<vec4<f32>, 8>, index: u32) -> f32 {
    let slot = index / 4u;
    let lane = index % 4u;
    let value = table[slot];
    if (lane == 0u) {
        return value.x;
    }
    if (lane == 1u) {
        return value.y;
    }
    if (lane == 2u) {
        return value.z;
    }
    return value.w;
}

fn sample_lut(table: array<vec4<f32>, 8>, input: f32) -> f32 {
    let last = f32(composite.lut_size - 1u);
    let position = clamp(input, 0.0, 1.0) * last;
    let low = u32(floor(position));
    let high = min(low + 1u, composite.lut_size - 1u);
    let t = position - floor(position);
    return mix(lut_component(table, low), lut_component(table, high), t);
}

fn line_at(value: f32, center: f32, pixels: f32, width_px: f32) -> f32 {
    let width = width_px / max(pixels, 1.0);
    let distance = abs(value - center);
    return 1.0 - smoothstep(width, width * 2.0, distance);
}

fn repeating_line(value: f32, spacing: f32, pixels: f32, width_px: f32) -> f32 {
    let distance = abs(fract(value / spacing + 0.5) - 0.5) * spacing;
    let width = width_px / max(pixels, 1.0);
    return 1.0 - smoothstep(width, width * 2.0, distance);
}

fn guide_mask(uv: vec2<f32>, mode: u32) -> f32 {
    let size = vec2<f32>(textureDimensions(scene_texture));

    if (mode == 1u) {
        // Eight-cell alignment grid plus a stronger centre cross.
        let grid = max(
            repeating_line(uv.x, 0.125, size.x, 1.0),
            repeating_line(uv.y, 0.125, size.y, 1.0),
        );
        let axes = max(
            line_at(uv.x, 0.5, size.x, 1.5),
            line_at(uv.y, 0.5, size.y, 1.5),
        );
        return max(grid, axes);
    }
    if (mode == 2u) {
        return max(
            line_at(uv.x, 0.5, size.x, 1.5),
            line_at(uv.y, 0.5, size.y, 1.5),
        );
    }
    if (mode == 3u) {
        // Outer 5% action-safe frame and an inner 10% title-safe frame.
        let outer = max(
            max(line_at(uv.x, 0.05, size.x, 1.5), line_at(uv.x, 0.95, size.x, 1.5)),
            max(line_at(uv.y, 0.05, size.y, 1.5), line_at(uv.y, 0.95, size.y, 1.5)),
        );
        let inner = max(
            max(line_at(uv.x, 0.10, size.x, 1.0), line_at(uv.x, 0.90, size.x, 1.0)),
            max(line_at(uv.y, 0.10, size.y, 1.0), line_at(uv.y, 0.90, size.y, 1.0)),
        );
        return max(outer, inner);
    }
    if (mode == 4u) {
        return max(
            max(
                line_at(uv.x, 0.333333, size.x, 1.0),
                line_at(uv.x, 0.666667, size.x, 1.0),
            ),
            max(
                line_at(uv.y, 0.333333, size.y, 1.0),
                line_at(uv.y, 0.666667, size.y, 1.0),
            ),
        );
    }
    return 0.0;
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
    rgb = max(rgb * composite.color.x, vec3<f32>(0.0));

    // Calibration files describe the signal sent to the projector, so apply
    // the curves in display space and convert back to the linear working space.
    if (composite.lut_size > 1u) {
        let encoded = gamma_from_linear(rgb);
        let mapped = vec3<f32>(
            sample_lut(composite.lut_red, encoded.r),
            sample_lut(composite.lut_green, encoded.g),
            sample_lut(composite.lut_blue, encoded.b),
        );
        rgb = gamma_to_linear(mapped);
    }

    rgb = rgb * feather;

    // Guides are a session-only setup aid. Draw them after calibration and
    // feathering so their colour and position are predictable on every output.
    if (composite.guide_mode > 0u) {
        let mask = guide_mask(uv, composite.guide_mode);
        rgb = mix(rgb, vec3<f32>(1.0, 0.82, 0.2), mask * 0.9);
    }

    return vec4<f32>(rgb, 1.0);
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

enum DrawItem {
    Quad {
        offset: u32,
        uniforms: Uniforms,
        blend_index: usize,
        bind_group: wgpu::BindGroup,
        mask_bind_group: wgpu::BindGroup,
    },
    Grid {
        first_vertex: u32,
        vertex_count: u32,
        offset: u32,
        uniforms: GridFrameUniforms,
        blend_index: usize,
        bind_group: wgpu::BindGroup,
        mask_bind_group: wgpu::BindGroup,
    },
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
    calibration: Option<&'a CalibrationLut>,
    guide: GuideMode,
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
    grid_pipelines: [wgpu::RenderPipeline; 4],
    uniform_buffer: wgpu::Buffer,
    uniform_stride: u64,
    uniform_bind_group: wgpu::BindGroup,
    grid_uniform_buffer: wgpu::Buffer,
    grid_uniform_stride: u64,
    grid_uniform_bind_group: wgpu::BindGroup,
    grid_vertex_buffer: wgpu::Buffer,
    grid_vertex_capacity: u64,
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
    preferred_present_mode: Option<wgpu::PresentMode>,
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

        let shader_source = effects_shader(SHADER, "uniforms.effects");
        let shader = gpu
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("lpmt-output-shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let pipeline_layout =
            gpu.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("lpmt-output-pipeline-layout"),
                    bind_group_layouts: &[
                        Some(&uniform_bind_group_layout),
                        Some(media.bind_group_layout()),
                        Some(media.bind_group_layout()),
                    ],
                    immediate_size: 0,
                });

        let pipelines = BlendMode::ALL
            .map(|blend| create_pipeline(gpu, &shader, &pipeline_layout, SCENE_FORMAT, blend));

        let grid_shader_source = effects_shader(GRID_SHADER, "frame.effects");
        let grid_shader = gpu
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("lpmt-output-grid-shader"),
                source: wgpu::ShaderSource::Wgsl(grid_shader_source.into()),
            });
        let grid_uniform_size = std::mem::size_of::<GridFrameUniforms>() as u64;
        let grid_uniform_stride = grid_uniform_size.div_ceil(uniform_alignment) * uniform_alignment;
        let grid_uniform_binding_size = wgpu::BufferSize::new(grid_uniform_size)
            .expect("the grid uniform struct is never empty");
        let grid_uniform_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("lpmt-output-grid-uniforms"),
            size: grid_uniform_stride * MAX_LAYER_SLOTS as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let grid_uniform_layout =
            gpu.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("lpmt-output-grid-uniform-layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(grid_uniform_binding_size),
                        },
                        count: None,
                    }],
                });
        let grid_uniform_bind_group = gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lpmt-output-grid-uniform-bind-group"),
            layout: &grid_uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &grid_uniform_buffer,
                    offset: 0,
                    size: Some(grid_uniform_binding_size),
                }),
            }],
        });
        let grid_pipeline_layout =
            gpu.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("lpmt-output-grid-pipeline-layout"),
                    bind_group_layouts: &[
                        Some(&grid_uniform_layout),
                        Some(media.bind_group_layout()),
                        Some(media.bind_group_layout()),
                    ],
                    immediate_size: 0,
                });
        let grid_pipelines = BlendMode::ALL.map(|blend| {
            create_grid_pipeline(
                gpu,
                &grid_shader,
                &grid_pipeline_layout,
                SCENE_FORMAT,
                blend,
            )
        });
        let grid_vertex_capacity = INITIAL_GRID_VERTICES * std::mem::size_of::<GridVertex>() as u64;
        let grid_vertex_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("lpmt-output-grid-vertices"),
            size: grid_vertex_capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let composite_uniforms = CompositeUniforms::new(
            EdgeBlend::NONE,
            ColorCorrection::IDENTITY,
            None,
            GuideMode::None,
        );
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
            grid_pipelines,
            uniform_buffer,
            uniform_stride,
            uniform_bind_group,
            grid_uniform_buffer,
            grid_uniform_stride,
            grid_uniform_bind_group,
            grid_vertex_buffer,
            grid_vertex_capacity,
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
        media: &mut MediaCache,
        stage: &Stage,
        target: StageTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let StageTarget {
            view,
            edge_blend: edge,
            color,
            calibration,
            guide,
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
            bytemuck::bytes_of(&CompositeUniforms::new(edge, color, calibration, guide)),
        );
        let mut draws = Vec::new();
        let mut grid_vertices = Vec::new();
        for layer in stage.layers.iter().filter(|layer| layer.visible) {
            if draws.len() >= MAX_LAYER_SLOTS {
                break;
            }
            let bind_group = media.bind_group(&layer.source).clone();
            let blend_index = pipeline_index(layer.blend);
            for surface_id in &layer.surface_ids {
                if draws.len() >= MAX_LAYER_SLOTS {
                    break;
                }
                let Some(surface) = stage.surface(surface_id) else {
                    continue;
                };
                if !surface.visible {
                    continue;
                }

                let opacity = (layer.opacity * surface.opacity).clamp(0.0, 1.0);
                if opacity <= 0.0 {
                    continue;
                }
                let effects = SurfaceEffects::new(layer.crop);
                let mask_bind_group = media.mask_bind_group(gpu, surface)?;

                if let Some(grid) = surface
                    .grid
                    .as_ref()
                    .filter(|grid| surface.grid_enabled && grid.is_valid())
                {
                    let first_vertex = grid_vertices.len() as u32;
                    append_grid_vertices(&mut grid_vertices, grid, opacity);
                    let offset = (draws.len() as u64 * self.grid_uniform_stride) as u32;
                    draws.push(DrawItem::Grid {
                        first_vertex,
                        vertex_count: grid_vertices.len() as u32 - first_vertex,
                        offset,
                        uniforms: GridFrameUniforms {
                            view: [view.x, view.y, view.width, view.height],
                            effects,
                        },
                        blend_index,
                        bind_group: bind_group.clone(),
                        mask_bind_group: mask_bind_group.clone(),
                    });
                    continue;
                }

                let Ok(surface_to_stage) = surface.homography() else {
                    continue;
                };
                let Ok(stage_to_surface) = surface_to_stage.inverse() else {
                    continue;
                };

                let offset = (draws.len() as u64 * self.uniform_stride) as u32;
                draws.push(DrawItem::Quad {
                    offset,
                    uniforms: Uniforms {
                        inv_surface: stage_to_surface.to_column_major_4x4(),
                        resolution: [width as f32, height as f32],
                        _padding: [0.0; 2],
                        view: [view.x, view.y, view.width, view.height],
                        opacity,
                        _padding2: [0.0; 3],
                        effects,
                    },
                    blend_index,
                    bind_group: bind_group.clone(),
                    mask_bind_group,
                });
            }
        }

        for draw in &draws {
            match draw {
                DrawItem::Quad {
                    offset, uniforms, ..
                } => gpu.queue().write_buffer(
                    &self.uniform_buffer,
                    u64::from(*offset),
                    bytemuck::bytes_of(uniforms),
                ),
                DrawItem::Grid {
                    offset, uniforms, ..
                } => gpu.queue().write_buffer(
                    &self.grid_uniform_buffer,
                    u64::from(*offset),
                    bytemuck::bytes_of(uniforms),
                ),
            }
        }
        if !grid_vertices.is_empty() {
            self.ensure_grid_vertex_capacity(gpu, grid_vertices.len() as u64);
            gpu.queue().write_buffer(
                &self.grid_vertex_buffer,
                0,
                bytemuck::cast_slice(&grid_vertices),
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
                match draw {
                    DrawItem::Quad {
                        offset,
                        blend_index,
                        bind_group,
                        mask_bind_group,
                        ..
                    } => {
                        pass.set_pipeline(&self.pipelines[*blend_index]);
                        pass.set_bind_group(0, &self.uniform_bind_group, &[*offset]);
                        pass.set_bind_group(1, bind_group, &[]);
                        pass.set_bind_group(2, mask_bind_group, &[]);
                        pass.draw(0..3, 0..1);
                    }
                    DrawItem::Grid {
                        first_vertex,
                        vertex_count,
                        offset,
                        blend_index,
                        bind_group,
                        mask_bind_group,
                        ..
                    } => {
                        pass.set_pipeline(&self.grid_pipelines[*blend_index]);
                        pass.set_bind_group(0, &self.grid_uniform_bind_group, &[*offset]);
                        pass.set_bind_group(1, bind_group, &[]);
                        pass.set_bind_group(2, mask_bind_group, &[]);
                        pass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
                        pass.draw(*first_vertex..*first_vertex + *vertex_count, 0..1);
                    }
                }
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

    fn ensure_grid_vertex_capacity(&mut self, gpu: &Gpu, vertices: u64) {
        let bytes = vertices.saturating_mul(std::mem::size_of::<GridVertex>() as u64);
        if bytes <= self.grid_vertex_capacity {
            return;
        }
        let capacity = bytes
            .next_power_of_two()
            .max(INITIAL_GRID_VERTICES * std::mem::size_of::<GridVertex>() as u64);
        self.grid_vertex_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("lpmt-output-grid-vertices"),
            size: capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.grid_vertex_capacity = capacity;
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
        preferred_present_mode: Option<wgpu::PresentMode>,
    ) -> Result<Self> {
        let config = configure_surface(gpu, &surface, width, height, preferred_present_mode)?;
        let stage = StageRenderer::new(gpu, media, config.format, false)?;
        Ok(Self {
            surface,
            config,
            preferred_present_mode,
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

        self.config = configure_surface(
            gpu,
            &self.surface,
            width,
            height,
            self.preferred_present_mode,
        )?;
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
        media: &mut MediaCache,
        stage: &Stage,
        output: &OutputSpec,
        guide: GuideMode,
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
                self.config = configure_surface(
                    gpu,
                    &self.surface,
                    self.config.width,
                    self.config.height,
                    self.preferred_present_mode,
                )?;
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
                calibration: output.calibration.as_ref(),
                guide,
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
    pub fn render(&mut self, gpu: &Gpu, media: &mut MediaCache, stage: &Stage) -> Result<()> {
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
                calibration: None,
                guide: GuideMode::None,
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

fn append_grid_vertices(out: &mut Vec<GridVertex>, grid: &WarpGrid, opacity: f32) {
    let columns = f32::from(grid.columns);
    let rows = f32::from(grid.rows);
    for row in 0..grid.rows {
        for column in 0..grid.columns {
            let (Some(p00), Some(p10), Some(p11), Some(p01)) = (
                grid.point(row, column),
                grid.point(row, column + 1),
                grid.point(row + 1, column + 1),
                grid.point(row + 1, column),
            ) else {
                continue;
            };
            let u0 = f32::from(column) / columns;
            let u1 = f32::from(column + 1) / columns;
            let v0 = f32::from(row) / rows;
            let v1 = f32::from(row + 1) / rows;

            out.push(GridVertex {
                position: [p00.x, p00.y],
                uv: [u0, v0],
                opacity,
                _padding: [0.0; 3],
            });
            out.push(GridVertex {
                position: [p10.x, p10.y],
                uv: [u1, v0],
                opacity,
                _padding: [0.0; 3],
            });
            out.push(GridVertex {
                position: [p11.x, p11.y],
                uv: [u1, v1],
                opacity,
                _padding: [0.0; 3],
            });

            out.push(GridVertex {
                position: [p00.x, p00.y],
                uv: [u0, v0],
                opacity,
                _padding: [0.0; 3],
            });
            out.push(GridVertex {
                position: [p11.x, p11.y],
                uv: [u1, v1],
                opacity,
                _padding: [0.0; 3],
            });
            out.push(GridVertex {
                position: [p01.x, p01.y],
                uv: [u0, v1],
                opacity,
                _padding: [0.0; 3],
            });
        }
    }
}

fn create_grid_pipeline(
    gpu: &Gpu,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    blend: BlendMode,
) -> wgpu::RenderPipeline {
    let label = format!("lpmt-grid-pipeline-{}", blend.label().to_lowercase());
    let vertex_layout = GridVertex::layout();
    gpu.device()
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout)],
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

fn select_present_mode(
    available: &[wgpu::PresentMode],
    preferred: Option<wgpu::PresentMode>,
) -> Option<wgpu::PresentMode> {
    // Fullscreen outputs on a dedicated display can use tear-free Fifo.
    // Windowed outputs prefer Mailbox or Immediate: blocking Fifo is what made
    // the whole event loop stall when a window was obscured by the editor.
    preferred
        .filter(|mode| available.contains(mode))
        .or_else(|| {
            [
                wgpu::PresentMode::Mailbox,
                wgpu::PresentMode::Immediate,
                wgpu::PresentMode::Fifo,
            ]
            .into_iter()
            .find(|mode| available.contains(mode))
        })
}

fn configure_surface(
    gpu: &Gpu,
    surface: &wgpu::Surface<'_>,
    width: u32,
    height: u32,
    preferred_present_mode: Option<wgpu::PresentMode>,
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
    if let Some(mode) = select_present_mode(&capabilities.present_modes, preferred_present_mode) {
        config.present_mode = mode;
    }
    log::debug!(
        "output present mode: {:?} (requested: {preferred_present_mode:?}, available: {:?})",
        config.present_mode,
        capabilities.present_modes
    );

    surface.configure(gpu.device(), &config);
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lpmt_core::model::Point2;
    use std::mem::{offset_of, size_of};

    #[test]
    fn windowed_outputs_avoid_blocking_fifo() {
        let modes = [wgpu::PresentMode::Fifo, wgpu::PresentMode::Immediate];
        assert_eq!(
            select_present_mode(&modes, None),
            Some(wgpu::PresentMode::Immediate)
        );

        let modes = [
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
        ];
        assert_eq!(
            select_present_mode(&modes, None),
            Some(wgpu::PresentMode::Mailbox)
        );
    }

    #[test]
    fn dedicated_fullscreen_outputs_request_fifo() {
        let modes = [wgpu::PresentMode::Fifo, wgpu::PresentMode::Immediate];
        assert_eq!(
            select_present_mode(&modes, Some(wgpu::PresentMode::Fifo)),
            Some(wgpu::PresentMode::Fifo)
        );
        assert_eq!(
            select_present_mode(
                &[wgpu::PresentMode::Immediate],
                Some(wgpu::PresentMode::Fifo)
            ),
            Some(wgpu::PresentMode::Immediate)
        );
    }

    #[test]
    fn mask_and_crop_uniform_layout_matches_the_wgsl_structs() {
        assert_eq!(offset_of!(SurfaceEffects, crop), 0);
        assert_eq!(offset_of!(SurfaceEffects, circle), 16);
        assert_eq!(size_of::<SurfaceEffects>(), 32);
        assert_eq!(offset_of!(Uniforms, effects), 112);
        assert_eq!(size_of::<Uniforms>(), 144);
        assert_eq!(offset_of!(GridFrameUniforms, view), 0);
        assert_eq!(offset_of!(GridFrameUniforms, effects), 16);
        assert_eq!(size_of::<GridFrameUniforms>(), 48);
    }

    #[test]
    fn composite_uniform_layout_matches_the_wgsl_struct() {
        assert_eq!(offset_of!(CompositeUniforms, edge_blend), 0);
        assert_eq!(offset_of!(CompositeUniforms, color), 16);
        assert_eq!(offset_of!(CompositeUniforms, lut_size), 32);
        assert_eq!(offset_of!(CompositeUniforms, guide_mode), 36);
        assert_eq!(offset_of!(CompositeUniforms, _lut_padding), 40);
        assert_eq!(offset_of!(CompositeUniforms, lut_red), 48);
        assert_eq!(offset_of!(CompositeUniforms, lut_green), 48 + 128);
        assert_eq!(offset_of!(CompositeUniforms, lut_blue), 48 + 256);
        assert_eq!(size_of::<CompositeUniforms>(), 48 + 384);
    }

    #[test]
    fn composite_uniforms_skip_a_neutral_lut() {
        let lut = CalibrationLut::identity("neutral");
        let uniforms = CompositeUniforms::new(
            EdgeBlend::NONE,
            ColorCorrection::IDENTITY,
            Some(&lut),
            GuideMode::None,
        );
        assert_eq!(uniforms.lut_size, 0);
    }

    #[test]
    fn composite_uniforms_fill_each_calibration_channel() {
        let lut =
            CalibrationLut::from_triples("test", &[[0.0, 0.0, 0.0], [1.0, 0.5, 1.0]]).unwrap();
        let uniforms = CompositeUniforms::new(
            EdgeBlend::NONE,
            ColorCorrection::IDENTITY,
            Some(&lut),
            GuideMode::None,
        );
        assert_eq!(uniforms.lut_size, LUT_SAMPLES as u32);
        assert_eq!(uniforms.lut_red[0][0], 0.0);
        assert_eq!(uniforms.lut_red[LUT_SLOTS - 1][3], 1.0);
        assert_eq!(uniforms.lut_green[LUT_SLOTS - 1][3], 0.5);
        assert_eq!(uniforms.lut_blue[LUT_SLOTS - 1][3], 1.0);
    }

    #[test]
    fn grid_vertices_triangulate_every_cell() {
        let grid = WarpGrid::from_quad(
            [
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(0.0, 1.0),
            ],
            2,
            2,
        );
        let mut vertices = Vec::new();
        append_grid_vertices(&mut vertices, &grid, 0.5);
        assert_eq!(vertices.len(), 4 * 6);
        assert!(vertices.iter().all(|vertex| vertex.opacity == 0.5));
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[1].uv, [0.5, 0.0]);
        assert_eq!(vertices[2].uv, [0.5, 0.5]);
    }

    #[test]
    fn composite_uniforms_store_the_guide_mode() {
        let uniforms = CompositeUniforms::new(
            EdgeBlend::NONE,
            ColorCorrection::IDENTITY,
            None,
            GuideMode::SafeArea,
        );
        assert_eq!(uniforms.guide_mode, GuideMode::SafeArea.shader_value());
    }
}

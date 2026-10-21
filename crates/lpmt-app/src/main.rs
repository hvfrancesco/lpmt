//! LPMT application bootstrap.
//!
//! Milestone 0/1 intentionally uses one event loop, one GPU device, an egui
//! editor window and a separate output window. This is the smallest useful
//! demonstration of the Stage + Outputs architecture.

mod audio_runtime;
mod clock;
mod history;
mod project_io;
mod video_runtime;

use audio_runtime::AudioRuntime;
use clock::ShowClock;
use history::{History, HistoryKey};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use lpmt_control::{OscEvent, OscListener};
use lpmt_core::guide::GuideMode;
use lpmt_core::lut::parse_cube_1d;
use lpmt_core::model::{
    BlendMode, Layer, LayerId, MediaAsset, MediaId, MediaKind, OutputGroup, OutputId, OutputSpec,
    OutputTarget, Point2, Project, SourceSpec, Stage, Surface, SurfaceId, SurfaceMask, TestPattern,
    MASK_POINT_LIMIT,
};
use lpmt_core::param::{EdgeSide, ParameterId, ParameterValue};
use lpmt_render::gpu::Gpu;
use lpmt_render::media::MediaCache;
use lpmt_render::output::{OutputRenderer, PreviewRenderer};
use lpmt_ui::editor::{
    self, EditorAction, EditorState, EditorStatus, OutputPacing, SurfaceGeometryEdit,
};
use video_runtime::VideoRuntime;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::monitor::MonitorHandle;
use winit::window::{Fullscreen, Window, WindowId};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
        Box::new(event_loop.owned_display_handle()),
    ));
    let gpu = Gpu::new(instance)?;

    let mut app = App::new(gpu)?;
    app.startup_project = parse_startup_project();
    event_loop.run_app(&mut app).context("event loop failed")?;
    Ok(())
}

struct App {
    gpu: Gpu,
    media: MediaCache,
    project: Project,
    saved_project: Project,
    history: History,
    ui_state: EditorState,
    editor: Option<EditorWindow>,
    outputs: Vec<OutputWindow>,
    output_guides: HashMap<OutputId, GuideMode>,
    preview: Option<PreviewRenderer>,
    preview_texture: Option<egui::TextureId>,
    media_thumbnails: HashMap<MediaId, egui::TextureId>,
    thumbnail_revision: u64,
    monitors: Vec<String>,
    videos: VideoRuntime,
    audio: AudioRuntime,
    osc: Option<OscListener>,
    osc_address: String,
    osc_messages: u64,
    osc_last: String,
    osc_error: Option<String>,
    clock: ShowClock,
    last_clock_seek: u64,
    last_clock_loop: u64,
    media_base: PathBuf,
    startup_project: Option<PathBuf>,
    pending_output_sync: bool,
    status_message: String,
    last_frame: Instant,
    fps: f32,
    frame_ms: f32,
}

struct EditorWindow {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

struct OutputWindow {
    id: OutputId,
    window: Arc<Window>,
    renderer: OutputRenderer,
    pacing: OutputPacing,
    last_presented: Option<Instant>,
    /// X11/Wayland report fully obscured windows. Do not acquire or present a
    /// surface while hidden: a compositor-throttled present can block the
    /// whole shared event loop.
    occluded: bool,
}

impl OutputWindow {
    fn pacing_status(&self) -> OutputPacing {
        let mut pacing = self.pacing;
        pacing.since_last_present_ms = self
            .last_presented
            .map_or(0.0, |last| last.elapsed().as_secs_f32() * 1000.0);
        pacing
    }
}

impl App {
    fn new(gpu: Gpu) -> Result<Self> {
        let media = MediaCache::new(&gpu)?;
        let project = Project::demo();
        let osc_address =
            std::env::var("LPMT_OSC_ADDR").unwrap_or_else(|_| "127.0.0.1:9000".to_owned());
        let (osc, osc_error) = match OscListener::bind(&osc_address) {
            Ok(listener) => {
                log::info!("OSC listening on {}", listener.address());
                (Some(listener), None)
            }
            Err(error) => {
                log::warn!("OSC listener unavailable: {error:#}");
                (None, Some(error.to_string()))
            }
        };
        let mut app = Self {
            gpu,
            media,
            saved_project: project.clone(),
            project,
            history: History::new(),
            ui_state: EditorState::default(),
            editor: None,
            outputs: Vec::new(),
            output_guides: HashMap::new(),
            preview: None,
            preview_texture: None,
            media_thumbnails: HashMap::new(),
            thumbnail_revision: 0,
            monitors: Vec::new(),
            videos: VideoRuntime::new(),
            audio: AudioRuntime::new(),
            osc,
            osc_address,
            osc_messages: 0,
            osc_last: String::new(),
            osc_error,
            clock: ShowClock::new(),
            last_clock_seek: 0,
            last_clock_loop: 0,
            media_base: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            startup_project: None,
            pending_output_sync: false,
            status_message: String::new(),
            last_frame: Instant::now(),
            fps: 0.0,
            frame_ms: 0.0,
        };
        app.ui_state.image_path = app
            .project
            .media
            .first()
            .map_or_else(String::new, |asset| asset.path.clone());
        Ok(app)
    }

    fn create_windows(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let editor_attributes = Window::default_attributes()
            .with_title("LPMT — Editor")
            .with_inner_size(LogicalSize::new(1180.0, 760.0))
            .with_min_inner_size(LogicalSize::new(900.0, 600.0));
        let editor_window = Arc::new(
            event_loop
                .create_window(editor_attributes)
                .context("failed to create editor window")?,
        );

        let editor_surface = self
            .gpu
            .instance()
            .create_surface(editor_window.clone())
            .context("failed to create editor surface")?;
        let editor_size = editor_window.inner_size();
        let editor_config = configure_egui_surface(
            &self.gpu,
            &editor_surface,
            editor_size.width,
            editor_size.height,
        )?;

        let egui_ctx = egui::Context::default();
        egui_ctx.set_visuals(egui::Visuals::dark());
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            editor_window.as_ref(),
            Some(editor_window.scale_factor() as f32),
            editor_window.theme(),
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            self.gpu.device(),
            editor_config.format,
            egui_wgpu::RendererOptions::default(),
        );

        self.editor = Some(EditorWindow {
            window: editor_window,
            surface: editor_surface,
            config: editor_config,
            egui_ctx,
            egui_state,
            egui_renderer,
        });

        self.monitors = event_loop
            .available_monitors()
            .filter_map(|monitor| monitor.name())
            .collect();
        log::info!("available displays: {:?}", self.monitors);

        if let Some(path) = self.startup_project.take() {
            let path = path.to_string_lossy().into_owned();
            if let Err(error) = self.load_project(&path) {
                log::error!("failed to load startup project: {error:#}");
                self.status_message = format!("Startup project failed: {error}");
                self.reload_media();
            }
        } else {
            self.reload_media();
        }

        // Outputs are created from the project, not the other way around. A
        // project can define any number of outputs, each with its own view.
        self.sync_output_windows(event_loop);

        Ok(())
    }

    /// Create windows for enabled outputs and close the rest. Windows are only
    /// opened or closed on project changes, never in the per-frame path.
    fn sync_output_windows(&mut self, event_loop: &ActiveEventLoop) {
        self.outputs.retain(|output| {
            self.project
                .outputs
                .iter()
                .any(|spec| spec.id == output.id && spec.enabled)
        });

        let missing: Vec<OutputId> = self
            .project
            .outputs
            .iter()
            .filter(|spec| spec.enabled && !self.outputs.iter().any(|output| output.id == spec.id))
            .map(|spec| spec.id.clone())
            .collect();
        let editor_monitor = self
            .editor
            .as_ref()
            .and_then(|editor| editor.window.current_monitor())
            .and_then(|monitor| monitor.name());
        log::debug!("editor monitor: {editor_monitor:?}");

        for id in missing {
            let used_monitors = self.used_fullscreen_monitors();
            let cascade = self.outputs.len() as i32;
            let result = match self.project.output(&id) {
                Some(spec) => create_output_window(
                    &self.gpu,
                    &self.media,
                    event_loop,
                    spec,
                    &used_monitors,
                    editor_monitor.as_deref(),
                    cascade,
                ),
                None => continue,
            };

            match result {
                Ok(output) => {
                    let name = self
                        .project
                        .output(&id)
                        .map_or_else(String::new, |spec| spec.name.clone());
                    log::info!("opened output '{name}' ({id})");
                    self.outputs.push(output);
                }
                Err(error) => {
                    log::error!("failed to open output {id}: {error:#}");
                    self.status_message = format!("Output {id} failed: {error}");
                    if let Some(spec) = self.project.output_mut(&id) {
                        spec.enabled = false;
                    }
                }
            }
        }
    }

    fn used_fullscreen_monitors(&self) -> Vec<String> {
        self.outputs
            .iter()
            .filter(|output| output.window.fullscreen().is_some())
            .filter_map(|output| output.window.current_monitor())
            .filter_map(|monitor| monitor.name())
            .collect()
    }

    fn handle_editor_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let mut close = false;
        let mut redraw = false;
        let mut resize = None;

        if let Some(editor) = self.editor.as_mut() {
            let response = editor.egui_state.on_window_event(&editor.window, &event);
            if response.repaint {
                editor.window.request_redraw();
            }

            match event {
                WindowEvent::CloseRequested => close = true,
                WindowEvent::Resized(size) => resize = Some(size),
                WindowEvent::RedrawRequested => redraw = true,
                _ => {}
            }
        }

        if close {
            event_loop.exit();
            return;
        }

        if let Some(size) = resize {
            if let Some(editor) = self.editor.as_mut() {
                if let Err(error) = editor.resize(&self.gpu, size.width, size.height) {
                    log::error!("failed to resize editor: {error:#}");
                }
            }
        }

        if redraw {
            self.render_editor();
        }
    }

    fn handle_output_event(&mut self, index: usize, event: WindowEvent) {
        let mut close = false;
        let mut resize = None;

        if let Some(output) = self.outputs.get_mut(index) {
            match event {
                WindowEvent::CloseRequested => close = true,
                WindowEvent::Resized(size) => resize = Some(size),
                WindowEvent::Occluded(occluded) => {
                    output.occluded = occluded;
                    log::debug!(
                        "output '{}' {}",
                        output.id,
                        if occluded { "occluded" } else { "visible" }
                    );
                }
                // Outputs are rendered once per application tick in
                // `about_to_wait`, so grouped outputs can share a frame.
                WindowEvent::RedrawRequested => {}
                _ => {}
            }
        }

        if close {
            // Closing an output window is not the same as quitting the show:
            // mark the output closed so it can be reopened from the inspector.
            let id = self.outputs[index].id.clone();
            let name = self
                .project
                .output(&id)
                .map_or_else(|| id.to_string(), |spec| spec.name.clone());
            self.outputs.remove(index);
            if let Some(spec) = self.project.output_mut(&id) {
                spec.enabled = false;
            }
            self.status_message = format!("Closed output '{name}'");
            log::info!("output '{name}' closed by the user");
            if let Some(editor) = self.editor.as_ref() {
                editor.window.request_redraw();
            }
            return;
        }

        if let Some(size) = resize {
            if let Some(output) = self.outputs.get_mut(index) {
                if let Err(error) = output.renderer.resize(&self.gpu, size.width, size.height) {
                    log::error!("failed to resize output: {error:#}");
                }
            }
        }
    }

    fn render_editor(&mut self) {
        self.sync_media_thumbnails();
        let status = self.status();
        let App {
            gpu,
            project,
            ui_state,
            editor,
            ..
        } = self;
        let Some(editor) = editor.as_mut() else {
            return;
        };
        if editor.config.width == 0 || editor.config.height == 0 {
            return;
        }

        let raw_input = editor.egui_state.take_egui_input(&editor.window);
        let mut actions = Vec::new();
        let mut full_output = editor.egui_ctx.run_ui(raw_input, |ui| {
            actions = editor::draw(ui, ui_state, project, &status);
        });
        editor
            .egui_state
            .handle_platform_output(&editor.window, full_output.platform_output);

        let paint_jobs = editor
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [editor.config.width, editor.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        for (id, image_deltas) in &full_output.textures_delta.set {
            for image_delta in image_deltas {
                editor
                    .egui_renderer
                    .update_texture(gpu.device(), gpu.queue(), *id, image_delta);
            }
        }

        let mut encoder = gpu
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("lpmt-editor-encoder"),
            });
        let user_command_buffers = editor.egui_renderer.update_buffers(
            gpu.device(),
            gpu.queue(),
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let frame = match editor.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => None,
            wgpu::CurrentSurfaceTexture::Outdated => {
                if let Err(error) = editor.reconfigure(gpu) {
                    log::error!("failed to reconfigure editor: {error:#}");
                }
                None
            }
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Validation => {
                log::error!("editor surface lost or invalid");
                None
            }
        };

        if let Some(frame) = frame {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            {
                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("lpmt-editor-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.012,
                                g: 0.014,
                                b: 0.02,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                let mut render_pass = render_pass.forget_lifetime();
                editor
                    .egui_renderer
                    .render(&mut render_pass, &paint_jobs, &screen_descriptor);
            }

            let mut command_buffers = user_command_buffers;
            command_buffers.push(encoder.finish());
            gpu.queue().submit(command_buffers);
            gpu.queue().present(frame);
        }

        for id in &full_output.textures_delta.free {
            editor.egui_renderer.free_texture(id);
        }
        full_output.textures_delta.clear();

        self.apply_actions(actions);
    }

    /// Render every open output once per tick. Outputs in the same group are
    /// prepared first, submitted in one queue call and then presented together.
    fn render_outputs(&mut self) {
        let groups: Vec<Vec<OutputId>> = self
            .project
            .output_groups
            .iter()
            .map(|group| group.outputs.clone())
            .collect();
        let mut rendered = vec![false; self.outputs.len()];

        for output_ids in groups {
            let indices: Vec<usize> = output_ids
                .iter()
                .filter_map(|id| self.outputs.iter().position(|output| &output.id == id))
                .filter(|index| !rendered[*index])
                .collect();
            if indices.is_empty() {
                continue;
            }
            for index in &indices {
                rendered[*index] = true;
            }
            self.present_outputs(&indices);
        }

        let ungrouped: Vec<usize> = (0..self.outputs.len())
            .filter(|index| !rendered[*index])
            .collect();
        for index in ungrouped {
            self.present_outputs(&[index]);
        }
    }

    fn present_outputs(&mut self, indices: &[usize]) {
        let App {
            gpu,
            media,
            project,
            outputs,
            output_guides,
            ..
        } = self;
        let mut prepared = Vec::new();

        for index in indices {
            let Some(output) = outputs.get_mut(*index) else {
                continue;
            };
            if output.occluded {
                continue;
            }
            output.pacing.note_attempt();
            let Some(output_spec) = project.output(&output.id) else {
                output.pacing.note_error();
                continue;
            };
            let Some(stage) = project
                .stage(&output_spec.stage_id)
                .or_else(|| project.primary_stage())
            else {
                output.pacing.note_error();
                continue;
            };
            let guide = output_guides
                .get(&output.id)
                .copied()
                .unwrap_or(GuideMode::None);
            match output
                .renderer
                .prepare(gpu, media, stage, output_spec, guide)
            {
                Ok(Some(frame)) => prepared.push((*index, frame)),
                Ok(None) => output.pacing.note_skipped(),
                Err(error) => {
                    output.pacing.note_error();
                    log::error!("output '{}' render failed: {error:#}", output_spec.name);
                }
            }
        }

        if prepared.is_empty() {
            return;
        }

        let count = prepared.len();
        let mut frames = Vec::with_capacity(count);
        let mut command_buffers = Vec::with_capacity(count);
        for (index, prepared) in prepared {
            let (frame, command_buffer) = prepared.finish();
            frames.push((index, frame));
            command_buffers.push(command_buffer);
        }

        // One submit for the whole group is the frame barrier: no output is
        // presented until every prepared frame in the batch is enqueued.
        gpu.queue().submit(command_buffers);
        let now = Instant::now();
        for (index, frame) in frames {
            gpu.queue().present(frame);
            if let Some(output) = outputs.get_mut(index) {
                let interval = output
                    .last_presented
                    .map(|last| now.duration_since(last).as_secs_f32() * 1000.0);
                output.pacing.note_present(interval);
                output.last_presented = Some(now);
            }
        }
        if indices.len() > 1 {
            log::trace!("presented {count} grouped outputs in one batch");
        }
    }

    fn poll_osc(&mut self) {
        let events = self
            .osc
            .as_ref()
            .map(OscListener::drain)
            .unwrap_or_default();
        for event in events {
            self.handle_osc_event(event);
        }
    }

    fn handle_osc_event(&mut self, event: OscEvent) {
        match event {
            OscEvent::Parameter {
                parameter,
                value,
                address,
            } => {
                self.osc_messages = self.osc_messages.saturating_add(1);
                self.osc_last = format!("{address} = {value}");
                match self.handle_osc_parameter(parameter, value) {
                    Ok(()) => {
                        self.osc_error = None;
                        log::debug!("OSC applied {address}");
                    }
                    Err(error) => {
                        self.osc_error = Some(error.clone());
                        log::warn!("OSC rejected {address}: {error}");
                    }
                }
            }
            OscEvent::Invalid { address, reason } => {
                self.osc_messages = self.osc_messages.saturating_add(1);
                let error = format!("{address}: {reason}");
                self.osc_last.clone_from(&error);
                self.osc_error = Some(error.clone());
                log::warn!("OSC invalid message: {error}");
            }
            OscEvent::Error { message } => {
                self.osc_error = Some(message.clone());
                log::error!("OSC error: {message}");
            }
        }
    }

    fn handle_osc_parameter(
        &mut self,
        parameter: ParameterId,
        value: ParameterValue,
    ) -> Result<(), String> {
        let path = parameter.path();
        match parameter {
            ParameterId::TransportPlaying => {
                let playing = require_bool(&value, &path)?;
                if playing != self.clock.is_playing() {
                    if playing {
                        self.clock.play();
                    } else {
                        self.clock.pause();
                    }
                }
                Ok(())
            }
            ParameterId::TransportPosition => {
                let position = require_float(&value, &path)?;
                self.clock.seek(f64::from(position));
                Ok(())
            }
            ParameterId::TransportLooping => {
                self.clock.set_looping(require_bool(&value, &path)?);
                Ok(())
            }
            _ => {
                let actions = self.project_actions_for_parameter(&parameter, &value)?;
                if !actions.is_empty() {
                    self.apply_actions(actions);
                }
                Ok(())
            }
        }
    }

    fn project_actions_for_parameter(
        &self,
        parameter: &ParameterId,
        value: &ParameterValue,
    ) -> Result<Vec<EditorAction>, String> {
        let path = parameter.path();
        let mut actions = Vec::new();
        match parameter {
            ParameterId::SurfaceOpacity(id) => {
                let surface = self
                    .surface_index(id)
                    .ok_or_else(|| format!("{path}: surface not found"))?;
                actions.push(EditorAction::SetSurfaceOpacity {
                    surface,
                    opacity: require_float(value, &path)?,
                });
            }
            ParameterId::SurfaceVisible(id) => {
                let surface = self
                    .surface_index(id)
                    .ok_or_else(|| format!("{path}: surface not found"))?;
                actions.push(EditorAction::SetSurfaceVisibility {
                    surface,
                    visible: require_bool(value, &path)?,
                });
            }
            ParameterId::SurfaceMaskEnabled(id) => {
                let surface = self
                    .surface_index(id)
                    .ok_or_else(|| format!("{path}: surface not found"))?;
                actions.push(EditorAction::SetSurfaceMaskEnabled {
                    surface,
                    enabled: require_bool(value, &path)?,
                });
            }
            ParameterId::SurfaceMaskInvert(id) => {
                let surface = self
                    .surface_index(id)
                    .ok_or_else(|| format!("{path}: surface not found"))?;
                actions.push(EditorAction::SetSurfaceMaskInvert {
                    surface,
                    invert: require_bool(value, &path)?,
                });
            }
            ParameterId::SurfaceMaskFeather(id) => {
                let surface = self
                    .surface_index(id)
                    .ok_or_else(|| format!("{path}: surface not found"))?;
                actions.push(EditorAction::SetSurfaceMaskFeather {
                    surface,
                    feather: require_float(value, &path)?,
                });
            }
            ParameterId::LayerOpacity(id) => {
                let layer = self
                    .layer_index(id)
                    .ok_or_else(|| format!("{path}: layer not found"))?;
                actions.push(EditorAction::SetLayerOpacity {
                    layer,
                    opacity: require_float(value, &path)?,
                });
            }
            ParameterId::LayerVisible(id) => {
                let layer = self
                    .layer_index(id)
                    .ok_or_else(|| format!("{path}: layer not found"))?;
                actions.push(EditorAction::SetLayerVisibility {
                    layer,
                    visible: require_bool(value, &path)?,
                });
            }
            ParameterId::LayerBlend(id) => {
                let layer = self
                    .layer_index(id)
                    .ok_or_else(|| format!("{path}: layer not found"))?;
                let blend = value
                    .blend_mode()
                    .ok_or_else(|| format!("{path} expects normal/add/screen/multiply"))?;
                actions.push(EditorAction::SetLayerBlend { layer, blend });
            }
            ParameterId::OutputEnabled(id) => {
                let output = self
                    .output_index(id)
                    .ok_or_else(|| format!("{path}: output not found"))?;
                actions.push(EditorAction::SetOutputEnabled {
                    output,
                    enabled: require_bool(value, &path)?,
                });
            }
            ParameterId::OutputBrightness(id)
            | ParameterId::OutputContrast(id)
            | ParameterId::OutputGamma(id) => {
                let output = self
                    .output_index(id)
                    .ok_or_else(|| format!("{path}: output not found"))?;
                let Some(spec) = self.project.outputs.get(output) else {
                    return Err(format!("{path}: output not found"));
                };
                let mut color = spec.color;
                let amount = require_float(value, &path)?;
                match parameter {
                    ParameterId::OutputBrightness(_) => color.brightness = amount,
                    ParameterId::OutputContrast(_) => color.contrast = amount,
                    ParameterId::OutputGamma(_) => color.gamma = amount,
                    _ => unreachable!("guard only admits colour parameters"),
                }
                actions.push(EditorAction::SetOutputColor { output, color });
            }
            ParameterId::OutputEdgeBlend(id, side) => {
                let output = self
                    .output_index(id)
                    .ok_or_else(|| format!("{path}: output not found"))?;
                let Some(spec) = self.project.outputs.get(output) else {
                    return Err(format!("{path}: output not found"));
                };
                let mut edge = spec.edge_blend;
                let amount = require_float(value, &path)?;
                match side {
                    EdgeSide::Left => edge.left = amount,
                    EdgeSide::Right => edge.right = amount,
                    EdgeSide::Top => edge.top = amount,
                    EdgeSide::Bottom => edge.bottom = amount,
                }
                actions.push(EditorAction::SetOutputEdgeBlend {
                    output,
                    edge_blend: edge,
                });
            }
            ParameterId::ShowAudioVolume => {
                actions.push(EditorAction::SetShowAudioVolume {
                    volume: require_float(value, &path)?,
                });
            }
            ParameterId::ShowAudioMuted => {
                actions.push(EditorAction::SetShowAudioMuted {
                    muted: require_bool(value, &path)?,
                });
            }
            ParameterId::TransportPlaying
            | ParameterId::TransportPosition
            | ParameterId::TransportLooping => {
                return Err(format!("{path} is applied directly to the show clock"));
            }
        }
        Ok(actions)
    }

    fn surface_index(&self, id: &SurfaceId) -> Option<usize> {
        self.project
            .primary_stage()
            .and_then(|stage| stage.surfaces.iter().position(|surface| &surface.id == id))
    }

    fn layer_index(&self, id: &LayerId) -> Option<usize> {
        self.project
            .primary_stage()
            .and_then(|stage| stage.layers.iter().position(|layer| &layer.id == id))
    }

    fn output_index(&self, id: &OutputId) -> Option<usize> {
        self.project
            .outputs
            .iter()
            .position(|output| &output.id == id)
    }

    fn apply_actions(&mut self, actions: Vec<EditorAction>) {
        let mut pending: Option<(Project, HistoryKey, &'static str)> = None;
        for action in actions {
            // Finish the previous edit before starting the next one. Keeping
            // the snapshot until here means an early `continue` below still
            // commits the history entry at the end of the batch.
            if let Some((before, key, label)) = pending.take() {
                self.finish_history(before, key, label);
            }
            if let Some((key, label)) = history_key(&action) {
                pending = Some((self.project.clone(), key, label));
            } else {
                self.history.break_coalescing();
            }
            match action {
                EditorAction::SetCorner {
                    surface,
                    corner,
                    value,
                } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.set_corner(corner, value);
                    }
                }
                EditorAction::SetSurfaceCorners { surfaces, .. } => {
                    if let Some(stage) = self.project.primary_stage_mut() {
                        for (index, corners) in surfaces {
                            if let Some(surface) = stage.surfaces.get_mut(index) {
                                surface
                                    .set_corners_preserving_grid(corners.map(Point2::clamp_unit));
                            }
                        }
                    }
                }
                EditorAction::SetSurfaceGridEnabled { surface, enabled } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.set_grid_enabled(enabled);
                    }
                }
                EditorAction::SetSurfaceGridResolution {
                    surface,
                    rows,
                    columns,
                } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.set_grid_resolution(rows, columns);
                    }
                }
                EditorAction::ResetSurfaceGrid { surface } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.reset_grid();
                    }
                }
                EditorAction::SetGridPoint {
                    surface,
                    row,
                    column,
                    value,
                } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.set_grid_point(row, column, value);
                    }
                }
                EditorAction::ResetSurface { surface } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.reset_to_full_stage();
                    }
                }
                EditorAction::AddSurface => {
                    let Some(stage) = self.project.primary_stage_mut() else {
                        continue;
                    };
                    let mut index = stage.surfaces.len() + 1;
                    let id = loop {
                        let candidate = format!("surface-{index}");
                        if !stage
                            .surfaces
                            .iter()
                            .any(|surface| surface.id.as_str() == candidate)
                        {
                            break candidate;
                        }
                        index += 1;
                    };
                    let inset = 0.08 * (index % 4) as f32;
                    let surface = Surface::new(
                        id,
                        format!("Surface {index}"),
                        [
                            Point2::new(0.2 + inset, 0.2 + inset),
                            Point2::new(0.8 - inset, 0.2 + inset),
                            Point2::new(0.8 - inset, 0.8 - inset),
                            Point2::new(0.2 + inset, 0.8 - inset),
                        ],
                    );
                    stage.surfaces.push(surface);
                    self.ui_state.select_only_surface(stage.surfaces.len() - 1);
                }
                EditorAction::DeleteSurface { surface } => {
                    let remaining = {
                        let Some(stage) = self.project.primary_stage_mut() else {
                            continue;
                        };
                        if stage.surfaces.len() <= 1 || surface >= stage.surfaces.len() {
                            continue;
                        }
                        let surface_id = stage.surfaces[surface].id.clone();
                        if stage.layers.iter().any(|layer| {
                            layer.surface_ids.len() <= 1 && layer.uses_surface(&surface_id)
                        }) {
                            self.status_message =
                                "Reassign layers that only use this surface first".to_owned();
                            continue;
                        }
                        for layer in &mut stage.layers {
                            layer.surface_ids.retain(|id| id != &surface_id);
                        }
                        stage.surfaces.remove(surface);
                        stage.surfaces.len()
                    };
                    self.ui_state.remove_surface_selection(surface, remaining);
                }
                EditorAction::RenameSurface { surface, name } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        if !name.trim().is_empty() {
                            surface.name = name;
                        }
                    }
                }
                EditorAction::SetSurfaceVisibility { surface, visible } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.visible = visible;
                    }
                }
                EditorAction::SetSurfaceOpacity { surface, opacity } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.opacity = opacity.clamp(0.0, 1.0);
                    }
                }
                EditorAction::SetSurfaceMaskEnabled { surface, enabled } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        if enabled && surface.mask.points.len() < 3 {
                            surface.mask = SurfaceMask::rectangle(0.08);
                        }
                        surface.mask.enabled = enabled;
                    }
                }
                EditorAction::SetSurfaceMaskInvert { surface, invert } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.mask.invert = invert;
                    }
                }
                EditorAction::SetSurfaceMaskFeather { surface, feather } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.mask.feather = feather.clamp(0.0, 0.5);
                    }
                }
                EditorAction::SetSurfaceMaskPoint {
                    surface,
                    point,
                    value,
                } => {
                    if let Some(point) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                        .and_then(|surface| surface.mask.points.get_mut(point))
                    {
                        *point = value.clamp_unit();
                    }
                }
                EditorAction::AddSurfaceMaskPoint { surface, point } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        if surface.mask.points.len() >= MASK_POINT_LIMIT {
                            self.status_message =
                                format!("A mask can have at most {MASK_POINT_LIMIT} points");
                        } else {
                            surface.mask.points.push(point.clamp_unit());
                        }
                    }
                }
                EditorAction::RemoveSurfaceMaskPoint { surface, point } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        if point < surface.mask.points.len() {
                            surface.mask.points.remove(point);
                        }
                    }
                }
                EditorAction::ResetSurfaceMask { surface } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.mask = SurfaceMask::rectangle(0.08);
                    }
                }
                EditorAction::ClearSurfaceMask { surface } => {
                    if let Some(surface) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.surfaces.get_mut(surface))
                    {
                        surface.mask = SurfaceMask::default();
                    }
                }
                EditorAction::SetMediaPath { media, path } => {
                    let fallback = self
                        .project
                        .media
                        .get(media)
                        .map_or(MediaKind::Image, |asset| asset.kind);
                    let kind = media_kind_from_path(Path::new(&path), fallback);
                    if let Some(asset) = self.project.media.get_mut(media) {
                        asset.path.clone_from(&path);
                    }
                    self.apply_media_kind(media, kind);
                    self.ui_state.image_path = path;
                    self.reload_media();
                }
                EditorAction::RenameLayer { layer, name } => {
                    if let Some(layer) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.layers.get_mut(layer))
                    {
                        if !name.trim().is_empty() {
                            layer.name = name;
                        }
                    }
                }
                EditorAction::SetLayerMedia { layer, media } => {
                    let Some(asset) = self.project.media.get(media) else {
                        continue;
                    };
                    let source = match asset.kind {
                        MediaKind::Image => SourceSpec::Image {
                            media_id: asset.id.clone(),
                        },
                        MediaKind::Video => SourceSpec::Video {
                            media_id: asset.id.clone(),
                        },
                        MediaKind::Audio => {
                            self.status_message =
                                "Audio assets are assigned in the Audio section".to_owned();
                            continue;
                        }
                    };
                    let name = asset.name.clone();
                    let Some(layer) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.layers.get_mut(layer))
                    else {
                        continue;
                    };
                    layer.source = source;
                    layer.name = name;
                    self.reload_media();
                }
                EditorAction::SetLayerVisibility { layer, visible } => {
                    if let Some(layer) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.layers.get_mut(layer))
                    {
                        layer.visible = visible;
                    }
                }
                EditorAction::SetLayerOpacity { layer, opacity } => {
                    if let Some(layer) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.layers.get_mut(layer))
                    {
                        layer.opacity = opacity.clamp(0.0, 1.0);
                    }
                }
                EditorAction::SetLayerBlend { layer, blend } => {
                    if let Some(layer) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.layers.get_mut(layer))
                    {
                        layer.blend = blend;
                    }
                }
                EditorAction::SetLayerTestPattern { layer, pattern } => {
                    if let Some(layer) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.layers.get_mut(layer))
                    {
                        if let SourceSpec::TestPattern { pattern: current } = &mut layer.source {
                            *current = pattern;
                        }
                    }
                }
                EditorAction::SetLayerCrop { layer, crop } => {
                    if let Some(layer) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.layers.get_mut(layer))
                    {
                        layer.crop = crop.clamped();
                    }
                }
                EditorAction::SetLayerSurfaces { layer, surfaces } => {
                    let Some(stage) = self.project.primary_stage() else {
                        continue;
                    };
                    let surface_ids: Vec<_> = surfaces
                        .iter()
                        .filter_map(|index| stage.surfaces.get(*index))
                        .map(|surface| surface.id.clone())
                        .collect();
                    if surface_ids.is_empty() {
                        continue;
                    }
                    if let Some(layer) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.layers.get_mut(layer))
                    {
                        layer.surface_ids = surface_ids;
                    }
                }
                EditorAction::AddLayer { surfaces } => {
                    let Some(stage) = self.project.primary_stage_mut() else {
                        continue;
                    };
                    let surface_ids = selected_surface_ids(stage, &surfaces);
                    let Some(primary) = surface_ids.first() else {
                        continue;
                    };
                    let mut index = stage.layers.len() + 1;
                    let id = loop {
                        let candidate = format!("layer-{index}");
                        if !stage
                            .layers
                            .iter()
                            .any(|layer| layer.id.as_str() == candidate)
                        {
                            break candidate;
                        }
                        index += 1;
                    };
                    let mut layer =
                        Layer::test_pattern(id, format!("Test pattern {index}"), primary);
                    layer.surface_ids = surface_ids;
                    layer.opacity = 0.45;
                    layer.blend = BlendMode::Screen;
                    stage.layers.push(layer);
                    self.ui_state.selected_layer = stage.layers.len() - 1;
                }
                EditorAction::AddLayerFromMedia { surfaces, media } => {
                    let Some(asset) = self.project.media.get(media) else {
                        continue;
                    };
                    if asset.kind == MediaKind::Audio {
                        self.status_message =
                            "Audio assets are assigned in the Show tab".to_owned();
                        continue;
                    }
                    let media_id = asset.id.clone();
                    let name = asset.name.clone();
                    let kind = asset.kind;
                    let Some(stage) = self.project.primary_stage_mut() else {
                        continue;
                    };
                    let surface_ids = selected_surface_ids(stage, &surfaces);
                    let Some(primary) = surface_ids.first() else {
                        continue;
                    };
                    let mut index = stage.layers.len() + 1;
                    let id = loop {
                        let candidate = format!("layer-{index}");
                        if !stage
                            .layers
                            .iter()
                            .any(|layer| layer.id.as_str() == candidate)
                        {
                            break candidate;
                        }
                        index += 1;
                    };
                    let mut layer = match kind {
                        MediaKind::Image => Layer::image(id, name, primary, &media_id),
                        MediaKind::Video => Layer::video(id, name, primary, &media_id),
                        MediaKind::Audio => unreachable!("audio is rejected above"),
                    };
                    layer.surface_ids = surface_ids;
                    stage.layers.push(layer);
                    self.ui_state.selected_layer = stage.layers.len() - 1;
                    self.reload_media();
                }
                EditorAction::DeleteLayer { layer } => {
                    let remaining = {
                        let Some(stage) = self.project.primary_stage_mut() else {
                            continue;
                        };
                        if stage.layers.len() <= 1 || layer >= stage.layers.len() {
                            continue;
                        }
                        stage.layers.remove(layer);
                        stage.layers.len()
                    };
                    let selected = self.ui_state.selected_layer;
                    self.ui_state.selected_layer = if selected > layer {
                        selected - 1
                    } else {
                        selected.min(remaining.saturating_sub(1))
                    };
                }
                EditorAction::ReorderLayer { layer, offset } => {
                    let target = layer as isize + offset as isize;
                    let moved = {
                        let Some(stage) = self.project.primary_stage_mut() else {
                            continue;
                        };
                        if layer >= stage.layers.len()
                            || target < 0
                            || target >= stage.layers.len() as isize
                        {
                            continue;
                        }
                        stage.layers.swap(layer, target as usize);
                        target as usize
                    };
                    if self.ui_state.selected_layer == layer {
                        self.ui_state.selected_layer = moved;
                    }
                }
                EditorAction::ToggleTransport => {
                    self.clock.toggle();
                    log::info!(
                        "transport {} at {:.3}s",
                        if self.clock.is_playing() {
                            "play"
                        } else {
                            "pause"
                        },
                        self.clock.position_seconds()
                    );
                }
                EditorAction::RestartTransport => {
                    self.clock.restart();
                    log::info!("transport restart");
                }
                EditorAction::SetTransportLoop { looping } => self.clock.set_looping(looping),
                EditorAction::SeekTransport { position } => {
                    self.clock.seek(position);
                    log::info!("transport seek to {:.3}s", self.clock.position_seconds());
                }
                EditorAction::SetShowAudio { media } => {
                    let selected_layer = self.ui_state.selected_layer;
                    if let Some(media) = media {
                        let Some(asset) = self.project.media.get(media) else {
                            continue;
                        };
                        if asset.kind != MediaKind::Audio {
                            self.status_message =
                                "Only audio assets can be used as the show track".to_owned();
                            continue;
                        }
                        self.project.audio.media_id = Some(asset.id.clone());
                    } else {
                        self.project.audio.media_id = None;
                    }
                    self.reload_media();
                    self.keep_selected_layer(selected_layer);
                }
                EditorAction::SetShowAudioVolume { volume } => {
                    let volume = volume.clamp(0.0, 1.0);
                    self.project.audio.volume = volume;
                    self.audio.set_volume(volume);
                }
                EditorAction::SetShowAudioMuted { muted } => {
                    self.project.audio.muted = muted;
                    self.audio.set_muted(muted);
                }
                EditorAction::AddOutput => {
                    let Some(stage_id) = self.project.primary_stage().map(|stage| stage.id.clone())
                    else {
                        continue;
                    };
                    let mut index = self.project.outputs.len() + 1;
                    let id = loop {
                        let candidate = format!("output-{index}");
                        if !self
                            .project
                            .outputs
                            .iter()
                            .any(|output| output.id.as_str() == candidate)
                        {
                            break candidate;
                        }
                        index += 1;
                    };
                    let mut spec = OutputSpec::full_stage(id, format!("Output {index}"), &stage_id);
                    spec.target = OutputTarget::Windowed {
                        width: 960,
                        height: 540,
                    };
                    self.project.outputs.push(spec);
                    self.ui_state.selected_output = self.project.outputs.len() - 1;
                    self.pending_output_sync = true;
                }
                EditorAction::DeleteOutput { output } => {
                    if output >= self.project.outputs.len() || self.project.outputs.len() <= 1 {
                        continue;
                    }
                    let removed = self.project.outputs.remove(output);
                    self.project.remove_output_from_groups(&removed.id);
                    self.outputs.retain(|window| window.id != removed.id);
                    self.output_guides.remove(&removed.id);
                    self.ui_state.selected_output = self
                        .ui_state
                        .selected_output
                        .min(self.project.outputs.len() - 1);
                    self.pending_output_sync = true;
                }
                EditorAction::SetOutputView { output, view } => {
                    if let Some(spec) = self.project.outputs.get_mut(output) {
                        spec.view = view;
                    }
                }
                EditorAction::SetOutputEnabled { output, enabled } => {
                    if let Some(spec) = self.project.outputs.get_mut(output) {
                        spec.enabled = enabled;
                    }
                    self.pending_output_sync = true;
                }
                EditorAction::SetOutputEdgeBlend { output, edge_blend } => {
                    if let Some(spec) = self.project.outputs.get_mut(output) {
                        spec.edge_blend = edge_blend.clamped();
                    }
                }
                EditorAction::SetOutputColor { output, color } => {
                    if let Some(spec) = self.project.outputs.get_mut(output) {
                        spec.color = color.clamped();
                    }
                }
                EditorAction::LoadOutputLut { output } => {
                    let dialog = rfd::FileDialog::new()
                        .set_directory(&self.media_base)
                        .add_filter("1D calibration LUT", &["cube"]);
                    let Some(path) = dialog.pick_file() else {
                        continue;
                    };
                    let fallback_name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("calibration.cube");
                    let source = match std::fs::read_to_string(&path) {
                        Ok(source) => source,
                        Err(error) => {
                            self.status_message =
                                format!("LUT load failed: {}: {error}", path.display());
                            log::warn!("failed to read LUT {}: {error}", path.display());
                            continue;
                        }
                    };
                    let lut = match parse_cube_1d(&source, fallback_name) {
                        Ok(lut) => lut,
                        Err(error) => {
                            self.status_message = format!("LUT load failed: {error}");
                            log::warn!("failed to parse LUT {}: {error}", path.display());
                            continue;
                        }
                    };
                    let name = lut.name.clone();
                    if let Some(spec) = self.project.outputs.get_mut(output) {
                        spec.calibration = Some(lut);
                    }
                    self.status_message = format!("Loaded LUT '{name}'");
                    log::info!("loaded calibration LUT '{name}' from {}", path.display());
                }
                EditorAction::ClearOutputLut { output } => {
                    if let Some(spec) = self.project.outputs.get_mut(output) {
                        spec.calibration = None;
                        self.status_message = "Cleared calibration LUT".to_owned();
                    }
                }
                EditorAction::SetOutputGuide { output, guide } => {
                    let Some(id) = self.project.outputs.get(output).map(|spec| spec.id.clone())
                    else {
                        continue;
                    };
                    if guide == GuideMode::None {
                        self.output_guides.remove(&id);
                    } else {
                        self.output_guides.insert(id, guide);
                    }
                    self.status_message = format!("Output guide: {}", guide.label());
                }
                EditorAction::SetOutputMonitor { output, monitor } => {
                    let Some(spec) = self.project.outputs.get_mut(output) else {
                        continue;
                    };
                    spec.target = OutputTarget::Fullscreen { monitor };
                    let id = spec.id.clone();
                    self.outputs.retain(|window| window.id != id);
                    self.pending_output_sync = true;
                }
                EditorAction::SetOutputFullscreen { output, fullscreen } => {
                    let Some(spec) = self.project.outputs.get_mut(output) else {
                        continue;
                    };
                    spec.target = if fullscreen {
                        OutputTarget::Fullscreen { monitor: None }
                    } else {
                        OutputTarget::Windowed {
                            width: 960,
                            height: 540,
                        }
                    };
                    // Fullscreen and windowed attributes differ, so the
                    // window has to be recreated with the new target.
                    let id = spec.id.clone();
                    self.outputs.retain(|window| window.id != id);
                    self.pending_output_sync = true;
                }
                EditorAction::AddOutputGroup => {
                    let mut index = self.project.output_groups.len() + 1;
                    let id = loop {
                        let candidate = format!("group-{index}");
                        if !self
                            .project
                            .output_groups
                            .iter()
                            .any(|group| group.id.as_str() == candidate)
                        {
                            break candidate;
                        }
                        index += 1;
                    };
                    self.project.output_groups.push(OutputGroup::new(
                        id,
                        format!("Group {index}"),
                        [],
                    ));
                    self.ui_state.selected_group = self.project.output_groups.len() - 1;
                }
                EditorAction::DeleteOutputGroup { group } => {
                    if group >= self.project.output_groups.len() {
                        continue;
                    }
                    self.project.output_groups.remove(group);
                    self.ui_state.selected_group = self
                        .ui_state
                        .selected_group
                        .min(self.project.output_groups.len().saturating_sub(1));
                }
                EditorAction::SetOutputGroupMember {
                    group,
                    output,
                    member,
                } => {
                    let Some(output_id) =
                        self.project.outputs.get(output).map(|spec| spec.id.clone())
                    else {
                        continue;
                    };
                    if member {
                        self.project.remove_output_from_groups(&output_id);
                        if let Some(group) = self.project.output_groups.get_mut(group) {
                            group.outputs.push(output_id);
                        }
                    } else if let Some(group) = self.project.output_groups.get_mut(group) {
                        group.outputs.retain(|id| id != &output_id);
                    }
                }
                EditorAction::ReloadMedia => self.reload_media(),
                EditorAction::Undo => self.undo(),
                EditorAction::Redo => self.redo(),
                EditorAction::SaveProject { path } => {
                    if let Err(error) = self.save_project(&path) {
                        log::error!("failed to save project: {error:#}");
                        self.status_message = format!("Save failed: {error}");
                    }
                }
                EditorAction::SaveProjectDialog => {
                    let current = PathBuf::from(&self.ui_state.project_path);
                    let file_name = current
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("project.lpmt.json")
                        .to_owned();
                    let mut dialog = rfd::FileDialog::new()
                        .add_filter("LPMT project", &["json"])
                        .set_file_name(&file_name);
                    dialog = if let Some(parent) = current
                        .parent()
                        .filter(|parent| !parent.as_os_str().is_empty())
                    {
                        dialog.set_directory(parent)
                    } else {
                        dialog.set_directory(&self.media_base)
                    };
                    if let Some(mut path) = dialog.save_file() {
                        if !path
                            .extension()
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
                        {
                            path.set_extension("json");
                        }
                        let path = path.display().to_string();
                        if let Err(error) = self.save_project(&path) {
                            log::error!("failed to save project: {error:#}");
                            self.status_message = format!("Save failed: {error}");
                        }
                    }
                }
                EditorAction::LoadProject { path } => {
                    if let Err(error) = self.load_project(&path) {
                        log::error!("failed to load project: {error:#}");
                        self.status_message = format!("Load failed: {error}");
                    }
                }
                EditorAction::OpenProjectDialog => {
                    let current = PathBuf::from(&self.ui_state.project_path);
                    let mut dialog = rfd::FileDialog::new().add_filter("LPMT project", &["json"]);
                    dialog = if let Some(parent) = current
                        .parent()
                        .filter(|parent| !parent.as_os_str().is_empty())
                    {
                        dialog.set_directory(parent)
                    } else {
                        dialog.set_directory(&self.media_base)
                    };
                    if let Some(path) = dialog.pick_file() {
                        let path = path.display().to_string();
                        if let Err(error) = self.load_project(&path) {
                            log::error!("failed to load project: {error:#}");
                            self.status_message = format!("Load failed: {error}");
                        }
                    }
                }
                EditorAction::AddMedia => {
                    let selected_layer = self.ui_state.selected_layer;
                    let dialog = media_file_dialog(&self.media_base);
                    if let Some(path) = dialog.pick_file() {
                        self.record_history("add media");
                        let stored = self.stored_media_path(&path);
                        let kind = media_kind_from_path(&path, MediaKind::Image);
                        let name = path
                            .file_stem()
                            .and_then(|name| name.to_str())
                            .unwrap_or("Media")
                            .to_owned();
                        let mut index = self.project.media.len() + 1;
                        let id = loop {
                            let candidate = format!("media-{index}");
                            if !self
                                .project
                                .media
                                .iter()
                                .any(|asset| asset.id.as_str() == candidate)
                            {
                                break candidate;
                            }
                            index += 1;
                        };
                        let asset = match kind {
                            MediaKind::Image => MediaAsset::image(id, name, stored.clone()),
                            MediaKind::Video => MediaAsset::video(id, name, stored.clone()),
                            MediaKind::Audio => MediaAsset::audio(id, name, stored.clone()),
                        };
                        self.project.media.push(asset);
                        self.ui_state.selected_media = self.project.media.len() - 1;
                        self.ui_state.image_path = stored;
                        self.reload_media();
                        self.keep_selected_layer(selected_layer);
                    }
                }
                EditorAction::RemoveMedia { media } => {
                    let Some(asset) = self.project.media.get(media) else {
                        continue;
                    };
                    if self.project.is_media_used(&asset.id) {
                        self.status_message =
                            "That asset is used by a layer or the show audio; reassign it first"
                                .to_owned();
                        continue;
                    }
                    self.project.media.remove(media);
                    self.ui_state.selected_media = self
                        .ui_state
                        .selected_media
                        .min(self.project.media.len().saturating_sub(1));
                    self.ui_state.image_path = self
                        .project
                        .media
                        .get(self.ui_state.selected_media)
                        .map_or_else(String::new, |asset| asset.path.clone());
                    self.reload_media();
                }
                EditorAction::PickMediaFile { media } => {
                    let Some(asset) = self.project.media.get(media) else {
                        continue;
                    };
                    let selected_layer = self.ui_state.selected_layer;
                    let fallback = asset.kind;
                    let dialog = media_file_dialog(&self.media_base);
                    if let Some(path) = dialog.pick_file() {
                        let before = self.project.clone();
                        let stored = self.stored_media_path(&path);
                        let kind = media_kind_from_path(&path, fallback);
                        if let Some(asset) = self.project.media.get_mut(media) {
                            asset.path = stored.clone();
                        }
                        self.apply_media_kind(media, kind);
                        self.ui_state.image_path = stored;
                        self.reload_media();
                        self.keep_selected_layer(selected_layer);
                        if before != self.project {
                            self.history
                                .record(before, HistoryKey::Discrete, "replace media");
                            self.history.break_coalescing();
                        }
                    }
                }
            }
        }
        if let Some((before, key, label)) = pending {
            self.finish_history(before, key, label);
        }
    }

    fn finish_history(&mut self, before: Project, key: HistoryKey, label: &'static str) {
        if before != self.project {
            self.history.record(before, key, label);
        }
    }

    /// Media loading must not change which layer the inspector is editing.
    fn keep_selected_layer(&mut self, layer: usize) {
        let count = self
            .project
            .primary_stage()
            .map_or(0, |stage| stage.layers.len());
        self.ui_state.selected_layer = layer.min(count.saturating_sub(1));
    }

    /// Record a snapshot for an edit whose actual mutation happens after a
    /// dialog or another operation that may be cancelled.
    fn record_history(&mut self, label: &'static str) {
        self.history
            .record(self.project.clone(), HistoryKey::Discrete, label);
        self.history.break_coalescing();
    }

    fn undo(&mut self) {
        let current = self.project.clone();
        if let Some((project, label)) = self.history.undo(current) {
            self.restore_project(project);
            self.status_message = format!("Undid {label}");
        }
    }

    fn redo(&mut self) {
        let current = self.project.clone();
        if let Some((project, label)) = self.history.redo(current) {
            self.restore_project(project);
            self.status_message = format!("Redid {label}");
        }
    }

    fn restore_project(&mut self, project: Project) {
        let media_changed = project.media != self.project.media
            || project.audio != self.project.audio
            || used_media_ids(&project) != used_media_ids(&self.project);
        self.project = project;
        self.ui_state.reset_text_buffers();
        let surface_count = self
            .project
            .primary_stage()
            .map_or(0, |stage| stage.surfaces.len());
        self.ui_state.normalize_surface_selection(surface_count);
        self.ui_state.selected_layer = self.ui_state.selected_layer.min(
            self.project
                .primary_stage()
                .map_or(0, |stage| stage.layers.len().saturating_sub(1)),
        );
        self.ui_state.selected_media = self
            .ui_state
            .selected_media
            .min(self.project.media.len().saturating_sub(1));
        self.ui_state.selected_output = self
            .ui_state
            .selected_output
            .min(self.project.outputs.len().saturating_sub(1));
        self.ui_state.selected_group = self
            .ui_state
            .selected_group
            .min(self.project.output_groups.len().saturating_sub(1));
        self.ui_state.cancel_drag();
        self.pending_output_sync = true;
        if media_changed {
            self.reload_media();
        }
    }

    /// Keep layers pointing at a media asset valid when the file type changes
    /// between image, video and audio.
    fn apply_media_kind(&mut self, media: usize, kind: MediaKind) {
        let Some(media_id) = self.project.media.get(media).map(|asset| asset.id.clone()) else {
            return;
        };
        if let Some(asset) = self.project.media.get_mut(media) {
            asset.kind = kind;
        }
        if kind != MediaKind::Audio && self.project.audio.media_id.as_ref() == Some(&media_id) {
            self.project.audio.media_id = None;
        }
        let Some(stage) = self.project.primary_stage_mut() else {
            return;
        };
        for layer in &mut stage.layers {
            let uses_media = matches!(
                &layer.source,
                SourceSpec::Image { media_id: id } | SourceSpec::Video { media_id: id }
                    if id == &media_id
            );
            if uses_media {
                layer.source = match kind {
                    MediaKind::Image => SourceSpec::Image {
                        media_id: media_id.clone(),
                    },
                    MediaKind::Video => SourceSpec::Video {
                        media_id: media_id.clone(),
                    },
                    MediaKind::Audio => SourceSpec::TestPattern {
                        pattern: TestPattern::Checker,
                    },
                };
            }
        }
    }

    fn reload_media(&mut self) {
        let App {
            gpu,
            media,
            project,
            videos,
            audio,
            media_base,
            ..
        } = self;

        media.clear();
        let (video_loaded, _) = videos.load(project, media_base);
        let mut loaded = video_loaded;
        let (audio_loaded, audio_failed) = audio.load(project, media_base);
        if audio_loaded {
            loaded += 1;
        }
        let mut failed_names: Vec<String> = project
            .media
            .iter()
            .filter(|asset| asset.kind == MediaKind::Video && project.is_media_used(&asset.id))
            .filter(|asset| !videos.is_loaded(&asset.id))
            .map(|asset| asset.name.clone())
            .collect();
        if let Some(name) = audio_failed {
            failed_names.push(name);
        }

        for asset in &project.media {
            // Load every image, even when unused: the media pool and the
            // layer picker show a thumbnail for each visual asset.
            if asset.kind != MediaKind::Image {
                continue;
            }

            let path = if Path::new(&asset.path).is_absolute() {
                PathBuf::from(&asset.path)
            } else {
                media_base.join(&asset.path)
            };

            match lpmt_media::load_image(&path) {
                Ok(image) => match media.upload_image(gpu, asset.id.clone(), &image) {
                    Ok(()) => {
                        loaded += 1;
                        log::info!(
                            "loaded media '{}' from {} ({}x{})",
                            asset.name,
                            path.display(),
                            image.width,
                            image.height
                        );
                    }
                    Err(error) => {
                        failed_names.push(asset.name.clone());
                        log::error!(
                            "failed to upload media '{}' ({}): {error:#}",
                            asset.name,
                            path.display()
                        );
                    }
                },
                Err(error) => {
                    failed_names.push(asset.name.clone());
                    log::warn!(
                        "media '{}' could not be loaded from {}: {error:#}",
                        asset.name,
                        path.display()
                    );
                }
            }
        }

        self.status_message = if failed_names.is_empty() {
            format!("Loaded {loaded} media asset(s)")
        } else {
            let mut hints = Vec::new();
            if !cfg!(feature = "video")
                && project
                    .media
                    .iter()
                    .any(|asset| asset.kind == MediaKind::Video && project.is_media_used(&asset.id))
            {
                hints.push("rebuild with --features video");
            }
            if !cfg!(feature = "audio") && project.audio_asset().is_some() {
                hints.push("rebuild with --features audio");
            }
            let hint = if hints.is_empty() {
                String::new()
            } else {
                format!(" · {}", hints.join(" · "))
            };
            format!(
                "Loaded {loaded}, failed {}: {}{hint}",
                failed_names.len(),
                failed_names.join(", ")
            )
        };

        let duration = match (videos.max_duration(), audio.max_duration()) {
            (Some(video), Some(audio)) => Some(video.max(audio)),
            (Some(video), None) => Some(video),
            (None, Some(audio)) => Some(audio),
            (None, None) => None,
        };
        self.clock.set_duration(duration);
    }

    /// Store media paths relative to the project when possible, so a show
    /// directory stays movable. Files outside the project directory stay
    /// absolute.
    fn stored_media_path(&self, path: &Path) -> String {
        let base =
            std::fs::canonicalize(&self.media_base).unwrap_or_else(|_| self.media_base.clone());
        let absolute = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        absolute
            .strip_prefix(&base)
            .map(|relative| relative.to_string_lossy().into_owned())
            .unwrap_or_else(|_| absolute.display().to_string())
    }

    fn save_project(&mut self, path: &str) -> Result<()> {
        let path = PathBuf::from(path);
        let json =
            serde_json::to_string_pretty(&self.project).context("failed to serialize project")?;
        project_io::write_atomic(&path, json.as_bytes())?;
        self.ui_state.project_path = path.display().to_string();
        self.saved_project = self.project.clone();
        self.status_message = format!("Saved {}", path.display());
        log::info!("saved project to {}", path.display());
        Ok(())
    }

    fn load_project(&mut self, path: &str) -> Result<()> {
        let path = PathBuf::from(path);
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let project: Project =
            serde_json::from_str(&json).context("failed to parse project JSON")?;
        anyhow::ensure!(
            project.schema_version <= 2,
            "project schema version {} is newer than this build supports",
            project.schema_version
        );

        self.project = project;
        self.saved_project = self.project.clone();
        self.history.clear();
        self.output_guides.clear();
        self.ui_state.project_path = path.display().to_string();
        self.media_base = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        self.ui_state.selected_surface = 0;
        self.ui_state.normalize_surface_selection(
            self.project
                .primary_stage()
                .map_or(0, |stage| stage.surfaces.len()),
        );
        self.ui_state.selected_layer = 0;
        self.ui_state.selected_media = 0;
        self.ui_state.selected_output = 0;
        self.ui_state.selected_group = 0;
        self.ui_state.reset_text_buffers();
        self.pending_output_sync = true;
        self.ui_state.image_path = self
            .project
            .media
            .first()
            .map_or_else(String::new, |asset| asset.path.clone());
        self.reload_media();
        let media_status = self.status_message.clone();
        self.status_message = format!("Loaded {} · {media_status}", path.display());
        log::info!("loaded project from {}", path.display());
        Ok(())
    }

    fn status(&self) -> EditorStatus {
        let output = if self.outputs.is_empty() {
            "No output windows".to_owned()
        } else {
            let mut parts = Vec::with_capacity(self.outputs.len());
            for window in &self.outputs {
                let (width, height) = window.renderer.size();
                let name = self
                    .project
                    .output(&window.id)
                    .map_or_else(|| window.id.to_string(), |spec| spec.name.clone());
                parts.push(format!("{name} {width}×{height}"));
            }
            format!(
                "{} open / {} defined · {}",
                self.outputs.len(),
                self.project.outputs.len(),
                parts.join(" · ")
            )
        };
        let transport = editor::TransportStatus {
            playing: self.clock.is_playing(),
            looping: self.clock.is_looping(),
            position_seconds: self.clock.position_seconds(),
            duration_seconds: self.clock.duration_seconds(),
        };

        EditorStatus {
            adapter: self.gpu.adapter_info().name.clone(),
            output,
            fps: self.fps,
            frame_ms: self.frame_ms,
            message: self.status_message.clone(),
            transport,
            video_supported: cfg!(feature = "video"),
            audio_supported: cfg!(feature = "audio"),
            audio_error: self.audio.error().map(str::to_owned),
            monitors: self.monitors.clone(),
            dirty: self.project != self.saved_project,
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            undo_label: self.history.undo_label().map(str::to_owned),
            redo_label: self.history.redo_label().map(str::to_owned),
            output_pacing: self
                .project
                .outputs
                .iter()
                .map(|spec| {
                    self.outputs
                        .iter()
                        .find(|window| window.id == spec.id)
                        .map_or(OutputPacing::default(), OutputWindow::pacing_status)
                })
                .collect(),
            osc: editor::OscStatus {
                address: self.osc_address.clone(),
                listening: self.osc.is_some(),
                messages: self.osc_messages,
                last: self.osc_last.clone(),
                error: self.osc_error.clone(),
            },
            output_guides: self
                .project
                .outputs
                .iter()
                .map(|spec| {
                    self.output_guides
                        .get(&spec.id)
                        .copied()
                        .unwrap_or(GuideMode::None)
                })
                .collect(),
            media_thumbnails: self.media_thumbnails.clone(),
        }
    }

    /// Register one reusable egui texture per loaded media asset for the media
    /// pool and layer picker. Video textures update in place, so the same
    /// registration keeps showing the latest decoded frame.
    fn sync_media_thumbnails(&mut self) {
        let revision = self.media.revision();
        if revision == self.thumbnail_revision {
            return;
        }

        let App {
            gpu,
            media,
            project,
            editor,
            media_thumbnails,
            thumbnail_revision,
            ..
        } = self;
        let Some(editor) = editor.as_mut() else {
            return;
        };

        for texture in media_thumbnails.values() {
            editor.egui_renderer.free_texture(texture);
        }
        media_thumbnails.clear();
        for asset in project
            .media
            .iter()
            .filter(|asset| asset.kind != MediaKind::Audio)
        {
            if let Some(view) = media.texture_view(&asset.id) {
                let texture = editor.egui_renderer.register_native_texture(
                    gpu.device(),
                    view,
                    wgpu::FilterMode::Linear,
                );
                media_thumbnails.insert(asset.id.clone(), texture);
            }
        }
        *thumbnail_revision = revision;
    }

    /// Keep the offscreen preview matched to the current stage aspect.
    fn ensure_preview(&mut self) -> Result<()> {
        let Some(stage) = self.project.primary_stage() else {
            self.clear_preview();
            return Ok(());
        };
        let width = 960u32;
        let height = ((width as f32 / stage.aspect.max(0.1)).round() as u32).max(1);
        if self
            .preview
            .as_ref()
            .is_some_and(|preview| preview.size() == (width, height))
        {
            return Ok(());
        }

        let renderer = PreviewRenderer::new(&self.gpu, &self.media, width, height)?;
        let App {
            editor,
            preview,
            preview_texture,
            ui_state,
            ..
        } = self;
        let Some(editor) = editor.as_mut() else {
            return Ok(());
        };
        let texture_id = editor.egui_renderer.register_native_texture(
            self.gpu.device(),
            renderer.view(),
            wgpu::FilterMode::Linear,
        );
        if let Some(old) = preview_texture.replace(texture_id) {
            editor.egui_renderer.free_texture(&old);
        }
        *preview = Some(renderer);
        ui_state.preview_texture = Some(texture_id);
        Ok(())
    }

    fn clear_preview(&mut self) {
        if let Some(texture) = self.preview_texture.take() {
            if let Some(editor) = self.editor.as_mut() {
                editor.egui_renderer.free_texture(&texture);
            }
        }
        self.preview = None;
        self.ui_state.preview_texture = None;
    }

    fn render_preview(&mut self) {
        if let Err(error) = self.ensure_preview() {
            log::error!("failed to create preview: {error:#}");
            self.clear_preview();
            return;
        }
        let App {
            gpu,
            media,
            project,
            preview,
            ..
        } = self;
        let Some(stage) = project.primary_stage() else {
            return;
        };
        let Some(preview) = preview.as_mut() else {
            return;
        };
        if let Err(error) = preview.render(gpu, media, stage) {
            log::error!("preview render failed: {error:#}");
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        self.clock.advance(now);
        let elapsed = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        if elapsed > 0.0 {
            let instantaneous = 1.0 / elapsed;
            self.fps = if self.fps == 0.0 {
                instantaneous
            } else {
                self.fps * 0.9 + instantaneous * 0.1
            };
            self.frame_ms = self.frame_ms * 0.9 + elapsed * 1000.0 * 0.1;
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.editor.is_none() {
            if let Err(error) = self.create_windows(event_loop) {
                log::error!("failed to create windows: {error:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .editor
            .as_ref()
            .is_some_and(|editor| editor.window.id() == window_id)
        {
            self.handle_editor_event(event_loop, event);
            return;
        }

        if let Some(index) = self
            .outputs
            .iter()
            .position(|output| output.window.id() == window_id)
        {
            self.handle_output_event(index, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.tick();
        self.poll_osc();

        if self.pending_output_sync {
            self.pending_output_sync = false;
            self.sync_output_windows(event_loop);
        }

        // Decode once per application tick and share the resulting textures
        // with every output, instead of advancing the player per window. The
        // show clock is authoritative: each decoder is corrected before the
        // newly decoded frame is uploaded.
        let seek_serial = self.clock.seek_serial();
        let force_seek = seek_serial != self.last_clock_seek;
        self.last_clock_seek = seek_serial;
        let loop_serial = self.clock.loop_serial();
        let loop_wrap = loop_serial != self.last_clock_loop;
        self.last_clock_loop = loop_serial;
        self.videos.sync_to_clock(
            self.clock.position_seconds(),
            self.clock.is_playing(),
            self.clock.is_looping(),
            force_seek,
        );
        self.audio.sync_to_clock(
            self.clock.position_seconds(),
            self.clock.is_playing(),
            force_seek || loop_wrap,
        );
        self.videos.poll(&self.gpu, &mut self.media);

        self.render_preview();
        self.render_outputs();

        if let Some(editor) = self.editor.as_ref() {
            editor.window.request_redraw();
        }
    }
}

impl EditorWindow {
    fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.config = configure_egui_surface(gpu, &self.surface, width, height)?;
        Ok(())
    }

    fn reconfigure(&mut self, gpu: &Gpu) -> Result<()> {
        self.config =
            configure_egui_surface(gpu, &self.surface, self.config.width, self.config.height)?;
        Ok(())
    }
}

fn create_output_window(
    gpu: &Gpu,
    media: &MediaCache,
    event_loop: &ActiveEventLoop,
    spec: &OutputSpec,
    used_monitors: &[String],
    editor_monitor: Option<&str>,
    cascade: i32,
) -> Result<OutputWindow> {
    let mut attributes = Window::default_attributes()
        .with_title(format!("LPMT — {}", spec.name))
        .with_active(false);
    let primary_name = event_loop
        .primary_monitor()
        .and_then(|monitor| monitor.name());
    let mut preferred_present_mode = None;

    match &spec.target {
        OutputTarget::Windowed { width, height } => {
            attributes = attributes
                .with_inner_size(PhysicalSize::new((*width).max(1), (*height).max(1)))
                .with_position(PhysicalPosition::new(80 + cascade * 40, 80 + cascade * 40));
        }
        OutputTarget::Fullscreen { monitor } => {
            let monitor = choose_monitor(
                event_loop,
                monitor.as_deref(),
                used_monitors,
                editor_monitor,
            );
            if let Some(monitor) = monitor {
                let monitor_name = monitor.name();
                let on_editor_display =
                    editor_monitor.is_some() && editor_monitor == monitor_name.as_deref();
                let on_primary = primary_name.as_deref() == monitor_name.as_deref();
                if !on_primary && !on_editor_display {
                    // Dedicated projector displays can stay tear-free. On the
                    // primary/editor display use the non-blocking fallback,
                    // even if the user explicitly requested fullscreen.
                    preferred_present_mode = Some(wgpu::PresentMode::Fifo);
                }
                log::info!(
                    "placing output '{}' fullscreen on monitor {:?}",
                    spec.name,
                    monitor.name()
                );
                attributes =
                    attributes.with_fullscreen(Some(Fullscreen::Borderless(Some(monitor))));
            } else {
                log::info!(
                    "no secondary monitor available for output '{}'; opening it windowed",
                    spec.name
                );
                attributes = attributes
                    .with_inner_size(PhysicalSize::new(1280, 720))
                    .with_position(PhysicalPosition::new(80 + cascade * 40, 80 + cascade * 40));
            }
        }
    }

    let window = Arc::new(
        event_loop
            .create_window(attributes)
            .with_context(|| format!("failed to create window for output '{}'", spec.name))?,
    );
    let surface = gpu
        .instance()
        .create_surface(window.clone())
        .with_context(|| format!("failed to create surface for output '{}'", spec.name))?;
    let size = window.inner_size();
    let renderer = OutputRenderer::new(
        gpu,
        media,
        surface,
        window.clone(),
        size.width,
        size.height,
        preferred_present_mode,
    )?;

    Ok(OutputWindow {
        id: spec.id.clone(),
        window,
        renderer,
        pacing: OutputPacing::default(),
        last_presented: None,
        occluded: false,
    })
}

/// Pick a display for a fullscreen output: an explicit monitor name wins,
/// otherwise prefer an unused secondary monitor so several projectors do not
/// land on the same screen.
fn choose_monitor(
    event_loop: &ActiveEventLoop,
    preferred: Option<&str>,
    used: &[String],
    editor_monitor: Option<&str>,
) -> Option<MonitorHandle> {
    let monitors: Vec<MonitorHandle> = event_loop.available_monitors().collect();
    if let Some(preferred) = preferred {
        if let Some(monitor) = monitors
            .iter()
            .find(|monitor| monitor.name().as_deref() == Some(preferred))
        {
            return Some(monitor.clone());
        }
    }

    // With one display there is no safe fullscreen target: it would cover the
    // editor. If the platform does not expose a primary monitor, treat the
    // first monitor as the editor-owned display rather than guessing.
    if monitors.len() < 2 {
        return None;
    }
    let primary_name = event_loop
        .primary_monitor()
        .and_then(|monitor| monitor.name())
        .or_else(|| monitors.first().and_then(|monitor| monitor.name()));
    monitors
        .iter()
        .find(|monitor| {
            let name = monitor.name();
            let is_editor = editor_monitor.is_some() && editor_monitor == name.as_deref();
            let is_primary = primary_name.as_deref() == name.as_deref();
            let is_used = used
                .iter()
                .any(|used_name| name.as_deref() == Some(used_name.as_str()));
            !is_editor && !is_primary && !is_used
        })
        .cloned()
}

fn require_float(value: &ParameterValue, path: &str) -> Result<f32, String> {
    value
        .as_f32()
        .ok_or_else(|| format!("{path} expects a finite number"))
}

fn require_bool(value: &ParameterValue, path: &str) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("{path} expects a boolean or 0/1"))
}

fn history_key(action: &EditorAction) -> Option<(HistoryKey, &'static str)> {
    use EditorAction::*;
    let key = match action {
        SetCorner {
            surface, corner, ..
        } => (
            HistoryKey::SurfaceCorner(*surface, *corner),
            "move surface corner",
        ),
        SetSurfaceCorners { edit, .. } => match edit {
            SurfaceGeometryEdit::Move => (HistoryKey::SurfaceGeometry, "move surfaces"),
            SurfaceGeometryEdit::Align => (HistoryKey::Discrete, "align surfaces"),
            SurfaceGeometryEdit::Distribute => (HistoryKey::Discrete, "distribute surfaces"),
        },
        SetSurfaceGridEnabled { .. } => (HistoryKey::Discrete, "toggle grid warp"),
        SetSurfaceGridResolution { .. } => (HistoryKey::Discrete, "change grid resolution"),
        ResetSurfaceGrid { .. } => (HistoryKey::Discrete, "reset grid warp"),
        SetGridPoint {
            surface,
            row,
            column,
            ..
        } => (
            HistoryKey::SurfaceGridPoint(*surface, *row, *column),
            "move grid point",
        ),
        ResetSurface { .. } => (HistoryKey::Discrete, "reset surface"),
        AddSurface => (HistoryKey::Discrete, "add surface"),
        DeleteSurface { .. } => (HistoryKey::Discrete, "delete surface"),
        RenameSurface { surface, .. } => (HistoryKey::SurfaceName(*surface), "rename surface"),
        SetSurfaceVisibility { .. } => (HistoryKey::Discrete, "toggle surface visibility"),
        SetSurfaceOpacity { surface, .. } => (
            HistoryKey::SurfaceOpacity(*surface),
            "change surface opacity",
        ),
        SetSurfaceMaskEnabled { .. } => (HistoryKey::Discrete, "toggle surface mask"),
        SetSurfaceMaskInvert { .. } => (HistoryKey::Discrete, "invert surface mask"),
        SetSurfaceMaskFeather { surface, .. } => (
            HistoryKey::SurfaceMaskFeather(*surface),
            "change mask feather",
        ),
        SetSurfaceMaskPoint { surface, point, .. } => (
            HistoryKey::SurfaceMaskPoint(*surface, *point),
            "move mask point",
        ),
        AddSurfaceMaskPoint { .. } => (HistoryKey::Discrete, "add mask point"),
        RemoveSurfaceMaskPoint { .. } => (HistoryKey::Discrete, "remove mask point"),
        ResetSurfaceMask { .. } => (HistoryKey::Discrete, "reset surface mask"),
        ClearSurfaceMask { .. } => (HistoryKey::Discrete, "clear surface mask"),
        SetMediaPath { media, .. } => (HistoryKey::MediaPath(*media), "change media path"),
        RenameLayer { layer, .. } => (HistoryKey::LayerName(*layer), "rename layer"),
        SetLayerMedia { .. } => (HistoryKey::Discrete, "assign layer media"),
        SetLayerVisibility { .. } => (HistoryKey::Discrete, "toggle layer visibility"),
        SetLayerOpacity { layer, .. } => (HistoryKey::LayerOpacity(*layer), "change layer opacity"),
        SetLayerBlend { .. } => (HistoryKey::Discrete, "change layer blend"),
        SetLayerTestPattern { .. } => (HistoryKey::Discrete, "change test pattern"),
        SetLayerCrop { layer, .. } => (HistoryKey::LayerCrop(*layer), "change layer crop"),
        SetLayerSurfaces { .. } => (HistoryKey::Discrete, "change layer surfaces"),
        AddLayer { .. } => (HistoryKey::Discrete, "add layer"),
        AddLayerFromMedia { .. } => (HistoryKey::Discrete, "add media layer"),
        DeleteLayer { .. } => (HistoryKey::Discrete, "delete layer"),
        ReorderLayer { .. } => (HistoryKey::Discrete, "reorder layer"),
        SetShowAudio { .. } => (HistoryKey::Discrete, "change show audio"),
        SetShowAudioVolume { .. } => (HistoryKey::ShowAudioVolume, "change show volume"),
        SetShowAudioMuted { .. } => (HistoryKey::Discrete, "mute show audio"),
        AddOutput => (HistoryKey::Discrete, "add output"),
        DeleteOutput { .. } => (HistoryKey::Discrete, "delete output"),
        SetOutputView { output, .. } => (HistoryKey::OutputView(*output), "change output view"),
        SetOutputEnabled { .. } => (HistoryKey::Discrete, "toggle output"),
        SetOutputFullscreen { .. } => (HistoryKey::Discrete, "change output mode"),
        SetOutputEdgeBlend { output, .. } => {
            (HistoryKey::OutputEdgeBlend(*output), "change edge blend")
        }
        SetOutputColor { output, .. } => (HistoryKey::OutputColor(*output), "change output color"),
        LoadOutputLut { .. } => (HistoryKey::Discrete, "load calibration LUT"),
        ClearOutputLut { .. } => (HistoryKey::Discrete, "clear calibration LUT"),
        SetOutputMonitor { .. } => (HistoryKey::Discrete, "assign output display"),
        AddOutputGroup => (HistoryKey::Discrete, "add output group"),
        DeleteOutputGroup { .. } => (HistoryKey::Discrete, "delete output group"),
        SetOutputGroupMember { .. } => (HistoryKey::Discrete, "change output group"),
        RemoveMedia { .. } => (HistoryKey::Discrete, "remove media"),
        // These actions either change only runtime state or open a dialog and
        // record history themselves after the user chooses a file.
        ToggleTransport
        | RestartTransport
        | SetTransportLoop { .. }
        | SeekTransport { .. }
        | ReloadMedia
        | SaveProject { .. }
        | SaveProjectDialog
        | LoadProject { .. }
        | OpenProjectDialog
        | AddMedia
        | PickMediaFile { .. }
        | SetOutputGuide { .. }
        | Undo
        | Redo => return None,
    };
    Some(key)
}

fn selected_surface_ids(stage: &Stage, indices: &[usize]) -> Vec<SurfaceId> {
    let mut ids = Vec::new();
    for index in indices {
        if let Some(surface) = stage.surfaces.get(*index) {
            if !ids.contains(&surface.id) {
                ids.push(surface.id.clone());
            }
        }
    }
    if ids.is_empty() {
        if let Some(surface) = stage.surfaces.first() {
            ids.push(surface.id.clone());
        }
    }
    ids
}

fn used_media_ids(project: &Project) -> Vec<MediaId> {
    project
        .media
        .iter()
        .filter(|asset| project.is_media_used(&asset.id))
        .map(|asset| asset.id.clone())
        .collect()
}

fn media_file_dialog(directory: &Path) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_directory(directory)
        .add_filter(
            "Images, video and audio",
            &[
                "png", "jpg", "jpeg", "mp4", "mov", "mkv", "webm", "avi", "m4v", "mp3", "wav",
                "flac", "ogg", "oga", "m4a", "aac",
            ],
        )
        .add_filter("Images", &["png", "jpg", "jpeg"])
        .add_filter(
            "Videos",
            &["mp4", "mov", "mkv", "webm", "avi", "m4v", "mpg", "mpeg"],
        )
        .add_filter("Audio", &["mp3", "wav", "flac", "ogg", "oga", "m4a", "aac"])
}

fn media_kind_from_path(path: &Path, fallback: MediaKind) -> MediaKind {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());
    match extension.as_deref() {
        Some("mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" | "mpg" | "mpeg") => MediaKind::Video,
        Some("png" | "jpg" | "jpeg") => MediaKind::Image,
        Some("mp3" | "wav" | "flac" | "ogg" | "oga" | "m4a" | "aac") => MediaKind::Audio,
        _ => fallback,
    }
}

fn parse_startup_project() -> Option<PathBuf> {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--project" {
            return arguments.next().map(PathBuf::from);
        }
        if !argument.starts_with('-') {
            return Some(PathBuf::from(argument));
        }
    }
    None
}

fn configure_egui_surface(
    gpu: &Gpu,
    surface: &wgpu::Surface<'_>,
    width: u32,
    height: u32,
) -> Result<wgpu::SurfaceConfiguration> {
    let mut config = surface
        .get_default_config(gpu.adapter(), width.max(1), height.max(1))
        .context("editor surface is not supported by the GPU adapter")?;

    let capabilities = surface.get_capabilities(gpu.adapter());
    // egui prefers a gamma-space target. The output renderer deliberately uses
    // an sRGB target; the editor can avoid egui's sRGB conversion shader.
    if let Some(format) = capabilities.formats.iter().copied().find(|format| {
        matches!(
            format,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
        )
    }) {
        config.format = format;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_surface_ids_deduplicates_indices() {
        let project = Project::demo();
        let stage = project.primary_stage().unwrap();
        let ids = selected_surface_ids(stage, &[1, 1, 0]);
        assert_eq!(
            ids,
            [stage.surfaces[1].id.clone(), stage.surfaces[0].id.clone()]
        );
        assert_eq!(
            selected_surface_ids(stage, &[]),
            [stage.surfaces[0].id.clone()]
        );
    }

    #[test]
    fn media_kind_is_inferred_from_extension() {
        assert_eq!(
            media_kind_from_path(Path::new("clip.MOV"), MediaKind::Image),
            MediaKind::Video
        );
        assert_eq!(
            media_kind_from_path(Path::new("still.jpg"), MediaKind::Video),
            MediaKind::Image
        );
        assert_eq!(
            media_kind_from_path(Path::new("asset.unknown"), MediaKind::Video),
            MediaKind::Video
        );
        assert_eq!(
            media_kind_from_path(Path::new("track.FLAC"), MediaKind::Image),
            MediaKind::Audio
        );
    }
}

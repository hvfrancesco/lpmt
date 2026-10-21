//! LPMT application bootstrap.
//!
//! Milestone 0/1 intentionally uses one event loop, one GPU device, an egui
//! editor window and a separate output window. This is the smallest useful
//! demonstration of the Stage + Outputs architecture.

mod clock;
mod video_runtime;

use clock::ShowClock;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use lpmt_core::model::{
    BlendMode, Layer, MediaAsset, MediaKind, OutputGroup, OutputId, OutputSpec, OutputTarget,
    Point2, Project, SourceSpec, Surface,
};
use lpmt_render::gpu::Gpu;
use lpmt_render::media::MediaCache;
use lpmt_render::output::OutputRenderer;
use lpmt_ui::editor::{self, EditorAction, EditorState, EditorStatus};
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
    ui_state: EditorState,
    editor: Option<EditorWindow>,
    outputs: Vec<OutputWindow>,
    monitors: Vec<String>,
    videos: VideoRuntime,
    clock: ShowClock,
    last_clock_seek: u64,
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
}

impl App {
    fn new(gpu: Gpu) -> Result<Self> {
        let media = MediaCache::new(&gpu)?;
        let mut app = Self {
            gpu,
            media,
            project: Project::demo(),
            ui_state: EditorState::default(),
            editor: None,
            outputs: Vec::new(),
            monitors: Vec::new(),
            videos: VideoRuntime::new(),
            clock: ShowClock::new(),
            last_clock_seek: 0,
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

        if let Some(_output) = self.outputs.get(index) {
            match event {
                WindowEvent::CloseRequested => close = true,
                WindowEvent::Resized(size) => resize = Some(size),
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
            ..
        } = self;
        let mut prepared = Vec::new();

        for index in indices {
            let Some(output) = outputs.get_mut(*index) else {
                continue;
            };
            let Some(output_spec) = project.output(&output.id) else {
                continue;
            };
            let Some(stage) = project
                .stage(&output_spec.stage_id)
                .or_else(|| project.primary_stage())
            else {
                continue;
            };
            match output.renderer.prepare(gpu, media, stage, output_spec) {
                Ok(Some(frame)) => prepared.push(frame),
                Ok(None) => {}
                Err(error) => {
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
        for prepared in prepared {
            let (frame, command_buffer) = prepared.finish();
            frames.push(frame);
            command_buffers.push(command_buffer);
        }

        // One submit for the whole group is the frame barrier: no output is
        // presented until every prepared frame in the batch is enqueued.
        gpu.queue().submit(command_buffers);
        for frame in frames {
            gpu.queue().present(frame);
        }
        if indices.len() > 1 {
            log::trace!("presented {count} grouped outputs in one batch");
        }
    }

    fn apply_actions(&mut self, actions: Vec<EditorAction>) {
        for action in actions {
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
                        surface.corners[corner] = value;
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
                    self.ui_state.selected_surface = stage.surfaces.len() - 1;
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
                        if stage
                            .layers
                            .iter()
                            .any(|layer| layer.surface_id == surface_id)
                        {
                            self.status_message =
                                "Delete or reassign the layers on this surface first".to_owned();
                            continue;
                        }
                        stage.surfaces.remove(surface);
                        stage.surfaces.len()
                    };
                    let selected = self.ui_state.selected_surface;
                    self.ui_state.selected_surface = if selected > surface {
                        selected - 1
                    } else {
                        selected.min(remaining.saturating_sub(1))
                    };
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
                EditorAction::SetLayerSurface { layer, surface } => {
                    let Some(surface_id) = self
                        .project
                        .primary_stage()
                        .and_then(|stage| stage.surfaces.get(surface))
                        .map(|surface| surface.id.clone())
                    else {
                        continue;
                    };
                    if let Some(layer) = self
                        .project
                        .primary_stage_mut()
                        .and_then(|stage| stage.layers.get_mut(layer))
                    {
                        layer.surface_id = surface_id;
                    }
                }
                EditorAction::AddLayer { surface } => {
                    let Some(stage) = self.project.primary_stage_mut() else {
                        continue;
                    };
                    let Some(surface_id) = stage
                        .surfaces
                        .get(surface)
                        .or_else(|| stage.surfaces.first())
                        .map(|surface| surface.id.clone())
                    else {
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
                        Layer::test_pattern(id, format!("Test pattern {index}"), &surface_id);
                    layer.opacity = 0.45;
                    layer.blend = BlendMode::Screen;
                    stage.layers.push(layer);
                    self.ui_state.selected_layer = stage.layers.len() - 1;
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
                    let dialog = media_file_dialog(&self.media_base);
                    if let Some(path) = dialog.pick_file() {
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
                        };
                        self.project.media.push(asset);
                        self.ui_state.selected_media = self.project.media.len() - 1;
                        self.ui_state.image_path = stored;
                        self.reload_media();
                    }
                }
                EditorAction::RemoveMedia { media } => {
                    let Some(asset) = self.project.media.get(media) else {
                        continue;
                    };
                    if self.project.is_media_used(&asset.id) {
                        self.status_message =
                            "That media asset is used by a layer; reassign the layer first"
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
                    let fallback = asset.kind;
                    let dialog = media_file_dialog(&self.media_base);
                    if let Some(path) = dialog.pick_file() {
                        let stored = self.stored_media_path(&path);
                        let kind = media_kind_from_path(&path, fallback);
                        if let Some(asset) = self.project.media.get_mut(media) {
                            asset.path = stored.clone();
                        }
                        self.apply_media_kind(media, kind);
                        self.ui_state.image_path = stored;
                        self.reload_media();
                    }
                }
            }
        }
    }

    /// Keep layers pointing at a media asset valid when the file type changes
    /// from image to video or back.
    fn apply_media_kind(&mut self, media: usize, kind: MediaKind) {
        let Some(media_id) = self.project.media.get(media).map(|asset| asset.id.clone()) else {
            return;
        };
        if let Some(asset) = self.project.media.get_mut(media) {
            asset.kind = kind;
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
            media_base,
            ..
        } = self;

        media.clear();
        let (video_loaded, _) = videos.load(project, media_base);
        let mut loaded = video_loaded;
        let mut failed_names: Vec<String> = project
            .media
            .iter()
            .filter(|asset| asset.kind == MediaKind::Video && project.is_media_used(&asset.id))
            .filter(|asset| !videos.is_loaded(&asset.id))
            .map(|asset| asset.name.clone())
            .collect();

        for asset in &project.media {
            if asset.kind != MediaKind::Image || !project.is_media_used(&asset.id) {
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

        self.status_message =
            if failed_names.is_empty() {
                format!("Loaded {loaded} media asset(s)")
            } else {
                let hint = if !cfg!(feature = "video")
                    && project.media.iter().any(|asset| {
                        asset.kind == MediaKind::Video && project.is_media_used(&asset.id)
                    }) {
                    " · rebuild with --features video"
                } else {
                    ""
                };
                format!(
                    "Loaded {loaded}, failed {}: {}{hint}",
                    failed_names.len(),
                    failed_names.join(", ")
                )
            };

        self.clock.set_duration(videos.max_duration());
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
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
        }
        std::fs::write(&path, json)
            .with_context(|| format!("failed to write {}", path.display()))?;
        self.ui_state.project_path = path.display().to_string();
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
        self.ui_state.project_path = path.display().to_string();
        self.media_base = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        self.ui_state.selected_surface = 0;
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
            monitors: self.monitors.clone(),
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
        self.videos.sync_to_clock(
            self.clock.position_seconds(),
            self.clock.is_playing(),
            self.clock.is_looping(),
            force_seek,
        );
        self.videos.poll(&self.gpu, &mut self.media);

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
    cascade: i32,
) -> Result<OutputWindow> {
    let mut attributes = Window::default_attributes()
        .with_title(format!("LPMT — {}", spec.name))
        .with_active(false);

    match &spec.target {
        OutputTarget::Windowed { width, height } => {
            attributes = attributes
                .with_inner_size(PhysicalSize::new((*width).max(1), (*height).max(1)))
                .with_position(PhysicalPosition::new(80 + cascade * 40, 80 + cascade * 40));
        }
        OutputTarget::Fullscreen { monitor } => {
            let monitor = choose_monitor(event_loop, monitor.as_deref(), used_monitors);
            if let Some(monitor) = monitor {
                log::info!(
                    "placing output '{}' fullscreen on monitor {:?}",
                    spec.name,
                    monitor.name()
                );
                attributes =
                    attributes.with_fullscreen(Some(Fullscreen::Borderless(Some(monitor))));
            } else {
                log::warn!(
                    "no monitor available for output '{}'; opening it windowed",
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
    let renderer =
        OutputRenderer::new(gpu, media, surface, window.clone(), size.width, size.height)?;

    Ok(OutputWindow {
        id: spec.id.clone(),
        window,
        renderer,
    })
}

/// Pick a display for a fullscreen output: an explicit monitor name wins,
/// otherwise prefer an unused secondary monitor so several projectors do not
/// land on the same screen.
fn choose_monitor(
    event_loop: &ActiveEventLoop,
    preferred: Option<&str>,
    used: &[String],
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

    let primary = event_loop.primary_monitor();
    monitors
        .iter()
        .find(|monitor| {
            primary.as_ref() != Some(*monitor)
                && !used
                    .iter()
                    .any(|name| monitor.name().as_deref() == Some(name.as_str()))
        })
        .or_else(|| {
            monitors.iter().find(|monitor| {
                !used
                    .iter()
                    .any(|name| monitor.name().as_deref() == Some(name.as_str()))
            })
        })
        // With every display already covered, a further fullscreen output
        // would hide the editor. Open it windowed instead.
        .or_else(|| {
            if used.is_empty() {
                monitors.first()
            } else {
                None
            }
        })
        .cloned()
}

fn media_file_dialog(directory: &Path) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_directory(directory)
        .add_filter(
            "Images and videos",
            &[
                "png", "jpg", "jpeg", "mp4", "mov", "mkv", "webm", "avi", "m4v",
            ],
        )
        .add_filter("Images", &["png", "jpg", "jpeg"])
        .add_filter(
            "Videos",
            &["mp4", "mov", "mkv", "webm", "avi", "m4v", "mpg", "mpeg"],
        )
}

fn media_kind_from_path(path: &Path, fallback: MediaKind) -> MediaKind {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());
    match extension.as_deref() {
        Some("mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" | "mpg" | "mpeg") => MediaKind::Video,
        Some("png" | "jpg" | "jpeg") => MediaKind::Image,
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
    }
}

//! LPMT application bootstrap.
//!
//! Milestone 0/1 intentionally uses one event loop, one GPU device, an egui
//! editor window and a separate output window. This is the smallest useful
//! demonstration of the Stage + Outputs architecture.

mod video_runtime;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use lpmt_core::model::{BlendMode, Layer, MediaKind, Project, SourceSpec};
use lpmt_render::gpu::Gpu;
use lpmt_render::output::OutputRenderer;
use lpmt_ui::editor::{self, EditorAction, EditorState, EditorStatus};
use video_runtime::VideoRuntime;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowId};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
        Box::new(event_loop.owned_display_handle()),
    ));
    let gpu = Gpu::new(instance)?;

    let mut app = App::new(gpu);
    app.startup_project = parse_startup_project();
    event_loop.run_app(&mut app).context("event loop failed")?;
    Ok(())
}

struct App {
    gpu: Gpu,
    project: Project,
    ui_state: EditorState,
    editor: Option<EditorWindow>,
    output: Option<OutputWindow>,
    videos: VideoRuntime,
    media_base: PathBuf,
    startup_project: Option<PathBuf>,
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
    window: Arc<Window>,
    renderer: OutputRenderer,
}

impl App {
    fn new(gpu: Gpu) -> Self {
        let mut app = Self {
            gpu,
            project: Project::demo(),
            ui_state: EditorState::default(),
            editor: None,
            output: None,
            videos: VideoRuntime::new(),
            media_base: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            startup_project: None,
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
        app
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

        let mut output_attributes = Window::default_attributes()
            .with_title("LPMT — Output 1")
            .with_inner_size(PhysicalSize::new(1280, 720))
            .with_active(false);

        let primary_monitor = event_loop.primary_monitor();
        let secondary = event_loop
            .available_monitors()
            .find(|monitor| Some(monitor) != primary_monitor.as_ref());
        if let Some(secondary) = secondary {
            log::info!(
                "placing output on secondary monitor: {:?}",
                secondary.name()
            );
            output_attributes =
                output_attributes.with_fullscreen(Some(Fullscreen::Borderless(Some(secondary))));
        } else {
            log::info!("no secondary monitor found; output opens in a 1280x720 window");
        }

        let output_window = Arc::new(
            event_loop
                .create_window(output_attributes)
                .context("failed to create output window")?,
        );
        let output_surface = self
            .gpu
            .instance()
            .create_surface(output_window.clone())
            .context("failed to create output surface")?;
        let output_size = output_window.inner_size();
        let output_renderer = OutputRenderer::new(
            &self.gpu,
            output_surface,
            output_window.clone(),
            output_size.width,
            output_size.height,
        )?;

        self.output = Some(OutputWindow {
            window: output_window,
            renderer: output_renderer,
        });
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

        Ok(())
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

    fn handle_output_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let mut close = false;
        let mut redraw = false;
        let mut resize = None;

        if let Some(_output) = self.output.as_mut() {
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
            if let Some(output) = self.output.as_mut() {
                if let Err(error) = output.renderer.resize(&self.gpu, size.width, size.height) {
                    log::error!("failed to resize output: {error:#}");
                }
            }
        }

        if redraw {
            self.render_output();
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

    fn render_output(&mut self) {
        let App {
            gpu,
            project,
            output,
            videos,
            ..
        } = self;
        let Some(output) = output.as_mut() else {
            return;
        };
        videos.poll(gpu, &mut output.renderer);
        let Some(output_spec) = project.outputs.first() else {
            return;
        };
        let Some(stage) = project
            .stage(&output_spec.stage_id)
            .or_else(|| project.primary_stage())
        else {
            return;
        };
        let view = output_spec.view;
        if let Err(error) = output.renderer.render(gpu, stage, view) {
            log::error!("output render failed: {error:#}");
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
                EditorAction::SetMediaPath { media, path } => {
                    if let Some(asset) = self.project.media.get_mut(media) {
                        asset.path.clone_from(&path);
                    }
                    self.ui_state.image_path = path;
                    self.reload_media();
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
                EditorAction::ToggleVideoPlay { media } => {
                    if let Some(media_id) =
                        self.project.media.get(media).map(|asset| asset.id.clone())
                    {
                        self.videos.toggle_play(&media_id);
                    }
                }
                EditorAction::RestartVideo { media } => {
                    if let Some(media_id) =
                        self.project.media.get(media).map(|asset| asset.id.clone())
                    {
                        self.videos.restart(&media_id);
                    }
                }
                EditorAction::SetVideoLoop { media, looping } => {
                    if let Some(media_id) =
                        self.project.media.get(media).map(|asset| asset.id.clone())
                    {
                        self.videos.set_looping(&media_id, looping);
                    }
                }
                EditorAction::SeekVideo { media, position } => {
                    if let Some(media_id) =
                        self.project.media.get(media).map(|asset| asset.id.clone())
                    {
                        self.videos.seek(&media_id, position);
                    }
                }
                EditorAction::ReloadMedia => self.reload_media(),
                EditorAction::SaveProject { path } => {
                    if let Err(error) = self.save_project(&path) {
                        log::error!("failed to save project: {error:#}");
                        self.status_message = format!("Save failed: {error}");
                    }
                }
                EditorAction::LoadProject { path } => {
                    if let Err(error) = self.load_project(&path) {
                        log::error!("failed to load project: {error:#}");
                        self.status_message = format!("Load failed: {error}");
                    }
                }
            }
        }
    }

    fn reload_media(&mut self) {
        let App {
            gpu,
            project,
            output,
            videos,
            media_base,
            ..
        } = self;
        let Some(output) = output.as_mut() else {
            return;
        };

        output.renderer.clear_media();
        let (video_loaded, video_failed) = videos.load(project, media_base);
        let mut loaded = video_loaded;
        let mut failed = video_failed;

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
                Ok(image) => match output.renderer.upload_image(gpu, asset.id.clone(), &image) {
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
                        failed += 1;
                        log::error!(
                            "failed to upload media '{}' ({}): {error:#}",
                            asset.name,
                            path.display()
                        );
                    }
                },
                Err(error) => {
                    failed += 1;
                    log::warn!(
                        "media '{}' could not be loaded from {}: {error:#}",
                        asset.name,
                        path.display()
                    );
                }
            }
        }

        self.status_message = if failed == 0 {
            format!("Loaded {loaded} media asset(s)")
        } else {
            format!("Loaded {loaded}, failed {failed} media asset(s)")
        };
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
        let output = self.output.as_ref().map_or_else(
            || "No output window".to_owned(),
            |output| {
                let (width, height) = output.renderer.size();
                let layers = self
                    .project
                    .primary_stage()
                    .map_or(0, |stage| stage.layers.len());
                format!("Output 1 · {width}×{height} · full stage · {layers} layers")
            },
        );
        let video = self
            .project
            .media
            .get(self.ui_state.selected_media)
            .filter(|asset| asset.kind == MediaKind::Video)
            .and_then(|asset| self.videos.status(&asset.id))
            .map(|state| editor::VideoStatus {
                playing: state.playing,
                looping: state.looping,
                position_seconds: state.position_seconds,
                duration_seconds: state.duration_seconds,
            });

        EditorStatus {
            adapter: self.gpu.adapter_info().name.clone(),
            output,
            fps: self.fps,
            frame_ms: self.frame_ms,
            message: self.status_message.clone(),
            video,
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
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

        if self
            .output
            .as_ref()
            .is_some_and(|output| output.window.id() == window_id)
        {
            self.handle_output_event(event_loop, event);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.tick();
        if let Some(editor) = self.editor.as_ref() {
            editor.window.request_redraw();
        }
        if let Some(output) = self.output.as_ref() {
            output.window.request_redraw();
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

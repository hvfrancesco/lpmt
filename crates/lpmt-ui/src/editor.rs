//! The editor window. This module only knows about the domain model and egui;
//! it emits actions instead of mutating renderer or window state directly.

use egui::{Align2, Color32, FontId, Pos2, Rect as EguiRect, Sense, Shape, Stroke, Vec2};
use lpmt_core::model::{
    BlendMode, ColorCorrection, EdgeBlend, LayerId, MediaKind, OutputTarget, Point2, Project, Rect,
    SourceSpec, SurfaceId,
};

#[derive(Debug, Clone, Default)]
pub struct EditorStatus {
    pub adapter: String,
    pub output: String,
    pub fps: f32,
    pub frame_ms: f32,
    pub message: String,
    pub video: Option<VideoStatus>,
    pub video_supported: bool,
    pub monitors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VideoStatus {
    pub playing: bool,
    pub looping: bool,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug)]
pub struct EditorState {
    pub selected_surface: usize,
    pub selected_layer: usize,
    pub selected_media: usize,
    pub selected_output: usize,
    pub selected_group: usize,
    pub project_path: String,
    pub image_path: String,
    surface_name: String,
    layer_name: String,
    surface_name_id: Option<SurfaceId>,
    layer_name_id: Option<LayerId>,
    drag_corner: Option<usize>,
    video_scrub: Option<f32>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected_surface: 0,
            selected_layer: 0,
            selected_media: 0,
            selected_output: 0,
            selected_group: 0,
            project_path: "project.lpmt.json".to_owned(),
            image_path: String::new(),
            surface_name: String::new(),
            layer_name: String::new(),
            surface_name_id: None,
            layer_name_id: None,
            drag_corner: None,
            video_scrub: None,
        }
    }
}

impl EditorState {
    /// Drop cached rename buffers after loading a different project.
    pub fn reset_text_buffers(&mut self) {
        self.surface_name_id = None;
        self.layer_name_id = None;
        self.surface_name.clear();
        self.layer_name.clear();
    }
}

#[derive(Debug, Clone)]
pub enum EditorAction {
    SetCorner {
        surface: usize,
        corner: usize,
        value: Point2,
    },
    ResetSurface {
        surface: usize,
    },
    AddSurface,
    DeleteSurface {
        surface: usize,
    },
    RenameSurface {
        surface: usize,
        name: String,
    },
    SetSurfaceVisibility {
        surface: usize,
        visible: bool,
    },
    SetSurfaceOpacity {
        surface: usize,
        opacity: f32,
    },
    SetMediaPath {
        media: usize,
        path: String,
    },
    RenameLayer {
        layer: usize,
        name: String,
    },
    SetLayerMedia {
        layer: usize,
        media: usize,
    },
    SetLayerVisibility {
        layer: usize,
        visible: bool,
    },
    SetLayerOpacity {
        layer: usize,
        opacity: f32,
    },
    SetLayerBlend {
        layer: usize,
        blend: BlendMode,
    },
    SetLayerSurface {
        layer: usize,
        surface: usize,
    },
    AddLayer {
        surface: usize,
    },
    DeleteLayer {
        layer: usize,
    },
    ReorderLayer {
        layer: usize,
        offset: i32,
    },
    ToggleVideoPlay {
        media: usize,
    },
    RestartVideo {
        media: usize,
    },
    SetVideoLoop {
        media: usize,
        looping: bool,
    },
    SeekVideo {
        media: usize,
        position: f64,
    },
    AddOutput,
    DeleteOutput {
        output: usize,
    },
    SetOutputView {
        output: usize,
        view: Rect,
    },
    SetOutputEnabled {
        output: usize,
        enabled: bool,
    },
    SetOutputFullscreen {
        output: usize,
        fullscreen: bool,
    },
    SetOutputEdgeBlend {
        output: usize,
        edge_blend: EdgeBlend,
    },
    SetOutputColor {
        output: usize,
        color: ColorCorrection,
    },
    SetOutputMonitor {
        output: usize,
        monitor: Option<String>,
    },
    AddOutputGroup,
    DeleteOutputGroup {
        group: usize,
    },
    SetOutputGroupMember {
        group: usize,
        output: usize,
        member: bool,
    },
    ReloadMedia,
    SaveProject {
        path: String,
    },
    SaveProjectDialog,
    LoadProject {
        path: String,
    },
    OpenProjectDialog,
    AddMedia,
    RemoveMedia {
        media: usize,
    },
    PickMediaFile {
        media: usize,
    },
}

pub fn draw(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    project: &Project,
    status: &EditorStatus,
) -> Vec<EditorAction> {
    let mut actions = Vec::new();
    let Some(stage) = project.primary_stage() else {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.centered_and_justified(|ui| ui.label("The project has no stage."));
        });
        return actions;
    };

    if state.selected_layer >= stage.layers.len() {
        state.selected_layer = stage.layers.len().saturating_sub(1);
    }
    if state.selected_surface >= stage.surfaces.len() {
        state.selected_surface = stage.surfaces.len().saturating_sub(1);
    }
    if state.selected_media >= project.media.len() {
        state.selected_media = 0;
        if let Some(asset) = project.media.first() {
            state.image_path.clone_from(&asset.path);
        }
    }
    if state.selected_output >= project.outputs.len() {
        state.selected_output = project.outputs.len().saturating_sub(1);
    }
    if state.selected_group >= project.output_groups.len() {
        state.selected_group = project.output_groups.len().saturating_sub(1);
    }
    if let Some(surface) = stage.surfaces.get(state.selected_surface) {
        if state.surface_name_id.as_ref() != Some(&surface.id) {
            state.surface_name.clone_from(&surface.name);
            state.surface_name_id = Some(surface.id.clone());
        }
    }
    if let Some(layer) = stage.layers.get(state.selected_layer) {
        if state.layer_name_id.as_ref() != Some(&layer.id) {
            state.layer_name.clone_from(&layer.name);
            state.layer_name_id = Some(layer.id.clone());
        }
    }

    egui::Panel::top("lpmt-top").show(ui, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading("LPMT");
            ui.label("Stage + Outputs prototype");
            ui.separator();
            ui.label(&project.name);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{:.0} fps · {:.2} ms", status.fps, status.frame_ms));
            });
        });
        ui.add_space(4.0);
    });

    egui::Panel::left("lpmt-inspector")
        .resizable(false)
        .default_size(340.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(8.0);
                ui.heading("Inspector");
                ui.separator();

                ui.label("Surfaces");
                for (index, surface) in stage.surfaces.iter().enumerate() {
                    if ui
                        .selectable_label(state.selected_surface == index, &surface.name)
                        .clicked()
                    {
                        state.selected_surface = index;
                    }
                }
                let selected_surface_in_use = stage
                    .surfaces
                    .get(state.selected_surface)
                    .is_some_and(|surface| {
                        stage
                            .layers
                            .iter()
                            .any(|layer| layer.surface_id == surface.id)
                    });
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Add surface").clicked() {
                        actions.push(EditorAction::AddSurface);
                        state.selected_surface = stage.surfaces.len();
                    }
                    let delete = ui.add_enabled(
                        stage.surfaces.len() > 1 && !selected_surface_in_use,
                        egui::Button::new("Delete surface"),
                    );
                    if delete.clicked() {
                        let surface = state.selected_surface;
                        actions.push(EditorAction::DeleteSurface { surface });
                        state.selected_surface =
                            surface.min(stage.surfaces.len().saturating_sub(2));
                    }
                    if selected_surface_in_use {
                        delete.on_disabled_hover_text(
                            "Reassign or delete the layers using this surface first",
                        );
                    }
                });

                ui.add_space(12.0);
                if let Some(surface) = stage.surfaces.get(state.selected_surface) {
                    ui.label("Surface name");
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut state.surface_name)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        actions.push(EditorAction::RenameSurface {
                            surface: state.selected_surface,
                            name: state.surface_name.clone(),
                        });
                    }

                    let mut visible = surface.visible;
                    if ui.checkbox(&mut visible, "Visible").changed() {
                        actions.push(EditorAction::SetSurfaceVisibility {
                            surface: state.selected_surface,
                            visible,
                        });
                    }
                    let mut opacity = surface.opacity;
                    if ui
                        .add(
                            egui::Slider::new(&mut opacity, 0.0..=1.0)
                                .text("Opacity")
                                .fixed_decimals(2),
                        )
                        .changed()
                    {
                        actions.push(EditorAction::SetSurfaceOpacity {
                            surface: state.selected_surface,
                            opacity,
                        });
                    }

                    ui.label("Normalized stage coordinates");
                    egui::Grid::new("surface-corners")
                        .num_columns(3)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Corner");
                            ui.label("x");
                            ui.label("y");
                            ui.end_row();

                            for (index, corner) in surface.corners.iter().enumerate() {
                                ui.label(corner_name(index));
                                let mut x = corner.x;
                                let mut y = corner.y;
                                let x_changed = ui
                                    .add(egui::DragValue::new(&mut x).speed(0.001).range(0.0..=1.0))
                                    .changed();
                                let y_changed = ui
                                    .add(egui::DragValue::new(&mut y).speed(0.001).range(0.0..=1.0))
                                    .changed();
                                if x_changed || y_changed {
                                    actions.push(EditorAction::SetCorner {
                                        surface: state.selected_surface,
                                        corner: index,
                                        value: Point2::new(x, y),
                                    });
                                }
                                ui.end_row();
                            }
                        });

                    ui.add_space(8.0);
                    if ui.button("Reset to full stage").clicked() {
                        actions.push(EditorAction::ResetSurface {
                            surface: state.selected_surface,
                        });
                    }
                }

                ui.add_space(16.0);
                ui.separator();
                ui.heading("Layers");
                ui.label("Bottom to top");
                for (index, layer) in stage.layers.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let mut visible = layer.visible;
                        if ui.checkbox(&mut visible, "").changed() {
                            actions.push(EditorAction::SetLayerVisibility {
                                layer: index,
                                visible,
                            });
                        }
                        let response = ui
                            .selectable_label(
                                state.selected_layer == index,
                                format!("{} · {}", layer.name, layer.blend.label()),
                            )
                            .on_hover_text(layer_source(layer, project));
                        if response.clicked() {
                            state.selected_layer = index;
                        }
                    });
                }

                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(!stage.surfaces.is_empty(), egui::Button::new("Add layer"))
                        .clicked()
                    {
                        actions.push(EditorAction::AddLayer {
                            surface: state.selected_surface,
                        });
                        state.selected_layer = stage.layers.len();
                    }
                    if ui
                        .add_enabled(stage.layers.len() > 1, egui::Button::new("Delete layer"))
                        .clicked()
                    {
                        let layer = state.selected_layer;
                        actions.push(EditorAction::DeleteLayer { layer });
                        state.selected_layer = layer.min(stage.layers.len().saturating_sub(2));
                    }
                    if ui
                        .add_enabled(state.selected_layer > 0, egui::Button::new("Lower"))
                        .clicked()
                    {
                        let layer = state.selected_layer;
                        actions.push(EditorAction::ReorderLayer { layer, offset: -1 });
                        state.selected_layer -= 1;
                    }
                    if ui
                        .add_enabled(
                            state.selected_layer + 1 < stage.layers.len(),
                            egui::Button::new("Raise"),
                        )
                        .clicked()
                    {
                        let layer = state.selected_layer;
                        actions.push(EditorAction::ReorderLayer { layer, offset: 1 });
                        state.selected_layer += 1;
                    }
                });

                if let Some(layer) = stage.layers.get(state.selected_layer) {
                    ui.add_space(6.0);
                    ui.label("Layer name");
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut state.layer_name)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        actions.push(EditorAction::RenameLayer {
                            layer: state.selected_layer,
                            name: state.layer_name.clone(),
                        });
                    }
                    let mut visible = layer.visible;
                    if ui.checkbox(&mut visible, "Visible").changed() {
                        actions.push(EditorAction::SetLayerVisibility {
                            layer: state.selected_layer,
                            visible,
                        });
                    }

                    let mut opacity = layer.opacity;
                    if ui
                        .add(
                            egui::Slider::new(&mut opacity, 0.0..=1.0)
                                .text("Opacity")
                                .fixed_decimals(2),
                        )
                        .changed()
                    {
                        actions.push(EditorAction::SetLayerOpacity {
                            layer: state.selected_layer,
                            opacity,
                        });
                    }

                    let mut blend = layer.blend;
                    egui::ComboBox::from_label("Blend")
                        .selected_text(blend.label())
                        .show_ui(ui, |ui| {
                            for mode in BlendMode::ALL {
                                ui.selectable_value(&mut blend, mode, mode.label());
                            }
                        });
                    if blend != layer.blend {
                        actions.push(EditorAction::SetLayerBlend {
                            layer: state.selected_layer,
                            blend,
                        });
                    }

                    let surface_index = stage
                        .surfaces
                        .iter()
                        .position(|surface| surface.id == layer.surface_id)
                        .unwrap_or(0);
                    let mut chosen_surface = surface_index;
                    let surface_name = stage
                        .surfaces
                        .get(surface_index)
                        .map_or("missing surface", |surface| surface.name.as_str());
                    egui::ComboBox::from_label("Surface")
                        .selected_text(surface_name)
                        .show_ui(ui, |ui| {
                            for (index, surface) in stage.surfaces.iter().enumerate() {
                                ui.selectable_value(&mut chosen_surface, index, &surface.name);
                            }
                        });
                    if chosen_surface != surface_index {
                        actions.push(EditorAction::SetLayerSurface {
                            layer: state.selected_layer,
                            surface: chosen_surface,
                        });
                    }
                } else {
                    ui.label("No layers");
                }

                ui.add_space(16.0);
                ui.separator();
                ui.heading("Media");
                if !status.video_supported
                    && project
                        .media
                        .iter()
                        .any(|asset| asset.kind == MediaKind::Video)
                {
                    ui.colored_label(
                        Color32::from_rgb(255, 184, 92),
                        "This build has no video support. Rebuild with --features video.",
                    );
                }
                if let Some(layer) = stage.layers.get(state.selected_layer) {
                    ui.label(format!(
                        "Layer: {} · {}",
                        layer.name,
                        layer_source(layer, project)
                    ));
                }

                if project.media.is_empty() {
                    ui.label("No media assets");
                }
                for (index, asset) in project.media.iter().enumerate() {
                    let kind = match asset.kind {
                        MediaKind::Image => "Image",
                        MediaKind::Video => "Video",
                    };
                    if ui
                        .selectable_label(
                            state.selected_media == index,
                            format!("{} · {kind}", asset.name),
                        )
                        .clicked()
                    {
                        state.selected_media = index;
                        state.image_path.clone_from(&asset.path);
                        state.video_scrub = None;
                    }
                }

                let selected_media_used = project
                    .media
                    .get(state.selected_media)
                    .is_some_and(|asset| project.is_media_used(&asset.id));
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Add media…").clicked() {
                        actions.push(EditorAction::AddMedia);
                        state.selected_media = project.media.len();
                    }
                    let remove = ui.add_enabled(
                        !project.media.is_empty() && !selected_media_used,
                        egui::Button::new("Remove media"),
                    );
                    if remove.clicked() {
                        let media = state.selected_media;
                        actions.push(EditorAction::RemoveMedia { media });
                        state.selected_media = media.min(project.media.len().saturating_sub(2));
                    }
                    if selected_media_used {
                        remove.on_disabled_hover_text(
                            "This asset is used by a layer; reassign that layer first",
                        );
                    }
                });

                if project.media.get(state.selected_media).is_some() {
                    ui.add_space(6.0);
                    ui.label("Media path (relative to project)");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.image_path)
                            .desired_width(f32::INFINITY),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Browse…").clicked() {
                            actions.push(EditorAction::PickMediaFile {
                                media: state.selected_media,
                            });
                        }
                        if ui.button("Apply path").clicked() {
                            actions.push(EditorAction::SetMediaPath {
                                media: state.selected_media,
                                path: state.image_path.clone(),
                            });
                        }
                        if ui.button("Reload asset").clicked() {
                            actions.push(EditorAction::ReloadMedia);
                        }
                        if ui
                            .add_enabled(
                                stage.layers.get(state.selected_layer).is_some(),
                                egui::Button::new("Use for layer"),
                            )
                            .clicked()
                        {
                            actions.push(EditorAction::SetLayerMedia {
                                layer: state.selected_layer,
                                media: state.selected_media,
                            });
                        }
                    });
                }

                let selected_is_video = project
                    .media
                    .get(state.selected_media)
                    .is_some_and(|asset| asset.kind == MediaKind::Video);
                if selected_is_video {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.heading("Playback");
                    if let Some(video) = status.video {
                        ui.horizontal(|ui| {
                            let label = if video.playing { "Pause" } else { "Play" };
                            if ui.button(label).clicked() {
                                actions.push(EditorAction::ToggleVideoPlay {
                                    media: state.selected_media,
                                });
                            }
                            if ui.button("Restart").clicked() {
                                actions.push(EditorAction::RestartVideo {
                                    media: state.selected_media,
                                });
                            }
                            let mut looping = video.looping;
                            if ui.checkbox(&mut looping, "Loop").changed() {
                                actions.push(EditorAction::SetVideoLoop {
                                    media: state.selected_media,
                                    looping,
                                });
                            }
                        });

                        let time = match video.duration_seconds {
                            Some(duration) => {
                                format!("{:.2} / {:.2} s", video.position_seconds, duration)
                            }
                            None => format!("{:.2} s", video.position_seconds),
                        };
                        ui.label(time);

                        if let Some(duration) =
                            video.duration_seconds.filter(|duration| *duration > 0.0)
                        {
                            let mut position = state
                                .video_scrub
                                .unwrap_or(video.position_seconds as f32)
                                .clamp(0.0, duration as f32);
                            let response = ui.add(
                                egui::Slider::new(&mut position, 0.0..=duration as f32)
                                    .show_value(false),
                            );
                            if response.dragged() {
                                state.video_scrub = Some(position);
                            }
                            if response.drag_stopped()
                                || (response.changed() && !response.dragged())
                            {
                                state.video_scrub = None;
                                actions.push(EditorAction::SeekVideo {
                                    media: state.selected_media,
                                    position: f64::from(position),
                                });
                            }
                        }
                    } else if status.video_supported {
                        ui.label("No player is running for this asset.");
                        ui.label("Click “Use for layer” and check that the path resolves.");
                    } else {
                        ui.label("Video support is not compiled into this build.");
                        ui.label("Rebuild with `cargo run -p lpmt-app --features video`.");
                    }
                }

                ui.add_space(16.0);
                ui.separator();
                ui.heading("Project file");
                ui.add(
                    egui::TextEdit::singleline(&mut state.project_path)
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Save").clicked() {
                        actions.push(EditorAction::SaveProject {
                            path: state.project_path.clone(),
                        });
                    }
                    if ui.button("Save as…").clicked() {
                        actions.push(EditorAction::SaveProjectDialog);
                    }
                    if ui.button("Open…").clicked() {
                        actions.push(EditorAction::OpenProjectDialog);
                    }
                    if ui.button("Load path").clicked() {
                        actions.push(EditorAction::LoadProject {
                            path: state.project_path.clone(),
                        });
                    }
                });

                ui.add_space(16.0);
                ui.separator();
                ui.heading("Outputs");
                ui.label(&status.output);

                for (index, output) in project.outputs.iter().enumerate() {
                    let state_label = if output.enabled { "" } else { " · closed" };
                    let label =
                        format!("{} · {}{state_label}", output.name, view_label(output.view));
                    if ui
                        .selectable_label(state.selected_output == index, label)
                        .clicked()
                    {
                        state.selected_output = index;
                    }
                }

                ui.horizontal_wrapped(|ui| {
                    if ui.button("Add output").clicked() {
                        actions.push(EditorAction::AddOutput);
                        state.selected_output = project.outputs.len();
                    }
                    if ui
                        .add_enabled(
                            project.outputs.len() > 1,
                            egui::Button::new("Delete output"),
                        )
                        .clicked()
                    {
                        let output = state.selected_output;
                        actions.push(EditorAction::DeleteOutput { output });
                        state.selected_output = output.min(project.outputs.len().saturating_sub(2));
                    }
                });

                if let Some(output) = project.outputs.get(state.selected_output) {
                    ui.add_space(6.0);
                    ui.heading(&output.name);

                    let mut enabled = output.enabled;
                    if ui.checkbox(&mut enabled, "Open window").changed() {
                        actions.push(EditorAction::SetOutputEnabled {
                            output: state.selected_output,
                            enabled,
                        });
                    }

                    let mut fullscreen = matches!(output.target, OutputTarget::Fullscreen { .. });
                    if ui.checkbox(&mut fullscreen, "Fullscreen").changed() {
                        actions.push(EditorAction::SetOutputFullscreen {
                            output: state.selected_output,
                            fullscreen,
                        });
                    }
                    match &output.target {
                        OutputTarget::Windowed { width, height } => {
                            ui.label(format!("Window {width}×{height}"));
                        }
                        OutputTarget::Fullscreen { monitor } => {
                            let mut chosen = monitor.clone();
                            egui::ComboBox::from_label("Display")
                                .selected_text(
                                    chosen.as_deref().unwrap_or("Automatic (unused display)"),
                                )
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut chosen,
                                        None,
                                        "Automatic (unused display)",
                                    );
                                    for name in &status.monitors {
                                        ui.selectable_value(&mut chosen, Some(name.clone()), name);
                                    }
                                });
                            if chosen != *monitor {
                                actions.push(EditorAction::SetOutputMonitor {
                                    output: state.selected_output,
                                    monitor: chosen,
                                });
                            }
                        }
                    }

                    ui.label("Stage view (normalized)");
                    let mut view = output.view;
                    egui::Grid::new("output-view")
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            for (label, value) in [
                                ("X", &mut view.x),
                                ("Y", &mut view.y),
                                ("Width", &mut view.width),
                                ("Height", &mut view.height),
                            ] {
                                ui.label(label);
                                ui.add(
                                    egui::DragValue::new(value)
                                        .speed(0.001)
                                        .range(0.0..=1.0)
                                        .fixed_decimals(3),
                                );
                                ui.end_row();
                            }
                        });
                    if view != output.view {
                        view.width = view.width.clamp(0.01, (1.0 - view.x).max(0.01));
                        view.height = view.height.clamp(0.01, (1.0 - view.y).max(0.01));
                        actions.push(EditorAction::SetOutputView {
                            output: state.selected_output,
                            view,
                        });
                    }

                    ui.horizontal_wrapped(|ui| {
                        for (label, preset) in [
                            ("Full stage", Rect::FULL),
                            ("Left half", Rect::new(0.0, 0.0, 0.5, 1.0)),
                            ("Right half", Rect::new(0.5, 0.0, 0.5, 1.0)),
                        ] {
                            if ui.button(label).clicked() {
                                actions.push(EditorAction::SetOutputView {
                                    output: state.selected_output,
                                    view: preset,
                                });
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.label("Edge blend");
                    let mut edge_blend = output.edge_blend;
                    egui::Grid::new("output-edge-blend")
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            for (label, value) in [
                                ("Left", &mut edge_blend.left),
                                ("Right", &mut edge_blend.right),
                                ("Top", &mut edge_blend.top),
                                ("Bottom", &mut edge_blend.bottom),
                            ] {
                                ui.label(label);
                                ui.add(
                                    egui::Slider::new(value, 0.0..=0.5)
                                        .fixed_decimals(3)
                                        .show_value(true),
                                );
                                ui.end_row();
                            }
                        });
                    if edge_blend != output.edge_blend {
                        actions.push(EditorAction::SetOutputEdgeBlend {
                            output: state.selected_output,
                            edge_blend: edge_blend.clamped(),
                        });
                    }

                    ui.add_space(8.0);
                    ui.label("Output colour");
                    let mut color = output.color;
                    egui::Grid::new("output-color")
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            for (label, value, range) in [
                                ("Brightness", &mut color.brightness, 0.0..=4.0),
                                ("Contrast", &mut color.contrast, 0.0..=4.0),
                                ("Gamma", &mut color.gamma, 0.1..=4.0),
                            ] {
                                ui.label(label);
                                ui.add(
                                    egui::Slider::new(value, range)
                                        .fixed_decimals(2)
                                        .show_value(true),
                                );
                                ui.end_row();
                            }
                        });
                    if color != output.color {
                        actions.push(EditorAction::SetOutputColor {
                            output: state.selected_output,
                            color: color.clamped(),
                        });
                    }
                    if (!color.is_identity() || !edge_blend.is_none())
                        && ui.button("Reset blend and colour").clicked()
                    {
                        actions.push(EditorAction::SetOutputEdgeBlend {
                            output: state.selected_output,
                            edge_blend: EdgeBlend::NONE,
                        });
                        actions.push(EditorAction::SetOutputColor {
                            output: state.selected_output,
                            color: ColorCorrection::IDENTITY,
                        });
                    }
                }

                ui.add_space(16.0);
                ui.separator();
                ui.heading("Output groups");
                ui.label("Grouped outputs render one frame and present together.");
                for (index, group) in project.output_groups.iter().enumerate() {
                    if ui
                        .selectable_label(
                            state.selected_group == index,
                            format!("{} · {} output(s)", group.name, group.outputs.len()),
                        )
                        .clicked()
                    {
                        state.selected_group = index;
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Add group").clicked() {
                        actions.push(EditorAction::AddOutputGroup);
                        state.selected_group = project.output_groups.len();
                    }
                    if ui
                        .add_enabled(
                            !project.output_groups.is_empty(),
                            egui::Button::new("Delete group"),
                        )
                        .clicked()
                    {
                        let group = state.selected_group;
                        actions.push(EditorAction::DeleteOutputGroup { group });
                        state.selected_group =
                            group.min(project.output_groups.len().saturating_sub(2));
                    }
                });
                if let Some(group) = project.output_groups.get(state.selected_group) {
                    ui.add_space(6.0);
                    ui.label(format!("Members of {}", group.name));
                    for (output_index, output) in project.outputs.iter().enumerate() {
                        let mut member = group.outputs.contains(&output.id);
                        if ui.checkbox(&mut member, &output.name).changed() {
                            actions.push(EditorAction::SetOutputGroupMember {
                                group: state.selected_group,
                                output: output_index,
                                member,
                            });
                        }
                    }
                    ui.label("An output belongs to at most one group.");
                }

                ui.add_space(16.0);
                ui.label(format!("Stage aspect: {:.2}:1", stage.aspect));
                ui.label("Drag a corner in the preview to warp the surface.");
                ui.add_space(8.0);
            });
        });

    egui::Panel::bottom("lpmt-status").show(ui, |ui| {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(format!("{} · wgpu · winit", std::env::consts::OS));
            ui.separator();
            ui.label(&status.adapter);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if status.message.is_empty() {
                    ui.label("Editor + output windows share one GPU device");
                } else {
                    ui.label(&status.message);
                }
            });
        });
        ui.add_space(2.0);
    });

    egui::CentralPanel::default().show(ui, |ui| {
        ui.add_space(4.0);
        let available = ui.available_size();
        let (outer, _) = ui.allocate_exact_size(available, Sense::hover());

        let aspect = stage.aspect.max(0.1);
        let mut preview_size = Vec2::new(outer.width(), outer.width() / aspect);
        if preview_size.y > outer.height() {
            preview_size = Vec2::new(outer.height() * aspect, outer.height());
        }
        preview_size.x = preview_size.x.max(1.0);
        preview_size.y = preview_size.y.max(1.0);
        let preview = EguiRect::from_center_size(outer.center(), preview_size);

        let response = ui.interact(
            preview,
            ui.scope_id().with("surface-preview"),
            Sense::click_and_drag(),
        );
        let painter = ui.painter();

        painter.rect_filled(
            preview,
            egui::CornerRadius::same(4),
            Color32::from_rgb(16, 18, 24),
        );
        let grid_stroke = Stroke::new(1.0, Color32::from_rgb(42, 46, 58));
        for step in 1..4 {
            let t = step as f32 / 4.0;
            let x = preview.left() + preview.width() * t;
            let y = preview.top() + preview.height() * t;
            painter.line_segment(
                [Pos2::new(x, preview.top()), Pos2::new(x, preview.bottom())],
                grid_stroke,
            );
            painter.line_segment(
                [Pos2::new(preview.left(), y), Pos2::new(preview.right(), y)],
                grid_stroke,
            );
        }

        let output_colors = [
            Color32::from_rgb(92, 214, 196),
            Color32::from_rgb(255, 188, 96),
            Color32::from_rgb(214, 132, 255),
            Color32::from_rgb(120, 170, 255),
        ];
        for (index, output) in project.outputs.iter().enumerate() {
            if !output.enabled {
                continue;
            }
            let view = output.view;
            let rect = EguiRect::from_min_size(
                Pos2::new(
                    preview.left() + preview.width() * view.x,
                    preview.top() + preview.height() * view.y,
                ),
                Vec2::new(preview.width() * view.width, preview.height() * view.height),
            );
            let color = output_colors[index % output_colors.len()];
            let stroke = Stroke::new(
                if index == state.selected_output {
                    2.0
                } else {
                    1.0
                },
                color,
            );
            let corners = [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ];
            for pair in 0..4 {
                painter.line_segment([corners[pair], corners[(pair + 1) % 4]], stroke);
            }
            painter.text(
                rect.left_top() + Vec2::new(4.0, 4.0),
                Align2::LEFT_TOP,
                &output.name,
                FontId::proportional(10.0),
                color,
            );
        }

        if let Some(surface) = stage.surfaces.get(state.selected_surface) {
            let points: Vec<Pos2> = surface
                .corners
                .iter()
                .map(|corner| stage_to_preview(preview, *corner))
                .collect();

            painter.add(Shape::convex_polygon(
                points.clone(),
                Color32::from_rgba_unmultiplied(74, 144, 226, 38),
                Stroke::new(2.0, Color32::from_rgb(108, 182, 255)),
            ));

            for (index, point) in points.iter().enumerate() {
                let active = state.drag_corner == Some(index);
                painter.circle_filled(
                    *point,
                    if active { 9.0 } else { 7.0 },
                    if active {
                        Color32::from_rgb(255, 210, 90)
                    } else {
                        Color32::from_rgb(238, 242, 250)
                    },
                );
                painter.circle_stroke(*point, 7.0, Stroke::new(1.0, Color32::from_rgb(20, 24, 32)));
                painter.text(
                    *point + Vec2::new(10.0, -12.0),
                    Align2::LEFT_TOP,
                    corner_name(index),
                    FontId::proportional(11.0),
                    Color32::from_rgb(180, 188, 202),
                );
            }

            if response.drag_started() {
                state.drag_corner = response
                    .interact_pointer_pos()
                    .and_then(|pointer| nearest_corner(&points, pointer, 18.0));
            }

            if response.dragged() {
                if let (Some(index), Some(pointer)) =
                    (state.drag_corner, response.interact_pointer_pos())
                {
                    let value = preview_to_stage(preview, pointer).clamp_unit();
                    actions.push(EditorAction::SetCorner {
                        surface: state.selected_surface,
                        corner: index,
                        value,
                    });
                }
            }

            if response.drag_stopped() {
                state.drag_corner = None;
            }
        }

        painter.text(
            preview.left_top() + Vec2::new(8.0, 8.0),
            Align2::LEFT_TOP,
            "Stage preview",
            FontId::proportional(12.0),
            Color32::from_rgb(120, 128, 142),
        );
    });

    actions
}

fn layer_source(layer: &lpmt_core::model::Layer, project: &Project) -> String {
    match &layer.source {
        SourceSpec::Image { media_id } | SourceSpec::Video { media_id } => project
            .media_asset(media_id)
            .map_or_else(|| "missing media".to_owned(), |asset| asset.name.clone()),
        SourceSpec::TestPattern { .. } => "test pattern".to_owned(),
    }
}

fn view_label(view: Rect) -> String {
    format!(
        "{:.0}..{:.0}% x {:.0}..{:.0}%",
        view.x * 100.0,
        (view.x + view.width) * 100.0,
        view.y * 100.0,
        (view.y + view.height) * 100.0
    )
}

fn corner_name(index: usize) -> &'static str {
    match index {
        0 => "TL",
        1 => "TR",
        2 => "BR",
        _ => "BL",
    }
}

fn stage_to_preview(rect: EguiRect, point: Point2) -> Pos2 {
    Pos2::new(
        rect.left() + rect.width() * point.x,
        rect.top() + rect.height() * point.y,
    )
}

fn preview_to_stage(rect: EguiRect, point: Pos2) -> Point2 {
    Point2::new(
        (point.x - rect.left()) / rect.width(),
        (point.y - rect.top()) / rect.height(),
    )
}

fn nearest_corner(points: &[Pos2], pointer: Pos2, threshold: f32) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| (index, point.distance(pointer)))
        .filter(|(_, distance)| *distance <= threshold)
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(index, _)| index)
}

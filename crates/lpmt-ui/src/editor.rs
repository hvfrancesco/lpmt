//! The editor window. This module only knows about the domain model and egui;
//! it emits actions instead of mutating renderer or window state directly.

use egui::{Align2, Color32, FontId, Pos2, Rect as EguiRect, Sense, Shape, Stroke, Vec2};
use lpmt_core::model::{BlendMode, MediaKind, Point2, Project, SourceSpec};

#[derive(Debug, Clone, Default)]
pub struct EditorStatus {
    pub adapter: String,
    pub output: String,
    pub fps: f32,
    pub frame_ms: f32,
    pub message: String,
    pub video: Option<VideoStatus>,
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
    pub project_path: String,
    pub image_path: String,
    drag_corner: Option<usize>,
    video_scrub: Option<f32>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected_surface: 0,
            selected_layer: 0,
            selected_media: 0,
            project_path: "project.lpmt.json".to_owned(),
            image_path: String::new(),
            drag_corner: None,
            video_scrub: None,
        }
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
    SetMediaPath {
        media: usize,
        path: String,
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
    ReloadMedia,
    SaveProject {
        path: String,
    },
    LoadProject {
        path: String,
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

                ui.add_space(12.0);
                if let Some(surface) = stage.surfaces.get(state.selected_surface) {
                    ui.heading(&surface.name);
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
                    ui.heading(&layer.name);
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

                if project.media.get(state.selected_media).is_some() {
                    ui.add_space(6.0);
                    ui.label("Media path (relative to project)");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.image_path)
                            .desired_width(f32::INFINITY),
                    );
                    ui.horizontal_wrapped(|ui| {
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
                    } else {
                        ui.label("No player is running for this asset.");
                        ui.label("Use it for a layer and build with `--features video`.");
                    }
                }

                ui.add_space(16.0);
                ui.separator();
                ui.heading("Project file");
                ui.add(
                    egui::TextEdit::singleline(&mut state.project_path)
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        actions.push(EditorAction::SaveProject {
                            path: state.project_path.clone(),
                        });
                    }
                    if ui.button("Load").clicked() {
                        actions.push(EditorAction::LoadProject {
                            path: state.project_path.clone(),
                        });
                    }
                });

                ui.add_space(16.0);
                ui.separator();
                ui.heading("Output");
                ui.label(&status.output);
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

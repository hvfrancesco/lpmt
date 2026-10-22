//! The editor window. This module only knows about the domain model and egui;
//! it emits actions instead of mutating renderer or window state directly.

use std::collections::{BTreeSet, HashMap};

use egui::{Align2, Color32, FontId, Pos2, Rect as EguiRect, RichText, Sense, Shape, Stroke, Vec2};
use lpmt_core::binding::{MidiMode, ParameterBinding};
use lpmt_core::guide::GuideMode;
use lpmt_core::lut::LUT_SAMPLES;
use lpmt_core::model::{
    BlendMode, CircleCrop, ColorCorrection, Crop, EdgeBlend, LayerId, MediaAsset, MediaId,
    MediaKind, OutputTarget, Point2, Project, Rect, SourceSpec, Stage, Surface, SurfaceId,
    TestPattern, WarpGrid,
};
use lpmt_core::param::{project_parameters, ParameterDescriptor, ParameterKind};
use lpmt_core::timeline::{
    TimelineCommand, TimelineInterpolation, TimelineKey, MAX_TIMELINE_DURATION,
    MIN_TIMELINE_DURATION,
};

#[derive(Debug, Clone, Default)]
pub struct EditorStatus {
    pub adapter: String,
    pub output: String,
    pub fps: f32,
    pub frame_ms: f32,
    pub message: String,
    pub transport: TransportStatus,
    pub video_supported: bool,
    pub audio_supported: bool,
    pub audio_error: Option<String>,
    pub monitors: Vec<String>,
    pub dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_label: Option<String>,
    pub redo_label: Option<String>,
    pub output_pacing: Vec<OutputPacing>,
    /// Session-only guide mode, parallel to `project.outputs`.
    pub output_guides: Vec<GuideMode>,
    pub osc: OscStatus,
    pub midi: MidiStatus,
    /// egui native textures for image and video thumbnails.
    pub media_thumbnails: HashMap<MediaId, egui::TextureId>,
}

#[derive(Debug, Clone, Default)]
pub struct OscStatus {
    pub address: String,
    pub listening: bool,
    pub messages: u64,
    pub last: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MidiStatus {
    pub supported: bool,
    pub devices: Vec<String>,
    pub messages: u64,
    pub last: String,
    pub error: Option<String>,
    pub learning: Option<(String, MidiMode)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TransportStatus {
    pub playing: bool,
    pub looping: bool,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
}

/// Per-output present statistics. Counts accumulate until the output window is
/// recreated; `missed` is every attempted frame that did not reach the screen.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OutputPacing {
    pub attempts: u64,
    pub presented: u64,
    pub skipped: u64,
    pub errors: u64,
    pub last_interval_ms: f32,
    pub average_interval_ms: f32,
    pub since_last_present_ms: f32,
}

impl OutputPacing {
    pub fn note_attempt(&mut self) {
        self.attempts = self.attempts.saturating_add(1);
    }

    pub fn note_skipped(&mut self) {
        self.skipped = self.skipped.saturating_add(1);
    }

    pub fn note_error(&mut self) {
        self.errors = self.errors.saturating_add(1);
    }

    pub fn note_present(&mut self, interval_ms: Option<f32>) {
        self.presented = self.presented.saturating_add(1);
        if let Some(interval) = interval_ms {
            let interval = interval.max(0.0);
            self.last_interval_ms = interval;
            self.average_interval_ms = if self.average_interval_ms > 0.0 {
                self.average_interval_ms * 0.9 + interval * 0.1
            } else {
                interval
            };
        }
    }

    pub fn missed(&self) -> u64 {
        self.attempts.saturating_sub(self.presented)
    }

    pub fn fps(&self) -> Option<f32> {
        (self.average_interval_ms > 0.0).then_some(1000.0 / self.average_interval_ms)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorkspaceMode {
    #[default]
    Mapping,
    Calibration,
    Show,
}

impl WorkspaceMode {
    pub const ALL: [Self; 3] = [Self::Mapping, Self::Calibration, Self::Show];

    pub fn label(self) -> &'static str {
        match self {
            Self::Mapping => "Mapping",
            Self::Calibration => "Calibration",
            Self::Show => "Show",
        }
    }
}

#[derive(Debug)]
struct SurfaceMoveDrag {
    start_stage: Point2,
    originals: Vec<(usize, [Point2; 4])>,
}

#[derive(Debug, Clone)]
struct TimelineClipboard {
    /// Key times are relative to the earliest copied key.
    keys: Vec<TimelineKey>,
}

#[derive(Debug)]
pub struct EditorState {
    pub mode: WorkspaceMode,
    /// Active surface shown in the inspector and with edit handles.
    pub selected_surface: usize,
    selected_surfaces: BTreeSet<usize>,
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
    /// Layer whose source is being chosen in the in-app media picker.
    media_picker: Option<usize>,
    binding_parameter: Option<String>,
    binding_learn: Option<String>,
    midi_parameter: Option<String>,
    midi_mode: MidiMode,
    timeline_selected_track: Option<usize>,
    timeline_selected_key: Option<usize>,
    timeline_drag_key: Option<(usize, usize)>,
    timeline_add_parameter: Option<String>,
    timeline_selected_trigger: Option<usize>,
    timeline_drag_trigger: Option<usize>,
    timeline_trigger_command: TimelineCommand,
    timeline_clipboard: Option<TimelineClipboard>,
    timeline_selected_keys: BTreeSet<usize>,
    timeline_selected_marker: Option<usize>,
    timeline_drag_marker: Option<usize>,
    timeline_marker_name: String,
    timeline_marker_name_id: Option<usize>,
    drag_corner: Option<usize>,
    drag_grid: Option<(u16, u16)>,
    drag_mask_point: Option<usize>,
    /// When set, a primary click in the preview adds a point to the active
    /// surface mask. This avoids relying on a desktop-reserved modifier.
    mask_add_mode: bool,
    surface_move: Option<SurfaceMoveDrag>,
    transport_scrub: Option<f32>,
    pub snap_corners: bool,
    snap_target: Option<Pos2>,
    pub preview_texture: Option<egui::TextureId>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            mode: WorkspaceMode::Mapping,
            selected_surface: 0,
            selected_surfaces: BTreeSet::from([0]),
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
            media_picker: None,
            binding_parameter: None,
            binding_learn: None,
            midi_parameter: None,
            midi_mode: MidiMode::Absolute,
            timeline_selected_track: None,
            timeline_selected_key: None,
            timeline_drag_key: None,
            timeline_add_parameter: None,
            timeline_selected_trigger: None,
            timeline_drag_trigger: None,
            timeline_trigger_command: TimelineCommand::Pause,
            timeline_clipboard: None,
            timeline_selected_keys: BTreeSet::new(),
            timeline_selected_marker: None,
            timeline_drag_marker: None,
            timeline_marker_name: String::new(),
            timeline_marker_name_id: None,
            drag_corner: None,
            drag_grid: None,
            drag_mask_point: None,
            mask_add_mode: false,
            surface_move: None,
            transport_scrub: None,
            snap_corners: true,
            snap_target: None,
            preview_texture: None,
        }
    }
}

impl EditorState {
    /// Drop cached rename buffers after loading a different project.
    pub fn reset_text_buffers(&mut self) {
        self.surface_name_id = None;
        self.layer_name_id = None;
        self.media_picker = None;
        self.binding_parameter = None;
        self.binding_learn = None;
        self.midi_parameter = None;
        self.midi_mode = MidiMode::Absolute;
        self.timeline_selected_track = None;
        self.timeline_selected_key = None;
        self.timeline_drag_key = None;
        self.timeline_add_parameter = None;
        self.timeline_selected_trigger = None;
        self.timeline_drag_trigger = None;
        self.timeline_clipboard = None;
        self.timeline_selected_keys.clear();
        self.timeline_selected_marker = None;
        self.timeline_drag_marker = None;
        self.timeline_marker_name.clear();
        self.timeline_marker_name_id = None;
        self.mask_add_mode = false;
        self.surface_name.clear();
        self.layer_name.clear();
    }

    /// Cancel any in-progress preview drag after the project changes underneath
    /// the editor (for example after undo).
    pub fn cancel_drag(&mut self) {
        self.drag_corner = None;
        self.drag_grid = None;
        self.surface_move = None;
        self.snap_target = None;
    }

    pub fn is_surface_selected(&self, index: usize) -> bool {
        self.selected_surfaces.contains(&index)
    }

    pub fn select_only_surface(&mut self, index: usize) {
        self.selected_surface = index;
        self.selected_surfaces.clear();
        self.selected_surfaces.insert(index);
        self.drag_corner = None;
        self.drag_grid = None;
        self.surface_move = None;
    }

    /// Shift-click behaviour: add/remove a surface while keeping at least one
    /// selected. The clicked surface becomes the active inspector target.
    pub fn toggle_surface_selection(&mut self, index: usize) {
        if self.selected_surfaces.contains(&index) {
            if self.selected_surfaces.len() > 1 {
                self.selected_surfaces.remove(&index);
                if self.selected_surface == index {
                    self.selected_surface = *self
                        .selected_surfaces
                        .iter()
                        .next()
                        .expect("at least one surface remains selected");
                }
            }
        } else {
            self.selected_surfaces.insert(index);
            self.selected_surface = index;
        }
        self.drag_corner = None;
        self.drag_grid = None;
        self.surface_move = None;
    }

    pub fn normalize_surface_selection(&mut self, surface_count: usize) {
        if surface_count == 0 {
            self.selected_surface = 0;
            self.selected_surfaces.clear();
            return;
        }
        self.selected_surface = self.selected_surface.min(surface_count - 1);
        self.selected_surfaces
            .retain(|index| *index < surface_count);
        if self.selected_surfaces.is_empty() {
            self.selected_surfaces.insert(self.selected_surface);
        } else if !self.selected_surfaces.contains(&self.selected_surface) {
            self.selected_surface = *self
                .selected_surfaces
                .iter()
                .next()
                .expect("the selection is not empty");
        }
    }

    pub fn remove_surface_selection(&mut self, removed: usize, remaining: usize) {
        let mut selection = BTreeSet::new();
        for index in std::mem::take(&mut self.selected_surfaces) {
            if index < removed {
                selection.insert(index);
            } else if index > removed {
                selection.insert(index - 1);
            }
        }
        self.selected_surfaces = selection;
        if self.selected_surface > removed {
            self.selected_surface -= 1;
        }
        self.normalize_surface_selection(remaining);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceGeometryEdit {
    /// Continuous whole-surface drag; coalesced in undo history.
    Move,
    /// One-shot alignment of two or more selected surfaces.
    Align,
    /// One-shot distribution of three or more selected surfaces.
    Distribute,
}

#[derive(Debug, Clone)]
pub enum EditorAction {
    SetCorner {
        surface: usize,
        corner: usize,
        value: Point2,
    },
    SetSurfaceCorners {
        surfaces: Vec<(usize, [Point2; 4])>,
        edit: SurfaceGeometryEdit,
    },
    SetSurfaceGridEnabled {
        surface: usize,
        enabled: bool,
    },
    SetSurfaceGridResolution {
        surface: usize,
        rows: u16,
        columns: u16,
    },
    ResetSurfaceGrid {
        surface: usize,
    },
    SetGridPoint {
        surface: usize,
        row: u16,
        column: u16,
        value: Point2,
    },
    AddBinding {
        parameter: String,
        key: String,
        ctrl: bool,
        shift: bool,
        alt: bool,
    },
    RemoveBinding {
        binding: usize,
    },
    TriggerBinding {
        binding: usize,
    },
    BeginMidiLearn {
        parameter: String,
        mode: MidiMode,
    },
    CancelMidiLearn,
    RemoveMidiBinding {
        binding: usize,
    },
    AddTimelineTrack {
        parameter: String,
    },
    RemoveTimelineTrack {
        track: usize,
    },
    AddTimelineKey {
        track: usize,
        time: f32,
    },
    PasteTimelineKeys {
        track: usize,
        time: f32,
        keys: Vec<TimelineKey>,
    },
    SetTimelineKey {
        track: usize,
        key: usize,
        time: f32,
        value: f32,
        interpolation: TimelineInterpolation,
    },
    RemoveTimelineKey {
        track: usize,
        key: usize,
    },
    SetTimelineDuration {
        duration: f32,
    },
    AddTimelineTrigger {
        time: f32,
        command: TimelineCommand,
    },
    SetTimelineTrigger {
        trigger: usize,
        time: f32,
        command: TimelineCommand,
    },
    RemoveTimelineTrigger {
        trigger: usize,
    },
    AddTimelineMarker {
        time: f32,
    },
    SetTimelineMarker {
        marker: usize,
        time: f32,
        name: String,
    },
    RemoveTimelineMarker {
        marker: usize,
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
    SetSurfaceMaskEnabled {
        surface: usize,
        enabled: bool,
    },
    SetSurfaceMaskInvert {
        surface: usize,
        invert: bool,
    },
    SetSurfaceMaskFeather {
        surface: usize,
        feather: f32,
    },
    SetSurfaceMaskPoint {
        surface: usize,
        point: usize,
        value: Point2,
    },
    AddSurfaceMaskPoint {
        surface: usize,
        point: Point2,
        /// Polygon insertion index, chosen from the nearest existing segment.
        insert_at: usize,
    },
    RemoveSurfaceMaskPoint {
        surface: usize,
        point: usize,
    },
    ResetSurfaceMask {
        surface: usize,
    },
    ClearSurfaceMask {
        surface: usize,
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
    SetLayerTestPattern {
        layer: usize,
        pattern: TestPattern,
    },
    SetLayerCrop {
        layer: usize,
        crop: Crop,
    },
    SetLayerSurfaces {
        layer: usize,
        surfaces: Vec<usize>,
    },
    AddLayer {
        surfaces: Vec<usize>,
    },
    AddLayerFromMedia {
        surfaces: Vec<usize>,
        media: usize,
    },
    DeleteLayer {
        layer: usize,
    },
    ReorderLayer {
        layer: usize,
        offset: i32,
    },
    ToggleTransport,
    RestartTransport,
    SetTransportLoop {
        looping: bool,
    },
    SeekTransport {
        position: f64,
    },
    SetShowAudio {
        media: Option<usize>,
    },
    SetShowAudioVolume {
        volume: f32,
    },
    SetShowAudioMuted {
        muted: bool,
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
    OpenOutput {
        output: usize,
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
    LoadOutputLut {
        output: usize,
    },
    ClearOutputLut {
        output: usize,
    },
    SetOutputGuide {
        output: usize,
        guide: GuideMode,
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
    Undo,
    Redo,
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
    if let Some(parameter) = state.binding_learn.clone() {
        match capture_hotkey(ui) {
            Some(HotkeyCapture::Cancel) => state.binding_learn = None,
            Some(HotkeyCapture::Key {
                key,
                ctrl,
                shift,
                alt,
            }) => {
                actions.push(EditorAction::AddBinding {
                    parameter: parameter.clone(),
                    key,
                    ctrl,
                    shift,
                    alt,
                });
                state.binding_parameter = Some(parameter);
                state.binding_learn = None;
            }
            None => {}
        }
    } else if !ui.ctx().egui_wants_keyboard_input() && state.media_picker.is_none() {
        let mut custom_matched = false;
        for (index, binding) in project.bindings.iter().enumerate() {
            if hotkey_binding_pressed(ui, binding) {
                actions.push(EditorAction::TriggerBinding { binding: index });
                custom_matched = true;
                break;
            }
        }
        if !custom_matched {
            if ui.input(|input| input.key_pressed(egui::Key::Space)) {
                actions.push(EditorAction::ToggleTransport);
            }
            if ui.input(|input| input.key_pressed(egui::Key::Home)) {
                actions.push(EditorAction::RestartTransport);
            }
            if ui.input(|input| input.key_pressed(egui::Key::M)) {
                state.media_picker = Some(state.selected_layer);
            }
            if ui.input(|input| input.key_pressed(egui::Key::F1)) {
                state.mode = WorkspaceMode::Mapping;
            }
            if ui.input(|input| input.key_pressed(egui::Key::F2)) {
                state.mode = WorkspaceMode::Calibration;
            }
            if ui.input(|input| input.key_pressed(egui::Key::F3)) {
                state.mode = WorkspaceMode::Show;
            }
            if ui.input(|input| input.key_pressed(egui::Key::G)) {
                if let Some(current) = status.output_guides.get(state.selected_output).copied() {
                    actions.push(EditorAction::SetOutputGuide {
                        output: state.selected_output,
                        guide: current.next(),
                    });
                }
            }
            if ui.input(|input| input.modifiers.command) {
                if ui.input(|input| input.key_pressed(egui::Key::Z)) {
                    if ui.input(|input| input.modifiers.shift) {
                        actions.push(EditorAction::Redo);
                    } else {
                        actions.push(EditorAction::Undo);
                    }
                } else if ui.input(|input| input.key_pressed(egui::Key::Y)) {
                    actions.push(EditorAction::Redo);
                }
            }
        }
    }
    let Some(stage) = project.primary_stage() else {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.centered_and_justified(|ui| ui.label("The project has no stage."));
        });
        return actions;
    };

    if state.selected_layer >= stage.layers.len() {
        state.selected_layer = stage.layers.len().saturating_sub(1);
    }
    state.normalize_surface_selection(stage.surfaces.len());
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
            ui.separator();
            for mode in WorkspaceMode::ALL {
                if ui
                    .selectable_label(state.mode == mode, mode.label())
                    .clicked()
                {
                    state.mode = mode;
                    if mode != WorkspaceMode::Show {
                        state.binding_learn = None;
                    }
                }
            }
            ui.separator();
            ui.menu_button("Project", |ui| {
                ui.set_min_width(320.0);
                ui.label("Project file");
                ui.add(
                    egui::TextEdit::singleline(&mut state.project_path)
                        .desired_width(f32::INFINITY),
                );
                if ui.button("Open…").clicked() {
                    actions.push(EditorAction::OpenProjectDialog);
                    ui.close();
                }
                if ui.button("Save").clicked() {
                    actions.push(EditorAction::SaveProject {
                        path: state.project_path.clone(),
                    });
                    ui.close();
                }
                if ui.button("Save as…").clicked() {
                    actions.push(EditorAction::SaveProjectDialog);
                    ui.close();
                }
                if ui.button("Load path").clicked() {
                    actions.push(EditorAction::LoadProject {
                        path: state.project_path.clone(),
                    });
                    ui.close();
                }
            });
            ui.separator();
            ui.label(&project.name);
            if status.dirty {
                ui.colored_label(Color32::from_rgb(255, 184, 92), "* unsaved");
            }
            ui.separator();
            let undo = ui.add_enabled(status.can_undo, egui::Button::new("Undo"));
            if undo.clicked() {
                actions.push(EditorAction::Undo);
            }
            if let Some(label) = &status.undo_label {
                undo.on_hover_text(format!("Undo {label} (Ctrl+Z)"));
            }
            let redo = ui.add_enabled(status.can_redo, egui::Button::new("Redo"));
            if redo.clicked() {
                actions.push(EditorAction::Redo);
            }
            if let Some(label) = &status.redo_label {
                redo.on_hover_text(format!("Redo {label} (Ctrl+Shift+Z)"));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{:.0} fps · {:.2} ms", status.fps, status.frame_ms));
            });
        });
        ui.add_space(4.0);
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

    if state.mode == WorkspaceMode::Show {
        egui::Panel::bottom("lpmt-timeline")
            .resizable(true)
            .default_size(240.0)
            .show(ui, |ui| {
                draw_timeline(ui, state, project, status, &mut actions);
            });
    }

    if state.mode == WorkspaceMode::Mapping {
        egui::Panel::left("lpmt-hierarchy")
            .resizable(true)
            .default_size(400.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    draw_hierarchy(ui, state, project, status, stage, &mut actions);
                });
            });
    }

    if state.mode != WorkspaceMode::Mapping {
        egui::Panel::right("lpmt-properties")
            .resizable(true)
            .default_size(360.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading(match state.mode {
                        WorkspaceMode::Mapping => "Properties",
                        WorkspaceMode::Calibration => "Calibration",
                        WorkspaceMode::Show => "Show",
                    });
                    ui.separator();

                    if state.mode == WorkspaceMode::Show {
                        ui.heading("Transport");
                        ui.horizontal_wrapped(|ui| {
                            let label = if status.transport.playing {
                                "Pause"
                            } else {
                                "Play"
                            };
                            if ui.button(label).clicked() {
                                actions.push(EditorAction::ToggleTransport);
                            }
                            if ui.button("Restart").clicked() {
                                actions.push(EditorAction::RestartTransport);
                            }
                            let mut looping = status.transport.looping;
                            if ui.checkbox(&mut looping, "Loop").changed() {
                                actions.push(EditorAction::SetTransportLoop { looping });
                            }
                        });
                        let time = match status.transport.duration_seconds {
                            Some(duration) => format!(
                                "{:.2} / {:.2} s",
                                status.transport.position_seconds, duration
                            ),
                            None => format!("{:.2} s", status.transport.position_seconds),
                        };
                        ui.label(time);
                        if let Some(duration) = status
                            .transport
                            .duration_seconds
                            .filter(|duration| *duration > 0.0)
                        {
                            let mut position = state
                                .transport_scrub
                                .unwrap_or(status.transport.position_seconds as f32)
                                .clamp(0.0, duration as f32);
                            let response = ui.add(
                                egui::Slider::new(&mut position, 0.0..=duration as f32)
                                    .show_value(false),
                            );
                            if response.dragged() {
                                state.transport_scrub = Some(position);
                            }
                            if response.drag_stopped()
                                || (response.changed() && !response.dragged())
                            {
                                state.transport_scrub = None;
                                actions.push(EditorAction::SeekTransport {
                                    position: f64::from(position),
                                });
                            }
                        }
                        ui.label("One clock drives every video and output.");

                        ui.add_space(12.0);
                        ui.separator();
                        ui.heading("OSC");
                        if status.osc.listening {
                            ui.label(format!("Listening on {}", status.osc.address));
                        } else {
                            ui.colored_label(
                                Color32::from_rgb(255, 184, 92),
                                format!("OSC unavailable on {}", status.osc.address),
                            );
                        }
                        ui.label(format!("{} message(s) received", status.osc.messages));
                        if !status.osc.last.is_empty() {
                            ui.label(RichText::new(&status.osc.last).small().monospace());
                        }
                        if let Some(error) = &status.osc.error {
                            ui.colored_label(Color32::from_rgb(255, 132, 112), error);
                        }
                        ui.label(
                            RichText::new(
                                "Try /lpmt/layer/layer-1/opacity 0.5 or /lpmt/transport/playing 1",
                            )
                            .small()
                            .color(Color32::from_rgb(132, 144, 166)),
                        );

                        ui.add_space(12.0);
                        ui.separator();
                        ui.heading("MIDI");
                        if !status.midi.supported {
                            ui.colored_label(
                                Color32::from_rgb(255, 184, 92),
                                "MIDI input is not available in this build. Rebuild with --features midi.",
                            );
                            if let Some(error) = &status.midi.error {
                                ui.colored_label(Color32::from_rgb(255, 132, 112), error);
                            }
                        } else {
                            if status.midi.devices.is_empty() {
                                ui.label("No MIDI inputs found.");
                            } else {
                                ui.label(format!(
                                    "Inputs: {}",
                                    status.midi.devices.join(", ")
                                ));
                            }
                            ui.label(format!(
                                "{} message(s) received",
                                status.midi.messages
                            ));
                            if !status.midi.last.is_empty() {
                                ui.label(RichText::new(&status.midi.last).small().monospace());
                            }
                            if let Some(error) = &status.midi.error {
                                ui.colored_label(Color32::from_rgb(255, 132, 112), error);
                            }

                            let parameters = project_parameters(project);
                            let previous_kind = state
                                .midi_parameter
                                .as_ref()
                                .and_then(|path| {
                                    parameters.iter().find(|descriptor| &descriptor.path() == path)
                                })
                                .map(|descriptor| descriptor.kind);
                            let selected_path = state
                                .midi_parameter
                                .clone()
                                .filter(|path| {
                                    parameters.iter().any(|descriptor| &descriptor.path() == path)
                                })
                                .or_else(|| parameters.first().map(|descriptor| descriptor.path()));
                            let selected_kind = selected_path
                                .as_ref()
                                .and_then(|path| {
                                    parameters.iter().find(|descriptor| &descriptor.path() == path)
                                })
                                .map(|descriptor| descriptor.kind);
                            let modes = midi_modes(selected_kind);
                            if previous_kind != selected_kind || !modes.contains(&state.midi_mode) {
                                state.midi_mode = modes.first().copied().unwrap_or(MidiMode::Absolute);
                            }
                            state.midi_parameter.clone_from(&selected_path);
                            if let Some((path, mode)) = &status.midi.learning {
                                let label = parameters
                                    .iter()
                                    .find(|descriptor| &descriptor.path() == path)
                                    .map_or(path.as_str(), |descriptor| {
                                        descriptor.label.as_str()
                                    });
                                ui.colored_label(
                                    Color32::from_rgb(255, 210, 90),
                                    format!(
                                        "Move a MIDI control or press a note for {label} ({})",
                                        mode.label()
                                    ),
                                );
                                if ui.button("Cancel MIDI learn").clicked() {
                                    actions.push(EditorAction::CancelMidiLearn);
                                }
                            } else if !parameters.is_empty() {
                                let selected_label = selected_path
                                    .as_ref()
                                    .and_then(|path| {
                                        parameters
                                            .iter()
                                            .find(|descriptor| &descriptor.path() == path)
                                    })
                                    .map_or("Select parameter", |descriptor| {
                                        descriptor.label.as_str()
                                    });
                                egui::ComboBox::from_id_salt("midi-parameter")
                                    .selected_text(selected_label)
                                    .show_ui(ui, |ui| {
                                        for descriptor in &parameters {
                                            let path = descriptor.path();
                                            let selected =
                                                selected_path.as_ref() == Some(&path);
                                            if ui
                                                .selectable_label(selected, &descriptor.label)
                                                .clicked()
                                            {
                                                state.midi_parameter = Some(path);
                                            }
                                        }
                                    });
                                if let Some(kind) = selected_kind {
                                    let mut mode = state.midi_mode;
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label("Mode:");
                                        for candidate in midi_modes(Some(kind)) {
                                            if ui
                                                .selectable_label(
                                                    mode == *candidate,
                                                    candidate.label(),
                                                )
                                                .clicked()
                                            {
                                                mode = *candidate;
                                            }
                                        }
                                    });
                                    state.midi_mode = mode;
                                    let hint = if kind == ParameterKind::Bool {
                                        "Toggle reacts to a press; Absolute follows note on/off or the CC threshold."
                                    } else {
                                        "Absolute maps 0..127; Relative uses encoder ticks. Note bindings use Absolute."
                                    };
                                    ui.label(
                                        RichText::new(hint)
                                            .small()
                                            .color(Color32::from_rgb(132, 144, 166)),
                                    );
                                }
                                if ui
                                    .add_enabled(
                                        selected_path.is_some()
                                            && !status.midi.devices.is_empty(),
                                        egui::Button::new("Learn MIDI"),
                                    )
                                    .clicked()
                                {
                                    if let Some(parameter) = selected_path {
                                        actions.push(EditorAction::BeginMidiLearn {
                                            parameter,
                                            mode: state.midi_mode,
                                        });
                                    }
                                }
                            }
                        }

                        let midi_parameters = project_parameters(project);
                        for (index, binding) in project.midi_bindings.iter().enumerate() {
                            let parameter_label = midi_parameters
                                .iter()
                                .find(|descriptor| descriptor.path() == binding.parameter)
                                .map_or_else(
                                    || binding.parameter.clone(),
                                    |descriptor| descriptor.label.clone(),
                                );
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(binding.source_label()).monospace());
                                ui.label(binding.mode.label());
                                ui.label("->");
                                ui.label(parameter_label)
                                    .on_hover_text(&binding.parameter);
                                if ui.small_button("Remove").clicked() {
                                    actions.push(EditorAction::RemoveMidiBinding {
                                        binding: index,
                                    });
                                }
                            });
                        }
                        if project.midi_bindings.is_empty() {
                            ui.label("No MIDI bindings yet.");
                        }

                        ui.add_space(12.0);
                        ui.separator();
                        ui.heading("Hotkey bindings");
                        ui.label(
                            RichText::new(
                                "Bind keys to bool and transport parameters. Bindings are saved in the project.",
                            )
                            .small()
                            .color(Color32::from_rgb(132, 144, 166)),
                        );
                        let toggle_parameters: Vec<_> = project_parameters(project)
                            .into_iter()
                            .filter(|descriptor| descriptor.kind == ParameterKind::Bool)
                            .collect();
                        if toggle_parameters.is_empty() {
                            ui.label("No toggleable parameters in this project.");
                        } else {
                            let selected_path = state
                                .binding_parameter
                                .clone()
                                .filter(|path| {
                                    toggle_parameters
                                        .iter()
                                        .any(|descriptor| &descriptor.path() == path)
                                })
                                .or_else(|| toggle_parameters.first().map(|descriptor| descriptor.path()));
                            state.binding_parameter.clone_from(&selected_path);
                            if let Some(path) = &state.binding_learn {
                                let label = toggle_parameters
                                    .iter()
                                    .find(|descriptor| &descriptor.path() == path)
                                    .map_or(path.as_str(), |descriptor| {
                                        descriptor.label.as_str()
                                    });
                                ui.colored_label(
                                    Color32::from_rgb(255, 210, 90),
                                    format!("Press a key for {label} (Esc cancels)"),
                                );
                                if ui.button("Cancel learn").clicked() {
                                    state.binding_learn = None;
                                }
                            } else {
                                let selected_label = selected_path
                                    .as_ref()
                                    .and_then(|path| {
                                        toggle_parameters
                                            .iter()
                                            .find(|descriptor| &descriptor.path() == path)
                                    })
                                    .map_or("Select parameter", |descriptor| {
                                        descriptor.label.as_str()
                                    });
                                egui::ComboBox::from_id_salt("binding-parameter")
                                    .selected_text(selected_label)
                                    .show_ui(ui, |ui| {
                                        for descriptor in &toggle_parameters {
                                            let path = descriptor.path();
                                            let selected =
                                                selected_path.as_ref() == Some(&path);
                                            if ui
                                                .selectable_label(selected, &descriptor.label)
                                                .clicked()
                                            {
                                                state.binding_parameter = Some(path);
                                            }
                                        }
                                    });
                                if ui
                                    .add_enabled(
                                        selected_path.is_some(),
                                        egui::Button::new("Learn key"),
                                    )
                                    .clicked()
                                {
                                    state.binding_learn = selected_path;
                                }
                            }
                        }

                        for (index, binding) in project.bindings.iter().enumerate() {
                            let parameter_label = project_parameters(project)
                                .into_iter()
                                .find(|descriptor| descriptor.path() == binding.parameter)
                                .map_or_else(
                                    || binding.parameter.clone(),
                                    |descriptor| descriptor.label,
                                );
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(binding.shortcut_label()).monospace());
                                ui.label("->");
                                ui.label(parameter_label)
                                    .on_hover_text(&binding.parameter);
                                if ui.small_button("Remove").clicked() {
                                    actions.push(EditorAction::RemoveBinding { binding: index });
                                }
                            });
                        }
                        if project.bindings.is_empty() {
                            ui.label("No bindings yet.");
                        }

                        ui.add_space(12.0);
                        ui.separator();
                        ui.heading("Audio");
                        if !status.audio_supported {
                            ui.colored_label(
                                Color32::from_rgb(255, 184, 92),
                                "This build has no audio support. Rebuild with --features audio.",
                            );
                        }
                        let audio_assets: Vec<(usize, &lpmt_core::model::MediaAsset)> = project
                            .media
                            .iter()
                            .enumerate()
                            .filter(|(_, asset)| asset.kind == MediaKind::Audio)
                            .collect();
                        let current_audio = project
                            .audio_asset()
                            .map_or("None", |asset| asset.name.as_str());
                        egui::ComboBox::from_id_salt("show-audio")
                            .selected_text(current_audio)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(project.audio.media_id.is_none(), "None")
                                    .clicked()
                                {
                                    actions.push(EditorAction::SetShowAudio { media: None });
                                }
                                for (index, asset) in &audio_assets {
                                    let selected =
                                        project.audio.media_id.as_ref() == Some(&asset.id);
                                    if ui.selectable_label(selected, &asset.name).clicked() {
                                        actions.push(EditorAction::SetShowAudio {
                                            media: Some(*index),
                                        });
                                    }
                                }
                            });
                        if audio_assets.is_empty() {
                            ui.label("Add an audio file in the media shelf.");
                        }
                        let mut volume = project.audio.volume;
                        if ui
                            .add(egui::Slider::new(&mut volume, 0.0..=1.0).text("Volume"))
                            .changed()
                        {
                            actions.push(EditorAction::SetShowAudioVolume { volume });
                        }
                        let mut muted = project.audio.muted;
                        if ui.checkbox(&mut muted, "Mute").changed() {
                            actions.push(EditorAction::SetShowAudioMuted { muted });
                        }
                        if let Some(error) = &status.audio_error {
                            ui.colored_label(Color32::from_rgb(255, 120, 120), error);
                        } else if project.audio_asset().is_some() && status.audio_supported {
                            ui.label("Follows the show clock.");
                        }
                    }

                    if state.mode == WorkspaceMode::Calibration {
                        ui.heading("Outputs");
                        ui.label(&status.output);
                        if let Some(pacing) = status.output_pacing.get(state.selected_output) {
                            ui.separator();
                            ui.label(format!(
                                "{} presented · {} missed",
                                pacing.presented,
                                pacing.missed()
                            ));
                            if let Some(fps) = pacing.fps() {
                                ui.label(format!(
                                    "{fps:.1} fps · {:.1} ms average",
                                    pacing.average_interval_ms
                                ));
                            } else {
                                ui.label("Waiting for the first presented frame…");
                            }
                            if pacing.skipped > 0 || pacing.errors > 0 {
                                ui.label(format!(
                                    "{} skipped · {} errors",
                                    pacing.skipped, pacing.errors
                                ));
                            }
                            if pacing.presented > 0 && pacing.since_last_present_ms > 1000.0 {
                                ui.colored_label(
                                    Color32::from_rgb(255, 190, 90),
                                    format!(
                                        "No present for {:.1}s",
                                        pacing.since_last_present_ms / 1000.0
                                    ),
                                );
                            }
                            ui.separator();
                        }

                        for (index, output) in project.outputs.iter().enumerate() {
                            let state_label = if output.enabled { "" } else { " · closed" };
                            let guide =
                                status.output_guides.get(index).copied().unwrap_or_default();
                            let guide_label = if guide == GuideMode::None {
                                String::new()
                            } else {
                                format!(" · {}", guide.label())
                            };
                            let label = format!(
                                "{} · {}{state_label}{guide_label}",
                                output.name,
                                view_label(output.view)
                            );
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
                                state.selected_output =
                                    output.min(project.outputs.len().saturating_sub(2));
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
                            if ui
                                .button("Open now")
                                .on_hover_text(
                                    "Open this output window; on a single display it may cover the editor",
                                )
                                .clicked()
                            {
                                actions.push(EditorAction::OpenOutput {
                                    output: state.selected_output,
                                });
                            }

                            let mut fullscreen =
                                matches!(output.target, OutputTarget::Fullscreen { .. });
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
                                            chosen
                                                .as_deref()
                                                .unwrap_or("Automatic (unused display)"),
                                        )
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut chosen,
                                                None,
                                                "Automatic (unused display)",
                                            );
                                            for name in &status.monitors {
                                                ui.selectable_value(
                                                    &mut chosen,
                                                    Some(name.clone()),
                                                    name,
                                                );
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
                                    for (label, value) in [
                                        "Tint red",
                                        "Tint green",
                                        "Tint blue",
                                    ]
                                    .into_iter()
                                    .zip(color.tint.iter_mut())
                                    {
                                        ui.label(label);
                                        ui.add(
                                            egui::Slider::new(value, 0.0..=4.0)
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

                            ui.add_space(8.0);
                            ui.label("Calibration LUT (1D RGB)");
                            if let Some(lut) = &output.calibration {
                                ui.label(format!("{} · {LUT_SAMPLES} samples/channel", lut.name));
                                if lut.is_identity() {
                                    ui.label("Neutral curves");
                                }
                                if ui.button("Clear LUT").clicked() {
                                    actions.push(EditorAction::ClearOutputLut {
                                        output: state.selected_output,
                                    });
                                }
                            } else {
                                ui.label("None");
                            }
                            if ui.button("Load 1D .cube…").clicked() {
                                actions.push(EditorAction::LoadOutputLut {
                                    output: state.selected_output,
                                });
                            }
                            ui.label("Applied in display space after colour correction.");

                            ui.add_space(8.0);
                            ui.label("Calibration guides (temporary)");
                            let current_guide = status
                                .output_guides
                                .get(state.selected_output)
                                .copied()
                                .unwrap_or_default();
                            let mut chosen_guide = current_guide;
                            egui::ComboBox::from_id_salt("output-guide")
                                .selected_text(chosen_guide.label())
                                .show_ui(ui, |ui| {
                                    for guide in GuideMode::ALL {
                                        ui.selectable_value(
                                            &mut chosen_guide,
                                            guide,
                                            guide.label(),
                                        );
                                    }
                                });
                            if chosen_guide != current_guide {
                                actions.push(EditorAction::SetOutputGuide {
                                    output: state.selected_output,
                                    guide: chosen_guide,
                                });
                            }
                            ui.label("Session only, not saved. Press G to cycle.");

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
                    }

                    ui.add_space(8.0);
                });
            });
    }

    if state.mode != WorkspaceMode::Mapping {
        state.mask_add_mode = false;
    }

    if state.mode == WorkspaceMode::Mapping {
        egui::Panel::bottom("lpmt-media-shelf")
            .resizable(true)
            .default_size(190.0)
            .show(ui, |ui| {
                draw_media_shelf(ui, state, project, status, stage, &mut actions);
            });
    }

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

        let preview_sense = if state.mode == WorkspaceMode::Mapping {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let response = ui.interact(
            preview,
            ui.scope_id().with("surface-preview"),
            preview_sense,
        );
        let painter = ui.painter();
        state.snap_target = None;

        painter.rect_filled(
            preview,
            egui::CornerRadius::same(4),
            Color32::from_rgb(16, 18, 24),
        );
        if let Some(texture_id) = state.preview_texture {
            painter.image(
                texture_id,
                preview,
                EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
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

        // Every surface is visible in the preview so it can be selected
        // directly. The inactive outlines sit under the output views; the
        // active surface is drawn again above them with edit handles.
        for (index, surface) in stage.surfaces.iter().enumerate() {
            if index == state.selected_surface {
                continue;
            }
            let points = surface_points(preview, surface);
            let selected = state.is_surface_selected(index);
            let (stroke_color, fill_color) = if selected {
                (
                    Color32::from_rgb(120, 220, 140),
                    Color32::from_rgba_unmultiplied(120, 220, 140, 24),
                )
            } else if surface.visible {
                (
                    Color32::from_rgb(108, 120, 142),
                    Color32::from_rgba_unmultiplied(96, 108, 132, 16),
                )
            } else {
                (Color32::from_rgb(72, 76, 88), Color32::TRANSPARENT)
            };
            painter.add(Shape::convex_polygon(
                points.clone(),
                fill_color,
                Stroke::new(if selected { 2.0 } else { 1.0 }, stroke_color),
            ));
            painter.text(
                polygon_center(&points),
                Align2::CENTER_CENTER,
                if !surface.visible {
                    format!("{} (hidden)", surface.name)
                } else if selected {
                    format!("{} · selected", surface.name)
                } else {
                    surface.name.clone()
                },
                FontId::proportional(10.0),
                stroke_color,
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
            let points = surface_points(preview, surface);
            painter.add(Shape::convex_polygon(
                points.clone(),
                Color32::from_rgba_unmultiplied(74, 144, 226, 38),
                Stroke::new(2.0, Color32::from_rgb(108, 182, 255)),
            ));
            let selection_count = state.selected_surfaces.len();
            painter.text(
                polygon_center(&points),
                Align2::CENTER_CENTER,
                if selection_count > 1 {
                    format!("{} · editing · {selection_count} selected", surface.name)
                } else {
                    format!("{} · editing", surface.name)
                },
                FontId::proportional(11.0),
                Color32::from_rgb(190, 220, 255),
            );

            if surface.grid_enabled {
                if let Some(grid) = &surface.grid {
                    let grid_color = Color32::from_rgb(108, 182, 255);
                    for row in 0..=grid.rows {
                        for column in 0..=grid.columns {
                            let Some(point) = grid.point(row, column) else {
                                continue;
                            };
                            let screen = stage_to_preview(preview, point);
                            if column < grid.columns {
                                if let Some(next) = grid.point(row, column + 1) {
                                    painter.line_segment(
                                        [screen, stage_to_preview(preview, next)],
                                        Stroke::new(1.0, grid_color),
                                    );
                                }
                            }
                            if row < grid.rows {
                                if let Some(next) = grid.point(row + 1, column) {
                                    painter.line_segment(
                                        [screen, stage_to_preview(preview, next)],
                                        Stroke::new(1.0, grid_color),
                                    );
                                }
                            }
                            let is_corner = (row == 0 || row == grid.rows)
                                && (column == 0 || column == grid.columns);
                            if is_corner {
                                continue;
                            }
                            let active = state.drag_grid == Some((row, column));
                            painter.circle_filled(
                                screen,
                                if active { 8.0 } else { 5.5 },
                                if active {
                                    Color32::from_rgb(255, 210, 90)
                                } else {
                                    Color32::from_rgb(238, 242, 250)
                                },
                            );
                            painter.circle_stroke(
                                screen,
                                5.5,
                                Stroke::new(1.0, Color32::from_rgb(20, 24, 32)),
                            );
                        }
                    }
                }
            }
            if surface.mask.enabled {
                let mask_points: Vec<Pos2> = surface
                    .mask
                    .points
                    .iter()
                    .filter_map(|point| {
                        surface_uv_to_stage(surface, *point)
                            .map(|stage| stage_to_preview(preview, stage))
                    })
                    .collect();
                let mask_color = if surface.mask.invert {
                    Color32::from_rgb(255, 132, 112)
                } else {
                    Color32::from_rgb(255, 170, 82)
                };
                if mask_points.len() >= 2 {
                    for index in 0..mask_points.len() {
                        painter.line_segment(
                            [
                                mask_points[index],
                                mask_points[(index + 1) % mask_points.len()],
                            ],
                            Stroke::new(1.5, mask_color),
                        );
                    }
                }
                for (index, point) in mask_points.iter().enumerate() {
                    let active = state.drag_mask_point == Some(index);
                    painter.circle_filled(
                        *point,
                        if active { 7.0 } else { 5.0 },
                        if active {
                            Color32::from_rgb(255, 210, 90)
                        } else {
                            mask_color
                        },
                    );
                    painter.circle_stroke(
                        *point,
                        5.0,
                        Stroke::new(1.0, Color32::from_rgb(20, 24, 32)),
                    );
                }
            }
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
        }

        // Click a surface to make it active. Shift-click toggles membership in
        // the selection; dragging a selected surface moves the whole selection.
        // Add-points mode or Ctrl-click adds a mask point; right-click removes
        // one.
        let shift = ui.input(|input| input.modifiers.shift);
        let ctrl = ui.input(|input| input.modifiers.ctrl);
        let add_mask_point = state.mask_add_mode || ctrl;
        let mut mask_click_handled = false;
        if response.secondary_clicked() {
            if let (Some(surface), Some(pointer)) = (
                stage.surfaces.get(state.selected_surface),
                response.interact_pointer_pos(),
            ) {
                if surface.mask.enabled {
                    if let Some(point) = nearest_mask_point(preview, surface, pointer, 14.0) {
                        actions.push(EditorAction::RemoveSurfaceMaskPoint {
                            surface: state.selected_surface,
                            point,
                        });
                        mask_click_handled = true;
                    }
                }
            }
        }
        if response.clicked() && !mask_click_handled && add_mask_point {
            if let (Some(surface), Some(pointer)) = (
                stage.surfaces.get(state.selected_surface),
                response.interact_pointer_pos(),
            ) {
                let stage_point = preview_to_stage(preview, pointer);
                if surface.mask.enabled
                    && point_in_surface(surface, stage_point)
                    && nearest_mask_point(preview, surface, pointer, 14.0).is_none()
                {
                    if let Some(uv) = stage_to_surface_uv(surface, stage_point) {
                        let insert_at = nearest_mask_segment(preview, surface, pointer)
                            .map_or(surface.mask.points.len(), |segment| segment + 1);
                        actions.push(EditorAction::AddSurfaceMaskPoint {
                            surface: state.selected_surface,
                            point: uv,
                            insert_at,
                        });
                        mask_click_handled = true;
                    }
                }
            }
        }
        if response.clicked() && !mask_click_handled {
            if let Some(index) = response
                .interact_pointer_pos()
                .and_then(|pointer| surface_at(preview, &stage.surfaces, pointer))
            {
                if shift {
                    state.toggle_surface_selection(index);
                } else {
                    state.select_only_surface(index);
                }
            }
        }

        if response.drag_started() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let mask_point = stage
                    .surfaces
                    .get(state.selected_surface)
                    .filter(|surface| surface.mask.enabled)
                    .and_then(|surface| nearest_mask_point(preview, surface, pointer, 18.0));
                let grid_point = stage
                    .surfaces
                    .get(state.selected_surface)
                    .filter(|surface| surface.grid_enabled)
                    .and_then(|surface| surface.grid.as_ref())
                    .and_then(|grid| nearest_grid_point(preview, grid, pointer, 18.0));
                if let Some(point) = mask_point {
                    state.select_only_surface(state.selected_surface);
                    state.drag_mask_point = Some(point);
                    state.drag_corner = None;
                    state.drag_grid = None;
                    state.surface_move = None;
                } else if let Some((surface, corner)) = nearest_surface_corner(
                    preview,
                    &stage.surfaces,
                    state.selected_surface,
                    pointer,
                    18.0,
                ) {
                    state.select_only_surface(surface);
                    state.drag_corner = Some(corner);
                    state.drag_mask_point = None;
                } else if let Some((row, column)) = grid_point {
                    state.drag_grid = Some((row, column));
                    state.drag_corner = None;
                    state.drag_mask_point = None;
                    state.surface_move = None;
                } else if let Some(surface) = surface_at(preview, &stage.surfaces, pointer) {
                    if shift && !state.is_surface_selected(surface) {
                        state.toggle_surface_selection(surface);
                    } else if !state.is_surface_selected(surface) {
                        state.select_only_surface(surface);
                    } else {
                        state.selected_surface = surface;
                        state.drag_corner = None;
                        state.drag_grid = None;
                        state.drag_mask_point = None;
                    }
                    let originals = state
                        .selected_surfaces
                        .iter()
                        .filter_map(|index| {
                            stage
                                .surfaces
                                .get(*index)
                                .map(|surface| (*index, surface.corners))
                        })
                        .collect::<Vec<_>>();
                    if !originals.is_empty() {
                        state.surface_move = Some(SurfaceMoveDrag {
                            start_stage: preview_to_stage(preview, pointer),
                            originals,
                        });
                    }
                }
            }
        }

        if response.dragged() {
            if let (Some(point), Some(pointer)) =
                (state.drag_mask_point, response.interact_pointer_pos())
            {
                if let Some(surface) = stage.surfaces.get(state.selected_surface) {
                    if let Some(value) =
                        stage_to_surface_uv(surface, preview_to_stage(preview, pointer))
                    {
                        actions.push(EditorAction::SetSurfaceMaskPoint {
                            surface: state.selected_surface,
                            point,
                            value,
                        });
                    }
                }
            } else if let (Some((row, column)), Some(pointer)) =
                (state.drag_grid, response.interact_pointer_pos())
            {
                actions.push(EditorAction::SetGridPoint {
                    surface: state.selected_surface,
                    row,
                    column,
                    value: preview_to_stage(preview, pointer).clamp_unit(),
                });
            } else if let (Some(index), Some(pointer)) =
                (state.drag_corner, response.interact_pointer_pos())
            {
                let mut value = preview_to_stage(preview, pointer).clamp_unit();
                if state.snap_corners {
                    if let Some((snap_value, snap_screen)) = nearest_surface_snap(
                        preview,
                        &stage.surfaces,
                        state.selected_surface,
                        pointer,
                        12.0,
                    ) {
                        value = snap_value;
                        state.snap_target = Some(snap_screen);
                    }
                }
                actions.push(EditorAction::SetCorner {
                    surface: state.selected_surface,
                    corner: index,
                    value,
                });
            } else if let (Some(pointer), Some(drag)) =
                (response.interact_pointer_pos(), &state.surface_move)
            {
                let current = preview_to_stage(preview, pointer);
                let (dx, dy) = clamp_translation(
                    &drag.originals,
                    current.x - drag.start_stage.x,
                    current.y - drag.start_stage.y,
                );
                if dx.abs() > f32::EPSILON || dy.abs() > f32::EPSILON {
                    let surfaces = drag
                        .originals
                        .iter()
                        .map(|(index, corners)| (*index, translated_corners(*corners, dx, dy)))
                        .collect();
                    actions.push(EditorAction::SetSurfaceCorners {
                        surfaces,
                        edit: SurfaceGeometryEdit::Move,
                    });
                }
            }
        }

        if response.drag_stopped() {
            state.drag_corner = None;
            state.drag_grid = None;
            state.drag_mask_point = None;
            state.surface_move = None;
            state.snap_target = None;
        }

        if let Some(target) = state.snap_target {
            painter.circle_stroke(
                target,
                11.0,
                Stroke::new(2.0, Color32::from_rgb(255, 210, 90)),
            );
            painter.text(
                target + Vec2::new(10.0, -12.0),
                Align2::LEFT_TOP,
                "snap",
                FontId::proportional(11.0),
                Color32::from_rgb(255, 220, 120),
            );
        }

        painter.text(
            preview.left_top() + Vec2::new(8.0, 8.0),
            Align2::LEFT_TOP,
            "Stage preview",
            FontId::proportional(12.0),
            Color32::from_rgb(120, 128, 142),
        );
        painter.text(
            preview.left_bottom() + Vec2::new(8.0, -6.0),
            Align2::LEFT_BOTTOM,
            if state.mode == WorkspaceMode::Mapping {
                if state.mask_add_mode {
                    "Click to add a mask point · right-click a point to remove · Shift-click multi-select"
                } else {
                    "Click to edit · Shift-click multi-select · drag surface · Ctrl-click adds a mask point"
                }
            } else {
                "Read-only preview · press F1 for Mapping"
            },
            FontId::proportional(10.0),
            Color32::from_rgb(120, 128, 142),
        );
    });

    draw_media_picker(ui, state, project, status, stage, &mut actions);

    actions
}

fn draw_media_shelf(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    project: &Project,
    status: &EditorStatus,
    stage: &Stage,
    actions: &mut Vec<EditorAction>,
) {
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.heading("Media shelf");
        if ui.button("Add media…").clicked() {
            actions.push(EditorAction::AddMedia);
            state.selected_media = project.media.len();
        }
        if let Some(layer) = stage.layers.get(state.selected_layer) {
            ui.separator();
            ui.label(format!(
                "Target: {} · {}",
                layer.name,
                layer_surface_label(stage, layer)
            ));
        }
        if let Some(asset) = project.media.get(state.selected_media) {
            ui.separator();
            ui.label(
                RichText::new(media_usage(project, asset))
                    .small()
                    .color(Color32::from_rgb(132, 144, 166)),
            );
        }
    });

    if project.media.is_empty() {
        ui.label("No media yet. Add an image, video or audio file.");
        return;
    }

    egui::ScrollArea::horizontal()
        .id_salt("media-shelf-scroll")
        .max_height(96.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (index, asset) in project.media.iter().enumerate() {
                    let mut clicked = false;
                    ui.vertical(|ui| {
                        ui.set_width(112.0);
                        let texture = status.media_thumbnails.get(&asset.id).copied();
                        let response =
                            media_thumbnail(ui, texture, asset.kind, Vec2::new(104.0, 58.0))
                                .on_hover_text(media_usage(project, asset));
                        if response.clicked() {
                            clicked = true;
                        }
                        let name = if asset.name.chars().count() > 14 {
                            format!("{}…", asset.name.chars().take(13).collect::<String>())
                        } else {
                            asset.name.clone()
                        };
                        if ui
                            .selectable_label(state.selected_media == index, name)
                            .clicked()
                        {
                            clicked = true;
                        }
                        ui.label(
                            RichText::new(media_kind_label(asset.kind))
                                .small()
                                .color(Color32::from_rgb(130, 142, 164)),
                        );
                    });
                    if clicked {
                        state.selected_media = index;
                        state.image_path.clone_from(&asset.path);
                    }
                }
            });
        });

    let selected_media_used = project
        .media
        .get(state.selected_media)
        .is_some_and(|asset| project.is_media_used(&asset.id));
    if let Some(asset) = project.media.get(state.selected_media) {
        ui.horizontal_wrapped(|ui| {
            if asset.kind == MediaKind::Audio {
                if ui.button("Assign as show audio").clicked() {
                    actions.push(EditorAction::SetShowAudio {
                        media: Some(state.selected_media),
                    });
                }
            } else {
                let target = stage.layers.get(state.selected_layer);
                let assign_label = target.map_or_else(
                    || "Select a layer".to_owned(),
                    |layer| format!("Use for {}", layer.name),
                );
                if ui
                    .add_enabled(target.is_some(), egui::Button::new(assign_label))
                    .clicked()
                {
                    actions.push(EditorAction::SetLayerMedia {
                        layer: state.selected_layer,
                        media: state.selected_media,
                    });
                }
                let selected_surfaces: Vec<usize> =
                    state.selected_surfaces.iter().copied().collect();
                let add_label = if selected_surfaces.len() == 1 {
                    format!(
                        "New layer on {}",
                        stage
                            .surfaces
                            .get(state.selected_surface)
                            .map_or("surface", |surface| surface.name.as_str())
                    )
                } else {
                    format!("New layer on {} surfaces", selected_surfaces.len())
                };
                if ui
                    .add_enabled(!selected_surfaces.is_empty(), egui::Button::new(add_label))
                    .clicked()
                {
                    actions.push(EditorAction::AddLayerFromMedia {
                        surfaces: selected_surfaces,
                        media: state.selected_media,
                    });
                }
            }
            if ui.button("Reload").clicked() {
                actions.push(EditorAction::ReloadMedia);
            }
            let remove = ui.add_enabled(!selected_media_used, egui::Button::new("Remove"));
            if remove.clicked() {
                let media = state.selected_media;
                actions.push(EditorAction::RemoveMedia { media });
                state.selected_media = media.min(project.media.len().saturating_sub(2));
            }
            if selected_media_used {
                remove.on_disabled_hover_text("Reassign this asset before removing it");
            }
            if asset.kind == MediaKind::Video && !status.video_supported {
                ui.colored_label(
                    Color32::from_rgb(255, 184, 92),
                    "Video support is not compiled in",
                );
            }
        });
        ui.horizontal(|ui| {
            ui.label("Path");
            ui.add(
                egui::TextEdit::singleline(&mut state.image_path)
                    .desired_width((ui.available_width() - 190.0).max(120.0)),
            );
            if ui.button("Browse…").clicked() {
                actions.push(EditorAction::PickMediaFile {
                    media: state.selected_media,
                });
            }
            if ui.button("Apply").clicked() {
                actions.push(EditorAction::SetMediaPath {
                    media: state.selected_media,
                    path: state.image_path.clone(),
                });
            }
        });
    }
}

fn draw_timeline(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    project: &Project,
    status: &EditorStatus,
    actions: &mut Vec<EditorAction>,
) {
    const LABEL_WIDTH: f32 = 190.0;
    const LANE_HEIGHT: f32 = 34.0;
    let timeline = &project.timeline;
    let duration = timeline.duration.max(MIN_TIMELINE_DURATION);
    let parameters = project_parameters(project);
    let shift = ui.input(|input| input.modifiers.shift);

    if state
        .timeline_selected_track
        .is_some_and(|track| track >= timeline.tracks.len())
    {
        state.timeline_selected_track = None;
        state.timeline_selected_key = None;
    }
    if let Some(track) = state.timeline_selected_track {
        let key_count = timeline.tracks[track].keys.len();
        state.timeline_selected_keys.retain(|key| *key < key_count);
        if state
            .timeline_selected_key
            .is_some_and(|key| key >= key_count)
        {
            state.timeline_selected_key = None;
        }
        if let Some(key) = state.timeline_selected_key {
            state.timeline_selected_keys.insert(key);
        } else if let Some(key) = state.timeline_selected_keys.iter().next_back().copied() {
            state.timeline_selected_key = Some(key);
        }
    } else {
        state.timeline_selected_keys.clear();
    }
    if state.timeline_drag_key.is_some_and(|(track, key)| {
        timeline
            .tracks
            .get(track)
            .is_none_or(|track| key >= track.keys.len())
    }) {
        state.timeline_drag_key = None;
    }
    if state
        .timeline_selected_trigger
        .is_some_and(|trigger| trigger >= timeline.triggers.len())
    {
        state.timeline_selected_trigger = None;
    }
    if state
        .timeline_drag_trigger
        .is_some_and(|trigger| trigger >= timeline.triggers.len())
    {
        state.timeline_drag_trigger = None;
    }
    if state
        .timeline_selected_marker
        .is_some_and(|marker| marker >= timeline.markers.len())
    {
        state.timeline_selected_marker = None;
        state.timeline_marker_name_id = None;
    }
    if state
        .timeline_drag_marker
        .is_some_and(|marker| marker >= timeline.markers.len())
    {
        state.timeline_drag_marker = None;
    }
    if state.timeline_selected_marker.is_some()
        && (state.timeline_selected_track.is_some() || state.timeline_selected_trigger.is_some())
    {
        state.timeline_selected_marker = None;
        state.timeline_marker_name_id = None;
    }
    if let Some(marker) = state.timeline_selected_marker {
        if state.timeline_marker_name_id != Some(marker) {
            state.timeline_marker_name = timeline.markers[marker].name.clone();
            state.timeline_marker_name_id = Some(marker);
        }
    }

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.heading("Timeline");
        let mut duration_edit = timeline.duration;
        if ui
            .add(
                egui::DragValue::new(&mut duration_edit)
                    .range(MIN_TIMELINE_DURATION..=MAX_TIMELINE_DURATION)
                    .speed(0.05)
                    .suffix(" s"),
            )
            .changed()
        {
            actions.push(EditorAction::SetTimelineDuration {
                duration: duration_edit,
            });
        }

        ui.separator();
        let candidates: Vec<&ParameterDescriptor> = parameters
            .iter()
            .filter(|descriptor| {
                timeline_candidate(descriptor)
                    && !timeline
                        .tracks
                        .iter()
                        .any(|track| track.parameter == descriptor.path())
            })
            .collect();
        if candidates.is_empty() {
            ui.label("All timeline parameters have tracks.");
        } else {
            let selected_path = state
                .timeline_add_parameter
                .clone()
                .filter(|path| {
                    candidates
                        .iter()
                        .any(|descriptor| &descriptor.path() == path)
                })
                .or_else(|| candidates.first().map(|descriptor| descriptor.path()));
            state.timeline_add_parameter.clone_from(&selected_path);
            let selected_label = selected_path
                .as_ref()
                .and_then(|path| {
                    candidates
                        .iter()
                        .find(|descriptor| &descriptor.path() == path)
                })
                .map_or("Select parameter", |descriptor| descriptor.label.as_str());
            egui::ComboBox::from_id_salt("timeline-add-parameter")
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    for descriptor in &candidates {
                        let path = descriptor.path();
                        if ui
                            .selectable_label(
                                selected_path.as_ref() == Some(&path),
                                &descriptor.label,
                            )
                            .clicked()
                        {
                            state.timeline_add_parameter = Some(path);
                        }
                    }
                });
            if ui
                .add_enabled(selected_path.is_some(), egui::Button::new("Add track"))
                .clicked()
            {
                if let Some(parameter) = selected_path {
                    let track = timeline
                        .track_index(&parameter)
                        .unwrap_or(timeline.tracks.len());
                    state.timeline_selected_track = Some(track);
                    state.timeline_selected_key = Some(0);
                    state.timeline_selected_keys.clear();
                    state.timeline_selected_trigger = None;
                    actions.push(EditorAction::AddTimelineTrack { parameter });
                }
            }
        }

        if let Some(track) = state.timeline_selected_track {
            if ui.button("Remove track").clicked() {
                actions.push(EditorAction::RemoveTimelineTrack { track });
                state.timeline_selected_track = None;
                state.timeline_selected_key = None;
                state.timeline_selected_keys.clear();
            }
            if let Some(clipboard) = state.timeline_clipboard.clone() {
                let label = if clipboard.keys.len() > 1 {
                    format!("Paste {} keys at playhead", clipboard.keys.len())
                } else {
                    "Paste key at playhead".to_owned()
                };
                if ui.button(label).clicked() {
                    let time = status.transport.position_seconds as f32;
                    let insert_at = timeline
                        .tracks
                        .get(track)
                        .map(|track| {
                            track
                                .keys
                                .iter()
                                .position(|key| key.time > time)
                                .unwrap_or(track.keys.len())
                        })
                        .unwrap_or(0);
                    state.timeline_selected_key = Some(insert_at);
                    state.timeline_selected_keys.clear();
                    state.timeline_selected_keys.insert(insert_at);
                    actions.push(EditorAction::PasteTimelineKeys {
                        track,
                        time,
                        keys: clipboard.keys,
                    });
                }
            }
        }

        ui.separator();
        let mut command = state.timeline_trigger_command;
        egui::ComboBox::from_id_salt("timeline-trigger-command")
            .selected_text(command.label())
            .show_ui(ui, |ui| {
                for candidate in TimelineCommand::ALL {
                    if ui
                        .selectable_label(command == candidate, candidate.label())
                        .clicked()
                    {
                        command = candidate;
                    }
                }
            });
        state.timeline_trigger_command = command;
        if ui.button("Add trigger").clicked() {
            let time = status.transport.position_seconds as f32;
            let trigger = timeline
                .triggers
                .iter()
                .position(|trigger| trigger.time > time)
                .unwrap_or(timeline.triggers.len());
            state.timeline_selected_trigger = Some(trigger);
            state.timeline_selected_track = None;
            state.timeline_selected_key = None;
            actions.push(EditorAction::AddTimelineTrigger { time, command });
        }
        if ui.button("Add marker").clicked() {
            let time = status.transport.position_seconds as f32;
            let marker = timeline
                .markers
                .iter()
                .position(|marker| marker.time > time)
                .unwrap_or(timeline.markers.len());
            state.timeline_selected_marker = Some(marker);
            state.timeline_marker_name_id = None;
            state.timeline_selected_track = None;
            state.timeline_selected_key = None;
            state.timeline_selected_trigger = None;
            actions.push(EditorAction::AddTimelineMarker { time });
        }
        let playhead = status.transport.position_seconds as f32;
        let previous = timeline
            .markers
            .iter()
            .rev()
            .find(|marker| marker.time < playhead - 1.0e-4)
            .map(|marker| marker.time);
        let next = timeline
            .markers
            .iter()
            .find(|marker| marker.time > playhead + 1.0e-4)
            .map(|marker| marker.time);
        if ui
            .add_enabled(previous.is_some(), egui::Button::new("Prev marker"))
            .clicked()
        {
            if let Some(time) = previous {
                actions.push(EditorAction::SeekTransport {
                    position: f64::from(time),
                });
            }
        }
        if ui
            .add_enabled(next.is_some(), egui::Button::new("Next marker"))
            .clicked()
        {
            if let Some(time) = next {
                actions.push(EditorAction::SeekTransport {
                    position: f64::from(time),
                });
            }
        }
    });

    if let Some((track_index, key_index)) = state
        .timeline_selected_track
        .zip(state.timeline_selected_key)
    {
        if let Some(track) = timeline.tracks.get(track_index) {
            if let Some(key) = track.keys.get(key_index).copied() {
                let label = timeline_parameter_label(&parameters, &track.parameter);
                let (minimum, maximum) = timeline_parameter_range(&parameters, &track.parameter);
                let kind = parameters
                    .iter()
                    .find(|descriptor| descriptor.path() == track.parameter)
                    .map_or(ParameterKind::Float, |descriptor| descriptor.kind);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(short_label(&label, 28)).strong());
                    ui.label("time");
                    let mut time = key.time;
                    if ui
                        .add(
                            egui::DragValue::new(&mut time)
                                .range(0.0..=duration)
                                .speed(0.01)
                                .suffix(" s"),
                        )
                        .changed()
                    {
                        actions.push(EditorAction::SetTimelineKey {
                            track: track_index,
                            key: key_index,
                            time,
                            value: key.value,
                            interpolation: key.interpolation,
                        });
                    }
                    ui.label("value");
                    if kind == ParameterKind::Bool {
                        let mut checked = key.value >= 0.5;
                        if ui.checkbox(&mut checked, "on").changed() {
                            actions.push(EditorAction::SetTimelineKey {
                                track: track_index,
                                key: key_index,
                                time: key.time,
                                value: if checked { 1.0 } else { 0.0 },
                                interpolation: TimelineInterpolation::Step,
                            });
                        }
                        ui.label("Step (bool)");
                    } else {
                        let mut value = key.value.clamp(minimum, maximum);
                        if ui
                            .add(egui::Slider::new(&mut value, minimum..=maximum).fixed_decimals(3))
                            .changed()
                        {
                            actions.push(EditorAction::SetTimelineKey {
                                track: track_index,
                                key: key_index,
                                time: key.time,
                                value,
                                interpolation: key.interpolation,
                            });
                        }
                        let mut interpolation = key.interpolation;
                        egui::ComboBox::from_id_salt("timeline-key-interpolation")
                            .selected_text(interpolation.label())
                            .show_ui(ui, |ui| {
                                for candidate in TimelineInterpolation::ALL {
                                    if ui
                                        .selectable_label(
                                            interpolation == candidate,
                                            candidate.label(),
                                        )
                                        .clicked()
                                    {
                                        interpolation = candidate;
                                    }
                                }
                            });
                        if interpolation != key.interpolation {
                            actions.push(EditorAction::SetTimelineKey {
                                track: track_index,
                                key: key_index,
                                time: key.time,
                                value: key.value,
                                interpolation,
                            });
                        }
                    }
                    let selected_count = state.timeline_selected_keys.len().max(1);
                    let copy_label = if selected_count > 1 {
                        format!("Copy {selected_count} keys")
                    } else {
                        "Copy key".to_owned()
                    };
                    if ui.button(copy_label).clicked() {
                        let mut indices: Vec<usize> = state
                            .timeline_selected_keys
                            .iter()
                            .copied()
                            .filter(|index| *index < track.keys.len())
                            .collect();
                        if indices.is_empty() {
                            indices.push(key_index);
                        }
                        if let Some(base) = indices
                            .iter()
                            .filter_map(|index| track.keys.get(*index))
                            .map(|key| key.time)
                            .min_by(f32::total_cmp)
                        {
                            let keys = indices
                                .iter()
                                .filter_map(|index| track.keys.get(*index))
                                .map(|key| TimelineKey {
                                    time: key.time - base,
                                    value: key.value,
                                    interpolation: key.interpolation,
                                })
                                .collect();
                            state.timeline_clipboard = Some(TimelineClipboard { keys });
                        }
                    }
                    let can_paste = state.timeline_clipboard.is_some();
                    if ui
                        .add_enabled(can_paste, egui::Button::new("Paste at playhead"))
                        .clicked()
                    {
                        if let Some(clipboard) = state.timeline_clipboard.clone() {
                            let time = status.transport.position_seconds as f32;
                            let insert_at = track
                                .keys
                                .iter()
                                .position(|key| key.time > time)
                                .unwrap_or(track.keys.len());
                            state.timeline_selected_track = Some(track_index);
                            state.timeline_selected_key = Some(insert_at);
                            state.timeline_selected_keys.clear();
                            state.timeline_selected_keys.insert(insert_at);
                            state.timeline_selected_trigger = None;
                            actions.push(EditorAction::PasteTimelineKeys {
                                track: track_index,
                                time,
                                keys: clipboard.keys,
                            });
                        }
                    }
                    if ui.button("Delete key").clicked() {
                        actions.push(EditorAction::RemoveTimelineKey {
                            track: track_index,
                            key: key_index,
                        });
                        state.timeline_selected_keys.remove(&key_index);
                        state.timeline_selected_key =
                            state.timeline_selected_keys.iter().next_back().copied();
                    }
                    if selected_count > 1 {
                        ui.label(
                            RichText::new(format!(
                                "{selected_count} keys selected; shift-click to extend"
                            ))
                            .small()
                            .color(Color32::from_rgb(132, 144, 166)),
                        );
                    }
                });
            }
        }
    } else if state.timeline_selected_trigger.is_none() {
        ui.label(
            RichText::new(
                "Double-click a track lane to add a key. Drag a key to move it; right-click to remove it.",
            )
            .small()
            .color(Color32::from_rgb(132, 144, 166)),
        );
    }

    if let Some(trigger_index) = state.timeline_selected_trigger {
        if let Some(trigger) = timeline.triggers.get(trigger_index).copied() {
            let mut time = trigger.time;
            let mut command = trigger.command;
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Trigger").strong());
                ui.label("time");
                ui.add(
                    egui::DragValue::new(&mut time)
                        .range(0.0..=duration)
                        .speed(0.01)
                        .suffix(" s"),
                );
                egui::ComboBox::from_id_salt("timeline-trigger-edit-command")
                    .selected_text(command.label())
                    .show_ui(ui, |ui| {
                        for candidate in TimelineCommand::ALL {
                            if ui
                                .selectable_label(command == candidate, candidate.label())
                                .clicked()
                            {
                                command = candidate;
                            }
                        }
                    });
                if ui.button("Delete trigger").clicked() {
                    actions.push(EditorAction::RemoveTimelineTrigger {
                        trigger: trigger_index,
                    });
                    state.timeline_selected_trigger = None;
                }
            });
            if time != trigger.time || command != trigger.command {
                actions.push(EditorAction::SetTimelineTrigger {
                    trigger: trigger_index,
                    time,
                    command,
                });
            }
        }
    }

    if let Some(marker_index) = state.timeline_selected_marker {
        if let Some(marker) = timeline.markers.get(marker_index).cloned() {
            let mut time = marker.time;
            let mut name = state.timeline_marker_name.clone();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Marker").strong());
                ui.label("time");
                ui.add(
                    egui::DragValue::new(&mut time)
                        .range(0.0..=duration)
                        .speed(0.01)
                        .suffix(" s"),
                );
                ui.label("name");
                ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(180.0)
                        .hint_text("Marker name"),
                );
                if ui.button("Delete marker").clicked() {
                    actions.push(EditorAction::RemoveTimelineMarker {
                        marker: marker_index,
                    });
                    state.timeline_selected_marker = None;
                    state.timeline_marker_name_id = None;
                }
            });
            if time != marker.time || name != marker.name {
                state.timeline_marker_name.clone_from(&name);
                actions.push(EditorAction::SetTimelineMarker {
                    marker: marker_index,
                    time,
                    name,
                });
            }
        }
    }

    let tick_step = timeline_tick_step(duration);
    ui.horizontal(|ui| {
        let (_, _) = ui.allocate_exact_size(Vec2::new(LABEL_WIDTH, 20.0), Sense::hover());
        let width = ui.available_width().max(1.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::click_and_drag());
        let painter = ui.painter();
        painter.rect_filled(rect, 2.0, Color32::from_rgb(22, 25, 33));
        let mut time = 0.0;
        while time <= duration + 0.0001 {
            let x = timeline_time_x(rect, time, duration);
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_rgb(48, 53, 66)),
            );
            painter.text(
                Pos2::new(x + 3.0, rect.top() + 2.0),
                Align2::LEFT_TOP,
                format!("{time:.1}s"),
                FontId::proportional(10.0),
                Color32::from_rgb(130, 140, 160),
            );
            time += tick_step;
        }
        let playhead = timeline_time_x(rect, status.transport.position_seconds as f32, duration);
        painter.line_segment(
            [
                Pos2::new(playhead, rect.top()),
                Pos2::new(playhead, rect.bottom()),
            ],
            Stroke::new(2.0, Color32::from_rgb(255, 120, 90)),
        );
        if let Some(pointer) = response.interact_pointer_pos() {
            if response.clicked() || response.dragged() {
                let time = timeline_x_time(rect, pointer.x, duration);
                actions.push(EditorAction::SeekTransport {
                    position: f64::from(time),
                });
            }
        }
    });

    egui::ScrollArea::vertical()
        .id_salt("timeline-tracks")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (track_index, track) in timeline.tracks.iter().enumerate() {
                let label = timeline_parameter_label(&parameters, &track.parameter);
                let (minimum, maximum) = timeline_parameter_range(&parameters, &track.parameter);
                let selected_track = state.timeline_selected_track == Some(track_index);
                ui.horizontal(|ui| {
                    let (label_rect, label_response) =
                        ui.allocate_exact_size(Vec2::new(LABEL_WIDTH, LANE_HEIGHT), Sense::click());
                    ui.painter().rect_filled(
                        label_rect,
                        3.0,
                        if selected_track {
                            Color32::from_rgb(43, 68, 72)
                        } else {
                            Color32::from_rgb(30, 34, 43)
                        },
                    );
                    ui.painter().text(
                        Pos2::new(label_rect.left() + 7.0, label_rect.center().y),
                        Align2::LEFT_CENTER,
                        short_label(&label, 25),
                        FontId::proportional(12.0),
                        if selected_track {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(190, 198, 214)
                        },
                    );
                    if label_response.clicked() {
                        state.timeline_selected_track = Some(track_index);
                        state.timeline_selected_key = None;
                        state.timeline_selected_keys.clear();
                        state.timeline_drag_key = None;
                        state.timeline_selected_trigger = None;
                    }
                    label_response.on_hover_text(&track.parameter);

                    let width = ui.available_width().max(1.0);
                    let (lane_rect, response) = ui.allocate_exact_size(
                        Vec2::new(width, LANE_HEIGHT),
                        Sense::click_and_drag(),
                    );
                    let painter = ui.painter();
                    painter.rect_filled(
                        lane_rect,
                        3.0,
                        if selected_track {
                            Color32::from_rgb(26, 31, 40)
                        } else {
                            Color32::from_rgb(20, 23, 30)
                        },
                    );
                    let mut tick = 0.0;
                    while tick <= duration + 0.0001 {
                        let x = timeline_time_x(lane_rect, tick, duration);
                        painter.line_segment(
                            [
                                Pos2::new(x, lane_rect.top()),
                                Pos2::new(x, lane_rect.bottom()),
                            ],
                            Stroke::new(1.0, Color32::from_rgb(38, 43, 55)),
                        );
                        tick += tick_step;
                    }

                    let value_y = |value: f32| {
                        let span = (maximum - minimum).max(1.0e-5);
                        let normalized = ((value - minimum) / span).clamp(0.0, 1.0);
                        lane_rect.bottom() - 5.0 - normalized * (lane_rect.height() - 10.0)
                    };
                    let curve_color = Color32::from_rgb(105, 205, 225);
                    if let Some(first) = track.keys.first() {
                        painter.line_segment(
                            [
                                Pos2::new(lane_rect.left(), value_y(first.value)),
                                Pos2::new(
                                    timeline_time_x(lane_rect, first.time, duration),
                                    value_y(first.value),
                                ),
                            ],
                            Stroke::new(1.5, curve_color),
                        );
                    }
                    for pair in track.keys.windows(2) {
                        let a = pair[0];
                        let b = pair[1];
                        let ax = timeline_time_x(lane_rect, a.time, duration);
                        let bx = timeline_time_x(lane_rect, b.time, duration);
                        let ay = value_y(a.value);
                        let by = value_y(b.value);
                        match a.interpolation {
                            TimelineInterpolation::Step => {
                                painter.line_segment(
                                    [Pos2::new(ax, ay), Pos2::new(bx, ay)],
                                    Stroke::new(1.5, curve_color),
                                );
                                painter.line_segment(
                                    [Pos2::new(bx, ay), Pos2::new(bx, by)],
                                    Stroke::new(1.5, curve_color),
                                );
                            }
                            TimelineInterpolation::Linear
                            | TimelineInterpolation::Smooth => {
                                painter.line_segment(
                                    [Pos2::new(ax, ay), Pos2::new(bx, by)],
                                    Stroke::new(1.5, curve_color),
                                );
                            }
                        }
                    }
                    if let Some(last) = track.keys.last() {
                        painter.line_segment(
                            [
                                Pos2::new(
                                    timeline_time_x(lane_rect, last.time, duration),
                                    value_y(last.value),
                                ),
                                Pos2::new(lane_rect.right(), value_y(last.value)),
                            ],
                            Stroke::new(1.5, curve_color),
                        );
                    }

                    let key_points: Vec<Pos2> = track
                        .keys
                        .iter()
                        .map(|key| {
                            Pos2::new(
                                timeline_time_x(lane_rect, key.time, duration),
                                value_y(key.value),
                            )
                        })
                        .collect();
                    for (key_index, point) in key_points.iter().enumerate() {
                        let selected = state.timeline_selected_track == Some(track_index)
                            && state.timeline_selected_keys.contains(&key_index);
                        painter.circle_filled(
                            *point,
                            if selected { 6.0 } else { 4.5 },
                            if selected {
                                Color32::from_rgb(255, 210, 90)
                            } else {
                                Color32::from_rgb(238, 242, 250)
                            },
                        );
                        painter.circle_stroke(
                            *point,
                            if selected { 6.0 } else { 4.5 },
                            Stroke::new(1.0, Color32::from_rgb(18, 22, 28)),
                        );
                    }

                    let playhead = timeline_time_x(
                        lane_rect,
                        status.transport.position_seconds as f32,
                        duration,
                    );
                    painter.line_segment(
                        [
                            Pos2::new(playhead, lane_rect.top()),
                            Pos2::new(playhead, lane_rect.bottom()),
                        ],
                        Stroke::new(1.5, Color32::from_rgb(255, 120, 90)),
                    );

                    let pointer = response.interact_pointer_pos();
                    let hit = pointer.and_then(|pointer| {
                        key_points
                            .iter()
                            .enumerate()
                            .map(|(index, point)| (index, point.distance(pointer)))
                            .filter(|(_, distance)| *distance <= 9.0)
                            .min_by(|left, right| left.1.total_cmp(&right.1))
                            .map(|(index, _)| index)
                    });

                    if response.secondary_clicked() {
                        if let Some(key) = hit {
                            if state.timeline_selected_track != Some(track_index) {
                                state.timeline_selected_keys.clear();
                            }
                            actions.push(EditorAction::RemoveTimelineKey {
                                track: track_index,
                                key,
                            });
                            state.timeline_selected_track = Some(track_index);
                            state.timeline_selected_keys.remove(&key);
                            state.timeline_selected_key = state
                                .timeline_selected_keys
                                .iter()
                                .next_back()
                                .copied();
                            state.timeline_drag_key = None;
                            state.timeline_selected_trigger = None;
                        }
                    } else if response.double_clicked() {
                        if let Some(pointer) = pointer {
                            if hit.is_none() {
                                let time = timeline_x_time(lane_rect, pointer.x, duration);
                                let insert_at = track
                                    .keys
                                    .iter()
                                    .position(|key| key.time > time)
                                    .unwrap_or(track.keys.len());
                                state.timeline_selected_track = Some(track_index);
                                state.timeline_selected_key = Some(insert_at);
                                state.timeline_selected_keys.clear();
                                state.timeline_selected_keys.insert(insert_at);
                                state.timeline_selected_trigger = None;
                                actions.push(EditorAction::AddTimelineKey {
                                    track: track_index,
                                    time,
                                });
                            }
                        }
                    } else if response.clicked() {
                        if state.timeline_selected_track != Some(track_index) {
                            state.timeline_selected_keys.clear();
                        }
                        state.timeline_selected_track = Some(track_index);
                        state.timeline_selected_trigger = None;
                        select_timeline_key(
                            &mut state.timeline_selected_keys,
                            &mut state.timeline_selected_key,
                            hit,
                            shift,
                        );
                        if hit.is_some() {
                            state.timeline_drag_key = None;
                        }
                    }

                    if response.drag_started() {
                        if let Some(key) = hit {
                            if state.timeline_selected_track != Some(track_index) {
                                state.timeline_selected_keys.clear();
                            }
                            state.timeline_selected_track = Some(track_index);
                            state.timeline_selected_key = Some(key);
                            state.timeline_selected_keys.clear();
                            state.timeline_selected_keys.insert(key);
                            state.timeline_drag_key = Some((track_index, key));
                            state.timeline_selected_trigger = None;
                        }
                    }
                    if response.dragged() {
                        if let (Some((drag_track, drag_key)), Some(pointer)) =
                            (state.timeline_drag_key, pointer)
                        {
                            if drag_track == track_index {
                                if let Some(key) = track.keys.get(drag_key) {
                                    let time = timeline_x_time(lane_rect, pointer.x, duration);
                                    actions.push(EditorAction::SetTimelineKey {
                                        track: track_index,
                                        key: drag_key,
                                        time,
                                        value: key.value,
                                        interpolation: key.interpolation,
                                    });
                                }
                            }
                        }
                    }
                    if response.drag_stopped() {
                        state.timeline_drag_key = None;
                    }
                });
            }

            let selected_trigger = state.timeline_selected_trigger.is_some();
            ui.horizontal(|ui| {
                let (label_rect, label_response) = ui.allocate_exact_size(
                    Vec2::new(LABEL_WIDTH, LANE_HEIGHT),
                    Sense::click(),
                );
                ui.painter().rect_filled(
                    label_rect,
                    3.0,
                    if selected_trigger {
                        Color32::from_rgb(58, 52, 40)
                    } else {
                        Color32::from_rgb(30, 34, 43)
                    },
                );
                ui.painter().text(
                    Pos2::new(label_rect.left() + 7.0, label_rect.center().y),
                    Align2::LEFT_CENTER,
                    "Triggers",
                    FontId::proportional(12.0),
                    if selected_trigger {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(210, 190, 150)
                    },
                );
                if label_response.clicked() {
                    state.timeline_selected_trigger = None;
                    state.timeline_selected_track = None;
                    state.timeline_selected_key = None;
                    state.timeline_drag_trigger = None;
                }

                let width = ui.available_width().max(1.0);
                let (lane_rect, response) = ui.allocate_exact_size(
                    Vec2::new(width, LANE_HEIGHT),
                    Sense::click_and_drag(),
                );
                let painter = ui.painter();
                painter.rect_filled(
                    lane_rect,
                    3.0,
                    if selected_trigger {
                        Color32::from_rgb(32, 30, 27)
                    } else {
                        Color32::from_rgb(22, 23, 28)
                    },
                );
                let mut tick = 0.0;
                while tick <= duration + 0.0001 {
                    let x = timeline_time_x(lane_rect, tick, duration);
                    painter.line_segment(
                        [
                            Pos2::new(x, lane_rect.top()),
                            Pos2::new(x, lane_rect.bottom()),
                        ],
                        Stroke::new(1.0, Color32::from_rgb(40, 41, 48)),
                    );
                    tick += tick_step;
                }
                let playhead = timeline_time_x(
                    lane_rect,
                    status.transport.position_seconds as f32,
                    duration,
                );
                painter.line_segment(
                    [
                        Pos2::new(playhead, lane_rect.top()),
                        Pos2::new(playhead, lane_rect.bottom()),
                    ],
                    Stroke::new(1.5, Color32::from_rgb(255, 120, 90)),
                );

                let trigger_points: Vec<Pos2> = timeline
                    .triggers
                    .iter()
                    .map(|trigger| {
                        Pos2::new(
                            timeline_time_x(lane_rect, trigger.time, duration),
                            lane_rect.center().y,
                        )
                    })
                    .collect();
                for (trigger_index, point) in trigger_points.iter().enumerate() {
                    let selected = state.timeline_selected_trigger == Some(trigger_index);
                    painter.circle_filled(
                        *point,
                        if selected { 6.0 } else { 5.0 },
                        if selected {
                            Color32::from_rgb(255, 210, 90)
                        } else {
                            Color32::from_rgb(220, 170, 100)
                        },
                    );
                    painter.circle_stroke(
                        *point,
                        if selected { 6.0 } else { 5.0 },
                        Stroke::new(1.0, Color32::from_rgb(40, 32, 20)),
                    );
                    if let Some(trigger) = timeline.triggers.get(trigger_index) {
                        painter.text(
                            *point + Vec2::new(9.0, 0.0),
                            Align2::LEFT_CENTER,
                            trigger_command_short(trigger.command),
                            FontId::proportional(10.0),
                            Color32::from_rgb(220, 190, 130),
                        );
                    }
                }

                let pointer = response.interact_pointer_pos();
                let hit = pointer.and_then(|pointer| {
                    trigger_points
                        .iter()
                        .enumerate()
                        .map(|(index, point)| (index, point.distance(pointer)))
                        .filter(|(_, distance)| *distance <= 10.0)
                        .min_by(|left, right| left.1.total_cmp(&right.1))
                        .map(|(index, _)| index)
                });

                if response.secondary_clicked() {
                    if let Some(trigger) = hit {
                        actions.push(EditorAction::RemoveTimelineTrigger { trigger });
                        state.timeline_selected_trigger = None;
                        state.timeline_drag_trigger = None;
                    }
                } else if response.double_clicked() {
                    if let Some(pointer) = pointer {
                        if hit.is_none() {
                            let time = timeline_x_time(lane_rect, pointer.x, duration);
                            let trigger = timeline
                                .triggers
                                .iter()
                                .position(|trigger| trigger.time > time)
                                .unwrap_or(timeline.triggers.len());
                            state.timeline_selected_trigger = Some(trigger);
                            state.timeline_selected_track = None;
                            state.timeline_selected_key = None;
                            actions.push(EditorAction::AddTimelineTrigger {
                                time,
                                command: state.timeline_trigger_command,
                            });
                        }
                    }
                } else if response.clicked() {
                    state.timeline_selected_trigger = hit;
                    state.timeline_selected_track = None;
                    state.timeline_selected_key = None;
                }

                if response.drag_started() {
                    if let Some(trigger) = hit {
                        state.timeline_selected_trigger = Some(trigger);
                        state.timeline_selected_track = None;
                        state.timeline_selected_key = None;
                        state.timeline_drag_trigger = Some(trigger);
                    }
                }
                if response.dragged() {
                    if let (Some(trigger_index), Some(pointer)) =
                        (state.timeline_drag_trigger, pointer)
                    {
                        if let Some(trigger) = timeline.triggers.get(trigger_index) {
                            let time = timeline_x_time(lane_rect, pointer.x, duration);
                            actions.push(EditorAction::SetTimelineTrigger {
                                trigger: trigger_index,
                                time,
                                command: trigger.command,
                            });
                        }
                    }
                }
                if response.drag_stopped() {
                    state.timeline_drag_trigger = None;
                }
            });

            let selected_marker = state.timeline_selected_marker.is_some();
            ui.horizontal(|ui| {
                let (label_rect, label_response) = ui.allocate_exact_size(
                    Vec2::new(LABEL_WIDTH, LANE_HEIGHT),
                    Sense::click(),
                );
                ui.painter().rect_filled(
                    label_rect,
                    3.0,
                    if selected_marker {
                        Color32::from_rgb(35, 50, 62)
                    } else {
                        Color32::from_rgb(30, 34, 43)
                    },
                );
                ui.painter().text(
                    Pos2::new(label_rect.left() + 7.0, label_rect.center().y),
                    Align2::LEFT_CENTER,
                    "Markers",
                    FontId::proportional(12.0),
                    if selected_marker {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(165, 195, 215)
                    },
                );
                if label_response.clicked() {
                    state.timeline_selected_marker = None;
                    state.timeline_marker_name_id = None;
                    state.timeline_selected_track = None;
                    state.timeline_selected_key = None;
                    state.timeline_selected_trigger = None;
                    state.timeline_drag_marker = None;
                }

                let width = ui.available_width().max(1.0);
                let (lane_rect, response) = ui.allocate_exact_size(
                    Vec2::new(width, LANE_HEIGHT),
                    Sense::click_and_drag(),
                );
                let painter = ui.painter();
                painter.rect_filled(
                    lane_rect,
                    3.0,
                    if selected_marker {
                        Color32::from_rgb(24, 30, 36)
                    } else {
                        Color32::from_rgb(20, 23, 30)
                    },
                );
                let mut tick = 0.0;
                while tick <= duration + 0.0001 {
                    let x = timeline_time_x(lane_rect, tick, duration);
                    painter.line_segment(
                        [
                            Pos2::new(x, lane_rect.top()),
                            Pos2::new(x, lane_rect.bottom()),
                        ],
                        Stroke::new(1.0, Color32::from_rgb(38, 43, 55)),
                    );
                    tick += tick_step;
                }
                let playhead = timeline_time_x(
                    lane_rect,
                    status.transport.position_seconds as f32,
                    duration,
                );
                painter.line_segment(
                    [
                        Pos2::new(playhead, lane_rect.top()),
                        Pos2::new(playhead, lane_rect.bottom()),
                    ],
                    Stroke::new(1.5, Color32::from_rgb(255, 120, 90)),
                );

                let marker_points: Vec<Pos2> = timeline
                    .markers
                    .iter()
                    .map(|marker| {
                        Pos2::new(
                            timeline_time_x(lane_rect, marker.time, duration),
                            lane_rect.center().y,
                        )
                    })
                    .collect();
                for (marker_index, point) in marker_points.iter().enumerate() {
                    let selected = state.timeline_selected_marker == Some(marker_index);
                    painter.circle_filled(
                        *point,
                        if selected { 6.0 } else { 4.5 },
                        if selected {
                            Color32::from_rgb(255, 210, 90)
                        } else {
                            Color32::from_rgb(150, 200, 230)
                        },
                    );
                    painter.circle_stroke(
                        *point,
                        if selected { 6.0 } else { 4.5 },
                        Stroke::new(1.0, Color32::from_rgb(20, 30, 36)),
                    );
                    if let Some(marker) = timeline.markers.get(marker_index) {
                        painter.text(
                            *point + Vec2::new(9.0, 0.0),
                            Align2::LEFT_CENTER,
                            short_label(
                                if marker.name.is_empty() {
                                    "Marker"
                                } else {
                                    marker.name.as_str()
                                },
                                18,
                            ),
                            FontId::proportional(10.0),
                            Color32::from_rgb(165, 200, 220),
                        );
                    }
                }

                let pointer = response.interact_pointer_pos();
                let hit = pointer.and_then(|pointer| {
                    marker_points
                        .iter()
                        .enumerate()
                        .map(|(index, point)| (index, point.distance(pointer)))
                        .filter(|(_, distance)| *distance <= 10.0)
                        .min_by(|left, right| left.1.total_cmp(&right.1))
                        .map(|(index, _)| index)
                });

                if response.secondary_clicked() {
                    if let Some(marker) = hit {
                        actions.push(EditorAction::RemoveTimelineMarker { marker });
                        state.timeline_selected_marker = None;
                        state.timeline_marker_name_id = None;
                        state.timeline_drag_marker = None;
                    }
                } else if response.double_clicked() {
                    if let Some(pointer) = pointer {
                        if hit.is_none() {
                            let time = timeline_x_time(lane_rect, pointer.x, duration);
                            let marker = timeline
                                .markers
                                .iter()
                                .position(|marker| marker.time > time)
                                .unwrap_or(timeline.markers.len());
                            state.timeline_selected_marker = Some(marker);
                            state.timeline_marker_name_id = None;
                            state.timeline_selected_track = None;
                            state.timeline_selected_key = None;
                            state.timeline_selected_trigger = None;
                            actions.push(EditorAction::AddTimelineMarker { time });
                        }
                    }
                } else if response.clicked() {
                    state.timeline_selected_marker = hit;
                    state.timeline_marker_name_id = None;
                    state.timeline_selected_track = None;
                    state.timeline_selected_key = None;
                    state.timeline_selected_trigger = None;
                    if let Some(marker) = hit {
                        if let Some(marker) = timeline.markers.get(marker) {
                            actions.push(EditorAction::SeekTransport {
                                position: f64::from(marker.time),
                            });
                        }
                    }
                }

                if response.drag_started() {
                    if let Some(marker) = hit {
                        state.timeline_selected_marker = Some(marker);
                        state.timeline_marker_name_id = None;
                        state.timeline_selected_track = None;
                        state.timeline_selected_key = None;
                        state.timeline_selected_trigger = None;
                        state.timeline_drag_marker = Some(marker);
                    }
                }
                if response.dragged() {
                    if let (Some(marker_index), Some(pointer)) =
                        (state.timeline_drag_marker, pointer)
                    {
                        if let Some(marker) = timeline.markers.get(marker_index) {
                            let time = timeline_x_time(lane_rect, pointer.x, duration);
                            actions.push(EditorAction::SetTimelineMarker {
                                marker: marker_index,
                                time,
                                name: marker.name.clone(),
                            });
                        }
                    }
                }
                if response.drag_stopped() {
                    state.timeline_drag_marker = None;
                }
            });

            if timeline.tracks.is_empty()
                && timeline.triggers.is_empty()
                && timeline.markers.is_empty()
            {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "No timeline tracks, triggers or markers yet. Choose a parameter or add an item.",
                    )
                    .small()
                    .color(Color32::from_rgb(132, 144, 166)),
                );
            }
        });
}

fn select_timeline_key(
    selected: &mut BTreeSet<usize>,
    primary: &mut Option<usize>,
    key: Option<usize>,
    shift: bool,
) {
    if let Some(key) = key {
        if shift {
            if !selected.insert(key) {
                selected.remove(&key);
            }
            *primary = if selected.contains(&key) {
                Some(key)
            } else {
                selected.iter().next_back().copied()
            };
        } else {
            selected.clear();
            selected.insert(key);
            *primary = Some(key);
        }
    } else {
        *primary = None;
        selected.clear();
    }
}

fn trigger_command_short(command: TimelineCommand) -> &'static str {
    match command {
        TimelineCommand::Pause => "Pause",
        TimelineCommand::Stop => "Stop",
        TimelineCommand::Restart => "Restart",
        TimelineCommand::LoopOn => "Loop+",
        TimelineCommand::LoopOff => "Loop-",
        TimelineCommand::NextMarker => "NextM",
        TimelineCommand::PreviousMarker => "PrevM",
    }
}

fn timeline_tick_step(duration: f32) -> f32 {
    const STEPS: [f32; 12] = [
        0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0,
    ];
    let target = duration / 8.0;
    STEPS
        .iter()
        .copied()
        .find(|step| *step >= target)
        .unwrap_or(600.0)
}

fn timeline_time_x(rect: EguiRect, time: f32, duration: f32) -> f32 {
    rect.left() + (time / duration.max(1.0e-5)).clamp(0.0, 1.0) * rect.width()
}

fn timeline_x_time(rect: EguiRect, x: f32, duration: f32) -> f32 {
    (((x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0) * duration).clamp(0.0, duration)
}

fn timeline_parameter_label(parameters: &[ParameterDescriptor], path: &str) -> String {
    parameters
        .iter()
        .find(|descriptor| descriptor.path() == path)
        .map_or_else(|| path.to_owned(), |descriptor| descriptor.label.clone())
}

fn timeline_parameter_range(parameters: &[ParameterDescriptor], path: &str) -> (f32, f32) {
    let Some(descriptor) = parameters
        .iter()
        .find(|descriptor| descriptor.path() == path)
    else {
        return (0.0, 1.0);
    };
    if descriptor.kind == ParameterKind::Bool {
        return (0.0, 1.0);
    }
    descriptor.range.unwrap_or((0.0, 1.0))
}

fn timeline_candidate(descriptor: &ParameterDescriptor) -> bool {
    let path = descriptor.path();
    match descriptor.kind {
        ParameterKind::Float => !path.starts_with("transport/") && !path.starts_with("audio/"),
        ParameterKind::Bool => {
            (path.starts_with("surface/")
                && (path.ends_with("/visible")
                    || path.ends_with("/mask/enabled")
                    || path.ends_with("/mask/invert")))
                || (path.starts_with("layer/") && path.ends_with("/visible"))
        }
        ParameterKind::Text => false,
    }
}

fn short_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() > max_chars {
        format!(
            "{}…",
            label
                .chars()
                .take(max_chars.saturating_sub(1))
                .collect::<String>()
        )
    } else {
        label.to_owned()
    }
}

fn draw_hierarchy(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    project: &Project,
    status: &EditorStatus,
    stage: &Stage,
    actions: &mut Vec<EditorAction>,
) {
    ui.add_space(8.0);
    ui.heading("Mapping");
    ui.label(
        RichText::new(
            "Media · Layer · Surfaces: choose a layer, set its source, then tick the surfaces it covers.",
        )
        .small()
        .color(Color32::from_rgb(142, 154, 176)),
    );
    ui.separator();

    ui.horizontal(|ui| {
        ui.heading("Surfaces");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button("+ Surface")
                .on_hover_text("Add surface")
                .clicked()
            {
                actions.push(EditorAction::AddSurface);
            }
        });
    });
    let shift = ui.input(|input| input.modifiers.shift);
    for (index, surface) in stage.surfaces.iter().enumerate() {
        let layer_count = stage
            .layers
            .iter()
            .filter(|layer| layer.uses_surface(&surface.id))
            .count();
        let suffix = if layer_count == 1 {
            "1 layer".to_owned()
        } else {
            format!("{layer_count} layers")
        };
        ui.horizontal(|ui| {
            let mut visible = surface.visible;
            if ui.checkbox(&mut visible, "").changed() {
                actions.push(EditorAction::SetSurfaceVisibility {
                    surface: index,
                    visible,
                });
            }
            let label = format!("{} · {suffix}", surface.name);
            let text = if state.selected_surface == index {
                RichText::new(label).strong()
            } else {
                RichText::new(label)
            };
            if ui
                .selectable_label(state.is_surface_selected(index), text)
                .clicked()
            {
                if shift {
                    state.toggle_surface_selection(index);
                } else {
                    state.select_only_surface(index);
                }
            }
        });
        if state.selected_surface == index {
            ui.indent(("surface-editor", index), |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    draw_surface_settings(ui, state, surface, actions);
                    let blocking = stage.layers.iter().any(|layer| {
                        layer.surface_ids.len() <= 1 && layer.uses_surface(&surface.id)
                    });
                    let delete = ui.add_enabled(
                        stage.surfaces.len() > 1 && !blocking,
                        egui::Button::new("Delete surface"),
                    );
                    if delete.clicked() {
                        actions.push(EditorAction::DeleteSurface { surface: index });
                    }
                    if blocking {
                        delete.on_disabled_hover_text(
                            "Reassign layers that only use this surface first",
                        );
                    }
                });
            });
        }
    }
    ui.label(
        RichText::new(format!(
            "{} selected · Shift-click to multi-select",
            state.selected_surfaces.len()
        ))
        .small()
        .color(Color32::from_rgb(132, 144, 166)),
    );
    ui.checkbox(&mut state.snap_corners, "Snap corners");

    let selected_indices: Vec<usize> = state.selected_surfaces.iter().copied().collect();
    if selected_indices.len() >= 2 {
        ui.add_space(6.0);
        ui.label(RichText::new("Align selection").strong());
        ui.horizontal_wrapped(|ui| {
            for (label, align) in [
                ("Left", SurfaceAlign::Left),
                ("Center X", SurfaceAlign::CenterX),
                ("Right", SurfaceAlign::Right),
                ("Top", SurfaceAlign::Top),
                ("Center Y", SurfaceAlign::CenterY),
                ("Bottom", SurfaceAlign::Bottom),
            ] {
                if ui.button(label).clicked() {
                    let surfaces = align_surfaces(&stage.surfaces, &selected_indices, align);
                    if !surfaces.is_empty() {
                        actions.push(EditorAction::SetSurfaceCorners {
                            surfaces,
                            edit: SurfaceGeometryEdit::Align,
                        });
                    }
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    selected_indices.len() >= 3,
                    egui::Button::new("Distribute horizontally"),
                )
                .on_hover_text("Evenly space selected surface centres horizontally")
                .clicked()
            {
                let surfaces = distribute_surfaces(
                    &stage.surfaces,
                    &selected_indices,
                    DistributeAxis::Horizontal,
                );
                if !surfaces.is_empty() {
                    actions.push(EditorAction::SetSurfaceCorners {
                        surfaces,
                        edit: SurfaceGeometryEdit::Distribute,
                    });
                }
            }
            if ui
                .add_enabled(
                    selected_indices.len() >= 3,
                    egui::Button::new("Distribute vertically"),
                )
                .on_hover_text("Evenly space selected surface centres vertically")
                .clicked()
            {
                let surfaces = distribute_surfaces(
                    &stage.surfaces,
                    &selected_indices,
                    DistributeAxis::Vertical,
                );
                if !surfaces.is_empty() {
                    actions.push(EditorAction::SetSurfaceCorners {
                        surfaces,
                        edit: SurfaceGeometryEdit::Distribute,
                    });
                }
            }
        });
    }

    ui.add_space(10.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.heading("Layers");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let new_surfaces: Vec<usize> = state.selected_surfaces.iter().copied().collect();
            if ui
                .add_enabled(
                    !new_surfaces.is_empty(),
                    egui::Button::new("+ Test pattern"),
                )
                .on_hover_text("Add a test pattern layer on the selected surfaces")
                .clicked()
            {
                actions.push(EditorAction::AddLayer {
                    surfaces: new_surfaces,
                });
                state.selected_layer = stage.layers.len();
            }
        });
    });
    if stage.layers.is_empty() {
        ui.label("No layers yet. Add a test pattern or choose media from the shelf.");
    }
    for (index, layer) in stage.layers.iter().enumerate() {
        let surface_indices = layer_surface_indices(stage, layer);
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
                    format!("{} · {}", layer.name, layer_surface_label(stage, layer)),
                )
                .on_hover_text(layer_source(layer, project));
            if response.clicked() {
                state.selected_layer = index;
                if !surface_indices.contains(&state.selected_surface) {
                    if let Some(first) = surface_indices.first() {
                        state.select_only_surface(*first);
                    }
                }
            }
        });
        if state.selected_layer == index {
            ui.indent(("layer-editor", index), |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    draw_layer_editor(ui, state, project, status, stage, layer, actions);
                });
            });
        }
    }

    ui.add_space(12.0);
    ui.separator();
    ui.label(format!("Stage aspect: {:.2}:1", stage.aspect));
    ui.label(
        RichText::new("Click a surface to edit it · drag a selected surface to move")
            .small()
            .color(Color32::from_rgb(132, 144, 166)),
    );
}

fn draw_layer_editor(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    project: &Project,
    status: &EditorStatus,
    stage: &Stage,
    layer: &lpmt_core::model::Layer,
    actions: &mut Vec<EditorAction>,
) {
    let index = state.selected_layer;
    ui.label("Layer name");
    if ui
        .add(egui::TextEdit::singleline(&mut state.layer_name).desired_width(f32::INFINITY))
        .changed()
    {
        actions.push(EditorAction::RenameLayer {
            layer: index,
            name: state.layer_name.clone(),
        });
    }

    let media_index = layer_media_index(layer, project);
    ui.add_space(4.0);
    ui.label("Source media");
    ui.horizontal(|ui| {
        if let Some(media_index) = media_index {
            if let Some(asset) = project.media.get(media_index) {
                let texture = status.media_thumbnails.get(&asset.id).copied();
                if media_thumbnail(ui, texture, asset.kind, Vec2::new(52.0, 34.0)).clicked() {
                    state.media_picker = Some(index);
                }
                ui.vertical(|ui| {
                    ui.label(&asset.name);
                    ui.label(
                        RichText::new(media_kind_label(asset.kind))
                            .small()
                            .color(Color32::from_rgb(132, 144, 166)),
                    );
                });
            }
        } else {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(52.0, 34.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, 3.0, Color32::from_rgb(26, 30, 38));
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "pattern",
                FontId::proportional(9.0),
                Color32::from_rgb(130, 140, 160),
            );
            ui.vertical(|ui| {
                ui.label("Test pattern");
                ui.label(
                    RichText::new(layer_source(layer, project))
                        .small()
                        .color(Color32::from_rgb(132, 144, 166)),
                );
            });
        }
        if ui.button("Choose media…").clicked() {
            state.media_picker = Some(index);
        }
        if let Some(media_index) = media_index {
            if ui.button("Show in shelf").clicked() {
                state.selected_media = media_index;
                if let Some(asset) = project.media.get(media_index) {
                    state.image_path.clone_from(&asset.path);
                }
            }
        }
    });

    ui.add_space(4.0);
    ui.label("Draw through surfaces");
    let assigned_count = layer_surface_indices(stage, layer).len();
    for (surface_index, surface) in stage.surfaces.iter().enumerate() {
        let mut assigned = layer.uses_surface(&surface.id);
        let is_last = assigned && assigned_count <= 1;
        let response = ui.add_enabled(!is_last, egui::Checkbox::new(&mut assigned, &surface.name));
        if response.changed() {
            let mut ids = layer_surface_indices(stage, layer);
            if assigned {
                if !ids.contains(&surface_index) {
                    ids.push(surface_index);
                }
            } else {
                ids.retain(|assigned| *assigned != surface_index);
            }
            ids.sort_unstable();
            actions.push(EditorAction::SetLayerSurfaces {
                layer: index,
                surfaces: ids,
            });
        }
        if is_last {
            response.on_disabled_hover_text("A layer must keep at least one surface");
        }
    }
    if assigned_count > 1 {
        ui.label(
            RichText::new("This source is drawn through each checked surface.")
                .small()
                .color(Color32::from_rgb(132, 144, 166)),
        );
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
            layer: index,
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
            layer: index,
            blend,
        });
    }
    if let SourceSpec::TestPattern { pattern } = &layer.source {
        let mut chosen = *pattern;
        egui::ComboBox::from_label("Test pattern")
            .selected_text(chosen.label())
            .show_ui(ui, |ui| {
                for candidate in TestPattern::ALL {
                    ui.selectable_value(&mut chosen, candidate, candidate.label());
                }
            });
        if chosen != *pattern {
            actions.push(EditorAction::SetLayerTestPattern {
                layer: index,
                pattern: chosen,
            });
        }
    }

    ui.add_space(4.0);
    ui.separator();
    ui.label(RichText::new("Source crop").strong());
    let mut crop = layer.crop;
    let mut crop_changed = false;
    egui::Grid::new(("layer-crop", index))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for (label, value) in [
                ("Top", &mut crop.top),
                ("Right", &mut crop.right),
                ("Bottom", &mut crop.bottom),
                ("Left", &mut crop.left),
            ] {
                ui.label(label);
                if ui
                    .add(egui::Slider::new(value, 0.0..=0.45).fixed_decimals(3))
                    .changed()
                {
                    crop_changed = true;
                }
                ui.end_row();
            }
        });

    let mut circular = crop.circle.is_some();
    if ui.checkbox(&mut circular, "Circular crop").changed() {
        crop.circle = circular.then_some(CircleCrop {
            center: Point2::new(0.5, 0.5),
            radius: 0.5,
        });
        crop_changed = true;
    }
    if let Some(circle) = &mut crop.circle {
        let mut center = circle.center;
        let mut radius = circle.radius;
        ui.horizontal(|ui| {
            ui.label("Centre");
            if ui
                .add(
                    egui::DragValue::new(&mut center.x)
                        .speed(0.002)
                        .range(0.0..=1.0),
                )
                .changed()
            {
                crop_changed = true;
            }
            if ui
                .add(
                    egui::DragValue::new(&mut center.y)
                        .speed(0.002)
                        .range(0.0..=1.0),
                )
                .changed()
            {
                crop_changed = true;
            }
        });
        if ui
            .add(
                egui::Slider::new(&mut radius, 0.0..=1.0)
                    .text("Radius")
                    .fixed_decimals(3),
            )
            .changed()
        {
            crop_changed = true;
        }
        circle.center = center;
        circle.radius = radius;
    }
    if ui.button("Clear crop").clicked() {
        crop = Crop::default();
        crop_changed = true;
    }
    if crop_changed {
        actions.push(EditorAction::SetLayerCrop { layer: index, crop });
    }

    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(index > 0, egui::Button::new("Lower"))
            .clicked()
        {
            actions.push(EditorAction::ReorderLayer {
                layer: index,
                offset: -1,
            });
            state.selected_layer = index - 1;
        }
        if ui
            .add_enabled(index + 1 < stage.layers.len(), egui::Button::new("Raise"))
            .clicked()
        {
            actions.push(EditorAction::ReorderLayer {
                layer: index,
                offset: 1,
            });
            state.selected_layer = index + 1;
        }
        if ui
            .add_enabled(stage.layers.len() > 1, egui::Button::new("Delete layer"))
            .clicked()
        {
            actions.push(EditorAction::DeleteLayer { layer: index });
            state.selected_layer = index.min(stage.layers.len().saturating_sub(2));
        }
    });
}

fn draw_media_picker(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    project: &Project,
    status: &EditorStatus,
    stage: &Stage,
    actions: &mut Vec<EditorAction>,
) {
    let Some(target_layer) = state.media_picker else {
        return;
    };
    let Some(layer) = stage.layers.get(target_layer) else {
        state.media_picker = None;
        return;
    };

    let mut open = true;
    let mut close = false;
    egui::Window::new("Choose media")
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .default_height(460.0)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            ui.heading(format!("Choose media for {}", layer.name));
            ui.label(format!("Drawn on: {}", layer_surface_label(stage, layer)));
            ui.label(
                RichText::new("Images and videos for this layer; audio is assigned in Show.")
                    .small()
                    .color(Color32::from_rgb(140, 152, 174)),
            );
            ui.separator();

            let visual_media: Vec<(usize, &MediaAsset)> = project
                .media
                .iter()
                .enumerate()
                .filter(|(_, asset)| asset.kind != MediaKind::Audio)
                .collect();
            if visual_media.is_empty() {
                ui.label("No visual media yet. Add an image or video in the Media tab.");
            } else {
                let current = layer_media_index(layer, project);
                egui::ScrollArea::vertical()
                    .max_height(330.0)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (index, asset) in visual_media {
                                let selected = current == Some(index);
                                let texture = status.media_thumbnails.get(&asset.id).copied();
                                let response = media_card(ui, asset, texture, selected);
                                if response.clicked() {
                                    actions.push(EditorAction::SetLayerMedia {
                                        layer: target_layer,
                                        media: index,
                                    });
                                    state.selected_media = index;
                                    state.image_path.clone_from(&asset.path);
                                    close = true;
                                }
                            }
                        });
                    });
            }

            ui.add_space(6.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    close = true;
                }
                ui.label(
                    RichText::new("Click a card to assign it to this layer.")
                        .small()
                        .color(Color32::from_rgb(140, 152, 174)),
                );
            });
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                close = true;
            }
        });

    if close || !open {
        state.media_picker = None;
    }
}

fn media_thumbnail(
    ui: &mut egui::Ui,
    texture: Option<egui::TextureId>,
    kind: MediaKind,
    size: Vec2,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    if let Some(texture) = texture {
        painter.image(
            texture,
            rect,
            EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.rect_filled(rect, 3.0, Color32::from_rgb(14, 17, 23));
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            media_kind_label(kind),
            FontId::proportional(10.0),
            Color32::from_rgb(120, 132, 152),
        );
    }
    response
}

fn media_card(
    ui: &mut egui::Ui,
    asset: &MediaAsset,
    texture: Option<egui::TextureId>,
    selected: bool,
) -> egui::Response {
    let size = Vec2::new(160.0, 126.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    let background = if selected {
        Color32::from_rgb(38, 58, 72)
    } else if response.hovered() {
        Color32::from_rgb(34, 40, 50)
    } else {
        Color32::from_rgb(24, 28, 36)
    };
    painter.rect_filled(rect, 6.0, background);

    let image_rect = EguiRect::from_min_size(
        rect.min + Vec2::new(6.0, 6.0),
        Vec2::new(size.x - 12.0, 88.0),
    );
    if let Some(texture) = texture {
        painter.image(
            texture,
            image_rect,
            EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.rect_filled(image_rect, 4.0, Color32::from_rgb(14, 17, 23));
        painter.text(
            image_rect.center(),
            Align2::CENTER_CENTER,
            media_kind_label(asset.kind),
            FontId::proportional(12.0),
            Color32::from_rgb(130, 140, 160),
        );
    }

    let name = if asset.name.chars().count() > 20 {
        format!("{}…", asset.name.chars().take(19).collect::<String>())
    } else {
        asset.name.clone()
    };
    painter.text(
        Pos2::new(rect.left() + 8.0, image_rect.bottom() + 7.0),
        Align2::LEFT_TOP,
        name,
        FontId::proportional(13.0),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(rect.left() + 8.0, image_rect.bottom() + 26.0),
        Align2::LEFT_TOP,
        media_kind_label(asset.kind),
        FontId::proportional(10.0),
        Color32::from_rgb(140, 150, 170),
    );
    if selected {
        painter.text(
            Pos2::new(rect.right() - 8.0, image_rect.bottom() + 26.0),
            Align2::RIGHT_TOP,
            "current",
            FontId::proportional(10.0),
            Color32::from_rgb(120, 220, 140),
        );
    }
    response
}

fn draw_surface_settings(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    surface: &Surface,
    actions: &mut Vec<EditorAction>,
) {
    ui.label("Surface name");
    if ui
        .add(egui::TextEdit::singleline(&mut state.surface_name).desired_width(f32::INFINITY))
        .changed()
    {
        actions.push(EditorAction::RenameSurface {
            surface: state.selected_surface,
            name: state.surface_name.clone(),
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

    ui.add_space(4.0);
    ui.separator();
    ui.label(RichText::new("Surface mask").strong());
    let mut mask_enabled = surface.mask.enabled;
    if ui
        .checkbox(&mut mask_enabled, "Enable polygon mask")
        .changed()
    {
        actions.push(EditorAction::SetSurfaceMaskEnabled {
            surface: state.selected_surface,
            enabled: mask_enabled,
        });
    }
    if !mask_enabled {
        state.mask_add_mode = false;
    }
    if mask_enabled {
        let mut invert = surface.mask.invert;
        if ui.checkbox(&mut invert, "Invert mask").changed() {
            actions.push(EditorAction::SetSurfaceMaskInvert {
                surface: state.selected_surface,
                invert,
            });
        }
        let mut feather = surface.mask.feather;
        if ui
            .add(
                egui::Slider::new(&mut feather, 0.0..=0.5)
                    .text("Feather")
                    .fixed_decimals(3),
            )
            .changed()
        {
            actions.push(EditorAction::SetSurfaceMaskFeather {
                surface: state.selected_surface,
                feather,
            });
        }
        let mut add_points = state.mask_add_mode;
        if ui
            .checkbox(&mut add_points, "Add points in preview")
            .changed()
        {
            state.mask_add_mode = add_points;
        }
        let hint = if state.mask_add_mode {
            format!(
                "{} point(s) · Click the preview to add a point · right-click a point to remove",
                surface.mask.points.len()
            )
        } else {
            format!(
                "{} point(s) · Ctrl-click or enable Add points · right-click a point to remove",
                surface.mask.points.len()
            )
        };
        ui.label(
            RichText::new(hint)
                .small()
                .color(Color32::from_rgb(132, 144, 166)),
        );
        ui.horizontal_wrapped(|ui| {
            if ui.button("Rectangle mask").clicked() {
                actions.push(EditorAction::ResetSurfaceMask {
                    surface: state.selected_surface,
                });
            }
            if ui.button("Clear mask").clicked() {
                state.mask_add_mode = false;
                actions.push(EditorAction::ClearSurfaceMask {
                    surface: state.selected_surface,
                });
            }
        });
    }

    ui.add_space(4.0);
    let mut grid_enabled = surface.grid.is_some() && surface.grid_enabled;
    if ui.checkbox(&mut grid_enabled, "Grid warp").changed() {
        actions.push(EditorAction::SetSurfaceGridEnabled {
            surface: state.selected_surface,
            enabled: grid_enabled,
        });
    }

    if grid_enabled {
        if let Some(grid) = &surface.grid {
            let mut chosen = (grid.rows, grid.columns);
            egui::ComboBox::from_label("Grid")
                .selected_text(format!("{} × {} cells", chosen.0, chosen.1))
                .show_ui(ui, |ui| {
                    for size in [2_u16, 3, 4, 5, 6] {
                        ui.selectable_value(
                            &mut chosen,
                            (size, size),
                            format!("{size} × {size} cells"),
                        );
                    }
                });
            if chosen != (grid.rows, grid.columns) {
                actions.push(EditorAction::SetSurfaceGridResolution {
                    surface: state.selected_surface,
                    rows: chosen.0,
                    columns: chosen.1,
                });
            }
            ui.label("Drag grid points to deform; drag corners to move the whole surface.");
        }
    } else if surface.grid.is_some() {
        ui.label("Grid warp off. The stored deformation will return when re-enabled.");
    }

    ui.label("Corners adjust the whole surface and its grid together.");
    egui::CollapsingHeader::new("Corners (numeric)")
        .default_open(false)
        .show(ui, |ui| {
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
        });

    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        if surface.grid.is_some() && ui.button("Reset grid").clicked() {
            actions.push(EditorAction::ResetSurfaceGrid {
                surface: state.selected_surface,
            });
        }
        if ui.button("Reset to full stage").clicked() {
            actions.push(EditorAction::ResetSurface {
                surface: state.selected_surface,
            });
        }
    });
}

fn midi_modes(kind: Option<ParameterKind>) -> &'static [MidiMode] {
    const BOOL_MODES: [MidiMode; 2] = [MidiMode::Toggle, MidiMode::Absolute];
    const VALUE_MODES: [MidiMode; 2] = [MidiMode::Absolute, MidiMode::Relative];
    match kind {
        Some(ParameterKind::Bool) => &BOOL_MODES,
        _ => &VALUE_MODES,
    }
}

enum HotkeyCapture {
    Cancel,
    Key {
        key: String,
        ctrl: bool,
        shift: bool,
        alt: bool,
    },
}

fn capture_hotkey(ui: &egui::Ui) -> Option<HotkeyCapture> {
    let mut captured = None;
    ui.input(|input| {
        for event in &input.events {
            let egui::Event::Key {
                key,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            if *repeat {
                continue;
            }
            if *key == egui::Key::Escape {
                captured = Some(HotkeyCapture::Cancel);
                break;
            }
            if is_modifier_key(*key) {
                continue;
            }
            captured = Some(HotkeyCapture::Key {
                key: format!("{key:?}").to_ascii_lowercase(),
                ctrl: modifiers.ctrl || modifiers.command,
                shift: modifiers.shift,
                alt: modifiers.alt,
            });
            break;
        }
    });
    captured
}

fn hotkey_binding_pressed(ui: &egui::Ui, binding: &ParameterBinding) -> bool {
    let mut pressed = false;
    ui.input(|input| {
        for event in &input.events {
            let egui::Event::Key {
                key,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            if *repeat || is_modifier_key(*key) {
                continue;
            }
            let name = format!("{key:?}").to_ascii_lowercase();
            if binding.matches_key(
                &name,
                modifiers.ctrl || modifiers.command,
                modifiers.shift,
                modifiers.alt,
            ) {
                pressed = true;
                break;
            }
        }
    });
    pressed
}

fn is_modifier_key(key: egui::Key) -> bool {
    matches!(
        key,
        egui::Key::ShiftLeft
            | egui::Key::ShiftRight
            | egui::Key::ControlLeft
            | egui::Key::ControlRight
            | egui::Key::AltLeft
            | egui::Key::AltRight
            | egui::Key::SuperLeft
            | egui::Key::SuperRight
    )
}

fn layer_source(layer: &lpmt_core::model::Layer, project: &Project) -> String {
    match &layer.source {
        SourceSpec::Image { media_id } | SourceSpec::Video { media_id } => project
            .media_asset(media_id)
            .map_or_else(|| "missing media".to_owned(), |asset| asset.name.clone()),
        SourceSpec::TestPattern { pattern } => format!("test pattern · {}", pattern.label()),
    }
}

fn layer_surface_indices(stage: &Stage, layer: &lpmt_core::model::Layer) -> Vec<usize> {
    let mut indices = Vec::new();
    for surface_id in &layer.surface_ids {
        if let Some(index) = stage
            .surfaces
            .iter()
            .position(|surface| &surface.id == surface_id)
        {
            if !indices.contains(&index) {
                indices.push(index);
            }
        }
    }
    indices
}

fn layer_surface_label(stage: &Stage, layer: &lpmt_core::model::Layer) -> String {
    let names: Vec<&str> = layer
        .surface_ids
        .iter()
        .filter_map(|id| stage.surface(id))
        .map(|surface| surface.name.as_str())
        .collect();
    match names.len() {
        0 => "no surface".to_owned(),
        1 => names[0].to_owned(),
        2 => format!("{} + {}", names[0], names[1]),
        count => format!("{count} surfaces"),
    }
}

fn layer_media_index(layer: &lpmt_core::model::Layer, project: &Project) -> Option<usize> {
    let media_id = match &layer.source {
        SourceSpec::Image { media_id } | SourceSpec::Video { media_id } => media_id,
        SourceSpec::TestPattern { .. } => return None,
    };
    project.media.iter().position(|asset| &asset.id == media_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceAlign {
    Left,
    CenterX,
    Right,
    Top,
    CenterY,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DistributeAxis {
    Horizontal,
    Vertical,
}

fn surface_bounds(surface: &Surface) -> (f32, f32, f32, f32) {
    surface.corners.iter().fold(
        (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ),
        |(left, top, right, bottom), corner| {
            (
                left.min(corner.x),
                top.min(corner.y),
                right.max(corner.x),
                bottom.max(corner.y),
            )
        },
    )
}

fn selection_bounds(surfaces: &[Surface], indices: &[usize]) -> Option<(f32, f32, f32, f32)> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for index in indices {
        let Some(surface) = surfaces.get(*index) else {
            continue;
        };
        let (left, top, right, bottom) = surface_bounds(surface);
        bounds = Some(match bounds {
            Some((current_left, current_top, current_right, current_bottom)) => (
                current_left.min(left),
                current_top.min(top),
                current_right.max(right),
                current_bottom.max(bottom),
            ),
            None => (left, top, right, bottom),
        });
    }
    bounds
}

fn translated_corners(corners: [Point2; 4], dx: f32, dy: f32) -> [Point2; 4] {
    corners.map(|corner| Point2::new(corner.x + dx, corner.y + dy))
}

/// Keep a whole-surface translation inside the normalized stage. Returning a
/// clamped delta lets both dragging and one-shot layout edits share one path.
fn clamp_translation(originals: &[(usize, [Point2; 4])], dx: f32, dy: f32) -> (f32, f32) {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (_, corners) in originals {
        for corner in corners {
            min_x = min_x.min(corner.x);
            min_y = min_y.min(corner.y);
            max_x = max_x.max(corner.x);
            max_y = max_y.max(corner.y);
        }
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return (0.0, 0.0);
    }

    let min_dx = -min_x;
    let max_dx = 1.0 - max_x;
    let min_dy = -min_y;
    let max_dy = 1.0 - max_y;
    (
        if min_dx <= max_dx {
            dx.clamp(min_dx, max_dx)
        } else {
            0.0
        },
        if min_dy <= max_dy {
            dy.clamp(min_dy, max_dy)
        } else {
            0.0
        },
    )
}

fn align_surfaces(
    surfaces: &[Surface],
    indices: &[usize],
    align: SurfaceAlign,
) -> Vec<(usize, [Point2; 4])> {
    let Some((left, top, right, bottom)) = selection_bounds(surfaces, indices) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for index in indices {
        let Some(surface) = surfaces.get(*index) else {
            continue;
        };
        let (surface_left, surface_top, surface_right, surface_bottom) = surface_bounds(surface);
        let (dx, dy) = match align {
            SurfaceAlign::Left => (left - surface_left, 0.0),
            SurfaceAlign::CenterX => (
                (left + right) * 0.5 - (surface_left + surface_right) * 0.5,
                0.0,
            ),
            SurfaceAlign::Right => (right - surface_right, 0.0),
            SurfaceAlign::Top => (0.0, top - surface_top),
            SurfaceAlign::CenterY => (
                0.0,
                (top + bottom) * 0.5 - (surface_top + surface_bottom) * 0.5,
            ),
            SurfaceAlign::Bottom => (0.0, bottom - surface_bottom),
        };
        let (dx, dy) = clamp_translation(&[(*index, surface.corners)], dx, dy);
        if dx.abs() > f32::EPSILON || dy.abs() > f32::EPSILON {
            result.push((*index, translated_corners(surface.corners, dx, dy)));
        }
    }
    result
}

fn distribute_surfaces(
    surfaces: &[Surface],
    indices: &[usize],
    axis: DistributeAxis,
) -> Vec<(usize, [Point2; 4])> {
    let mut ordered: Vec<(usize, f32)> = indices
        .iter()
        .filter_map(|index| {
            surfaces.get(*index).map(|surface| {
                let (left, top, right, bottom) = surface_bounds(surface);
                let center = match axis {
                    DistributeAxis::Horizontal => (left + right) * 0.5,
                    DistributeAxis::Vertical => (top + bottom) * 0.5,
                };
                (*index, center)
            })
        })
        .collect();
    ordered.sort_by(|left, right| left.1.total_cmp(&right.1));
    if ordered.len() < 3 {
        return Vec::new();
    }

    let first = ordered[0].1;
    let last = ordered[ordered.len() - 1].1;
    let step = (last - first) / (ordered.len() - 1) as f32;
    let mut result = Vec::new();
    for (rank, (index, _)) in ordered.iter().enumerate().skip(1).take(ordered.len() - 2) {
        let Some(surface) = surfaces.get(*index) else {
            continue;
        };
        let (left, top, right, bottom) = surface_bounds(surface);
        let target = first + step * rank as f32;
        let (dx, dy) = match axis {
            DistributeAxis::Horizontal => (target - (left + right) * 0.5, 0.0),
            DistributeAxis::Vertical => (0.0, target - (top + bottom) * 0.5),
        };
        let (dx, dy) = clamp_translation(&[(*index, surface.corners)], dx, dy);
        if dx.abs() > f32::EPSILON || dy.abs() > f32::EPSILON {
            result.push((*index, translated_corners(surface.corners, dx, dy)));
        }
    }
    result
}

fn media_kind_label(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "image",
        MediaKind::Video => "video",
        MediaKind::Audio => "audio",
    }
}

fn media_usage(project: &Project, asset: &MediaAsset) -> String {
    let mut roles = Vec::new();
    if project.audio.media_id.as_ref() == Some(&asset.id) {
        roles.push("show audio".to_owned());
    }
    for stage in &project.stages {
        for layer in &stage.layers {
            let uses_asset = matches!(
                &layer.source,
                SourceSpec::Image { media_id } | SourceSpec::Video { media_id }
                    if media_id == &asset.id
            );
            if uses_asset {
                roles.push(format!(
                    "layer {} on {}",
                    layer.name,
                    layer_surface_label(stage, layer)
                ));
            }
        }
    }
    if roles.is_empty() {
        "unused".to_owned()
    } else {
        roles.join(" · ")
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

fn surface_points(preview: EguiRect, surface: &Surface) -> Vec<Pos2> {
    surface
        .corners
        .iter()
        .map(|corner| stage_to_preview(preview, *corner))
        .collect()
}

fn lerp_point(a: Point2, b: Point2, t: f32) -> Point2 {
    Point2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// Map surface UV to stage space using the same bilinear grid interpolation as
/// the renderer, falling back to the four-corner homography.
fn surface_uv_to_stage(surface: &Surface, uv: Point2) -> Option<Point2> {
    let uv = uv.clamp_unit();
    if let Some(grid) = surface
        .grid
        .as_ref()
        .filter(|grid| surface.grid_enabled && grid.is_valid())
    {
        let columns = f32::from(grid.columns);
        let rows = f32::from(grid.rows);
        let cell_x = (uv.x * columns).clamp(0.0, columns - 1e-5);
        let cell_y = (uv.y * rows).clamp(0.0, rows - 1e-5);
        let column = cell_x.floor() as u16;
        let row = cell_y.floor() as u16;
        let local_x = cell_x - f32::from(column);
        let local_y = cell_y - f32::from(row);
        let p00 = grid.point(row, column)?;
        let p10 = grid.point(row, column + 1)?;
        let p11 = grid.point(row + 1, column + 1)?;
        let p01 = grid.point(row + 1, column)?;
        let top = lerp_point(p00, p10, local_x);
        let bottom = lerp_point(p01, p11, local_x);
        return Some(lerp_point(top, bottom, local_y));
    }
    surface.homography().ok()?.try_transform(uv)
}

/// Inverse of [`surface_uv_to_stage`] for mask editing. The homography gives
/// the initial estimate; an extra bilinear Newton pass keeps dragging accurate
/// when grid warp is enabled.
fn stage_to_surface_uv(surface: &Surface, stage_point: Point2) -> Option<Point2> {
    let to_surface = surface.homography().ok()?.inverse().ok()?;
    let mut uv = to_surface.try_transform(stage_point)?.clamp_unit();
    if surface
        .grid
        .as_ref()
        .is_some_and(|grid| surface.grid_enabled && grid.is_valid())
    {
        for _ in 0..6 {
            let current = surface_uv_to_stage(surface, uv)?;
            let error = Point2::new(stage_point.x - current.x, stage_point.y - current.y);
            if error.x.abs() + error.y.abs() < 1e-6 {
                break;
            }
            let step = 0.001;
            let (u_probe, u_step) = if uv.x + step <= 1.0 {
                (Point2::new(uv.x + step, uv.y), step)
            } else {
                (Point2::new(uv.x - step, uv.y), -step)
            };
            let (v_probe, v_step) = if uv.y + step <= 1.0 {
                (Point2::new(uv.x, uv.y + step), step)
            } else {
                (Point2::new(uv.x, uv.y - step), -step)
            };
            let u_point = surface_uv_to_stage(surface, u_probe)?;
            let v_point = surface_uv_to_stage(surface, v_probe)?;
            let j00 = (u_point.x - current.x) / u_step;
            let j10 = (u_point.y - current.y) / u_step;
            let j01 = (v_point.x - current.x) / v_step;
            let j11 = (v_point.y - current.y) / v_step;
            let determinant = j00 * j11 - j01 * j10;
            if determinant.abs() < 1e-6 {
                break;
            }
            uv.x = (uv.x + (j11 * error.x - j01 * error.y) / determinant).clamp(0.0, 1.0);
            uv.y = (uv.y + (-j10 * error.x + j00 * error.y) / determinant).clamp(0.0, 1.0);
        }
    }
    Some(uv)
}

fn nearest_mask_point(
    preview: EguiRect,
    surface: &Surface,
    pointer: Pos2,
    threshold: f32,
) -> Option<usize> {
    surface
        .mask
        .points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| {
            let screen = stage_to_preview(preview, surface_uv_to_stage(surface, *point)?);
            let distance = screen.distance(pointer);
            (distance <= threshold).then_some((index, distance))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(index, _)| index)
}

fn nearest_mask_segment(preview: EguiRect, surface: &Surface, pointer: Pos2) -> Option<usize> {
    let count = surface.mask.points.len();
    if count < 2 {
        return None;
    }
    let mut best: Option<(usize, f32)> = None;
    for segment in 0..count {
        let Some(a) = surface_uv_to_stage(surface, surface.mask.points[segment]) else {
            continue;
        };
        let Some(b) = surface_uv_to_stage(surface, surface.mask.points[(segment + 1) % count])
        else {
            continue;
        };
        let distance = distance_to_segment(
            pointer,
            stage_to_preview(preview, a),
            stage_to_preview(preview, b),
        );
        if best.is_none_or(|(_, best_distance)| distance < best_distance) {
            best = Some((segment, distance));
        }
    }
    best.map(|(segment, _)| segment)
}

fn distance_to_segment(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let length_squared = ab.x * ab.x + ab.y * ab.y;
    if length_squared <= f32::EPSILON {
        return point.distance(a);
    }
    let t = (((point.x - a.x) * ab.x + (point.y - a.y) * ab.y) / length_squared).clamp(0.0, 1.0);
    Pos2::new(a.x + ab.x * t, a.y + ab.y * t).distance(point)
}

fn polygon_center(points: &[Pos2]) -> Pos2 {
    if points.is_empty() {
        return Pos2::ZERO;
    }
    let sum = points
        .iter()
        .fold(Vec2::ZERO, |sum, point| sum + point.to_vec2());
    (sum / points.len() as f32).to_pos2()
}

fn surface_at(preview: EguiRect, surfaces: &[Surface], pointer: Pos2) -> Option<usize> {
    surfaces
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, surface)| {
            let points = surface_points(preview, surface);
            point_in_polygon(pointer, &points).then_some(index)
        })
}

fn nearest_grid_point(
    preview: EguiRect,
    grid: &WarpGrid,
    pointer: Pos2,
    threshold: f32,
) -> Option<(u16, u16)> {
    let mut best: Option<(f32, (u16, u16))> = None;
    for row in 0..=grid.rows {
        for column in 0..=grid.columns {
            let Some(point) = grid.point(row, column) else {
                continue;
            };
            let distance = stage_to_preview(preview, point).distance(pointer);
            if distance <= threshold
                && best.is_none_or(|(best_distance, _)| distance < best_distance)
            {
                best = Some((distance, (row, column)));
            }
        }
    }
    best.map(|(_, point)| point)
}

fn nearest_surface_corner(
    preview: EguiRect,
    surfaces: &[Surface],
    selected: usize,
    pointer: Pos2,
    threshold: f32,
) -> Option<(usize, usize)> {
    let closest = |index: usize| {
        let points = surface_points(preview, &surfaces[index]);
        nearest_corner(&points, pointer, threshold).map(|corner| (index, corner))
    };
    surfaces
        .get(selected)
        .and_then(|_| closest(selected))
        .or_else(|| {
            (0..surfaces.len())
                .filter(|index| *index != selected)
                .filter_map(closest)
                .min_by(|left, right| {
                    let left_distance =
                        surface_points(preview, &surfaces[left.0])[left.1].distance(pointer);
                    let right_distance =
                        surface_points(preview, &surfaces[right.0])[right.1].distance(pointer);
                    left_distance.total_cmp(&right_distance)
                })
        })
}

fn point_in_polygon(point: Pos2, polygon: &[Pos2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let a = polygon[current];
        let b = polygon[previous];
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn point_in_surface(surface: &Surface, point: Point2) -> bool {
    let polygon: Vec<Pos2> = surface
        .corners
        .iter()
        .map(|corner| Pos2::new(corner.x, corner.y))
        .collect();
    point_in_polygon(Pos2::new(point.x, point.y), &polygon)
}

fn nearest_surface_snap(
    preview: EguiRect,
    surfaces: &[Surface],
    exclude: usize,
    pointer: Pos2,
    threshold: f32,
) -> Option<(Point2, Pos2)> {
    let mut best: Option<(f32, Point2, Pos2)> = None;
    for (index, surface) in surfaces.iter().enumerate() {
        if index == exclude {
            continue;
        }
        for corner in &surface.corners {
            let screen = stage_to_preview(preview, *corner);
            let distance = screen.distance(pointer);
            if distance <= threshold
                && best.is_none_or(|(best_distance, ..)| distance < best_distance)
            {
                best = Some((distance, *corner, screen));
            }
        }
    }
    best.map(|(_, corner, screen)| (corner, screen))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_candidates_include_bool_state_but_not_transport_or_output_windows() {
        let project = Project::demo();
        let parameters = project_parameters(&project);
        let find = |path: &str| {
            parameters
                .iter()
                .find(|descriptor| descriptor.path() == path)
                .unwrap()
        };
        assert!(timeline_candidate(find("surface/surface-1/opacity")));
        assert!(timeline_candidate(find("surface/surface-1/visible")));
        assert!(timeline_candidate(find("layer/layer-1/visible")));
        assert!(!timeline_candidate(find("output/output-1/enabled")));
        assert!(!timeline_candidate(find("transport/playing")));
    }

    #[test]
    fn timeline_key_selection_extends_and_removes_with_shift() {
        let mut selected = BTreeSet::new();
        let mut primary = None;
        select_timeline_key(&mut selected, &mut primary, Some(1), false);
        assert_eq!(selected, BTreeSet::from([1]));
        assert_eq!(primary, Some(1));
        select_timeline_key(&mut selected, &mut primary, Some(3), true);
        assert_eq!(selected, BTreeSet::from([1, 3]));
        assert_eq!(primary, Some(3));
        select_timeline_key(&mut selected, &mut primary, Some(3), true);
        assert_eq!(selected, BTreeSet::from([1]));
        assert_eq!(primary, Some(1));
    }

    #[test]
    fn midi_modes_match_the_parameter_kind() {
        assert_eq!(
            midi_modes(Some(ParameterKind::Bool)),
            &[MidiMode::Toggle, MidiMode::Absolute][..]
        );
        assert_eq!(
            midi_modes(Some(ParameterKind::Float)),
            &[MidiMode::Absolute, MidiMode::Relative][..]
        );
        assert_eq!(
            midi_modes(Some(ParameterKind::Text)),
            &[MidiMode::Absolute, MidiMode::Relative][..]
        );
    }

    #[test]
    fn nearest_mask_segment_follows_polygon_order() {
        let mut surface = Surface::full_stage("surface", "Surface");
        surface.mask.points = vec![
            Point2::new(0.1, 0.1),
            Point2::new(0.9, 0.1),
            Point2::new(0.9, 0.9),
            Point2::new(0.1, 0.9),
        ];
        let preview = EguiRect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));
        assert_eq!(
            nearest_mask_segment(preview, &surface, Pos2::new(50.0, 10.0)),
            Some(0)
        );
        assert_eq!(
            nearest_mask_segment(preview, &surface, Pos2::new(90.0, 50.0)),
            Some(1)
        );
        assert_eq!(
            nearest_mask_segment(preview, &surface, Pos2::new(50.0, 90.0)),
            Some(2)
        );
        assert_eq!(
            nearest_mask_segment(preview, &surface, Pos2::new(10.0, 50.0)),
            Some(3)
        );
    }

    #[test]
    fn point_in_surface_respects_the_quad() {
        let surface = Surface::full_stage("surface", "Surface");
        assert!(point_in_surface(&surface, Point2::new(0.5, 0.5)));
        assert!(!point_in_surface(&surface, Point2::new(1.2, 0.5)));
        assert!(!point_in_surface(&surface, Point2::new(-0.1, 0.5)));
    }

    #[test]
    fn point_in_polygon_handles_inside_and_outside_points() {
        let square = [
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(Pos2::new(5.0, 5.0), &square));
        assert!(!point_in_polygon(Pos2::new(15.0, 5.0), &square));
        assert!(!point_in_polygon(Pos2::new(-1.0, 5.0), &square));
    }

    #[test]
    fn surface_uv_round_trips_through_grid_warp() {
        let mut surface = Surface::full_stage("surface", "Surface");
        surface.set_grid_enabled(true);
        surface.set_grid_resolution(2, 2);
        assert!(surface.set_grid_point(1, 1, Point2::new(0.55, 0.58)));

        for uv in [
            Point2::new(0.15, 0.2),
            Point2::new(0.5, 0.5),
            Point2::new(0.85, 0.8),
        ] {
            let stage = surface_uv_to_stage(&surface, uv).expect("uv maps to stage");
            let round_trip = stage_to_surface_uv(&surface, stage).expect("stage maps back");
            assert!((round_trip.x - uv.x).abs() < 1e-3, "{round_trip:?}");
            assert!((round_trip.y - uv.y).abs() < 1e-3, "{round_trip:?}");
        }
    }

    #[test]
    fn snapping_uses_the_nearest_corner_of_another_surface() {
        let preview = EguiRect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let left = Surface::full_stage("left", "Left");
        let right = Surface::new(
            "right",
            "Right",
            [
                Point2::new(0.5, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(0.5, 1.0),
            ],
        );
        let (value, screen) = nearest_surface_snap(
            preview,
            &[left.clone(), right.clone()],
            0,
            Pos2::new(52.0, 4.0),
            12.0,
        )
        .expect("the right surface corner should snap");
        assert_eq!(value, Point2::new(0.5, 0.0));
        assert_eq!(screen, Pos2::new(50.0, 0.0));
        assert!(
            nearest_surface_snap(preview, &[left, right], 0, Pos2::new(25.0, 50.0), 12.0).is_none()
        );
    }

    fn quad(id: &str, left: f32, top: f32, right: f32, bottom: f32) -> Surface {
        Surface::new(
            id,
            id,
            [
                Point2::new(left, top),
                Point2::new(right, top),
                Point2::new(right, bottom),
                Point2::new(left, bottom),
            ],
        )
    }

    #[test]
    fn surface_selection_always_keeps_an_active_surface() {
        let mut state = EditorState::default();
        state.normalize_surface_selection(3);
        assert!(state.is_surface_selected(0));
        assert_eq!(state.selected_surfaces.len(), 1);

        state.toggle_surface_selection(2);
        assert!(state.is_surface_selected(0));
        assert!(state.is_surface_selected(2));
        assert_eq!(state.selected_surface, 2);

        state.toggle_surface_selection(2);
        assert_eq!(state.selected_surface, 0);
        assert!(state.is_surface_selected(0));

        state.toggle_surface_selection(0);
        assert!(
            state.is_surface_selected(0),
            "the last surface stays selected"
        );
        assert_eq!(state.selected_surfaces.len(), 1);
    }

    #[test]
    fn removing_a_surface_keeps_selection_indices_valid() {
        let mut state = EditorState::default();
        state.normalize_surface_selection(3);
        state.toggle_surface_selection(1);
        state.toggle_surface_selection(2);
        assert_eq!(state.selected_surfaces.len(), 3);

        state.remove_surface_selection(1, 2);
        assert_eq!(
            state.selected_surfaces.iter().copied().collect::<Vec<_>>(),
            [0, 1]
        );
        assert!(state.selected_surface < 2);
    }

    #[test]
    fn alignment_uses_the_selection_bounds() {
        let surfaces = vec![
            quad("a", 0.1, 0.1, 0.3, 0.3),
            quad("b", 0.5, 0.4, 0.7, 0.6),
            quad("c", 0.8, 0.7, 1.0, 0.9),
        ];
        let indices = [0, 1, 2];

        let left = align_surfaces(&surfaces, &indices, SurfaceAlign::Left);
        assert!(!left.is_empty());
        for (_, corners) in left {
            let left_edge = corners
                .iter()
                .map(|corner| corner.x)
                .fold(f32::INFINITY, f32::min);
            assert!((left_edge - 0.1).abs() < 1e-5);
        }

        let centered = align_surfaces(&surfaces, &indices, SurfaceAlign::CenterX);
        for (_, corners) in centered {
            let left_edge = corners
                .iter()
                .map(|corner| corner.x)
                .fold(f32::INFINITY, f32::min);
            let right_edge = corners
                .iter()
                .map(|corner| corner.x)
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(((left_edge + right_edge) * 0.5 - 0.55).abs() < 1e-5);
        }

        let top = align_surfaces(&surfaces, &indices, SurfaceAlign::Top);
        for (_, corners) in top {
            let top_edge = corners
                .iter()
                .map(|corner| corner.y)
                .fold(f32::INFINITY, f32::min);
            assert!((top_edge - 0.1).abs() < 1e-5);
        }
    }

    #[test]
    fn distribution_spaces_surface_centres_evenly() {
        let surfaces = vec![
            quad("a", 0.0, 0.0, 0.2, 0.2),
            quad("b", 0.35, 0.3, 0.55, 0.5),
            quad("c", 0.8, 0.8, 1.0, 1.0),
        ];
        let moved = distribute_surfaces(&surfaces, &[0, 1, 2], DistributeAxis::Horizontal);
        assert_eq!(moved.len(), 1);
        let (index, corners) = moved[0];
        assert_eq!(index, 1);
        let center = (corners[0].x + corners[2].x) * 0.5;
        assert!((center - 0.5).abs() < 1e-5);

        let vertical = distribute_surfaces(&surfaces, &[0, 1, 2], DistributeAxis::Vertical);
        let (index, corners) = vertical[0];
        assert_eq!(index, 1);
        let center = (corners[0].y + corners[2].y) * 0.5;
        assert!((center - 0.5).abs() < 1e-5);
    }

    #[test]
    fn whole_surface_translation_stays_on_stage() {
        let originals = [(0, quad("a", 0.0, 0.0, 0.2, 0.2).corners)];
        assert_eq!(clamp_translation(&originals, -0.5, -0.5), (0.0, 0.0));
        assert_eq!(clamp_translation(&originals, 0.9, 0.9), (0.8, 0.8));
    }

    #[test]
    fn grid_point_hit_testing_finds_the_nearest_handle() {
        let preview = EguiRect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
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
        assert_eq!(
            nearest_grid_point(preview, &grid, Pos2::new(52.0, 49.0), 12.0),
            Some((1, 1))
        );
        assert_eq!(
            nearest_grid_point(preview, &grid, Pos2::new(2.0, 2.0), 12.0),
            Some((0, 0))
        );
        assert!(nearest_grid_point(preview, &grid, Pos2::new(25.0, 25.0), 12.0).is_none());
    }

    #[test]
    fn output_pacing_tracks_missed_frames_and_smooths_intervals() {
        let mut pacing = OutputPacing::default();
        pacing.note_attempt();
        pacing.note_skipped();
        pacing.note_attempt();
        pacing.note_present(None);
        pacing.note_attempt();
        pacing.note_present(Some(20.0));
        pacing.note_attempt();
        pacing.note_error();

        assert_eq!(pacing.presented, 2);
        assert_eq!(pacing.missed(), 2);
        assert_eq!(pacing.average_interval_ms, 20.0);
        assert_eq!(pacing.fps(), Some(50.0));
    }

    #[test]
    fn media_usage_explains_layer_and_show_audio_roles() {
        let mut project = Project::demo();
        let image = media_usage(&project, &project.media[0]);
        assert!(image.contains("layer ofTheo on Surface 1"), "{image}");
        assert_eq!(media_usage(&project, &project.media[1]), "unused");

        let audio = MediaAsset::audio("media-tone", "tone", "assets/audio/tone.wav");
        let audio_id = audio.id.clone();
        project.media.push(audio);
        project.audio.media_id = Some(audio_id);
        let asset = project.media.last().unwrap();
        assert_eq!(media_usage(&project, asset), "show audio");
    }

    #[test]
    fn layer_surface_labels_explain_single_and_multi_surface_assignments() {
        let mut project = Project::demo();
        let stage = &project.stages[0];
        assert_eq!(layer_surface_label(stage, &stage.layers[0]), "Surface 1");
        assert_eq!(layer_surface_indices(stage, &stage.layers[0]), [0]);

        let second = stage.surfaces[1].id.clone();
        project.stages[0].layers[0].surface_ids.push(second);
        let stage = &project.stages[0];
        assert_eq!(
            layer_surface_label(stage, &stage.layers[0]),
            "Surface 1 + Surface 2"
        );
        assert_eq!(layer_surface_indices(stage, &stage.layers[0]), [0, 1]);
    }

    #[test]
    fn layer_media_index_maps_a_layer_to_the_pool() {
        let project = Project::demo();
        let layer = &project.stages[0].layers[0];
        assert_eq!(layer_media_index(layer, &project), Some(0));

        let pattern = lpmt_core::model::Layer::test_pattern(
            "pattern",
            "Pattern",
            &project.stages[0].surfaces[0].id,
        );
        assert_eq!(layer_media_index(&pattern, &project), None);
    }

    #[test]
    fn workspace_modes_have_distinct_labels() {
        let labels: std::collections::HashSet<&str> =
            WorkspaceMode::ALL.iter().map(|tab| tab.label()).collect();
        assert_eq!(labels.len(), WorkspaceMode::ALL.len());
    }

    #[test]
    fn topmost_surface_wins_hit_testing() {
        let preview = EguiRect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let full = Surface::full_stage("full", "Full");
        let right = Surface::new(
            "right",
            "Right",
            [
                Point2::new(0.5, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(0.5, 1.0),
            ],
        );

        assert_eq!(
            surface_at(
                preview,
                &[full.clone(), right.clone()],
                Pos2::new(75.0, 50.0)
            ),
            Some(1)
        );
        assert_eq!(
            surface_at(preview, &[full, right], Pos2::new(25.0, 50.0)),
            Some(0)
        );
    }
}

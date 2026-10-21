//! The authoritative, renderer-independent description of an LPMT show.
//!
//! The domain deliberately starts small. It is better to grow a clear model
//! from real milestones than to port every field from LPMT 1 at once.

use serde::{Deserialize, Serialize};

use crate::homography::{Homography, HomographyError};
use crate::lut::CalibrationLut;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(ProjectId);
id_type!(StageId);
id_type!(SurfaceId);
id_type!(LayerId);
id_type!(OutputId);
id_type!(OutputGroupId);
id_type!(MediaId);

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    pub x: f32,
    pub y: f32,
}

impl Point2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn clamp_unit(self) -> Self {
        Self::new(self.x.clamp(0.0, 1.0), self.y.clamp(0.0, 1.0))
    }

    pub fn distance(self, other: Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

/// A normalized rectangle in stage coordinates. `(0, 0)` is the top-left of
/// the stage and `(1, 1)` is its bottom-right.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const FULL: Self = Self {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };

    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::FULL
    }
}

/// A quad in stage space. Surface geometry is intentionally independent from
/// the content that is drawn through it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Surface {
    pub id: SurfaceId,
    pub name: String,
    /// Top-left, top-right, bottom-right, bottom-left, in unit-square order.
    pub corners: [Point2; 4],
    pub opacity: f32,
    pub visible: bool,
}

impl Surface {
    pub fn new(id: impl Into<String>, name: impl Into<String>, corners: [Point2; 4]) -> Self {
        Self {
            id: SurfaceId::new(id),
            name: name.into(),
            corners,
            opacity: 1.0,
            visible: true,
        }
    }

    pub fn full_stage(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(
            id,
            name,
            [
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(0.0, 1.0),
            ],
        )
    }

    /// Maps the content unit square onto this surface's stage-space quad.
    pub fn homography(&self) -> Result<Homography, HomographyError> {
        Homography::from_unit_quad(self.corners)
    }

    pub fn reset_to_full_stage(&mut self) {
        self.corners = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    Normal,
    Add,
    Screen,
    Multiply,
}

impl BlendMode {
    pub const ALL: [Self; 4] = [Self::Normal, Self::Add, Self::Screen, Self::Multiply];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Add => "Add",
            Self::Screen => "Screen",
            Self::Multiply => "Multiply",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestPattern {
    Checker,
    Grid,
    ColorBars,
    Grayscale,
    Convergence,
}

impl TestPattern {
    pub const ALL: [Self; 5] = [
        Self::Checker,
        Self::Grid,
        Self::ColorBars,
        Self::Grayscale,
        Self::Convergence,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Checker => "Checker",
            Self::Grid => "Alignment grid",
            Self::ColorBars => "Colour bars",
            Self::Grayscale => "Grayscale ramp",
            Self::Convergence => "Convergence",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Video,
    Audio,
}

/// A source file referenced by one or more layers. Paths are stored relative
/// to the project file where possible so a show can be moved between machines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: MediaId,
    pub name: String,
    pub kind: MediaKind,
    pub path: String,
}

impl MediaAsset {
    pub fn image(id: impl Into<String>, name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: MediaId::new(id),
            name: name.into(),
            kind: MediaKind::Image,
            path: path.into(),
        }
    }

    pub fn video(id: impl Into<String>, name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: MediaId::new(id),
            name: name.into(),
            kind: MediaKind::Video,
            path: path.into(),
        }
    }

    pub fn audio(id: impl Into<String>, name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: MediaId::new(id),
            name: name.into(),
            kind: MediaKind::Audio,
            path: path.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceSpec {
    TestPattern { pattern: TestPattern },
    Image { media_id: MediaId },
    Video { media_id: MediaId },
}

/// Content drawn through a surface. A surface is geometry; a layer is the
/// image, video or generator that goes through it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub surface_id: SurfaceId,
    pub source: SourceSpec,
    pub opacity: f32,
    pub blend: BlendMode,
    pub visible: bool,
}

impl Layer {
    pub fn test_pattern(
        id: impl Into<String>,
        name: impl Into<String>,
        surface_id: &SurfaceId,
    ) -> Self {
        Self {
            id: LayerId::new(id),
            name: name.into(),
            surface_id: surface_id.clone(),
            source: SourceSpec::TestPattern {
                pattern: TestPattern::Checker,
            },
            opacity: 1.0,
            blend: BlendMode::Normal,
            visible: true,
        }
    }

    pub fn image(
        id: impl Into<String>,
        name: impl Into<String>,
        surface_id: &SurfaceId,
        media_id: &MediaId,
    ) -> Self {
        Self {
            id: LayerId::new(id),
            name: name.into(),
            surface_id: surface_id.clone(),
            source: SourceSpec::Image {
                media_id: media_id.clone(),
            },
            opacity: 1.0,
            blend: BlendMode::Normal,
            visible: true,
        }
    }

    pub fn video(
        id: impl Into<String>,
        name: impl Into<String>,
        surface_id: &SurfaceId,
        media_id: &MediaId,
    ) -> Self {
        Self {
            id: LayerId::new(id),
            name: name.into(),
            surface_id: surface_id.clone(),
            source: SourceSpec::Video {
                media_id: media_id.clone(),
            },
            opacity: 1.0,
            blend: BlendMode::Normal,
            visible: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage {
    pub id: StageId,
    pub name: String,
    pub aspect: f32,
    pub background: [f32; 4],
    pub surfaces: Vec<Surface>,
    pub layers: Vec<Layer>,
}

impl Stage {
    pub fn surface(&self, id: &SurfaceId) -> Option<&Surface> {
        self.surfaces.iter().find(|surface| &surface.id == id)
    }

    pub fn surface_mut(&mut self, id: &SurfaceId) -> Option<&mut Surface> {
        self.surfaces.iter_mut().find(|surface| &surface.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputTarget {
    Windowed { width: u32, height: u32 },
    Fullscreen { monitor: Option<String> },
}

/// Softness at each physical edge of an output, normalized to the output's
/// width/height. Overlapping projectors feather their shared seam by ramping
/// both neighbours to black.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EdgeBlend {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl EdgeBlend {
    pub const NONE: Self = Self {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }

    pub fn clamped(self) -> Self {
        Self {
            left: self.left.clamp(0.0, 0.5),
            right: self.right.clamp(0.0, 0.5),
            top: self.top.clamp(0.0, 0.5),
            bottom: self.bottom.clamp(0.0, 0.5),
        }
    }
}

/// Per-output colour stage applied after compositing, so overlapping outputs
/// can be matched to each other independently of the show content.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorCorrection {
    /// Linear multiplier, `1.0` is neutral.
    pub brightness: f32,
    /// Contrast around mid-grey, `1.0` is neutral.
    pub contrast: f32,
    /// Display gamma, `1.0` is neutral; values above 1 lift mid-tones.
    pub gamma: f32,
}

impl Default for ColorCorrection {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl ColorCorrection {
    pub const IDENTITY: Self = Self {
        brightness: 1.0,
        contrast: 1.0,
        gamma: 1.0,
    };

    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }

    pub fn clamped(self) -> Self {
        Self {
            brightness: self.brightness.clamp(0.0, 4.0),
            contrast: self.contrast.clamp(0.0, 4.0),
            gamma: self.gamma.clamp(0.1, 4.0),
        }
    }
}

/// An output is a view of a stage presented on a physical display or virtual
/// sink. Multiple outputs can show different regions of the same stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputSpec {
    pub id: OutputId,
    pub name: String,
    pub stage_id: StageId,
    pub view: Rect,
    pub target: OutputTarget,
    pub enabled: bool,
    #[serde(default)]
    pub edge_blend: EdgeBlend,
    #[serde(default)]
    pub color: ColorCorrection,
    /// Optional display-space 1D RGB calibration curves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration: Option<CalibrationLut>,
}

impl OutputSpec {
    pub fn full_stage(id: impl Into<String>, name: impl Into<String>, stage_id: &StageId) -> Self {
        Self {
            id: OutputId::new(id),
            name: name.into(),
            stage_id: stage_id.clone(),
            view: Rect::FULL,
            target: OutputTarget::Fullscreen { monitor: None },
            enabled: true,
            edge_blend: EdgeBlend::NONE,
            color: ColorCorrection::IDENTITY,
            calibration: None,
        }
    }
}

/// A set of outputs that should render the same show frame and be presented
/// together. Outputs outside every group still render independently.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputGroup {
    pub id: OutputGroupId,
    pub name: String,
    #[serde(default)]
    pub outputs: Vec<OutputId>,
}

impl OutputGroup {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        outputs: impl IntoIterator<Item = OutputId>,
    ) -> Self {
        Self {
            id: OutputGroupId::new(id),
            name: name.into(),
            outputs: outputs.into_iter().collect(),
        }
    }
}

/// Project-level audio track. It is intentionally independent from visual
/// layers: one asset is mixed to the output device and follows the show clock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowAudio {
    #[serde(default)]
    pub media_id: Option<MediaId>,
    #[serde(default = "default_audio_volume")]
    pub volume: f32,
    #[serde(default)]
    pub muted: bool,
}

fn default_audio_volume() -> f32 {
    1.0
}

impl Default for ShowAudio {
    fn default() -> Self {
        Self {
            media_id: None,
            volume: 1.0,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub id: ProjectId,
    pub name: String,
    pub stages: Vec<Stage>,
    pub outputs: Vec<OutputSpec>,
    /// Outputs in the same group are rendered from one frame state and
    /// submitted/presented as a batch.
    #[serde(default)]
    pub output_groups: Vec<OutputGroup>,
    #[serde(default)]
    pub media: Vec<MediaAsset>,
    #[serde(default)]
    pub audio: ShowAudio,
}

impl Project {
    pub fn stage(&self, id: &StageId) -> Option<&Stage> {
        self.stages.iter().find(|stage| &stage.id == id)
    }

    pub fn primary_stage(&self) -> Option<&Stage> {
        self.stages.first()
    }

    pub fn primary_stage_mut(&mut self) -> Option<&mut Stage> {
        self.stages.first_mut()
    }

    pub fn output(&self, id: &OutputId) -> Option<&OutputSpec> {
        self.outputs.iter().find(|output| &output.id == id)
    }

    pub fn output_mut(&mut self, id: &OutputId) -> Option<&mut OutputSpec> {
        self.outputs.iter_mut().find(|output| &output.id == id)
    }

    pub fn output_group(&self, id: &OutputGroupId) -> Option<&OutputGroup> {
        self.output_groups.iter().find(|group| &group.id == id)
    }

    pub fn output_group_mut(&mut self, id: &OutputGroupId) -> Option<&mut OutputGroup> {
        self.output_groups.iter_mut().find(|group| &group.id == id)
    }

    pub fn group_for_output(&self, id: &OutputId) -> Option<&OutputGroup> {
        self.output_groups
            .iter()
            .find(|group| group.outputs.contains(id))
    }

    pub fn remove_output_from_groups(&mut self, id: &OutputId) {
        for group in &mut self.output_groups {
            group.outputs.retain(|output| output != id);
        }
    }

    pub fn media_asset(&self, id: &MediaId) -> Option<&MediaAsset> {
        self.media.iter().find(|asset| &asset.id == id)
    }

    pub fn audio_asset(&self) -> Option<&MediaAsset> {
        self.audio
            .media_id
            .as_ref()
            .and_then(|id| self.media_asset(id))
    }

    /// True when a layer or the show audio track consumes this media asset.
    pub fn is_media_used(&self, id: &MediaId) -> bool {
        if self.audio.media_id.as_ref() == Some(id) {
            return true;
        }
        self.stages.iter().any(|stage| {
            stage.layers.iter().any(|layer| match &layer.source {
                SourceSpec::Image { media_id } | SourceSpec::Video { media_id } => media_id == id,
                SourceSpec::TestPattern { .. } => false,
            })
        })
    }

    /// A deliberately small show used by the first vertical slice. It has one
    /// stage, two warped surfaces, an ordered two-layer stack and two outputs:
    /// a full-stage show output and a windowed view of the right half.
    pub fn demo() -> Self {
        let stage_id = StageId::new("stage-main");
        let surface = Surface::new(
            "surface-1",
            "Surface 1",
            [
                Point2::new(0.12, 0.12),
                Point2::new(0.88, 0.12),
                Point2::new(0.80, 0.88),
                Point2::new(0.20, 0.88),
            ],
        );
        let surface2 = Surface::new(
            "surface-2",
            "Surface 2",
            [
                Point2::new(0.62, 0.56),
                Point2::new(0.94, 0.52),
                Point2::new(0.90, 0.90),
                Point2::new(0.66, 0.88),
            ],
        );
        let media = MediaAsset::image("media-ofTheo", "ofTheo", "assets/images/ofTheo.jpg");
        let video = MediaAsset::video("media-fingers", "fingers", "assets/video/fingers.mov");
        let car = MediaAsset::image("media-car", "car", "assets/images/car.jpg");
        let layer = Layer::image("layer-1", "ofTheo", &surface.id, &media.id);
        let mut layer2 = Layer::image("layer-2", "car", &surface2.id, &car.id);
        layer2.opacity = 0.8;
        layer2.blend = BlendMode::Screen;

        let main_output = OutputSpec::full_stage("output-1", "Main output", &stage_id);
        let mut right_half = OutputSpec::full_stage("output-2", "Right half", &stage_id);
        right_half.view = Rect::new(0.5, 0.0, 0.5, 1.0);
        right_half.target = OutputTarget::Windowed {
            width: 960,
            height: 540,
        };

        let group = OutputGroup::new(
            "group-main",
            "Main wall",
            [main_output.id.clone(), right_half.id.clone()],
        );

        Self {
            schema_version: 2,
            id: ProjectId::new("project-demo"),
            name: "Untitled show".to_owned(),
            stages: vec![Stage {
                id: stage_id.clone(),
                name: "Main stage".to_owned(),
                aspect: 16.0 / 9.0,
                background: [0.015, 0.018, 0.025, 1.0],
                surfaces: vec![surface, surface2],
                layers: vec![layer, layer2],
            }],
            outputs: vec![main_output, right_half],
            output_groups: vec![group],
            media: vec![media, video, car],
            audio: ShowAudio::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_project_is_serializable() {
        let project = Project::demo();
        let json = serde_json::to_string(&project).unwrap();
        let round_trip: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project, round_trip);
    }

    #[test]
    fn project_without_media_is_forward_compatible() {
        let mut value = serde_json::to_value(Project::demo()).unwrap();
        value.as_object_mut().unwrap().remove("media");
        let project: Project = serde_json::from_value(value).unwrap();
        assert!(project.media.is_empty());
    }

    #[test]
    fn older_projects_load_without_output_groups() {
        let mut value = serde_json::to_value(Project::demo()).unwrap();
        value.as_object_mut().unwrap().remove("output_groups");
        let project: Project = serde_json::from_value(value).unwrap();
        assert!(project.output_groups.is_empty());
    }

    #[test]
    fn older_projects_load_without_audio_track() {
        let mut value = serde_json::to_value(Project::demo()).unwrap();
        value.as_object_mut().unwrap().remove("audio");
        let project: Project = serde_json::from_value(value).unwrap();
        assert_eq!(project.audio, ShowAudio::default());
    }

    #[test]
    fn show_audio_counts_as_used_media() {
        let mut project = Project::demo();
        let track = MediaAsset::audio("media-tone", "tone", "assets/audio/tone.wav");
        let track_id = track.id.clone();
        project.media.push(track);
        project.audio.media_id = Some(track_id.clone());

        assert!(project.is_media_used(&track_id));
        assert_eq!(
            project.audio_asset().map(|asset| asset.id.clone()),
            Some(track_id)
        );
    }

    #[test]
    fn demo_outputs_share_one_render_group() {
        let mut project = Project::demo();
        let group = project
            .group_for_output(&project.outputs[0].id)
            .expect("demo output belongs to a group");
        assert_eq!(group.outputs.len(), 2);
        assert_eq!(
            project
                .group_for_output(&project.outputs[1].id)
                .map(|group| group.id.clone()),
            Some(group.id.clone())
        );

        let removed = project.outputs[1].id.clone();
        project.remove_output_from_groups(&removed);
        assert!(project.output_groups[0].outputs.len() == 1);
        assert!(project.group_for_output(&removed).is_none());
    }

    #[test]
    fn detects_used_media() {
        let project = Project::demo();
        let image = &project.media[0];
        let video = &project.media[1];
        let overlay = &project.media[2];
        assert!(project.is_media_used(&image.id));
        assert!(!project.is_media_used(&video.id));
        assert!(project.is_media_used(&overlay.id));
    }

    #[test]
    fn demo_has_two_outputs_with_distinct_views() {
        let project = Project::demo();
        assert_eq!(project.outputs.len(), 2);
        assert_eq!(project.outputs[0].view, Rect::FULL);
        assert_eq!(project.outputs[1].view, Rect::new(0.5, 0.0, 0.5, 1.0));
        assert_ne!(project.outputs[0].id, project.outputs[1].id);
        assert!(project
            .outputs
            .iter()
            .all(|output| output.stage_id == project.stages[0].id));
    }

    #[test]
    fn outputs_are_found_by_stable_id() {
        let mut project = Project::demo();
        let id = project.outputs[1].id.clone();
        assert_eq!(project.output(&id).unwrap().name, "Right half");
        project.output_mut(&id).unwrap().enabled = false;
        assert!(!project.output(&id).unwrap().enabled);
        assert!(project.output(&OutputId::new("missing")).is_none());
    }

    #[test]
    fn output_correction_defaults_are_neutral() {
        let output = OutputSpec::full_stage("output", "Output", &StageId::new("stage"));
        assert_eq!(output.edge_blend, EdgeBlend::NONE);
        assert_eq!(output.color, ColorCorrection::IDENTITY);
        assert!(output.edge_blend.is_none());
        assert!(output.color.is_identity());
    }

    #[test]
    fn older_outputs_load_without_correction_fields() {
        let mut value = serde_json::to_value(Project::demo()).unwrap();
        for output in value["outputs"].as_array_mut().unwrap() {
            let output = output.as_object_mut().unwrap();
            output.remove("edge_blend");
            output.remove("color");
            output.remove("calibration");
        }

        let project: Project = serde_json::from_value(value).unwrap();
        assert!(project.outputs.iter().all(|output| {
            output.edge_blend.is_none()
                && output.color.is_identity()
                && output.calibration.is_none()
        }));
    }

    #[test]
    fn calibration_lut_round_trips_with_the_project() {
        let mut project = Project::demo();
        project.outputs[0].calibration = Some(CalibrationLut::identity("test display"));
        let json = serde_json::to_string(&project).unwrap();
        let round_trip: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project, round_trip);
        assert_eq!(
            round_trip.outputs[0]
                .calibration
                .as_ref()
                .map(|lut| lut.name.as_str()),
            Some("test display")
        );
    }

    #[test]
    fn every_test_pattern_has_a_distinct_label() {
        let mut labels = std::collections::HashSet::new();
        for pattern in TestPattern::ALL {
            assert!(labels.insert(pattern.label()), "{pattern:?} is duplicated");
            let json = serde_json::to_string(&pattern).unwrap();
            assert_eq!(serde_json::from_str::<TestPattern>(&json).unwrap(), pattern);
        }
    }

    #[test]
    fn edge_blend_and_colour_clamp_to_sane_ranges() {
        let blend = EdgeBlend {
            left: -1.0,
            right: 2.0,
            top: 0.1,
            bottom: 0.6,
        }
        .clamped();
        assert_eq!(blend.left, 0.0);
        assert_eq!(blend.right, 0.5);
        assert_eq!(blend.top, 0.1);
        assert_eq!(blend.bottom, 0.5);

        let color = ColorCorrection {
            brightness: -1.0,
            contrast: 9.0,
            gamma: 0.0,
        }
        .clamped();
        assert_eq!(color.brightness, 0.0);
        assert_eq!(color.contrast, 4.0);
        assert_eq!(color.gamma, 0.1);
    }

    #[test]
    fn demo_has_an_ordered_layer_stack() {
        let project = Project::demo();
        let stage = project.primary_stage().unwrap();
        assert_eq!(stage.surfaces.len(), 2);
        assert_eq!(stage.layers.len(), 2);
        assert_eq!(stage.layers[0].blend, BlendMode::Normal);
        assert_eq!(stage.layers[1].blend, BlendMode::Screen);
    }

    #[test]
    fn video_layer_round_trips() {
        let mut project = Project::demo();
        let video_id = project.media[1].id.clone();
        let surface_id = project.primary_stage().unwrap().surfaces[0].id.clone();
        project.primary_stage_mut().unwrap().layers[0] =
            Layer::video("layer-video", "fingers", &surface_id, &video_id);

        let json = serde_json::to_string(&project).unwrap();
        let round_trip: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project, round_trip);
        assert!(round_trip.is_media_used(&video_id));
    }

    #[test]
    fn surface_homography_maps_corners() {
        let surface = Surface::new(
            "surface",
            "Surface",
            [
                Point2::new(0.1, 0.2),
                Point2::new(0.9, 0.1),
                Point2::new(0.8, 0.9),
                Point2::new(0.2, 0.7),
            ],
        );
        let homography = surface.homography().unwrap();
        let src = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        for (source, expected) in src.into_iter().zip(surface.corners) {
            let actual = homography.transform(source);
            assert!((actual.x - expected.x).abs() < 1e-5);
            assert!((actual.y - expected.y).abs() < 1e-5);
        }
    }
}

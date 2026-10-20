//! The authoritative, renderer-independent description of an LPMT show.
//!
//! The domain deliberately starts small. It is better to grow a clear model
//! from real milestones than to port every field from LPMT 1 at once.

use serde::{Deserialize, Serialize};

use crate::homography::{Homography, HomographyError};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestPattern {
    Checker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Video,
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
    #[serde(default)]
    pub media: Vec<MediaAsset>,
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

    pub fn media_asset(&self, id: &MediaId) -> Option<&MediaAsset> {
        self.media.iter().find(|asset| &asset.id == id)
    }

    /// True when at least one layer on any stage consumes this media asset.
    pub fn is_media_used(&self, id: &MediaId) -> bool {
        self.stages.iter().any(|stage| {
            stage.layers.iter().any(|layer| match &layer.source {
                SourceSpec::Image { media_id } | SourceSpec::Video { media_id } => media_id == id,
                SourceSpec::TestPattern { .. } => false,
            })
        })
    }

    /// A deliberately small show used by the first vertical slice. It has one
    /// stage, two warped surfaces, an ordered two-layer stack and one full-stage
    /// output.
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
            outputs: vec![OutputSpec::full_stage("output-1", "Main output", &stage_id)],
            media: vec![media, video, car],
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

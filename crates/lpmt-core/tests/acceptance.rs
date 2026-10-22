//! Automated portions of the M2/M5 acceptance checks.
//!
//! These tests deliberately build a two-surface show from scratch, add a
//! video, timeline, bindings and cue stack, then round-trip it through JSON.
//! The remaining M2/M3/M5 checks require a real media file, a second display,
//! MIDI hardware or a human listener; those live in `docs/acceptance.md`.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lpmt_core::binding::{MidiBinding, MidiMessageKind, MidiMode, ParameterBinding};
use lpmt_core::model::{
    BlendMode, ChromaKey, CircleCrop, Crop, Layer, LayerColor, MediaAsset, OutputGroup, OutputId,
    OutputSpec, OutputTarget, Point2, Project, ProjectId, Rect, ShowAudio, Stage, StageId, Surface,
    SurfaceMask,
};
use lpmt_core::timeline::{
    Timeline, TimelineColourKey, TimelineColourTrack, TimelineCommand, TimelineCue,
    TimelineInterpolation, TimelineKey, TimelineMarker, TimelineTrigger,
};

fn scratch_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "lpmt-acceptance-{name}-{}-{unique}.json",
        std::process::id()
    ))
}

#[test]
fn two_surface_show_with_video_timeline_bindings_and_cues_round_trips() {
    let stage_id = StageId::new("stage-acceptance");
    let surface1_id = lpmt_core::model::SurfaceId::new("surface-main");
    let surface2_id = lpmt_core::model::SurfaceId::new("surface-overlay");

    let mut surface1 = Surface::new(
        "surface-main",
        "Main",
        [
            Point2::new(0.08, 0.10),
            Point2::new(0.92, 0.10),
            Point2::new(0.86, 0.90),
            Point2::new(0.14, 0.90),
        ],
    );
    surface1.mask = SurfaceMask::rectangle(0.08);
    surface1.mask.feather = 0.03;

    let surface2 = Surface::full_stage("surface-overlay", "Overlay");

    let image = MediaAsset::image("media-image", "image", "assets/images/ofTheo.jpg");
    let video = MediaAsset::video("media-video", "video", "assets/video/fingers.mov");
    let audio = MediaAsset::audio("media-audio", "audio", "assets/audio/tone.wav");

    let mut image_layer = Layer::image("layer-main", "Main image", &surface1_id, &image.id);
    image_layer.crop = Crop {
        top: 0.04,
        right: 0.04,
        bottom: 0.04,
        left: 0.04,
        circle: Some(CircleCrop {
            center: Point2::new(0.5, 0.5),
            radius: 0.48,
        }),
    };
    image_layer.color = LayerColor {
        tint: [1.0, 0.9, 0.8],
    };
    image_layer.chroma_key = ChromaKey {
        enabled: true,
        color: [0.0, 1.0, 0.0],
        tolerance: 0.2,
        softness: 0.05,
        spill: 0.3,
    };
    let mut video_layer = Layer::video("layer-overlay", "Video overlay", &surface2_id, &video.id);
    video_layer.opacity = 0.8;
    video_layer.blend = BlendMode::Screen;

    let mut output1 = OutputSpec::full_stage("output-main", "Main output", &stage_id);
    output1.target = OutputTarget::Windowed {
        width: 1920,
        height: 1080,
    };
    output1.edge_blend.right = 0.1;
    let mut output2 = OutputSpec::full_stage("output-right", "Right output", &stage_id);
    output2.view = Rect::new(0.5, 0.0, 0.5, 1.0);
    output2.target = OutputTarget::Windowed {
        width: 1920,
        height: 1080,
    };
    let group = OutputGroup::new(
        "group-acceptance",
        "Acceptance wall",
        [output1.id.clone(), output2.id.clone()],
    );

    let mut timeline = Timeline {
        duration: 8.0,
        bpm: 120.0,
        ..Default::default()
    };
    let opacity_track = timeline.add_track("layer/layer-main/opacity");
    timeline.tracks[opacity_track].keys = vec![
        TimelineKey {
            time: 0.0,
            value: 0.0,
            interpolation: TimelineInterpolation::Linear,
        },
        TimelineKey {
            time: 4.0,
            value: 1.0,
            interpolation: TimelineInterpolation::Smooth,
        },
        TimelineKey {
            time: 8.0,
            value: 0.25,
            interpolation: TimelineInterpolation::Step,
        },
    ];
    let brightness_track = timeline.add_track("output/output-main/brightness");
    timeline.tracks[brightness_track].keys = vec![
        TimelineKey {
            time: 0.0,
            value: 0.75,
            interpolation: TimelineInterpolation::Linear,
        },
        TimelineKey {
            time: 8.0,
            value: 1.25,
            interpolation: TimelineInterpolation::Linear,
        },
    ];
    let mut colour_track = TimelineColourTrack::new(OutputId::new("output-main"));
    colour_track.keys = vec![
        TimelineColourKey {
            time: 0.0,
            tint: [1.0, 1.0, 1.0],
            interpolation: TimelineInterpolation::Linear,
        },
        TimelineColourKey {
            time: 8.0,
            tint: [0.8, 0.9, 1.0],
            interpolation: TimelineInterpolation::Smooth,
        },
    ];
    timeline.colour_tracks.push(colour_track);
    timeline.triggers.push(TimelineTrigger {
        time: 7.0,
        command: TimelineCommand::Pause,
    });
    timeline.markers.push(TimelineMarker {
        time: 4.0,
        name: "Reveal".to_owned(),
    });
    timeline.cues.push(TimelineCue {
        id: 1,
        name: "Start".to_owned(),
        time: 0.0,
        note: "Acceptance cue".to_owned(),
        color: [0.2, 0.8, 1.0],
        commands: vec![TimelineCommand::LoopOn],
        auto: true,
        follow_seconds: 1.5,
    });
    timeline.normalize();

    let project = Project {
        schema_version: 2,
        id: ProjectId::new("project-acceptance"),
        name: "Acceptance show".to_owned(),
        stages: vec![Stage {
            id: stage_id,
            name: "Acceptance stage".to_owned(),
            aspect: 16.0 / 9.0,
            background: [0.0, 0.0, 0.0, 1.0],
            surfaces: vec![surface1, surface2],
            layers: vec![image_layer, video_layer],
        }],
        outputs: vec![output1, output2],
        output_groups: vec![group],
        media: vec![image, video, audio.clone()],
        audio: ShowAudio {
            media_id: Some(audio.id),
            volume: 0.8,
            muted: false,
        },
        bindings: vec![ParameterBinding::new(
            "layer/layer-main/opacity",
            "f5",
            false,
            false,
            false,
        )],
        midi_bindings: vec![MidiBinding::new(
            "output/output-main/brightness",
            0,
            MidiMessageKind::ControlChange,
            7,
            MidiMode::Absolute,
        )],
        timeline,
    };

    let path = scratch_path("round-trip");
    fs::write(&path, serde_json::to_vec_pretty(&project).unwrap()).unwrap();
    let loaded: Project = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    fs::remove_file(&path).unwrap();

    assert_eq!(loaded, project);
    let stage = loaded.primary_stage().expect("stage survives reload");
    assert_eq!(stage.surfaces.len(), 2);
    assert_eq!(stage.layers.len(), 2);
    assert_eq!(stage.surfaces[0].mask.points.len(), 4);
    assert_eq!(stage.layers[0].color.tint, [1.0, 0.9, 0.8]);
    assert!(stage.layers[0].chroma_key.is_active());
    assert_eq!(loaded.outputs.len(), 2);
    assert_eq!(loaded.output_groups[0].outputs.len(), 2);
    assert!(loaded.is_media_used(&project.media[1].id));
    assert_eq!(loaded.audio.volume, 0.8);
    assert_eq!(loaded.bindings.len(), 1);
    assert_eq!(loaded.midi_bindings.len(), 1);

    let timeline = &loaded.timeline;
    assert_eq!(timeline.markers[0].name, "Reveal");
    assert_eq!(timeline.triggers[0].command, TimelineCommand::Pause);
    assert_eq!(timeline.cues[0].follow_seconds, 1.5);
    assert_eq!(
        timeline
            .tracks
            .iter()
            .find(|track| track.parameter == "layer/layer-main/opacity")
            .and_then(|track| track.value_at(2.0)),
        Some(0.5)
    );
    assert_eq!(
        timeline.sample(2.0),
        vec![
            ("layer/layer-main/opacity".to_owned(), 0.5),
            ("output/output-main/brightness".to_owned(), 0.875),
        ]
    );
}

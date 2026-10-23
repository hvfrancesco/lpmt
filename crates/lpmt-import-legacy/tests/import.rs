use std::time::{SystemTime, UNIX_EPOCH};

use lpmt_core::model::{BlendMode, SourceSpec};
use lpmt_import_legacy::{import_file, import_xml, ImportedProject};

const FIXTURE: &str = include_str!("fixtures/legacy-show.xml");

fn fixture_project() -> ImportedProject {
    import_xml(FIXTURE, "Legacy fixture").expect("fixture should import")
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.0e-5
}

#[test]
fn imports_the_reference_quad_show() {
    let imported = fixture_project();
    let project = &imported.project;
    let stage = project.primary_stage().expect("one imported stage");

    assert_eq!(project.name, "Legacy fixture");
    assert_eq!(stage.surfaces.len(), 4);
    assert_eq!(stage.layers.len(), 3);
    assert_eq!(project.media.len(), 2);
    assert_eq!(project.outputs.len(), 1);
    assert_eq!(
        project.outputs[0].target,
        lpmt_core::model::OutputTarget::Windowed {
            width: 1280,
            height: 720
        }
    );

    // Legacy LAYER values order the image (0), shared video (1) and solid (2)
    // layers independently of the quad XML order.
    let image = &stage.layers[0];
    assert!(matches!(image.source, SourceSpec::Image { .. }));
    assert_eq!(image.color.tint, [0.9, 0.8, 0.7]);
    assert!(close(image.opacity, 0.8));
    assert_eq!(image.blend, BlendMode::Screen);
    assert!(close(image.crop.top, 0.04));
    assert!(close(image.crop.right, 0.05));
    assert!(close(image.crop.bottom, 0.06));
    assert!(close(image.crop.left, 0.07));
    let circle = image.crop.circle.expect("circular crop");
    assert!(close(circle.radius, 0.48));

    let image_surface = &stage.surfaces[0];
    assert!(image_surface.mask.enabled);
    assert!(image_surface.mask.invert);
    assert_eq!(image_surface.mask.points.len(), 4);
    assert!(close(image_surface.mask.points[0].x, 0.10));
    assert!(close(image_surface.mask.points[0].y, 0.10));
    assert!(close(image_surface.mask.points[2].x, 0.90));
    assert!(close(image_surface.mask.points[2].y, 0.90));
    assert!(close(image_surface.corners[1].x, 0.91));
    assert!(close(image_surface.corners[2].y, 0.90));

    let shared = &stage.layers[1];
    let SourceSpec::Video { media_id } = &shared.source else {
        panic!("shared video should become a video layer");
    };
    assert_eq!(shared.blend, BlendMode::Add);
    let asset = project.media_asset(media_id).expect("shared video media");
    assert_eq!(asset.path, "assets/video/shared-loop.mov");

    let solid = &stage.layers[2];
    let SourceSpec::Solid { color } = solid.source else {
        panic!("solid colour should become a solid layer");
    };
    assert_eq!(color, [0.20, 0.40, 0.80]);
    assert!(close(solid.opacity, 0.5));

    assert_eq!(imported.report.quads, 4);
    assert_eq!(imported.report.surfaces, 4);
    assert_eq!(imported.report.layers, 3);
    assert_eq!(imported.report.media, 2);
    for feature in [
        "timeline",
        "cameras",
        "per-quad edge blending",
        "colour transitions",
    ] {
        assert!(
            imported
                .report
                .unsupported_features
                .iter()
                .any(|item| item == feature),
            "missing unsupported feature {feature}: {:?}",
            imported.report.unsupported_features
        );
    }
    assert!(imported
        .report
        .warnings
        .iter()
        .any(|warning| warning.contains("camera content")));

    let json = serde_json::to_string(project).expect("imported project serializes");
    let round_trip: lpmt_core::model::Project =
        serde_json::from_str(&json).expect("imported project reloads");
    assert_eq!(&round_trip, project);
}

#[test]
fn infers_quad_count_when_general_count_is_missing() {
    let xml = r#"
        <LPMT>
          <QUADS>
            <QUAD_0>
              <IS_ON>1</IS_ON>
              <CORNERS>
                <CORNER_0><X>0.0</X><Y>0.0</Y></CORNER_0>
                <CORNER_1><X>1.0</X><Y>0.0</Y></CORNER_1>
                <CORNER_2><X>1.0</X><Y>1.0</Y></CORNER_2>
                <CORNER_3><X>0.0</X><Y>1.0</Y></CORNER_3>
              </CORNERS>
              <COLOR><ACTIVE>1</ACTIVE><R>0.1</R><G>0.2</G><B>0.3</B><A>1.0</A></COLOR>
            </QUAD_0>
            <QUAD_1>
              <IS_ON>1</IS_ON>
              <CORNERS>
                <CORNER_0><X>0.2</X><Y>0.2</Y></CORNER_0>
                <CORNER_1><X>0.8</X><Y>0.2</Y></CORNER_1>
                <CORNER_2><X>0.8</X><Y>0.8</Y></CORNER_2>
                <CORNER_3><X>0.2</X><Y>0.8</Y></CORNER_3>
              </CORNERS>
              <COLOR><ACTIVE>1</ACTIVE><R>0.4</R><G>0.5</G><B>0.6</B><A>1.0</A></COLOR>
            </QUAD_1>
          </QUADS>
        </LPMT>
    "#;
    let imported = import_xml(xml, "Inferred").expect("inferred show imports");
    assert_eq!(imported.report.quads, 2);
    assert_eq!(imported.project.stages[0].surfaces.len(), 2);
    assert_eq!(imported.project.stages[0].layers.len(), 2);
}

#[test]
fn unsupported_content_does_not_create_a_layer() {
    let xml = r#"
        <LPMT>
          <GENERAL><N_OF_QUADS>1</N_OF_QUADS></GENERAL>
          <QUADS>
            <QUAD_0>
              <IS_ON>0</IS_ON>
              <CORNERS>
                <CORNER_0><X>0.0</X><Y>0.0</Y></CORNER_0>
                <CORNER_1><X>1.0</X><Y>0.0</Y></CORNER_1>
                <CORNER_2><X>1.0</X><Y>1.0</Y></CORNER_2>
                <CORNER_3><X>0.0</X><Y>1.0</Y></CORNER_3>
              </CORNERS>
              <CAM><ACTIVE>1</ACTIVE></CAM>
              <SLIDESHOW><ACTIVE>1</ACTIVE></SLIDESHOW>
            </QUAD_0>
          </QUADS>
        </LPMT>
    "#;
    let imported = import_xml(xml, "Unsupported").expect("unsupported content imports");
    assert_eq!(imported.project.stages[0].surfaces.len(), 1);
    assert!(!imported.project.stages[0].surfaces[0].visible);
    assert!(imported.project.stages[0].layers.is_empty());
    assert!(imported
        .report
        .unsupported_features
        .iter()
        .any(|item| item == "cameras"));
    assert!(imported
        .report
        .warnings
        .iter()
        .any(|warning| warning.contains("slideshow is active but no folder")));
}

#[test]
fn imports_slideshow_folders_relative_to_the_xml_file() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "lpmt-import-slideshow-{}-{unique}",
        std::process::id()
    ));
    let slides = directory.join("slides");
    std::fs::create_dir_all(&slides).unwrap();
    std::fs::write(slides.join("b.jpg"), b"").unwrap();
    std::fs::write(slides.join("a.png"), b"").unwrap();
    std::fs::write(slides.join("ignore.txt"), b"").unwrap();
    let xml_path = directory.join("_lpmt_settings.xml");
    std::fs::write(
        &xml_path,
        r#"
        <LPMT>
          <GENERAL><N_OF_QUADS>1</N_OF_QUADS></GENERAL>
          <QUADS>
            <QUAD_0>
              <NUMBER>1</NUMBER><LAYER>0</LAYER><IS_ON>1</IS_ON>
              <CORNERS>
                <CORNER_0><X>0.0</X><Y>0.0</Y></CORNER_0>
                <CORNER_1><X>1.0</X><Y>0.0</Y></CORNER_1>
                <CORNER_2><X>1.0</X><Y>1.0</Y></CORNER_2>
                <CORNER_3><X>0.0</X><Y>1.0</Y></CORNER_3>
              </CORNERS>
              <SLIDESHOW>
                <ACTIVE>1</ACTIVE>
                <LOADED_SLIDESHOW>slides</LOADED_SLIDESHOW>
                <SPEED>2.5</SPEED>
              </SLIDESHOW>
              <IMG><COLORIZE><R>0.9</R><G>0.9</G><B>0.9</B><A>1.0</A></COLORIZE></IMG>
            </QUAD_0>
          </QUADS>
        </LPMT>
        "#,
    )
    .unwrap();

    let imported = import_file(&xml_path).expect("slideshow show imports");
    let project = &imported.project;
    assert_eq!(project.media.len(), 2);
    assert_eq!(project.media[0].path, "slides/a.png");
    assert_eq!(project.media[1].path, "slides/b.jpg");
    let layer = &project.stages[0].layers[0];
    let SourceSpec::Slideshow { slideshow } = &layer.source else {
        panic!("slideshow source expected");
    };
    assert_eq!(slideshow.media_ids.len(), 2);
    assert_eq!(slideshow.interval, 2.5);
    assert!(slideshow.looping);
    assert_eq!(layer.color.tint, [0.9, 0.9, 0.9]);

    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn malformed_xml_is_an_error() {
    assert!(import_xml("<LPMT><QUADS>", "Broken").is_err());
}

#[test]
fn a_file_without_quads_is_not_treated_as_a_show() {
    let error = import_xml(
        "<LPMT><GENERAL><N_OF_QUADS>0</N_OF_QUADS></GENERAL></LPMT>",
        "Config",
    )
    .expect_err("no-quad XML should be rejected");
    assert!(error.to_string().contains("no quads found"));
}

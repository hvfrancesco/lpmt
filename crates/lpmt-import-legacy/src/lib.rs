//! Importer for LPMT 1.x XML project files.
//!
//! LPMT 1 stored a show as a flat `ofXml` tree. The tree does not contain a
//! stage or output model: it contains up to 36 quads, each with corners, an
//! optional mask/crop and a set of possible content sources. This module maps
//! that representation onto LPMT 2's Surface + Layer + Output model.
//!
//! The importer is deliberately independent from the UI and renderer so it can
//! be tested against fixtures and used from a command line migration path.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use lpmt_core::model::{
    BlendMode, CircleCrop, Crop, Layer, LayerColor, MediaAsset, MediaId, MediaKind, OutputSpec,
    OutputTarget, Point2, Project, ProjectId, ShowAudio, Slideshow, Stage, StageId, Surface,
    SurfaceId, SurfaceMask,
};
use lpmt_core::timeline::Timeline;
use roxmltree::Document;
use serde::Serialize;

const MAX_LEGACY_QUADS: usize = 36;
const LEGACY_SCHEMA_VERSION: u32 = 2;
const IMPORTED_STAGE_ID: &str = "stage-imported";
const IMPORTED_OUTPUT_ID: &str = "output-imported";

/// A migrated project plus information about anything that could not be
/// represented faithfully in LPMT 2.
#[derive(Debug, Clone)]
pub struct ImportedProject {
    pub project: Project,
    pub report: ImportReport,
}

/// Human-readable migration diagnostics. Warnings are per quad; unsupported
/// features are deduplicated so a large show does not repeat the same message
/// dozens of times.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportReport {
    pub quads: usize,
    pub surfaces: usize,
    pub layers: usize,
    pub media: usize,
    pub warnings: Vec<String>,
    pub unsupported_features: Vec<String>,
}

impl ImportReport {
    fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    fn unsupported(&mut self, feature: &str, message: impl Into<String>) {
        if self
            .unsupported_features
            .iter()
            .all(|existing| existing != feature)
        {
            self.unsupported_features.push(feature.to_owned());
        }
        self.warn(message);
    }
}

/// Import an LPMT 1 XML settings file from disk.
///
/// The project name defaults to the input file stem. Media paths are preserved
/// exactly as written in the legacy file; the caller can decide where to store
/// the imported JSON and how to resolve those paths.
pub fn import_file(path: impl AsRef<Path>) -> Result<ImportedProject> {
    let path = path.as_ref();
    let xml = fs::read_to_string(path)
        .with_context(|| format!("failed to read legacy XML {}", path.display()))?;
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or("Imported LPMT 1 show");
    import_xml_with_base(
        &xml,
        name,
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty()),
    )
}

/// Import LPMT 1 settings from an in-memory XML string.
pub fn import_xml(xml: &str, name: &str) -> Result<ImportedProject> {
    import_xml_with_base(xml, name, None)
}

fn import_xml_with_base(xml: &str, name: &str, base: Option<&Path>) -> Result<ImportedProject> {
    let values = flatten_xml(xml)?;
    if values.is_empty() {
        bail!("legacy XML contains no settings");
    }

    let mut report = ImportReport::default();
    let stage_id = StageId::new(IMPORTED_STAGE_ID);
    let mut media = Vec::new();
    let mut media_by_path = HashMap::new();
    let mut surfaces = Vec::new();
    let mut pending_layers = Vec::new();

    // Shared videos are global in LPMT 1. They are referenced by a one-based
    // index, unlike the zero-based XML element names.
    let mut shared_videos = HashMap::new();
    for index in 0..4 {
        let key = format!("SHARED_VIDEOS:VIDEO_{index}:PATH");
        let path = get_string(&values, &key);
        if path.is_empty() {
            continue;
        }
        let id = ensure_media(
            &mut media,
            &mut media_by_path,
            &path,
            MediaKind::Video,
            &format!("Shared video {}", index + 1),
        );
        shared_videos.insert(index + 1, id);
    }

    let quad_count = inferred_quad_count(&values);
    if quad_count == 0 {
        bail!("no quads found; is this an LPMT 1 settings file rather than config.xml?");
    }

    for quad_index in 0..quad_count {
        let prefix = format!("QUADS:QUAD_{quad_index}");
        report.quads += 1;

        let number =
            get_i32(&values, &format!("{prefix}:NUMBER"), quad_index as i32 + 1).max(0) as u32;
        let display_number = if number == 0 {
            quad_index + 1
        } else {
            number as usize
        };
        let surface_name = format!("Quad {display_number}");
        let legacy_layer =
            get_i32(&values, &format!("{prefix}:LAYER"), quad_index as i32).max(0) as i32;
        let is_on = get_bool(&values, &format!("{prefix}:IS_ON"));

        let corners = read_corners(&values, &prefix, &surface_name, &mut report);
        let surface_id = SurfaceId::new(format!("surface-quad-{quad_index}"));
        let mut surface = Surface::new(surface_id.as_str(), surface_name.clone(), corners);
        surface.visible = is_on;
        surface.mask = read_mask(&values, &prefix, display_number, &mut report);

        let crop = read_crop(&values, &prefix);
        let blend = read_blend_mode(&values, &prefix, quad_index, &mut report);
        let edge_blend = get_bool(&values, &format!("{prefix}:EDGE_BLENDING:ON"));
        if edge_blend {
            report.unsupported(
                "per-quad edge blending",
                format!(
                    "Quad {display_number}: legacy per-quad edge blending has no direct per-surface equivalent; output edge blending must be configured in LPMT 2."
                ),
            );
        }
        if get_bool(&values, &format!("{prefix}:DEFORM:BEZIER:ON"))
            || get_bool(&values, &format!("{prefix}:DEFORM:GRID:ON"))
        {
            report.unsupported(
                "Bézier/grid deform",
                format!(
                    "Quad {display_number}: Bézier or grid deformation is not migrated; the quad's four corners are used."
                ),
            );
        }
        if get_bool(&values, &format!("{prefix}:COLOR:TRANS:ACTIVE")) {
            report.unsupported(
                "colour transitions",
                format!("Quad {display_number}: colour transitions are not migrated."),
            );
        }
        if get_bool(&values, &format!("{prefix}:CAM:ACTIVE")) {
            report.unsupported(
                "cameras",
                format!("Quad {display_number}: camera content is not migrated."),
            );
        }
        // LPMT 1 can draw several content sources on the same quad. Keep that
        // ordering: solid colour, video, shared video, camera/slideshow, then
        // image. Camera is unsupported; slideshow is imported when the legacy
        // folder can be resolved from the XML file location.
        if get_bool(&values, &format!("{prefix}:COLOR:ACTIVE")) {
            let color = [
                get_f32(&values, &format!("{prefix}:COLOR:R"), 0.0).clamp(0.0, 1.0),
                get_f32(&values, &format!("{prefix}:COLOR:G"), 0.0).clamp(0.0, 1.0),
                get_f32(&values, &format!("{prefix}:COLOR:B"), 0.0).clamp(0.0, 1.0),
            ];
            let alpha = get_f32(&values, &format!("{prefix}:COLOR:A"), 1.0).clamp(0.0, 1.0);
            if alpha > 0.0 {
                let layer_id = format!("layer-quad-{quad_index}-solid");
                let mut layer = Layer::solid(
                    layer_id,
                    format!("{surface_name} solid"),
                    &surface_id,
                    color,
                );
                layer.opacity = alpha;
                layer.visible = is_on;
                layer.crop = crop;
                layer.blend = blend;
                pending_layers.push(PendingLayer {
                    legacy_layer,
                    quad_index,
                    source_order: 0,
                    layer,
                });
            }
        }

        if get_bool(&values, &format!("{prefix}:VIDEO:ACTIVE")) {
            let path = get_string(&values, &format!("{prefix}:VIDEO:LOADED_VIDEO_PATH"));
            if path.is_empty() {
                report.warn(format!(
                    "Quad {display_number}: video is active but no video path was saved."
                ));
            } else {
                let id = ensure_media(
                    &mut media,
                    &mut media_by_path,
                    &path,
                    MediaKind::Video,
                    &get_string(&values, &format!("{prefix}:VIDEO:LOADED_VIDEO")),
                );
                let mut layer = Layer::video(
                    format!("layer-quad-{quad_index}-video"),
                    format!("{surface_name} video"),
                    &surface_id,
                    &id,
                );
                apply_legacy_layer_style(
                    &mut layer,
                    &values,
                    &prefix,
                    LegacyLayerStyle {
                        source: "VIDEO",
                        is_on,
                        crop,
                        blend,
                        display_number,
                    },
                    &mut report,
                );
                pending_layers.push(PendingLayer {
                    legacy_layer,
                    quad_index,
                    source_order: 1,
                    layer,
                });
            }
        }

        if get_bool(&values, &format!("{prefix}:SHARED_VIDEO:ACTIVE")) {
            let shared_number = get_usize(&values, &format!("{prefix}:SHARED_VIDEO:NUM"), 1).max(1);
            if let Some(id) = shared_videos.get(&shared_number) {
                let mut layer = Layer::video(
                    format!("layer-quad-{quad_index}-shared-video"),
                    format!("{surface_name} shared video {shared_number}"),
                    &surface_id,
                    id,
                );
                apply_legacy_layer_style(
                    &mut layer,
                    &values,
                    &prefix,
                    LegacyLayerStyle {
                        source: "VIDEO",
                        is_on,
                        crop,
                        blend,
                        display_number,
                    },
                    &mut report,
                );
                pending_layers.push(PendingLayer {
                    legacy_layer,
                    quad_index,
                    source_order: 2,
                    layer,
                });
            } else {
                report.warn(format!(
                    "Quad {display_number}: shared video {shared_number} is active but that shared video slot is empty."
                ));
            }
        }

        if get_bool(&values, &format!("{prefix}:SLIDESHOW:ACTIVE")) {
            let folder = get_string(&values, &format!("{prefix}:SLIDESHOW:LOADED_SLIDESHOW"));
            if folder.is_empty() {
                report.warn(format!(
                    "Quad {display_number}: slideshow is active but no folder was saved."
                ));
            } else if let Some(base) = base {
                let folder_path = Path::new(&folder);
                let resolved = if folder_path.is_absolute() {
                    folder_path.to_path_buf()
                } else {
                    base.join(folder_path)
                };
                match slideshow_images(&resolved) {
                    Ok(images) if images.is_empty() => report.warn(format!(
                        "Quad {display_number}: no PNG/JPEG images found in slideshow folder {}.",
                        resolved.display()
                    )),
                    Ok(images) => {
                        let mut media_ids = Vec::with_capacity(images.len());
                        for image in images {
                            let stored = if folder_path.is_absolute() {
                                image.display().to_string()
                            } else {
                                folder_path
                                    .join(
                                        image
                                            .file_name()
                                            .unwrap_or_else(|| std::ffi::OsStr::new("slide")),
                                    )
                                    .to_string_lossy()
                                    .into_owned()
                            };
                            media_ids.push(ensure_media(
                                &mut media,
                                &mut media_by_path,
                                &stored,
                                MediaKind::Image,
                                "",
                            ));
                        }
                        let interval = get_f32(&values, &format!("{prefix}:SLIDESHOW:SPEED"), 1.0);
                        let mut layer = Layer::slideshow(
                            format!("layer-quad-{quad_index}-slideshow"),
                            format!("{surface_name} slideshow"),
                            &surface_id,
                            Slideshow {
                                media_ids,
                                interval,
                                looping: true,
                                start_index: 0,
                            },
                        );
                        apply_legacy_layer_style(
                            &mut layer,
                            &values,
                            &prefix,
                            LegacyLayerStyle {
                                source: "IMG",
                                is_on,
                                crop,
                                blend,
                                display_number,
                            },
                            &mut report,
                        );
                        pending_layers.push(PendingLayer {
                            legacy_layer,
                            quad_index,
                            source_order: 4,
                            layer,
                        });
                        if get_bool(&values, &format!("{prefix}:SLIDESHOW:FIT")) {
                            report.unsupported(
                                "slideshow fit/aspect",
                                format!(
                                    "Quad {display_number}: slideshow fit/keep-aspect is not migrated; images use the layer crop and surface warp."
                                ),
                            );
                        }
                    }
                    Err(error) => report.warn(format!(
                        "Quad {display_number}: could not read slideshow folder {}: {error:#}",
                        resolved.display()
                    )),
                }
            } else {
                report.unsupported(
                    "slideshows",
                    format!(
                        "Quad {display_number}: slideshow needs file context; import through the XML file path to resolve its folder."
                    ),
                );
            }
        }

        if get_bool(&values, &format!("{prefix}:IMG:ACTIVE")) {
            let path = get_string(&values, &format!("{prefix}:IMG:LOADED_IMG_PATH"));
            if path.is_empty() {
                report.warn(format!(
                    "Quad {display_number}: image is active but no image path was saved."
                ));
            } else {
                let id = ensure_media(
                    &mut media,
                    &mut media_by_path,
                    &path,
                    MediaKind::Image,
                    &get_string(&values, &format!("{prefix}:IMG:LOADED_IMG")),
                );
                let mut layer = Layer::image(
                    format!("layer-quad-{quad_index}-image"),
                    format!("{surface_name} image"),
                    &surface_id,
                    &id,
                );
                apply_legacy_layer_style(
                    &mut layer,
                    &values,
                    &prefix,
                    LegacyLayerStyle {
                        source: "IMG",
                        is_on,
                        crop,
                        blend,
                        display_number,
                    },
                    &mut report,
                );
                pending_layers.push(PendingLayer {
                    legacy_layer,
                    quad_index,
                    source_order: 5,
                    layer,
                });
            }
        }

        surfaces.push(surface);
    }

    pending_layers.sort_by_key(|pending| {
        (
            pending.legacy_layer,
            pending.quad_index,
            pending.source_order,
        )
    });
    let layers: Vec<Layer> = pending_layers
        .into_iter()
        .map(|pending| pending.layer)
        .collect();

    if get_bool(&values, "TIMELINE:USE_TIMELINE") {
        report.unsupported(
            "timeline",
            "Legacy timeline cues and tracks are not migrated; the v2 timeline starts empty.",
        );
    }

    let stage = Stage {
        id: stage_id.clone(),
        name: "Imported LPMT 1 stage".to_owned(),
        aspect: 16.0 / 9.0,
        background: [0.0, 0.0, 0.0, 1.0],
        surfaces,
        layers,
    };

    let mut output = OutputSpec::full_stage(IMPORTED_OUTPUT_ID, "Imported output", &stage_id);
    output.target = OutputTarget::Windowed {
        width: 1280,
        height: 720,
    };

    let project_name = if name.trim().is_empty() {
        "Imported LPMT 1 show".to_owned()
    } else {
        name.trim().to_owned()
    };
    let project = Project {
        schema_version: LEGACY_SCHEMA_VERSION,
        id: ProjectId::new("project-imported"),
        name: project_name,
        stages: vec![stage],
        outputs: vec![output],
        output_groups: Vec::new(),
        media,
        audio: ShowAudio::default(),
        bindings: Vec::new(),
        midi_bindings: Vec::new(),
        timeline: Timeline::default(),
    };

    report.surfaces = project.stages[0].surfaces.len();
    report.layers = project.stages[0].layers.len();
    report.media = project.media.len();

    Ok(ImportedProject { project, report })
}

struct PendingLayer {
    legacy_layer: i32,
    quad_index: usize,
    source_order: u8,
    layer: Layer,
}

struct LegacyLayerStyle {
    source: &'static str,
    is_on: bool,
    crop: Crop,
    blend: BlendMode,
    display_number: usize,
}

fn apply_legacy_layer_style(
    layer: &mut Layer,
    values: &HashMap<String, String>,
    prefix: &str,
    style: LegacyLayerStyle,
    report: &mut ImportReport,
) {
    let source = style.source;
    let tint = [
        get_f32(values, &format!("{prefix}:{source}:COLORIZE:R"), 1.0).clamp(0.0, 4.0),
        get_f32(values, &format!("{prefix}:{source}:COLORIZE:G"), 1.0).clamp(0.0, 4.0),
        get_f32(values, &format!("{prefix}:{source}:COLORIZE:B"), 1.0).clamp(0.0, 4.0),
    ];
    let alpha = get_f32(values, &format!("{prefix}:{source}:COLORIZE:A"), 1.0).clamp(0.0, 1.0);
    layer.color = LayerColor { tint };
    layer.opacity = alpha;
    layer.visible = style.is_on && alpha > 0.0;
    layer.crop = style.crop;
    layer.blend = style.blend;

    let h_flip = get_bool(values, &format!("{prefix}:{source}:FLIP:H"));
    let v_flip = get_bool(values, &format!("{prefix}:{source}:FLIP:V"));
    let mult_x = get_f32(values, &format!("{prefix}:{source}:MULT_X"), 1.0);
    let mult_y = get_f32(values, &format!("{prefix}:{source}:MULT_Y"), 1.0);
    if h_flip
        || v_flip
        || (mult_x - 1.0).abs() > f32::EPSILON
        || (mult_y - 1.0).abs() > f32::EPSILON
    {
        report.unsupported(
            "source flips/scaling",
            format!(
                "Quad {}: {source} flips or source scaling were not migrated; use the surface warp or crops instead.",
                style.display_number
            ),
        );
    }

    if source == "VIDEO" {
        let speed = get_f32(values, &format!("{prefix}:VIDEO:SPEED"), 1.0);
        let volume = get_f32(values, &format!("{prefix}:VIDEO:VOLUME"), 0.0);
        let looping = get_bool_default(values, &format!("{prefix}:VIDEO:LOOP"), true);
        if (speed - 1.0).abs() > f32::EPSILON || volume > 0.0 || !looping {
            report.warn(format!(
                "Quad {}: video speed, volume and loop settings are not migrated.",
                style.display_number
            ));
        }
    }
}

fn read_corners(
    values: &HashMap<String, String>,
    prefix: &str,
    surface_name: &str,
    report: &mut ImportReport,
) -> [Point2; 4] {
    let mut corners = [Point2::new(0.0, 0.0); 4];
    let mut complete = true;
    for (index, corner) in corners.iter_mut().enumerate() {
        let x_key = format!("{prefix}:CORNERS:CORNER_{index}:X");
        let y_key = format!("{prefix}:CORNERS:CORNER_{index}:Y");
        let Some(x) = get_optional_f32(values, &x_key) else {
            complete = false;
            break;
        };
        let Some(y) = get_optional_f32(values, &y_key) else {
            complete = false;
            break;
        };
        *corner = Point2::new(x, y).clamp_unit();
    }
    if complete {
        corners
    } else {
        report.warn(format!(
            "{surface_name}: corners are incomplete; using a full-stage quad."
        ));
        Surface::full_stage("fallback", surface_name).corners
    }
}

fn read_mask(
    values: &HashMap<String, String>,
    prefix: &str,
    display_number: usize,
    report: &mut ImportReport,
) -> SurfaceMask {
    let enabled = get_bool(values, &format!("{prefix}:MASK:ON"));
    let point_count = get_usize(values, &format!("{prefix}:MASK:N_POINTS"), 0);
    let mut raw_points = Vec::new();
    for point_index in 0..point_count.min(256) {
        let x_key = format!("{prefix}:MASK:POINTS:POINT_{point_index}:X");
        let y_key = format!("{prefix}:MASK:POINTS:POINT_{point_index}:Y");
        let (Some(x), Some(y)) = (
            get_optional_f32(values, &x_key),
            get_optional_f32(values, &y_key),
        ) else {
            continue;
        };
        raw_points.push((x, y));
    }

    // LPMT 1 stored mask points in screen pixels, while LPMT 2's SurfaceMask
    // is in surface UV space. Newer/hand-edited files may already contain
    // normalized points, so only rescale when a coordinate is outside 0..1.
    let pixel_space = raw_points
        .iter()
        .any(|(x, y)| x.abs() > 1.0 || y.abs() > 1.0);
    let mut scale_x = 1.0;
    let mut scale_y = 1.0;
    if pixel_space {
        let saved_width = get_f32(values, &format!("{prefix}:CONTENT:WIDTH"), 0.0);
        let saved_height = get_f32(values, &format!("{prefix}:CONTENT:HEIGHT"), 0.0);
        scale_x = if saved_width > 1.0 {
            saved_width
        } else {
            raw_points.iter().map(|(x, _)| *x).fold(1.0_f32, f32::max)
        };
        scale_y = if saved_height > 1.0 {
            saved_height
        } else {
            raw_points.iter().map(|(_, y)| *y).fold(1.0_f32, f32::max)
        };
        if saved_width <= 1.0 || saved_height <= 1.0 {
            report.warn(format!(
                "Quad {display_number}: mask points are in screen pixels but the legacy content size was not saved; they were normalized by their maximum coordinate."
            ));
        }
    }
    let points: Vec<Point2> = raw_points
        .into_iter()
        .map(|(x, y)| Point2::new((x / scale_x).clamp(0.0, 1.0), (y / scale_y).clamp(0.0, 1.0)))
        .collect();

    if enabled && points.len() < 3 {
        report.warn(format!(
            "Quad {display_number}: mask is enabled but has fewer than three valid points; the mask is disabled."
        ));
    }
    SurfaceMask {
        enabled: enabled && points.len() >= 3,
        invert: get_bool(values, &format!("{prefix}:MASK:INVERT_MASK")),
        feather: 0.0,
        points,
    }
}

fn read_crop(values: &HashMap<String, String>, prefix: &str) -> Crop {
    let circle_x = get_f32(values, &format!("{prefix}:MASK:CROP:CIRCULAR:X"), 0.5).clamp(0.0, 1.0);
    let circle_y = get_f32(values, &format!("{prefix}:MASK:CROP:CIRCULAR:Y"), 0.5).clamp(0.0, 1.0);
    let radius =
        get_f32(values, &format!("{prefix}:MASK:CROP:CIRCULAR:RADIUS"), 0.0).clamp(0.0, 1.0);
    Crop {
        top: get_f32(values, &format!("{prefix}:MASK:CROP:RECTANGULAR:TOP"), 0.0),
        right: get_f32(
            values,
            &format!("{prefix}:MASK:CROP:RECTANGULAR:RIGHT"),
            0.0,
        ),
        bottom: get_f32(
            values,
            &format!("{prefix}:MASK:CROP:RECTANGULAR:BOTTOM"),
            0.0,
        ),
        left: get_f32(values, &format!("{prefix}:MASK:CROP:RECTANGULAR:LEFT"), 0.0),
        circle: (radius > 0.0).then_some(CircleCrop {
            center: Point2::new(circle_x, circle_y),
            radius,
        }),
    }
    .clamped()
}

fn read_blend_mode(
    values: &HashMap<String, String>,
    prefix: &str,
    quad_index: usize,
    report: &mut ImportReport,
) -> BlendMode {
    if !get_bool(values, &format!("{prefix}:BLENDING:ON")) {
        return BlendMode::Normal;
    }
    match get_i32(values, &format!("{prefix}:BLENDING:MODE"), 0) {
        0 => BlendMode::Screen,
        1 => BlendMode::Add,
        3 => BlendMode::Multiply,
        other => {
            report.unsupported(
                "subtractive blending",
                format!(
                    "Quad {}: legacy blend mode {other} is subtractive and has no direct equivalent; Normal was used.",
                    quad_index + 1
                ),
            );
            BlendMode::Normal
        }
    }
}

fn inferred_quad_count(values: &HashMap<String, String>) -> usize {
    if let Some(saved) = values
        .get("GENERAL:N_OF_QUADS")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|saved| *saved > 0)
    {
        return saved.min(MAX_LEGACY_QUADS);
    }
    let mut highest = None;
    for key in values.keys() {
        if let Some(rest) = key.strip_prefix("QUADS:QUAD_") {
            if let Some(index) = rest
                .split(':')
                .next()
                .and_then(|index| index.parse::<usize>().ok())
            {
                highest = Some(highest.map_or(index, |current: usize| current.max(index)));
            }
        }
    }
    highest.map_or(0, |index| (index + 1).min(MAX_LEGACY_QUADS))
}

fn slideshow_images(folder: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut images = Vec::new();
    for entry in fs::read_dir(folder)
        .with_context(|| format!("failed to read slideshow folder {}", folder.display()))?
    {
        let path = entry
            .with_context(|| format!("failed to read an entry in {}", folder.display()))?
            .path();
        if !path.is_file() {
            continue;
        }
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        if matches!(extension.as_deref(), Some("png" | "jpg" | "jpeg")) {
            images.push(path);
        }
    }
    images.sort();
    images.truncate(512);
    Ok(images)
}

fn ensure_media(
    media: &mut Vec<MediaAsset>,
    by_path: &mut HashMap<String, MediaId>,
    path: &str,
    kind: MediaKind,
    name_hint: &str,
) -> MediaId {
    let path = path.trim();
    let key = format!("{kind:?}\u{1f}{path}");
    if let Some(id) = by_path.get(&key) {
        return id.clone();
    }

    let id = MediaId::new(format!("media-imported-{}", media.len() + 1));
    let name = if name_hint.trim().is_empty() {
        Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.trim().is_empty())
            .unwrap_or("Imported media")
            .to_owned()
    } else {
        name_hint.trim().to_owned()
    };
    let asset = match kind {
        MediaKind::Image => MediaAsset::image(id.as_str(), name, path),
        MediaKind::Video => MediaAsset::video(id.as_str(), name, path),
        MediaKind::Audio => MediaAsset::audio(id.as_str(), name, path),
    };
    media.push(asset);
    by_path.insert(key, id.clone());
    id
}

/// Convert the nested `ofXml` tree into `PATH:TO:VALUE` keys. Both the wrapped
/// root form (`<LPMT><GENERAL>...`) and a bare root element are accepted.
fn flatten_xml(xml: &str) -> Result<HashMap<String, String>> {
    let xml = xml.trim_start_matches('\u{feff}');
    let document = Document::parse(xml).context("failed to parse legacy XML")?;
    let root = document.root_element();
    let mut values = HashMap::new();
    for child in root.children().filter(|node| node.is_element()) {
        collect_values(child, "", &mut values);
    }
    // Some ofXml writers omit the wrapper and place the first section at the
    // root. Indexing from the root name as well keeps both forms working.
    let root_name = root.tag_name().name();
    if !root_name.eq_ignore_ascii_case("LPMT") {
        collect_values(root, root_name, &mut values);
    }
    Ok(values)
}

fn collect_values(
    node: roxmltree::Node<'_, '_>,
    parent: &str,
    values: &mut HashMap<String, String>,
) {
    let name = node.tag_name().name();
    let path = if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}:{name}")
    };
    if node.children().any(|child| child.is_element()) {
        for child in node.children().filter(|child| child.is_element()) {
            collect_values(child, &path, values);
        }
    } else {
        values.insert(path, node.text().unwrap_or("").trim().to_owned());
    }
}

fn get_string(values: &HashMap<String, String>, key: &str) -> String {
    values
        .get(key)
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

fn get_optional_f32(values: &HashMap<String, String>, key: &str) -> Option<f32> {
    values
        .get(key)
        .and_then(|value| value.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite())
}

fn get_f32(values: &HashMap<String, String>, key: &str, default: f32) -> f32 {
    get_optional_f32(values, key).unwrap_or(default)
}

fn get_i32(values: &HashMap<String, String>, key: &str, default: i32) -> i32 {
    values
        .get(key)
        .and_then(|value| value.trim().parse::<i32>().ok())
        .unwrap_or(default)
}

fn get_usize(values: &HashMap<String, String>, key: &str, default: usize) -> usize {
    values
        .get(key)
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn get_bool(values: &HashMap<String, String>, key: &str) -> bool {
    get_bool_default(values, key, false)
}

fn get_bool_default(values: &HashMap<String, String>, key: &str, default: bool) -> bool {
    values
        .get(key)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

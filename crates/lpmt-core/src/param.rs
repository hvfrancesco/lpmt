//! Stable, addressable show parameters.
//!
//! The editor, OSC, MIDI and the future timeline all address the same parameter
//! tree. Paths use stable object IDs, for example:
//!
//! `surface/surface-1/opacity`
//! `layer/layer-1/opacity`
//! `output/output-1/brightness`
//! `transport/playing`

use std::fmt;

use crate::model::{BlendMode, LayerId, OutputId, Project, SurfaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl EdgeSide {
    pub const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Top, Self::Bottom];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TintChannel {
    Red,
    Green,
    Blue,
}

impl TintChannel {
    pub const ALL: [Self; 3] = [Self::Red, Self::Green, Self::Blue];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "red" | "r" => Some(Self::Red),
            "green" | "g" => Some(Self::Green),
            "blue" | "b" => Some(Self::Blue),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterKind {
    Float,
    Bool,
    Text,
}

impl ParameterKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Text => "text",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParameterId {
    SurfaceOpacity(SurfaceId),
    SurfaceVisible(SurfaceId),
    SurfaceMaskEnabled(SurfaceId),
    SurfaceMaskInvert(SurfaceId),
    SurfaceMaskFeather(SurfaceId),
    LayerOpacity(LayerId),
    LayerVisible(LayerId),
    LayerBlend(LayerId),
    OutputEnabled(OutputId),
    OutputBrightness(OutputId),
    OutputContrast(OutputId),
    OutputGamma(OutputId),
    OutputTint(OutputId, TintChannel),
    OutputEdgeBlend(OutputId, EdgeSide),
    ShowAudioVolume,
    ShowAudioMuted,
    TransportPlaying,
    TransportPosition,
    TransportLooping,
}

impl ParameterId {
    pub fn path(&self) -> String {
        match self {
            Self::SurfaceOpacity(id) => format!("surface/{id}/opacity"),
            Self::SurfaceVisible(id) => format!("surface/{id}/visible"),
            Self::SurfaceMaskEnabled(id) => format!("surface/{id}/mask/enabled"),
            Self::SurfaceMaskInvert(id) => format!("surface/{id}/mask/invert"),
            Self::SurfaceMaskFeather(id) => format!("surface/{id}/mask/feather"),
            Self::LayerOpacity(id) => format!("layer/{id}/opacity"),
            Self::LayerVisible(id) => format!("layer/{id}/visible"),
            Self::LayerBlend(id) => format!("layer/{id}/blend"),
            Self::OutputEnabled(id) => format!("output/{id}/enabled"),
            Self::OutputBrightness(id) => format!("output/{id}/brightness"),
            Self::OutputContrast(id) => format!("output/{id}/contrast"),
            Self::OutputGamma(id) => format!("output/{id}/gamma"),
            Self::OutputTint(id, channel) => {
                format!("output/{id}/tint/{}", channel.label())
            }
            Self::OutputEdgeBlend(id, side) => {
                format!("output/{id}/edge_blend/{}", side.label())
            }
            Self::ShowAudioVolume => "audio/volume".to_owned(),
            Self::ShowAudioMuted => "audio/muted".to_owned(),
            Self::TransportPlaying => "transport/playing".to_owned(),
            Self::TransportPosition => "transport/position".to_owned(),
            Self::TransportLooping => "transport/looping".to_owned(),
        }
    }

    pub fn parse(path: &str) -> Result<Self, ParameterError> {
        let normalized = path.trim().trim_matches('/');
        let normalized = normalized.strip_prefix("lpmt/").unwrap_or(normalized);
        let parts: Vec<&str> = normalized.split('/').collect();
        let unknown = || ParameterError::UnknownPath(path.to_owned());
        match parts.as_slice() {
            ["surface", id, "opacity"] => Ok(Self::SurfaceOpacity(SurfaceId::new(*id))),
            ["surface", id, "visible"] => Ok(Self::SurfaceVisible(SurfaceId::new(*id))),
            ["surface", id, "mask", "enabled"] => Ok(Self::SurfaceMaskEnabled(SurfaceId::new(*id))),
            ["surface", id, "mask", "invert"] => Ok(Self::SurfaceMaskInvert(SurfaceId::new(*id))),
            ["surface", id, "mask", "feather"] => Ok(Self::SurfaceMaskFeather(SurfaceId::new(*id))),
            ["layer", id, "opacity"] => Ok(Self::LayerOpacity(LayerId::new(*id))),
            ["layer", id, "visible"] => Ok(Self::LayerVisible(LayerId::new(*id))),
            ["layer", id, "blend"] => Ok(Self::LayerBlend(LayerId::new(*id))),
            ["output", id, "enabled"] => Ok(Self::OutputEnabled(OutputId::new(*id))),
            ["output", id, "brightness"] => Ok(Self::OutputBrightness(OutputId::new(*id))),
            ["output", id, "contrast"] => Ok(Self::OutputContrast(OutputId::new(*id))),
            ["output", id, "gamma"] => Ok(Self::OutputGamma(OutputId::new(*id))),
            ["output", id, "tint", channel] => TintChannel::parse(channel)
                .map(|channel| Self::OutputTint(OutputId::new(*id), channel))
                .ok_or_else(unknown),
            ["output", id, "edge_blend", side] => EdgeSide::parse(side)
                .map(|side| Self::OutputEdgeBlend(OutputId::new(*id), side))
                .ok_or_else(unknown),
            ["audio", "volume"] => Ok(Self::ShowAudioVolume),
            ["audio", "muted"] => Ok(Self::ShowAudioMuted),
            ["transport", "playing"] => Ok(Self::TransportPlaying),
            ["transport", "position"] => Ok(Self::TransportPosition),
            ["transport", "looping"] => Ok(Self::TransportLooping),
            _ => Err(unknown()),
        }
    }

    pub const fn kind(&self) -> ParameterKind {
        match self {
            Self::SurfaceOpacity(_)
            | Self::SurfaceMaskFeather(_)
            | Self::LayerOpacity(_)
            | Self::OutputBrightness(_)
            | Self::OutputContrast(_)
            | Self::OutputGamma(_)
            | Self::OutputTint(_, _)
            | Self::OutputEdgeBlend(_, _)
            | Self::ShowAudioVolume
            | Self::TransportPosition => ParameterKind::Float,
            Self::SurfaceVisible(_)
            | Self::SurfaceMaskEnabled(_)
            | Self::SurfaceMaskInvert(_)
            | Self::LayerVisible(_)
            | Self::OutputEnabled(_)
            | Self::ShowAudioMuted
            | Self::TransportPlaying
            | Self::TransportLooping => ParameterKind::Bool,
            Self::LayerBlend(_) => ParameterKind::Text,
        }
    }

    /// Inclusive normalized range for simple UI/OSC validation. Parameters
    /// without a fixed upper bound, such as transport position, return `None`.
    pub fn range(&self) -> Option<(f32, f32)> {
        match self {
            Self::SurfaceOpacity(_) | Self::LayerOpacity(_) | Self::ShowAudioVolume => {
                Some((0.0, 1.0))
            }
            Self::SurfaceMaskFeather(_) => Some((0.0, 0.5)),
            Self::OutputBrightness(_) | Self::OutputContrast(_) => Some((0.0, 4.0)),
            Self::OutputGamma(_) => Some((0.1, 4.0)),
            Self::OutputTint(_, _) => Some((0.0, 4.0)),
            Self::OutputEdgeBlend(_, _) => Some((0.0, 0.5)),
            _ => None,
        }
    }
}

impl fmt::Display for ParameterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.path())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    Text(String),
}

impl ParameterValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Float(value) => value.is_finite().then_some(*value),
            Self::Int(value) => Some(*value as f32),
            Self::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Self::Text(value) => value.parse::<f32>().ok().filter(|value| value.is_finite()),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            Self::Int(value) => Some(*value != 0),
            Self::Float(value) => value.is_finite().then_some(*value != 0.0),
            Self::Text(value) => match value.trim().to_ascii_lowercase().as_str() {
                "1" | "true" | "on" | "yes" => Some(true),
                "0" | "false" | "off" | "no" => Some(false),
                text => text.parse::<f32>().ok().map(|value| value != 0.0),
            },
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    pub fn blend_mode(&self) -> Option<BlendMode> {
        self.as_text().and_then(BlendMode::parse)
    }
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(value) => write!(f, "{value:.3}"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Text(value) => f.write_str(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParameterError {
    #[error("unknown parameter path: {0}")]
    UnknownPath(String),
}

#[derive(Debug, Clone)]
pub struct ParameterDescriptor {
    pub id: ParameterId,
    pub label: String,
    pub kind: ParameterKind,
    pub range: Option<(f32, f32)>,
}

impl ParameterDescriptor {
    pub fn path(&self) -> String {
        self.id.path()
    }
}

/// Enumerate the parameters currently exposed by a project. The stable path is
/// derived from object IDs, so it survives reordering and renaming.
pub fn project_parameters(project: &Project) -> Vec<ParameterDescriptor> {
    let mut parameters = Vec::new();
    if let Some(stage) = project.primary_stage() {
        for surface in &stage.surfaces {
            for id in [
                ParameterId::SurfaceOpacity(surface.id.clone()),
                ParameterId::SurfaceVisible(surface.id.clone()),
                ParameterId::SurfaceMaskEnabled(surface.id.clone()),
                ParameterId::SurfaceMaskInvert(surface.id.clone()),
                ParameterId::SurfaceMaskFeather(surface.id.clone()),
            ] {
                let label = format!("{} {}", surface.name, suffix(&id));
                parameters.push(descriptor(id, label));
            }
        }
        for layer in &stage.layers {
            for id in [
                ParameterId::LayerOpacity(layer.id.clone()),
                ParameterId::LayerVisible(layer.id.clone()),
                ParameterId::LayerBlend(layer.id.clone()),
            ] {
                let label = format!("{} {}", layer.name, suffix(&id));
                parameters.push(descriptor(id, label));
            }
        }
    }
    for output in &project.outputs {
        for id in [
            ParameterId::OutputEnabled(output.id.clone()),
            ParameterId::OutputBrightness(output.id.clone()),
            ParameterId::OutputContrast(output.id.clone()),
            ParameterId::OutputGamma(output.id.clone()),
        ] {
            let label = format!("{} {}", output.name, suffix(&id));
            parameters.push(descriptor(id, label));
        }
        for channel in TintChannel::ALL {
            let id = ParameterId::OutputTint(output.id.clone(), channel);
            parameters.push(descriptor(
                id.clone(),
                format!("{} {}", output.name, suffix(&id)),
            ));
        }
        for side in EdgeSide::ALL {
            let id = ParameterId::OutputEdgeBlend(output.id.clone(), side);
            parameters.push(descriptor(
                id.clone(),
                format!("{} {}", output.name, suffix(&id)),
            ));
        }
    }
    parameters.push(descriptor(
        ParameterId::ShowAudioVolume,
        "Show volume".to_owned(),
    ));
    parameters.push(descriptor(
        ParameterId::ShowAudioMuted,
        "Show audio mute".to_owned(),
    ));
    parameters.push(descriptor(
        ParameterId::TransportPlaying,
        "Transport playing".to_owned(),
    ));
    parameters.push(descriptor(
        ParameterId::TransportPosition,
        "Transport position".to_owned(),
    ));
    parameters.push(descriptor(
        ParameterId::TransportLooping,
        "Transport looping".to_owned(),
    ));
    parameters
}

fn descriptor(id: ParameterId, label: String) -> ParameterDescriptor {
    ParameterDescriptor {
        kind: id.kind(),
        range: id.range(),
        id,
        label,
    }
}

fn suffix(id: &ParameterId) -> &'static str {
    match id {
        ParameterId::SurfaceOpacity(_) => "opacity",
        ParameterId::SurfaceVisible(_) => "visible",
        ParameterId::SurfaceMaskEnabled(_) => "mask enabled",
        ParameterId::SurfaceMaskInvert(_) => "mask invert",
        ParameterId::SurfaceMaskFeather(_) => "mask feather",
        ParameterId::LayerOpacity(_) => "opacity",
        ParameterId::LayerVisible(_) => "visible",
        ParameterId::LayerBlend(_) => "blend",
        ParameterId::OutputEnabled(_) => "enabled",
        ParameterId::OutputBrightness(_) => "brightness",
        ParameterId::OutputContrast(_) => "contrast",
        ParameterId::OutputGamma(_) => "gamma",
        ParameterId::OutputTint(_, channel) => match channel {
            TintChannel::Red => "tint red",
            TintChannel::Green => "tint green",
            TintChannel::Blue => "tint blue",
        },
        ParameterId::OutputEdgeBlend(_, side) => match side {
            EdgeSide::Left => "edge left",
            EdgeSide::Right => "edge right",
            EdgeSide::Top => "edge top",
            EdgeSide::Bottom => "edge bottom",
        },
        ParameterId::ShowAudioVolume => "volume",
        ParameterId::ShowAudioMuted => "muted",
        ParameterId::TransportPlaying => "playing",
        ParameterId::TransportPosition => "position",
        ParameterId::TransportLooping => "looping",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_round_trip_for_every_parameter() {
        let project = Project::demo();
        for descriptor in project_parameters(&project) {
            let parsed = ParameterId::parse(&descriptor.id.path()).unwrap();
            assert_eq!(parsed, descriptor.id);
            assert_eq!(descriptor.kind, descriptor.id.kind());
            assert_eq!(descriptor.range, descriptor.id.range());
        }
    }

    #[test]
    fn osc_style_prefix_is_accepted() {
        assert_eq!(
            ParameterId::parse("/lpmt/output/output-1/brightness").unwrap(),
            ParameterId::OutputBrightness(OutputId::new("output-1"))
        );
        assert_eq!(
            ParameterId::parse("/surface/surface-1/mask/feather").unwrap(),
            ParameterId::SurfaceMaskFeather(SurfaceId::new("surface-1"))
        );
    }

    #[test]
    fn unknown_and_malformed_paths_are_rejected() {
        assert!(ParameterId::parse("layer/layer-1/nope").is_err());
        assert!(ParameterId::parse("output/output-1/edge_blend/sideways").is_err());
        assert!(ParameterId::parse("").is_err());
    }

    #[test]
    fn parameter_values_coerce_for_osc_convenience() {
        assert_eq!(ParameterValue::Int(1).as_bool(), Some(true));
        assert_eq!(ParameterValue::Float(0.0).as_bool(), Some(false));
        assert_eq!(ParameterValue::Text("on".to_owned()).as_bool(), Some(true));
        assert_eq!(ParameterValue::Text("0.75".to_owned()).as_f32(), Some(0.75));
        assert_eq!(
            ParameterValue::Text("screen".to_owned()).blend_mode(),
            Some(BlendMode::Screen)
        );
        assert_eq!(ParameterValue::Float(f32::NAN).as_f32(), None);
    }

    #[test]
    fn project_parameters_are_unique_and_include_the_demo_objects() {
        let parameters = project_parameters(&Project::demo());
        let mut paths = std::collections::HashSet::new();
        for descriptor in &parameters {
            assert!(paths.insert(descriptor.path()), "duplicate path");
        }
        assert!(paths.contains("surface/surface-1/opacity"));
        assert!(paths.contains("layer/layer-1/opacity"));
        assert!(paths.contains("output/output-1/edge_blend/right"));
        assert!(paths.contains("output/output-1/tint/red"));
        assert!(paths.contains("transport/playing"));
    }
}

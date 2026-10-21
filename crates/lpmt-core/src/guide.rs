//! Session-only calibration guides drawn on an output.
//!
//! Guides are deliberately not part of the project schema: they are a setup
//! aid for aligning projectors and should never be saved into a show.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GuideMode {
    #[default]
    None,
    Grid,
    Crosshair,
    SafeArea,
    Thirds,
}

impl GuideMode {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Grid,
        Self::Crosshair,
        Self::SafeArea,
        Self::Thirds,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Grid => "Grid",
            Self::Crosshair => "Crosshair",
            Self::SafeArea => "Safe area",
            Self::Thirds => "Thirds",
        }
    }

    /// Value consumed by the composite shader. Keep this stable: zero means
    /// no overlay and every other value has a matching branch in WGSL.
    pub const fn shader_value(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Grid => 1,
            Self::Crosshair => 2,
            Self::SafeArea => 3,
            Self::Thirds => 4,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Grid,
            Self::Grid => Self::Crosshair,
            Self::Crosshair => Self::SafeArea,
            Self::SafeArea => Self::Thirds,
            Self::Thirds => Self::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycling_visits_every_mode_and_returns_to_none() {
        let mut mode = GuideMode::None;
        let mut seen = Vec::new();
        for _ in 0..GuideMode::ALL.len() {
            seen.push(mode);
            mode = mode.next();
        }
        assert_eq!(seen, GuideMode::ALL);
        assert_eq!(mode, GuideMode::None);
    }

    #[test]
    fn shader_values_are_unique_and_zero_is_none() {
        assert_eq!(GuideMode::None.shader_value(), 0);
        let values: std::collections::HashSet<u32> = GuideMode::ALL
            .iter()
            .map(|mode| mode.shader_value())
            .collect();
        assert_eq!(values.len(), GuideMode::ALL.len());
    }
}

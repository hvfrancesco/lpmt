//! Numeric timeline tracks.
//!
//! Timeline keys target the same stable parameter paths as OSC, MIDI and the
//! editor. The first slice is deliberately numeric: value curves with step or
//! linear interpolation. Bool/trigger and colour tracks can build on the same
//! timing model without changing the project format.

use serde::{Deserialize, Serialize};

pub const MIN_TIMELINE_DURATION: f32 = 0.1;
pub const MAX_TIMELINE_DURATION: f32 = 3600.0;
const DEFAULT_TIMELINE_DURATION: f32 = 10.0;
const DUPLICATE_TIME_EPSILON: f32 = 1.0e-4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineInterpolation {
    Step,
    #[default]
    Linear,
}

impl TimelineInterpolation {
    pub const ALL: [Self; 2] = [Self::Linear, Self::Step];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Step => "Step",
            Self::Linear => "Linear",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimelineKey {
    pub time: f32,
    pub value: f32,
    #[serde(default)]
    pub interpolation: TimelineInterpolation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineTrack {
    /// Stable parameter path, for example `layer/layer-1/opacity`.
    pub parameter: String,
    #[serde(default)]
    pub keys: Vec<TimelineKey>,
}

impl TimelineTrack {
    pub fn new(parameter: impl Into<String>) -> Self {
        Self {
            parameter: parameter.into(),
            keys: Vec::new(),
        }
    }

    /// Value at `seconds`. The timeline clamps to the first/last key outside
    /// the keyed range, so a track does not jump back to zero at either end.
    pub fn value_at(&self, seconds: f32) -> Option<f32> {
        let first = self.keys.first()?;
        if seconds <= first.time {
            return Some(first.value);
        }
        let last = self.keys.last()?;
        if seconds >= last.time {
            return Some(last.value);
        }
        for pair in self.keys.windows(2) {
            let a = pair[0];
            let b = pair[1];
            if seconds < b.time {
                let span = b.time - a.time;
                if span.abs() <= DUPLICATE_TIME_EPSILON {
                    return Some(b.value);
                }
                let t = ((seconds - a.time) / span).clamp(0.0, 1.0);
                return Some(match a.interpolation {
                    TimelineInterpolation::Step => a.value,
                    TimelineInterpolation::Linear => a.value + (b.value - a.value) * t,
                });
            }
        }
        Some(last.value)
    }

    pub fn sort_keys(&mut self) {
        self.keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        self.keys
            .dedup_by(|a, b| (a.time - b.time).abs() <= DUPLICATE_TIME_EPSILON);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    #[serde(default = "default_duration")]
    pub duration: f32,
    #[serde(default)]
    pub tracks: Vec<TimelineTrack>,
}

fn default_duration() -> f32 {
    DEFAULT_TIMELINE_DURATION
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            duration: DEFAULT_TIMELINE_DURATION,
            tracks: Vec::new(),
        }
    }
}

impl Timeline {
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn is_active(&self) -> bool {
        !self.is_empty()
    }

    pub fn duration_seconds(&self) -> f64 {
        f64::from(
            self.duration
                .clamp(MIN_TIMELINE_DURATION, MAX_TIMELINE_DURATION),
        )
    }

    /// Clamp and sort after loading or before first use.
    pub fn normalize(&mut self) {
        self.duration = self
            .duration
            .clamp(MIN_TIMELINE_DURATION, MAX_TIMELINE_DURATION);
        for track in &mut self.tracks {
            for key in &mut track.keys {
                key.time = key.time.clamp(0.0, self.duration);
            }
            track.sort_keys();
        }
    }

    pub fn track_index(&self, parameter: &str) -> Option<usize> {
        self.tracks
            .iter()
            .position(|track| track.parameter == parameter)
    }

    pub fn add_track(&mut self, parameter: impl Into<String>) -> usize {
        let parameter = parameter.into();
        if let Some(index) = self.track_index(&parameter) {
            return index;
        }
        self.tracks.push(TimelineTrack::new(parameter));
        self.tracks.len() - 1
    }

    pub fn sample(&self, seconds: f32) -> Vec<(String, f32)> {
        self.tracks
            .iter()
            .filter_map(|track| {
                track
                    .value_at(seconds)
                    .map(|value| (track.parameter.clone(), value))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track() -> TimelineTrack {
        TimelineTrack {
            parameter: "layer/layer-1/opacity".to_owned(),
            keys: vec![
                TimelineKey {
                    time: 0.0,
                    value: 0.0,
                    interpolation: TimelineInterpolation::Linear,
                },
                TimelineKey {
                    time: 2.0,
                    value: 1.0,
                    interpolation: TimelineInterpolation::Step,
                },
                TimelineKey {
                    time: 4.0,
                    value: 0.0,
                    interpolation: TimelineInterpolation::Linear,
                },
            ],
        }
    }

    #[test]
    fn linear_and_step_keys_are_evaluated() {
        let track = track();
        assert_eq!(track.value_at(-1.0), Some(0.0));
        assert_eq!(track.value_at(0.0), Some(0.0));
        assert_eq!(track.value_at(1.0), Some(0.5));
        assert_eq!(track.value_at(2.0), Some(1.0));
        assert_eq!(track.value_at(3.0), Some(1.0));
        assert_eq!(track.value_at(4.0), Some(0.0));
        assert_eq!(track.value_at(9.0), Some(0.0));
    }

    #[test]
    fn tracks_are_sampled_by_stable_parameter_path() {
        let mut timeline = Timeline::default();
        let index = timeline.add_track("layer/layer-1/opacity");
        timeline.tracks[index].keys = track().keys;
        assert_eq!(
            timeline.sample(1.0),
            vec![("layer/layer-1/opacity".to_owned(), 0.5)]
        );
        assert_eq!(timeline.add_track("layer/layer-1/opacity"), index);
    }

    #[test]
    fn normalize_sorts_keys_and_clamps_time() {
        let mut timeline = Timeline {
            duration: 2.0,
            tracks: vec![TimelineTrack {
                parameter: "layer/layer-1/opacity".to_owned(),
                keys: vec![
                    TimelineKey {
                        time: 3.0,
                        value: 1.0,
                        interpolation: TimelineInterpolation::Linear,
                    },
                    TimelineKey {
                        time: -1.0,
                        value: 0.0,
                        interpolation: TimelineInterpolation::Linear,
                    },
                ],
            }],
        };
        timeline.normalize();
        assert_eq!(timeline.tracks[0].keys[0].time, 0.0);
        assert_eq!(timeline.tracks[0].keys[1].time, 2.0);
    }
}

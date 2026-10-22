//! Persisted parameter bindings.
//!
//! A binding maps a physical key shortcut onto a stable parameter path. The
//! application resolves the path when the key is pressed; keeping the path in
//! the project means bindings survive reordering and renaming of objects.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterBinding {
    /// Stable parameter path, for example `layer/layer-1/visible`.
    pub parameter: String,
    /// Lower-case egui key debug name, for example `space` or `f5`.
    pub key: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}

impl ParameterBinding {
    pub fn new(
        parameter: impl Into<String>,
        key: impl Into<String>,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> Self {
        Self {
            parameter: parameter.into(),
            key: key.into().to_ascii_lowercase(),
            ctrl,
            shift,
            alt,
        }
    }

    pub fn matches_key(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        self.key.eq_ignore_ascii_case(key)
            && self.ctrl == ctrl
            && self.shift == shift
            && self.alt == alt
    }

    pub fn shortcut_label(&self) -> String {
        let mut label = String::new();
        if self.ctrl {
            label.push_str("Ctrl+");
        }
        if self.shift {
            label.push_str("Shift+");
        }
        if self.alt {
            label.push_str("Alt+");
        }
        label.push_str(&pretty_key(&self.key));
        label
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MidiMode {
    #[default]
    Absolute,
    Relative,
    Toggle,
}

impl MidiMode {
    pub const ALL: [Self; 3] = [Self::Absolute, Self::Relative, Self::Toggle];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Absolute => "Absolute",
            Self::Relative => "Relative",
            Self::Toggle => "Toggle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MidiMessageKind {
    ControlChange,
    Note,
}

impl MidiMessageKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ControlChange => "CC",
            Self::Note => "Note",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiBinding {
    /// Stable parameter path, for example `layer/layer-1/opacity`.
    pub parameter: String,
    /// MIDI channel in the wire range 0..=15.
    pub channel: u8,
    pub message: MidiMessageKind,
    /// Controller or note number in the wire range 0..=127.
    pub number: u8,
    #[serde(default)]
    pub mode: MidiMode,
}

impl MidiBinding {
    pub fn new(
        parameter: impl Into<String>,
        channel: u8,
        message: MidiMessageKind,
        number: u8,
        mode: MidiMode,
    ) -> Self {
        Self {
            parameter: parameter.into(),
            channel: channel.min(15),
            message,
            number: number.min(127),
            mode,
        }
    }

    pub fn matches(&self, channel: u8, message: MidiMessageKind, number: u8) -> bool {
        self.channel == channel && self.message == message && self.number == number
    }

    pub fn source_label(&self) -> String {
        format!(
            "CH{} {} {}",
            u16::from(self.channel) + 1,
            self.message.label(),
            self.number
        )
    }
}

fn pretty_key(key: &str) -> String {
    match key.to_ascii_lowercase().as_str() {
        "space" => "Space".to_owned(),
        "enter" => "Enter".to_owned(),
        "tab" => "Tab".to_owned(),
        "escape" => "Esc".to_owned(),
        "backspace" => "Backspace".to_owned(),
        "delete" => "Delete".to_owned(),
        "home" => "Home".to_owned(),
        "end" => "End".to_owned(),
        "pageup" => "PgUp".to_owned(),
        "pagedown" => "PgDn".to_owned(),
        "arrowup" => "↑".to_owned(),
        "arrowdown" => "↓".to_owned(),
        "arrowleft" => "←".to_owned(),
        "arrowright" => "→".to_owned(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_labels_and_matching_are_stable() {
        let binding = ParameterBinding::new("layer/layer-1/visible", "Space", true, false, false);
        assert_eq!(binding.key, "space");
        assert_eq!(binding.shortcut_label(), "Ctrl+Space");
        assert!(binding.matches_key("space", true, false, false));
        assert!(!binding.matches_key("space", false, false, false));
        assert!(!binding.matches_key("enter", true, false, false));
    }

    #[test]
    fn midi_bindings_match_their_wire_identity() {
        let binding = MidiBinding::new(
            "layer/layer-1/opacity",
            0,
            MidiMessageKind::ControlChange,
            7,
            MidiMode::Absolute,
        );
        assert_eq!(binding.source_label(), "CH1 CC 7");
        assert!(binding.matches(0, MidiMessageKind::ControlChange, 7));
        assert!(!binding.matches(1, MidiMessageKind::ControlChange, 7));
        assert!(!binding.matches(0, MidiMessageKind::Note, 7));
    }

    #[test]
    fn midi_mode_and_binding_round_trip() {
        let binding = MidiBinding::new(
            "audio/volume",
            15,
            MidiMessageKind::Note,
            60,
            MidiMode::Relative,
        );
        let json = serde_json::to_string(&binding).unwrap();
        let round_trip: MidiBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(binding, round_trip);
        assert_eq!(round_trip.source_label(), "CH16 Note 60");
        assert_eq!(MidiMode::Toggle.label(), "Toggle");
    }

    #[test]
    fn special_keys_have_readable_labels() {
        assert_eq!(
            ParameterBinding::new("transport/playing", "PageUp", false, false, true)
                .shortcut_label(),
            "Alt+PgUp"
        );
        assert_eq!(
            ParameterBinding::new("transport/playing", "arrowup", false, false, false)
                .shortcut_label(),
            "↑"
        );
    }
}

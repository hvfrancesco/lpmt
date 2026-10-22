//! MIDI input adapter.
//!
//! The listener keeps one midir connection per available input port and sends
//! parsed messages to the application through a channel. MIDI support is
//! optional so the default image-only build does not need an ALSA development
//! package.

use std::sync::mpsc::{self, Receiver};

#[cfg(feature = "midi")]
use anyhow::Context;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiEvent {
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
    },
    PitchBend {
        channel: u8,
        value: u16,
    },
}

impl MidiEvent {
    pub fn from_bytes(message: &[u8]) -> Option<Self> {
        let status = *message.first()?;
        let kind = status & 0xF0;
        let channel = status & 0x0F;
        let data = |index: usize| message.get(index).copied();
        match kind {
            0x80 => Some(Self::NoteOff {
                channel,
                note: data(1)?,
            }),
            0x90 => {
                let note = data(1)?;
                let velocity = data(2)?;
                if velocity == 0 {
                    Some(Self::NoteOff { channel, note })
                } else {
                    Some(Self::NoteOn {
                        channel,
                        note,
                        velocity,
                    })
                }
            }
            0xB0 => Some(Self::ControlChange {
                channel,
                controller: data(1)?,
                value: data(2)?,
            }),
            0xE0 => Some(Self::PitchBend {
                channel,
                value: u16::from(data(1)?) | (u16::from(data(2)?) << 7),
            }),
            _ => None,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::ControlChange {
                channel,
                controller,
                value,
            } => format!("CH{} CC {controller} = {value}", u16::from(*channel) + 1),
            Self::NoteOn {
                channel,
                note,
                velocity,
            } => format!("CH{} Note {note} on ({velocity})", u16::from(*channel) + 1),
            Self::NoteOff { channel, note } => {
                format!("CH{} Note {note} off", u16::from(*channel) + 1)
            }
            Self::PitchBend { channel, value } => {
                format!("CH{} Pitch {value}", u16::from(*channel) + 1)
            }
        }
    }
}

pub struct MidiListener {
    devices: Vec<String>,
    receiver: Receiver<MidiEvent>,
    supported: bool,
    #[cfg(feature = "midi")]
    _connections: Vec<midir::MidiInputConnection<()>>,
}

impl MidiListener {
    pub fn open() -> Result<Self> {
        #[cfg(not(feature = "midi"))]
        {
            let (_sender, receiver) = mpsc::channel();
            Ok(Self {
                devices: Vec::new(),
                receiver,
                supported: false,
            })
        }

        #[cfg(feature = "midi")]
        {
            let enumerator =
                midir::MidiInput::new("LPMT").context("failed to initialise MIDI input")?;
            let ports: Vec<(String, String)> = enumerator
                .ports()
                .iter()
                .map(|port| {
                    let name = enumerator.port_name(port).unwrap_or_else(|_| port.id());
                    (port.id(), name)
                })
                .collect();
            drop(enumerator);

            let (sender, receiver) = mpsc::channel();
            let mut devices = Vec::new();
            let mut connections = Vec::new();
            for (id, name) in ports {
                let input = match midir::MidiInput::new("LPMT") {
                    Ok(input) => input,
                    Err(error) => {
                        log::warn!("MIDI input '{name}' unavailable: {error}");
                        continue;
                    }
                };
                let Some(port) = input.find_port_by_id(&id) else {
                    continue;
                };
                let sender = sender.clone();
                match input.connect(
                    &port,
                    "lpmt-midi",
                    move |_stamp, message, _| {
                        if let Some(event) = MidiEvent::from_bytes(message) {
                            let _ = sender.send(event);
                        }
                    },
                    (),
                ) {
                    Ok(connection) => {
                        devices.push(name);
                        connections.push(connection);
                    }
                    Err(error) => log::warn!("failed to connect MIDI input '{name}': {error}"),
                }
            }

            log::info!("MIDI inputs: {devices:?}");
            Ok(Self {
                devices,
                receiver,
                supported: true,
                _connections: connections,
            })
        }
    }

    pub fn is_supported(&self) -> bool {
        self.supported
    }

    pub fn devices(&self) -> &[String] {
        &self.devices
    }

    pub fn drain(&self) -> Vec<MidiEvent> {
        self.receiver.try_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_change_messages_are_parsed() {
        assert_eq!(
            MidiEvent::from_bytes(&[0xB2, 7, 99]),
            Some(MidiEvent::ControlChange {
                channel: 2,
                controller: 7,
                value: 99,
            })
        );
    }

    #[test]
    fn note_on_zero_velocity_is_note_off() {
        assert_eq!(
            MidiEvent::from_bytes(&[0x93, 60, 0]),
            Some(MidiEvent::NoteOff {
                channel: 3,
                note: 60,
            })
        );
        assert_eq!(
            MidiEvent::from_bytes(&[0x93, 60, 100]),
            Some(MidiEvent::NoteOn {
                channel: 3,
                note: 60,
                velocity: 100,
            })
        );
    }

    #[cfg(not(feature = "midi"))]
    #[test]
    fn default_build_reports_midi_unsupported() {
        let listener = MidiListener::open().unwrap();
        assert!(!listener.is_supported());
        assert!(listener.devices().is_empty());
    }

    #[test]
    fn pitch_bend_and_truncated_messages_are_handled() {
        assert_eq!(
            MidiEvent::from_bytes(&[0xE0, 0x00, 0x40]),
            Some(MidiEvent::PitchBend {
                channel: 0,
                value: 8192,
            })
        );
        assert_eq!(MidiEvent::from_bytes(&[0x90, 60]), None);
        assert_eq!(MidiEvent::from_bytes(&[]), None);
    }
}

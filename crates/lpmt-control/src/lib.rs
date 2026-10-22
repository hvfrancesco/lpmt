//! Show-control adapters for the LPMT rewrite.
//!
//! OSC is the first adapter. It receives UDP messages on a background thread
//! and turns them into stable parameter changes for the application to apply.

use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};
use lpmt_core::param::{ParameterId, ParameterValue};
use rosc::{OscMessage, OscPacket, OscType};

mod midi;
pub use midi::{MidiEvent, MidiListener};

#[derive(Debug, Clone, PartialEq)]
pub enum OscEvent {
    Parameter {
        parameter: ParameterId,
        value: ParameterValue,
        address: String,
    },
    Invalid {
        address: String,
        reason: String,
    },
    Error {
        message: String,
    },
}

/// UDP OSC receiver. Binding happens synchronously so the application can
/// report a port conflict; packet reception runs on a background thread.
pub struct OscListener {
    address: SocketAddr,
    receiver: Receiver<OscEvent>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl OscListener {
    pub fn bind(address: &str) -> Result<Self> {
        let socket = UdpSocket::bind(address)
            .with_context(|| format!("failed to bind OSC listener on {address}"))?;
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .context("failed to configure OSC socket timeout")?;
        let address = socket.local_addr().context("OSC socket has no address")?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let (sender, receiver) = mpsc::channel();

        let thread = thread::Builder::new()
            .name("lpmt-osc".to_owned())
            .spawn(move || {
                let mut buffer = [0_u8; 65_535];
                while !thread_stop.load(Ordering::Relaxed) {
                    match socket.recv_from(&mut buffer) {
                        Ok((length, _)) => match rosc::decoder::decode_udp(&buffer[..length]) {
                            Ok((_, packet)) => {
                                for event in events_from_packet(&packet) {
                                    if sender.send(event).is_err() {
                                        return;
                                    }
                                }
                            }
                            Err(error) => {
                                if sender
                                    .send(OscEvent::Error {
                                        message: format!("OSC decode failed: {error}"),
                                    })
                                    .is_err()
                                {
                                    return;
                                }
                            }
                        },
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                            ) => {}
                        Err(error) => {
                            let _ = sender.send(OscEvent::Error {
                                message: format!("OSC receive failed: {error}"),
                            });
                            return;
                        }
                    }
                }
            })
            .context("failed to start OSC listener thread")?;

        Ok(Self {
            address,
            receiver,
            stop,
            thread: Some(thread),
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Drain all messages received since the previous poll.
    pub fn drain(&self) -> Vec<OscEvent> {
        self.receiver.try_iter().collect()
    }
}

impl Drop for OscListener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn events_from_packet(packet: &OscPacket) -> Vec<OscEvent> {
    match packet {
        OscPacket::Message(message) => events_from_message(message).into_iter().collect(),
        OscPacket::Bundle(bundle) => bundle.content.iter().flat_map(events_from_packet).collect(),
    }
}

fn events_from_message(message: &OscMessage) -> Option<OscEvent> {
    let address = message.addr.clone();
    let Some(value) = message.args.first().and_then(parameter_value) else {
        return Some(OscEvent::Invalid {
            address,
            reason: "message needs a numeric, boolean or string argument".to_owned(),
        });
    };
    match ParameterId::parse(&address) {
        Ok(parameter) => Some(OscEvent::Parameter {
            parameter,
            value,
            address,
        }),
        Err(error) => Some(OscEvent::Invalid {
            address,
            reason: error.to_string(),
        }),
    }
}

fn parameter_value(value: &OscType) -> Option<ParameterValue> {
    match value {
        OscType::Int(value) => Some(ParameterValue::Int(*value)),
        OscType::Long(value) => Some(ParameterValue::Float(*value as f32)),
        OscType::Float(value) => Some(ParameterValue::Float(*value)),
        OscType::Double(value) => Some(ParameterValue::Float(*value as f32)),
        OscType::Bool(value) => Some(ParameterValue::Bool(*value)),
        OscType::String(value) => Some(ParameterValue::Text(value.clone())),
        OscType::Char(value) => Some(ParameterValue::Text(value.to_string())),
        OscType::Inf => Some(ParameterValue::Float(f32::INFINITY)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lpmt_core::model::OutputId;
    use std::time::{Duration, Instant};

    #[test]
    fn osc_message_maps_to_a_parameter() {
        let message = OscMessage {
            addr: "/lpmt/output/output-1/brightness".to_owned(),
            args: vec![OscType::Float(1.25)],
        };
        assert_eq!(
            events_from_message(&message),
            Some(OscEvent::Parameter {
                parameter: ParameterId::OutputBrightness(OutputId::new("output-1")),
                value: ParameterValue::Float(1.25),
                address: "/lpmt/output/output-1/brightness".to_owned(),
            })
        );
    }

    #[test]
    fn bundles_and_type_coercion_are_supported() {
        let packet = OscPacket::Bundle(rosc::OscBundle {
            timetag: rosc::OscTime {
                seconds: 0,
                fractional: 0,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/surface/surface-1/visible".to_owned(),
                    args: vec![OscType::Int(1)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/transport/playing".to_owned(),
                    args: vec![OscType::Bool(true)],
                }),
            ],
        });
        let events = events_from_packet(&packet);
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            OscEvent::Parameter {
                value: ParameterValue::Int(1),
                ..
            }
        ));
        assert!(matches!(
            &events[1],
            OscEvent::Parameter {
                parameter: ParameterId::TransportPlaying,
                ..
            }
        ));
    }

    #[test]
    fn listener_receives_udp_parameter_events() {
        let listener = OscListener::bind("127.0.0.1:0").expect("bind ephemeral OSC port");
        let sender = UdpSocket::bind("127.0.0.1:0").expect("bind OSC sender");
        let packet = OscPacket::Message(OscMessage {
            addr: "/lpmt/layer/layer-1/opacity".to_owned(),
            args: vec![OscType::Float(0.42)],
        });
        let bytes = rosc::encoder::encode(&packet).expect("encode OSC");
        sender
            .send_to(&bytes, listener.address())
            .expect("send OSC datagram");

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let events = listener.drain();
            if let Some(event) = events.into_iter().next() {
                assert!(matches!(
                    event,
                    OscEvent::Parameter {
                        parameter: ParameterId::LayerOpacity(_),
                        value: ParameterValue::Float(0.42),
                        ..
                    }
                ));
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for OSC event");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn unknown_paths_and_missing_values_are_reported() {
        let unknown = OscMessage {
            addr: "/lpmt/layer/layer-1/nope".to_owned(),
            args: vec![OscType::Float(1.0)],
        };
        assert!(matches!(
            events_from_message(&unknown),
            Some(OscEvent::Invalid { .. })
        ));

        let missing = OscMessage {
            addr: "/lpmt/layer/layer-1/opacity".to_owned(),
            args: Vec::new(),
        };
        assert!(matches!(
            events_from_message(&missing),
            Some(OscEvent::Invalid { .. })
        ));
    }
}

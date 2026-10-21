//! Audio playback for the shared show clock.
//!
//! `rodio` owns the output device and decoding. This wrapper keeps a single
//! source alive, exposes seek/volume controls and lets the application correct
//! its position against the show clock.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

pub struct AudioPlayer {
    /// Keeping the device sink alive keeps the output stream running.
    _stream: MixerDeviceSink,
    player: Player,
    path: PathBuf,
    duration: Option<Duration>,
    volume: f32,
    muted: bool,
    playing: bool,
}

impl AudioPlayer {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut stream = DeviceSinkBuilder::open_default_sink()
            .context("failed to open the default audio output device")?;
        stream.log_on_drop(false);
        let player = Player::connect_new(stream.mixer());
        let mut audio = Self {
            _stream: stream,
            player,
            path,
            duration: None,
            volume: 1.0,
            muted: false,
            playing: false,
        };
        audio.load_source(Duration::ZERO)?;
        audio.player.pause();
        Ok(audio)
    }

    pub fn duration_seconds(&self) -> Option<f64> {
        self.duration.map(|duration| duration.as_secs_f64())
    }

    pub fn position_seconds(&self) -> f64 {
        self.player.get_pos().as_secs_f64()
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.apply_volume();
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        self.apply_volume();
    }

    /// Follow the show clock. `force_seek` is used after an explicit transport
    /// seek or a show loop wrap, when a small circular drift would otherwise
    /// hide a real discontinuity.
    pub fn sync_to_clock(&mut self, position: f64, playing: bool, force_seek: bool) {
        let target = self.target_seconds(position);
        let duration = self
            .duration
            .map_or(f64::INFINITY, |duration| duration.as_secs_f64());
        let drift = (self.position_seconds() - target).abs();

        if !playing {
            if self.playing {
                self.player.pause();
                self.playing = false;
            }
            if force_seek || drift > 0.15 {
                self.seek_to(target, false);
            }
            return;
        }

        if target >= duration - 0.001 {
            if self.playing {
                self.player.pause();
                self.playing = false;
            }
            return;
        }

        // Keep audio within a fraction of a frame of the show clock. A
        // correction is a seek, so it is only worth doing when it would be
        // audible as lip-sync drift.
        if force_seek || self.player.empty() || drift > 0.12 {
            self.seek_to(target, true);
        } else if !self.playing {
            self.player.play();
            self.playing = true;
        }
    }

    fn target_seconds(&self, position: f64) -> f64 {
        target_seconds(position, self.duration_seconds())
    }

    fn seek_to(&mut self, target: f64, play: bool) {
        let target = Duration::from_secs_f64(target.max(0.0));
        if self.player.empty() {
            if let Err(error) = self.load_source(target) {
                log::warn!("audio source could not be restarted: {error:#}");
                return;
            }
        } else if let Err(error) = self.player.try_seek(target) {
            log::warn!("audio seek failed ({error}); reopening the source");
            if let Err(error) = self.load_source(target) {
                log::warn!("audio source could not be reloaded: {error:#}");
                return;
            }
        }

        self.playing = play;
        if play {
            self.player.play();
        } else {
            self.player.pause();
        }
    }

    fn load_source(&mut self, target: Duration) -> Result<()> {
        self.player.clear();
        let file = File::open(&self.path)
            .with_context(|| format!("failed to open audio {}", self.path.display()))?;
        let decoder = Decoder::try_from(file)
            .with_context(|| format!("failed to decode audio {}", self.path.display()))?;
        self.duration = decoder.total_duration();
        self.player.append(decoder);
        self.apply_volume();
        if let Err(error) = self.player.try_seek(target) {
            log::warn!(
                "audio source {} could not seek to {target:?}: {error}",
                self.path.display()
            );
        }
        Ok(())
    }

    fn apply_volume(&self) {
        self.player
            .set_volume(if self.muted { 0.0 } else { self.volume });
    }
}

fn target_seconds(position: f64, duration: Option<f64>) -> f64 {
    let position = if position.is_finite() {
        position.max(0.0)
    } else {
        0.0
    };
    match duration {
        Some(duration) => position.min(duration),
        None => position,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_target_is_clamped_to_the_track() {
        assert_eq!(target_seconds(-1.0, Some(3.0)), 0.0);
        assert_eq!(target_seconds(1.5, Some(3.0)), 1.5);
        assert_eq!(target_seconds(9.0, Some(3.0)), 3.0);
        assert_eq!(target_seconds(9.0, None), 9.0);
        assert_eq!(target_seconds(f64::NAN, Some(3.0)), 0.0);
    }
}

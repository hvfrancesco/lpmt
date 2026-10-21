//! The shared show clock.
//!
//! One clock is authoritative for every output and every video player. Videos
//! are resynced to it instead of free-running, and the transport UI edits this
//! clock rather than individual decoders.

use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ShowClock {
    position_seconds: f64,
    playing: bool,
    looping: bool,
    duration_seconds: Option<f64>,
    last_tick: Instant,
    seek_serial: u64,
}

impl ShowClock {
    pub fn new() -> Self {
        Self {
            position_seconds: 0.0,
            playing: false,
            looping: true,
            duration_seconds: None,
            last_tick: Instant::now(),
            seek_serial: 0,
        }
    }

    pub fn position_seconds(&self) -> f64 {
        self.position_seconds
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn is_looping(&self) -> bool {
        self.looping
    }

    pub fn duration_seconds(&self) -> Option<f64> {
        self.duration_seconds
    }

    /// Increments whenever the clock jumps to a new position, so media can
    /// distinguish a deliberate seek from a natural loop wrap.
    pub fn seek_serial(&self) -> u64 {
        self.seek_serial
    }

    /// Advance by the wall-clock time elapsed since the previous tick.
    pub fn advance(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        if !self.playing {
            return;
        }

        self.position_seconds += elapsed.as_secs_f64();
        self.normalize();
    }

    pub fn play(&mut self) {
        if self
            .duration_seconds
            .is_some_and(|duration| self.position_seconds >= duration - 0.001)
        {
            self.position_seconds = 0.0;
        }
        self.playing = true;
        self.last_tick = Instant::now();
    }

    pub fn pause(&mut self) {
        self.playing = false;
        self.last_tick = Instant::now();
    }

    pub fn toggle(&mut self) {
        if self.playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Return to the start and play.
    pub fn restart(&mut self) {
        self.position_seconds = 0.0;
        self.playing = true;
        self.last_tick = Instant::now();
        self.seek_serial = self.seek_serial.wrapping_add(1);
    }

    pub fn seek(&mut self, seconds: f64) {
        self.position_seconds = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
        if let Some(duration) = self.duration_seconds {
            self.position_seconds = self.position_seconds.min(duration);
        }
        self.last_tick = Instant::now();
        self.seek_serial = self.seek_serial.wrapping_add(1);
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    /// Set the show duration, usually the longest used video. `None` means the
    /// clock runs freely until the user pauses it.
    pub fn set_duration(&mut self, duration: Option<f64>) {
        let duration = duration.filter(|duration| duration.is_finite() && *duration > 0.0);
        if duration != self.duration_seconds {
            self.seek_serial = self.seek_serial.wrapping_add(1);
        }
        self.duration_seconds = duration;
        self.normalize();
    }

    fn normalize(&mut self) {
        let Some(duration) = self.duration_seconds else {
            return;
        };

        if self.position_seconds < duration {
            return;
        }
        if self.looping {
            self.position_seconds %= duration;
        } else {
            self.position_seconds = duration;
            self.playing = false;
        }
    }
}

impl Default for ShowClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn advances_only_while_playing() {
        let mut clock = ShowClock::new();
        clock.advance(Instant::now() + Duration::from_secs(2));
        assert_eq!(clock.position_seconds(), 0.0);

        clock.play();
        let start = Instant::now();
        clock.advance(start + Duration::from_secs(1));
        assert!((clock.position_seconds() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn loops_or_stops_at_duration() {
        let mut clock = ShowClock::new();
        clock.set_duration(Some(5.0));
        clock.play();
        let start = Instant::now();
        clock.advance(start + Duration::from_secs(7));

        assert!(clock.is_playing());
        assert!((clock.position_seconds() - 2.0).abs() < 1e-3);

        clock.set_looping(false);
        let resumed = Instant::now();
        clock.advance(resumed + Duration::from_secs(10));
        assert!(!clock.is_playing());
        assert!((clock.position_seconds() - 5.0).abs() < 1e-3);
    }

    #[test]
    fn seek_clamps_to_duration() {
        let mut clock = ShowClock::new();
        clock.set_duration(Some(5.0));
        clock.seek(-1.0);
        assert_eq!(clock.position_seconds(), 0.0);
        clock.seek(99.0);
        assert_eq!(clock.position_seconds(), 5.0);
    }

    #[test]
    fn seeks_are_distinguishable_from_loop_wraps() {
        let mut clock = ShowClock::new();
        let serial = clock.seek_serial();
        clock.set_duration(Some(5.0));
        let after_duration = clock.seek_serial();
        assert_ne!(serial, after_duration);

        clock.seek(2.0);
        assert_ne!(after_duration, clock.seek_serial());
        clock.restart();
        assert_ne!(clock.seek_serial(), 0);
    }
}

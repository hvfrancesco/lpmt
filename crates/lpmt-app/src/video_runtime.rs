//! Runtime ownership for video players.
//!
//! Video decoding is optional so the default build has no system FFmpeg
//! dependency. The rest of the application talks to this module instead of
//! branching on Cargo features.

use std::path::Path;

use lpmt_core::model::{MediaId, MediaKind, Project};
use lpmt_render::gpu::Gpu;
use lpmt_render::media::MediaCache;

#[cfg(feature = "video")]
use std::collections::HashMap;
#[cfg(feature = "video")]
use std::path::PathBuf;

pub struct VideoRuntime {
    inner: Inner,
}

impl VideoRuntime {
    pub fn new() -> Self {
        Self {
            inner: Inner::new(),
        }
    }

    /// Start players for every media asset referenced by a layer.
    pub fn load(&mut self, project: &Project, base: &Path) -> (usize, usize) {
        self.inner.load(project, base)
    }

    /// Upload the newest decoded frame for each player, if one arrived.
    pub fn poll(&mut self, gpu: &Gpu, media: &mut MediaCache) {
        self.inner.poll(gpu, media);
    }

    /// Keep every decoder aligned with the shared show clock. Videos that have
    /// drifted are corrected with a seek; paused transports get a one-frame
    /// preview so scrubbing still updates the output.
    pub fn sync_to_clock(&self, position: f64, playing: bool, looping: bool, force_seek: bool) {
        self.inner
            .sync_to_clock(position, playing, looping, force_seek);
    }

    /// Longest used video, used as the show duration until a real timeline
    /// supplies its own length.
    pub fn max_duration(&self) -> Option<f64> {
        self.inner.max_duration()
    }

    pub fn is_loaded(&self, media_id: &MediaId) -> bool {
        self.inner.is_loaded(media_id)
    }
}

impl Default for VideoRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "video")]
struct Inner {
    players: HashMap<MediaId, lpmt_media::VideoPlayer>,
    logged_first_frame: bool,
}

#[cfg(feature = "video")]
impl Inner {
    fn new() -> Self {
        Self {
            players: HashMap::new(),
            logged_first_frame: false,
        }
    }

    fn load(&mut self, project: &Project, base: &Path) -> (usize, usize) {
        self.players.clear();
        self.logged_first_frame = false;

        let mut loaded = 0_usize;
        let mut failed = 0_usize;
        for asset in project
            .media
            .iter()
            .filter(|asset| asset.kind == MediaKind::Video && project.is_media_used(&asset.id))
        {
            let path = resolve_path(base, &asset.path);
            match lpmt_media::VideoPlayer::open(&path, true) {
                Ok(player) => {
                    let info = player.info();
                    log::info!(
                        "video '{}' {}x{} @ {:.3} fps from {}",
                        asset.name,
                        info.width,
                        info.height,
                        info.fps,
                        path.display()
                    );
                    self.players.insert(asset.id.clone(), player);
                    loaded += 1;
                }
                Err(error) => {
                    failed += 1;
                    log::warn!(
                        "video '{}' could not be opened from {}: {error:#}",
                        asset.name,
                        path.display()
                    );
                }
            }
        }

        (loaded, failed)
    }

    fn poll(&mut self, gpu: &Gpu, media: &mut MediaCache) {
        let mut uploaded = false;
        for (media_id, player) in &self.players {
            if let Some(frame) = player.take_frame() {
                if let Err(error) = media.update_image(gpu, media_id.clone(), &frame.image) {
                    log::error!("failed to upload video frame for {media_id}: {error:#}");
                } else {
                    uploaded = true;
                }
            }
        }

        if uploaded && !self.logged_first_frame {
            self.logged_first_frame = true;
            log::info!("first video frame uploaded to the output texture cache");
        }
    }

    fn sync_to_clock(&self, position: f64, playing: bool, looping: bool, force_seek: bool) {
        for (media_id, player) in &self.players {
            let state = player.state();
            if state.looping != looping {
                player.set_looping(looping);
            }

            let duration = state.duration_seconds.unwrap_or(0.0);
            let target = clock_target(position, duration, looping);
            // A natural wrap is only circular while playing. A paused player
            // cannot wrap on its own, so compare raw positions instead.
            let drift = drift_seconds(state.position_seconds, target, duration, looping && playing);

            if playing {
                if !state.playing {
                    player.play();
                }
                if force_seek || drift > 0.25 {
                    log::debug!(
                        "resync video {media_id} from {:.3}s to {:.3}s (drift {drift:.3}s)",
                        state.position_seconds,
                        target
                    );
                    player.seek(target);
                }
            } else {
                if state.playing {
                    player.pause();
                }
                if force_seek || drift > 0.05 {
                    log::debug!(
                        "preview seek video {media_id} from {:.3}s to {:.3}s",
                        state.position_seconds,
                        target
                    );
                    player.seek_paused(target);
                }
            }
        }
    }

    fn max_duration(&self) -> Option<f64> {
        self.players
            .values()
            .filter_map(|player| player.info().duration_seconds)
            .reduce(f64::max)
    }

    fn is_loaded(&self, media_id: &MediaId) -> bool {
        self.players.contains_key(media_id)
    }
}

#[cfg(not(feature = "video"))]
#[derive(Default)]
struct Inner;

#[cfg(not(feature = "video"))]
impl Inner {
    fn new() -> Self {
        Self
    }

    fn load(&mut self, project: &Project, _base: &Path) -> (usize, usize) {
        let requested = project
            .media
            .iter()
            .filter(|asset| asset.kind == MediaKind::Video && project.is_media_used(&asset.id))
            .count();
        if requested > 0 {
            log::warn!(
                "{requested} video asset(s) are used by the show, but video support \
                 is not compiled in; rebuild with --features video"
            );
        }
        (0, requested)
    }

    fn poll(&mut self, _gpu: &Gpu, _media: &mut MediaCache) {}

    fn sync_to_clock(&self, _position: f64, _playing: bool, _looping: bool, _force_seek: bool) {}

    fn max_duration(&self) -> Option<f64> {
        None
    }

    fn is_loaded(&self, _media_id: &MediaId) -> bool {
        false
    }
}

#[cfg(feature = "video")]
fn resolve_path(base: &Path, path: &str) -> PathBuf {
    if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        base.join(path)
    }
}

/// Map the show-clock position into a player's own loop.
#[cfg(feature = "video")]
fn clock_target(position: f64, duration: f64, looping: bool) -> f64 {
    if duration <= 0.0 {
        position
    } else if looping {
        position.rem_euclid(duration)
    } else if position >= duration {
        duration - 0.001
    } else {
        position
    }
}

/// Circular drift for looping players, so a wrap from the end to zero is not
/// mistaken for a full-duration seek.
#[cfg(feature = "video")]
fn drift_seconds(actual: f64, target: f64, duration: f64, looping: bool) -> f64 {
    let raw = (actual - target).abs();
    if duration > 0.0 && looping {
        raw.min((duration - raw).max(0.0))
    } else {
        raw
    }
}

#[cfg(all(test, feature = "video"))]
mod tests {
    use super::*;
    use lpmt_core::model::SourceSpec;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn maps_clock_into_player_loop() {
        assert!((clock_target(7.5, 3.0, true) - 1.5).abs() < 1e-9);
        assert!((clock_target(7.5, 3.0, false) - 2.999).abs() < 1e-9);
        assert!((clock_target(1.5, 3.0, false) - 1.5).abs() < 1e-9);
        assert!((clock_target(1.5, 0.0, false) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn drift_wraps_around_loop_boundary() {
        assert!((drift_seconds(0.05, 2.95, 3.0, true) - 0.1).abs() < 1e-9);
        assert!((drift_seconds(0.05, 2.95, 3.0, false) - 2.9).abs() < 1e-9);
        assert!((drift_seconds(1.0, 1.2, 3.0, true) - 0.2).abs() < 1e-9);
    }

    #[test]
    fn clock_sync_pauses_resumes_and_reseeks_the_player() {
        let mut project = Project::demo();
        let video_id = project.media[1].id.clone();
        project.stages[0].layers[0].source = SourceSpec::Video {
            media_id: video_id.clone(),
        };

        let mut runtime = VideoRuntime::new();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(runtime.load(&project, &base), (1, 0));

        runtime.sync_to_clock(2.0, false, true, false);
        let player = runtime.inner.players.get(&video_id).unwrap();
        assert!(!player.state().playing);

        runtime.sync_to_clock(2.0, true, true, false);
        assert!(player.state().playing);

        let deadline = Instant::now() + Duration::from_secs(2);
        while player.state().position_seconds < 2.1 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(player.state().position_seconds >= 2.1);
        while player.take_frame().is_some() {}

        // A deliberate backward seek must be applied even when the wrapped
        // distance would be small, e.g. 0.1s -> 9.9s in a 10s loop.
        runtime.sync_to_clock(0.1, false, true, true);
        assert!(!player.state().playing);
        assert!((player.state().position_seconds - 0.1).abs() < 0.01);
    }
}

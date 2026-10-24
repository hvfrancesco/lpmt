//! Runtime ownership for video players.
//!
//! Video decoding is optional so the default build has no system FFmpeg
//! dependency. The rest of the application talks to this module instead of
//! branching on Cargo features.

use std::path::Path;
#[cfg(feature = "video")]
use std::time::{Duration, Instant};

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
    pub fn sync_to_clock(&mut self, position: f64, playing: bool, looping: bool, force_seek: bool) {
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
    /// Last time a seek was requested per player. Timeline scrubbing can emit
    /// a new seek every frame; coalescing these prevents decoder starvation.
    last_seek: HashMap<MediaId, Instant>,
    /// Last target handed to each player, used to discard duplicate seeks.
    last_target: HashMap<MediaId, f64>,
    /// While playing, a seek request is held until the timeline has been quiet
    /// for a moment. Seeking on every drag frame makes the decoder chase a
    /// moving target and can leave playback frozen until the drag ends.
    pending_seek: HashMap<MediaId, PendingSeek>,
    logged_first_frame: bool,
}

#[cfg(feature = "video")]
#[derive(Clone, Copy)]
struct PendingSeek {
    target: f64,
    updated: Instant,
}

#[cfg(feature = "video")]
impl Inner {
    fn new() -> Self {
        Self {
            players: HashMap::new(),
            last_seek: HashMap::new(),
            last_target: HashMap::new(),
            pending_seek: HashMap::new(),
            logged_first_frame: false,
        }
    }

    fn load(&mut self, project: &Project, base: &Path) -> (usize, usize) {
        self.players.clear();
        self.last_seek.clear();
        self.last_target.clear();
        self.pending_seek.clear();
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

    fn sync_to_clock(&mut self, position: f64, playing: bool, looping: bool, force_seek: bool) {
        let now = Instant::now();
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
            let last_target = self.last_target.get(media_id).copied();
            let target_changed =
                last_target.is_none_or(|last| (last - target).abs() > SEEK_TARGET_EPSILON);

            if playing {
                if !state.playing {
                    player.play();
                }
                if force_seek && target_changed {
                    // Defer the seek until the user stops moving the playhead.
                    // Each seek reopens the decoder; issuing one per drag frame
                    // leaves it seeking forever while the clock keeps moving.
                    if let Some(pending) = self.pending_seek.get_mut(media_id) {
                        if (pending.target - target).abs() > SEEK_TARGET_EPSILON {
                            pending.target = target;
                            pending.updated = now;
                        }
                    } else {
                        self.pending_seek.insert(
                            media_id.clone(),
                            PendingSeek {
                                target,
                                updated: now,
                            },
                        );
                    }
                }

                let pending = self.pending_seek.get(media_id).copied();
                if pending.is_some_and(|pending| {
                    now.duration_since(pending.updated) >= PLAYING_SEEK_DEBOUNCE
                }) && can_request_seek(self.last_seek.get(media_id), now, true)
                {
                    if target_changed {
                        log::debug!(
                            "resync video {media_id} from {:.3}s to {:.3}s (drift {drift:.3}s)",
                            state.position_seconds,
                            target
                        );
                        player.seek(target);
                        self.last_seek.insert(media_id.clone(), now);
                        self.last_target.insert(media_id.clone(), target);
                    }
                    self.pending_seek.remove(media_id);
                } else if pending.is_none()
                    && drift > 0.25
                    && target_changed
                    && can_request_seek(self.last_seek.get(media_id), now, true)
                {
                    let ahead = target - state.position_seconds;
                    if ahead > 0.0 && ahead <= MAX_SKIP_SECONDS {
                        log::debug!(
                            "fast-forward video {media_id} from {:.3}s to {:.3}s",
                            state.position_seconds,
                            target
                        );
                        player.skip_to(target);
                    } else {
                        log::debug!(
                            "resync video {media_id} from {:.3}s to {:.3}s (drift {drift:.3}s)",
                            state.position_seconds,
                            target
                        );
                        player.seek(target);
                    }
                    self.last_seek.insert(media_id.clone(), now);
                    self.last_target.insert(media_id.clone(), target);
                }
            } else {
                if state.playing {
                    player.pause();
                }

                if force_seek && target_changed {
                    let has_pending = self.pending_seek.contains_key(media_id);
                    let leading_ready = !has_pending
                        && self.last_seek.get(media_id).is_none_or(|last| {
                            now.duration_since(*last) >= PAUSED_SEEK_LEADING_INTERVAL
                        });
                    if leading_ready && can_request_seek(self.last_seek.get(media_id), now, false) {
                        log::debug!(
                            "preview seek video {media_id} from {:.3}s to {:.3}s",
                            state.position_seconds,
                            target
                        );
                        player.seek_paused(target);
                        self.last_seek.insert(media_id.clone(), now);
                        self.last_target.insert(media_id.clone(), target);
                        self.pending_seek.remove(media_id);
                    } else if let Some(pending) = self.pending_seek.get_mut(media_id) {
                        if (pending.target - target).abs() > SEEK_TARGET_EPSILON {
                            pending.target = target;
                            pending.updated = now;
                        }
                    } else {
                        self.pending_seek.insert(
                            media_id.clone(),
                            PendingSeek {
                                target,
                                updated: now,
                            },
                        );
                    }
                }

                let pending = self.pending_seek.get(media_id).copied();
                if pending.is_some_and(|pending| {
                    now.duration_since(pending.updated) >= PAUSED_SEEK_DEBOUNCE
                }) && can_request_seek(self.last_seek.get(media_id), now, false)
                {
                    if target_changed {
                        log::debug!(
                            "preview seek video {media_id} from {:.3}s to {:.3}s",
                            state.position_seconds,
                            target
                        );
                        player.seek_paused(target);
                        self.last_seek.insert(media_id.clone(), now);
                        self.last_target.insert(media_id.clone(), target);
                    }
                    self.pending_seek.remove(media_id);
                } else if pending.is_none()
                    && target_changed
                    && drift > 0.05
                    && can_request_seek(self.last_seek.get(media_id), now, false)
                {
                    log::debug!(
                        "preview seek video {media_id} from {:.3}s to {:.3}s",
                        state.position_seconds,
                        target
                    );
                    player.seek_paused(target);
                    self.last_seek.insert(media_id.clone(), now);
                    self.last_target.insert(media_id.clone(), target);
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

#[cfg(feature = "video")]
const PLAYING_SEEK_DEBOUNCE: Duration = Duration::from_millis(90);
#[cfg(feature = "video")]
const PAUSED_SEEK_DEBOUNCE: Duration = Duration::from_millis(80);
#[cfg(feature = "video")]
const PAUSED_SEEK_LEADING_INTERVAL: Duration = Duration::from_millis(150);
#[cfg(feature = "video")]
const SEEK_TARGET_EPSILON: f64 = 0.001;
#[cfg(feature = "video")]
const MAX_SKIP_SECONDS: f64 = 10.0;

#[cfg(feature = "video")]
fn can_request_seek(last: Option<&Instant>, now: Instant, playing: bool) -> bool {
    let interval = if playing {
        Duration::from_millis(60)
    } else {
        Duration::from_millis(30)
    };
    last.is_none_or(|last| now.duration_since(*last) >= interval)
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
    fn seek_requests_are_coalesced_per_player() {
        let now = Instant::now();
        assert!(can_request_seek(None, now, true));
        assert!(can_request_seek(
            Some(&(now - Duration::from_millis(100))),
            now,
            true
        ));
        assert!(!can_request_seek(Some(&now), now, true));
        assert!(!can_request_seek(
            Some(&(now - Duration::from_millis(20))),
            now,
            false
        ));
    }

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
    fn playing_seeks_wait_for_a_quiet_timeline() {
        let mut project = Project::demo();
        let video_id = project.media[1].id.clone();
        project.stages[0].layers[0].source = SourceSpec::Video {
            media_id: video_id.clone(),
        };

        let mut runtime = VideoRuntime::new();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(runtime.load(&project, &base), (1, 0));

        runtime.sync_to_clock(1.0, true, true, true);
        assert!(runtime.inner.pending_seek.contains_key(&video_id));
        assert_eq!(runtime.inner.last_target.get(&video_id), None);

        thread::sleep(PLAYING_SEEK_DEBOUNCE + Duration::from_millis(20));
        runtime.sync_to_clock(1.0, true, true, false);
        assert!(!runtime.inner.pending_seek.contains_key(&video_id));
        assert_eq!(runtime.inner.last_target.get(&video_id), Some(&1.0));

        // A duplicate seek to the position already requested must not reopen
        // the decoder.
        runtime.sync_to_clock(1.0, true, true, true);
        assert!(!runtime.inner.pending_seek.contains_key(&video_id));
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
        {
            let player = runtime.inner.players.get(&video_id).unwrap();
            assert!(!player.state().playing);
        }

        runtime.sync_to_clock(2.0, true, true, false);
        {
            let player = runtime.inner.players.get(&video_id).unwrap();
            assert!(player.state().playing);

            let deadline = Instant::now() + Duration::from_secs(2);
            while player.state().position_seconds < 2.1 && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            assert!(player.state().position_seconds >= 2.1);
            while player.take_frame().is_some() {}
        }

        // A deliberate backward seek must be applied even when the wrapped
        // distance would be small, e.g. 0.1s -> 9.9s in a 10s loop.
        runtime.sync_to_clock(0.1, false, true, true);
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let state = runtime.inner.players.get(&video_id).unwrap().state();
            if (state.position_seconds - 0.1).abs() < 0.01 || Instant::now() >= deadline {
                assert!(!state.playing);
                assert!((state.position_seconds - 0.1).abs() < 0.01);
                break;
            }
            runtime.sync_to_clock(0.1, false, true, false);
            thread::sleep(Duration::from_millis(10));
        }
    }
}

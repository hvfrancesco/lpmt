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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackSnapshot {
    pub playing: bool,
    pub looping: bool,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
}

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

    pub fn status(&self, media_id: &MediaId) -> Option<PlaybackSnapshot> {
        self.inner.status(media_id)
    }

    pub fn toggle_play(&self, media_id: &MediaId) {
        self.inner.toggle_play(media_id);
    }

    pub fn restart(&self, media_id: &MediaId) {
        self.inner.restart(media_id);
    }

    pub fn set_looping(&self, media_id: &MediaId, looping: bool) {
        self.inner.set_looping(media_id, looping);
    }

    pub fn seek(&self, media_id: &MediaId, position: f64) {
        self.inner.seek(media_id, position);
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

    fn status(&self, media_id: &MediaId) -> Option<PlaybackSnapshot> {
        self.players.get(media_id).map(|player| {
            let state = player.state();
            PlaybackSnapshot {
                playing: state.playing,
                looping: state.looping,
                position_seconds: state.position_seconds,
                duration_seconds: state.duration_seconds,
            }
        })
    }

    fn toggle_play(&self, media_id: &MediaId) {
        if let Some(player) = self.players.get(media_id) {
            player.toggle_play();
        }
    }

    fn restart(&self, media_id: &MediaId) {
        if let Some(player) = self.players.get(media_id) {
            player.restart();
        }
    }

    fn set_looping(&self, media_id: &MediaId, looping: bool) {
        if let Some(player) = self.players.get(media_id) {
            player.set_looping(looping);
        }
    }

    fn seek(&self, media_id: &MediaId, position: f64) {
        if let Some(player) = self.players.get(media_id) {
            player.seek(position);
        }
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

    fn status(&self, _media_id: &MediaId) -> Option<PlaybackSnapshot> {
        None
    }

    fn toggle_play(&self, _media_id: &MediaId) {}

    fn restart(&self, _media_id: &MediaId) {}

    fn set_looping(&self, _media_id: &MediaId, _looping: bool) {}

    fn seek(&self, _media_id: &MediaId, _position: f64) {}
}

#[cfg(feature = "video")]
fn resolve_path(base: &Path, path: &str) -> PathBuf {
    if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        base.join(path)
    }
}

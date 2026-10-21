//! Runtime ownership for the project audio track.
//!
//! Audio is optional so the default build has no ALSA/cpal dependency. The
//! application talks to this module instead of branching on Cargo features.

use std::path::Path;

#[cfg(feature = "audio")]
use lpmt_core::model::MediaKind;
use lpmt_core::model::Project;

#[cfg(feature = "audio")]
use std::path::PathBuf;

pub struct AudioRuntime {
    inner: Inner,
}

impl AudioRuntime {
    pub fn new() -> Self {
        Self {
            inner: Inner::new(),
        }
    }

    /// Open the project's show audio track. Returns whether a player is loaded
    /// and the asset name when loading failed.
    pub fn load(&mut self, project: &Project, base: &Path) -> (bool, Option<String>) {
        self.inner.load(project, base)
    }

    pub fn sync_to_clock(&mut self, position: f64, playing: bool, force_seek: bool) {
        self.inner.sync_to_clock(position, playing, force_seek);
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.inner.set_volume(volume);
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.inner.set_muted(muted);
    }

    pub fn max_duration(&self) -> Option<f64> {
        self.inner.max_duration()
    }

    pub fn error(&self) -> Option<&str> {
        self.inner.error()
    }
}

impl Default for AudioRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "audio")]
struct Inner {
    player: Option<lpmt_media::AudioPlayer>,
    error: Option<String>,
}

#[cfg(feature = "audio")]
impl Inner {
    fn new() -> Self {
        Self {
            player: None,
            error: None,
        }
    }

    fn load(&mut self, project: &Project, base: &Path) -> (bool, Option<String>) {
        self.player = None;
        self.error = None;

        let Some(asset) = project.audio_asset() else {
            return (false, None);
        };
        if asset.kind != MediaKind::Audio {
            let message = format!("'{}' is not an audio asset", asset.name);
            self.error = Some(message);
            return (false, Some(asset.name.clone()));
        }

        let path = resolve_path(base, &asset.path);
        match lpmt_media::AudioPlayer::open(&path) {
            Ok(mut player) => {
                player.set_volume(project.audio.volume);
                player.set_muted(project.audio.muted);
                log::info!(
                    "audio track '{}' opened from {} ({:.3}s)",
                    asset.name,
                    path.display(),
                    player.duration_seconds().unwrap_or(0.0)
                );
                self.player = Some(player);
                (true, None)
            }
            Err(error) => {
                let message = format!("audio unavailable: {error:#}");
                log::warn!(
                    "audio track '{}' could not be opened: {error:#}",
                    asset.name
                );
                self.error = Some(message);
                (false, Some(asset.name.clone()))
            }
        }
    }

    fn sync_to_clock(&mut self, position: f64, playing: bool, force_seek: bool) {
        if let Some(player) = &mut self.player {
            player.sync_to_clock(position, playing, force_seek);
        }
    }

    fn set_volume(&mut self, volume: f32) {
        if let Some(player) = &mut self.player {
            player.set_volume(volume);
        }
    }

    fn set_muted(&mut self, muted: bool) {
        if let Some(player) = &mut self.player {
            player.set_muted(muted);
        }
    }

    fn max_duration(&self) -> Option<f64> {
        self.player
            .as_ref()
            .and_then(lpmt_media::AudioPlayer::duration_seconds)
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

#[cfg(not(feature = "audio"))]
#[derive(Default)]
struct Inner;

#[cfg(not(feature = "audio"))]
impl Inner {
    fn new() -> Self {
        Self
    }

    fn load(&mut self, project: &Project, _base: &Path) -> (bool, Option<String>) {
        if project.audio_asset().is_some() {
            log::warn!(
                "the show has an audio track, but audio support is not compiled in; \
                 rebuild with --features audio"
            );
        }
        (false, None)
    }

    fn sync_to_clock(&mut self, _position: f64, _playing: bool, _force_seek: bool) {}

    fn set_volume(&mut self, _volume: f32) {}

    fn set_muted(&mut self, _muted: bool) {}

    fn max_duration(&self) -> Option<f64> {
        None
    }

    fn error(&self) -> Option<&str> {
        None
    }
}

#[cfg(feature = "audio")]
fn resolve_path(base: &Path, path: &str) -> PathBuf {
    if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        base.join(path)
    }
}

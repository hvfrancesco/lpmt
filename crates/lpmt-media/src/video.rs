//! Background video decoding with `ffmpeg-next`.
//!
//! Decoding runs on a worker thread and publishes the newest frame into a
//! single-slot mailbox. The UI thread never blocks on disk or codec work; it
//! simply takes the latest frame and uploads it to a GPU texture.
//!
//! Playback state is shared with the worker: play/pause, looping and seeks are
//! handled at packet/frame boundaries, and a seek reopens the decoder at the
//! target position. Audio, per-layer sync and hardware acceleration are left for
//! later milestones.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use ffmpeg::format::Pixel;
use ffmpeg::media::Type;
use ffmpeg::software::scaling::context::Context as ScalingContext;
use ffmpeg::software::scaling::flag::Flags;
use ffmpeg::util::frame::video::Video as RawVideoFrame;
use ffmpeg_next as ffmpeg;

use crate::DecodedImage;

#[derive(Debug, Clone, Copy)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackState {
    pub playing: bool,
    pub looping: bool,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub image: DecodedImage,
    pub pts_seconds: f64,
}

struct Shared {
    latest: Mutex<Option<VideoFrame>>,
    state: Mutex<PlaybackState>,
    seek_request: Mutex<Option<f64>>,
    stop: AtomicBool,
}

impl Shared {
    fn new(looping: bool, duration_seconds: Option<f64>) -> Self {
        Self {
            latest: Mutex::new(None),
            state: Mutex::new(PlaybackState {
                playing: true,
                looping,
                position_seconds: 0.0,
                duration_seconds,
            }),
            seek_request: Mutex::new(None),
            stop: AtomicBool::new(false),
        }
    }

    fn state(&self) -> PlaybackState {
        *lock(&self.state)
    }

    fn update_state(&self, update: impl FnOnce(&mut PlaybackState)) {
        update(&mut lock(&self.state));
    }

    fn set_position(&self, seconds: f64) {
        self.update_state(|state| state.position_seconds = seconds.max(0.0));
    }

    fn request_seek(&self, seconds: f64) {
        let duration = self.state().duration_seconds;
        let position = clamp_position(seconds, duration);
        self.update_state(|state| {
            state.position_seconds = position;
            state.playing = true;
        });
        *lock(&self.seek_request) = Some(position);
    }

    fn take_seek(&self) -> Option<f64> {
        lock(&self.seek_request).take()
    }

    fn store_frame(&self, frame: VideoFrame) {
        *lock(&self.latest) = Some(frame);
    }

    fn take_frame(&self) -> Option<VideoFrame> {
        lock(&self.latest).take()
    }

    fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    fn is_playing(&self) -> bool {
        lock(&self.state).playing
    }

    fn is_looping(&self) -> bool {
        lock(&self.state).looping
    }
}

pub struct VideoPlayer {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
    info: VideoInfo,
}

impl VideoPlayer {
    /// Open a video and start decoding in the background.
    pub fn open(path: impl AsRef<Path>, looping: bool) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let info = probe(&path)?;
        let shared = Arc::new(Shared::new(looping, info.duration_seconds));

        let thread_shared = Arc::clone(&shared);
        let thread_path = path.clone();
        let thread_name = format!(
            "lpmt-video-{}",
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("player")
        );
        let thread = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                log::debug!("video worker started for {}", thread_path.display());
                if let Err(error) = run_player(&thread_path, thread_shared.clone()) {
                    thread_shared.update_state(|state| state.playing = false);
                    if !thread_shared.is_stopped() {
                        log::error!(
                            "video playback stopped for {}: {error:#}",
                            thread_path.display()
                        );
                    }
                }
            })
            .context("failed to spawn video decoder thread")?;

        Ok(Self {
            shared,
            thread: Some(thread),
            info,
        })
    }

    pub fn info(&self) -> VideoInfo {
        self.info
    }

    pub fn state(&self) -> PlaybackState {
        self.shared.state()
    }

    /// Take the newest decoded frame, if a new one arrived since the last call.
    pub fn take_frame(&self) -> Option<VideoFrame> {
        self.shared.take_frame()
    }

    pub fn play(&self) {
        let state = self.state();
        if state
            .duration_seconds
            .is_some_and(|duration| state.position_seconds >= duration - 0.001)
        {
            self.seek(0.0);
        } else {
            self.shared.update_state(|state| state.playing = true);
        }
    }

    pub fn pause(&self) {
        self.shared.update_state(|state| state.playing = false);
    }

    pub fn toggle_play(&self) {
        if self.state().playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Seek to a position in seconds. Seeking resumes a paused player.
    pub fn seek(&self, seconds: f64) {
        self.shared.request_seek(seconds);
    }

    pub fn restart(&self) {
        self.seek(0.0);
    }

    pub fn set_looping(&self, looping: bool) {
        self.shared.update_state(|state| state.looping = looping);
    }

    pub fn stop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn init_ffmpeg() -> Result<()> {
    static INIT: OnceLock<Result<(), String>> = OnceLock::new();
    let result = INIT.get_or_init(|| {
        ffmpeg::init().map_err(|error| error.to_string())?;
        ffmpeg::log::set_level(ffmpeg::log::Level::Warning);
        Ok(())
    });

    result
        .as_ref()
        .map_err(|error| anyhow::anyhow!("failed to initialize FFmpeg: {error}"))?;
    Ok(())
}

fn probe(path: &Path) -> Result<VideoInfo> {
    init_ffmpeg()?;
    let input = ffmpeg::format::input(path)
        .with_context(|| format!("failed to open video {}", path.display()))?;
    let stream = input
        .streams()
        .best(Type::Video)
        .context("file has no video stream")?;
    let decoder = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
        .context("failed to create video decoder context")?
        .decoder()
        .video()
        .context("failed to open video decoder")?;

    let fps = decoder
        .frame_rate()
        .map(f64::from)
        .unwrap_or_else(|| f64::from(stream.avg_frame_rate()));
    let fps = if fps.is_finite() && fps > 0.0 {
        fps.clamp(1.0, 240.0)
    } else {
        30.0
    };

    let duration_seconds = if stream.duration() > 0 {
        Some(stream.duration() as f64 * f64::from(stream.time_base()))
    } else if input.duration() > 0 {
        Some(input.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE))
    } else {
        None
    };

    Ok(VideoInfo {
        width: decoder.width(),
        height: decoder.height(),
        fps,
        duration_seconds,
    })
}

fn run_player(path: &Path, shared: Arc<Shared>) -> Result<()> {
    let mut start = 0.0;
    let mut next_frame = None;

    loop {
        match decode_once(path, &shared, start, &mut next_frame)? {
            DecodeOutcome::Eof => {
                if shared.is_stopped() {
                    break;
                }

                if shared.is_looping() {
                    start = 0.0;
                    next_frame = None;
                    shared.set_position(0.0);
                    continue;
                }

                let duration = shared.state().duration_seconds.unwrap_or(0.0);
                shared.update_state(|state| {
                    state.position_seconds = duration;
                    state.playing = false;
                });

                match wait_for_play(&shared) {
                    WaitOutcome::Playing(_) => {
                        start = 0.0;
                        next_frame = None;
                        shared.set_position(0.0);
                    }
                    WaitOutcome::Seek(position) => {
                        start = position;
                        next_frame = None;
                    }
                    WaitOutcome::Stopped => break,
                }
            }
            DecodeOutcome::Seek(position) => {
                start = position;
                next_frame = None;
                shared.set_position(position);
            }
            DecodeOutcome::Stopped => break,
        }
    }

    Ok(())
}

fn decode_once(
    path: &Path,
    shared: &Shared,
    start: f64,
    next_frame: &mut Option<Instant>,
) -> Result<DecodeOutcome> {
    match wait_for_play(shared) {
        WaitOutcome::Playing(waited) => {
            if waited {
                *next_frame = None;
            }
        }
        WaitOutcome::Seek(position) => return Ok(DecodeOutcome::Seek(position)),
        WaitOutcome::Stopped => return Ok(DecodeOutcome::Stopped),
    }

    let mut input = ffmpeg::format::input(path)
        .with_context(|| format!("failed to open video {}", path.display()))?;
    let video_stream = input
        .streams()
        .best(Type::Video)
        .context("file has no video stream")?;
    let stream_index = video_stream.index();
    let time_base = f64::from(video_stream.time_base());
    anyhow::ensure!(time_base > 0.0, "video stream has an invalid time base");
    let codec_context = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
        .context("failed to create video decoder context")?;
    let mut decoder = codec_context
        .decoder()
        .video()
        .context("failed to open video decoder")?;

    let width = decoder.width();
    let height = decoder.height();
    let fps = decoder
        .frame_rate()
        .map(f64::from)
        .unwrap_or_else(|| f64::from(video_stream.avg_frame_rate()));
    let fps = if fps.is_finite() && fps > 0.0 {
        fps.clamp(1.0, 240.0)
    } else {
        30.0
    };
    let interval = Duration::from_secs_f64(1.0 / fps);

    let mut scaler = ScalingContext::get(
        decoder.format(),
        width,
        height,
        Pixel::RGBA,
        width,
        height,
        Flags::BILINEAR,
    )
    .context("failed to create RGBA video scaler")?;

    let start = start.max(0.0);
    if start > 0.0 {
        let timestamp = (start / time_base).round() as i64;
        input
            .seek(timestamp, ..timestamp)
            .with_context(|| format!("failed to seek video to {start:.3}s"))?;
        decoder.flush();
        *next_frame = None;
    }

    log::debug!(
        "video decoder opened {}x{} at {:.3} fps from {:.3}s",
        width,
        height,
        fps,
        start
    );

    for (stream, packet) in input.packets() {
        if shared.is_stopped() {
            return Ok(DecodeOutcome::Stopped);
        }
        if let Some(position) = shared.take_seek() {
            return Ok(DecodeOutcome::Seek(position));
        }
        match wait_for_play(shared) {
            WaitOutcome::Playing(waited) => {
                if waited {
                    *next_frame = None;
                }
            }
            WaitOutcome::Seek(position) => return Ok(DecodeOutcome::Seek(position)),
            WaitOutcome::Stopped => return Ok(DecodeOutcome::Stopped),
        }

        if stream.index() != stream_index {
            continue;
        }

        decoder.send_packet(&packet)?;
        match receive_frames(
            &mut decoder,
            &mut scaler,
            shared,
            next_frame,
            interval,
            time_base,
            start,
        )? {
            FrameOutcome::Continue => {}
            FrameOutcome::Seek(position) => return Ok(DecodeOutcome::Seek(position)),
            FrameOutcome::Stopped => return Ok(DecodeOutcome::Stopped),
        }
    }

    let _ = decoder.send_eof();
    match receive_frames(
        &mut decoder,
        &mut scaler,
        shared,
        next_frame,
        interval,
        time_base,
        start,
    )? {
        FrameOutcome::Continue => Ok(DecodeOutcome::Eof),
        FrameOutcome::Seek(position) => Ok(DecodeOutcome::Seek(position)),
        FrameOutcome::Stopped => Ok(DecodeOutcome::Stopped),
    }
}

enum DecodeOutcome {
    Eof,
    Seek(f64),
    Stopped,
}

enum FrameOutcome {
    Continue,
    Seek(f64),
    Stopped,
}

enum WaitOutcome {
    /// `true` when playback was paused and has just resumed.
    Playing(bool),
    Seek(f64),
    Stopped,
}

fn wait_for_play(shared: &Shared) -> WaitOutcome {
    if shared.is_stopped() {
        return WaitOutcome::Stopped;
    }
    if let Some(position) = shared.take_seek() {
        return WaitOutcome::Seek(position);
    }
    if shared.is_playing() {
        return WaitOutcome::Playing(false);
    }

    loop {
        thread::sleep(Duration::from_millis(10));
        if shared.is_stopped() {
            return WaitOutcome::Stopped;
        }
        if let Some(position) = shared.take_seek() {
            return WaitOutcome::Seek(position);
        }
        if shared.is_playing() {
            return WaitOutcome::Playing(true);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn receive_frames(
    decoder: &mut ffmpeg::decoder::Video,
    scaler: &mut ScalingContext,
    shared: &Shared,
    next_frame: &mut Option<Instant>,
    interval: Duration,
    time_base: f64,
    start: f64,
) -> Result<FrameOutcome> {
    let mut decoded = RawVideoFrame::empty();
    while decoder.receive_frame(&mut decoded).is_ok() {
        match wait_for_play(shared) {
            WaitOutcome::Playing(waited) => {
                if waited {
                    *next_frame = None;
                }
            }
            WaitOutcome::Seek(position) => return Ok(FrameOutcome::Seek(position)),
            WaitOutcome::Stopped => return Ok(FrameOutcome::Stopped),
        }

        let pts_seconds = decoded.pts().map(|pts| pts as f64 * time_base);
        if let Some(pts) = pts_seconds {
            if pts + 0.001 < start {
                continue;
            }
        }
        let pts_seconds = pts_seconds.unwrap_or(start);

        let mut rgba = RawVideoFrame::empty();
        scaler.run(&decoded, &mut rgba)?;
        let width = rgba.width();
        let height = rgba.height();
        let pixels = copy_rgba_rows(rgba.data(0), rgba.stride(0), width, height)?;
        let image = DecodedImage::from_rgba8(width, height, pixels)?;
        shared.store_frame(VideoFrame { image, pts_seconds });
        shared.set_position(pts_seconds);

        pace(next_frame, interval);
    }

    Ok(FrameOutcome::Continue)
}

fn pace(next_frame: &mut Option<Instant>, interval: Duration) {
    let now = Instant::now();
    let target = next_frame.unwrap_or(now);
    if target > now {
        thread::sleep(target - now);
    }
    *next_frame = Some(target.max(now) + interval);
}

fn clamp_position(seconds: f64, duration: Option<f64>) -> f64 {
    let seconds = if seconds.is_finite() {
        seconds.max(0.0)
    } else {
        0.0
    };
    duration.map_or(seconds, |duration| seconds.min(duration.max(0.0)))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn copy_rgba_rows(data: &[u8], stride: usize, width: u32, height: u32) -> Result<Vec<u8>> {
    let row_bytes = width as usize * 4;
    anyhow::ensure!(
        stride >= row_bytes,
        "RGBA row stride is smaller than the row"
    );
    let last_row_start = (height as usize)
        .checked_sub(1)
        .map_or(0, |row| row * stride);
    anyhow::ensure!(
        data.len() >= last_row_start + row_bytes,
        "decoded frame buffer is too small"
    );

    let mut pixels = Vec::with_capacity(row_bytes * height as usize);
    for row in 0..height as usize {
        let start = row * stride;
        pixels.extend_from_slice(&data[start..start + row_bytes]);
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_rgba_rows_with_stride_padding() {
        let data = [
            1, 2, 3, 4, 99, 99, // first row, two padding bytes
            5, 6, 7, 8, 99, 99, // second row, two padding bytes
        ];
        let pixels = copy_rgba_rows(&data, 6, 1, 2).unwrap();
        assert_eq!(pixels, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn rejects_short_frame_buffer() {
        assert!(copy_rgba_rows(&[0; 8], 8, 3, 1).is_err());
    }

    #[test]
    fn clamps_seek_positions() {
        assert_eq!(clamp_position(-1.0, Some(10.0)), 0.0);
        assert_eq!(clamp_position(4.0, Some(10.0)), 4.0);
        assert_eq!(clamp_position(12.0, Some(10.0)), 10.0);
        assert_eq!(clamp_position(12.0, None), 12.0);
        assert_eq!(clamp_position(f64::NAN, Some(10.0)), 0.0);
    }
}

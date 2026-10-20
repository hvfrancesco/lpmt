use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use lpmt_media::VideoPlayer;

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/video/fingers.mov".to_owned());

    let mut player = VideoPlayer::open(&path, true)?;
    println!("opened {path}");
    println!("initial state: {:?}", player.state());

    let mut first_frame_taken = false;
    wait_until(Duration::from_secs(2), || {
        if player.take_frame().is_some() {
            first_frame_taken = true;
        }
        first_frame_taken
    })
    .context("no first frame")?;

    player.pause();
    std::thread::sleep(Duration::from_millis(60));
    while player.take_frame().is_some() {}
    let paused_at = player.state();
    std::thread::sleep(Duration::from_millis(350));
    let still_paused = player.state();
    if player.take_frame().is_some() {
        bail!("a frame arrived while paused");
    }
    anyhow::ensure!(
        (still_paused.position_seconds - paused_at.position_seconds).abs() < 0.02,
        "position advanced while paused: {paused_at:?} -> {still_paused:?}"
    );
    println!("pause ok at {:.3}s", paused_at.position_seconds);

    player.play();
    let resumed_at = paused_at.position_seconds;
    wait_until(Duration::from_secs(2), || {
        player.state().position_seconds > resumed_at + 0.05
    })
    .context("playback did not resume")?;
    println!("resume ok at {:.3}s", player.state().position_seconds);

    player.seek(3.0);
    wait_until(Duration::from_secs(2), || {
        player.state().position_seconds >= 2.9 && player.state().playing
    })
    .context("seek to 3.0s did not take effect")?;
    println!("seek ok at {:.3}s", player.state().position_seconds);

    player.set_looping(false);
    player.seek(6.8);
    wait_until(Duration::from_secs(3), || !player.state().playing)
        .context("non-looping playback did not stop at the end")?;
    let ended = player.state();
    println!("end ok at {:.3}s", ended.position_seconds);

    player.play();
    wait_until(Duration::from_secs(2), || {
        let state = player.state();
        state.playing && state.position_seconds < 1.0
    })
    .context("play at end did not restart from the beginning")?;
    println!("restart ok at {:.3}s", player.state().position_seconds);

    player.stop();
    println!("playback controls ok");
    Ok(())
}

fn wait_until(timeout: Duration, mut condition: impl FnMut() -> bool) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    bail!("timed out after {timeout:?}")
}

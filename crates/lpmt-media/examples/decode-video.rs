use std::time::{Duration, Instant};

use lpmt_media::VideoPlayer;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/video/fingers.mov".to_owned());

    let mut player = VideoPlayer::open(&path, false)?;
    let info = player.info();
    println!(
        "{}: {}x{}, {:.3} fps, duration {:?}s",
        path, info.width, info.height, info.fps, info.duration_seconds
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut frames = 0_usize;
    let mut last = None;
    while Instant::now() < deadline {
        if let Some(frame) = player.take_frame() {
            frames += 1;
            last = Some(frame);
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    anyhow::ensure!(frames > 0, "no frames were decoded");
    if let Some(frame) = last {
        println!(
            "decoded {frames} frames, last {}x{} at {:.3}s",
            frame.image.width, frame.image.height, frame.pts_seconds
        );
    }
    player.stop();
    Ok(())
}

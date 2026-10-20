# LPMT 2 — Rust rewrite (work in progress)

LPMT 2 is the modern Rust/GPU rewrite of **LPMT — Little Projection-Mapping Tool**.
It keeps the projection-mapping workflow of the original C++/openFrameworks
application while replacing the fixed-capacity, single-process architecture with a
real editor, independent output windows and a portable rendering stack.

The original LPMT 1 source and data are preserved in
[`archive/lpmt-1/`](archive/lpmt-1/) as the behavioral reference.

**Current status (2026-10-21):** Linux-first M0/M1 vertical slice plus ordered
layer compositing and optional video playback.

- One event loop, one `wgpu` device and queue, two windows.
- `egui` editor with a stage preview, draggable surface corners and numeric corner
  fields.
- Independent output window rendering an image or looping video through a
  four-corner projective warp, with a checker test card as fallback.
- Ordered layer stack composited bottom-to-top with per-layer visibility,
  opacity, surface assignment and `Normal`, `Add`, `Screen` or `Multiply`
  blending. The demo has two warped surfaces and two layers.
- Core project model (`Project`, `Stage`, `Surface`, `Layer`, `MediaAsset`,
  `OutputSpec`) with stable IDs and JSON serialization.
- Image decoding in `lpmt-media` and GPU texture upload/caching in `lpmt-render`.
- Optional `ffmpeg-next` video decoding behind the `video` Cargo feature:
  background worker thread, looped playback, play/pause, restart, seek/scrub,
  newest-frame mailbox and in-place GPU texture updates.
- Project save/load from the editor (path field plus Save/Load buttons). Media
  paths are resolved relative to the project file.
- Startup project loading with `cargo run -p lpmt-app -- --project show.json`.
- Homography, media decoding and JSON compatibility unit tests.
- The output opens fullscreen on a secondary monitor when one is present; otherwise
  it opens as a 1280×720 window.

### Next session

The next implementation slice is **multiple output views**:

- open more than one output window and map each to a stable `OutputId`;
- render each output through its own `OutputSpec.view` rectangle of the stage;
- add per-output geometry/colour correction and edge blending after that.

Audio plus a shared show clock are the other open M2 items. The legacy XML importer
remains planned; the full roadmap is in
[`docs/rewrite-rfc.md`](docs/rewrite-rfc.md).

## Build and run

Requirements:

- Rust 1.95 or newer.
- Linux with Vulkan (or another `wgpu` backend), Wayland or X11.
- On macOS and Windows the same workspace is intended to build unchanged; those
  platforms are not validated yet.

```bash
cargo run -p lpmt-app
# open a saved show directly
cargo run -p lpmt-app -- --project show.json
```

The demo show loads [`assets/images/ofTheo.jpg`](assets/images/ofTheo.jpg) on the
background layer and [`assets/images/car.jpg`](assets/images/car.jpg) on a smaller
`Screen`-blended layer. Missing assets fall back to the procedural checker card.
Use the **Layers** section to select a layer, toggle it, change opacity or blend
mode, move it between surfaces, reorder it, or add/delete test-pattern layers.

### Video playback

Video is an optional feature because it links against the system FFmpeg libraries.
The default build stays image-only and has no system media dependency.

On Debian/Ubuntu:

```bash
sudo apt install ffmpeg libavcodec-dev libavformat-dev libavutil-dev \
  libswscale-dev libclang-dev pkg-config
cargo run -p lpmt-app --features video
```

On macOS, install FFmpeg and pkg-config with Homebrew:

```bash
brew install ffmpeg pkg-config
cargo run -p lpmt-app --features video
```

Windows support for the video feature is not validated yet.

In the editor's **Media** section, select `fingers · Video` and click **Use for
layer**. Playback loops automatically and a **Playback** panel appears with
play/pause, restart, loop and scrub controls. Seeking resumes a paused player.
Without the `video` feature the output falls back to the checker card and the
status bar reports that video support is not compiled in.

The initial backend is `ffmpeg-next`; GStreamer remains a later option for hardware
decode and pipeline-oriented capture.

Useful commands:

```bash
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
```

The first run compiles `wgpu`, `winit` and `egui`, so it can take a few minutes.

## Repository layout

```text
crates/          Rust workspace and the LPMT 2 application
assets/          Small sample media used by demos and tests
docs/            Architecture and rewrite RFC
archive/lpmt-1/  Original C++/openFrameworks application (reference only)
```

| Crate | Responsibility |
|---|---|
| `lpmt-core` | Domain model, stable IDs, geometry and homography math |
| `lpmt-render` | `wgpu` device setup and output rendering |
| `lpmt-ui` | `egui` editor, emitting actions rather than mutating runtime state |
| `lpmt-media` | Image decoding and optional `ffmpeg-next` video playback; capture next |
| `lpmt-control` | Future OSC, MIDI, DMX and hotkey adapters |
| `lpmt-import-legacy` | Future LPMT 1 XML importer |
| `lpmt-app` | Window/event-loop composition and application state |

The architecture, output topology and milestones are documented in
[`docs/rewrite-rfc.md`](docs/rewrite-rfc.md). The short version: **one virtual Stage
per show, many Outputs as views of that Stage**.

In the editor, select a surface to drag or numerically edit its corners. The
**Layers** section selects the active layer and edits its visibility, opacity,
blend mode, surface, order and lifetime. Use the **Media** section to change a
media path, reload it, assign it to the active layer, or control video playback.
Use the Project file section to save or load the JSON show file.

## Legacy LPMT 1 archive

The original C++/openFrameworks application is preserved in
[`archive/lpmt-1/`](archive/lpmt-1/). It includes the original source, sample
media, shaders, OSC documentation and build files. It is **not** part of the Cargo
workspace and is not built by CI.

See [`archive/README.md`](archive/README.md) for the archive layout and
[`archive/lpmt-1/README.md`](archive/lpmt-1/README.md) for the original feature
list and workflow notes.

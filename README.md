# LPMT 2 — Rust rewrite (work in progress)

LPMT 2 is the modern Rust/GPU rewrite of **LPMT — Little Projection-Mapping Tool**.
It keeps the projection-mapping workflow of the original C++/openFrameworks
application while replacing the fixed-capacity, single-process architecture with a
real editor, independent output windows and a portable rendering stack.

The original LPMT 1 source and data are preserved in
[`archive/lpmt-1/`](archive/lpmt-1/) as the behavioral reference.

**Current status (2026-10-21):** Linux-first M0/M1 vertical slice plus ordered
layer compositing, optional video and audio playback, a shared show clock,
multiple output windows and grouped output presentation.

- One event loop, one `wgpu` device and queue, one editor window and any number
  of independent output windows.
- `egui` editor with a stage preview, draggable surface corners and numeric corner
  fields. Surfaces and layers can be renamed; surfaces can be added or deleted
  when no layer uses them; surface visibility/opacity and layer
  visibility/opacity/blend/surface are editable.
- Each output is mapped to a stable `OutputId` and renders its own
  `OutputSpec.view` rectangle of the stage. The editor can add, delete, open,
  close and fullscreen outputs, edit their normalized view rectangles and pick
  an explicit display for fullscreen outputs. Output views are drawn in the
  stage preview so overlap is visible.
- Outputs can be collected into **output groups**. Group members are rendered
  from the same frame state, submitted in a single queue call and presented
  together; a full-stage confidence monitor can stay outside the group and keep
  running independently.
- Outputs render images or looping video through a four-corner projective warp,
  with a checker test card as fallback.
- Each output renders through an offscreen scene pass followed by a composite
  pass that applies per-edge blending (left/right/top/bottom feather) and
  per-output brightness, contrast and gamma. This is the base for projector
  seam matching.
- Ordered layer stack composited bottom-to-top with per-layer visibility,
  opacity, surface assignment and `Normal`, `Add`, `Screen` or `Multiply`
  blending. The demo has two warped surfaces and two layers.
- Core project model (`Project`, `Stage`, `Surface`, `Layer`, `MediaAsset`,
  `OutputSpec`, `OutputGroup`) with stable IDs and JSON serialization.
- Image decoding in `lpmt-media`; `lpmt-render` owns one shared GPU texture cache
  (`MediaCache`) so every output samples the same decoded media without
  duplicating textures.
- Optional `ffmpeg-next` video decoding behind the `video` Cargo feature:
  background worker thread, looped playback, newest-frame mailbox and in-place
  GPU texture updates.
- Optional `rodio`/Symphonia audio playback behind the `audio` Cargo feature.
  The project has one show-audio track with volume and mute; it follows the
  shared clock, restarts on a show loop and stays silent after its own end when
  a longer video defines the show duration.
- One **show clock** is authoritative for every output, video player and the
  show audio track. The Inspector transport (play/pause, restart, loop, scrub)
  drives it; players are paused, resumed and resynced to the clock, and a paused
  scrub decodes one preview frame so the output still updates.
- Project save/load from the editor, with typed paths and native file pickers
  (Open…, Save as…, Browse… for media). Media paths are resolved relative to the
  project file; picked files inside the show directory are stored relative.
- Startup project loading with `cargo run -p lpmt-app -- --project show.json`.
- Homography, media decoding and JSON compatibility unit tests.
- Fullscreen outputs claim an unused secondary monitor when one is present; the
  next outputs open as windows (960×540 or 1280×720) so the editor stays
  reachable. With a single display the first fullscreen output uses it.

### Next session

The next implementation slice is **editor history and preview**:

- undo/redo and atomic saves;
- a preview viewport independent from the outputs;
- output pacing diagnostics and calibration LUTs.

Audio is now on the shared clock; a per-layer mix bus and timeline remain later
work. The legacy XML importer remains planned; the full roadmap is in
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

File pickers use the desktop portal on Linux; the typed path fields remain
available as a fallback in minimal window managers without a portal.

The demo show loads [`assets/images/ofTheo.jpg`](assets/images/ofTheo.jpg) on the
background layer and [`assets/images/car.jpg`](assets/images/car.jpg) on a smaller
`Screen`-blended layer. Missing assets fall back to the procedural checker card.
Use the **Layers** section to select a layer, toggle it, change opacity or blend
mode, move it between surfaces, reorder it, or add/delete test-pattern layers.

The demo also defines two outputs: **Main output** (full stage, fullscreen on a
secondary display when available) and **Right half** (windowed, view
`x=0.5..1.0`). The **Outputs** section adds or deletes outputs, toggles their
window, chooses fullscreen and an explicit display, edits the normalized view
rectangle, applies `Full stage` / `Left half` / `Right half` presets, and sets
edge blending plus brightness/contrast/gamma. The **Output groups** section
assigns outputs to synchronized groups; the demo puts both outputs in
**Main wall**. The stage preview outlines every open output view.

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
layer**. Video assets are opened when a layer uses them; unused video entries do
not start a decoder.

Playback is controlled by the **Transport** section at the top of the Inspector,
not by individual assets. The transport starts paused: press **Play** to advance
the shared show clock. Every used video follows that clock, is paused/resumed
with it and is seeked when it drifts. **Restart** returns to zero and plays;
**Loop** repeats the show at its duration (the longest used video until a real
timeline exists). Scrubbing while paused decodes a single preview frame at the
new position. **Space** toggles play/pause and **Home** restarts when no text
field has focus.

**Add media…** imports a new image, video or audio asset. The file picker
accepts all three types even when replacing an existing asset; the kind is
inferred from the file extension. Referencing layers follow a change between
image and video; if an asset becomes audio, those layers fall back to the test
card. Unused assets can be removed; assets referenced by a layer or used as the
show track must be reassigned first.

#### If video does not play

- **Start the transport.** The show clock starts paused; press **Play** in the
  Inspector. A video that is assigned to a layer but not visible may also be
  behind another layer or outside its surface.
- **Check the build feature.** A default `cargo run -p lpmt-app` has no video
  support. The Media section shows an amber warning and the status bar names the
  failed assets with `rebuild with --features video`. Use
  `cargo run -p lpmt-app --features video`.
- **Check the path.** Media paths are resolved relative to the project file, not
  the current directory. The status bar lists failed asset names; the log shows
  the resolved path.
- **Check the file itself** with
  `ffprobe assets/video/fingers.mov`, or run the decoder smoke test:
  `cargo run -p lpmt-media --features video --example decode-video -- assets/video/fingers.mov`.
- Without the `video` feature the output falls back to the checker card.

The initial backend is `ffmpeg-next`; GStreamer remains a later option for hardware
decode and pipeline-oriented capture.

### Audio playback

Audio is also optional, so the default build has no ALSA/cpal dependency. It
decodes MP3, WAV, FLAC, Ogg/Vorbis and AAC/M4A through Symphonia.

On Debian/Ubuntu:

```bash
sudo apt install libasound2-dev pkg-config
cargo run -p lpmt-app --features audio
```

Use `--features video,audio` for a show with both. macOS uses CoreAudio without
extra packages; Windows support is not validated yet.

Select an asset in **Media**, then either click **Use as show audio** or choose
it from the **Audio** section. Set **Volume** and **Mute** there. The track is
controlled only by the shared **Transport**: it starts paused, follows
play/pause and scrubs, restarts on **Restart**, and wraps with **Loop**. If the
audio is shorter than the longest video, it plays once and restarts at the next
show loop.

Audio assets cannot be assigned to visual layers. A default build warns that
audio support is missing and suggests `rebuild with --features audio`; if the
output device cannot be opened, the show still opens and the Audio section
shows the device error.

Useful commands:

```bash
cargo test --workspace
cargo test -p lpmt-app --features video,audio
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
| `lpmt-render` | `wgpu` device setup, shared media textures and output rendering |
| `lpmt-ui` | `egui` editor, emitting actions rather than mutating runtime state |
| `lpmt-media` | Image decoding plus optional `ffmpeg-next` video and `rodio` audio playback; capture next |
| `lpmt-control` | Future OSC, MIDI, DMX and hotkey adapters |
| `lpmt-import-legacy` | Future LPMT 1 XML importer |
| `lpmt-app` | Window/event-loop composition and application state |

The architecture, output topology and milestones are documented in
[`docs/rewrite-rfc.md`](docs/rewrite-rfc.md). The short version: **one virtual Stage
per show, many Outputs as views of that Stage**.

In the editor, select a surface to drag or numerically edit its corners. The
**Layers** section selects the active layer and edits its visibility, opacity,
blend mode, surface, order and lifetime. Use the **Media** section to change a
media path, reload it or assign images/videos to the active layer; audio assets
are assigned to the show track in the **Audio** section. The **Transport**
section at the top plays, pauses, restarts, loops and scrubs the shared show
clock, which also drives the show audio. The **Outputs** section manages output
windows and their stage views. Use the Project file section to save or load the
JSON show file.

## Legacy LPMT 1 archive

The original C++/openFrameworks application is preserved in
[`archive/lpmt-1/`](archive/lpmt-1/). It includes the original source, sample
media, shaders, OSC documentation and build files. It is **not** part of the Cargo
workspace and is not built by CI.

See [`archive/README.md`](archive/README.md) for the archive layout and
[`archive/lpmt-1/README.md`](archive/lpmt-1/README.md) for the original feature
list and workflow notes.

# LPMT 2 — Rust rewrite (work in progress)

LPMT 2 is the modern Rust/GPU rewrite of **LPMT — Little Projection-Mapping Tool**.
It keeps the projection-mapping workflow of the original C++/openFrameworks
application while replacing the fixed-capacity, single-process architecture with a
real editor, independent output windows and a portable rendering stack.

The original LPMT 1 source and data are preserved in
[`archive/lpmt-1/`](archive/lpmt-1/) as the behavioral reference.

**Current status (2026-10-22):** Linux-first M0–M3 complete; M4 mapping depth and
M5 show control are in progress. Ordered layer compositing, optional video and
audio playback, a shared show clock,
undo/redo and atomic saves, multiple output windows, grouped output
presentation, a live editor preview, corner snapping, per-output pacing
diagnostics, 1D calibration LUTs, projection test patterns, temporary output
guides, multi-surface mapping, quad/grid warp, polygon masks, rectangular and
circular crops, thumbnail media picking and an OSC control surface. Mapping uses
an inline editable hierarchy beside the live preview with a bottom media shelf;
Calibration and Show are separate modes.

- One event loop, one `wgpu` device and queue, one editor window and any number
  of independent output windows.
- `egui` editor with a live composited **preview viewport** rendered offscreen at
  the stage aspect and independent of every output window. The preview shows the
  actual image/video layers under the surface and output outlines. Every surface
  is outlined; single-click any surface to edit it and drag any corner to warp
  it. Surfaces and layers can be renamed; a surface can be deleted when no
  layer relies on it as its only surface, and deleting it unassigns it from
  multi-surface layers. Surface visibility/opacity and layer
  visibility/opacity/blend/surface assignments are editable.
- Optional **corner snapping**: when enabled, dragging a surface corner snaps it
  to the nearest corner of another surface within a small screen-space
  threshold, with a visible snap target. The toggle lives in the hierarchy.
- **Multi-surface mapping:** Shift-click surfaces in the preview or list to add
  or remove them from the selection. Dragging any selected surface moves the
  whole selection; the surface list exposes align left/centre/right or
  top/middle/bottom to the selection bounds and distribute three or more
  surface centres horizontally or vertically. Multi-surface moves are one
  coalesced undo step.
- **Surface masks and layer crops:** each surface can use an editable polygon
  mask with invert and feather. Alt-click the preview to add a point,
  right-click a point to remove it, and drag points to reshape the mask. Each
  layer can apply top/right/bottom/left and circular source crops. Mask coverage
  is rasterized once per change into a shared texture, so preview and every
  output use the same data without slowing the render path.
- **Quad and grid warp:** each surface starts as a four-corner quad and can
  switch to a regular 2×2 to 6×6 control grid. Grid points are dragged directly
  in the preview, with coalesced undo. The four corners always remain available:
  moving one carries the whole grid through the same homography, so the
  deformation follows the surface. Toggling grid warp off preserves the stored
  deformation; **Reset grid** rebuilds it and **Reset to full stage** resets the
  whole surface. Old projects load as quad surfaces.
- Each output is mapped to a stable `OutputId` and renders its own
  `OutputSpec.view` rectangle of the stage. The editor can add, delete, open,
  close and fullscreen outputs, edit their normalized view rectangles and pick
  an explicit display for fullscreen outputs. Output views are drawn in the
  stage preview so overlap is visible.
- Outputs can be collected into **output groups**. Group members are rendered
  from the same frame state, submitted in a single queue call and presented
  together; a full-stage confidence monitor can stay outside the group and keep
  running independently. The Outputs inspector reports per-output pacing:
  presented and missed frames, skipped/occluded frames, errors, average frame
  interval and fps.
- Outputs render images or looping video through a four-corner projective warp,
  with a procedural test pattern as fallback. Test-pattern layers can select a
  checker, alignment grid, colour bars, grayscale ramp or RGB convergence card.
- Each output renders through an offscreen scene pass followed by a composite
  pass that applies per-edge blending (left/right/top/bottom feather),
  per-output brightness/contrast/gamma and an optional display-space 1D RGB
  calibration LUT. This is the base for projector seam matching.
- **Calibration LUTs:** load a Resolve-style 1D `.cube` file for the selected
  output. The curves are resampled to 32 samples per channel, stored in the
  project (so the show remains portable), and applied after colour correction.
  The Load/Clear controls are in Calibration mode; standard `0..1` domains and
  per-channel curves are supported, while 3D `.cube` files are rejected for
  now.
- **Temporary calibration guides:** each output can show a grid, centre
  crosshair, safe-area frames or rule-of-thirds lines. Guides are session-only
  and never saved into the project; press `G` to cycle the selected output.
- **Inline Mapping editor and media shelf:** Mapping is a single editable
  hierarchy beside a larger live preview. The active surface and active layer
  expand inline, so renaming, source selection, assigned surfaces, opacity,
  blend and ordering all live next to the object they affect. The bottom media
  shelf has an explicit target-layer selector, thumbnails for images and decoded
  video, and every asset's current role. It can assign an image/video to the
  selected layer, add it as a new layer on all selected surfaces, or make an
  audio file the show track. **Choose media** (`M`) opens a thumbnail picker
  without leaving the layer. Importing or reloading media keeps the current
  target layer selected.
- **Workspace modes:** **F1 Mapping**, **F2 Calibration** and **F3 Show** switch
  between authoring, output calibration and transport/audio. Project open/save
  lives in the top **Project** menu.
- Ordered layer stack composited bottom-to-top with per-layer visibility,
  opacity, one-or-more surface assignment and `Normal`, `Add`, `Screen` or
  `Multiply` blending. A single layer can drive several surfaces without
  duplicating its media or settings. Legacy single-surface project files still
  load.
- The demo has two warped surfaces and two layers.
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
  show audio track. The **Show** mode transport (play/pause, restart, loop,
  scrub) drives it; players are paused, resumed and resynced to the clock, and a
  paused scrub decodes one preview frame so the output still updates.
- **OSC control:** a background UDP listener exposes stable parameter paths for
  layers, surfaces, outputs, show audio and transport. OSC changes go through
  the same parameter/action path as the editor and participate in undo/redo
  where they affect project state. The Show mode reports the listener address,
  message count and last accepted or rejected message. Bindings are session-only
  for now.
- Project save/load from the editor, with typed paths and native file pickers
  (Open…, Save as…, Browse… for media). Media paths are resolved relative to the
  project file; picked files inside the show directory are stored relative.
  Saves write a temporary file in the destination directory and rename it into
  place, so an interrupted save cannot leave a half-written project.
- Undo/redo for project edits, with coalesced drags, sliders and typing;
  **Ctrl+Z**, **Ctrl+Shift+Z** or **Ctrl+Y** work when no text field has focus.
  The top bar marks unsaved changes.
- Startup project loading with `cargo run -p lpmt-app -- --project show.json`.
- Homography, grid mesh, LUT parsing, test-pattern generation, media decoding,
  undo history, atomic-save and JSON compatibility unit tests.
- Fullscreen outputs claim an unused secondary monitor when one is present; the
  next outputs open as windows (960×540 or 1280×720) so the editor stays
  reachable. On a single-display machine, auto-fullscreen outputs open as
  windows so the editor remains usable; assign a display explicitly to force
  fullscreen. Windowed outputs prefer `Mailbox` and fall back to `Immediate`
  instead of blocking `Fifo`, so an obscured output cannot freeze the whole
  event loop. Fullscreen outputs on a dedicated secondary display keep
  tear-free `Fifo`. On drivers that only expose `Fifo`/`Immediate`, windowed
  outputs may tear.

### Next session

The preview viewport, snapping, pacing diagnostics, calibration LUTs, test
patterns, output guides, multi-surface mapping, grid warp, surface masks, layer
crops, stable parameter paths and OSC control are now in place. The next
implementation slice is **M5.1 show control**:

- MIDI and configurable hotkeys on the same parameter/binding layer;
- persisted bindings and generic learn;
- a timeline with numeric curves, colour tracks and triggers.

Remaining M4 work is Bézier-compatible patches and 3D LUT support. Audio is on
the shared clock; a per-layer mix bus and hardware decode remain later work. The
legacy XML importer remains planned; the full roadmap is in
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
mode, assign it to one or more surfaces, reorder it, or add/delete test-pattern
layers. If several surfaces are selected in the Mapping view, **Add test
pattern** creates one layer on all of them.
When a test-pattern layer is selected, its **Test pattern** selector switches
between the checker, alignment grid, colour bars, grayscale ramp and RGB
convergence card.

The demo also defines two outputs: **Main output** (full stage, fullscreen on a
secondary display when available) and **Right half** (windowed, view
`x=0.5..1.0`). The **Outputs** section adds or deletes outputs, toggles their
window, chooses fullscreen and an explicit display, edits the normalized view
rectangle, applies `Full stage` / `Left half` / `Right half` presets, and sets
edge blending plus brightness/contrast/gamma. The same section loads or clears a
1D calibration `.cube` and selects a temporary calibration guide for the
selected output (`G` cycles it). The **Output groups** section assigns outputs to
synchronized groups; the demo puts both outputs in **Main wall**. The stage
preview outlines every open output view.

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

In the **media shelf**, select `fingers · Video` and click **Use for layer**.
Video assets are opened when a layer uses them; unused video entries do not
start a decoder.

Playback is controlled by the **Transport** section in **Show** mode, not by
individual assets. The transport starts paused: press **Play** to advance
the shared show clock. Every used video follows that clock, is paused/resumed
with it and is seeked when it drifts. **Restart** returns to zero and plays;
**Loop** repeats the show at its duration (the longest used video until a real
timeline exists). Scrubbing while paused decodes a single preview frame at the
new position. **Space** toggles play/pause, **Home** restarts, **G** cycles the
selected output's calibration guide, **M** opens the media picker for the
selected layer and **F1/F2/F3** switch Mapping, Calibration and Show when no
text field has focus.

**Add media…** imports a new image, video or audio asset. The file picker
accepts all three types even when replacing an existing asset; the kind is
inferred from the file extension. Referencing layers follow a change between
image and video; if an asset becomes audio, those layers fall back to the test
card. Unused assets can be removed; assets referenced by a layer or used as the
show track must be reassigned first.

#### If video does not play

- **Start the transport.** The show clock starts paused; press **Play** in
  **Show** mode. A video that is assigned to a layer but not visible may also be
  behind another layer or outside its surface.
- **Check the build feature.** A default `cargo run -p lpmt-app` has no video
  support. The media shelf shows an amber warning and the status bar names the
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

Select an audio asset in the **media shelf**, then click **Assign as show
audio** or choose it from **Audio** in **Show** mode. Set **Volume** and
**Mute** there. The track is
controlled only by the shared **Transport**: it starts paused, follows
play/pause and scrubs, restarts on **Restart**, and wraps with **Loop**. If the
audio is shorter than the longest video, it plays once and restarts at the next
show loop.

Audio assets cannot be assigned to visual layers. A default build warns that
audio support is missing and suggests `rebuild with --features audio`; if the
output device cannot be opened, the show still opens and the Audio section
shows the device error.

### OSC control

The editor starts a UDP OSC listener on `127.0.0.1:9000`. Override it with
`LPMT_OSC_ADDR`, for example:

```bash
LPMT_OSC_ADDR=0.0.0.0:9000 cargo run -p lpmt-app
```

Only the first argument of a message is used. Bundles are accepted. The
`/lpmt/` prefix is optional; these are the main session paths:

```text
/lpmt/layer/layer-1/opacity          f 0.5
/lpmt/layer/layer-1/visible          i 1
/lpmt/layer/layer-1/blend            s screen
/lpmt/surface/surface-1/opacity      f 0.75
/lpmt/surface/surface-1/mask/feather f 0.04
/lpmt/output/output-1/brightness     f 1.0
/lpmt/output/output-1/edge_blend/right f 0.1
/lpmt/audio/volume                   f 0.8
/lpmt/transport/playing              i 1
/lpmt/transport/position             f 3.0
/lpmt/transport/looping              i 0
```

For example, with liblo tools:

```bash
oscsend localhost 9000 /lpmt/layer/layer-1/opacity f 0.5
oscsend localhost 9000 /lpmt/transport/playing i 1
```

The **Show** mode displays the listener address, received message count and the
last accepted or rejected message. Bindings are currently session-only;
persisted bindings, generic learn, MIDI and the timeline are the next control
milestone.

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
| `lpmt-core` | Domain model, stable IDs, geometry, LUT and guide descriptors |
| `lpmt-render` | `wgpu` device setup, shared media textures, calibration and output rendering |
| `lpmt-ui` | `egui` editor, emitting actions rather than mutating runtime state |
| `lpmt-media` | Image decoding plus optional `ffmpeg-next` video and `rodio` audio playback; capture next |
| `lpmt-control` | OSC listener; MIDI, DMX and hotkey adapters next |
| `lpmt-import-legacy` | Future LPMT 1 XML importer |
| `lpmt-app` | Window/event-loop composition and application state |

The architecture, output topology and milestones are documented in
[`docs/rewrite-rfc.md`](docs/rewrite-rfc.md). The short version: **one virtual Stage
per show, many Outputs as views of that Stage**.

In Mapping, all surfaces are outlined in the live preview. Click one to make it
active; its settings expand inline in the left editor, while the matching layer
editor stays next to its own layer row. A surface is either a four-corner quad
or a grid warp; enable **Grid warp** in the surface card and choose a 2×2 to 6×6
cell grid, then drag the blue grid points in the preview. Corner handles remain
active and move the grid deformation with the surface. Disabling **Grid warp**
keeps the deformation for later; use **Reset grid** to rebuild the mesh and
**Reset to full stage** for a complete reset. With **Snap corners** enabled, a
dragged quad corner locks onto another surface's nearest corner.
Enable **Polygon mask** in a surface card to cut a feathered, optionally inverted
polygon out of that surface. Alt-click the preview to add a mask point,
right-click a point to remove it, and drag the orange handles to reshape it. The
layer card also exposes **Source crop**: four edge insets plus an optional
circular crop. Both are undone/redone with the rest of the project.
Shift-click surfaces, or drag a selected surface, to edit several at once; align
and distribute actions appear under the surface list. The active layer card
contains its name, thumbnail source, **Choose media…** button,
assigned-surface checkboxes, opacity, blend and order controls. The bottom media
shelf reports whether each asset is unused, used by a layer, or assigned as show
audio, and can add a visual asset as a new layer on every selected surface.
**Calibration** mode shows pacing, calibration LUT controls and temporary
calibration guides. Audio and transport live in **Show** mode. Use the top
**Project** menu to save or load the JSON show file; **Undo**/**Redo** in the
top bar or Ctrl+Z / Ctrl+Shift+Z step through project edits.

## Legacy LPMT 1 archive

The original C++/openFrameworks application is preserved in
[`archive/lpmt-1/`](archive/lpmt-1/). It includes the original source, sample
media, shaders, OSC documentation and build files. It is **not** part of the Cargo
workspace and is not built by CI.

See [`archive/README.md`](archive/README.md) for the archive layout and
[`archive/lpmt-1/README.md`](archive/lpmt-1/README.md) for the original feature
list and workflow notes.

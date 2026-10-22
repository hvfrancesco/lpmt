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
circular crops, thumbnail media picking, an OSC control surface, persisted
hotkey bindings, optional MIDI learn and a parameter timeline with tracks, triggers and markers. Mapping
uses an inline editable hierarchy beside the live preview with a bottom media
shelf; Calibration and Show are separate modes.

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
  mask with invert and feather. Enable **Add points in preview** and click the
  preview, or Ctrl-click, to add a point; new points are inserted next to the
  nearest existing polygon segment. Right-click a point to remove it and drag
  points to reshape the mask. Each layer can apply
  top/right/bottom/left and circular source crops. Mask coverage is rasterized
  once per change into a shared texture, so preview and every output use the
  same data without slowing the render path.
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
- Outputs render images, looping video or solid colours through a four-corner
  projective warp, with a procedural test pattern as fallback. Test-pattern
  layers can select a checker, alignment grid, colour bars, grayscale ramp or
  RGB convergence card; solid-colour layers have an RGB colour picker and use
  the normal opacity and blend controls.
- Each output renders through an offscreen scene pass followed by a composite
  pass that applies per-edge blending (left/right/top/bottom feather),
  per-output brightness/contrast/gamma, RGB tint and an optional display-space
  1D RGB calibration LUT. Unmapped areas always clear to pure black so projectors
  emit no light outside the mapped content; a coloured background should be a
  full-stage layer. Tint channels are exposed as stable parameters and can be
  animated by the timeline. This is the base for projector seam matching.
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
  lives in the top **Project** menu; **Help → About LPMT** shows version,
  renderer and license information, and **Help → Show splash** replays the
  updated LPMT 2 startup splash.
- Ordered layer stack composited bottom-to-top with per-layer visibility,
  opacity, one-or-more surface assignment and `Normal`, `Add`, `Screen` or
  `Multiply` blending. A single layer can drive several surfaces without
  duplicating its media or settings. Legacy single-surface project files still
  load.
- **Layer colour and chroma key:** every layer has an RGB tint and an optional
  chroma key with key colour, tolerance, softness and spill suppression. Both
  are applied before surface masking, work in the quad and grid warp paths, and
  participate in undo/redo and JSON persistence.
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
  message count and last accepted or rejected message. OSC addresses are
  session-only for now.
- **Hotkey bindings:** Show mode can learn a key for any bool or transport
  parameter. Bindings are persisted in the project, undoable and take precedence
  over built-in shortcuts.
- **MIDI control:** an optional `midi` build feature enumerates MIDI inputs and
  feeds CC and note messages into the same parameter layer. Show mode learns a
  CC or note for the selected parameter, persists the binding in the project and
  supports absolute, relative and toggle modes. Absolute values map `0..127`
  onto the parameter range; relative mode follows encoder ticks; toggle mode
  turns a pad or button into a latching control. Project changes made through
  MIDI use the same actions and undo/redo path as the editor.
- **Parameter timeline:** Show mode has a bottom timeline panel targeting float
  parameters such as surface/layer opacity, solid-layer R/G/B channels, output
  brightness/contrast/gamma and edge blending, plus dedicated output colour tracks
  with RGB keys and bool
  state tracks for surface/layer visibility and mask enable/invert. Double-click
  a lane to add a key, drag keys to move them, right-click to remove, and choose
  Step, Linear or Smooth interpolation. Shift-click selects multiple numeric
  keys across tracks; copy and paste restores each key to its source parameter
  track at the playhead. Bool tracks use Step
  values. A trigger lane can pause, stop/rewind, restart, switch looping on/off
  or jump to the next/previous marker at a chosen time, and named markers provide
  seek points with header navigation. Zoom from **1×** to **64×** with the
  **-**, **+** and **Fit** controls, drag the view slider to scroll, or use
  the mouse wheel over the ruler (or **Ctrl+wheel** over any track) to zoom,
  and **Shift+wheel** to pan. **Snap** quantizes key, trigger and marker edits
  to a configurable interval. Show mode also lists markers and triggers in a
  manual **Cues** panel with **Go** buttons.
  The ruler seeks the shared show clock, so playback and scrubbing evaluate the
  keys in sync. Timeline values are a render-time overlay: playback does not
  modify the saved parameter values or mark the project dirty. A manual
  parameter edit while paused temporarily suspends the overlay until the next
  seek, play or timeline edit.
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
- On machines with multiple displays, outputs claim an unused secondary
  monitor or open as windows so the editor stays reachable. On a single display,
  **output windows are not opened automatically**; the live preview is used for
  authoring, and outputs can be opened explicitly from **Calibration > Outputs**
  with **Open now**. This avoids Wayland compositors stacking output windows
  over the editor, where there is no portable way to raise the editor back.
- Windowed outputs prefer `Mailbox` and fall back to `Immediate` instead of
  blocking `Fifo`, so an obscured output cannot freeze the whole event loop.
  Fullscreen outputs on a dedicated secondary display keep tear-free `Fifo`.
  On drivers that only expose `Fifo`/`Immediate`, windowed outputs may tear.

### Next session

The editor, multi-output rendering, mapping/masking, calibration, OSC/MIDI
control, timeline automation and cue stack are now substantially implemented.
The project is no longer planning the next timeline feature by default:
milestone acceptance and breadth take priority over adding more M5 depth.

M2/M3/M5 acceptance was recorded on **2026-10-22** in
[`docs/acceptance.md`](docs/acceptance.md). M5 remains feature-frozen for v1.

The immediate order is:

1. Record the M4 colourisation/chroma-key acceptance check, then decide whether
   3D LUT support is in v1. Bézier patches are explicitly deferred.
2. Build the **LPMT 1 legacy XML importer** with real fixtures, followed by
   packaging, recovery and performance validation (M7).
3. Add M6 media breadth: slideshows, cameras/capture, live shared textures,
   per-layer audio mixing and hardware decode.
4. Post-v1 backlog: tempo maps, additional cue commands, DMX/Art-Net, MTC/LTC
   and per-stage timelines.

Keep the checklist and smoke runner in the release process:
`./scripts/acceptance-smoke.sh` (and `LPMT_ALL_FEATURES=1` on a Linux machine
with FFmpeg/ALSA development packages). Milestone status, deferred scope and
acceptance decisions are tracked in the
[rewrite RFC](docs/rewrite-rfc.md#14-milestones).

## Build and run

Requirements:

- Rust 1.95 or newer.
- Linux with Vulkan (or another `wgpu` backend), Wayland or X11.
- Optional MIDI on Linux uses ALSA; install `libasound2-dev` and `pkg-config`.
  The default build stays image-only; enable MIDI with `--features midi`.
- On macOS and Windows the same workspace is intended to build unchanged; those
  platforms are not validated yet.

```bash
cargo run -p lpmt-app
# with MIDI input
cargo run -p lpmt-app --features midi
# open a saved show directly
cargo run -p lpmt-app -- --project show.json
```

File pickers use the desktop portal on Linux; the typed path fields remain
available as a fallback in minimal window managers without a portal.

The demo show loads [`assets/images/ofTheo.jpg`](assets/images/ofTheo.jpg) on the
background layer and [`assets/images/car.jpg`](assets/images/car.jpg) on a smaller
`Screen`-blended layer. Missing assets fall back to the procedural checker card.
Use the **Layers** section to select a layer, toggle it, change opacity or blend
mode, assign it to one or more surfaces, reorder it, or add/delete layers. If
several surfaces are selected in the Mapping view, **Add test pattern** or
**Add solid colour** creates one layer on all of them. A solid layer’s colour
picker updates the rendered RGB live and coalesces continuous picker changes in
undo. Its red, green and blue channels are exposed as stable float parameters,
so they can be hotkey/MIDI/OSC-bound or animated by timeline tracks. When a
test-pattern layer is selected, its **Test pattern** selector switches between
the checker, alignment grid, colour bars, grayscale ramp and RGB convergence
card.

The demo also defines two outputs: **Main output** (full stage, windowed at
1280×720) and **Right half** (windowed, view `x=0.5..1.0`). On a multi-display
machine they open automatically; on a single display they stay closed until you
press **Open now**. Either can be made fullscreen on an explicitly chosen
display from the Outputs section. The **Outputs** section adds or deletes outputs, toggles their
window, chooses fullscreen and an explicit display, edits the normalized view
rectangle, applies `Full stage` / `Left half` / `Right half` presets, and sets
edge blending plus brightness/contrast/gamma and per-channel RGB tint. The same section loads or clears a
1D calibration `.cube` and selects a temporary calibration guide for the
selected output (`G` cycles it). The **Output groups** section assigns outputs to
synchronized groups; the demo puts both outputs in **Main wall**. The stage
preview outlines every open output view.

#### Single-display / Wayland troubleshooting

On a single display, output windows stay closed at startup because Wayland
compositors may stack them above the editor and there is no portable
application-level “raise this window” operation. Author in the live preview,
then use **Calibration > Outputs > Open now** when you want the selected output
in a window. It may cover the editor; use Alt+Tab or window controls to switch
back. For an always-visible production output, connect a second display and
assign the output to it.

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
**Loop** repeats the show at its duration (the timeline duration when tracks
exist, otherwise the longest used video or audio). Scrubbing while paused
decodes a single preview frame at the new position. **Space** toggles play/pause, **Home** restarts, **G** cycles the
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
/lpmt/output/output-1/tint/red       f 1.0
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
last accepted or rejected message. OSC addresses are session-only; timeline
colour tracks and richer command triggers are the next control milestone.

### MIDI control

MIDI is an optional build feature. With `--features midi`, Show mode enumerates
the available input ports and reports the message count and last message. To
bind a controller, choose a parameter, choose a mode, press **Learn MIDI**, then
move a CC or press a note. The source is stored in the project; learning the same
channel/CC or channel/note again replaces that binding.

- **Bool parameters** offer **Toggle** and **Absolute**. Toggle reacts to a pad
  or button press: a fresh Note On toggles, Note Off and repeated Note On while
  held are ignored, and a CC toggles on the rising edge across 64. Absolute
  follows note on/off, or the CC value around the midpoint.
- **Float and text parameters** offer **Absolute** and **Relative**. Absolute
  maps `0..127` across the parameter range; Relative applies an increment or
  decrement using the usual `1..63` / `65..127` encoder convention. A note
  learned in Relative mode falls back to Absolute, since note deltas are not
  meaningful.
- Transport playing/looping and the show position are addressable too; position
  mapping uses the current show duration.
- Knobs are the natural source for float and text parameters; pads and buttons
  are the natural source for Toggle.

MIDI inputs are enumerated at startup, so connect controllers before opening the
app. Bindings are saved with the project and changes made through MIDI are
undoable like the equivalent editor action.

### Hotkey bindings

Show mode has a **Hotkey bindings** section for bool and transport parameters.
Pick a parameter, press **Learn key**, then press the shortcut you want. `Esc`
cancels learning. Bindings are stored in the project, participate in undo/redo
and survive reopening the show. Learning a shortcut that is already bound
replaces the old assignment.

Custom bindings take precedence over the built-in shortcuts. They currently
toggle bool parameters (surface and layer visibility, masks, output enabled,
show audio mute) and the transport playing/looping state. MIDI bindings and
timeline tracks use the same stable parameter paths.

### Timeline

Show mode includes a parameter timeline below the preview. Choose a float
parameter or a supported bool state (surface/layer visibility, mask enable or
invert), press **Add track**, then edit its keys:

- **double-click** a lane to add a key at that time;
- **drag** a key to move it between its neighbours;
- **right-click** a key to remove it;
- select a key to edit its time and value; float keys offer **Step**/**Linear**/
  **Smooth** interpolation, while bool keys are Step-only on/off states;
- **Shift-click** keys on any track to extend the selection; **Copy** and
  **Paste at playhead** preserve relative timing and restore each key to its
  source parameter track, recreating missing tracks and clamping values to each
  parameter's range;
- click or drag the ruler to seek the shared show clock;
- use **-**, **+** and **Fit** to zoom the timeline, drag the view slider to
  scroll horizontally, roll the wheel over the ruler to zoom, use **Ctrl+wheel**
  over any track to zoom, or **Shift+wheel** to pan;
- enable **Snap** and choose **Time**, **Beat** or **Bar** quantization for new
  and dragged keys, triggers and markers; **BPM**, **beats/bar** and the beat
  denominator define the musical grid. Ruler scrubbing remains continuous;
- use the normal transport to play, pause, restart and loop the timeline.

Choose an output and press **Add colour track** for a dedicated RGB tint lane.
Double-click to add colour keys, drag to retime, right-click to remove, and edit
R/G/B plus Step/Linear/Smooth interpolation in the colour-key row. Shift-click
colour keys to extend the selection; copy and paste preserve their relative
timing at the playhead. Colour tracks override the output tint channels during
playback while remaining a render-time overlay.

The **Triggers** lane schedules transport commands. Choose **Pause transport**,
**Stop and rewind**, **Restart transport**, **Loop on**, **Loop off**, **Jump to
next marker**, or **Jump to previous marker** in the header, press **Add trigger**
at the playhead, or double-click the trigger lane. Drag a trigger marker to
retime it and right-click to remove it. Triggers fire while the clock is playing,
including on a natural loop wrap; scrubbing does not fire them.

The **Markers** lane holds named seek points. Press **Add marker** at the
playhead or double-click the lane, then edit the name and time in the marker
row. Clicking a marker seeks to it; dragging retimes it and right-click removes
it. **Prev marker** and **Next marker** — or the `[` and `]` keys when no text
field has focus — jump between markers relative to the playhead. Markers are
saved with the project and participate in undo/redo.

The Show sidebar holds a persisted **Cues** stack. Add a cue at the playhead,
then edit its name, note, colour and any combination of transport commands.
**Go** fires that cue immediately; **Prev cue** and **Next cue** select by cue
time relative to the playhead. Enable **Auto at cue time** to fire a cue while
playback crosses its time, including loop wraps; scrubbing does not fire it.
Set **Follow** above zero to fire the next cue in time order after that many
wall-clock seconds; follow delays continue even when the transport is paused.
Cue metadata and command lists are saved with the project and undoable, while
firing or following a cue is a runtime action and does not dirty the project.
Timeline markers and triggers remain available as automatic events.

Timeline keys target stable parameter paths, so renaming a layer or surface does
not break automation. Track and trigger edits are undoable and saved with the
project.

Timeline values are evaluated as a render-time overlay. Playing the timeline
does not modify the underlying project parameter values or mark the project
dirty. When paused, manually changing an automated parameter temporarily
suspends the overlay so the edit is visible; the next seek, play, or timeline
edit re-enables it.

Useful commands:

```bash
cargo test --workspace
cargo test -p lpmt-app --features video,audio
cargo test -p lpmt-app --features midi
cargo fmt --all
cargo clippy --workspace --all-targets
```

The first run compiles `wgpu`, `winit` and `egui`, so it can take a few minutes.

## Repository layout

```text
crates/          Rust workspace and the LPMT 2 application
assets/          Small sample media used by demos and tests
docs/            Architecture, rewrite RFC and acceptance checklist
scripts/         Acceptance smoke runner
archive/lpmt-1/  Original C++/openFrameworks application (reference only)
```

| Crate | Responsibility |
|---|---|
| `lpmt-core` | Domain model, stable IDs, geometry, timeline, LUT and guide descriptors |
| `lpmt-render` | `wgpu` device setup, shared media textures, calibration and output rendering |
| `lpmt-ui` | `egui` editor, emitting actions rather than mutating runtime state |
| `lpmt-media` | Image decoding plus optional `ffmpeg-next` video and `rodio` audio playback; capture next |
| `lpmt-control` | OSC and optional MIDI listeners; DMX next |
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
polygon out of that surface. Turn on **Add points in preview** and click the
preview to add a mask point, or use Ctrl-click as a shortcut. The new point is
inserted into the polygon next to the nearest existing segment, so the outline
stays coherent. Right-click a point to remove it and drag the orange handles to
reshape it. The layer card also exposes **Source crop**: four edge insets plus
an optional circular crop. Both are undone/redone with the rest of the project.
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

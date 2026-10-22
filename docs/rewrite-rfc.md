# LPMT 2.0 — Rewrite Architecture RFC

- **Status:** Draft for discussion
- **Scope:** Replace the openFrameworks/C++ implementation with a modern, GPU-accelerated application.
- **Original:** LPMT 1.0, openFrameworks 0.8, 2010–2014, GPL-3.0.
- **Working name:** LPMT 2 (rename later).

---

## 1. Summary

LPMT is a practical projection-mapping tool built around a simple and effective idea:
*project content onto an ordered set of warped surfaces, with masks, blending and a timeline.*

The rewrite should preserve that idea and the show-control features that made LPMT useful,
while replacing the implementation with:

- a proper data model separated from rendering and UI;
- a GPU renderer built on `wgpu` (Vulkan / Metal / DirectX 12);
- a real editor window, separate from projector outputs;
- one process that can drive multiple output windows;
- a virtual **Stage** abstraction so multi-projector shows are authored in one coordinate space;
- a generic parameter/control system shared by OSC, MIDI, DMX, hotkeys and the timeline;
- a versioned, portable project format;
- a path to distributed rendering and non-window outputs later.

The most important architectural decision is the **output topology**. The recommendation is:

> **One application process, one virtual Stage per show, many Outputs as independent windows or sinks.
> Each Output is a view/region of a Stage. Outputs can be grouped for edge blending and frame sync.**

This supports a single projector, overlapping dual projectors, many independent projectors, LED
tiles, confidence monitors and later remote render nodes, without changing the authoring model.

### 1.1 Implementation status

The rewrite has started on the recommended direction. The original LPMT 1 source,
data and project files are archived under `archive/lpmt-1/` for behavioral reference.

- **Platform order:** Linux is the first validated target. The workspace, dependencies and window
  setup are kept cross-platform so macOS and Windows can be added without a structural rewrite.
- **M0 is substantially complete:** a Rust workspace builds, one `wgpu` device feeds an `egui`
  editor window and a separate output window, and the output renderer is independent of the editor.
- **M1 vertical slice exists:** `Stage` → `Surface` (four corners) → homography → output view,
  rendering a real image texture with a checker fallback. The editor can drag corners or edit
  their normalized coordinates, change media paths, reload media, and changes appear on every
  output.
- **Core schema v2 seed:** `Project`, `Stage`, `Surface`, `Layer`, `MediaAsset`, `SourceSpec`,
  `ShowAudio`, `OutputSpec`, `OutputGroup`, `TestPattern` and `CalibrationLut` with stable IDs,
  JSON serialization and homography/LUT tests.
- **Ordered layer stack:** the renderer composites visible layers bottom-to-top with per-layer
  visibility, opacity, one-or-more surface assignment and Normal/Add/Screen/Multiply blend modes.
  Each layer uses a dynamic uniform slice per assigned surface, so there is one pipeline per blend
  mode rather than one pipeline per layer. The editor can select, show/hide, reorder, delete,
  create test-pattern layers and toggle a layer's surfaces. The demo uses two surfaces/layers to
  exercise compositing.
- **Masks and crops:** surfaces support editable polygon masks with invert and feather, stored in
  surface UV so they follow the warp. Mask coverage is rasterized on the CPU into a cached,
  device-wide coverage texture and sampled by both the quad and grid shaders. Layers support
  rectangular edge insets and an optional circular source crop. The editor edits mask points
  directly in the live preview (Add-points mode or Ctrl-click add, right-click remove, drag to
  move) and exposes crop controls in the layer card. New points are inserted next to the nearest
  existing polygon segment instead of being appended, so the outline stays coherent. The explicit
  add mode avoids desktop shortcuts that reserve modifiers for window management. All changes
  participate in undo/redo.
- **Project persistence:** JSON save/load in the editor plus `--project <path>` startup loading,
  with native Open/Save-as dialogs and media import/replace/remove. Saves are atomic: the JSON is
  written and flushed to a temporary file in the destination directory, then renamed over the
  previous project. The media picker accepts images, videos and audio, infers `MediaKind` from the
  extension and rewrites layers that reference a changed asset. Picked media inside the show
  directory is stored project-relative; external files stay absolute. A missing asset falls back to
  the test card. Legacy layers with a single `surface_id` deserialize into a one-element
  `surface_ids` array.
- **Editor history and safety:** project edits are undoable/redoable (Ctrl+Z,
  Ctrl+Shift+Z/Ctrl+Y). Continuous edits such as corner drags, opacity/volume sliders and name
  typing are coalesced into one history step, the history is cleared when another project is
  loaded, and the top bar marks unsaved changes relative to the last saved file. The live preview
  viewport renders the composited stage offscreen, independently of the output windows, under the
  surface/output outlines; clicking a surface makes it active and dragging any corner warps it.
  Optional corner snapping locks a dragged corner to the nearest corner of another surface.
- **Optional video slice:** the `video` Cargo feature uses `ffmpeg-next` on a worker thread,
  loops a media asset, keeps only the newest frame and updates the GPU texture in place. Players are
  shared by `MediaId` and follow the application's shared show clock: the transport pauses, resumes
  and seeks them, drift is corrected, and a paused scrub decodes one preview frame. Image-only builds
  have no system FFmpeg dependency.
- **Optional audio slice:** the `audio` Cargo feature uses `rodio`/Symphonia for one project-level
  show track with volume and mute. It follows the same clock, is seeked after transport changes and
  restarts on a show loop; if it ends before a longer video, it stays silent until the next loop.
  A build without the feature has no ALSA/cpal dependency and reports a rebuild hint.
- **Shared show clock:** one `ShowClock` in the application is authoritative for every output, every
  video and the show audio. The editor transport edits that clock (play/pause, restart, loop, scrub);
  when timeline tracks exist their duration is authoritative, otherwise the clock uses the longest
  used video or audio track. A loop serial distinguishes a natural wrap from a deliberate seek.
- **Multiple output windows:** every enabled `OutputSpec` owns a window keyed by its stable
  `OutputId` and renders its own normalized `view` rectangle of the stage. Opening, closing,
  fullscreen toggling, display assignment and adding/deleting outputs recreate windows only on
  project changes. `lpmt-render::MediaCache` holds one texture/bind-group per `MediaId`, shared by
  all outputs; decoded video frames are uploaded once per application tick instead of once per
  window.
- **Presentation robustness:** fully occluded output windows skip surface acquisition and
  presentation. Windowed outputs prefer `Mailbox`/`Immediate` so a compositor-throttled `Fifo`
  present cannot stall the shared loop; fullscreen outputs on a dedicated secondary display keep
  tear-free `Fifo`. On a single display, output windows stay closed by default (the live preview
  is used instead) so a Wayland compositor cannot stack them over the editor; **Open now** opts in
  explicitly.
- **Control foundation, OSC and MIDI:** `lpmt-core::param` defines stable IDs and paths for layer,
  surface, output, audio and transport parameters, plus a project parameter inventory. The
  `lpmt-control` crate runs a background UDP OSC listener (default `127.0.0.1:9000`, override with
  `LPMT_OSC_ADDR`), accepts messages and bundles, and maps numeric/bool/text arguments onto the same
  editor actions used by the UI. Show mode reports listener status, message count and the last
  accepted or rejected address. Configurable hotkey bindings can be learned in Show mode, are
  stored in the project and toggle bool/transport parameters through the same paths. The optional
  `midi` feature adds a `midir` listener; Show mode learns CC/note sources into persisted bindings
  with absolute, relative and toggle modes, and maps them through the same parameter actions. Toggle
  gives pads and buttons latching control of bool parameters. A parameter timeline evaluates float
  curves and bool state tracks from the shared show clock in the Show panel, with transport triggers,
  named markers, a persisted cue stack and a render-time overlay so playback does not dirty the
  project. OSC addresses remain session-only; DMX, tempo maps and additional cue commands are
  next.
- **Output groups and frame barrier:** outputs assigned to an `OutputGroup` are prepared in one
  application tick, submitted in a single `queue.submit` call and presented as a batch. Outputs
  outside a group keep rendering independently, so a confidence monitor does not stall the wall.
- **Pacing diagnostics:** each output tracks attempted, presented, skipped/occluded and errored
  frames plus an EMA frame interval. The Outputs inspector shows presented/missed counts, average
  frame time and fps, and warns when a previously presenting output has gone stale.
- **Per-output composition:** each output renders its layers into an offscreen scene target, then a
  composite pass applies left/right/top/bottom edge feathering, brightness/contrast/gamma
  correction, per-channel RGB tint and an optional 1D RGB calibration LUT. Tint is also exposed
  as three stable float parameters for timeline automation. The scene target clears to pure black,
  regardless of the persisted stage background, so a projector emits no light outside the mapped
  content; coloured backgrounds belong in a full-stage layer. Correction happens after compositing,
  so overlapping outputs can be matched without changing the show content. The editor exposes all
  four edges, the three colour controls and LUT load/clear actions.
- **Test patterns and calibration:** test-pattern layers can select checker, alignment grid, colour
  bars, grayscale ramp or RGB convergence cards. A Resolve-style 1D `.cube` LUT can be loaded per
  output; it is resampled to 32 samples per channel, stored in the project and applied in display
  space after colour correction. Standard `0..1` domains are supported and 3D `.cube` files are
  rejected with a clear diagnostic.
- **Temporary output guides:** grid, centre crosshair, safe-area frames and rule-of-thirds lines can
  be shown per output. They live in application session state, not the project, and are drawn after
  calibration so their colour is predictable. `G` cycles the selected output's guide.
- **Workspace structure:** Mapping is a single editable hierarchy beside a large preview, with a
  bottom media shelf. The active surface and active layer expand inline, keeping rename, source,
  assigned surfaces, opacity and order next to the object they affect. Calibration and Show are
  separate modes (`F1`/`F2`/`F3`), project open/save lives in the top Project menu and Help →
  About LPMT reports the build/renderer version. The updated LPMT 2 splash appears at startup and
  **Help → Show splash** replays it. The media
  shelf has an explicit target-layer selector, image/video thumbnails, reports each asset's role and
  can assign visual media to that layer or create a new layer on the selected surfaces. The layer
  card opens an in-app thumbnail picker without leaving Mapping.
- **Multi-surface mapping:** Shift-click adds or removes surfaces from the selection. Dragging a
  selected surface moves the whole selection within the normalized stage; align left/centre/right,
  top/middle/bottom and distribute centres horizontally/vertically operate on the selection bounds.
  Multi-surface drags coalesce into one undo step.
- **Quad and grid warp:** a surface starts as a four-corner quad and can switch to a regular 2×2 to
  6×6 cell grid. The renderer builds a triangle mesh and samples the source over the grid UVs.
  Corner edits still apply a four-corner homography and carry the whole grid with them; toggling
  grid warp off preserves the stored deformation, with explicit Reset grid and Reset to full stage
  actions. Grid point drags coalesce in undo.
- **Not implemented yet:** per-layer audio mixing and device selection, hardware video decode,
  Bézier-compatible patches, 3D LUTs, DMX/Art-Net bindings, MIDI hotplug/device selection, tempo
  maps and additional command triggers, the legacy XML importer, packaging and validated
  macOS/Windows builds.
- **Next implementation slice:** tempo maps and additional command triggers, all targeting the
  stable parameter paths.

Run `cargo run -p lpmt-app` from the repository root. On a multi-display machine the demo opens
two windowed outputs: the full-stage main output and a “Right half” view (`x=0.5..1.0`). On a
single-display machine output windows stay closed by default and the live preview is used until
the user presses **Open now** in Calibration > Outputs. If opened on the same display they may
cover the editor; use the window switcher or connect a second display for an always-visible
production output. Fullscreen is opt-in; auto-fullscreen never targets the editor’s display. An
explicit display assignment is required to force fullscreen on the editor or primary display.

---

## 2. Goals and non-goals

### 2.1 Goals

- Reliable 1080p60 output on modest hardware, with multiple outputs on one machine.
- Real editor UI, with projector output windows free of GUI overlays.
- Persistent project format with stable IDs, media references and migration from LPMT 1 XML.
- Multi-projector edge blending and per-output colour/geometry correction.
- The existing mapping capabilities: quad warp, grid/Bézier deformation, masks, crops.
- The existing content types: images, video, shared video, slideshows, cameras, solid colours,
  colour transitions, and live textures.
- A control layer that can be driven by OSC, MIDI, DMX/Art-Net, keyboard and the timeline.
- Reasonable memory use: resources proportional to active media and outputs, not to a fixed
  maximum surface count.
- Clean failure modes: missing media, unplugged displays, unsupported codecs and lost devices
  should be visible and recoverable.

### 2.2 Non-goals for the first release

- A general 3D engine, DCC-style node graph or full compositor.
- Distributed rendering across machines (design for it, implement later).
- Hardware genlock / PTP as a v1 requirement (document and support later).
- Kinect v1 support. Depth-camera input can return later as a generic capture/source plugin.
- Byte-for-byte compatibility with the old XML or old timeline files.
- Plugin marketplace / scripting language.

---

## 3. Current implementation: what to keep and what to replace

### 3.1 Strengths to preserve

- Surface + content + layer + transform + mask + blend is the right conceptual model.
- Normalized coordinates and a per-surface homography are simple and predictable.
- The edge-blend math (exponent, gamma, luminance, four edges) is worth keeping.
- Workflow details: corner snap, double-click surface activation, rotation feedback, mask editing,
  shared videos, resync, and timeline triggers.
- Comprehensive remote control. It demonstrates what operators actually need.

### 3.2 Structural limitations

| Area | Current state | Consequence |
|---|---|---|
| Application structure | `testApp` owns input, GUI, OSC, MIDI, timeline, cameras and quads | A single god object; every feature touches the same state |
| Surface object | `quad` mixes model, media, FBOs, shaders, overlays and drawing | Hard to test, reuse or extend |
| Capacity | `quads[36]`, 8 shared videos, fixed 4×4 Bézier | Arbitrary limits leak into UI and persistence |
| Rendering | 3 screen-sized FBOs per quad; four duplicated render branches | Huge memory use and high maintenance cost |
| Graphics API | GLUT, fixed-function matrix/Bezier, GLSL 1.20 rectangle textures | Legacy platform and driver constraints |
| Control | 1,834-line OSC `if/else` chain mutating active object | Duplication, no reusable mapping model |
| Persistence | Ad-hoc XML, incomplete fields, versionless | Data loss and fragile migrations |
| UI | GUI drawn into the projection window | No true operator/output separation |
| Media | Per-quad player objects; allocation in update paths | Hard to share, sync and reclaim resources |
| Quality | No tests, no undo, little error handling | Regressions and show-time surprises |

The old code should be treated as a **feature and behaviour reference**, not as a code base to port
line by line.

---

## 4. Output topology — recommendation

### 4.1 Options considered

| Option | Description | Strengths | Problems |
|---|---|---|---|
| A. Window = canvas | One window is one complete projection canvas; add windows by duplicating the app | Simple, close to LPMT 1 | Cannot author a single composition across projectors; duplication of state; no edge blending across outputs |
| B. One giant window spanning displays | A single OS window stretched across all projectors | One canvas, simple rendering | Fragile with window managers, DPI, refresh rates; cannot independently fullscreen; bad on mixed hardware |
| C. **Stage + Outputs** | One virtual stage; each output shows a region of it; multiple windows/sinks | Flexible, authoring in one space, edge blending, confidence monitors, future remote nodes | Requires a clear output/view model; per-output rendering logic |
| D. Distributed-first | Renderer processes on multiple machines from day one | Scales to many projectors | Much higher complexity; not needed for most shows; slows initial delivery |

### 4.2 Recommended model: Stage + Outputs

```
Project
├── Stages
│   ├── Stage "Main wall"
│   │   ├── Surfaces (mapping geometry)
│   │   ├── Layers (content + effects + compositing)
│   │   └── Timeline bindings
│   └── Stage "Side screens"
├── Media assets
├── Outputs
│   ├── Output "Projector L"  → view of Stage "Main wall"
│   ├── Output "Projector R"  → view of Stage "Main wall"
│   └── Output "Confidence"   → full view of Stage "Main wall"
└── Output groups
    └── "Wall pair" → { Projector L, Projector R }  (edge blend + frame barrier)
```

**Stage** is an abstract composition canvas with a reference aspect/size. Surface coordinates are
normalized to the stage. **Output** is a physical presentation target: a window, fullscreen display,
headless target, or later NDI/DeckLink/SDI stream. An output has a **view**: usually a normalized
rectangle of a stage, but the model can later support a camera or independent per-output transform.

Examples:

- **Single projector:** one stage 16:9, one output showing the whole stage.
- **Two overlapping projectors:** stage is wider than 16:9. Output L shows `x=0.0..0.588`, output R
  shows `x=0.412..1.0` for ~15% overlap. A right-edge blend on L and left-edge blend on R feather
  the seam. Both outputs show the same surfaces in the shared stage coordinate system.
- **Two independent rooms:** two stages, each with its own output group. No cross-talk.
- **LED wall / tiles:** one stage split into arbitrary output regions.
- **Confidence monitor:** an extra output showing the full stage at reduced preview resolution; it
  does not affect show output.
- **Future distributed show:** an output group’s outputs can be assigned to remote render workers;
  the project and stage model do not change.

### 4.3 Why this is the most flexible choice

- It separates **what the show is** (stage geometry and composition) from **where it is displayed**
  (outputs and views). That is the same separation a DAW makes between a session and audio devices.
- It supports both unified authoring and independent outputs without two different application modes.
- It makes edge blending an output/seam concern, which is how projectors physically overlap, instead
  of a property of a content surface.
- It gives a natural place for future output types: windows, headless, NDI, DeckLink, virtual displays.
- It keeps distributed rendering optional: a worker is simply a process that owns a set of outputs.

### 4.4 Rendering consequences

Do **not** render one giant stage framebuffer by default. A 4-projector 4K-wide stage would be
expensive and mostly wasted. Render **each output independently** from the same scene snapshot:

1. The engine produces one immutable frame state (timeline time, media frame, parameters).
2. For each output in an output group:
   - set up the output viewport/region;
   - cull surfaces that do not intersect the output region;
   - render those surfaces with their layer/effect stacks;
   - apply output-level edge blending, colour correction and calibration;
   - present to the output sink.
3. Present all outputs for frame N from the same state.

The current renderer already separates the last two output steps: layers are composited into an
offscreen scene texture, then a fullscreen composite pass applies edge blending, colour correction
and the optional 1D calibration LUT before presenting. That pass is also the natural place for
masks and future 3D LUTs. Group members are prepared together in one application tick, submitted in
a single queue call and then presented as a batch; outputs outside the group remain independent.

A full-stage render target remains available as an optional mode for effects that must cross output
boundaries (large blur, stage-wide feedback, preview thumbnails). It is not the default path.

### 4.5 Synchronization strategy

Three tiers, implemented progressively:

1. **Shared software clock (v1):** one engine frame counter and one timeline/media clock. All outputs
   consume the same frame state. Good for normal multi-window use on one GPU.
2. **Output-group barrier:** render all outputs for a frame before presenting any of them; optionally
   pace to a master refresh and disable per-window vsync when tearing/latency matters. This is still
   software sync and cannot beat display/genlock jitter.
3. **Hardware/distributed sync (later):** genlock/PTP or a coordinator/worker frame barrier for
   multi-machine setups. Keep frame IDs in the protocol and make media clocks deterministic.

The old MPE concept is worth replacing with a small coordinator/worker protocol rather than porting
the addon. Design the boundary now; implement after single-machine multi-output is solid.

---

## 5. System architecture

### 5.1 Principles

1. **Core is pure.** The project model, parameters, timeline and commands do not depend on `wgpu`,
   `winit`, media decoders or UI.
2. **Rendering is a consumer.** The renderer reads a frame snapshot; it never owns project state.
3. **UI is a client.** The editor edits through commands/parameters; it does not mutate `quad`
   internals.
4. **Stable IDs, not array indices.** Legacy index-based OSC paths are an adapter, not the model.
5. **One source of truth per resource.** Media assets, textures and clocks are shared and reference
   counted.
6. **Explicit state machines for I/O.** Media, capture and outputs have loading/ready/error states
   visible in the UI.
7. **Allocator discipline.** Allocate render targets and textures on configuration changes, never in
   the per-frame draw path.
8. **Import compatibility is a feature.** The old XML importer is a separate, tested boundary.

### 5.2 Suggested workspace layout

```
lpmt/
├── Cargo.toml                 # workspace
├── crates/
│   ├── lpmt-core/             # model, IDs, parameters, timeline, commands
│   ├── lpmt-media/            # assets, decoders, players, capture, audio
│   ├── lpmt-render/           # wgpu renderer, mesh, shaders, output sinks
│   ├── lpmt-control/          # OSC, MIDI, DMX, keyboard mapping
│   ├── lpmt-ui/               # egui editor, preview, mapping tools
│   ├── lpmt-import-legacy/    # LPMT 1 XML importer
│   ├── lpmt-sync/             # optional, later: output-group/frame sync
│   └── lpmt-app/              # binary, event loop, composition root
├── shaders/                   # WGSL
├── docs/
└── assets/                    # built-in test patterns, icons, default font
```

Dependency direction:

```
lpmt-app ──> lpmt-ui, lpmt-render, lpmt-media, lpmt-control, lpmt-import-legacy
lpmt-ui ───> lpmt-core
lpmt-render > lpmt-core
lpmt-media > lpmt-core
lpmt-control > lpmt-core
lpmt-core ─> (serde, math; no GPU, no windowing, no codecs)
```

### 5.3 Runtime model

- **Main thread:** `winit` event loop, editor window, output windows, UI, frame pacing.
- **Engine update:** fixed timestep (e.g., 60 Hz nominal) producing frame state and timeline/media clocks.
- **Render:** one `wgpu::Device`/`Queue` per render node (initially one GPU), one surface per window output.
- **Media workers:** decode/capture threads or async tasks, publishing new frames into shared textures.
- **Control listeners:** OSC/MIDI/DMX on background threads, sending typed parameter events into the engine.
- **UI:** immediate-mode `egui`, reading a consistent snapshot and issuing commands. No rendering work
  that can block the output path.

For the first implementation, keep engine update and render on the main loop. Move simulation to its
own thread only if measurements show UI stalls affecting output.

### 5.4 Commands and events

- `Command` — intent: `AddSurface`, `LoadMedia`, `SetParameter`, `AssignOutput`, `StartTimeline`.
- `Event` — fact: `ProjectLoaded`, `MediaReadinessChanged`, `OutputLost`, `ParameterChanged`.
- Commands are serializable and undoable. Control adapters and the UI both produce commands.
- The renderer and media system consume state snapshots/events; they do not reach back into UI code.

---

## 6. Data model

### 6.1 Core entities

| Entity | Responsibility |
|---|---|
| `Project` | Root document: stages, surfaces, layers, media references, outputs, timelines, bindings |
| `Stage` | Virtual composition canvas; normalized coordinate space |
| `Surface` | Mapping geometry: 4 corners, optional warp mesh, mask polygons, crop; in stage coordinates |
| `Layer` | Binds a source to a surface, with content transform, effects, opacity and blend mode |
| `MediaAsset` | A file or device description; stable ID; metadata; readiness/error state |
| `SourceBinding` | How a layer consumes media: image, video, slideshow, camera, solid, transition, shared texture |
| `Output` | Presentation target: display/window/sink, view region of a stage, resolution, colour, blend zones |
| `OutputGroup` | Outputs that are edge-blended and frame-synchronized together |
| `Timeline` | Transport plus tracks of numbers, colours and triggers |
| `ParameterBinding` / `MidiBinding` | Maps hotkey or MIDI input to a parameter path |

### 6.2 Why separate Surface and Layer?

LPMT 1 conflates “the mapped shape” and “the content on it”. Separating them enables:

- multiple content layers on one mapped object (e.g. background + logo + live capture);
- one surface reused by several layers or stages;
- effects and blending without touching mapping geometry;
- a clean inspector: **Surface** (where) vs **Layer** (what);
- future sharing of surfaces between outputs or stages.

For simple use, the UI can create a Surface and a Layer together and present them as a single item.

### 6.3 Coordinate spaces

Pick and document these once:

- **Stage space:** normalized `0..1`, origin top-left, y down. Surface corners and output views live here.
- **Surface UV:** normalized `0..1`; masks and warp mesh are defined in this space.
- **Layer/content space:** normalized `0..1` inside the surface, with fit/fill/stretch transform.
- **Output UV:** normalized `0..1` for output-level edge blending and calibration.

Do not mix pixels and normalized values in the model. Convert at the renderer/UI boundary.

### 6.4 Illustrative project shape

```json
{
  "schema_version": 1,
  "name": "Demo wall",
  "stages": [
    {
      "id": "stg_main",
      "name": "Main wall",
      "aspect": 3.02,
      "surfaces": [
        {
          "id": "srf_left",
          "name": "Left panel",
          "corners": [[0.0, 0.0], [0.47, 0.0], [0.47, 1.0], [0.0, 1.0]],
          "warp": { "mode": "grid", "rows": 6, "columns": 7, "points": [] },
          "mask": { "mode": "polygon", "invert": false, "points": [] }
        }
      ],
      "layers": [
        {
          "id": "lyr_left_video",
          "name": "Left content",
          "surfaces": ["srf_left"],
          "source": { "kind": "video", "asset": "ast_intro", "loop": true, "volume": 0.0 },
          "transform": { "fit": "cover", "offset": [0.0, 0.0], "scale": [1.0, 1.0], "flip_h": false, "flip_v": false },
          "opacity": 1.0,
          "blend": "normal",
          "effects": [{ "kind": "colorize", "color": [1.0, 1.0, 1.0, 1.0] }]
        }
      ]
    }
  ],
  "outputs": [
    {
      "id": "out_left",
      "name": "Projector L",
      "stage": "stg_main",
      "view": [0.0, 0.0, 0.588, 1.0],
      "display": { "connector": "DP-1", "resolution": [1920, 1080], "fullscreen": true },
      "blend": { "right": { "width": 0.12, "exponent": 2.0, "gamma": 1.8, "luminance": 0.0 } }
    }
  ]
}
```

This is illustrative; final schema belongs in a separate `schema.md`.

### 6.5 Parameters

All animatable/controllable values are addressed by stable paths, e.g.:

```
surface/srf_left/corners/2/x
layer/lyr_left_video/opacity
output/out_left/blend/right/width
stage/stg_main/layer/lyr_left_video/effects/0/color
```

A parameter has a value, type, range/unit, metadata and optional bindings. UI, OSC, MIDI, DMX and
timeline all read/write through this layer. It also gives us generic MIDI learn, OSC auto-learn,
presets, undo and show snapshots for free.

---

## 7. Renderer design

### 7.1 Output pipeline

For each output and frame:

1. Resolve view region and output target.
2. Clear the output scene to pure black; a coloured background is content, not a clear colour.
3. For each visible layer intersecting the output:
   - sample the source texture;
   - apply content transform, crop, flips, colour and source effects;
   - map onto the surface geometry (4-corner projective warp + optional deformation);
   - apply surface mask;
   - composite with the layer blend mode into the output target or a scratch layer target;
4. Apply output edge blending and colour calibration.
5. Present.

Layers that need offscreen effects (blur, feedback, large mask feathering) render into a scratch
target; simple layers composite directly. Scratch targets are pooled and reused.

### 7.2 Warp and deformation

- Base mapping: four corners define a homography. For an exact projective result, sample the source
  with inverse homography in the fragment shader, or use a sufficiently tessellated mesh with
  perspective-correct sampling. Do not naively interpolate UVs across two triangles.
- Deformation: a regular `rows × columns` warp mesh whose points are offsets in surface UV. This
  subsumes LPMT’s grid warp and provides a foundation for Bézier patches and future freeform meshes.
- Masks: polygons in surface UV, rasterized into a mask texture or evaluated analytically in the
  fragment shader. Include rectangular/circular crop as generated masks.
- Edge blending: output-level 4-edge curve (width, exponent, gamma, luminance), plus optional
  per-surface feathering later. Keep the old shader math as the reference implementation.

### 7.3 Effects and compositing

Start with a small, explicit effect set rather than a generic node graph:

- opacity / colour tint / colour temperature;
- chroma key (green screen) with tolerance and spill suppression;
- mask invert / feather;
- blend modes: normal, screen, add, subtract, multiply, and a few modern ones;
- flips, crop, fit/fill/stretch;
- optional blur/glow if needed for a show.

Represent effects as an ordered, typed stack on a layer. The renderer compiles/caches pipeline state
on change. This avoids the old four-way duplication of mask/edge-blend/deform branches.

### 7.4 Colour pipeline

Decide and document:

- render in linear light, convert from source transfer function (sRGB/video) at sampling;
- output in the display’s transfer function;
- 8-bit per channel initially, with 10-bit and HDR output as a later option;
- consistent alpha handling and premultiplied alpha rules;
- edge-blend curve evaluated in the intended domain, with a configurable gamma to match projectors;
- per-output 1D calibration curves loaded from `.cube` files and applied in display space;
- optional test patterns (checker, grid, colour bars, grayscale, convergence) for alignment and
  projector matching. 3D LUTs and HDR output remain later work.

Colour correctness across projectors matters as much as warp correctness. Make it an explicit output
concern, not an accidental consequence of blend functions.

### 7.5 Performance rules

- No per-frame allocation of textures/FBOs/pipelines.
- One decode per media asset, shared by layers; independent playheads where required.
- Cull surfaces per output; skip invisible/zero-opacity layers.
- Cache warp meshes, masks and pipeline state; invalidate on change.
- Use dirty flags for parameter changes and static scenes.
- Target: 4×1080p60 outputs on a mid-range GPU for typical scenes; measure with GPU timestamps and
  CPU/GPU profiling.
- Avoid full-stage render targets unless a selected effect requires them.

### 7.6 GPU and backend policy

- Primary: `wgpu` with Vulkan, Metal and DX12 native backends.
- Pick a minimum feature level after a hardware/portability spike (e.g. Vulkan 1.1 / Metal 2 / DX12).
- One render device initially. Multiple GPUs can later be represented as multiple render nodes or
  output workers; do not complicate the first release.
- Use `wgpu` surface configuration per output, with explicit format/colour-space negotiation.

---

## 8. Media architecture

- `MediaStore`: owns `MediaAsset`s and their metadata; resolves paths; tracks readiness and errors.
- `Decoder` implementations behind a trait: image, video, slideshow, camera, shared texture.
- `Player`: a playback instance referencing an asset, with position/speed/loop/audio state.
- `Clock`: media clocks and the master show clock; timeline drives/resyncs them.
- **Shared videos** become first-class: one asset/player (or several views of one player) referenced
  by many layers, with deterministic resync. No per-quad copies of decoder state.
- Video backend decision: `ffmpeg-next` is the first backend. The spike confirmed straightforward
  frame extraction and RGBA conversion on the target Linux machine, including looped playback from
  a worker thread. GStreamer is deferred as the likely path for hardware decode and richer capture
  pipelines. Keep decoding behind a trait as playback features grow.
- Cameras: `nokhwa`/V4L2 on Linux initially; Media Foundation/macOS capture later. Capture devices
  and virtual cameras should look like the same source abstraction.
- Audio v1: one output device and one project-level show track decoded by
  `rodio`/Symphonia, with volume/mute and clock-driven transport. The per-layer mix bus, per-layer
  volume/mute and device selection are later work. Do not tie audio to a per-surface video player
  callback.
- Loading is asynchronous and cancellable. The UI shows “loading / missing codec / device lost”.
- Media paths can be relative to the project bundle or absolute; support search paths.

The old QuickTime/codec limitation must not be carried forward: codec support should be a property of
the decoder backend, not the platform.

---

## 9. Show control and parameters

### 9.1 Adapters

- **OSC:** map addresses to parameter paths and trigger commands. Ship a legacy compatibility map
  for LPMT 1 addresses (`/active/...`, `/corners/...`, etc.) during migration.
- **MIDI:** map notes and CC to parameters through optional `midir` inputs; absolute values, relative
  encoders, latching Toggle mode for bool parameters and Show-mode learn are implemented. Pitch bend
  is parsed and reported but not yet bindable; hotplug and per-device selection remain later work.
- **DMX/Art-Net:** later, using the same binding model.
- **Keyboard:** persisted bindings target stable parameter paths. Show-mode hotkey learn currently
  toggles bool/transport parameters; increments and command bindings follow on the same layer.
- **Timeline:** curves and triggers write parameters/commands through the same API.
- **Remote UI:** optional WebSocket/JSON API for tablets and browser control.

### 9.2 Parameter semantics

- Typed values: bool, int, float, colour, enum, vec2/vec3, string/path.
- Metadata: range, unit, display name, group, read-only, persistable, animatable.
- Atomic transactions for undo and for consistent snapshots.
- Bindings can be enabled/disabled per show; learn mode is generic.
- A show snapshot = project state + timeline position + media state; useful for troubleshooting.

This replaces the old pattern where every OSC address knew the internals of the active quad.

---

## 10. Timeline

Timeline should be core functionality, not an addon. The current v1 implements:

- One global project timeline driven by the shared show clock.
- Float tracks targeting stable parameter paths, with Step, Linear and smoothstep interpolation.
  Output tint and solid-layer RGB channels use the same mechanism.
- Dedicated output colour tracks with RGB keys, editing, playback overlay, multi-key shift
  selection and copy/paste at the playhead.
- Multi-track key selection with shift-click and copy/paste at the playhead. Keys carry their stable
  parameter path, so paste restores each key to its source track, recreating missing tracks and
  clamping values to each parameter's range.
- Bool state tracks for surface/layer visibility and mask enable/invert, using on/off Step keys.
- A trigger lane for pause, stop/rewind, restart, loop on/off and marker-jump commands, fired while
  playing and on loop wrap.
- Named seek markers with click-to-seek, previous/next navigation, `[`/`]` keyboard jumps,
  retiming, rename and undo support.
- Horizontal timeline navigation with 1×–64× zoom, a view scrollbar, wheel-over-ruler zoom,
  Ctrl+wheel zoom over tracks and Shift+wheel pan.
- Configurable time, beat or bar snap for key, trigger and marker edits, driven by persisted BPM,
  beats-per-bar and time-signature denominator settings.
- A persisted Show-mode cue stack with stable IDs, name, note, colour, multiple transport commands,
  manual **Go** / previous / next actions, optional automatic firing at the cue time and
  wall-clock follow delays that advance the next cue in time order.
- Show-mode editing: add/remove tracks, add keys at a time, drag keys, edit time/value/mode, manage
  triggers, seek from the ruler, and resize the timeline duration.
- Deterministic evaluation: playback and scrubbing sample the same clock used by video and audio.
- A render-time overlay that does not modify saved parameter values or dirty the project. Manual
  parameter edits while paused suspend the overlay until the next seek/play/timeline edit.
- Undoable timeline edits and project persistence.

Still planned:

- Tempo maps and additional command triggers.
- Per-stage timelines or independent cue stacks, MTC/LTC input, OSC timecode.

The old LPMT timeline XML is addon-specific. A dedicated importer can be attempted later; the project
model and timeline UI are more important than binary compatibility.

---

## 11. Persistence and migration

### 11.1 Project format

- `project.json` (human-readable, versioned) in a project bundle folder.
- `schema_version` mandatory; migrations run on load; unknown fields preserved where safe.
- Media references use IDs plus relative paths; missing media is reported, not silently dropped.
- Atomic save (write temp + rename), autosave and backup rotation.
- Optional embedded project snapshot/log for support and recovery.

### 11.2 Legacy LPMT 1 importer

Implement as a separate crate with fixtures and golden tests:

- Parse `_lpmt_settings.xml` and `config.xml`.
- Map quads → surfaces + layers, preserving corner positions, layer order, content type, colour,
  flip/scale, masks, crops, edge blend, Bezier control points.
- Map shared videos, images, videos and slideshow folders to media assets.
- Report fields that cannot be migrated (for example, grid points are not saved by LPMT 1’s
  `setXml`, and only four shared videos are persisted).
- Import the timeline separately if feasible; otherwise document manual recreation.

The importer is the bridge that makes the rewrite trustworthy to existing users.

---

## 12. Editor UI

- Separate **Editor** window; output windows never contain controls or debug overlays by default.
- Panels: Stages/Surfaces/Layers tree, inspector, media library, output configuration, timeline,
  preview/confidence view, logs and diagnostics. The current editor groups these into Show,
  Mapping (inline editable hierarchy, preview and media shelf), Calibration and Show modes.
- Editing happens in a preview viewport. Calibration guides can be shown on an output as an explicit
  temporary session mode; they are never stored in the project.
- Direct manipulation: corner handles, grid points, rotation, snap, multi-select, align/distribute.
- Undo/redo for every command; keyboard shortcuts are configurable bindings.
- Output status: connected display, resolution, refresh, frame time, frame ID, blend preview.
- Recovery: unplugged display, missing media, decoder failure and GPU device loss must be visible and
  recoverable without restarting the show.
- Use `egui` initially. A more bespoke UI can come later if needed; the core must not depend on it.

---

## 13. Distributed rendering (design now, build later)

Keep these boundaries so distributed mode is an extension, not a rewrite:

- Outputs are identified by stable IDs and assigned to a **render node**.
- A render node owns a GPU device, one or more output sinks, and a media subset.
- The coordinator owns authoritative project state and the show clock.
- Protocol carries: project snapshot/hash, parameter deltas, frame number, timeline time,
  play/stop/resync commands, readiness and error events.
- Media must exist on workers (shared asset sync or pre-distribution). Streaming video between
  machines is a separate feature.
- Frame barrier: coordinator issues frame N, workers render, acknowledge; late workers are reported.
- Add hardware genlock/PTP support behind the output group abstraction when a show requires it.

Do not build this in v1. Do keep `OutputSink` and frame IDs in the design from day one.

---

## 14. Milestones

### M0 — Bootstrap and spikes

- [x] Rust workspace.
- [x] `wgpu` + `winit` + `egui` spike: editor window plus a second output window.
- [x] Linux/Vulkan GPU baseline; cross-platform workspace.
- [x] CI workflow for Linux, macOS and Windows (not yet run on hosted runners).
- [x] Video decode spike on target hardware: `ffmpeg-next` selected for the first backend;
      GStreamer deferred for hardware decode/capture.
- [x] Multi-output present behaviour.
- [x] Output-group frame pacing diagnostics.

### M1 — Minimal renderer

- [x] Project model seed with one stage, a surface and a layer.
- [x] Four-corner homography.
- [x] Output window with fullscreen/windowed fallback and a stage-view uniform.
- [x] Texture/image layers and normal blending.
- [x] Project load/save skeleton and UI.
- [x] Acceptance: a PNG mapped onto a quad and shown fullscreen on a second display at 60 fps.

### M2 — Editor and video

- [x] Surface/layer inspector, drag handles, selection, rename, add/delete surfaces and surface
      visibility/opacity.
- [x] Ordered layer stack with visibility, opacity, one-or-more surface assignment and basic blend
      modes.
- [x] Solid-colour layers with an RGB colour picker, cached 1×1 render textures and stable
      per-channel timeline/OSC/MIDI parameters.
- [x] Native project open/save and media file pickers; picked media stored relative when possible.
- [x] Media thumbnails, an in-app layer media picker and explicit target-layer assignment.
- [x] Inline Mapping editor, separate Calibration/Show modes and a bottom media shelf.
- [x] Atomic project saves and dirty-state tracking.
- [x] Undo/redo for project edits with coalesced drags, sliders and typing.
- [x] Stage preview shows every surface; click to select and drag any corner to warp.
- [x] Video asset playback with background decoding and looped playback (optional feature).
- [x] Shared player per `MediaId`, driven by one show clock with play/pause, restart, loop and
      seek/scrub controls.
- [x] Shared show clock and video drift resync, including paused-scrub preview frames.
- [x] Project show-audio track on the shared clock, with volume/mute and loop restart (optional
      `audio` feature).
- [ ] Per-layer audio mix bus and device selection.
- [ ] Hardware decode and per-output video timing.
- [x] Live preview viewport separate from output, with corner snapping.
- Acceptance: create a two-surface show from scratch, save, reload, play video.

### M3 — Multi-output and edge blending

- [x] Multiple output windows and stage-region views.
- [x] Per-output edge blend and colour/gamma controls.
- [x] Output groups and a shared frame barrier.
- [x] Explicit display assignment per output.
- [x] Per-output pacing diagnostics and missed-present reporting.
- Acceptance: two overlapping 1080p outputs with a blended seam; confidence monitor unaffected.

### M4 — Mapping depth

- [x] Regular grid warp with 2×2 to 6×6 control cells.
- [x] Polygon surface masks with invert/feather and rectangular/circular layer crops.
- [ ] Bézier-compatible patches.
- [ ] Colourisation, chroma key and advanced blend modes.
- [x] 1D per-output calibration LUTs and projection test patterns.
- [x] Temporary calibration guides on outputs.
- [x] Multi-select, whole-surface drag, align and distribute.
- [ ] 3D LUTs and broader calibration diagnostics.
- Acceptance: feature parity with LPMT 1’s core mapping and masking, except Bézier patches.

### M5 — Show control

- [x] Stable parameter IDs/paths and a project parameter inventory.
- [x] OSC listener with numeric/bool/text arguments, bundles and Show-mode status.
- [x] Configurable hotkey bindings with Show-mode learn.
- [x] Persisted parameter bindings, including undo/redo and project reload.
- [x] MIDI CC/note bindings with absolute, relative and toggle modes plus Show-mode learn.
- [x] Float and bool-state timeline tracks with scrubbing and render-time playback.
- [x] Timed transport triggers (pause, stop/rewind, restart, loop on/off and marker jumps).
- [x] Smooth easing, multi-key copy/paste and named timeline markers.
- [x] Per-output RGB tint parameters, dedicated output colour tracks and multi-key colour copy/paste.
- [x] Marker `[`/`]` keyboard navigation.
- [x] Persisted cue stack with names, notes, colours, multiple commands, Go/prev/next and auto firing.
- [x] Configurable time/beat/bar snap driven by BPM, beats-per-bar and denominator.
- [x] Multi-track key selection with path-based copy/paste.
- [x] Cue follow delays that advance the stack after a configurable interval.
- [ ] Tempo maps and additional command triggers.
- [ ] DMX/Art-Net bindings on the same parameter layer.
- Acceptance: run a timed show via OSC and timeline with no GUI on outputs.

### M6 — Media breadth and sync

- Slideshows, cameras, capture devices, shared textures (NDI/Spout/Syphon/PipeWire as appropriate).
- Optional output-group barrier and remote sync prototype.
- Acceptance: live camera + video + shared source in one show; repeatable resync.

### M7 — Migration and release candidate

- Legacy XML importer with fixtures.
- Packaging for target platforms, crash/recovery UX, performance validation.
- Acceptance: import a real LPMT 1 show and run it with documented differences.

---

## 15. Testing and observability

- Unit tests: IDs, parameter paths, timeline evaluation, homography, UV transforms, blend math,
  legacy XML parsing.
- Golden image tests: render offscreen through `wgpu` and compare reference frames with tolerances.
- Integration tests: mock output sink and mock decoder; project load/save round-trip.
- Device tests: hot-plug display, missing media, unsupported codec, GPU device loss.
- Logging/tracing with categories (`media`, `render`, `output`, `control`, `timeline`).
- Runtime diagnostics: frame time, GPU timestamps, dropped frames, decoder queue depth, output status.
- Optional profiling integration (Tracy/puffin) behind a feature flag.

---

## 16. Key risks and open questions

| Topic | Risk / question | Initial position |
|---|---|---|
| Primary platform | Linux-first, macOS, Windows? | Build portable; validate Linux first, then macOS/Windows |
| Video decode | ffmpeg vs GStreamer, hardware acceleration, packaging | Spike before M2; keep decoder trait |
| Multi-window present | Vsync/refresh mismatch, blocking presents, tearing, latency | Shared frame state; output group barrier; skip obscured windows; prefer Mailbox/Immediate over blocking Fifo; document tearing limits |
| Colour | Gamma/linear blending, projector differences | Explicit pipeline, calibration controls, golden tests |
| Edge blending | Physical overlap vs model regions | Output-level zones; per-surface feather later |
| Distributed outputs | Scope creep | Design boundary only; implementation post-v1 |
| Legacy import | Missing/Lossy old XML fields | Best-effort importer with a clear report |
| GPU compatibility | Old show machines | Publish minimum GPU/driver requirements |
| Licensing | Original is GPL-3.0 | Treat rewrite as GPL-3.0 unless advised otherwise |
| Naming | “LPMT 2” vs new product name | Defer; repository structure is neutral |

---

## 17. Immediate next steps

1. Review and accept (or amend) the **Stage + Outputs** topology.
2. Run the M0 spikes: multi-window `wgpu`, `egui` integration, video decode, 2-output presentation.
3. Freeze a minimal v1 data-model schema and write fixtures from a real LPMT 1 project.
4. Build the M1 vertical slice: image → surface → warp → output window.
5. Use that slice to validate memory, frame time and operator workflow before adding features.

The guiding rule: **rebuild the concepts, not the code**. LPMT’s domain model and show workflow are
the valuable part; the old rendering and control implementation are the part to leave behind.

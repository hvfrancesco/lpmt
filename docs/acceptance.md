# Milestone acceptance

This document is the working acceptance record for LPMT 2. The roadmap rule is
**close milestone acceptance before extending a feature area**. M2, M3, M4 and
M5 acceptance were recorded on 2026-10-22; M7 migration is in progress. M5
remains feature-frozen for v1 and 3D LUT support is deferred to post-v1.

Use this document to run the checks, record evidence and list blockers. Do not
mark a milestone complete from code review alone.

## Evidence header

Record one header per acceptance session:

```text
Date:                    2026-10-22
Commit:                  4020448
OS / session:            Linux / Wayland
GPU / driver:            NVIDIA Quadro M520 / Vulkan
Build:                   video,audio,midi
Tester:                  project owner
```

## Automated smoke checks

Run these first. They cover the model, serialization, bindings and the OSC
transport path; they do not replace the manual output/display checks.

```bash
# Default checks plus the model/persistence and OSC smoke tests:
./scripts/acceptance-smoke.sh

# On a Linux machine with FFmpeg/ALSA development packages:
LPMT_ALL_FEATURES=1 ./scripts/acceptance-smoke.sh
```

The script runs formatting, clippy, workspace tests, the core acceptance test,
the OSC listener test and, when `LPMT_ALL_FEATURES=1`, the video decoder and
playback-control examples.

The `lpmt-core` acceptance test builds a two-surface show from scratch with
image/video/audio assets, masks/crops, output groups, timeline tracks, triggers,
markers, cues, hotkeys and MIDI bindings, then saves and reloads it.

## M2 — Editor and video

Target: create a two-surface show from scratch, save, reload, play video and
audio.

Checklist:

- [x] Launch with `cargo run -p lpmt-app --features video,audio`.
- [x] Create two surfaces, warp them, and confirm they appear in the preview.
- [x] Add an image layer and a video layer; assign media through the shelf.
- [x] Test layer visibility/opacity/blend, surface visibility/opacity, masks and
      rectangular/circular crops.
- [x] Save to a new JSON path.
- [x] Reload the saved project and verify geometry, media assignments, masks,
      crops and layer order.
- [x] Play the video from the Show transport; verify pause, resume, restart,
      loop and seek/scrub.
- [x] Assign the show audio track and verify volume, mute, seek and loop.
- [x] Undo and redo representative geometry, media, mask and crop edits.
- [x] Confirm the editor remains responsive throughout.

Result: `PASS`

Notes / blockers:

```text
All checks passed.
```

## M3 — Multi-output and edge blending

Target: two overlapping 1080p outputs with a blended seam; an independent
confidence monitor remains unaffected.

This check requires two physical displays or a documented equivalent setup. On
a single display the app intentionally opens in preview-only mode; use
Calibration > Outputs > **Open now** only if you accept that the output window
may cover the editor.

Checklist:

- [x] Open two outputs on separate 1920×1080 displays.
- [x] Set one output to the left stage region and one to the right, with the
      intended overlap (for example `x=0.0..0.588` and `x=0.412..1.0`).
- [x] Apply edge blending on the shared seam and match brightness/contrast/gamma.
- [x] Assign both outputs to one output group; confirm they present together.
- [x] Add a third full-stage confidence monitor outside the group; confirm it
      continues rendering independently.
- [x] Check pacing diagnostics and the absence of freezes while focusing the
      editor and the outputs in turn.
- [x] Save the project and verify the output views, groups, edge blend and colour
      correction reload correctly.

Result: `PASS`

Notes / blockers:

```text
All checks passed.
```

## M4 — Mapping depth: colourisation and chroma key

Target: layer colourisation and chroma key match the LPMT 1 core workflow.

Checklist:

- [X] Set RGB tint on an image, a video and a solid layer; reset to neutral and
      verify preview and output.
- [X] Apply a chroma key to a green-screen image or video; tune key colour,
      tolerance, softness and spill and verify transparent and retained pixels.
- [X] Confirm tint/chroma combine correctly with rectangular/circular crops and
      surface masks.
- [X] Check both quad and grid-warp surfaces.
- [X] Save/reload and undo/redo tint and chroma changes.
- [x] Decide and record whether 3D LUT support is in v1 (decision only). Bézier
      patches remain deferred.

Result: `PASS`

Notes / blockers:

```text
All checks passed.
```

## M5 — Show control

Target: run a timed show via OSC and the timeline with no GUI interaction on the
outputs. Run this with `--features video,audio` and, where available,
`--features midi`.

### OSC

- [x] Start the app with the default OSC listener and confirm Show mode reports
      `Listening on 127.0.0.1:9000`.
- [x] Change a layer/surface parameter and verify the preview/output update.
- [x] Start, pause and seek the transport over OSC.
- [x] Change an output parameter and an audio parameter over OSC.
- [x] Send an unknown path and confirm the rejection is visible in Show mode.
- [x] Repeat with `LPMT_OSC_ADDR` set to a non-default port.

Example paths:

```text
/lpmt/layer/layer-1/opacity
/lpmt/surface/surface-1/opacity
/lpmt/output/output-1/brightness
/lpmt/audio/volume
/lpmt/transport/playing
/lpmt/transport/position
```

### Hotkeys and MIDI

- [x] Learn a hotkey for a bool parameter and a transport parameter in Show
      mode.
- [x] Confirm a learned binding takes precedence over a built-in shortcut.
- [x] Save, reload and verify the bindings survive; undo/redo binding edits.
- [x] With the `midi` feature, enumerate an input port and learn a CC or note.
- [x] Verify absolute, relative and toggle modes as applicable.
- [x] Reconnect/restart the app and verify the persisted binding still resolves
      its parameter path.

### Timeline, markers and cues

- [x] Add a float track (for example layer opacity) and place Step, Linear and
      Smooth keys.
- [x] Scrub the ruler and confirm the preview/output follows the timeline.
- [x] Play through the keys and verify values are not written into the saved
      base parameters and do not mark the project dirty.
- [x] Add a bool state track and a dedicated output colour track; verify both
      evaluate during playback.
- [x] Add a marker, seek with the ruler and use previous/next marker navigation.
- [x] Add a trigger and verify pause/stop/restart/loop/marker-jump behavior.
- [x] Create a cue stack with multiple commands, auto firing and a follow delay;
      use Go/previous/next and verify the follow advances correctly.
- [x] Select multiple numeric keys, copy/paste them and verify they return to
      their source parameter tracks.
- [x] Test beat/bar snapping with the configured BPM, beats-per-bar and beat unit.
- [x] Save, reload and verify tracks, keys, markers, triggers and cues persist.

### No-GUI timed run

- [x] Load the acceptance project from the command line.
- [x] Do not interact with the editor after startup.
- [x] Trigger the show by OSC or the timeline.
- [x] Confirm outputs, transport, audio/video and cues run for the full duration.
- [x] Record any long-run pacing, drift, decoder or audio issues.

Result: `PASS`

Notes / blockers:

```text
All checks passed.
```

## M7 — Migration and release

Target: import a real LPMT 1 show and run it with documented differences, then
validate packaging, recovery and long-run performance.

Checklist:

- [x] Automated import of a representative LPMT 1 XML fixture, including
      corners, masks, crops, layer order, tint/opacity, blend modes and shared
      video references.
- [ ] Import a real LPMT 1 show, run the converted project and record remaining
      differences (timeline, cameras, slideshows, edge blending, deform).
- [x] Produce, extract, launch and install the Linux release package.
- [ ] Produce and launch packages for macOS and Windows (scope decision pending).
- [x] Verify crash/recovery UX and the atomic-backup path: autosave snapshot on
      dirty projects, restore/discard modal, stale-snapshot cleanup and save
      removal.
- [x] Run the repeatable release performance check with two output windows:
      120 s, 59.67 fps average, 16.76 ms/frame.
- [x] Add and validate the telemetry soak harness: 300 s, 59.84 fps,
      +2.7 MiB steady RSS, 28.96% average CPU, 46 °C GPU peak.
- [ ] Run a longer production soak/thermal test with the target output topology.

Result: `PENDING`

Notes / blockers:

```text
Importer, crash recovery and Linux packaging are implemented and covered by
automated/unit checks. The portable package was extracted, launched and installed
into a temporary prefix. A 120 s release run with two forced output windows held
59.67 fps / 16.76 ms per frame on the reference machine. A 300 s telemetry soak
held 59.84 fps with 2.7 MiB steady RSS growth, 28.96% average CPU and a 46 °C GPU
peak. No real LPMT 1 settings file was available locally, so real-show validation,
macOS/Windows packaging and a longer production soak are still pending.
```

## Evidence log

| Date | Commit | Environment | Build features | Checks | Result | Notes |
|---|---|---|---|---|---|---|
| 2026-10-22 | 4020448 | Linux / Wayland / Quadro M520 | video,audio,midi | M2 | PASS | Two-surface show, save/reload, video/audio, masks/crops and undo/redo. |
| 2026-10-22 | 4020448 | Linux / Wayland / Quadro M520 | video,audio,midi | M3 | PASS | Two 1080p outputs, blended seam, output group and independent confidence monitor. |
| 2026-10-22 | 8277601 | Linux / Wayland / Quadro M520 | video,audio,midi | M4 | PASS | RGB tint + chroma key verified with image/solid content, masks/crops and warp paths. 3D LUT deferred post-v1. |
| 2026-10-22 | 4020448 | Linux / Wayland / Quadro M520 | video,audio,midi | M5 | PASS | OSC, hotkeys/MIDI, timeline, cues and no-GUI timed run. |
| 2026-10-23 | 255768c | Linux / Wayland / Quadro M520 | default | M7 (partial) | PENDING | Linux package extract/launch/install passed; 120 s release run 59.67 fps; 300 s telemetry soak 59.84 fps, +2.7 MiB steady RSS, GPU 46 °C. Real-show, macOS/Windows and long production soak remain. |

## Known constraints

- **Single display / Wayland:** output windows are preview-only by default
  because the compositor may stack them over the editor. Use Calibration >
  Outputs > Open now when needed; a second display is required for an
  always-visible production output.
- **M3:** the seam and confidence-monitor check cannot be completed on a single
  display.
- **MIDI:** hardware/ALSA input is required; the feature is optional and must be
  built explicitly.
- **Feature builds:** `--all-features` needs FFmpeg, ALSA and their development
  packages on Linux.
- **Post-v1:** tempo maps, additional cue commands, DMX/Art-Net, MTC/LTC,
  per-stage timelines, Bézier patches and distributed sync are not acceptance
  blockers unless the scope decision changes.

## Sign-off

```text
M2: PASS     Reviewed by: project owner     Date: 2026-10-22
M3: PASS     Reviewed by: project owner     Date: 2026-10-22
M4: PASS     Reviewed by: project owner     Date: 2026-10-22
M5: PASS     Reviewed by: project owner     Date: 2026-10-22
M7: PENDING  Reviewed by:                       Date:

Blocking issues: None for M2/M3/M4/M5; M7 validation pending.
```

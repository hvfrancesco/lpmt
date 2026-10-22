# Packaging and performance checks

## Linux portable package

Build a release binary with the default feature set and create a portable
tarball plus SHA-256 checksum:

```bash
./scripts/package-linux.sh
```

Output:

```text
dist/lpmt-<version>-linux-<arch>/
dist/lpmt-<version>-linux-<arch>.tar.gz
dist/lpmt-<version>-linux-<arch>.tar.gz.sha256
```

The package contains the `lpmt` binary, sample assets, README/licence files,
acceptance/RFC docs, a desktop entry and an `install.sh` helper.

Feature builds can be packaged with the same script:

```bash
LPMT_FEATURES=video,audio,midi ./scripts/package-linux.sh
```

The binary can run from the extracted directory, or from anywhere because the
default demo searches `assets/` next to the executable.

### Install

Install for the current user:

```bash
tar -xzf dist/lpmt-*.tar.gz
cd lpmt-*-linux-*
./install.sh
```

This installs to `~/.local/bin/lpmt`, `~/.local/share/lpmt/assets`, the desktop
entry and an icon. Use another prefix with `PREFIX=/usr/local ./install.sh`.
The installed binary finds assets under `<prefix>/share/lpmt/assets`.

## Performance check

The app has a timed run mode used by the release check:

```bash
./scripts/performance-check.sh
LPMT_PERF_SECONDS=120 ./scripts/performance-check.sh
LPMT_PERF_PROJECT=/path/to/show.json ./scripts/performance-check.sh
```

The script builds the release binary, runs the app with `--run-seconds`, opens
output windows even on a single display, and requires the app to print a
summary:

```text
performance: elapsed_s=... frames=... avg_fps=... avg_frame_ms=... open_outputs=... defined_outputs=...
```

Useful environment variables:

| Variable | Default | Meaning |
|---|---|---|
| `LPMT_PERF_SECONDS` | `30` | Timed run length |
| `LPMT_PERF_PROFILE` | `release` | `release` or `debug` |
| `LPMT_FEATURES` | default | Feature set to build |
| `LPMT_PERF_PROJECT` | demo | Project JSON to load |
| `LPMT_PERF_OPEN_OUTPUTS` | `1` | Force output windows open |
| `LPMT_MIN_FPS` | `0` | Fail if the measured average is below this |

The command-line options are also useful directly:

```bash
lpmt --project show.json --run-seconds 30 --open-outputs
lpmt --version
lpmt --help
```

## Validation status

- **Linux x86_64 portable package:** built, extracted, launched and installed
  into a temporary prefix.
- **Performance baseline:** release build, demo show, two output windows,
  120 seconds at ~59.67 fps / ~16.76 ms per frame on the Linux/Wayland Quadro
  M520 reference machine.
- **Still pending:** macOS/Windows packages, real LPMT 1 show validation and a
  longer thermal/soak run on production output hardware.

# Archive

This directory contains historical code kept for reference and migration work.
Nothing here is part of the LPMT 2 Cargo workspace or its CI.

## LPMT 1

`lpmt-1/` is the original C++/openFrameworks application, including its source,
data, shaders, OSC documentation and build files. It is the behavioral reference
for the Rust rewrite, not a supported build target.

- Original README: [`lpmt-1/README.md`](lpmt-1/README.md)
- OSC documentation: [`lpmt-1/README.osc`](lpmt-1/README.osc)
- Source: [`lpmt-1/src/`](lpmt-1/src/)
- Legacy media and shaders: [`lpmt-1/bin/data/`](lpmt-1/bin/data/)

The archived project expects openFrameworks 0.8 and its old addons. Moving the
folder changes the relative depth to openFrameworks, so the old project files may
need path adjustments if someone wants to build it again. LPMT 2 does not depend
on it.

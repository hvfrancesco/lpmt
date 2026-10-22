//! Convert an LPMT 1 XML settings file into an LPMT 2 JSON project.
//!
//! Usage:
//!   cargo run -p lpmt-import-legacy --example import-legacy -- show.xml show.json

use std::path::PathBuf;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let input = arguments
        .next()
        .map(PathBuf::from)
        .context("usage: import-legacy <show.xml> [show.json]")?;
    let output = arguments.next().map(PathBuf::from);

    let imported = lpmt_import_legacy::import_file(&input)?;
    let json = serde_json::to_string_pretty(&imported.project)
        .context("failed to serialize imported project")?;

    if let Some(output) = output {
        if let Some(parent) = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(&output, json)
            .with_context(|| format!("failed to write {}", output.display()))?;
        println!(
            "Imported {} quads, {} surfaces, {} layers and {} media assets to {}",
            imported.report.quads,
            imported.report.surfaces,
            imported.report.layers,
            imported.report.media,
            output.display()
        );
    } else {
        println!("{json}");
    }

    for warning in &imported.report.warnings {
        eprintln!("warning: {warning}");
    }
    if !imported.report.unsupported_features.is_empty() {
        eprintln!(
            "unsupported legacy features: {}",
            imported.report.unsupported_features.join(", ")
        );
    }

    Ok(())
}

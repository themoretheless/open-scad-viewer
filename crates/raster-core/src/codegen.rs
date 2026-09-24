//! Codegen: exports the WGSL sources to the TypeScript shader library.
//!
//! `crates/raster-core/shaders/*.wgsl` is the single source of truth. The
//! browser renderer consumes generated copies of these exact texts; this
//! module renders `src/services/shaders/generated/sources.ts` and the
//! `wgsl_export` binary writes it to disk. A `--check` mode and the
//! `generated_ts_matches_the_wgsl_sources` test keep the checked-in file in
//! lockstep with the WGSL sources.

use crate::shaders::{
    DEEP_MESH_WGSL, EDGE_WGSL, GRID_WGSL, LINE_WGSL, MESH_WGSL, SELECTION_OVERLAY_WGSL,
};

/// (export name, WGSL source, doc comment) in stable emission order.
pub const TS_SOURCES: [(&str, &str, &str); 6] = [
    ("MESH_WGSL", MESH_WGSL, "Lit opaque/transparent mesh surface with per-object style and GPU morph blend."),
    ("DEEP_MESH_WGSL", DEEP_MESH_WGSL, "X-ray deep-selection mesh (depth Always, translucent orange)."),
    ("EDGE_WGSL", EDGE_WGSL, "Per-mesh wireframe edges with selection/hover tinting."),
    ("LINE_WGSL", LINE_WGSL, "Plain colored line list (measurements, grid axes)."),
    ("GRID_WGSL", GRID_WGSL, "Full-viewport XY grid, reconstructed from camera rays with adaptive spacing."),
    ("SELECTION_OVERLAY_WGSL", SELECTION_OVERLAY_WGSL, "Per-vertex colored overlay for source-face highlighting."),
];

const HEADER: &str = "\
// GENERATED FILE — do not edit.
// Generated from crates/raster-core/shaders/*.wgsl (the single source of truth)
// by the raster-core `wgsl_export` codegen. Regenerate:
//   cargo run --manifest-path crates/Cargo.toml -p raster-core --bin wgsl_export
// Verify:
//   cargo run --manifest-path crates/Cargo.toml -p raster-core --bin wgsl_export -- --check
";

/// Escapes a WGSL source for inclusion in a TS template literal.
fn ts_escape(source: &str) -> String {
    source.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${")
}

/// Renders the full `sources.ts` content.
pub fn generate_ts_sources() -> String {
    let mut out = String::from(HEADER);
    for (name, source, doc) in TS_SOURCES {
        out.push_str(&format!("\n/** {doc} */\nexport const {name} = /* wgsl */`\n"));
        // Emit without the leading/trailing blank lines the .wgsl files carry.
        out.push_str(&ts_escape(source.trim()));
        out.push_str("`\n");
    }
    out
}

/// Repo-relative path of the generated TS module, resolved against the crate
/// manifest directory (`crates/raster-core`).
pub fn generated_ts_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src/services/shaders/generated/sources.ts")
}

/// Writes the generated TS module to disk, creating parent directories.
pub fn write_ts_sources() -> std::io::Result<std::path::PathBuf> {
    let path = generated_ts_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, generate_ts_sources())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_module_exports_every_source_as_template_literal() {
        let ts = generate_ts_sources();
        for (name, source, _doc) in TS_SOURCES {
            assert!(ts.contains(&format!("export const {name} = /* wgsl */`")), "missing export {name}");
            assert!(ts.contains(source.trim()), "{name} body drifted from the WGSL source");
        }
        // TS template-literal hazards are escaped.
        assert!(!ts.contains("${"));
    }
}

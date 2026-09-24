//! Exports the WGSL sources to the TypeScript shader library.
//!
//! Usage:
//!   cargo run --manifest-path crates/Cargo.toml -p raster-core --bin wgsl_export
//!   cargo run --manifest-path crates/Cargo.toml -p raster-core --bin wgsl_export -- --check

fn main() {
    let check = std::env::args().any(|arg| arg == "--check");
    if check {
        let path = raster_core::codegen::generated_ts_path();
        let on_disk = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            eprintln!("wgsl_export: cannot read {}: {error}", path.display());
            std::process::exit(1);
        });
        let generated = raster_core::codegen::generate_ts_sources();
        if on_disk != generated {
            eprintln!("wgsl_export: {} is stale — regenerate with `wgsl_export`", path.display());
            std::process::exit(1);
        }
        println!("wgsl_export: {} is up to date", path.display());
    } else {
        match raster_core::codegen::write_ts_sources() {
            Ok(path) => println!("wgsl_export: wrote {}", path.display()),
            Err(error) => {
                eprintln!("wgsl_export: write failed: {error}");
                std::process::exit(1);
            }
        }
    }
}

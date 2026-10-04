#![doc = include_str!("../README.md")]
//! Validated mesh serialization, 3MF OPC packaging and transactional scene export.
//! Algorithms borrow buffers and do not depend on polygon-core or transport.
pub mod export_file;
pub mod import;
pub mod export_prepare;
pub mod mesh_export;
pub mod model_3mf;
pub mod package_3mf;
pub use math_core::{Error, Result};
use math_core::{cross, norm, sub};
pub use mesh_export::{export, export_print_mesh};
pub use mesh_topology::MeshView;
use std::fmt::Write;
fn error(message: impl Into<String>) -> Error {
    Error::new("MESH_IO_INVALID_INPUT", message)
}
fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, "MESH_IO_INVALID_INPUT", message)
}
pub fn export_stl(mesh: &MeshView<'_>) -> Result<String> {
    let report = mesh.inspect()?;
    check(
        report.closed && report.signed_volume_mm3 > 0.,
        "STL export requires a closed, consistently oriented mesh with positive volume.",
    )?;
    let mut output = String::from("solid modelgraph_nurbs_sampled\n");
    for t in mesh.indices.as_chunks::<3>().0 {
        let a = mesh.point(t[0])?;
        let b = mesh.point(t[1])?;
        let c = mesh.point(t[2])?;
        let normal = cross(sub(b, a), sub(c, a));
        let len = norm(normal);
        writeln!(
            output,
            "  facet normal {} {} {}\n    outer loop",
            normal[0] / len,
            normal[1] / len,
            normal[2] / len
        )
        .unwrap();
        for p in [a, b, c] {
            writeln!(output, "      vertex {} {} {}", p[0], p[1], p[2]).unwrap();
        }
        output.push_str("    endloop\n  endfacet\n");
        check(output.len() <= 4 * 1024 * 1024, "STL export exceeds 4 MiB.")?;
    }
    output.push_str("endsolid modelgraph_nurbs_sampled\n");
    Ok(output)
}

//! Sampled shell and local triangle-mesh blend orchestration in the kernel.
use crate::{Result, encode, field, input};
use polygon_core::Mesh;
use value_codec::Value;
pub fn shell(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let thickness: f64 = field(&v, "thickness")?;
    let requested = v["step"].as_f64().unwrap_or(0.);
    if !requested.is_finite() || requested < 0. {
        return Err(input("Grid step must be finite and nonnegative."));
    }
    let topology = mesh_topology::planar::topology(mesh.view())?;
    let faces = topology.faces;
    let mut openings = std::collections::BTreeSet::new();
    for i in field::<Vec<usize>>(&v, "openings")? {
        let f = faces.get(i).ok_or_else(|| input("Invalid opening face."))?;
        openings.extend(f.triangles.iter().copied());
    }
    let mut span = 0_f64;
    for k in 0..3 {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for p in mesh.positions.as_chunks::<3>().0 {
            min = min.min(p[k]);
            max = max.max(p[k]);
        }
        span = span.max(max - min);
    }
    let adaptive = v["adaptive"].as_bool().unwrap_or(false);
    let step = if requested > 0. {
        requested
    } else if adaptive {
        thickness / 4.
    } else {
        (thickness / 4.).max(span / 59.)
    };
    encode(crate::mesh_shell::shell_options(
        &mesh,
        &openings.into_iter().collect::<Vec<_>>(),
        thickness,
        step,
        adaptive,
    )?)
}
pub fn blend(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    let target: Mesh = field(&v, "target")?;
    encode(polygon_core::local_blend::blend(
        &mesh,
        &target,
        field(&v, "edge")?,
        field(&v, "radius")?,
        field(&v, "endRadius")?,
        v["kind"].as_str() == Some("fillet"),
    )?)
}

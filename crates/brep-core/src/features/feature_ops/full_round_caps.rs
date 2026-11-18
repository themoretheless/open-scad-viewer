//! Author exact XY cap clocks without dividing original curve coefficients.
//! Only fresh axis-aligned rectangular cap carriers from the prism constructor
//! are admitted; world geometry and original edge curves are unchanged.
use crate::{Model, Result};

pub(super) fn world_charts(model: &mut Model) -> Result<()> {
    let mut edits = Vec::new();
    for (id, face) in model.faces.iter().enumerate() {
        let Some(z) = crate::prism::planar_cap_z(&face.surface) else {
            continue;
        };
        let s = &face.surface;
        let a = &s.control_points[0][0];
        let b = &s.control_points[1][0];
        let c = &s.control_points[0][1];
        let d = &s.control_points[1][1];
        if a[0] != c[0]
            || b[0] != d[0]
            || a[1] != b[1]
            || c[1] != d[1]
            || b[0] <= a[0]
            || c[1] <= a[1]
        {
            return Err(super::error(
                "Full Round cap requires an authored XY rectangle carrier",
            ));
        }
        let mut pcurves = Vec::new();
        for w in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for (i, coedge) in model.loops[w].coedges.iter().enumerate() {
                let mut curve = crate::prism::directed_edge(model, coedge)?;
                if curve.control_points.iter().any(|p| p[2] != z) {
                    return Err(super::error("Full Round cap rim leaves its original plane"));
                }
                for p in &mut curve.control_points {
                    p.truncate(2);
                }
                pcurves.push((w, i, curve));
            }
        }
        edits.push((id, [a[0], b[0]], [a[1], c[1]], pcurves));
    }
    if edits.len() != 2 {
        return Err(super::error(
            "Full Round requires two complete authored prism caps",
        ));
    }
    for (id, u, v, pcurves) in edits {
        let s = &mut model.faces[id].surface;
        s.knots_u = vec![u[0], u[0], u[1], u[1]];
        s.knots_v = vec![v[0], v[0], v[1], v[1]];
        for (w, i, curve) in pcurves {
            model.loops[w].coedges[i].pcurve = curve;
        }
    }
    model.validate()?;
    Ok(())
}

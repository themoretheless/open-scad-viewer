use super::*;

/// Interior refinement points are midpoints of the boundary triangulation,
/// so on the concave side of a curved trim they can fall into the sliver
/// between the chord and the exact pcurve, and evaluate off the face. Test
/// every interior point against a dense sampling of the exact trim loops and
/// pull the outliers onto that boundary.
pub(super) fn clamp_interior_to_trims(
    model: &brep_core::Model,
    face: &brep_core::Face,
    mut mesh: FaceMesh,
) -> Result<FaceMesh> {
    const DENSE: usize = 48;
    let mut loops: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut curved = false;
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        let mut polyline = Vec::new();
        for coedge in &model.loops[wire].coedges {
            let curve = &coedge.pcurve;
            curved |= curve.degree > 1;
            let [a, b] = curve.domain();
            for i in 0..DENSE {
                let t = a + (b - a) * i as f64 / DENSE as f64;
                let p = curve.evaluate(t)?.point;
                polyline.push([p[0], p[1]]);
            }
        }
        loops.push(polyline);
    }
    if !curved {
        return Ok(mesh);
    }
    let inside = |p: [f64; 2]| {
        let mut crossings = 0usize;
        for polyline in &loops {
            for i in 0..polyline.len() {
                let a = polyline[i];
                let b = polyline[(i + 1) % polyline.len()];
                if (a[1] > p[1]) != (b[1] > p[1]) {
                    let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
                    if x > p[0] {
                        crossings += 1;
                    }
                }
            }
        }
        crossings % 2 == 1
    };
    for (index, point) in mesh.uv.iter_mut().enumerate() {
        if mesh.shared[index].is_some() || inside(*point) {
            continue;
        }
        let mut best: Option<(f64, [f64; 2])> = None;
        for polyline in &loops {
            for i in 0..polyline.len() {
                let a = polyline[i];
                let b = polyline[(i + 1) % polyline.len()];
                let d = [b[0] - a[0], b[1] - a[1]];
                let len2 = d[0] * d[0] + d[1] * d[1];
                let t = if len2 > 0. {
                    (((point[0] - a[0]) * d[0] + (point[1] - a[1]) * d[1]) / len2).clamp(0., 1.)
                } else {
                    0.
                };
                let q = [a[0] + t * d[0], a[1] + t * d[1]];
                let dist = (q[0] - point[0]).powi(2) + (q[1] - point[1]).powi(2);
                if best.map(|(bd, _)| dist < bd).unwrap_or(true) {
                    best = Some((dist, q));
                }
            }
        }
        if let Some((_, q)) = best {
            // Land just inside the boundary rather than on it, so the moved
            // sample never coincides with a boundary sample (which would fold
            // a triangle or break the shared seam samples).
            let away = [point[0] - q[0], point[1] - q[1]];
            let len = away[0].hypot(away[1]);
            let nudged = if len > 0. {
                [q[0] - away[0] / len * 1e-4, q[1] - away[1] / len * 1e-4]
            } else {
                q
            };
            *point = if inside(nudged) { nudged } else { q };
        }
    }
    Ok(mesh)
}

pub fn polygons(model: &polygon_core::solid::brep::Model) -> Result<Tessellation> {
    let t = polygon_core::solid::brep::tessellate(model)?;
    finish(
        t.mesh,
        t.face_ids,
        None,
        model.tolerance_mm,
        model.shells.iter().all(|s| s.closed),
    )
}

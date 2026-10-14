use super::*;

/// Split only interior UV edges; all boundary positions retain their registry ownership.
pub(super) fn refine_trimmed_face(
    face: &brep_core::Face,
    mesh: FaceMesh,
    segments: usize,
) -> Result<FaceMesh> {
    let surface = &face.surface;
    let mut uv = mesh.uv;
    let mut shared = mesh.shared;
    let mut triangles = mesh.triangles;
    let width = surface.knots_u[surface.control_points.len()] - surface.knots_u[surface.degree_u];
    let height =
        surface.knots_v[surface.control_points[0].len()] - surface.knots_v[surface.degree_v];
    let key = |a: usize, b: usize| if a < b { [a, b] } else { [b, a] };
    for _ in 0..24 {
        let mut incidence = BTreeMap::<[usize; 2], usize>::new();
        for t in &triangles {
            for i in 0..3 {
                *incidence.entry(key(t[i], t[(i + 1) % 3])).or_default() += 1;
            }
        }
        let mut splits = BTreeMap::<[usize; 2], usize>::new();
        for (edge, count) in incidence {
            let [a, b] = edge;
            let length =
                ((uv[a][0] - uv[b][0]) / width).powi(2) + ((uv[a][1] - uv[b][1]) / height).powi(2);
            if count == 2 && length > 2. / (segments * segments) as f64 {
                splits.insert(edge, uv.len());
                uv.push([(uv[a][0] + uv[b][0]) / 2., (uv[a][1] + uv[b][1]) / 2.]);
                shared.push(None);
            }
        }
        if splits.is_empty() {
            return Ok(FaceMesh {
                uv,
                shared,
                triangles,
            });
        }
        let mut refined = Vec::new();
        for mut t in triangles {
            let count = (0..3)
                .filter(|&i| splits.contains_key(&key(t[i], t[(i + 1) % 3])))
                .count();
            if count == 0 {
                refined.push(t);
                continue;
            }
            if count == 3 {
                let [a, b, c] = t;
                let ab = splits[&key(a, b)];
                let bc = splits[&key(b, c)];
                let ca = splits[&key(c, a)];
                refined.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
            } else {
                // Rotate so the one marked edge is AB, or the two are AB and BC.
                while !(splits.contains_key(&key(t[0], t[1]))
                    && (count == 1 || splits.contains_key(&key(t[1], t[2]))))
                {
                    t.rotate_left(1);
                }
                let [a, b, c] = t;
                let ab = splits[&key(a, b)];
                if count == 1 {
                    refined.extend([[a, ab, c], [ab, b, c]]);
                } else {
                    let bc = splits[&key(b, c)];
                    refined.extend([[ab, b, bc], [a, ab, c], [ab, bc, c]]);
                }
            }
        }
        if refined.len() > 20000 {
            return Err(input(format!(
                "B-rep tessellation exceeds 20000 triangles during interior refinement: {} triangles at segments {}",
                refined.len(),
                segments
            )));
        }
        triangles = refined;
    }
    Err(input(
        "B-rep interior tessellation exceeded its refinement budget",
    ))
}
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

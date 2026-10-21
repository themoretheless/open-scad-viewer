//! Orthographic retained edges with bounded trimmed NURBS-surface occlusion
//! and an explicitly selected mesh hidden-line mode.
use crate::{Result, field, input};
use nurbs_core::curve::Curve;
use polygon_core::Mesh;
use std::fmt::Write;
use value_codec::{Value, json};
#[inline(always)]
fn project(p: &[f64], view: &str) -> [f64; 3] {
    match view {
        "top" => [p[0], -p[1], p[2]],
        "front" => [p[0], -p[2], -p[1]],
        _ => [p[1], -p[2], p[0]],
    }
}
fn curve_path(c: &Curve) -> Result<String> {
    let p = &c.control_points;
    let a = &p[0];
    let b = p.last().unwrap();
    let mut d = String::with_capacity(64);
    write!(d, "M{} {} ", a[0], a[1]).unwrap();
    if c.degree == 1 {
        write!(d, "L{} {}", b[0], b[1]).unwrap();
    } else if c.degree == 3 && c.weights.iter().all(|w| *w == c.weights[0]) {
        write!(
            d,
            "C{} {} {} {} {} {}",
            p[1][0], p[1][1], p[2][0], p[2][1], b[0], b[1]
        )
        .unwrap();
    } else if c.degree == 2 && p.len() == 3 {
        let w = c.weights[1] / (c.weights[0] * c.weights[2]).sqrt();
        if w == 1. {
            write!(d, "Q{} {} {} {}", p[1][0], p[1][1], b[0], b[1]).unwrap();
        } else if w > 0. && w < 1. {
            let w2 = w * w;
            let one_minus_w2 = 1. - w2;
            let center = [
                ((a[0] + b[0]) * 0.5 - w2 * p[1][0]) / one_minus_w2,
                ((a[1] + b[1]) * 0.5 - w2 * p[1][1]) / one_minus_w2,
            ];
            let u = [w * (p[1][0] - center[0]), w * (p[1][1] - center[1])];
            let denom = 2. * one_minus_w2.sqrt();
            let v = [(b[0] - a[0]) / denom, (b[1] - a[1]) / denom];
            let m = [
                u[0] * u[0] + v[0] * v[0],
                u[0] * u[1] + v[0] * v[1],
                u[1] * u[1] + v[1] * v[1],
            ];
            let dm = m[0] - m[2];
            let radius = (dm * dm + 4. * m[1] * m[1]).sqrt();
            let rx = ((m[0] + m[2] + radius) * 0.5).sqrt();
            // det avoids subtracting nearly equal eigenvalues for edge-on ellipses.
            let det = u[0] * v[1] - u[1] * v[0];
            let ry = if rx > 0. { det.abs() / rx } else { 0. };
            if ry > 1e-10 * rx.max(1.) {
                let angle = 0.5 * (2. * m[1]).atan2(m[0] - m[2]);
                write!(
                    d,
                    "A{rx} {ry} {} 0 {} {} {}",
                    angle.to_degrees(),
                    if det > 0. { 1 } else { 0 },
                    b[0],
                    b[1]
                )
                .unwrap();
            } else {
                let axis = if u[0].abs() + v[0].abs() > u[1].abs() + v[1].abs() {
                    0
                } else {
                    1
                };
                let end = w.acos();
                let phase = v[axis].atan2(u[axis]);
                let mut angles = [-end, end, 0., 0., 0., 0., 0.];
                let mut len = 2usize;
                for k in -2..=2 {
                    let t = phase + f64::from(k) * std::f64::consts::PI;
                    if -end < t && t < end {
                        angles[len] = t;
                        len += 1;
                    }
                }
                angles[..len].sort_unstable_by(f64::total_cmp);
                for &t in &angles[1..len] {
                    let (sin_t, cos_t) = t.sin_cos();
                    write!(
                        d,
                        "L{} {} ",
                        center[0] + u[0] * cos_t + v[0] * sin_t,
                        center[1] + u[1] * cos_t + v[1] * sin_t
                    )
                    .unwrap();
                }
            }
        } else {
            return Err(input("Projected conic is not an admitted ellipse"));
        }
    } else {
        return Err(input("Projected curve degree/weights are unsupported"));
    }
    Ok(d)
}
fn triangle_state(c: &Curve, t: &[[f64; 3]; 3], tol: f64) -> i8 {
    let [a, b, z] = *t;
    let cross = |a: [f64; 3], b: [f64; 3], p: &[f64]| {
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
    };
    let determinant = (b[0] - a[0]) * (z[1] - a[1]) - (b[1] - a[1]) * (z[0] - a[0]);
    if determinant.abs() < 1e-16 {
        return 0;
    }
    let sign = determinant.signum();
    let mut definitely_inside = true;
    for (u, v) in [(a, b), (b, z), (z, a)] {
        let margin = tol * (v[0] - u[0]).hypot(v[1] - u[1]);
        let mut all_outside = true;
        let mut all_inside = true;
        for p in &c.control_points {
            let val = sign * cross(u, v, p);
            if !(val < -margin) {
                all_outside = false;
            }
            if !(val >= margin) {
                all_inside = false;
            }
        }
        if all_outside {
            return 0;
        }
        definitely_inside &= all_inside;
    }
    let mut all_front = true;
    let mut all_behind = true;
    for p in &c.control_points {
        let s = ((p[0] - a[0]) * (z[1] - a[1]) - (p[1] - a[1]) * (z[0] - a[0])) / determinant;
        let u = ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])) / determinant;
        let d = p[2] - (a[2] + s * (b[2] - a[2]) + u * (z[2] - a[2]));
        if !(d >= -tol) {
            all_front = false;
        }
        if !(d < -tol) {
            all_behind = false;
        }
    }
    if all_front {
        return 0;
    }
    if definitely_inside && all_behind {
        return 1;
    }
    -1
}
/// Affine retained surface chart, including the authored UV trim and holes.
struct PlanarFace {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    uv: [[f64; 2]; 2],
    domain: brep_core::face_domain::FaceDomain,
    boundaries: Vec<Curve>,
}
impl PlanarFace {
    fn state(&self, c: &Curve, tolerance: f64, work: &mut usize) -> Result<i8> {
        let det = self.u[0] * self.v[1] - self.u[1] * self.v[0];
        if det.abs()
            <= 256. * f64::EPSILON * self.u[0].hypot(self.u[1]) * self.v[0].hypot(self.v[1])
        {
            return Ok(0);
        }
        let mut uv = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
        let mut depth = [f64::INFINITY, f64::NEG_INFINITY];
        for p in &c.control_points {
            let x = p[0] - self.origin[0];
            let y = p[1] - self.origin[1];
            let a = (x * self.v[1] - y * self.v[0]) / det;
            let b = (self.u[0] * y - self.u[1] * x) / det;
            let z = p[2] - (self.origin[2] + a * self.u[2] + b * self.v[2]);
            depth[0] = depth[0].min(z);
            depth[1] = depth[1].max(z);
            for (k, t) in [a, b].into_iter().enumerate() {
                let value = self.uv[k][0] + t * (self.uv[k][1] - self.uv[k][0]);
                uv[k][0] = uv[k][0].min(value);
                uv[k][1] = uv[k][1].max(value);
            }
        }
        // Positive rational weights enclose the complete curve, including
        // its inverse affine UV chart and signed distance to the surface.
        let coincident = self.boundaries.iter().any(|boundary| {
            if boundary.degree != c.degree
                || boundary.control_points.len() != c.control_points.len()
            {
                return false;
            }
            [false, true].into_iter().any(|reverse| {
                (0..c.control_points.len()).all(|i| {
                    let j = if reverse {
                        c.control_points.len() - 1 - i
                    } else {
                        i
                    };
                    (c.weights[i] / c.weights[0]
                        - boundary.weights[j]
                            / boundary.weights[if reverse {
                                boundary.weights.len() - 1
                            } else {
                                0
                            }])
                    .abs()
                        <= 256. * f64::EPSILON
                        && (0..2).all(|k| {
                            (c.control_points[i][k] - boundary.control_points[j][k]).abs()
                                <= 256.
                                    * f64::EPSILON
                                    * c.control_points[i][k]
                                        .abs()
                                        .max(boundary.control_points[j][k].abs())
                                        .max(1.)
                        })
                })
            })
        });
        if coincident {
            return Ok(0);
        }
        if depth[0] >= -tolerance {
            return Ok(0);
        }
        for k in 0..2 {
            if uv[k][1] < self.uv[k][0] || uv[k][0] > self.uv[k][1] {
                return Ok(0);
            }
        }
        // An entire projected span on a natural surface-domain edge has no
        // interior ray crossing. This also avoids subdivision along silhouettes.
        if (0..2).any(|k| {
            self.uv[k].iter().any(|edge| {
                uv[k]
                    .iter()
                    .all(|t| (t - edge).abs() <= 256. * f64::EPSILON * edge.abs().max(1.))
            })
        }) {
            return Ok(0);
        }
        if (0..2).any(|k| uv[k][0] < self.uv[k][0] || uv[k][1] > self.uv[k][1]) {
            return Ok(-1);
        }
        let classification = self.domain.classify(uv, 4096)?;
        *work += classification.cells;
        if *work > 2000000 {
            return Err(input("B-rep trim classification work budget exceeded"));
        }
        match classification.location {
            nurbs_core::trim_domain::Location::Outside => Ok(0),
            nurbs_core::trim_domain::Location::Inside if depth[1] < -tolerance => Ok(1),
            _ => Ok(-1),
        }
    }
}
enum RetainedFace {
    Boundary(Vec<Curve>),
    Planar(PlanarFace),
    Curved {
        surface: nurbs_core::surface::Surface,
        domain: brep_core::face_domain::FaceDomain,
    },
}
impl RetainedFace {
    fn state(&self, curve: &Curve, tolerance: f64, work: &mut usize) -> Result<i8> {
        if let Self::Boundary(edges) = self {
            for edge in edges {
                *work += 1;
                if *work > 2000000 {
                    return Err(input("B-rep boundary coverage work budget exceeded"));
                }
                for fixed in 0..2 {
                    let value = edge.control_points[0][fixed];
                    if !edge
                        .control_points
                        .iter()
                        .chain(&curve.control_points)
                        .all(|p| p[fixed] == value)
                    {
                        continue;
                    }
                    let axis = 1 - fixed;
                    let a = edge.control_points.first().unwrap()[axis];
                    let b = edge.control_points.last().unwrap()[axis];
                    // Continuity and endpoint bracketing prove a boundary point
                    // for every projected coordinate. The depth hull proves
                    // strict occlusion without guessing its parameter.
                    if curve
                        .control_points
                        .iter()
                        .all(|p| a.min(b) <= p[axis] && p[axis] <= a.max(b))
                    {
                        let z = edge
                            .control_points
                            .iter()
                            .map(|p| p[2])
                            .fold(f64::INFINITY, f64::min);
                        let target = curve
                            .control_points
                            .iter()
                            .map(|p| p[2])
                            .fold(f64::NEG_INFINITY, f64::max);
                        if nurbs_core::interval_eval::Interval::point(z)
                            .sub(nurbs_core::interval_eval::Interval::point(target))?
                            .lo
                            > tolerance
                        {
                            return Ok(1);
                        }
                    }
                }
            }
            return Ok(0);
        }
        let Self::Curved { surface, domain } = self else {
            let Self::Planar(face) = self else {
                unreachable!()
            };
            return face.state(curve, tolerance, work);
        };
        // Positive rational weights enclose both complete definitions in
        // their Euclidean control hulls. Reject distant charts before the
        // more expensive dependency-sensitive homogeneous interval jets.
        if (0..2).any(|k| {
            let lo = curve
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min);
            let hi = curve
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max);
            surface.control_points.iter().flatten().all(|p| p[k] < lo)
                || surface.control_points.iter().flatten().all(|p| p[k] > hi)
        }) {
            return Ok(0);
        }
        if nurbs_core::surface_occlusion::ball_silhouette_excludes(surface, curve, tolerance)? {
            return Ok(0);
        }
        // A curve entirely on a supporting coordinate line of the projected
        // surface has no interior projected crossing (silhouette convention,
        // shared with the affine natural-domain boundary case above).
        if (0..2).any(|k| {
            let value = curve.control_points[0][k];
            curve.control_points.iter().all(|p| p[k] == value)
                && (surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[k] >= value)
                    || surface
                        .control_points
                        .iter()
                        .flatten()
                        .all(|p| p[k] <= value))
        }) {
            return Ok(0);
        }
        if *work >= 2000000 - 64 {
            return Err(input("B-rep surface occlusion work budget exceeded"));
        }
        *work += 64;
        if let Some(cover) =
            nurbs_core::surface_occlusion::extrusion_cover(surface, curve, tolerance, 64)?
        {
            if domain.contains_closed_rectangle(cover) || domain.on_linear_boundary(cover) {
                return Ok(1);
            }
            let classified = domain.classify(cover, (2000000 - *work).min(4096))?;
            *work += classified.cells;
            if classified.location == nurbs_core::trim_domain::Location::Inside {
                return Ok(1);
            }
        }
        if *work >= 2000000 {
            return Err(input("B-rep surface occlusion work budget exceeded"));
        }
        let report = nurbs_core::surface_occlusion::inspect(
            surface,
            curve,
            tolerance,
            (2000000 - *work).min(512),
        )?;
        *work += report.cells;
        let mut uncertain = false;
        for (uv, cover) in report
            .covers
            .into_iter()
            .map(|uv| (uv, true))
            .chain(report.unresolved.into_iter().map(|uv| (uv, false)))
        {
            if *work >= 2000000 {
                return Err(input("B-rep trim classification work budget exceeded"));
            }
            *work += 1;
            if domain.contains_closed_rectangle(uv) {
                if cover {
                    return Ok(1);
                }
                uncertain = true;
                continue;
            }
            if cover && domain.on_linear_boundary(uv) {
                return Ok(1);
            }
            if *work >= 2000000 {
                return Err(input("B-rep trim classification work budget exceeded"));
            }
            let classification = domain.classify(uv, (2000000 - *work).min(4096))?;
            *work += classification.cells;
            match classification.location {
                nurbs_core::trim_domain::Location::Inside if cover => return Ok(1),
                nurbs_core::trim_domain::Location::Outside => {}
                _ => uncertain = true,
            }
        }
        Ok(if uncertain { -1 } else { 0 })
    }
}
fn curved_face(model: &brep_core::Model, index: usize, view: &str) -> Result<RetainedFace> {
    let mut surface = model.faces[index].surface.clone();
    for p in surface.control_points.iter_mut().flatten() {
        *p = project(p, view).to_vec();
    }
    Ok(RetainedFace::Curved {
        surface,
        domain: brep_core::face_domain::FaceDomain::new(model, index, 1e-9)?,
    })
}
fn retained_faces(model: &brep_core::Model, view: &str) -> Result<Vec<RetainedFace>> {
    let mut faces = Vec::new();
    let mut boundaries = Vec::new();
    for edge in &model.edges {
        for segment in edge.curve.decompose()? {
            let mut curve = segment.definition().clone();
            for p in &mut curve.control_points {
                *p = project(p, view).to_vec();
            }
            if !(0..2).any(|k| {
                curve
                    .control_points
                    .iter()
                    .all(|p| p[k] == curve.control_points[0][k])
            }) {
                continue;
            }
            let mut pieces = vec![curve];
            for _ in 0..if pieces[0].degree == 1 { 0 } else { 3 } {
                let mut split = Vec::new();
                for c in pieces {
                    let [a, b] = c.domain();
                    split.extend(c.split((a + b) * 0.5)?);
                }
                pieces = split;
            }
            boundaries.extend(pieces);
            if boundaries.len() > 160000 {
                return Err(input("B-rep boundary coverage budget exceeded"));
            }
        }
    }
    faces.push(RetainedFace::Boundary(boundaries));
    for (index, face) in model.faces.iter().enumerate() {
        let s = &face.surface;
        let projected: Vec<Vec<_>> = s
            .control_points
            .iter()
            .map(|row| row.iter().map(|p| project(p, view)).collect())
            .collect();
        let independent_v = projected.iter().enumerate().all(|(i, row)| {
            row.iter()
                .enumerate()
                .all(|(j, p)| p[..2] == row[0][..2] && s.weights[i][j] == s.weights[i][0])
        });
        let independent_u = projected.iter().enumerate().all(|(i, row)| {
            row.iter()
                .enumerate()
                .all(|(j, p)| p[..2] == projected[0][j][..2] && s.weights[i][j] == s.weights[0][j])
        });
        // Exact homogeneous independence proves an edge-on surface image even
        // for curved extrusion walls. It cannot hide a projected open region.
        if independent_u || independent_v {
            continue;
        }
        if s.degree_u != 1
            || s.degree_v != 1
            || s.control_points.len() != 2
            || s.control_points[0].len() != 2
            || s.weights.iter().flatten().any(|w| *w != s.weights[0][0])
        {
            faces.push(curved_face(model, index, view)?);
            continue;
        }
        let a = project(&s.control_points[0][0], view);
        let b = project(&s.control_points[1][0], view);
        let c = project(&s.control_points[0][1], view);
        let d = project(&s.control_points[1][1], view);
        if (0..3).any(|k| {
            (d[k] - (b[k] + c[k] - a[k])).abs()
                > 256. * f64::EPSILON * (a[k].abs() + b[k].abs() + c[k].abs() + d[k].abs()).max(1.)
        }) {
            faces.push(curved_face(model, index, view)?);
            continue;
        }
        let uv = [[s.knots_u[1], s.knots_u[2]], [s.knots_v[1], s.knots_v[2]]];
        let mut boundaries = Vec::new();
        for index in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &model.loops[index].coedges {
                for segment in coedge.pcurve.decompose()? {
                    let mut curve = segment.definition().clone();
                    for p in &mut curve.control_points {
                        let u = (p[0] - uv[0][0]) / (uv[0][1] - uv[0][0]);
                        let v = (p[1] - uv[1][0]) / (uv[1][1] - uv[1][0]);
                        *p = vec![
                            a[0] + u * (b[0] - a[0]) + v * (c[0] - a[0]),
                            a[1] + u * (b[1] - a[1]) + v * (c[1] - a[1]),
                            0.,
                        ];
                    }
                    boundaries.push(curve);
                }
            }
        }
        faces.push(RetainedFace::Planar(PlanarFace {
            origin: a,
            u: std::array::from_fn(|k| b[k] - a[k]),
            v: std::array::from_fn(|k| c[k] - a[k]),
            uv: [[s.knots_u[1], s.knots_u[2]], [s.knots_v[1], s.knots_v[2]]],
            domain: brep_core::face_domain::FaceDomain::new(model, index, 1e-9)?,
            boundaries,
        }));
    }
    Ok(faces)
}
fn clip(
    c: Curve,
    triangles: &[[[f64; 3]; 3]],
    faces: &[RetainedFace],
    tol: f64,
    work: &mut usize,
    output: &mut Vec<(Curve, bool)>,
    depth: usize,
) -> Result<()> {
    if output.len() > 8192 || depth > 32 {
        return Err(input("Hidden-line fragment budget exceeded"));
    }
    let min = |k: usize| {
        c.control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::INFINITY, f64::min)
    };
    let max = |k: usize| {
        c.control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let bounds = [min(0), min(1), max(0), max(1)];
    let mut uncertain = false;
    for t in triangles {
        if (0..2).any(|k| {
            t.iter().all(|p| p[k] < bounds[k] - tol) || t.iter().all(|p| p[k] > bounds[k + 2] + tol)
        }) {
            continue;
        }
        *work += 1;
        if *work > 2000000 {
            return Err(input(
                "Hidden-line projection exceeds 2000000 overlap tests",
            ));
        }
        match triangle_state(&c, t, tol) {
            1 => {
                output.push((c, true));
                return Ok(());
            }
            -1 => uncertain = true,
            _ => {}
        }
    }
    for face in faces {
        *work += 1;
        if *work > 2000000 {
            return Err(input("B-rep hidden-line work budget exceeded"));
        }
        match face.state(&c, tol, work)? {
            1 => {
                output.push((c, true));
                return Ok(());
            }
            -1 => uncertain = true,
            _ => {}
        }
    }
    if !uncertain || (bounds[2] - bounds[0]).hypot(bounds[3] - bounds[1]) <= tol {
        output.push((c, false));
        return Ok(());
    }
    let domain = c.domain();
    let [a, b] = c.split((domain[0] + domain[1]) * 0.5)?;
    clip(a, triangles, faces, tol, work, output, depth + 1)?;
    clip(b, triangles, faces, tol, work, output, depth + 1)
}
fn topology_signature(model: &brep_core::Model) -> String {
    format!(
        "{}:{:?}:{:?}",
        model.vertices.len(),
        model.edges.iter().map(|e| e.vertices).collect::<Vec<_>>(),
        model
            .loops
            .iter()
            .map(|l| l
                .coedges
                .iter()
                .map(|c| (c.edge, c.reversed))
                .collect::<Vec<_>>())
            .collect::<Vec<_>>()
    )
}
fn linked_dimensions(v: &Value, bodies: &[Value], view: &str) -> Result<Vec<Value>> {
    let Some(requests) = v.get("dimensions").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    if requests.len() > 64 {
        return Err(input("Drawing exceeds 64 linked dimensions"));
    }
    let mut result = Vec::new();
    let mut prepared = std::collections::BTreeMap::new();
    for request in requests {
        let id: String = field(request, "bodyId")?;
        let indices: [usize; 2] = field(request, "vertices")?;
        let signature: String = field(request, "topologySignature")?;
        let axis: String = field(request, "axis")?;
        if !prepared.contains_key(&id) {
            let matches: Vec<_> = bodies
                .iter()
                .filter(|b| b["id"].as_str() == Some(&id))
                .collect();
            if matches.len() != 1 {
                return Err(input("Linked dimension body is missing or ambiguous"));
            }
            let model: brep_core::Model = field(matches[0], "brep")?;
            let index = brep_core::topology_reference::VertexReferenceIndex::new(&model)?;
            prepared.insert(id.clone(), (model, index));
        }
        let (model, index) = prepared.get(&id).unwrap();
        let mut indices = indices;
        let mut status = ["legacy-index"; 2];
        if request.get("vertexIds").is_some() {
            let ids: [String; 2] = field(request, "vertexIds")?;
            for endpoint in 0..2 {
                let target = brep_core::TopoId::parse(&ids[endpoint])
                    .map_err(|_| input("Invalid dimension vertex identity"))?;
                use brep_core::topology_reference::VertexResolution;
                match index.resolve(target, 100000)? {
                    VertexResolution::Persistent(i) => {
                        indices[endpoint] = i;
                        status[endpoint] = "persistent";
                    }
                    VertexResolution::Followed(i) => {
                        indices[endpoint] = i;
                        status[endpoint] = "followed";
                    }
                    VertexResolution::Missing => {
                        return Err(input(
                            "Linked dimension vertex was deleted or its history is unavailable; reattach that dimension",
                        ));
                    }
                    VertexResolution::Ambiguous(candidates) => {
                        return Err(input(&format!(
                            "Linked dimension endpoint {} has {} surviving vertices after a split; choose a new attachment",
                            endpoint + 1,
                            candidates.len()
                        )));
                    }
                }
            }
        } else if topology_signature(model) != signature {
            return Err(input(
                "Linked dimension topology changed; reattach its vertices",
            ));
        }
        let a = model
            .vertices
            .get(indices[0])
            .ok_or_else(|| input("Missing dimension vertex"))?
            .point;
        let b = model
            .vertices
            .get(indices[1])
            .ok_or_else(|| input("Missing dimension vertex"))?
            .point;
        let p = project(&a, view);
        let q = project(&b, view);
        let value = match axis.as_str() {
            "horizontal" => (q[0] - p[0]).abs(),
            "vertical" => (q[1] - p[1]).abs(),
            _ => return Err(input("Dimension axis must be horizontal or vertical")),
        };
        if value <= 1e-7 {
            return Err(input("Linked dimension collapses in this projection"));
        }
        result.push(json!({"a":[p[0],p[1]],"b":[q[0],q[1]],"valueMm":value,"axis":axis,"bodyId":id,"vertices":indices,"vertexIds":[model.1.vertices[indices[0]],model.1.vertices[indices[1]]],"referenceStatus":status}));
    }
    Ok(result)
}

/// Frame generation uses native geometry and deterministic painter ordering;
/// browser timing/GPU capture never determines which timeline frames exist.
pub fn frame(v: &Value) -> Result<Value> {
    let bodies: Vec<Value> = field(v, "bodies")?;
    let yaw: f64 = field(v, "yaw")?;
    let pitch: f64 = field(v, "pitch")?;
    if !yaw.is_finite() || !pitch.is_finite() || bodies.len() > 200 {
        return Err(input("Invalid animation frame camera"));
    }
    let mut faces = Vec::new();
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for body in bodies {
        let mesh: Mesh = field(&body, "mesh")?;
        let color = body["material"]["color"].as_str().unwrap_or("#7094ba");
        if color.len() != 7
            || !color.starts_with('#')
            || !color[1..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(input("Invalid animation frame color"));
        }
        for (p, _, depth) in polygon_core::mesh_editor::project_faces(&mesh, yaw, pitch, 0.)? {
            for q in p {
                for k in 0..2 {
                    bounds[k] = bounds[k].min(q[k]);
                    bounds[k + 2] = bounds[k + 2].max(q[k]);
                }
            }
            faces.push((p, depth, color.to_owned()));
            if faces.len() > 50000 {
                return Err(input("Animation frame exceeds 50000 mesh triangles"));
            }
        }
    }
    if faces.is_empty() {
        return Err(input("Animation frame has no geometry"));
    }
    faces.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut paths = String::new();
    for (p, _, color) in faces {
        writeln!(
            paths,
            "<path d=\"M{} {} L{} {} L{} {} Z\" fill=\"{color}\"/>",
            p[0][0], p[0][1], p[1][0], p[1][1], p[2][0], p[2][1]
        )
        .unwrap();
    }
    Ok(json!({"paths":paths,"bounds":bounds}))
}
#[cfg(test)]
#[path = "drawing_projection/tests/tests_integration.rs"]
mod tests;

#[path = "drawing_projection/drawing_layout.rs"]
mod drawing_layout;
pub use drawing_layout::drawing;

//! Orthographic retained edges with bounded trimmed NURBS-surface occlusion
//! and an explicitly selected mesh hidden-line mode.
use crate::{Result, field, input};
use nurbs_core::curve::Curve;
use polygon_core::Mesh;
use std::fmt::Write;
use value_codec::{Value, json};
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
    let mut d = format!("M{} {} ", a[0], a[1]);
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
            let center: [f64; 2] =
                [0, 1].map(|k| ((a[k] + b[k]) * 0.5 - w * w * p[1][k]) / (1. - w * w));
            let u: [f64; 2] = [0, 1].map(|k| w * (p[1][k] - center[k]));
            let v: [f64; 2] = [0, 1].map(|k| (b[k] - a[k]) / (2. * (1. - w * w).sqrt()));
            let m = [
                u[0] * u[0] + v[0] * v[0],
                u[0] * u[1] + v[0] * v[1],
                u[1] * u[1] + v[1] * v[1],
            ];
            let radius = ((m[0] - m[2]).powi(2) + 4. * m[1] * m[1]).sqrt();
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
                let mut angles = vec![-end, end];
                for k in -2..=2 {
                    let t = phase + f64::from(k) * std::f64::consts::PI;
                    if -end < t && t < end {
                        angles.push(t);
                    }
                }
                angles.sort_by(f64::total_cmp);
                for t in angles.into_iter().skip(1) {
                    write!(
                        d,
                        "L{} {} ",
                        center[0] + u[0] * t.cos() + v[0] * t.sin(),
                        center[1] + u[1] * t.cos() + v[1] * t.sin()
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
    let cross = |a: [f64; 3], b: [f64; 3], p: &Vec<f64>| {
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
        let values: Vec<_> = c
            .control_points
            .iter()
            .map(|p| sign * cross(u, v, p))
            .collect();
        if values.iter().all(|v| *v < -margin) {
            return 0;
        }
        definitely_inside &= values.iter().all(|v| *v >= margin);
    }
    let dz: Vec<_> = c
        .control_points
        .iter()
        .map(|p| {
            let s = ((p[0] - a[0]) * (z[1] - a[1]) - (p[1] - a[1]) * (z[0] - a[0])) / determinant;
            let u = ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])) / determinant;
            p[2] - (a[2] + s * (b[2] - a[2]) + u * (z[2] - a[2]))
        })
        .collect();
    if dz.iter().all(|d| *d >= -tol) {
        return 0;
    }
    if definitely_inside && dz.iter().all(|d| *d < -tol) {
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
pub fn drawing(v: &Value) -> Result<Value> {
    let bodies: Vec<Value> = field(v, "bodies")?;
    let view: String = field(v, "view")?;
    let hidden: String = field(v, "hidden")?;
    if bodies.is_empty()
        || bodies.len() > 200
        || !matches!(view.as_str(), "top" | "front" | "right")
        || !matches!(hidden.as_str(), "show" | "hide" | "dash")
    {
        return Err(input("Invalid drawing projection options"));
    }
    let occlusion = v.get("occlusion").and_then(Value::as_str).unwrap_or("mesh");
    if !matches!(occlusion, "mesh" | "brep") {
        return Err(input("Invalid drawing occlusion mode"));
    }
    let tolerance = if occlusion == "brep" { 1e-5 } else { 0.02 };
    let mut faces = Vec::new();
    let mut triangles = Vec::new();
    let mut curves = Vec::new();
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut approximate = false;
    let dimensions = linked_dimensions(v, &bodies, &view)?;
    let mut references = Vec::new();
    for body in &bodies {
        let mesh: Mesh = field(&body, "mesh")?;
        mesh.validate()?;
        let points: Vec<_> = mesh
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| project(p, &view))
            .collect();
        for p in &points {
            for k in 0..2 {
                bounds[k] = bounds[k].min(p[k]);
                bounds[k + 2] = bounds[k + 2].max(p[k]);
            }
        }
        triangles.extend(
            mesh.indices
                .as_chunks::<3>()
                .0
                .iter()
                .map(|i| i.map(|i| points[i])),
        );
        if triangles.len() > 100000 {
            return Err(input("Drawing exceeds 100000 triangles"));
        }
        if body.get("brep").is_some_and(|b| !b.is_null()) {
            let model: brep_core::Model = field(&body, "brep")?;
            model.validate()?;
            references.push(json!({"bodyId":body["id"],"topologySignature":topology_signature(&model),"vertices":model.vertices.iter().map(|v|project(&v.point,&view)).collect::<Vec<_>>(),"vertexIds":model.1.vertices.clone()}));
            if occlusion == "brep" {
                faces.extend(retained_faces(&model, &view)?);
                if faces.len() > 20000 {
                    return Err(input("Drawing exceeds 20000 retained faces"));
                }
            }
            for edge in &model.edges {
                for segment in edge.curve.decompose()? {
                    let mut c = segment.definition().clone();
                    for p in &mut c.control_points {
                        *p = project(p, &view).to_vec();
                    }
                    if curve_path(&c).is_ok() {
                        curves.push(c);
                    } else {
                        approximate = true;
                        let report = nurbs_core::tessellation::curve_tessellation::tessellate(
                            &c, 0.01, 8192,
                        )?;
                        if !report.within_tolerance {
                            return Err(input("Drawing curve projection tolerance is unresolved"));
                        }
                        for s in report.segments {
                            curves.push(Curve::from_polyline(s.points.to_vec())?);
                        }
                    }
                }
            }
        } else {
            if occlusion == "brep" {
                return Err(input(
                    "B-rep surface occlusion requires retained geometry on every body",
                ));
            }
            approximate = true;
            let topology =
                mesh_topology::planar::topology(mesh.view()).map_err(|e| input(e.message))?;
            for edge in topology.edges {
                curves.push(Curve::from_polyline(vec![
                    points[edge.a].to_vec(),
                    points[edge.b].to_vec(),
                ])?);
            }
        }
    }
    if occlusion == "brep" {
        bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for c in &curves {
            for k in 0..2 {
                let mut parameters = c.domain().to_vec();
                parameters.extend(nurbs_core::curve_extrema::axis_extrema(c, k)?);
                for t in parameters {
                    let value = c.evaluate(t)?.point[k];
                    bounds[k] = bounds[k].min(value);
                    bounds[k + 2] = bounds[k + 2].max(value);
                }
            }
        }
    }
    if curves.len() > 20000 {
        return Err(input("Drawing exceeds 20000 edge spans"));
    }
    let mut paths = Vec::new();
    let mut work = 0;
    for c in curves {
        let source = c.clone();
        let mut pieces = Vec::new();
        if hidden == "show" {
            pieces.push((c, false));
        } else {
            clip(
                c,
                if occlusion == "mesh" { &triangles } else { &[] },
                &faces,
                tolerance,
                &mut work,
                &mut pieces,
                0,
            )?;
        }
        // Subdivision proves visibility; merge consecutive equal states back
        // into source trims instead of exporting thousands of tiny SVG paths.
        let mut ranges: Vec<(f64, f64, bool)> = Vec::new();
        for (piece, hidden) in pieces {
            let [a, b] = piece.domain();
            if let Some(last) = ranges.last_mut()
                && last.2 == hidden
                && last.1 == a
            {
                last.1 = b;
            } else {
                ranges.push((a, b, hidden));
            }
        }
        for (a, b, is_hidden) in ranges {
            let c = if [a, b] == source.domain() {
                source.clone()
            } else {
                source.trim(a, b)?
            };
            if hidden == "hide" && is_hidden {
                continue;
            }
            paths.push(json!({"d":curve_path(&c)?,"hidden":is_hidden}));
            if paths.len() > 20000 {
                return Err(input("Drawing exceeds 20000 path fragments"));
            }
        }
    }
    Ok(
        json!({"paths":paths,"bounds":bounds,"approximate":approximate,"occlusionToleranceMm":tolerance,"occlusionGeometry":occlusion,"dimensions":dimensions,"references":references}),
    )
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
mod tests {
    use super::*;
    fn body(a: [f64; 3], b: [f64; 3]) -> Value {
        let model = brep_core::cuboid(a, b).unwrap();
        let mesh = crate::brep::nurbs(&model, 1).unwrap().built.mesh;
        json!({"brep":model,"mesh":mesh})
    }
    #[test]
    fn curved_cylinder_hides_a_body_behind_its_retained_wall() {
        let model = brep_core::cylinder(2., 4.).unwrap();
        let front = json!({"brep":model,"mesh":crate::brep::nurbs(&model,1).unwrap().built.mesh});
        let behind = body([-0.2, 4., 1.], [0.2, 5., 2.]);
        let source: brep_core::Model = field(&behind, "brep").unwrap();
        let mut vertical_paths = Vec::new();
        for edge in &source.edges {
            if edge.curve.control_points.first().unwrap()[2]
                == edge.curve.control_points.last().unwrap()[2]
            {
                continue;
            }
            let mut curve = edge.curve.clone();
            for p in &mut curve.control_points {
                *p = project(p, "front").to_vec();
            }
            vertical_paths.push(curve_path(&curve).unwrap());
        }
        assert_eq!(vertical_paths.len(), 4);
        let result = drawing(
            &json!({"bodies":[front,behind],"view":"front","hidden":"dash","occlusion":"brep"}),
        )
        .unwrap();
        for path in vertical_paths {
            assert!(
                result["paths"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["d"].as_str() == Some(&path) && p["hidden"] == true)
            );
        }
        assert!(
            result["paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["hidden"] == true)
        );
    }
    #[test]
    fn sphere_hides_separate_body_without_display_mesh() {
        let model = brep_core::sphere(2.).unwrap();
        let front = json!({"brep":model,"mesh":crate::brep::nurbs(&model,1).unwrap().built.mesh});
        let behind = body([0.3, 0.3, -4.], [0.5, 0.5, -3.]);
        let alone = drawing(
            &json!({"bodies":[behind.clone()],"view":"top","hidden":"show","occlusion":"brep"}),
        )
        .unwrap();
        for view in ["front", "right"] {
            drawing(&json!({"bodies":[front.clone(),behind.clone()],"view":view,"hidden":"dash","occlusion":"brep"})).unwrap_or_else(|e|panic!("{view}: {e:?}"));
        }
        let result = drawing(
            &json!({"bodies":[front,behind],"view":"top","hidden":"dash","occlusion":"brep"}),
        )
        .unwrap();
        for path in alone["paths"].as_array().unwrap() {
            assert!(
                result["paths"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["d"] == path["d"] && p["hidden"] == true),
                "missing hidden path: {}",
                path["d"]
            );
        }
    }
    #[test]
    fn curved_retained_face_uses_uniform_surface_proof() {
        let model = brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let surface = nurbs_core::surface::Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![0.5, 0., 3.], vec![0.5, 1., 3.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        // The cuboid contributes only its authored unit-square UV trim; the
        // surface under test is the independent curved chart with that domain.
        let face = RetainedFace::Curved {
            surface,
            domain: brep_core::face_domain::FaceDomain::new(&model, 0, 1e-9).unwrap(),
        };
        let behind = Curve::from_polyline(vec![vec![0.4, 0.4, -1.], vec![0.6, 0.6, -1.]]).unwrap();
        let mut work = 0;
        assert_eq!(face.state(&behind, 1e-5, &mut work).unwrap(), 1);
        assert!(work > 0);
        let front = Curve::from_polyline(vec![vec![0.4, 0.4, 5.], vec![0.6, 0.6, 5.]]).unwrap();
        assert_eq!(face.state(&front, 1e-5, &mut work).unwrap(), 0);
    }
    #[test]
    fn curved_chart_uv_hole_keeps_the_body_behind_it_visible() {
        let outer =
            brep_core::sketch::polygon_wire(vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]).unwrap();
        let hole = brep_core::sketch::polygon_wire(vec![
            [0.35, 0.35],
            [0.65, 0.35],
            [0.65, 0.65],
            [0.35, 0.65],
        ])
        .unwrap();
        let loops = brep_core::planar_trim::orient_even_odd(&[outer, hole], 1e-7).unwrap();
        let model = brep_core::prism::extrude(&loops, 0., 1.).unwrap();
        let index = model
            .faces
            .iter()
            .position(|face| !face.holes.is_empty())
            .unwrap();
        let surface = nurbs_core::surface::Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![0.5, 0., 3.], vec![0.5, 1., 3.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let face = RetainedFace::Curved {
            surface,
            domain: brep_core::face_domain::FaceDomain::new(&model, index, 1e-9).unwrap(),
        };
        let through_hole =
            Curve::from_polyline(vec![vec![0.45, 0.5, -1.], vec![0.55, 0.5, -1.]]).unwrap();
        let behind_material =
            Curve::from_polyline(vec![vec![0.1, 0.1, -1.], vec![0.2, 0.1, -1.]]).unwrap();
        let mut work = 0;
        assert_eq!(face.state(&through_hole, 1e-5, &mut work).unwrap(), 0);
        assert_eq!(face.state(&behind_material, 1e-5, &mut work).unwrap(), 1);
    }
    #[test]
    fn retained_box_edges_and_hidden_lines_have_native_proof() {
        let b = body([0., 0., 0.], [10., 8., 6.]);
        let all = drawing(&json!({"bodies":[b.clone()],"view":"top","hidden":"show"})).unwrap();
        assert_eq!(all["paths"].as_array().unwrap().len(), 12);
        assert_eq!(all["approximate"].as_bool(), Some(false));
        let dashed_box =
            drawing(&json!({"bodies":[b.clone()],"view":"top","hidden":"dash"})).unwrap();
        assert_eq!(dashed_box["paths"].as_array().unwrap().len(), 12);
        let behind = body([2., 2., -4.], [8., 6., -2.]);
        let dashed =
            drawing(&json!({"bodies":[b.clone(),behind.clone()],"view":"top","hidden":"dash"}))
                .unwrap();
        assert!(
            dashed["paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["hidden"].as_bool() == Some(true))
        );
        let hidden = drawing(&json!({"bodies":[b,behind],"view":"top","hidden":"hide"})).unwrap();
        assert!(
            hidden["paths"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["hidden"].as_bool() == Some(false))
        );
        assert!(
            hidden["paths"].as_array().unwrap().len() < dashed["paths"].as_array().unwrap().len()
        );
    }
    #[test]
    fn planar_brep_occlusion_is_independent_of_display_mesh() {
        let front = body([0., 0., 0.], [10., 8., 6.]);
        let mut behind = body([2., 2., -4.], [8., 6., -2.]);
        let baseline=drawing(&json!({"bodies":[front.clone(),behind.clone()],"view":"top","hidden":"dash","occlusion":"brep"})).unwrap();
        assert!(
            baseline["paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["hidden"] == true)
        );
        behind["mesh"] = front["mesh"].clone();
        let changed = drawing(
            &json!({"bodies":[front,behind],"view":"top","hidden":"dash","occlusion":"brep"}),
        )
        .unwrap();
        assert_eq!(baseline["paths"], changed["paths"]);
        assert_eq!(baseline["bounds"], changed["bounds"]);
    }
    #[test]
    fn retained_planar_hole_does_not_occlude_a_body_behind_it() {
        let outer = brep_core::sketch::polygon_wire(vec![[0., 0.], [10., 0.], [10., 8.], [0., 8.]])
            .unwrap();
        let mut inner = brep_core::sketch::circle_wire(1.).unwrap();
        for curve in &mut inner {
            for p in &mut curve.control_points {
                p[0] += 5.;
                p[1] += 4.;
            }
        }
        let loops = brep_core::planar_trim::orient_even_odd(&[outer, inner], 1e-7).unwrap();
        let model = brep_core::prism::extrude(&loops, 0., 6.).unwrap();
        let front = json!({"brep":model,"mesh":crate::brep::nurbs(&model,2).unwrap().built.mesh});
        let behind = body([4.7, 3.7, -4.], [5.3, 4.3, -2.]);
        let all = drawing(
            &json!({"bodies":[behind.clone()],"view":"top","hidden":"show","occlusion":"brep"}),
        )
        .unwrap();
        let result = drawing(
            &json!({"bodies":[front,behind],"view":"top","hidden":"dash","occlusion":"brep"}),
        )
        .unwrap();
        for path in all["paths"].as_array().unwrap() {
            assert!(
                result["paths"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["d"] == path["d"] && p["hidden"] == false)
            );
        }
    }
    #[test]
    fn durable_dimensions_survive_subdivision_reordering_and_transform() {
        let source = brep_core::cuboid([0., 0., 0.], [2., 1., 1.]).unwrap();
        let second = source
            .vertices
            .iter()
            .position(|v| v.point[0] != source.vertices[0].point[0])
            .unwrap();
        let ids = [source.1.vertices[0], source.1.vertices[second]];
        let request = json!({"bodyId":"part","vertices":[0,second],"topologySignature":topology_signature(&source),"vertexIds":ids,"axis":"horizontal"});
        let wire =
            brep_core::sketch::polygon_wire(vec![[0., 0.], [1., 0.], [2., 0.], [2., 1.], [0., 1.]])
                .unwrap();
        let mut changed = brep_core::prism::extrude(&[wire], 0., 1.).unwrap();
        changed.inherit_topology_ids(&[&source]);
        changed.vertices.swap(0, 1);
        changed.1.vertices.swap(0, 1);
        for edge in &mut changed.edges {
            for v in &mut edge.vertices {
                if *v == 0 {
                    *v = 1;
                } else if *v == 1 {
                    *v = 0;
                }
            }
        }
        let changed = brep_core::transform::affine(
            &changed,
            [
                [2., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let body = json!({"id":"part","brep":changed,"mesh":crate::brep::nurbs(&changed,1).unwrap().built.mesh});
        let result =
            drawing(&json!({"bodies":[body],"view":"top","hidden":"show","dimensions":[request]}))
                .unwrap();
        assert_eq!(result["dimensions"][0]["valueMm"].as_f64(), Some(4.));
        assert_eq!(result["dimensions"][0]["vertexIds"], json!(ids));
        assert_eq!(
            result["dimensions"][0]["referenceStatus"],
            json!(["persistent", "persistent"])
        );
        assert_eq!(
            result["references"][0]["vertexIds"]
                .as_array()
                .unwrap()
                .len(),
            10
        );
    }
    #[test]
    fn durable_dimension_split_or_deleted_vertex_requires_explicit_reattachment() {
        let mut model = brep_core::cuboid([0., 0., 0.], [2., 1., 1.]).unwrap();
        let ids = [model.1.vertices[0], model.1.vertices[1]];
        let request = json!({"bodyId":"part","vertices":[0,1],"topologySignature":topology_signature(&model),"vertexIds":ids,"axis":"horizontal"});
        model.1.change_set.changes.push(brep_core::TopologyChange {
            kind: brep_core::ChangeKind::Split,
            topo_kind: brep_core::TopoKind::Vertex,
            parents: vec![ids[0]],
            children: ids.to_vec(),
            provenance: brep_core::ChangeProvenance {
                operation: "drawing-test".into(),
                operand: None,
                occurrence: "part".into(),
            },
            role: "vertex".into(),
            anchor: None,
        });
        let body = json!({"id":"part","brep":model,"mesh":crate::brep::nurbs(&model,1).unwrap().built.mesh});
        let error = drawing(
            &json!({"bodies":[body],"view":"top","hidden":"show","dimensions":[request.clone()]}),
        )
        .unwrap_err();
        assert!(error.message.contains("surviving vertices"));
        let mut replacement = self::body([10., 0., 0.], [12., 1., 1.]);
        replacement["id"] = json!("part");
        let error = drawing(
            &json!({"bodies":[replacement],"view":"top","hidden":"show","dimensions":[request]}),
        )
        .unwrap_err();
        assert!(error.message.contains("deleted"));
    }
    #[test]
    fn linked_vertex_dimensions_recompute_and_reject_changed_topology() {
        let mut b = body([0., 0., 0.], [10., 8., 6.]);
        b["id"] = json!("part");
        let first = drawing(&json!({"bodies":[b.clone()],"view":"top","hidden":"show"})).unwrap();
        let signature = &first["references"][0]["topologySignature"];
        let model: brep_core::Model = field(&b, "brep").unwrap();
        let pair = (0..model.vertices.len())
            .flat_map(|i| (i + 1..model.vertices.len()).map(move |j| [i, j]))
            .find(|p| (model.vertices[p[0]].point[0] - model.vertices[p[1]].point[0]).abs() > 1.)
            .unwrap();
        let request = json!({"bodyId":"part","vertices":pair,"topologySignature":signature,"axis":"horizontal"});
        let original = drawing(
            &json!({"bodies":[b],"view":"top","hidden":"show","dimensions":[request.clone()]}),
        )
        .unwrap();
        let mut changed = body([0., 0., 0.], [20., 8., 6.]);
        changed["id"] = json!("part");
        let next=drawing(&json!({"bodies":[changed.clone()],"view":"top","hidden":"show","dimensions":[request.clone()]})).unwrap();
        assert_eq!(
            next["dimensions"][0]["valueMm"].as_f64().unwrap(),
            2. * original["dimensions"][0]["valueMm"].as_f64().unwrap()
        );
        let mut invalid = request;
        invalid["topologySignature"] = json!("stale");
        assert!(
            drawing(
                &json!({"bodies":[changed],"view":"top","hidden":"show","dimensions":[invalid]})
            )
            .is_err()
        );
    }
    #[test]
    fn original_cubic_and_conic_paths_are_not_polyline_approximations() {
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 1., 0.]]).unwrap();
        assert!(curve_path(&c).unwrap().contains('L'));
        let c = Curve {
            periodic: false,
            degree: 3,
            control_points: vec![
                vec![0., 0., 0.],
                vec![0., 1., 0.],
                vec![1., 1., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1.; 4],
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        };
        assert!(curve_path(&c).unwrap().contains('C'));
        let c = Curve {
            periodic: false,
            degree: 2,
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1., 0.5_f64.sqrt(), 1.],
            knots: vec![0., 0., 0., 1., 1., 1.],
        };
        assert!(curve_path(&c).unwrap().contains('A'));
    }
}

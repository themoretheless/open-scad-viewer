//! Orthographic retained edge projection and bounded mesh hidden-line clipping.
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
fn retained_planes(model: &brep_core::Model, view: &str) -> Result<Vec<PlanarFace>> {
    let mut faces = Vec::new();
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
            return Err(input(
                "B-rep surface occlusion currently requires affine planar faces; choose mesh mode for curved surfaces",
            ));
        }
        let a = project(&s.control_points[0][0], view);
        let b = project(&s.control_points[1][0], view);
        let c = project(&s.control_points[0][1], view);
        let d = project(&s.control_points[1][1], view);
        if (0..3).any(|k| {
            (d[k] - (b[k] + c[k] - a[k])).abs()
                > 256. * f64::EPSILON * (a[k].abs() + b[k].abs() + c[k].abs() + d[k].abs()).max(1.)
        }) {
            return Err(input("B-rep surface is bilinear rather than affine planar"));
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
        faces.push(PlanarFace {
            origin: a,
            u: std::array::from_fn(|k| b[k] - a[k]),
            v: std::array::from_fn(|k| c[k] - a[k]),
            uv: [[s.knots_u[1], s.knots_u[2]], [s.knots_v[1], s.knots_v[2]]],
            domain: brep_core::face_domain::FaceDomain::new(model, index, 1e-9)?,
            boundaries,
        });
    }
    Ok(faces)
}
fn clip(
    c: Curve,
    triangles: &[[[f64; 3]; 3]],
    faces: &[PlanarFace],
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
    for request in requests {
        let id: String = field(request, "bodyId")?;
        let indices: [usize; 2] = field(request, "vertices")?;
        let signature: String = field(request, "topologySignature")?;
        let axis: String = field(request, "axis")?;
        let body = bodies
            .iter()
            .find(|b| b["id"].as_str() == Some(&id))
            .ok_or_else(|| input("Linked dimension body no longer exists"))?;
        let model: brep_core::Model = field(body, "brep")?;
        model.validate()?;
        if topology_signature(&model) != signature {
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
        result.push(json!({"a":[p[0],p[1]],"b":[q[0],q[1]],"valueMm":value,"axis":axis,"bodyId":id,"vertices":indices}));
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
            references.push(json!({"bodyId":body["id"],"topologySignature":topology_signature(&model),"vertices":model.vertices.iter().map(|v|project(&v.point,&view)).collect::<Vec<_>>()}));
            if occlusion == "brep" {
                faces.extend(retained_planes(&model, &view)?);
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

use super::*;

/// One recognized box face: an affine rectangle `P(u,v) = origin + u U + v V`
/// with its outward unit normal.
#[derive(Clone, Debug)]
pub(super) struct BoxFace {
    pub(super) origin: [f64; 3],
    pub(super) u: [f64; 3],
    pub(super) v: [f64; 3],
    pub(super) u_len: f64,
    pub(super) v_len: f64,
    /// Outward unit normal of the solid at this face.
    pub(super) outward: [f64; 3],
}

impl BoxFace {
    pub(super) fn uv_of(&self, p: [f64; 3]) -> [f64; 2] {
        let rel = sub(p, self.origin);
        [
            dot(rel, self.u) / (self.u_len * self.u_len),
            dot(rel, self.v) / (self.v_len * self.v_len),
        ]
    }
    pub(super) fn point_at(&self, uv: [f64; 2]) -> [f64; 3] {
        std::array::from_fn(|k| self.origin[k] + uv[0] * self.u[k] + uv[1] * self.v[k])
    }
}

pub(super) struct CanonicalBox<'m> {
    pub(super) model: &'m Model,
    pub(super) faces: Vec<BoxFace>,
    pub(super) error: f64,
    /// Box edge -> the two faces at it.
    pub(super) edge_faces: Vec<Vec<usize>>,
}

/// Recognizes the six-face cuboid, rigidly placed. Returns `None` for
/// anything else; the caller then refuses through the analytic matrix.
pub(super) fn recognize_box(model: &Model) -> Result<Option<CanonicalBox<'_>>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 6
        || model.vertices.len() != 8
        || model.edges.len() != 12
        || !model.shells[0].closed
    {
        return Ok(None);
    }
    let centroid = {
        let mut c = [0.; 3];
        for vertex in &model.vertices {
            for axis in 0..3 {
                c[axis] += vertex.point[axis] / 8.;
            }
        }
        c
    };
    let mut faces = Vec::with_capacity(6);
    let mut error: f64 = 0.;
    let mut edge_faces = vec![Vec::new(); 12];
    for (index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        if surface.degree_u != 1
            || surface.degree_v != 1
            || surface.periodic_u
            || surface.periodic_v
            || surface.knots_u != [0., 0., 1., 1.]
            || surface.knots_v != [0., 0., 1., 1.]
            || surface.control_points.len() != 2
            || surface.control_points.iter().any(|row| row.len() != 2)
            || surface.weights != [[1., 1.], [1., 1.]]
            || !face.holes.is_empty()
        {
            return Ok(None);
        }
        let wire = &model.loops[face.outer];
        if wire.coedges.len() != 4 {
            return Ok(None);
        }
        let boundary = [
            ([0., 0.], [1., 0.]),
            ([1., 0.], [1., 1.]),
            ([1., 1.], [0., 1.]),
            ([0., 1.], [0., 0.]),
        ];
        let mut seen = [false; 4];
        for coedge in &wire.coedges {
            let mut hit = false;
            for (k, &(from, to)) in boundary.iter().enumerate() {
                if !seen[k] && sphere_sphere::axis_line(&coedge.pcurve, from, to) {
                    seen[k] = true;
                    hit = true;
                    break;
                }
            }
            if !hit || model.edges[coedge.edge].curve.degree != 1 {
                return Ok(None);
            }
            edge_faces[coedge.edge].push(index);
        }
        let p = &surface.control_points;
        let p00 = point3(&p[0][0]);
        let p10 = point3(&p[1][0]);
        let p01 = point3(&p[0][1]);
        let p11 = point3(&p[1][1]);
        let u = sub(p10, p00);
        let v = sub(p01, p00);
        let u_len = norm(u);
        let v_len = norm(v);
        if !(1e-5..=1e6).contains(&u_len) || !(1e-5..=1e6).contains(&v_len) {
            return Ok(None);
        }
        let scale = u_len.max(v_len);
        let affine_deviation = dist(
            p11,
            [
                p00[0] + u[0] + v[0],
                p00[1] + u[1] + v[1],
                p00[2] + u[2] + v[2],
            ],
        );
        if !affine_deviation.is_finite() || affine_deviation > RECOGNITION * scale + 1e-12 {
            return Ok(None);
        }
        error = error.max(affine_deviation);
        let skew = dot(u, v).abs() / u_len.min(v_len);
        if !skew.is_finite() || skew > RECOGNITION * scale {
            return Ok(None);
        }
        error = error.max(skew);
        let normal = cross(u, v);
        let n_len = norm(normal);
        if !n_len.is_finite() || n_len <= 0. {
            return Ok(None);
        }
        let surface_normal = normal.map(|x| x / n_len);
        let outward = if face_reversed(model, index) {
            surface_normal.map(|x| -x)
        } else {
            surface_normal
        };
        // The face must look away from the box centroid: this is what makes the
        // half-space classification below a solid classification.
        if dot(outward, sub(p00, centroid)) <= RECOGNITION * scale {
            return Ok(None);
        }
        faces.push(BoxFace {
            origin: p00,
            u,
            v,
            u_len,
            v_len,
            outward,
        });
    }
    for i in 0..6 {
        for j in i + 1..6 {
            let d = dot(faces[i].outward, faces[j].outward).abs();
            if d > RECOGNITION && (d - 1.).abs() > RECOGNITION {
                return Ok(None);
            }
        }
    }
    if edge_faces.iter().any(|f| f.len() != 2) {
        return Ok(None);
    }
    Ok(Some(CanonicalBox {
        model,
        faces,
        error,
        edge_faces,
    }))
}

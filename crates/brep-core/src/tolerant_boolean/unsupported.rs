use super::*;

pub(super) fn unsupported(message: impl Into<String>) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}
pub(super) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(super) fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub(super) fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub(super) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(super) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(super) fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
pub(super) fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    (n > 0.).then(|| scale(a, 1. / n))
}
pub(super) fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(sub(a, b))
}

// ---------------------------------------------------------------------------
// Surface access
// ---------------------------------------------------------------------------

pub(super) struct Jet {
    pub(super) p: [f64; 3],
    pub(super) du: [f64; 3],
    pub(super) dv: [f64; 3],
}

type CoedgeSamples = (Vec<[f64; 2]>, usize, bool);
type CoedgeLoops = Vec<Vec<CoedgeSamples>>;

/// Trimmed face with its UV domain, dense boundary polygons and per-coedge
/// samples for locating boundary crossings.
pub(super) struct Chart<'m> {
    pub(super) model: &'m Model,
    pub(super) face: usize,
    pub(super) domain: [[f64; 2]; 2],
    /// Sampled loops (outer first) in UV.
    pub(super) polygons: Vec<Vec<[f64; 2]>>,
    /// For every loop, for every coedge: UV samples along the coedge
    /// (DENSE + 1 points) and the coedge's edge / reversed flag.
    pub(super) coedges: CoedgeLoops,
}

impl<'m> Chart<'m> {
    pub(super) fn new(model: &'m Model, face: usize) -> Result<Self> {
        let f = &model.faces[face];
        let s = &f.surface;
        let domain = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let mut polygons = Vec::new();
        let mut coedges = Vec::new();
        for &wire in std::iter::once(&f.outer).chain(&f.holes) {
            let mut polygon = Vec::new();
            let mut list = Vec::new();
            for coedge in &model.loops[wire].coedges {
                let [a, b] = coedge.pcurve.domain();
                let mut samples = Vec::with_capacity(DENSE + 1);
                for i in 0..=DENSE {
                    let t = a + (b - a) * i as f64 / DENSE as f64;
                    samples.push(pcurve_point(&coedge.pcurve, t)?);
                }
                polygon.extend(samples[..DENSE].iter().copied());
                list.push((samples, coedge.edge, coedge.reversed));
            }
            polygons.push(polygon);
            coedges.push(list);
        }
        Ok(Self {
            model,
            face,
            domain,
            polygons,
            coedges,
        })
    }
    fn surface(&self) -> &'m Surface {
        &self.model.faces[self.face].surface
    }
    pub(super) fn clamp(&self, uv: [f64; 2]) -> [f64; 2] {
        [
            uv[0].clamp(self.domain[0][0], self.domain[0][1]),
            uv[1].clamp(self.domain[1][0], self.domain[1][1]),
        ]
    }
    pub(super) fn jet(&self, uv: [f64; 2]) -> Result<Jet> {
        let uv = self.clamp(uv);
        let e = self.surface().evaluate(uv[0], uv[1])?;
        let (du, dv) = e.first_derivatives().ok_or_else(|| {
            unsupported("Tolerant Boolean: surface derivative undefined at a traced point")
        })?;
        Ok(Jet { p: e.point, du, dv })
    }
    pub(super) fn point(&self, uv: [f64; 2]) -> Result<[f64; 3]> {
        let uv = self.clamp(uv);
        Ok(self.surface().evaluate(uv[0], uv[1])?.point)
    }
    /// Signed containment in the trimmed region: winding parity over the
    /// sampled loops.
    pub(super) fn inside(&self, uv: [f64; 2]) -> bool {
        let mut crossings = 0usize;
        for polygon in &self.polygons {
            for i in 0..polygon.len() {
                let a = polygon[i];
                let b = polygon[(i + 1) % polygon.len()];
                if (a[1] > uv[1]) != (b[1] > uv[1]) {
                    let x = a[0] + (uv[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
                    if x > uv[0] {
                        crossings += 1;
                    }
                }
            }
        }
        crossings % 2 == 1
    }
    /// Distance from a UV point to the nearest boundary sample segment.
    pub(super) fn boundary_distance(&self, uv: [f64; 2]) -> f64 {
        let mut best = f64::INFINITY;
        for polygon in &self.polygons {
            for i in 0..polygon.len() {
                let a = polygon[i];
                let b = polygon[(i + 1) % polygon.len()];
                best = best.min(segment_distance(a, b, uv));
            }
        }
        best
    }
    /// The coedge whose sampled polyline the UV segment `from -> to`
    /// crosses first: (loop, coedge index, coedge-local parameter in [0, 1]).
    pub(super) fn crossing(&self, from: [f64; 2], to: [f64; 2]) -> Option<(usize, usize, f64)> {
        let mut best: Option<(f64, (usize, usize, f64))> = None;
        for (li, list) in self.coedges.iter().enumerate() {
            for (ci, (samples, _, _)) in list.iter().enumerate() {
                for k in 0..DENSE {
                    let (a, b) = (samples[k], samples[k + 1]);
                    if let Some((s, u)) = segments_intersect(from, to, a, b) {
                        let local = (k as f64 + u) / DENSE as f64;
                        if best.map(|(bs, _)| s < bs).unwrap_or(true) {
                            best = Some((s, (li, ci, local)));
                        }
                    }
                }
            }
        }
        best.map(|(_, hit)| hit)
    }
}

pub(super) fn segment_distance(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len2 = d[0] * d[0] + d[1] * d[1];
    let t = if len2 > 0. {
        (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / len2).clamp(0., 1.)
    } else {
        0.
    };
    let q = [a[0] + t * d[0], a[1] + t * d[1]];
    (q[0] - p[0]).hypot(q[1] - p[1])
}

/// Parameters (s along p->q, u along a->b) of a proper crossing.
pub(super) fn segments_intersect(p: [f64; 2], q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> Option<(f64, f64)> {
    let r = [q[0] - p[0], q[1] - p[1]];
    let d = [b[0] - a[0], b[1] - a[1]];
    let den = r[0] * d[1] - r[1] * d[0];
    if den.abs() < 1e-18 {
        return None;
    }
    let w = [a[0] - p[0], a[1] - p[1]];
    let s = (w[0] * d[1] - w[1] * d[0]) / den;
    let u = (w[0] * r[1] - w[1] * r[0]) / den;
    ((-1e-9..=1. + 1e-9).contains(&s) && (-1e-9..=1. + 1e-9).contains(&u))
        .then(|| (s.clamp(0., 1.), u.clamp(0., 1.)))
}

/// Solve a small dense linear system by Gaussian elimination with partial
/// pivoting; `None` when singular.
pub(super) fn solve(mut m: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let n = rhs.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
        if m[pivot][col].abs() < 1e-300 {
            return None;
        }
        m.swap(col, pivot);
        rhs.swap(col, pivot);
        for row in col + 1..n {
            let f = m[row][col] / m[col][col];
            if f == 0. {
                continue;
            }
            for k in col..n {
                m[row][k] -= f * m[col][k];
            }
            rhs[row] -= f * rhs[col];
        }
    }
    let mut x = vec![0.; n];
    for row in (0..n).rev() {
        let mut acc = rhs[row];
        for k in row + 1..n {
            acc -= m[row][k] * x[k];
        }
        if m[row][row].abs() < 1e-300 {
            return None;
        }
        x[row] = acc / m[row][row];
    }
    Some(x)
}

// ---------------------------------------------------------------------------
// Surface/surface tracing
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub(super) struct TracePoint {
    pub(super) p: [f64; 3],
    pub(super) a: [f64; 2],
    pub(super) b: [f64; 2],
}

/// Where an open chain ends: on a trim boundary of operand `operand`, at
/// (edge, edge parameter), with the UV on the other face.
#[derive(Clone, Copy, Debug)]
pub(super) struct BoundaryHit {
    pub(super) operand: usize,
    pub(super) edge: usize,
    pub(super) t: f64,
    pub(super) point: [f64; 3],
    pub(super) a: [f64; 2],
    pub(super) b: [f64; 2],
}

pub(super) struct Chain {
    pub(super) points: Vec<TracePoint>,
    pub(super) closed: bool,
    pub(super) start: Option<BoundaryHit>,
    pub(super) end: Option<BoundaryHit>,
}

pub(super) struct Tracer<'m> {
    pub(super) a: Chart<'m>,
    pub(super) b: Chart<'m>,
    pub(super) scale: f64,
    pub(super) step: f64,
}

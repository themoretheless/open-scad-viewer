//! Complete-chart separation of original source face unions. Superset bounds
//! include every retained trim, including root-ended fragments and holes.
//! This is a clearance lower certificate, not an interior material wall chord.
use crate::source_volume::Body;
use nurbs_core::{
    Error, Result,
    surface::Surface,
    surface_distance::{enclosure_distance, rectangle_bounds},
};

pub struct Limits {
    pub cells: usize,
    pub spans: usize,
}
pub struct Certificate<'a> {
    body: &'a Body,
    faces: [Vec<usize>; 2],
    lower_mm: f64,
}
impl<'a> Certificate<'a> {
    pub fn body(&self) -> &'a Body {
        self.body
    }
    pub fn faces(&self) -> [&[usize]; 2] {
        [&self.faces[0], &self.faces[1]]
    }
    pub fn lower_mm(&self) -> f64 {
        self.lower_mm
    }
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub cells: usize,
    pub spans: usize,
    pub uncertain_faces: Option<[usize; 2]>,
    pub uncertain_uv: Option<[[[f64; 2]; 2]; 2]>,
    pub reason: &'static str,
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
// With identical XY controls and weights across each V row, the projected
// source image is independent of V. Prefer splitting U while Z boxes overlap;
// full V coverage is retained in every evaluated rectangle and certificate.
fn xy_independent_v(s: &Surface) -> bool {
    s.control_points
        .iter()
        .zip(&s.weights)
        .all(|(row, weights)| {
            row.iter().all(|p| p[0] == row[0][0] && p[1] == row[0][1])
                && weights.iter().all(|w| *w == weights[0])
        })
}
fn spans(s: &Surface, d: [[f64; 2]; 2]) -> usize {
    let count = |k: &[f64], degree: usize, n: usize, r: [f64; 2]| {
        (degree..n)
            .filter(|&i| k[i] < k[i + 1] && k[i] <= r[1] && k[i + 1] >= r[0])
            .count()
    };
    count(&s.knots_u, s.degree_u, s.control_points.len(), d[0])
        * count(&s.knots_v, s.degree_v, s.control_points[0].len(), d[1])
}
/// Certificates retain every selected pair. An exhausted or unresolved pair
/// refuses; chart clearance never creates a trimmed-face upper witness.
pub fn qualify<'a>(
    body: &'a Body,
    faces: [&[usize]; 2],
    minimum_mm: f64,
    limits: Limits,
) -> Result<Report<'a>> {
    let shell = body.geometry().shell();
    if !minimum_mm.is_finite()
        || minimum_mm <= 0.
        || !(1..=100000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.spans)
        || faces.iter().any(|g| {
            g.is_empty()
                || g.len() > shell.faces().len()
                || g.iter().any(|&f| f >= shell.faces().len())
                || g.iter()
                    .copied()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != g.len()
        })
        || faces[0].iter().any(|f| faces[1].contains(f))
    {
        return Err(Error::new(
            "BREP_SOURCE_FACE_GAP",
            "Choose disjoint owned face groups, a positive gap and bounded work",
        ));
    }
    let surface = |f: usize| shell.faces()[f][0].edges()[0].surface();
    let mut out = Report {
        certificate: None,
        cells: 0,
        spans: 0,
        uncertain_faces: None,
        uncertain_uv: None,
        reason: "source-face-gap-work-limit",
    };
    let mut lower = f64::INFINITY;
    for &a in faces[0] {
        for &b in faces[1] {
            let surfaces = [surface(a), surface(b)];
            let natural = surfaces.map(domain);
            let mut queue = vec![natural];
            while let Some(uv) = queue.pop() {
                out.uncertain_faces = Some([a, b]);
                out.uncertain_uv = Some(uv);
                let work = spans(surfaces[0], uv[0]) + spans(surfaces[1], uv[1]);
                if out.cells == limits.cells || work > limits.spans - out.spans {
                    return Ok(out);
                }
                out.cells += 1;
                out.spans += work;
                let boxes = [
                    rectangle_bounds(surfaces[0], uv[0])?,
                    rectangle_bounds(surfaces[1], uv[1])?,
                ];
                let bound = enclosure_distance(&boxes[0], &boxes[1])?.0;
                if bound >= minimum_mm {
                    lower = lower.min(bound);
                    continue;
                }
                let projected = surfaces.iter().all(|s| xy_independent_v(s))
                    && boxes[0][2][0] <= boxes[1][2][1]
                    && boxes[1][2][0] <= boxes[0][2][1];
                let (side, axis) = (0..2)
                    .flat_map(|side| (0..2).map(move |axis| (side, axis)))
                    .filter(|&(_, axis)| !projected || axis == 0)
                    .max_by(|&(s, a), &(t, b)| {
                        let width = |(s, a): (usize, usize)| {
                            (uv[s][a][1] - uv[s][a][0]) / (natural[s][a][1] - natural[s][a][0])
                        };
                        width((s, a)).total_cmp(&width((t, b)))
                    })
                    .unwrap();
                let r = uv[side][axis];
                let mid = r[0] * 0.5 + r[1] * 0.5;
                if !(r[0] < mid && mid < r[1]) {
                    out.reason = "source-face-gap-resolution-limit";
                    return Ok(out);
                }
                let mut left = uv;
                let mut right = uv;
                left[side][axis][1] = mid;
                right[side][axis][0] = mid;
                queue.push(right);
                queue.push(left);
            }
        }
    }
    out.certificate = Some(Certificate {
        body,
        faces: faces.map(|g| g.to_vec()),
        lower_mm: lower,
    });
    out.uncertain_faces = None;
    out.uncertain_uv = None;
    out.reason = "source-face-gap-qualified";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_weight_change_keeps_transverse_parameter_active() {
        let mut s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 2.]],
                vec![vec![1., 0., 0.], vec![1., 0., 2.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        assert!(xy_independent_v(&s));
        s.weights[1][1] = 2.;
        assert!(!xy_independent_v(&s));
        let a = s.evaluate(0.5, 0.).unwrap().point;
        let b = s.evaluate(0.5, 1.).unwrap().point;
        assert!((a[0] - b[0]).abs() > 0.1);
    }
}
